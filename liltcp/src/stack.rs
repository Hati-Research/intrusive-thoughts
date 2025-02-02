use core::cell::{RefCell, RefMut};

use smoltcp::iface::{Interface, SocketSet, SocketStorage};

pub struct InnerStack<'a> {
    sockets: SocketSet<'a>,
    interface: Interface,
}

impl<'a> InnerStack<'a> {
    pub fn new(storage: &'a mut [SocketStorage<'a>], interface: Interface) -> Self {
        Self {
            sockets: SocketSet::new(storage),
            interface,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Stack<'a> {
    inner: &'a RefCell<InnerStack<'a>>,
}

impl<'a> Stack<'a> {
    pub fn new(inner: &'a RefCell<InnerStack<'a>>) -> Self {
        Self { inner }
    }

    pub fn with<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce((&mut SocketSet<'a>, &mut Interface)) -> U,
    {
        let (mut interface, mut sockets) = RefMut::map_split(self.inner.borrow_mut(), |r| {
            (&mut r.interface, &mut r.sockets)
        });
        f((&mut sockets, &mut interface))
    }
}
