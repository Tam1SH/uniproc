use slint::Image;

pub struct Icons;

impl Icons {
    pub fn get(name: &str) -> Image {
        let bytes: &[u8] = match name {
            "app" => include_bytes!("../../slint-adapter/ui/assets/app.svg"),
            "arrow-up-filled" => {
                include_bytes!("../../slint-adapter/ui/assets/arrow-up-filled.svg")
            }
            "arrow-up-regular" => {
                include_bytes!("../../slint-adapter/ui/assets/arrow-up-regular.svg")
            }
            "coin" => include_bytes!("../../slint-adapter/ui/assets/coin.svg"),
            "dashed-settings" => {
                include_bytes!("../../slint-adapter/ui/assets/dashed-settings.svg")
            }
            "data-area" => include_bytes!("../../slint-adapter/ui/assets/data-area.svg"),
            "database" => include_bytes!("../../slint-adapter/ui/assets/database.svg"),
            "dismiss" => include_bytes!("../../slint-adapter/ui/assets/dismiss.svg"),
            "docker" => include_bytes!("../../slint-adapter/ui/assets/docker.svg"),
            "download-regular" => {
                include_bytes!("../../slint-adapter/ui/assets/download-regular.svg")
            }
            "extension" => include_bytes!("../../slint-adapter/ui/assets/extension.svg"),
            "folder" => include_bytes!("../../slint-adapter/ui/assets/folder.svg"),
            "info" => include_bytes!("../../slint-adapter/ui/assets/info.svg"),
            "layer-filled" => include_bytes!("../../slint-adapter/ui/assets/layer-filled.svg"),
            "layer-regular" => include_bytes!("../../slint-adapter/ui/assets/layer-regular.svg"),
            "linux" => include_bytes!("../../slint-adapter/ui/assets/linux.svg"),
            "maximize" => include_bytes!("../../slint-adapter/ui/assets/maximize.svg"),
            "minimize" => include_bytes!("../../slint-adapter/ui/assets/minimize.svg"),
            "new-task" => include_bytes!("../../slint-adapter/ui/assets/new-task.svg"),
            "people" => include_bytes!("../../slint-adapter/ui/assets/people.svg"),
            "proc-filled" => include_bytes!("../../slint-adapter/ui/assets/proc-filled.svg"),
            "proc-regular" => include_bytes!("../../slint-adapter/ui/assets/proc-regular.svg"),
            "prohibited" => include_bytes!("../../slint-adapter/ui/assets/prohibited.svg"),
            "pulse-filled" => include_bytes!("../../slint-adapter/ui/assets/pulse-filled.svg"),
            "pulse-regular" => include_bytes!("../../slint-adapter/ui/assets/pulse-regular.svg"),
            "puzzle" => include_bytes!("../../slint-adapter/ui/assets/puzzle.svg"),
            "restore" => include_bytes!("../../slint-adapter/ui/assets/restore.svg"),
            "search" => include_bytes!("../../slint-adapter/ui/assets/search.svg"),
            "settings-filled" => {
                include_bytes!("../../slint-adapter/ui/assets/settings-filled.svg")
            }
            "settings-regular" => {
                include_bytes!("../../slint-adapter/ui/assets/settings-regular.svg")
            }
            "settings" => include_bytes!("../../slint-adapter/ui/assets/settings.svg"),
            "spinner" => include_bytes!("../../slint-adapter/ui/assets/spinner.svg"),
            "statistics" => include_bytes!("../../slint-adapter/ui/assets/statistics.svg"),
            "terminate" => include_bytes!("../../slint-adapter/ui/assets/terminate.svg"),
            "ubuntu" => include_bytes!("../../slint-adapter/ui/assets/ubuntu.svg"),
            "uniproc-logo" => include_bytes!("../../slint-adapter/ui/assets/uniproc-logo.svg"),
            "windows-11" => include_bytes!("../../slint-adapter/ui/assets/windows-11.svg"),
            "windows" => include_bytes!("../../slint-adapter/ui/assets/windows.svg"),
            "wsl" => include_bytes!("../../slint-adapter/ui/assets/wsl.svg"),
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
