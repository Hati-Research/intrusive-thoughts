fn main() {
    println!("hello")
}

// hides interior mutability implementation detail (creation of the inner state), infects code with
// references
// Can't be copy
// PROs
// - hides interior mutability primitive (is this desired though? - embassy-mutex flexibility)
// CONs
// - can't have &mut self receiver
mod reference_outside {
    use std::cell::RefCell;

    struct Inner {
        a: i32,
    }

    struct Outer(RefCell<Inner>);

    impl Outer {
        fn new() -> Self {
            Self(RefCell::new(Inner { a: 0 }))
        }

        fn describe(&self) {
            println!("a: {}", self.0.borrow().a)
        }

        // Can't pass &mut
        fn modify(&self, a: i32) {
            self.0.borrow_mut().a = a;
        }
    }

    fn main() {
        let outer = Outer::new();
    }

    fn a(outer: &Outer) {
        outer.describe();
    }

    fn b(outer: &Outer) {
        outer.modify(1);
    }
}

// shows interior mutability implementation detail (creation of the inner state), infects code with
// lifetimes
// Is meant to be copied
// PROs
// - can have &mut self receiver - API shows intent better
// CONs
// - internal implementation detail is shown
mod reference_inside {
    use std::cell::RefCell;

    struct Inner {
        a: i32,
    }

    #[derive(Clone, Copy)]
    struct Outer<'a>(&'a RefCell<Inner>);

    impl<'a> Outer<'a> {
        fn new(inner: &'a RefCell<Inner>) -> Self {
            Self(inner)
        }

        fn describe(&self) {
            println!("a: {}", self.0.borrow().a)
        }

        fn modify(&mut self, a: i32) {
            self.0.borrow_mut().a = a;
        }
    }

    fn a(outer: Outer) {
        outer.describe();
    }

    fn b(mut outer: Outer) {
        outer.modify(1);
    }

    fn main() {
        let inner = RefCell::new(Inner { a: 0 });
        let outer = Outer::new(&inner);

        a(outer);
        b(outer);
    }
}

// Hides interior mutability implementation detail (creation of the inner state), infects code with
// lifetimes
//
// Is meant to be copied
//
// Makes need to allocate resources still visible
//
// PROs
// - can have &mut self receiver - API shows intent better
// - internal implementation detail is hidden
// - handling of init with multiple resources is easier
// - still shows that there is some shared state
// CONs
// - Boilerplate, that should be removable with a macro
mod reference_inside_hide_state {
    use std::{cell::RefCell, mem::MaybeUninit};

    struct Inner {
        a: i32,
    }

    struct OuterAllocations {
        inner: MaybeUninit<RefCell<Inner>>,
    }

    impl Default for OuterAllocations {
        fn default() -> Self {
            OuterAllocations {
                inner: MaybeUninit::uninit(),
            }
        }
    }

    #[derive(Clone, Copy)]
    struct Outer<'a> {
        inner: &'a RefCell<Inner>,
    }

    impl<'a> Outer<'a> {
        // &'a mut here makes sure that allocations is not used multiple times
        fn new(allocations: &'a mut OuterAllocations) -> Self {
            let inner = &*allocations.inner.write(RefCell::new(Inner { a: 0 }));

            Self { inner }
        }

        fn describe(&self) {
            println!("a: {}", self.inner.borrow().a)
        }

        fn modify(&mut self, a: i32) {
            self.inner.borrow_mut().a = a;
        }
    }

    fn a(outer: Outer) {
        outer.describe();
    }

    fn b(mut outer: Outer) {
        outer.modify(1);
    }

    fn main() {
        let mut allocations = OuterAllocations::default();
        let outer = Outer::new(&mut allocations);

        a(outer);
        b(outer);
    }
}
