use crate::codec::traits::AsyncMessageCodec;
use crate::server::commands::{Request, Response};
use crate::server::protocol::Protocol;
use rkyv::Deserialize;
use std::time::Duration;

mod builder;
mod client;
pub mod commands;
pub mod main_loop;
mod protocol;
mod rkyv_protocol;

pub trait ServiceHandler<P: Protocol>: Send + Sync + 'static {
    async fn on_request(&self, req: &P::RequestView) -> anyhow::Result<P::Response>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::builder::setup;
    use crate::server::client::Client;
    use crate::server::commands::{ArchivedRequest, ArchivedResponse};
    use crate::server::rkyv_protocol::RkyvProtocol;
    use crate::vsock::Stream;
    use std::ops::Deref;

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
        let port = 11001;

        setup()
            .bind(port)
            .service(EchoHandler)
            .run()
            .await
            .expect("Failed to start server");

        compio::time::sleep(Duration::from_millis(50)).await;
        let stream = Stream::connect(0, port).await.unwrap();
        let client = Client::<RkyvProtocol<Request, Response>>::connect(stream)
            .await
            .expect("Failed to connect client");

        let res = client.call(Request::Ping).await.expect("Call failed");
        assert_eq!(*res.deref(), ArchivedResponse::Pong);
    }
}

#[cfg(test)]
mod bench_tests {
    use super::*;
    use crate::server::builder::setup;
    use crate::server::client::Client;
    use crate::server::commands::{ArchivedResponse, Request, Response};
    use crate::server::rkyv_protocol::RkyvProtocol;
    use crate::vsock::{Listener, Stream};
    use compio::net::{TcpListener, TcpStream};
    use hdrhistogram::Histogram;
    use std::ops::Deref;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    struct EchoHandler;
    impl ServiceHandler<RkyvProtocol<Request, Response>> for EchoHandler {
        async fn on_request(&self, req: &commands::ArchivedRequest) -> anyhow::Result<Response> {
            match req {
                _ => Ok(Response::Pong),
            }
        }
    }

    #[compio::test]
    async fn bench_rpc_performance() {
        let port = 4343;
        let test_duration = Duration::from_secs(10);

        let listener = Listener::bind(port)
            // .await
            .unwrap();

        let f = move || Stream::connect(0, port);

        // let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        //     .await
        //     .unwrap();
        //
        // let f = move || TcpStream::connect(format!("127.0.0.1:{port}"));

        setup()
            .bind(port)
            .with_listener(listener)
            .service(EchoHandler)
            .run()
            .await
            .expect("Failed to start server");

        compio::time::sleep(Duration::from_millis(100)).await;

        // 2. Метрики
        let rps_counter = Arc::new(AtomicU64::new(0));
        let mut hist = Histogram::<u64>::new_with_bounds(100, 1_000_000_000, 3).unwrap(); // от 100нс до 1с

        // 3. Создаем несколько клиентов для нагрузки (Saturators)
        // RPC обычно упирается в конкурентность, поэтому запустим несколько воркеров
        let concurrency = 1;
        for i in 0..concurrency {
            let rps_clone = rps_counter.clone();
            compio::runtime::spawn(async move {
                let stream = f().await.unwrap();

                let client = Client::<RkyvProtocol<Request, Response>>::connect(stream)
                    .await
                    .expect("Saturator client failed to connect");

                loop {
                    if client.call(Request::Ping).await.is_ok() {
                        rps_clone.fetch_add(1, Ordering::Relaxed);
                    } else {
                        break;
                    }
                }
            })
            .detach();
        }

        println!(
            "🚀 Starting RPC Bench ({} workers, {:?})...",
            concurrency, test_duration
        );

        // 4. Основной цикл: Измерение LATENCY (Probe)
        // Используем отдельного клиента для замеров, чтобы не стоять в очереди воркеров

        let stream = f().await.unwrap();

        let probe_client = Client::<RkyvProtocol<Request, Response>>::connect(stream)
            .await
            .expect("Probe client failed to connect");

        let start_test = Instant::now();
        let mut samples = 0;

        while start_test.elapsed() < test_duration {
            let now = Instant::now();

            let res = probe_client.call(Request::Ping).await;

            if let Ok(resp) = res {
                // Проверяем корректность (опционально)
                if *resp.deref() == ArchivedResponse::Pong {
                    let rtt = now.elapsed();
                    hist.record(rtt.as_nanos() as u64).unwrap();
                    samples += 1;
                }
            }

            // Небольшая пауза между пробами, чтобы не превратить саму пробу в сатуратор
            compio::time::sleep(Duration::from_micros(100)).await;
        }

        // 5. Отчет
        let total_duration = start_test.elapsed();
        let total_requests = rps_counter.load(Ordering::Relaxed);
        let rps = total_requests as f64 / total_duration.as_secs_f64();

        println!("\n{}", "=".repeat(50));
        println!("🏁 RPC PERFORMANCE REPORT");
        println!("{}", "=".repeat(50));
        println!("Duration:         {:.2} s", total_duration.as_secs_f64());
        println!("Total Requests:   {}", total_requests);
        println!("Throughput (RPS): {:.2} req/sec", rps);
        println!("Latency Samples:  {}", samples);
        println!("{}", "-".repeat(50));
        println!("Latency (RTT):");
        println!(
            "  P50 (Median):   {:>10?}",
            Duration::from_nanos(hist.value_at_quantile(0.50))
        );
        println!(
            "  P95:            {:>10?}",
            Duration::from_nanos(hist.value_at_quantile(0.95))
        );
        println!(
            "  P99:            {:>10?}",
            Duration::from_nanos(hist.value_at_quantile(0.99))
        );
        println!(
            "  Max:            {:>10?}",
            Duration::from_nanos(hist.max())
        );
        println!("{}", "=".repeat(50));
    }
}
