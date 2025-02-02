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

## Configuring the IP address

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

## Network task

Now, that the preparations are out of the way, we can define our net_task.
This task will handle both polling of the stack and handling of TCP
(even though it will be simplified.

```rust,ignored
{{#include ../../liltcp/src/bin/polled_tcp.rs:net_task}}
```

First, we define buffers that the TCP socket will internally use.
These are defined as mutable statics, because they need to have the same
lifetime or outlive the `'a` lifetime defined for the `SocketSet`.
Next, we create a TCP socket and add it to our `SocketSet`.
This call gives us a handle that can be used to later access the socket through
the `SocketSet`.

Now, the polling itself takes place.
This is done in a loop with a labeled block called `'worker`.
First, we check that the link is UP, if it is not the case, let's just break the
`'worker` block.
If the link is UP, we poll the interface to check if there are any new data
to be processed by our socket.
When there are, we can access our socket using the aforementioned handle and we
can do operations with it.
In this case, we check if it is open, if it is not the case, we attempt to connect to a
remote endpoint and break the `'worker` block to let the interface be polled again.
On next polls, if the socket is open, we attempt to do a read and subsequently
a write.

In the case of completion of the `'worker` block or the block being interrupted
by the `break 'worker`, the task will sleep for a millisecond.

<div class="warning">
This implementation is not meant to showcase an implementation of a TCP socket.
Right now, there are many unhandled states and it is very likely that this
will panic if you look at it wrong.
<br>
<br>
Another big problem here is performance, the polling loop runs with a
fixed period of 1 ms.
</div>

## Spawning the network task

Now we can simply spawn our task and let it do the polling and TCP handling.

```rust,ignored
{{#include ../../liltcp/src/bin/polled_tcp.rs:spawn}}
```

## Conclusions

This solution is probably good enough for a simple tests, but apart from it not
being async, there is one big problem - adding the TCP handling will soon
become a hassle, with any addition.

This is caused by these factors:

- It is tightly coupled with smoltcp stack polls.
- Adding more sockets will clutter the code even more.
- Adding any kind of timeout would block the entire task,
or you'd need to implement some sort of a state machine that will handle
this - but this is what we want to use async for.

Let's now have a quick intermezzo concerning decoupling of polling
and socket handling. Let's share the smoltcp stack across tasks.
