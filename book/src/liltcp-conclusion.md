# Conclusion

The goal of this tutorial was to explore the way to implement an
asynchronous networking stack and to show how `embassy-net` works under the hood.
Huge kudos to [@dirbaio](https://github.com/dirbaio) for all the work he did
to make this possible.

The tutorial went from a strictly blocking code up to a fully asynchronous TCP
client socket.
I did some measurements on its throughput and the maximum throughput
on the Nucleo devkit was around 8 Mbits,
`embassy-net` achieves 24 Mbits, which is likely due to polling each time
a buffer is dispatched through the peripheral.
Adding support for this would require significant changes to
the `stm32h7xx-hal` crate.

The whole source code for this tutorial is available in the [intrusive-thoughts repo](https://github.com/Hati-Research/intrusive-thoughts/tree/main/liltcp).
Don't hesitate to open any issues or post pull-requests with improvements.

It should be possible to make these wrappers HAL agnostic and have
an async stack that can be shared across many HALs, but that is out of scope
of this tutorial.
