use crate::actor::UiThreadToken;
use crate::actor::envelope::{Envelope, MessageEnvelope};
use crate::actor::traits::{Context, Handler, Message};
use crate::actor::{short_type_name, should_trace_actor_message};
use crate::lifecycle_tracker::FeatureLifecycle;
use crate::trace::{DispatchMeta, current_meta, is_message_enabled, is_scope_enabled};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    pub static REGISTRY: RefCell<HashMap<usize, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

pub struct Addr<A: 'static> {
    pub(super) id: usize,
    pub(super) guard: UiThreadToken,
    state: Rc<RefCell<A>>,
    queue: Rc<RefCell<VecDeque<Box<dyn Envelope<A>>>>>,
    is_processing: Rc<Cell<bool>>,
    counter: Rc<&'static str>,
}

impl<A: 'static> Clone for Addr<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            state: self.state.clone(),
            guard: self.guard.clone(),
            queue: self.queue.clone(),
            is_processing: self.is_processing.clone(),
            counter: self.counter.clone(),
        }
    }
}

impl<A: 'static> Addr<A> {
    pub fn new(state: A, guard: UiThreadToken, tracker: &FeatureLifecycle) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let addr = Self {
            id,
            guard,
            state: Rc::new(RefCell::new(state)),
            queue: Rc::new(RefCell::new(VecDeque::new())),
            is_processing: Rc::new(Cell::new(false)),
            counter: Rc::new(short_type_name::<A>()),
        };

        let addr_clone = addr.clone();
        REGISTRY.with(|reg| {
            reg.borrow_mut().insert(id, Box::new(addr_clone));
        });

        tracker.track_actor(&addr);
        addr
    }

    pub fn handler<M>(&self, msg: M) -> impl Fn() + 'static
    where
        M: Message + Clone,
        A: Handler<M>,
    {
        let addr = self.clone();
        move || addr.do_send(msg.clone())
    }

    pub fn handler_with<M, T, F>(&self, f: F) -> impl Fn(T) + 'static
    where
        F: Fn(T) -> M + 'static,
        M: Message,
        A: Handler<M>,
    {
        let addr = self.clone();
        move |arg| addr.do_send(f(arg))
    }

    pub fn send<M>(&self, msg: M)
    where
        M: Message,
        A: Handler<M>,
    {
        self.do_send(msg);
    }

    #[cfg(feature = "test-utils")]
    pub fn send_test<M>(&self, msg: M) -> crate::test_kit::Interaction<()>
    where
        M: Message,
        A: Handler<M>,
    {
        self.do_send(msg);
        crate::test_kit::Interaction::new(())
    }

    fn do_send<M>(&self, msg: M)
    where
        M: Message,
        A: Handler<M>,
    {
        let meta =
            current_meta().unwrap_or_else(|| DispatchMeta::capture_or_root("core.actor.send"));
        self.send_with_meta(msg, meta);
    }

    pub(crate) fn send_with_meta<M>(&self, msg: M, meta: DispatchMeta)
    where
        M: Message,
        A: Handler<M>,
    {
        let message_name = short_type_name::<M>();
        if is_scope_enabled("core.actor.send")
            && should_trace_actor_message(message_name)
            && is_message_enabled(message_name)
        {
            tracing::debug!(
                parent: &meta.span,
                actor = short_type_name::<A>(),
                message = message_name,
                op_id = meta.op_id,
                correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                "actor.send"
            );
        }

        self.queue.borrow_mut().push_back(Box::new(MessageEnvelope {
            message: Some(msg),
            meta,
        }));

        self.process_queue();
    }

    pub fn get_token(&self) -> UiThreadToken {
        self.guard.clone()
    }
    pub fn strong_count_ptr(&self) -> Rc<&'static str> {
        self.counter.clone()
    }

    fn process_queue(&self) {
        if self.is_processing.get() {
            return;
        }
        self.is_processing.set(true);

        loop {
            let mut envelope = {
                let mut q = self.queue.borrow_mut();
                match q.pop_front() {
                    Some(e) => e,
                    None => {
                        self.is_processing.set(false);
                        break;
                    }
                }
            };

            let ctx = Context { addr: self.clone() };

            {
                let mut state_guard = self.state.borrow_mut();
                Envelope::<A>::handle(envelope.as_mut(), &mut *state_guard, &ctx);
            }
        }

        self.is_processing.set(false);
    }
}
