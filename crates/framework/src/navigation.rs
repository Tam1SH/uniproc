use crate::uri::AppUri;
use crate::uri::ContextlessAppUri;
use app_core::actor::traits::Message;
use std::sync::RwLock;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RouteDeactivated {
    pub window_id: usize,
    pub uri: AppUri,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RouteActivated {
    pub window_id: usize,
    pub uri: AppUri,
}

impl Message for RouteActivated {}
impl Message for RouteDeactivated {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub uri: ContextlessAppUri,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteRegistrySnapshot {
    pub routes: Vec<Route>,
}

pub struct RouteRegistry {
    routes: RwLock<Vec<Route>>,
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
        }
    }

    pub fn replace_routes(&self, routes: Vec<Route>) {
        *self.routes.write().unwrap() = routes;
    }

    pub fn all(&self) -> Vec<Route> {
        self.routes.read().unwrap().clone()
    }

    pub fn find_by_page_key(&self, page_key: &str) -> Option<Route> {
        self.routes
            .read()
            .unwrap()
            .iter()
            .find(|route| route.uri.segment.as_ref() == page_key)
            .cloned()
    }

    pub fn snapshot(&self) -> RouteRegistrySnapshot {
        RouteRegistrySnapshot {
            routes: self.routes.read().unwrap().clone(),
        }
    }
}
