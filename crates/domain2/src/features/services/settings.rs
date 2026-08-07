use amethystate::amethystate;

#[amethystate(prefix = "services")]
pub struct ServicesSettings {
    #[amestate(default = 2000u64)]
    scan_interval_ms: u64,
}
