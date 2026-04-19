use app_core::actor::traits::Message;

#[derive(Clone, Debug)]
pub struct RequestTransition {
    pub from_index: i32,
    pub to_index: i32,
}
impl Message for RequestTransition {}
