use crate::actor::envelope::{Envelope, FnEnvelope, MessageEnvelope};
use crate::actor::event_bus::builder::EventSubscription;
use crate::actor::traits::{Handler, Message};
use crate::actor::{short_type_name, ManagedActor};
use crate::actor::{Context, UiThreadToken};
use crate::lifecycle_tracker::LifecycleTracker;
use crate::trace::{current_meta, is_message_enabled, is_scope_enabled, DispatchMeta};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
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
    pub fn new_managed(state: A, token: UiThreadToken, tracker: &impl LifecycleTracker) -> Self
    where
        A: ManagedActor,
    {
        let addr = Self::new(state, token, tracker);

        A::Bus::subscribe_into(addr.clone(), tracker);

        addr
    }

    pub fn new(state: A, guard: UiThreadToken, tracker: &impl LifecycleTracker) -> Self {
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

    pub fn apply<F>(&self, f: F)
    where
        F: FnOnce(&mut A, &Context<A>) + Send + 'static,
    {
        let meta =
            current_meta().unwrap_or_else(|| DispatchMeta::capture_or_root("core.actor.apply"));

        self.queue.borrow_mut().push_back(Box::new(FnEnvelope {
            func: Some(f),
            meta,
            phantom: PhantomData,
        }));

        self.process_queue();
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

    pub fn handler_with2<M, T1, T2, F>(&self, f: F) -> impl Fn(T1, T2) + 'static
    where
        F: Fn(T1, T2) -> M + 'static,
        M: Message,
        A: Handler<M>,
    {
        let addr = self.clone();
        move |arg1, arg2| addr.do_send(f(arg1, arg2))
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
        if is_scope_enabled("core.actor.send") && is_message_enabled(message_name) {
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
