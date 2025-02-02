# Fully asynchronous TCP client

In the previous chapter, we managed to share a wrapper around `smoltcp` between tasks.
That means that we are now ready to separate polling the stack and handling sockets.

## Polling the stack

Let's start by implementing the stack polling.
There are two signals that should trigger polling:

1. The Ethernet interrupt
2. smoltcp's internal timers

<div class="warning">
There can be many more signals that could, in theory, improve performance -
such as triggering poll whenever a buffer is filled with data,
or whenever a new buffer is read, or written to the peripheral's descriptor ring.
However, adding these sources is out of scope for this tutorial.
In the case of the descriptor ring buffers, it'd require hacking the HAL itself.
</div>

As for signaling from the Ethernet interrupt, we can use `lilos`'s
`Notify` synchronization primitive.

```rust,ignored
{{#include ../../liltcp/src/bin/async_tcp.rs:irq_notify}}
```

We must declare it statically, so that it can be accessed from the interrupt handler.
Luckily, it has a `const` `new()` function, so nothing special needs to be done
to initialize it.

Now, whenever the interrupt handler is called, we can notify that something happened.

```rust,ignored
{{#include ../../liltcp/src/bin/async_tcp.rs:eth_irq}}
```

We can wait for the signal in our polling task using the `Notify::until_next` method.

Now, let's go back to the polling signaled by the smoltcp internal timers.
`smoltcp`'s `Interface` contains a mechanism of letting the polling code know
when it should be polled next or after how much time it should be polled next.
For the delaying of the polling, we can use `lilos::time::sleep_for` async function.
So, we now have two futures, we need to combine and whenever one of them
completes, we can poll the interface.
For this we can use the `select(A, B)` asynchronous function from
`embassy-futures`, which does exactly what we need,
receives two features and returns whenever one of the features resolves.

The whole polling task is in the following snippet.

```rust,ignored
{{#include ../../liltcp/src/bin/async_tcp.rs:net_task}}
```

Apart from just polling, it also handles the link state.

## Adding a TCP client socket

With polling out of the way, we can now focus on adding a task that will handle a
TCP connection.
What we want is to connect to a TCP server, and loopback the data the server sent us.
This time, let's start with the top-down approach and write the body of the task
first, without worrying about the implementation.

```rust,ignored
{{#include ../../liltcp/src/bin/async_tcp.rs:tcp_client_task}}
```

We can see, that first, we initialize the transmitting and receiving buffers.
Then we create a new socket on our stack and pass it the buffers.
`unsafe` here is unavoidable without a lot of code because `static mut`s are
inherently unsafe and will not even be possible in the future.

### Socket definition and initialization

Let's have a look at the socket definition and initialization.

```rust,ignored
{{#include ../../liltcp/src/tcp.rs:tcp_client}}
```

Here, the `TcpClient` struct contains the wrapper to our `Stack` and a
handle pointing to the `Stack`'s `SocketSet`.

```rust,ignored
{{#include ../../liltcp/src/tcp.rs:tcp_new}}
```

What happens here is wrapping the raw buffers into `smoltcp`'s ring buffers.
Then, a new socket is initialized with them and the socket is added
to the `Stack`'s `SocketSet`.
The `SocketSet::add` call returns a `SocketHandle`, which we can later use
to access the socket.

### Accessing the socket

The `TcpClient` is basically a wrapper around the `Stack` with a `SocketHandle`,
together forming a "wrapper" around `smoltcp::socket::tcp::Socket`,
which can be indirectly accessed with these two values.

That means that whenever we want to do something with the raw TCP socket,
we need to obtain a reference to it via a handle.

To do this, we can utilize a similar pattern as in the previous chapter with
the `Stack`.

```rust,ignored
{{#include ../../liltcp/src/tcp.rs:with}}
```

This way, when doing anything with the socket, we don't need to write
the boilerplate needed to access it via the `Stack` and `SocketHandle` combo.

### Connecting

Let's now connect to the server.
This will be the first async function utilizing `smoltcp`'s async support.

```rust,ignored
{{#include ../../liltcp/src/tcp.rs:connect}}
```

Here, we first, initiate the connecting process and then, we create
a future using the `poll_fn`.
The `poll_fn` creates a future, that upon being polled calls a closure returning
`core::task::Poll`, the closure also has access to `Future` `Context`,
meaning that we can register its `Waker` to the socket.

That means that after the connecting process is initiated, the closure is
called once and then whenever it is awaken by `smoltcp`.
In the body of the closure, the state of the socket is checked for possible failures,
or a success.
In the case, there is nothing yet to be done, it registers its waker to
the socket (this is done every time, because some executors may change
the waker over time).

> This is the working principle of all the async smoltcp glue code.

### Sending data

Sending data utilizes the same working principle as connecting.
When polled, it attempts to write as much data to the socket buffers as possible
and postpones its execution if the buffers are full.

```rust,ignored
{{#include ../../liltcp/src/tcp.rs:send}}
```

### Receiving data

Receiving the data is similar to send data.
When polled, it attempts to read some bytes, and when no_data is available,
it waits for next poll.

```rust,ignored
{{#include ../../liltcp/src/tcp.rs:recv}}
```

## Conclusion

And that is all there is to it. We now have a working async networking stack
with quite nice API.

The TCP socket is by no means complete, but adding more functionality to it
should not be much of a problem.
