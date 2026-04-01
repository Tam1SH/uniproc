use context::caches::icons::IconRequest;
use context::caches::{icons::IconProvider, strings::StringsProvider};
use slint::{Image, SharedString};

pub struct ProcessMetadataService;

impl ProcessMetadataService {
    pub fn clean_name(&self, raw_name: &str) -> SharedString {
        StringsProvider::global().get_stripped(raw_name)
    }

    pub fn icon_by_path(&self, req: IconRequest) -> Image {
        IconProvider::global(|p| p.get_icon(req))
    }
}
