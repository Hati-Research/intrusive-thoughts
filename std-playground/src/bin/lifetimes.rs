use std::ptr::addr_of;

mod not_working {
    pub struct Queue<'storage>(pub &'storage mut ());

    pub struct Pusher<'storage>(&'storage Queue<'storage>);
    pub struct Popper<'storage>(&'storage Queue<'storage>);
    impl<'storage> Queue<'storage> {
        pub fn split(&'storage mut self) -> (Pusher<'storage>, Popper<'storage>) {
            (Pusher(self), Popper(self))
        }
    }

    impl<'storage> Drop for Queue<'storage> {
        fn drop(&mut self) {
            println!("drop")
        }
    }

    fn split<'storage>(q: &'storage mut Queue<'storage>) -> (Pusher<'storage>, Popper<'storage>) {
        let (peepee, poopoo) = q.split();

        (peepee, poopoo)
    }

    pub fn run() -> ! {
        let mut storage = ();
        let mut queue = Queue(&mut storage);
        //let (pp, poo) = queue.split();
        //let (peepee, poopoo) = split(&mut queue);

        loop {}
    }
}

mod not_working_fix {
    pub struct Queue<'storage>(pub &'storage mut ());

    pub struct Pusher<'split, 'storage>(&'split Queue<'storage>);
    pub struct Popper<'split, 'storage>(&'split Queue<'storage>);
    impl<'split, 'storage> Queue<'storage> {
        pub fn split(&'split mut self) -> (Pusher<'split, 'storage>, Popper<'split, 'storage>) {
            (Pusher(self), Popper(self))
        }
    }

    impl<'storage> Drop for Queue<'storage> {
        fn drop(&mut self) {
            println!("drop")
        }
    }

    fn split<'split, 'storage>(
        q: &'split mut Queue<'storage>,
    ) -> (Pusher<'split, 'storage>, Popper<'split, 'storage>) {
        let (peepee, poopoo) = q.split();

        (peepee, poopoo)
    }

    pub fn run() -> ! {
        let mut storage = ();
        let mut queue = Queue(&mut storage);
        let (peepee, poopoo) = split(&mut queue);

        loop {}
    }
}
// Clone, Copy: Clone

struct A {
    a: [u8; 1024 * 1024],
}

impl Clone for A {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for A {}

fn b(a: A) {
    dbg!(addr_of!(a));
}

fn main() {
    //not_working::run();
    let a = A {
        a: [0; 1024 * 1024],
    };
    dbg!(addr_of!(a));

    b(a);
}
