use crate::align_buffer::AlignedBuffer;
use crate::server::client::Priority;
use crate::server::tpc_pool::{InnerPool, Mixed, TpcPool};
use crate::server::transport::raw::{MsgBatch, OutgoingMsg, SendHandle};
use crate::server::transport::stream::AsyncStream;
use bytes::BytesMut;
use compio::buf::{IntoInner, IoBuf, IoBufMut, IoVectoredBufMut, SetLen, Slice};
use compio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use compio::BufResult;
use futures::{FutureExt, SinkExt, StreamExt};
use std::io;
use tracing::{debug, error, info, instrument, trace};

#[derive(Clone, Debug)]
pub struct PeerConfig {
    pub priority: Priority,
    pub channel_size: usize,
    pub batch_limit: usize,
    pub read_buffer_capacity: usize,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self::for_priority(Priority::Normal)
    }
}

impl PeerConfig {
    pub fn for_priority(priority: Priority) -> Self {
        match priority {
            Priority::Critical => Self {
                priority,
                channel_size: 16,
                batch_limit: 16,
                read_buffer_capacity: 32 * 1024, // 32KB
            },
            Priority::Normal => Self {
                priority,
                channel_size: 16,
                batch_limit: 32,
                read_buffer_capacity: 256 * 1024, // 256KB
            },
            Priority::Bulk => Self {
                priority,
                channel_size: 32,
                batch_limit: 64,
                read_buffer_capacity: 1024 * 1024, // 1MB
            },
        }
    }
}

pub struct Peer;

impl Peer {
    #[instrument(skip_all)]
    pub fn new<S: AsyncStream + Clone + 'static>(
        stream: S,
        config: PeerConfig,
    ) -> io::Result<(SendHandle, flume::Receiver<AlignedBuffer>)> {
        let (outgoing_tx, outgoing_rx) = flume::bounded(config.channel_size);
        let (incoming_tx, incoming_rx) = flume::bounded(config.channel_size);

        let (writer, reader) = stream.split()?;

        compio::runtime::spawn(Self::run_writer(writer, outgoing_rx, config.clone())).detach();

        compio::runtime::spawn(Self::run_reader(reader, incoming_tx, config)).detach();

        Ok((SendHandle { outgoing_tx }, incoming_rx))
    }

    #[instrument(skip_all, name = "peer_writer")]
    async fn run_writer<W: AsyncWrite>(
        mut writer: W,
        outgoing_rx: flume::Receiver<OutgoingMsg>,
        config: PeerConfig,
    ) -> anyhow::Result<()> {
        debug!("Writer worker started");

        let mut batch = Vec::with_capacity(config.batch_limit * 2);

        while let Ok(msg_enum) = outgoing_rx.recv_async().await {
            TpcPool::with(|pool| {
                let add_to_batch = |p: &mut InnerPool, b: &mut Vec<Mixed>, msg: AlignedBuffer| {
                    let header = p.acquire_header(msg.0.len());
                    b.push(Mixed::Bytes(header));
                    b.push(Mixed::AlignedBuffer(msg));
                };

                match msg_enum {
                    OutgoingMsg::Single(msg) => add_to_batch(pool, &mut batch, msg),
                    OutgoingMsg::Batch(msgs) => {
                        for msg in msgs {
                            add_to_batch(pool, &mut batch, msg);
                        }
                    }
                }

                while batch.len() < config.batch_limit * 2 {
                    match outgoing_rx.try_recv() {
                        Ok(OutgoingMsg::Single(msg)) => add_to_batch(pool, &mut batch, msg),
                        Ok(OutgoingMsg::Batch(msgs)) => {
                            for msg in msgs {
                                add_to_batch(pool, &mut batch, msg);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            trace!(
                msgs_count = batch.len() / 2,
                total_iovs = batch.len(),
                "Sending vectored batch"
            );

            let BufResult(res, returned_batch) = writer.write_vectored_all(batch).await;

            if let Err(e) = res {
                error!(error = ?e, "Failed to write batch to stream");
                return Err(e.into());
            }

            batch = returned_batch;

            TpcPool::with(|pool| {
                for buf in batch.drain(..) {
                    pool.release_mixed(buf);
                }
            });
        }

        info!("Writer worker exiting");
        Ok(())
    }

    #[instrument(skip_all, name = "peer_reader")]
    pub async fn run_reader<R: AsyncRead>(
        mut reader: R,
        incoming_tx: flume::Sender<AlignedBuffer>,
        config: PeerConfig,
    ) -> anyhow::Result<()> {
        debug!("Reader worker started");

        let mut buffer = TpcPool::acquire_body(config.read_buffer_capacity);

        loop {
            if buffer.len() == buffer.0.capacity() {
                let current_cap = buffer.0.capacity();
                let new_cap = if current_cap == 0 {
                    4096
                } else {
                    current_cap * 2
                };

                if new_cap > 100 * 1024 * 1024 {
                    return Err(anyhow::anyhow!("Buffer limit exceeded"));
                }

                let mut new_buf = TpcPool::acquire_body(new_cap);

                unsafe {
                    let len = buffer.len();
                    new_buf.set_len(len);
                    if len > 0 {
                        std::ptr::copy_nonoverlapping(buffer.as_ptr(), new_buf.as_mut_ptr(), len);
                    }
                }

                TpcPool::release_body(buffer);
                buffer = new_buf;
            }

            let prev_len = buffer.len();

            let BufResult(res, returned_buf) = reader.read(buffer).await;
            buffer = returned_buf;

            let n = match res {
                Ok(0) => {
                    info!("Reader reached EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };

            unsafe { buffer.set_len(prev_len + n) };

            let mut offset = 0;
            loop {
                let available = buffer.len() - offset;

                if available < 4 {
                    break;
                }

                let len_slice = &buffer.0[offset..offset + 4];
                let msg_len = u32::from_le_bytes(len_slice.try_into().unwrap()) as usize;

                let total_frame_len = 4 + msg_len;

                if available < total_frame_len {
                    break;
                }

                let mut payload = TpcPool::acquire_body(msg_len);
                unsafe {
                    payload.set_len(msg_len);

                    std::ptr::copy_nonoverlapping(
                        buffer.as_ptr().add(offset + 4),
                        payload.as_mut_ptr(),
                        msg_len,
                    );
                }

                if incoming_tx.send_async(payload).await.is_err() {
                    return Err(anyhow::anyhow!("Channel closed"));
                }

                offset += total_frame_len;
            }

            if offset > 0 {
                let remaining = buffer.len() - offset;
                if remaining > 0 {
                    unsafe {
                        let ptr = buffer.as_mut_ptr();

                        std::ptr::copy(ptr.add(offset), ptr, remaining);
                    }
                }
                unsafe { buffer.set_len(remaining) };
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::transport::stream::vsock::general::{VListener, VStream};
    use rkyv::util::AlignedVec;
    use tracing::Level;

    async fn setup_vsock_pair(port: u32) -> (VStream, VStream) {
        let listener = VListener::bind(port).expect("Vsock bind failed");

        let server_handle =
            compio::runtime::spawn(
                async move { listener.accept().await.expect("Vsock accept failed") },
            );

        let client_stream = VStream::connect(0, port)
            .await
            .expect("Vsock connect failed");

        let (server_stream, _) = server_handle.await.unwrap();

        (server_stream, client_stream)
    }

    #[compio::test]
    async fn test_peer_simple_delivery() {
        let (server_stream, client_stream) = setup_vsock_pair(8000).await;

        let (mut s_handle, _) = Peer::new(server_stream, PeerConfig::default()).unwrap();
        let (_, mut c_rx) = Peer::new(client_stream, PeerConfig::default()).unwrap();

        let msg = vec![1u8, 3u8, 3u8, 7u8];
        let mut a = AlignedVec::with_capacity(4);
        a.extend_from_slice(&msg[..]);

        s_handle.send(AlignedBuffer(a.clone())).await.unwrap();

        let received = c_rx.recv_async().await.expect("No message received");
        assert_eq!(received.0.as_slice(), a.as_slice());
    }

    #[compio::test]
    async fn test_peer_large_packet() {
        let (server_stream, client_stream) = setup_vsock_pair(8001).await;

        let (s_handle, _s_rx) = Peer::new(server_stream, PeerConfig::default()).unwrap();
        let (_c_handle, c_rx) = Peer::new(client_stream, PeerConfig::default()).unwrap();

        let large_data = vec![0xAAu8; 1024 * 1024];
        let mut a = AlignedVec::with_capacity(1024 * 1024);
        a.extend_from_slice(&large_data[..]);

        s_handle.send(AlignedBuffer(a.clone())).await.unwrap();

        let received = c_rx.recv_async().await.expect("No message received");
        assert_eq!(received.0.len(), large_data.len());
        assert_eq!(received.0.as_slice(), a.as_slice());
    }

    #[compio::test]
    async fn test_peer_full_duplex_bidirectional() {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .init();

        let (server_stream, client_stream) = setup_vsock_pair(8002).await;

        let (mut s_tx, mut s_rx) = Peer::new(server_stream, PeerConfig::default()).unwrap();
        let (mut c_tx, mut c_rx) = Peer::new(client_stream, PeerConfig::default()).unwrap();

        let mut s_data = vec![0x11u8; 64];
        let mut c_data = vec![0x22u8; 64];

        s_data[0..8].copy_from_slice(&0xDEADBEEF_00000001u64.to_le_bytes());
        s_data[56..64].copy_from_slice(&0xDEADBEEF_00000002u64.to_le_bytes());

        c_data[0..8].copy_from_slice(&0xDEADBEEF_00000001u64.to_le_bytes());
        c_data[56..64].copy_from_slice(&0xDEADBEEF_00000002u64.to_le_bytes());

        let (res_c, res_s) = futures::join!(
            async {
                let mut a = AlignedVec::with_capacity(c_data.len());
                a.extend_from_slice(&c_data);
                c_tx.send(AlignedBuffer(a)).await.unwrap();
                c_rx.recv_async().await.unwrap()
            },
            async {
                let mut a = AlignedVec::with_capacity(s_data.len());
                a.extend_from_slice(&s_data);
                s_tx.send(AlignedBuffer(a)).await.unwrap();
                s_rx.recv_async().await.unwrap()
            }
        );

        assert_eq!(res_c.0.as_slice(), s_data.as_slice());
        assert_eq!(res_s.0.as_slice(), c_data.as_slice());
    }

    #[compio::test]
    async fn test_peer_multiple_messages_order() {
        let (server_stream, client_stream) = setup_vsock_pair(8003).await;
        let (mut s_tx, _) = Peer::new(server_stream, PeerConfig::default()).unwrap();
        let (_, mut c_rx) = Peer::new(client_stream, PeerConfig::default()).unwrap();

        let counts = [10, 20, 30];

        for &len in &counts {
            let mut a = AlignedVec::with_capacity(len);
            a.extend_from_slice(&vec![len as u8; len]);
            s_tx.send(AlignedBuffer(a)).await.unwrap();
        }

        for &len in &counts {
            let received = c_rx.recv_async().await.expect("Failed to receive");
            assert_eq!(received.0.len(), len);
            assert!(received.0.iter().all(|&b| b == len as u8));
        }
    }

    #[compio::test]
    async fn test_peer_zero_length_packet() {
        let (server_stream, client_stream) = setup_vsock_pair(8004).await;
        let (mut s_tx, _) = Peer::new(server_stream, PeerConfig::default()).unwrap();
        let (_, mut c_rx) = Peer::new(client_stream, PeerConfig::default()).unwrap();

        s_tx.send(AlignedBuffer(AlignedVec::new())).await.unwrap();

        let mut a = AlignedVec::with_capacity(3);
        a.extend_from_slice(&[1, 2, 3]);
        s_tx.send(AlignedBuffer(a)).await.unwrap();

        let received = c_rx.recv_async().await.unwrap();

        if received.0.is_empty() {
            let second = c_rx.recv_async().await.unwrap();
            assert_eq!(second.0.as_slice(), &[1, 2, 3]);
        } else {
            assert_eq!(received.0.as_slice(), &[1, 2, 3]);
        }
    }

    #[compio::test]
    async fn test_peer_varying_sizes_exceeding_buffer() {
        let (server_stream, client_stream) = setup_vsock_pair(8005).await;

        let mut config = PeerConfig::default();
        config.read_buffer_capacity = 4096;

        let (s_handle, _) = Peer::new(server_stream, PeerConfig::default()).unwrap();
        let (_, c_rx) = Peer::new(client_stream, config).unwrap();

        let sizes = vec![100, 4000, 5000, 64 * 1024, 1024 * 1024];

        for &size in &sizes {
            let mut data = AlignedVec::with_capacity(size);

            let content: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            data.extend_from_slice(&content);

            debug!("Sending message of size {}", size);
            s_handle.send(AlignedBuffer(data)).await.unwrap();
        }

        for &size in &sizes {
            let received = c_rx.recv_async().await.expect("Failed to receive message");

            debug!("Received message of len {}", received.0.len());

            assert_eq!(
                received.0.len(),
                size,
                "Message size mismatch. Expected {}, got {}",
                size,
                received.0.len()
            );

            let expected: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            assert_eq!(
                received.0.as_slice(),
                expected.as_slice(),
                "Content mismatch for size {}",
                size
            );
        }
    }
}

#[cfg(test)]
mod bench_peer {
    use super::*;
    use compio::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    async fn create_peer_pair(
        config: PeerConfig,
    ) -> (
        SendHandle,
        flume::Receiver<AlignedBuffer>,
        SendHandle,
        flume::Receiver<AlignedBuffer>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let conf = config.clone();
        let server_task = compio::runtime::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            Peer::new(stream, conf).unwrap()
        });

        let client_stream = TcpStream::connect(addr).await.unwrap();
        let (c_handle, c_rx) = Peer::new(client_stream, config).unwrap();
        let (s_handle, s_rx) = server_task.await.unwrap();

        (c_handle, c_rx, s_handle, s_rx)
    }

    #[cfg(feature = "dhat-heap")]
    #[global_allocator]
    static ALLOC: dhat::Alloc = dhat::Alloc;

    #[compio::test]
    async fn stress_test_peer_throughput_with_latency() {
        #[cfg(feature = "dhat-heap")]
        let _profiler = dhat::Profiler::new_heap();

        let duration = Duration::from_secs(5);
        let msg_size = 64;

        let config = PeerConfig {
            priority: Priority::Normal,
            channel_size: 8,
            batch_limit: 16,
            read_buffer_capacity: 32 * 1024,
        };

        let (c_handle, _c_rx, _s_handle, s_rx) = create_peer_pair(config).await;

        let total_msgs = Arc::new(AtomicU64::new(0));
        let total_latency_ns = Arc::new(AtomicU64::new(0));
        let latency_samples = Arc::new(AtomicU64::new(0));

        let t_msgs = total_msgs.clone();
        let t_lat = total_latency_ns.clone();
        let l_samples = latency_samples.clone();

        compio::runtime::spawn(async move {
            while let Ok(msg) = s_rx.recv_async().await {
                t_msgs.fetch_add(1, Ordering::Relaxed);

                if msg.0.len() >= 8 {
                    let sent_at_bits = u64::from_le_bytes(msg.0[..8].try_into().unwrap());
                    if sent_at_bits != 0 {
                        let sent_at =
                            Instant::now().checked_sub(Duration::from_nanos(sent_at_bits));
                        if let Some(elapsed) = sent_at {
                            t_lat.fetch_add(elapsed.elapsed().as_nanos() as u64, Ordering::Relaxed);
                            l_samples.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                TpcPool::release_body(msg);
            }
        })
        .detach();

        let start = Instant::now();

        for _ in [1] {
            let h = c_handle.clone();
            compio::runtime::spawn(async move {
                let mut local_counter = 0u64;
                loop {
                    local_counter += 1;
                    let mut buf = TpcPool::acquire_body(msg_size);
                    unsafe {
                        buf.set_len(msg_size);
                    }

                    if local_counter % 10000 == 0 {
                        let now_ns = start.elapsed().as_nanos() as u64;
                        buf.0[..8].copy_from_slice(&now_ns.to_le_bytes());
                    } else {
                        buf.0[..8].fill(0);
                    }

                    if h.send(buf).await.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }

        compio::time::sleep(duration).await;

        let total = total_msgs.load(Ordering::Acquire);
        let samples = latency_samples.load(Ordering::Acquire);
        let lat_sum = total_latency_ns.load(Ordering::Acquire);

        let elapsed = start.elapsed().as_secs_f64();
        let rps = total as f64 / elapsed;
        let avg_latency_us = if samples > 0 {
            (lat_sum as f64 / samples as f64) / 1000.0
        } else {
            0.0
        };

        println!("\n📊 PEER PERFORMANCE REPORT");
        println!("RPS:          {:.2} req/sec", rps);
        println!(
            "Throughput:   {:.2} MB/sec",
            (total * (msg_size as u64 + 4)) as f64 / 1024.0 / 1024.0 / elapsed
        );
        println!(
            "Avg Latency:  {:.2} μs (sampled every 1000th msg)",
            avg_latency_us
        );
    }
}
