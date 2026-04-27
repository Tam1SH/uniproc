use crate::actor::event_bus::EventBus;
use crate::actor::traits::Message;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Clone)]
pub struct RpcRequest<T> {
    pub correlation_id: Uuid,
    pub payload: T,
}

impl<T: Message + Clone + Send> Message for RpcRequest<T> {}

#[derive(Clone)]
pub struct RpcResponse<T> {
    pub correlation_id: Uuid,
    pub payload: T,
}

impl<T: Message + Clone + Send> Message for RpcResponse<T> {}

pub trait RpcCall: Message + Clone + Send {
    type Response: Message + Clone + Send;
}

impl<Req: RpcCall> RpcRequest<Req> {
    pub fn reply(self, response: Req::Response) {
        AsyncBus::reply(self.correlation_id, response);
    }
}

static PENDING_REQUESTS: Lazy<RwLock<HashMap<Uuid, oneshot::Sender<Box<dyn Any + Send>>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub struct AsyncBus;

impl AsyncBus {
    pub async fn request<Req>(payload: Req, timeout: Duration) -> anyhow::Result<Req::Response>
    where
        Req: RpcCall,
    {
        let correlation_id = Uuid::new_v4();
        let envelope = RpcRequest {
            correlation_id,
            payload,
        };

        let (tx, rx) = oneshot::channel();
        PENDING_REQUESTS.write().insert(correlation_id, tx);

        EventBus::publish(envelope);

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(any_res)) => match any_res.downcast::<RpcResponse<Req::Response>>() {
                Ok(res) => Ok(res.payload),
                Err(_) => Err(anyhow::anyhow!("Type mismatch in async response")),
            },
            Ok(Err(_)) => Err(anyhow::anyhow!("Response channel closed")),
            Err(_) => {
                PENDING_REQUESTS.write().remove(&correlation_id);
                Err(anyhow::anyhow!("RPC request timed out"))
            }
        }
    }

    pub fn reply<Res>(correlation_id: Uuid, payload: Res)
    where
        Res: Message + Clone + Send,
    {
        let envelope = RpcResponse {
            correlation_id,
            payload,
        };

        if let Some(tx) = PENDING_REQUESTS.write().remove(&correlation_id) {
            let _ = tx.send(Box::new(envelope.clone()));
        }

        EventBus::publish(envelope);
    }
}

#[macro_export]
macro_rules! rpc_bind {
    ($( $req:ident => $res:ident );* $(;)?) => {
        $(
            impl $crate::actor::traits::Message for $req {}
            impl $crate::actor::traits::Message for $res {}

            impl $crate::actor::event_bus::rpc::RpcCall for $req {
                type Response = $res;
            }
        )*
    };
}
