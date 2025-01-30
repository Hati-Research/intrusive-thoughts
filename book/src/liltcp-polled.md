# Polled TCP

When developing a classic embedded Rust application that uses smoltcp for networking
(either using RTIC or no executor at all),
a common way to do that is to handle networking as part of the ethernet interrupt.
This has a few problems:

- Dependencies to the interrupt have to be declared as global statics.
- The IRQ must never block.
- It is harder to add another source of forcing the stack polling.
- It is up to the developer to handle the state machine properly.
  (This will be solved in the next chapter with async.)

Let's try to solve the first two problems by adding a simple async task, which will periodically poll the `smoltcp` interface and handle a TCP client.

For reference, an example of an RTIC example can be found [here](https://github.com/stm32-rs/stm32-eth/blob/master/examples/rtic-echo.rs).

At this point, we will be using the network layer, so the first thing we need
to do is to configure an IP address for our smoltcp interface.

```rust,ignored
{{#include ../../liltcp/src/bin/polled_tcp.rs:interface_init}}
```

The IP address and PREFIX_LEN are defined in the `lib.rs` as follows:

```rust,ignored
{{#include ../../liltcp/src/lib.rs:ip_address_constants}}
```

In theory, it should be possible to initialize the whole CIDR address
in a single constant, but the patch has only landed recently
and is not released yet.

Another thing included in the snippet is allocation of a `SocketStorage`
and a `SocketSet`, which is `smoltcp`'s way of storing active sockets.
In this case, we will add only one socket, so the storage array length will be `1`.

---

This example is a bit more elaborate and its source can be found [here](https://github.com/Hati-Research/intrusive-thoughts/blob/main/liltcp/src/bin/polled_tcp.rs).

Let's describe the important parts here.

First, we need to initialize the interface.
As can be seen, an IP address is set to the interface, according to your local network settings.
Then we statically allocate a `SocketStorage` with a fixed capacity of `1`,
since we will be adding only one socket for now.

```rust,ignored
{{#include ../../liltcp/src/bin/polled_tcp.rs:interface_init}}
```

Next, we want to add a task that will handle the polling:

```rust,ignored
{{#include ../../liltcp/src/bin/polled_tcp.rs:poll_smoltcp}}
```

This task first, allocates a TCP socket in the provided `SocketSet`,
then attempts to connect (if socket is not open)
and then tries to send and receive data).

The whole polling runs with a millisecond loop.
This is definitely not performat, we want the polling to be triggered by
either the ethernet interrupt or when `smoltcp` tells us to via its `poll_at` method.

Finally, we spawn the task in `main` using the following.

```rust, ignored
{{#include ../../liltcp/src/bin/polled_tcp.rs:spawn}}
```

We now have a TCP client that is able to connect to a remote server
and it works for the most basic of use cases.
Apart from the mentioned performance shortcomings,
it is also tiresome to add more sockets or their handling.
We'd need to handle everything networking-related in this task,
which would not be very readable
and would break both the Locality of Behavior and Separation of Concers principles,
that we want to uphold in all of our code.

> The functionality was tested using a simple program found [here](https://github.com/Hati-Research/intrusive-thoughts/blob/main/test-tcp-server/src/main.rs).

### Splitting polling and socket access

Let's now try to split polling and accessing the sockets themselves,
so that we can access sockets without having to incorporate them into
the state machine in the poll_smolltcp task.

We start by creating a new `poll_socket` and the first problem we encounter is trying to share the socket storage between the two tasks because they both take a mutable reference. Let's work around this for now using a mutex in a combo with `RefCell`.

Once we finish fighting the borrow checker on this front,
we figure out that another thing that needs sharing like this is the `interface`.
Right now, whe have two values that need to be accessed while holding a mutex,
so to make things simple, let's group them into a `StackState` struct.
This brings anothe advantage, which is the possibility to write member methods
for this struct, making the code a bit more readable.
Again, mainly for better readability, let's typealias this type:
`Mutex<RefCell<StackState<'a>>>` to just `SharedStackState<'a>`.
