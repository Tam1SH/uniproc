use foundation_services::caches::{icons::IconProvider, strings::StringsProvider};
use slint::{Image, SharedString};

pub trait ProcessMetadata {
    fn clean_name(&self, raw_name: &str) -> SharedString;
    fn icon_by_path(&self, path: &str) -> Image;
}

pub struct ProcessMetadataService;

impl ProcessMetadata for ProcessMetadataService {
    fn clean_name(&self, raw_name: &str) -> SharedString {
        StringsProvider::global(|p| p.get_clean(raw_name))
    }

    fn icon_by_path(&self, path: &str) -> Image {
        IconProvider::global(|p| p.get_icon(path))
    }
}
