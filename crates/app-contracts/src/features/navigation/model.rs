use crate::features::tabs::TabContextKey;
use app_core::actor::traits::Message;
use framework::uri::ContextlessAppUri;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageRouteDescriptor {
    pub segment: &'static str,
    pub layout: Option<&'static str>,
    pub features: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/navigation_routes.rs"));

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnownRouteDescriptor {
    pub uri: ContextlessAppUri,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NavigationProjectionChanged {
    pub known_routes: Vec<KnownRouteDescriptor>,
}

impl Message for NavigationProjectionChanged {}

pub fn canonical_route_path(context_key: &TabContextKey, route_segment: &str) -> String {
    format!("/{}/{}", context_key.0, route_segment)
}
