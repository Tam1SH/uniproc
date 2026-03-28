use app_core::actor::traits::Message;
use ogurpchik::codecs::base::HasAllocator;
use ogurpchik::codecs::base::MessageCodec;
use ogurpchik::high::client::Client;
use ogurpchik::pool::buf_guard::BufGuard;
use uniproc_protocol::{LinuxCodec, WindowsCodec};

type RpcClient<C> = Client<
    C,
    <C as MessageCodec>::Dest,
    BufGuard<<C as MessageCodec>::Dest, <<C as MessageCodec>::Dest as HasAllocator>::SharedAlloc>,
>;

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub type AgentClient = RpcClient<WindowsCodec>;
    } else {
        pub type AgentClient = RpcClient<LinuxCodec>;
    }
}

pub type WslClient = RpcClient<LinuxCodec>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WslDistroDto {
    pub name: String,
    pub is_installed: bool,
    pub is_running: bool,
    pub latency_ms: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentConnectionState {
    Disconnected,
    Connecting,
    Connected,
    WaitingRetry { delay_secs: u64 },
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub type WslConnectionState = AgentConnectionState;

        #[derive(Clone)]
        pub struct WslAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for WslAgentRuntimeEvent {}

        #[derive(Clone)]
        pub struct WindowsAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub client: Option<AgentClient>,
            pub latency_ms: Option<i32>,
        }
        impl Message for WindowsAgentRuntimeEvent {}
    } else {
        #[derive(Clone)]
        pub struct LinuxAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for LinuxAgentRuntimeEvent {}
    }
}

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
