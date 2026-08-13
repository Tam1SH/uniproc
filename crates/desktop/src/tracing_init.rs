use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::MakeWriter;

const LOG_FILE: &str = "run_desktop.log";

#[derive(Clone)]
struct StderrAndFile {
    file: Option<Arc<Mutex<File>>>,
}

struct Fanout {
    file: Option<Arc<Mutex<File>>>,
}

impl Write for Fanout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(file) = &self.file
            && let Ok(mut file) = file.lock()
        {
            let _ = file.write_all(buf);
        }
        io::stderr().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = &self.file
            && let Ok(mut file) = file.lock()
        {
            let _ = file.flush();
        }
        io::stderr().flush()
    }
}

impl<'a> MakeWriter<'a> for StderrAndFile {
    type Writer = Fanout;

    fn make_writer(&'a self) -> Self::Writer {
        Fanout {
            file: self.file.clone(),
        }
    }
}

pub fn init() -> anyhow::Result<()> {
    let targets = Targets::new()
        .with_default(LevelFilter::DEBUG)
        .with_target("ogurpchik", LevelFilter::WARN);

    let file = File::create(LOG_FILE).ok().map(|f| Arc::new(Mutex::new(f)));

    guinea_trace::init_subscriber(StderrAndFile { file }, 64, targets)
}
