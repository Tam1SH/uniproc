use crate::features::processes::services::providers::{IconProvider, NameProvider};
use slint::{Image, SharedString};

pub trait ProcessMetadata {
    fn clean_name(&mut self, raw_name: &str) -> SharedString;
    fn icon_by_path(&mut self, path: &str) -> Image;
}

pub struct ProcessMetadataService {
    name_provider: NameProvider,
    icon_provider: IconProvider,
}

impl ProcessMetadataService {
    pub fn new() -> Self {
        Self {
            name_provider: NameProvider::new(),
            icon_provider: IconProvider::new(),
        }
    }
}

impl ProcessMetadata for ProcessMetadataService {
    fn clean_name(&mut self, raw_name: &str) -> SharedString {
        self.name_provider.get_clean(raw_name)
    }

    fn icon_by_path(&mut self, path: &str) -> Image {
        self.icon_provider.get_icon(path)
    }
}

