use anyhow::anyhow;
use futures::channel::oneshot;
use std::future::Future;
use tracing::debug;

pub trait RpcService: 'static {
    type Session: Clone + 'static;
    type Request: Send + 'static;
    type Reply: Send + 'static;

    const NAME: &'static str;

    fn connect(timeout_secs: u64) -> impl Future<Output = anyhow::Result<Self::Session>>;

    fn dispatch(
        session: Self::Session,
        request: Self::Request,
    ) -> impl Future<Output = anyhow::Result<Self::Reply>>;
}

struct Envelope<S: RpcService> {
    request: S::Request,
    reply: oneshot::Sender<anyhow::Result<S::Reply>>,
}

pub struct RpcHandle<S: RpcService> {
    tx: flume::Sender<Envelope<S>>,
}

impl<S: RpcService> Clone for RpcHandle<S> {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

impl<S: RpcService> RpcHandle<S> {
    pub async fn connect(timeout_secs: u64) -> anyhow::Result<Self> {
        let (ready_tx, ready_rx) = oneshot::channel::<anyhow::Result<()>>();
        let (tx, rx) = flume::unbounded::<Envelope<S>>();

        std::thread::Builder::new()
            .name(format!("rpc-{}", S::NAME))
            .spawn(move || {
                let runtime = match compio::runtime::Runtime::new() {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        let _ = ready_tx.send(Err(anyhow!(
                            "[{}] failed to start the compio runtime: {err}",
                            S::NAME
                        )));
                        return;
                    }
                };

                runtime.block_on(async move {
                    let session = match S::connect(timeout_secs).await {
                        Ok(session) => session,
                        Err(err) => {
                            let _ = ready_tx.send(Err(err));
                            return;
                        }
                    };

                    if ready_tx.send(Ok(())).is_err() {
                        debug!("[{}] connected, but the requester is gone", S::NAME);
                        return;
                    }

                    while let Ok(envelope) = rx.recv_async().await {
                        let session = session.clone();
                        compio::runtime::spawn(async move {
                            let result = S::dispatch(session, envelope.request).await;
                            let _ = envelope.reply.send(result);
                        })
                        .detach();
                    }

                    debug!("[{}] all handles dropped, closing session", S::NAME);
                });
            })
            .map_err(|err| anyhow!("[{}] failed to spawn the rpc thread: {err}", S::NAME))?;

        ready_rx
            .await
            .map_err(|_| anyhow!("[{}] rpc thread died before connecting", S::NAME))??;

        Ok(Self { tx })
    }

    const CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    pub async fn call(&self, request: S::Request) -> anyhow::Result<S::Reply> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send_async(Envelope { request, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("[{}] rpc thread is gone", S::NAME))?;

        match tokio::time::timeout(Self::CALL_TIMEOUT, reply_rx).await {
            Ok(reply) => reply.map_err(|_| anyhow!("[{}] rpc thread dropped the request", S::NAME))?,
            Err(_) => Err(anyhow!(
                "[{}] no reply within {:?} - treating the session as dead",
                S::NAME,
                Self::CALL_TIMEOUT
            )),
        }
    }
}
