use tracing_subscriber::filter::{LevelFilter, Targets};

pub fn init() -> anyhow::Result<()> {
    let targets = Targets::new()
        .with_default(LevelFilter::DEBUG)
        .with_target("ogurpchik", LevelFilter::WARN);

    guinea_trace::init_subscriber(std::io::stderr, 64, targets)
}
