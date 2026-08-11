//! Bridge between the actor world (multi-threaded, tokio, everything `Send`)
//! and capnp-rpc (single-threaded, compio, everything `!Send`).
//!
//! A capnp-rpc client is `Rc`-backed and its `RpcSystem` driver task must stay
//! on the runtime that created it, so the session can neither be stored in an
//! actor nor moved into a `tokio::spawn`. Instead each connected agent owns a
//! dedicated OS thread running its own compio runtime; the only thing that
//! crosses the boundary is a plain `Send` request/reply pair over a channel.
//!
//! [`RpcHandle`] is what the actor holds in place of a client: `Send + Sync +
//! Clone`, and dropping the last one ends the loop, which drops the session
//! and unwinds the whole thread. That makes "drop the client" the single
//! disconnect path, same as before.

use anyhow::anyhow;
use futures::channel::oneshot;
use std::future::Future;
use tracing::debug;

/// One agent's RPC surface, as seen from *inside* its dedicated thread.
///
/// Neither `Session` nor the futures returned here need to be `Send` - they
/// never leave the thread that created them. Only `Request`/`Reply` cross it.
pub trait RpcService: 'static {
    /// The connected capnp client. `Clone` because each in-flight request gets
    /// its own handle so a slow call can't block the others.
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

// Derived `Clone` would demand `S: Clone`, which the marker types are not -
// only the channel is actually cloned.
impl<S: RpcService> Clone for RpcHandle<S> {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

impl<S: RpcService> RpcHandle<S> {
    /// Starts the agent's thread and waits for its session to come up.
    ///
    /// Resolves only once the connection *and* handshake have succeeded, so a
    /// returned handle is always usable - matching what the connection FSM
    /// expects from a `ConnectSucceeded`.
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

                    // A dropped receiver means the caller gave up (timed out,
                    // actor torn down) - close the session instead of serving
                    // a connection nobody is holding.
                    if ready_tx.send(Ok(())).is_err() {
                        debug!("[{}] connected, but the requester is gone", S::NAME);
                        return;
                    }

                    // `recv_async` ends when every `RpcHandle` clone is dropped,
                    // which is the intended shutdown signal.
                    while let Ok(envelope) = rx.recv_async().await {
                        let session = session.clone();
                        // Spawned, not awaited: a call that blocks for seconds
                        // on the agent side (a service restart, say) must not
                        // hold up pings behind it.
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

    /// How long a request may go unanswered before it is called a failure.
    ///
    /// There has to be a bound. A call whose reply never arrives leaves the
    /// caller awaiting forever, and the ping is the caller that matters: the
    /// actor keeps a `ping_in_flight` latch, so one ping that never returns
    /// stops every later ping - and the ping is the only thing that notices a
    /// dead agent. The result was an agent that had gone away hours ago,
    /// still reported as connected, with nothing in the log to say so.
    ///
    /// Generous on purpose: the agent answers a scan in milliseconds, and the
    /// slowest legitimate call (restarting a service) is still well inside
    /// this.
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
