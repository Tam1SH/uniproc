use amethystate::amethystate;

#[amethystate(prefix = "agents")]
pub struct AgentSettings {
    #[amestate(default = 8u64)]
    pub connect_timeout_secs: u64,

    #[amestate(default = 2000u64)]
    pub ping_interval_ms: u64,

    /// Far longer than [`connect_timeout_secs`]: we launch the WSL agent
    /// ourselves and it has to bring eBPF up before it starts listening, which
    /// measured ~30s on a warm VM. The shared 8s default never reached it.
    #[amestate(default = 90u64)]
    pub wsl_connect_timeout_secs: u64,

    /// WSL distribution hosting the Linux agent.
    #[amestate(default = "Ubuntu".to_string())]
    pub wsl_distro: String,

    /// Path to the agent binary *inside* the distribution. It needs
    /// `cap_bpf,cap_net_admin,cap_perfmon,cap_syslog+ep` set on it once
    /// (`setcap`), which is what lets us start it as a normal user instead of
    /// asking for a password on every launch.
    #[amestate(default = "/usr/local/bin/uniproc-agent".to_string())]
    pub wsl_agent_path: String,
}
