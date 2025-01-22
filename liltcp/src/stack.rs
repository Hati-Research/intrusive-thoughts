use core::cell::{RefCell, RefMut};

use cortex_m::interrupt::Mutex;
use smoltcp::iface::{Interface, SocketSet};

// TODO: visibility
pub struct StackState<'a> {
    pub sockets: SocketSet<'a>,
    pub interface: Interface,
}

#[derive(Clone, Copy)]
pub struct Stack<'a> {
    pub inner: &'a Mutex<RefCell<StackState<'a>>>,
}

impl<'a> Stack<'a> {
    pub fn with<F, U>(&self, cs: &cortex_m::interrupt::CriticalSection, f: F) -> U
    where
        F: FnOnce((&mut SocketSet<'a>, &mut Interface)) -> U,
    {
        let (mut interface, mut sockets) =
            RefMut::map_split(self.inner.borrow(cs).borrow_mut(), |r| {
                (&mut r.interface, &mut r.sockets)
            });
        f((&mut sockets, &mut interface))
    }
}
