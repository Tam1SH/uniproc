use crate::server::commands::Response;
// use crate::server::dispatcher::RequestHandler;
use std::sync::Arc;

pub struct WslService {
    host_name: String,
}

impl WslService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            host_name: "Windows-11-Host".to_string(),
        })
    }

    pub(crate) async fn handle_identity(&self) -> Response {
        Response::Identity(self.host_name.clone())
    }

    pub(crate) async fn handle_get_processes(&self, limit: u32) -> Response {
        let procs = (0..limit).collect();
        Response::ProcessList(procs)
    }

    pub(crate) async fn handle_kill(&self, pid: u32) -> Response {
        println!("Request to kill PID: {}", pid);
        if pid == 0 {
            Response::Error("Cannot kill system".into())
        } else {
            Response::Success
        }
    }
}

// impl RequestHandler for WslService {
//     async fn dispatch(&self, req: &ArchivedRequest) -> Response {
//         match req {
//             ArchivedRequest::GetIdentity => self.handle_identity().await,
//             ArchivedRequest::GetProcesses { limit } => {
//                 self.handle_get_processes(u32::from(*limit)).await
//             }
//             ArchivedRequest::KillProcess { pid } => self.handle_kill(u32::from(*pid)).await,
//
//             _ => Response::Error("Method not supported".into()),
//         }
//     }
// }
