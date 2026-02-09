use crate::align_buffer::AlignedBuffer;
use crate::peer::tpc_pool::TpcPool;
use crate::vsock::AsyncStream;
use bytes::{Bytes, BytesMut};
use compio::buf::{IoBufMut, SetLen};
use compio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use compio::time::timeout;
use compio::BufResult;
use futures::{channel::mpsc, FutureExt, SinkExt, StreamExt};
use std::io;
use std::time::Duration;
use tracing::{debug, error, info, instrument, trace};

pub mod tpc_pool;
#[derive(Clone)]
pub struct PeerHandle {
    outgoing_tx: flume::Sender<AlignedBuffer>,
}

impl PeerHandle {
    pub async fn send_raw(&self, data: AlignedBuffer) -> anyhow::Result<()> {
        self.outgoing_tx
            .send_async(data)
            .await
            .map_err(|e| anyhow::anyhow!("Peer dead: {}", e))
    }
}

pub struct Peer;

impl Peer {
    #[instrument(skip_all)]
    pub fn new<S: AsyncStream + Clone + 'static>(
        stream: S,
    ) -> io::Result<(PeerHandle, flume::Receiver<AlignedBuffer>)> {
        info!("Initializing Peer");
        let (outgoing_tx, outgoing_rx) = flume::bounded::<AlignedBuffer>(128);
        let (incoming_tx, incoming_rx) = flume::bounded::<AlignedBuffer>(128);

        let (writer, reader) = stream.split()?;

        compio::runtime::spawn(async move {
            if let Err(e) = Self::run_writer(writer, outgoing_rx).await {
                error!("Peer writer died: {:?}", e);
            }
        })
        .detach();

        compio::runtime::spawn(async move {
            if let Err(e) = Self::run_reader(reader, incoming_tx).await {
                error!("Peer reader died: {:?}", e);
            }
        })
        .detach();

        Ok((PeerHandle { outgoing_tx }, incoming_rx))
    }

    #[instrument(skip_all, name = "peer_writer")]
    async fn run_writer<W: AsyncWrite>(
        mut writer: W,
        outgoing_rx: flume::Receiver<AlignedBuffer>,
    ) -> anyhow::Result<()> {
        debug!("Writer worker started");

        while let Ok(msg) = outgoing_rx.recv_async().await {
            let len = msg.0.len() as u32;

            let mut head_buf = TpcPool::acquire_header();
            unsafe { head_buf.set_len(4) };
            head_buf[..4].copy_from_slice(&len.to_le_bytes());

            trace!(len, "Sending packet");

            let BufResult(res, (head_buf, (msg,))) =
                writer.write_vectored_all((head_buf, (msg,))).await;

            TpcPool::release_header(head_buf);

            if let Err(e) = res {
                error!(error = ?e, "Failed to write packet to stream");

                return Err(e.into());
            }
        }

        info!("Writer worker exiting");
        Ok(())
    }

    #[instrument(skip_all, name = "peer_reader")]
    async fn run_reader<R: AsyncRead>(
        mut reader: R,
        incoming_tx: flume::Sender<AlignedBuffer>,
    ) -> anyhow::Result<()> {
        debug!("Reader worker started");
        loop {
            trace!("Waiting for next packet length...");
            let mut head_buf = TpcPool::acquire_header();
            unsafe { head_buf.set_len(4) };

            let BufResult(res, head_buf) = reader.read_exact(head_buf).await;
            if let Err(e) = res {
                TpcPool::release_header(head_buf);
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    info!("Reader reached EOF");
                } else {
                    error!(error = ?e, "Failed to read packet length");
                }
                break;
            }

            let len = u32::from_le_bytes(head_buf[..4].try_into()?) as usize;
            TpcPool::release_header(head_buf);

            if len == 0 {
                trace!("Received zero-length packet, skipping");
                continue;
            }

            if len > 100 * 1024 * 1024 {
                // 100MB sanity check
                error!(len, "Packet too large, possible protocol desync");
                return Err(anyhow::anyhow!("Packet too large"));
            }

            debug!(len, "Reading packet body");
            let mut body_buf = TpcPool::acquire_body(len);
            unsafe { body_buf.set_len(len) };

            let BufResult(res, body_buf) = reader.read_exact(body_buf).await;
            if let Err(e) = res {
                error!(error = ?e, len, "Failed to read packet body");
                TpcPool::release_body(body_buf);
                return Err(e.into());
            }

            trace!(len, "Dispatching packet to application");

            if let Err(e) = incoming_tx.send_async(body_buf).await {
                error!(error = ?e, "Failed to send packet to incoming_rx (receiver dropped)");

                TpcPool::release_body(e.0);
                break;
            }
        }
        info!("Reader worker exiting");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vsock::{Listener, Stream};
    use rkyv::util::AlignedVec;
    use tracing::Level;

    async fn setup_vsock_pair(port: u32) -> (Stream, Stream) {
        let listener = Listener::bind(port).expect("Vsock bind failed");

        let server_handle =
            compio::runtime::spawn(
                async move { listener.accept().await.expect("Vsock accept failed") },
            );

        let client_stream = Stream::connect(0, port)
            .await
            .expect("Vsock connect failed");

        let (server_stream, _) = server_handle.await.unwrap();

        (server_stream, client_stream)
    }

    #[compio::test]
    async fn test_peer_simple_delivery() {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .init();

        let (server_stream, client_stream) = setup_vsock_pair(8000).await;

        let (mut s_handle, _) = Peer::new(server_stream).unwrap();
        let (_, mut c_rx) = Peer::new(client_stream).unwrap();

        let msg = vec![1u8, 3u8, 3u8, 7u8];
        let mut a = AlignedVec::with_capacity(4);
        a.extend_from_slice(&msg[..]);

        s_handle.send_raw(AlignedBuffer(a.clone())).await.unwrap();

        let received = c_rx.recv_async().await.expect("No message received");
        assert_eq!(received.0.as_slice(), a.as_slice());
    }

    #[compio::test]
    async fn test_peer_large_packet() {
        let (server_stream, client_stream) = setup_vsock_pair(8001).await;

        let (mut s_handle, _s_rx) = Peer::new(server_stream).unwrap();
        let (_c_handle, mut c_rx) = Peer::new(client_stream).unwrap();

        let large_data = vec![0xAAu8; 1024 * 1024];
        let mut a = AlignedVec::with_capacity(1024 * 1024);
        a.extend_from_slice(&large_data[..]);

        s_handle.send_raw(AlignedBuffer(a.clone())).await.unwrap();

        let received = c_rx.recv_async().await.expect("No message received");
        assert_eq!(received.0.len(), large_data.len());
        assert_eq!(received.0.as_slice(), a.as_slice());
    }

    #[compio::test]
    async fn test_peer_full_duplex_bidirectional() {
        let (server_stream, client_stream) = setup_vsock_pair(8002).await;

        let (mut s_tx, mut s_rx) = Peer::new(server_stream).unwrap();
        let (mut c_tx, mut c_rx) = Peer::new(client_stream).unwrap();

        let s_data = vec![0x11u8; 64];
        let c_data = vec![0x22u8; 64];

        let (res_c, res_s) = futures::join!(
            async {
                let mut a = AlignedVec::with_capacity(c_data.len());
                a.extend_from_slice(&c_data);
                c_tx.send_raw(AlignedBuffer(a)).await.unwrap();
                c_rx.recv_async().await.unwrap()
            },
            async {
                let mut a = AlignedVec::with_capacity(s_data.len());
                a.extend_from_slice(&s_data);
                s_tx.send_raw(AlignedBuffer(a)).await.unwrap();
                s_rx.recv_async().await.unwrap()
            }
        );

        assert_eq!(res_c.0.as_slice(), s_data.as_slice());
        assert_eq!(res_s.0.as_slice(), c_data.as_slice());
    }

    #[compio::test]
    async fn test_peer_multiple_messages_order() {
        let (server_stream, client_stream) = setup_vsock_pair(8003).await;
        let (mut s_tx, _) = Peer::new(server_stream).unwrap();
        let (_, mut c_rx) = Peer::new(client_stream).unwrap();

        let counts = [10, 20, 30];

        for &len in &counts {
            let mut a = AlignedVec::with_capacity(len);
            a.extend_from_slice(&vec![len as u8; len]);
            s_tx.send_raw(AlignedBuffer(a)).await.unwrap();
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
        let (mut s_tx, _) = Peer::new(server_stream).unwrap();
        let (_, mut c_rx) = Peer::new(client_stream).unwrap();

        s_tx.send_raw(AlignedBuffer(AlignedVec::new()))
            .await
            .unwrap();

        let mut a = AlignedVec::with_capacity(3);
        a.extend_from_slice(&[1, 2, 3]);
        s_tx.send_raw(AlignedBuffer(a)).await.unwrap();

        let received = c_rx.recv_async().await.unwrap();

        if received.0.is_empty() {
            let second = c_rx.recv_async().await.unwrap();
            assert_eq!(second.0.as_slice(), &[1, 2, 3]);
        } else {
            assert_eq!(received.0.as_slice(), &[1, 2, 3]);
        }
    }
}
