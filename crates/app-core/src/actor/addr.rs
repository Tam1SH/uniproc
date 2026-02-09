use crate::actor::envelope::{Envelope, MessageEnvelope};
use crate::actor::traits::{Context, Handler, Message};
use slint::ComponentHandle;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    pub static REGISTRY: RefCell<HashMap<usize, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

pub struct Addr<A: 'static, TWindow: ComponentHandle + 'static> {
    pub(super) id: usize,
    state: Rc<RefCell<A>>,
    queue: Rc<RefCell<VecDeque<Box<dyn Envelope<A, TWindow>>>>>,
    is_processing: Rc<Cell<bool>>,
    ui_weak: slint::Weak<TWindow>,
}

impl<A: 'static, TWindow: ComponentHandle + 'static> Clone for Addr<A, TWindow> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            state: self.state.clone(),
            queue: self.queue.clone(),
            is_processing: self.is_processing.clone(),
            ui_weak: self.ui_weak.clone(),
        }
    }
}

impl<A: 'static, TWindow: ComponentHandle + 'static> Addr<A, TWindow> {
    pub fn handler<M>(&self, msg: M) -> impl Fn() + 'static
    where
        M: Message + Clone,
        A: Handler<M, TWindow>,
    {
        let addr = self.clone();
        move || addr.send(msg.clone())
    }

    pub fn handler_with<M, T, F>(&self, f: F) -> impl Fn(T) + 'static
    where
        F: Fn(T) -> M + 'static,
        M: Message,
        A: Handler<M, TWindow>,
    {
        let addr = self.clone();
        move |arg| addr.send(f(arg))
    }

    pub fn handler_scoped<M, T, F>(&self, f: F) -> impl Fn(T) + 'static
    where
        F: Fn(&Addr<A, TWindow>, T) -> M + 'static,
        M: Message,
        A: Handler<M, TWindow>,
    {
        let addr = self.clone();
        move |arg| {
            let msg = f(&addr, arg);
            addr.send(msg);
        }
    }

    pub fn scope<T, F>(&self, f: F) -> impl Fn(T) + 'static
    where
        F: Fn(&Addr<A, TWindow>, T) + 'static,
    {
        let addr = self.clone();
        move |arg| f(&addr, arg)
    }

    pub fn new(state: A, ui_weak: slint::Weak<TWindow>) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let addr = Self {
            id,
            state: Rc::new(RefCell::new(state)),
            queue: Rc::new(RefCell::new(VecDeque::new())),
            is_processing: Rc::new(Cell::new(false)),
            ui_weak,
        };

        let addr_clone = addr.clone();
        REGISTRY.with(|reg| {
            reg.borrow_mut().insert(id, Box::new(addr_clone));
        });

        addr
    }

    pub fn send<M>(&self, msg: M)
    where
        M: Message,
        A: Handler<M, TWindow>,
    {
        self.queue
            .borrow_mut()
            .push_back(Box::new(MessageEnvelope { message: Some(msg) }));

        self.process_queue();
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

            let ctx = Context {
                addr: self.clone(),
                ui_weak: self.ui_weak.clone(),
            };

            {
                let mut state_guard = self.state.borrow_mut();
                Envelope::<A, TWindow>::handle(envelope.as_mut(), &mut *state_guard, &ctx);
            }
        }

        self.is_processing.set(false);
    }
}
