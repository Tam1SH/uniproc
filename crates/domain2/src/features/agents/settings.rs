use amethystate::amethystate;

#[amethystate(prefix = "agents")]
pub struct AgentSettings {
    #[amestate(default = 8u64)]
    pub connect_timeout_secs: u64,

    #[amestate(default = 2000u64)]
    pub ping_interval_ms: u64,
}
