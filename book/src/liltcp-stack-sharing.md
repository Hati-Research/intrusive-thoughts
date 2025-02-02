# Intermezzo - sharing smoltcp stack between tasks

Sharing data between tasks is usually dependent on the executor and other environment.
For example in embassy, sharing can be done with references with a static
lifetime, since tasks are allocated in statics.
In the `std` environment, you'd typically used something like an `Arc`.

In our environment (`lilos` executor), tasks are allocated on the stack.
This means that for sharing data, we don't need to use references with a static
lifetime, but with a generic lifetime.
This is important, as we don't have to deal with either `static mut`s or
initialization of statics with local data.

A simple example of this can be seen in the following snippet.

```rust,ignored
fn main() -> ! {
    let shared_resource = 0;

    lilos::run_tasks(
        &mut [
            pin!(task_a(&shared_resource)),
            pin!(task_b(&shared_resource)),
        ] 
    )
}

async fn task_a(res: &i32) -> Infallible { .. }
async fn task_b(res: &i32) -> Infallible { .. }
```

## Mutating the shared resources

This basically solves the problem of sharing data between tasks,
but one problem still remains - how can we mutate the shared data?
We can't have multiple mutable references at the same time, so we need to
utilize some kind of interior mutability pattern.
This is usually done with the `Cell` or `RefCell` types.
`Cell` is not very useful for our use case, since it provides mutability by
moving in and out of it.
`RefCell` is much more interesting, because it allows us to obtain mutable and
immutable references to our data.
Without going into much detail, `RefCell` basically implements the borrow checker
and its rules in the runtime, instead of compile time.

<div class="warning">
In embedded systems, there is one more thing we care about and that is sharing
data between our tasks and interrupt handlers.
This is usually done by using something along the lines of Mutexes that protect
data access using a critical sections.
This has been ommitted here on purpose, since our system doesn't require it.
</div>

When we wrap our shared resource with `RefCell`, our example code will look
like the following snippet.

```rust,ignored
fn main() -> ! {
    let shared_resource = RefCell::new(i32);

    lilos::run_tasks(
        &mut [
            pin!(task_a(&shared_resource)),
            pin!(task_b(&shared_resource)),
        ] 
    )
}

async fn task_a(res: &RefCell<i32> -> Infallible { .. }
async fn task_b(res: &RefCell<i32> -> Infallible { .. }
```

Now, when we want to access some data in a task we can do:

```rust,ignored
async fn task_a(res: &RefCell<i32>) -> Infallible {
  {
    let r = res.borrow_mut();
    *r += 1;
  }
  yield_cpu().await;
}
```

Notice, that the shared reference access is done in a block.
That is to assure that the `r` which is actually a "smart" pointer
to the underlying data is dropped before we yield control to the executor.
If it weren't dropped before the yield (actually any `await` point),
the code would crash upon obtaining another mutable borrow from the `RefCell`.

## Hiding the implementation details and providing a nice API

This code is quite good until there are more shared resources, or the need
arises to implement methods on the shared resource.
Ideally, we'd like to be able to wrap the shared state into a structure and not
expose the implementation detail of the shared reference and interior mutability.

The approach I have chosen for this is to create a wrapper around the shared reference.
Until we add some more fields to the wrapper, it will be trivially copyable -
meaning it can be passed into as many tasks as required and using it, we can
make a nice API, that hides the aforementioned implementation detail.
This pattern is generally used, `embassy-net`, which this tutorial is based on,
also uses it.

Let's implement it:

We'll define our shared state as `InnerStack` struct.

```rust,ignored
pub struct InnerStack {
  // stack fields
}
```

Now, let's create a wrapper struct that we'll implement our API on.

```rust,ignored
pub struct Stack<'a> {
  pub inner: &'a RefCell<InnerStack>,
}
```

We want to avoid handling the `RefCell` in every function call, so let's create
an accessor function.

```rust,ignored
impl<'a> Stack<'a> {
    pub fn with<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce(&mut InnerStack) -> U,
    {
        f(&mut self.inner.borrow_mut())
    }
}
```

Now, we can implement methods on the `Stack` that look like this:

```rust, ignored
impl<'a> Stack<'a> {
  pub fn poll(&mut self) -> bool {
    self.with(|stack| stack.poll())
  }
}
```

Which is much more readable, hides the `RefCell` and most importantly limits
the scope of the `RefCell` borrows.

## Sharing a smoltcp stack

This implementation works for the simpler cases, but there is a problem with
smoltcp: for some calls, you need to have mutable references to two fields of
the `InnerStack` - to the `SocketStorage` and to the `Interface`.

This seems simple at first, but is a bit involved as it goes against the borrow
checker's rules on mutable borrows.
Trying it out is left as an exercise to the reader.

The solution to this is to use the `RefMut::map_split` function to effectively
split one `RefMut` into two `RefMut`s.

Combining all the above together and modifying it to fit the needs of
a `smoltcp` wrapper, we get the following code.

```rust,ignored
{{#include ../../liltcp/src/stack.rs}}
```

## Cleaning up the API

The code now implements everything we need from it, but still has a problem that
we are leaking the information about the `RefCell` to the creator of the stack,
which in turn requires us to make the `InnerStack` public.

A possible solution to this is the following:

```rust,ignored
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
}
```

This code is a heavily distilled solution of how `embassy-net` does this.
You can find the original solution [here](https://github.com/embassy-rs/embassy/blob/ae5ad91bbb6a158971c858f69ad25ca86025f2be/embassy-net/src/lib.rs#L289).

<div class="warning">
This approach will not be used in the remainder of the tutorial
because I believe it complicates things and doesn't add much value
to the goal of the tutorial,
which is to write an async glue between smoltcp and any HAL.
</div>

Having this out of the way, we can now finally go and implement an asynchronous
TCP socket.
