// AUTO-GENERATED — do not edit manually

use slint::Image;

pub struct Icons;

impl Icons {
    pub fn get(name: &str) -> Image {
        let bytes: &[u8] = match name {
            "app" => include_bytes!("../../ui/assets/app.svg"),
            "arrow-up-filled" => include_bytes!("../../ui/assets/arrow-up-filled.svg"),
            "arrow-up-regular" => include_bytes!("../../ui/assets/arrow-up-regular.svg"),
            "coin" => include_bytes!("../../ui/assets/coin.svg"),
            "dashed-settings" => include_bytes!("../../ui/assets/dashed-settings.svg"),
            "data-area" => include_bytes!("../../ui/assets/data-area.svg"),
            "database" => include_bytes!("../../ui/assets/database.svg"),
            "dismiss" => include_bytes!("../../ui/assets/dismiss.svg"),
            "docker" => include_bytes!("../../ui/assets/docker.svg"),
            "download-regular" => include_bytes!("../../ui/assets/download-regular.svg"),
            "extension" => include_bytes!("../../ui/assets/extension.svg"),
            "folder" => include_bytes!("../../ui/assets/folder.svg"),
            "info" => include_bytes!("../../ui/assets/info.svg"),
            "layer-filled" => include_bytes!("../../ui/assets/layer-filled.svg"),
            "layer-regular" => include_bytes!("../../ui/assets/layer-regular.svg"),
            "linux" => include_bytes!("../../ui/assets/linux.svg"),
            "maximize" => include_bytes!("../../ui/assets/maximize.svg"),
            "minimize" => include_bytes!("../../ui/assets/minimize.svg"),
            "new-task" => include_bytes!("../../ui/assets/new-task.svg"),
            "people" => include_bytes!("../../ui/assets/people.svg"),
            "proc-filled" => include_bytes!("../../ui/assets/proc-filled.svg"),
            "proc-regular" => include_bytes!("../../ui/assets/proc-regular.svg"),
            "prohibited" => include_bytes!("../../ui/assets/prohibited.svg"),
            "pulse-filled" => include_bytes!("../../ui/assets/pulse-filled.svg"),
            "pulse-regular" => include_bytes!("../../ui/assets/pulse-regular.svg"),
            "puzzle" => include_bytes!("../../ui/assets/puzzle.svg"),
            "restore" => include_bytes!("../../ui/assets/restore.svg"),
            "search" => include_bytes!("../../ui/assets/search.svg"),
            "settings-filled" => include_bytes!("../../ui/assets/settings-filled.svg"),
            "settings-regular" => include_bytes!("../../ui/assets/settings-regular.svg"),
            "settings" => include_bytes!("../../ui/assets/settings.svg"),
            "spinner" => include_bytes!("../../ui/assets/spinner.svg"),
            "statistics" => include_bytes!("../../ui/assets/statistics.svg"),
            "terminate" => include_bytes!("../../ui/assets/terminate.svg"),
            "ubuntu" => include_bytes!("../../ui/assets/ubuntu.svg"),
            "uniproc-logo" => include_bytes!("../../ui/assets/uniproc-logo.svg"),
            "windows-11" => include_bytes!("../../ui/assets/windows-11.svg"),
            "windows" => include_bytes!("../../ui/assets/windows.svg"),
            "wsl" => include_bytes!("../../ui/assets/wsl.svg"),
            _ => {
                tracing::warn!(target: "internal", "Unknown icon: {name}");
                return Image::default();
            }
        };

        Image::load_from_svg_data(bytes).unwrap_or_else(|e| {
            tracing::error!(target: "internal", "Failed to decode icon '{name}': {e}");
            Image::default()
        })
    }
}
