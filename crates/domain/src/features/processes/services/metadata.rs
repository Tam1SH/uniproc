use crate::features::processes::services::providers::{IconProvider, NameProvider};
use slint::{Image, SharedString};
use std::time::Duration;

pub trait ProcessMetadata {
    fn clean_name(&mut self, raw_name: &str) -> SharedString;
    fn icon_by_path(&mut self, path: &str) -> Image;
}

pub struct ProcessMetadataService {
    name_provider: NameProvider,
    icon_provider: IconProvider,
}

impl ProcessMetadataService {
    pub fn new(name_ttl: Duration, icon_ttl: Duration) -> Self {
        Self {
            name_provider: NameProvider::new(name_ttl),
            icon_provider: IconProvider::new(icon_ttl),
        }
    }

    pub fn set_ttls(&mut self, name_ttl: Duration, icon_ttl: Duration) {
        self.name_provider.set_ttl(name_ttl);
        self.icon_provider.set_ttl(icon_ttl);
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
