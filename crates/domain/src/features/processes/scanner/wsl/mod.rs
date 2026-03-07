pub mod visitor;

use crate::features::processes::scanner::base::{ProcessScanner, ScanResult};
use crate::features::processes::scanner::wsl::visitor::WslScanResult;
use app_contracts::features::environments::WslClient;
use async_trait::async_trait;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use tracing::warn;
use uniproc_protocol::{HostRequest, HostResponse, MachineStats};

pub type SharedWslClient = Arc<RwLock<Option<WslClient>>>;

pub struct WslScanner {
    client: SharedWslClient,
}

impl WslScanner {
    pub fn new(client: SharedWslClient) -> Self {
        Self { client }
    }

    fn empty_result() -> Box<dyn ScanResult> {
        Box::new(WslScanResult {
            processes: Vec::new(),
            machine: MachineStats::default(),
        })
    }
}

#[async_trait]
impl ProcessScanner for WslScanner {
    fn schema_id(&self) -> &'static str {
        "wsl"
    }

    async fn scan(&mut self) -> Box<dyn ScanResult> {
        let client = self.client.read().ok().and_then(|guard| guard.clone());

        let Some(client) = client else {
            return Self::empty_result();
        };

        let response = match client.call(HostRequest::GetReport).await {
            Ok(response) => response,
            Err(err) => {
                warn!("WSL report request failed: {err}");
                return Self::empty_result();
            }
        };

        let decoded =
            match rkyv::deserialize::<HostResponse, rkyv::rancor::Error>(*response.deref()) {
                Ok(decoded) => decoded,
                Err(err) => {
                    warn!("WSL report decode failed: {err}");
                    return Self::empty_result();
                }
            };

        match decoded {
            HostResponse::Report(report) => Box::new(WslScanResult {
                processes: report.processes,
                machine: report.machine,
            }) as Box<dyn ScanResult>,
            HostResponse::Pong => Self::empty_result(),
        }
    }
}
