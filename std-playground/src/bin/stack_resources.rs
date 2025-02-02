use core::{cell::RefCell, mem::MaybeUninit};

pub struct StackResources {
    inner: MaybeUninit<RefCell<InnerStack>>,
}

struct InnerStack {
    resource_a: i32,
}

struct Stack<'a> {
    inner: &'a RefCell<InnerStack>,
}

impl<'a> Stack<'a> {
    fn new(resources: &'a mut StackResources) -> Self {
        let inner = resources
            .inner
            .write(RefCell::new(InnerStack { resource_a: 42 }));
        Self { inner }
    }
    fn with<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce(&mut InnerStack) -> U,
    {
        f(&mut self.inner.borrow_mut())
    }
}

fn main() {}
