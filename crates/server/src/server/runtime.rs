use crate::server::utils::set_thread_high_priority;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use tracing::info;

type Job = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>;

struct CoreRuntime {
    tx: flume::Sender<Job>,
}

static POOL: OnceLock<Vec<CoreRuntime>> = OnceLock::new();

pub fn init() {
    if POOL.get().is_some() {
        return;
    }

    let num_cores = num_cpus::get();
    let mut runtimes = Vec::with_capacity(num_cores);

    for core_id in 0..num_cores {
        let (tx, rx) = flume::unbounded::<Job>();

        std::thread::spawn(move || {
            let _ = affinity::set_thread_affinity(&[core_id]);

            if core_id == 0 {
                set_thread_high_priority();
            }

            let runtime = compio::runtime::Runtime::new().expect("Runtime init failed");
            info!("Core {} runtime operational", core_id);

            runtime.block_on(async move {
                while let Ok(factory) = rx.recv_async().await {
                    let local_future = factory();

                    compio::runtime::spawn(local_future).detach();
                }
            });
        });

        runtimes.push(CoreRuntime { tx });
    }
    let _ = POOL.set(runtimes);
}

pub fn spawn_on<F, Fut>(core_id: usize, factory: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    let pool = POOL.get().expect("Call runtime::init() first");
    let core = &pool[core_id % pool.len()];

    let job = Box::new(move || Box::pin(factory()) as Pin<Box<dyn Future<Output = ()>>>);

    let _ = core.tx.send(job);
}

pub fn core_count() -> usize {
    POOL.get().map(|p| p.len()).unwrap_or(0)
}
