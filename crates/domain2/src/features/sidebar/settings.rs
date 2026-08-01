use amethystate::amethystate;

#[amethystate(prefix = "sidebar")]
pub struct SidebarSettings {
    #[amestate(default = 260u64)]
    pub width: u64,
    #[amestate(default = true)]
    pub open: bool,
}
