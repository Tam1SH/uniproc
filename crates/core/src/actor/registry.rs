use crate::actor::addr::Addr;
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
pub struct ActorRegistry {
    actors: RwLock<HashMap<TypeId, Box<dyn Any>>>,
}

// HACK: this registry for test only
unsafe impl Send for ActorRegistry {}
unsafe impl Sync for ActorRegistry {}

impl ActorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<A: 'static>(&self, addr: Addr<A>) {
        let mut actors = self.actors.write();
        actors.insert(TypeId::of::<A>(), Box::new(addr));
    }

    pub fn get<A: 'static>(&self) -> Option<Addr<A>> {
        let actors = self.actors.read();
        actors
            .get(&TypeId::of::<A>())?
            .downcast_ref::<Addr<A>>()
            .cloned()
    }
}
