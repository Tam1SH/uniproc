use app_core::actor::traits::Message;
use ogurpchik::align_buffer::AlignedBuffer;
use ogurpchik::client::Client;
use uniproc_protocol::HostCodec;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WslDistroDto {
    pub name: String,
    pub is_installed: bool,
    pub is_running: bool,
    pub latency_ms: i32,
}

pub type WslClient = Client<HostCodec, AlignedBuffer>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WslConnectionState {
    Disconnected,
    Connecting,
    Connected,
    WaitingRetry { delay_secs: u64 },
}

#[derive(Clone)]
pub struct WslAgentRuntimeEvent {
    pub state: WslConnectionState,
    pub client: Option<WslClient>,
}

impl Message for WslAgentRuntimeEvent {}

pub trait EnvironmentsUiPort: 'static {
    fn set_host_name(&self, name: String);
    fn set_host_icon_by_key(&self, icon_key: &str);
    fn set_selected_env(&self, name: String);
    fn set_wsl_distros(&self, distros: Vec<WslDistroDto>);
    fn set_has_wsl(&self, has_wsl: bool);
    fn set_wsl_is_loading(&self, loading: bool);
    fn set_wsl_distros_is_loading(&self, loading: bool);
}

pub trait EnvironmentsUiBindings: 'static {
    fn on_install_agent<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;
}
