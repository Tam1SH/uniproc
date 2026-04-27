use super::{Addr, Handler, Message};

pub struct UiBinder<'p, A: 'static, P> {
    addr: Addr<A>,
    port: &'p P,
}

impl<'p, A: 'static, P> UiBinder<'p, A, P> {
    pub fn new(addr: &Addr<A>, port: &'p P) -> Self {
        Self {
            addr: addr.clone(),
            port,
        }
    }

    pub fn on0<M>(self, reg: impl FnOnce(&P, Box<dyn Fn() + 'static>), msg: M) -> Self
    where
        M: Message + Clone,
        A: Handler<M>,
    {
        let a = self.addr.clone();
        reg(self.port, Box::new(move || a.send(msg.clone())));
        self
    }

    pub fn on1<M, T: 'static>(
        self,
        reg: impl FnOnce(&P, Box<dyn Fn(T) + 'static>),
        ctor: impl Fn(T) -> M + 'static,
    ) -> Self
    where
        M: Message,
        A: Handler<M>,
    {
        reg(self.port, Box::new(self.addr.handler_with(ctor)));
        self
    }

    pub fn on2<M, T1: 'static, T2: 'static>(
        self,
        reg: impl FnOnce(&P, Box<dyn Fn(T1, T2) + 'static>),
        ctor: impl Fn(T1, T2) -> M + 'static,
    ) -> Self
    where
        M: Message,
        A: Handler<M>,
    {
        reg(self.port, Box::new(self.addr.handler_with2(ctor)));
        self
    }

    pub fn raw(self, f: impl FnOnce(&Addr<A>, &P)) -> Self {
        f(&self.addr, self.port);
        self
    }
}
