use app_core::actor::traits::Message;
use std::any::Any;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct OpenedWindow {
    pub key: String,
    pub data: Arc<dyn Any + Send + Sync>,
}

impl Message for OpenedWindow {}
