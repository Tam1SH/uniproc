use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, amethystate::AmeType)]
pub struct ColumnConfig {
    pub width: u64,
    pub min_width: u64,
    pub visible: bool,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            width: 110,
            min_width: 80,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ProcessCategory {
    App,
    BackgroundMicrosoft,
    BackgroundThirdParty,
    WindowsService,
    WindowsKernel,
}

impl ProcessCategory {
    pub fn classify(
        has_visible_window: bool,
        is_kernel_process: bool,
        is_service: bool,
        is_microsoft_signed: bool,
    ) -> Self {
        if has_visible_window {
            Self::App
        } else if is_kernel_process {
            Self::WindowsKernel
        } else if is_service {
            Self::WindowsService
        } else if is_microsoft_signed {
            Self::BackgroundMicrosoft
        } else {
            Self::BackgroundThirdParty
        }
    }

    pub const ORDER: [Self; 5] = [
        Self::App,
        Self::BackgroundThirdParty,
        Self::BackgroundMicrosoft,
        Self::WindowsService,
        Self::WindowsKernel,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::BackgroundThirdParty => "background-third-party",
            Self::BackgroundMicrosoft => "background-microsoft",
            Self::WindowsService => "windows-service",
            Self::WindowsKernel => "windows-kernel",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ORDER.into_iter().find(|c| c.id() == id)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub display_name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub net_bytes: u64,
    pub exe_path: String,
    pub package_full_name: String,
    pub category: ProcessCategory,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct MachineSummary {
    pub cpu_percent: f32,
    pub cpu_current_mhz: u64,
    pub cpu_max_mhz: u64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
}
