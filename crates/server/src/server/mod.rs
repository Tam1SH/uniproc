use crate::codec::traits::AsyncMessageCodec;
use crate::server::commands::{Request, Response};
use crate::server::message_protocol::MessageProtocol;
use rkyv::Deserialize;
use std::time::Duration;

pub mod builder;
pub mod client;
pub mod client_per_core;
pub mod commands;
pub mod main_loop;
pub mod message_protocol;
pub mod rkyv_protocol;
pub mod runtime;
pub mod utils;
pub mod worker;

pub mod tpc_pool;
pub mod transport;

pub trait ServiceHandler<P: MessageProtocol>: Clone + Send + Sync + 'static {
    async fn on_request(&self, req: &P::RequestView) -> anyhow::Result<P::Response>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::builder::setup;
    use crate::server::client::{Client, Priority};
    use crate::server::commands::{ArchivedRequest, ArchivedResponse};
    use crate::server::rkyv_protocol::RkyvProtocol;
    use crate::server::transport::raw::peer::PeerConfig;
    use crate::server::transport::stream::adapters::tcp::TcpTransport;
    use std::ops::Deref;

    #[derive(Clone)]
    struct EchoHandler;
    impl ServiceHandler<RkyvProtocol<Request, Response>> for EchoHandler {
        async fn on_request(&self, req: &ArchivedRequest) -> anyhow::Result<Response> {
            match req {
                ArchivedRequest::Ping => Ok(Response::Pong),

                _ => Err(anyhow::anyhow!("not a ping")),
            }
        }
    }

    #[compio::test]
    async fn test_rpc_builder_flow() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_thread_ids(true)
            .init();

        runtime::init();

        let num_cores = 3;

        let transport_config = TcpTransport::new("127.0.0.1", 11001, PeerConfig::default());

        let srv_transport = transport_config.clone();

        compio::runtime::spawn(async move {
            setup::<RkyvProtocol<Request, Response>>()
                .with_transport(move |id| srv_transport.server_builder(id))
                .cores(num_cores)
                .service(EchoHandler)
                .run()
                .await
                .expect("Failed to start server");
        })
        .detach();

        compio::time::sleep(Duration::from_millis(200)).await;

        let client = Client::<RkyvProtocol<Request, Response>>::connect_with(num_cores, |id| {
            transport_config.client_connector(id)
        })
        .await
        .expect("Failed to connect fat client");

        let res = client
            .call(Request::Ping, Priority::Normal)
            .await
            .expect("Call failed");

        assert_eq!(*res.deref(), ArchivedResponse::Pong);
    }
}

#[cfg(test)]
mod bench_tests {
    use super::*;
    use crate::server::builder::setup;
    use crate::server::client::{Client, Priority};
    use crate::server::rkyv_protocol::RkyvProtocol;
    use crate::server::transport::raw::peer::PeerConfig;
    use crate::server::transport::stream::adapters::tcp::TcpTransport;
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    use hdrhistogram::Histogram;
    use rkyv::{Archive, Deserialize, Serialize};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[derive(Archive, Deserialize, Serialize, Debug)]
    #[rkyv(derive(Debug, PartialEq, Eq))]
    pub enum Request {
        Ping,
        SmallTask(u64),
        BigData(Vec<u8>),
    }

    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
    #[rkyv(derive(Debug, PartialEq, Eq))]
    pub enum Response {
        Pong,
        Processed(u64),
        DataReceived(usize),
    }

    #[derive(Clone)]
    struct EchoHandler;
    impl ServiceHandler<RkyvProtocol<Request, Response>> for EchoHandler {
        async fn on_request(&self, req: &ArchivedRequest) -> anyhow::Result<Response> {
            match req {
                ArchivedRequest::Ping => Ok(Response::Pong),
                ArchivedRequest::SmallTask(val) => Ok(Response::Processed(u64::from(*val))),
                ArchivedRequest::BigData(data) => Ok(Response::DataReceived(data.len())),
            }
        }
    }

    #[compio::test]
    async fn bench_rpc_comprehensive() {
        runtime::init();

        let base_port = 4343;

        let test_duration = Duration::from_secs(10);
        let transport_config = Arc::new(TcpTransport::new(
            "127.0.0.1",
            base_port,
            PeerConfig::default(),
        ));

        let srv_transport = transport_config.clone();
        compio::runtime::spawn(async move {
            let server = setup::<RkyvProtocol<Request, Response>>()
                .cores(num_cpus::get())
                .service(EchoHandler)
                .with_transport(move |core_id| srv_transport.server_builder(core_id))
                .run()
                .await;

            if let Err(e) = server {
                eprintln!("Server error: {}", e);
            }
        })
        .detach();

        compio::time::sleep(Duration::from_millis(500)).await;

        let client =
            Client::<RkyvProtocol<Request, Response>>::connect_with(num_cpus::get(), |core_id| {
                transport_config.client_connector(core_id)
            })
            .await
            .expect("Failed to connect FatClient");

        let critical_counter = Arc::new(AtomicU64::new(0));
        let normal_counter = Arc::new(AtomicU64::new(0));
        let bulk_counter = Arc::new(AtomicU64::new(0));
        let bulk_bytes = Arc::new(AtomicU64::new(0));

        let mut critical_hist = Histogram::<u64>::new_with_bounds(100, 1_000_000_000, 3).unwrap();

        let normal_concurrency = 12;
        for _ in 0..normal_concurrency {
            let client_clone = client.clone();
            let counter = normal_counter.clone();
            compio::runtime::spawn(async move {
                let mut futures = FuturesUnordered::new();
                for _ in 0..12 {
                    futures.push(client_clone.call(Request::SmallTask(42), Priority::Normal));
                }
                while let Some(res) = futures.next().await {
                    if res.is_ok() {
                        counter.fetch_add(1, Ordering::Relaxed);
                        futures.push(client_clone.call(Request::SmallTask(42), Priority::Normal));
                    }
                }
            })
            .detach();
        }

        let bulk_concurrency = 32;
        let large_payload = vec![0xAAu8; 64 * 1024];
        for _ in 0..bulk_concurrency {
            let client_clone = client.clone();
            let counter = bulk_counter.clone();
            let bytes = bulk_bytes.clone();
            let payload = large_payload.clone();
            compio::runtime::spawn(async move {
                let mut futures = FuturesUnordered::new();
                for _ in 0..5 {
                    futures
                        .push(client_clone.call(Request::BigData(payload.clone()), Priority::Bulk));
                }
                while let Some(res) = futures.next().await {
                    if res.is_ok() {
                        counter.fetch_add(1, Ordering::Relaxed);
                        bytes.fetch_add(payload.len() as u64, Ordering::Relaxed);
                        futures.push(
                            client_clone.call(Request::BigData(payload.clone()), Priority::Bulk),
                        );
                    }
                }
            })
            .detach();
        }

        println!("🔥 Warm-up (2 sec)...");
        compio::time::sleep(Duration::from_secs(2)).await;

        println!(
            "🚀 Benchmarking (Duration: {:?}, Cores: {})...",
            test_duration,
            runtime::core_count()
        );

        let start_test = Instant::now();

        while start_test.elapsed() < test_duration {
            let now = Instant::now();
            let res = client.call(Request::Ping, Priority::Critical).await;

            if let Ok(_) = res {
                let rtt = now.elapsed();
                critical_hist.record(rtt.as_nanos() as u64).unwrap();
                critical_counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        let total_duration = start_test.elapsed().as_secs_f64();

        let crit_total = critical_counter.load(Ordering::Relaxed);
        let norm_total = normal_counter.load(Ordering::Relaxed);
        let bulk_total = bulk_counter.load(Ordering::Relaxed);
        let bulk_mb = bulk_bytes.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0;

        println!("\n{}", "=".repeat(60));
        println!("📊 FINAL RPC PERFORMANCE REPORT");
        println!("{}", "=".repeat(60));

        println!("CRITICAL (Latency Oriented):");
        println!("  Total:      {} req", crit_total);
        println!(
            "  RPS:        {:.2} req/sec",
            crit_total as f64 / total_duration
        );
        println!(
            "  P50 Latency: {:>10?}",
            Duration::from_nanos(critical_hist.value_at_quantile(0.5))
        );
        println!(
            "  P99 Latency: {:>10?}",
            Duration::from_nanos(critical_hist.value_at_quantile(0.99))
        );
        println!(
            "  Max Latency: {:>10?}",
            Duration::from_nanos(critical_hist.max())
        );

        println!("\nNORMAL (Balanced):");
        println!("  Total:      {} req", norm_total);
        println!(
            "  RPS:        {:.2} req/sec",
            norm_total as f64 / total_duration
        );

        println!("\nBULK (Throughput Oriented):");
        println!("  Total:      {} req", bulk_total);
        println!(
            "  RPS:        {:.2} req/sec",
            bulk_total as f64 / total_duration
        );
        println!("  Bandwidth:  {:.2} MB/sec", bulk_mb / total_duration);

        println!("{}", "=".repeat(60));
        println!(
            "Total Aggregated RPS: {:.2} req/sec",
            (crit_total + norm_total + bulk_total) as f64 / total_duration
        );
        println!("{}", "=".repeat(60));
    }

    #[compio::test]
    async fn bench_rpc_normal_stress() {
        runtime::init();

        let base_port = 5353;
        let test_duration = Duration::from_secs(5);
        let num_cores = num_cpus::get();

        let transport_config = Arc::new(TcpTransport::new(
            "127.0.0.1",
            base_port,
            PeerConfig::default(),
        ));

        let srv_transport = transport_config.clone();
        compio::runtime::spawn(async move {
            let _ = setup::<RkyvProtocol<Request, Response>>()
                .cores(num_cores)
                .service(EchoHandler)
                .with_transport(move |core_id| srv_transport.server_builder(core_id))
                .run()
                .await;
        })
        .detach();

        compio::time::sleep(Duration::from_millis(500)).await;

        let client =
            Client::<RkyvProtocol<Request, Response>>::connect_with(num_cores, |core_id| {
                transport_config.client_connector(core_id)
            })
            .await
            .expect("Failed to connect FatClient");

        let normal_counter = Arc::new(AtomicU64::new(0));
        let error_counter = Arc::new(AtomicU64::new(0));

        let hist = Arc::new(std::sync::Mutex::new(
            Histogram::<u64>::new_with_bounds(100, 1_000_000_000, 3).unwrap(),
        ));

        let concurrency_per_lane = 4;
        let num_normal_lanes = (num_cores - 1 + 1) / 2;
        let total_workers = num_normal_lanes * concurrency_per_lane;

        println!(
            "🔥 Initializing stress test with {} workers for NORMAL priority...",
            total_workers
        );

        for _ in 0..total_workers {
            let client_clone = client.clone();
            let counter = normal_counter.clone();
            let err_counter = error_counter.clone();
            let hist_clone = hist.clone();

            compio::runtime::spawn(async move {
                loop {
                    let start = Instant::now();
                    let res = client_clone
                        .call(Request::SmallTask(0), Priority::Normal)
                        .await;

                    let elapsed = start.elapsed().as_nanos() as u64;

                    match res {
                        Ok(_) => {
                            counter.fetch_add(1, Ordering::Relaxed);
                            let mut h = hist_clone.lock().unwrap();
                            let _ = h.record(elapsed);
                        }
                        Err(_) => {
                            err_counter.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            })
            .detach();
        }

        println!("🚀 Warming up (3 sec)...");
        compio::time::sleep(Duration::from_secs(3)).await;

        normal_counter.store(0, Ordering::SeqCst);
        {
            let mut h = hist.lock().unwrap();
            h.reset();
        }

        println!("⚡ Starting measurements for {:?}...", test_duration);
        let start_time = Instant::now();
        compio::time::sleep(test_duration).await;
        let actual_duration = start_time.elapsed();

        let total_requests = normal_counter.load(Ordering::SeqCst);
        let total_errors = error_counter.load(Ordering::SeqCst);
        let rps = total_requests as f64 / actual_duration.as_secs_f64();

        let h = hist.lock().unwrap();

        println!("\n{}", "=".repeat(60));
        println!("📊 NORMAL PRIORITY STRESS REPORT");
        println!("{}", "=".repeat(60));
        println!("Cores used:          {}", num_cores);
        println!("Normal Lanes:        {}", num_normal_lanes);
        println!("Workers:             {}", total_workers);
        println!("Duration:            {:.2?}", actual_duration);
        println!("{}", "-".repeat(60));
        println!("Throughput:");
        println!("  Total Requests:    {}", total_requests);
        println!("  RPS:               {:.0} req/sec", rps);
        println!("  Errors:            {}", total_errors);
        println!("{}", "-".repeat(60));
        println!("Latency:");
        println!(
            "  P50:               {:?}",
            Duration::from_nanos(h.value_at_quantile(0.5))
        );
        println!(
            "  P90:               {:?}",
            Duration::from_nanos(h.value_at_quantile(0.9))
        );
        println!(
            "  P99:               {:?}",
            Duration::from_nanos(h.value_at_quantile(0.99))
        );
        println!(
            "  P99.9:             {:?}",
            Duration::from_nanos(h.value_at_quantile(0.999))
        );
        println!("  Max:               {:?}", Duration::from_nanos(h.max()));
        println!("{}", "=".repeat(60));
    }
}
