use crate::navigation::{RouteActivated, RouteDeactivated};
use crate::uri::AppUri;

#[derive(Clone, Debug)]
pub struct FeatureContextState {
    pub window_id: usize,
    pub capability_name: &'static str,
}

impl FeatureContextState {
    pub fn new(window_id: usize, capability_name: &'static str) -> Self {
        Self {
            window_id,
            capability_name,
        }
    }

    pub fn handle_activation<'a>(&mut self, msg: &'a RouteActivated) -> Option<&'a AppUri> {
        if msg.window_id == self.window_id
            && msg
                .uri
                .base
                .capabilities
                .iter()
                .any(|f| f == self.capability_name)
        {
            Some(&msg.uri)
        } else {
            None
        }
    }

    pub fn handle_deactivation(&mut self, msg: &RouteDeactivated) -> bool {
        msg.window_id == self.window_id
            && msg
                .uri
                .base
                .capabilities
                .iter()
                .any(|f| f == self.capability_name)
    }
}
