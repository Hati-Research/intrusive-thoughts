# Introduction

> I've been having some intrusive thoughts - that while async embedded Rust is great,
it could also be better, more transparent and best practices should be documented.

This book serves two main purposes:

* To demystify some parts of the current embedded Rust ecosystem and
provide example solution to some pain points that exist today.
* To serve as a notebook for my ideas. Note that these are just ideas,
not a definitive source of truth.
These ideas may be presented in a very raw form and important parts may be missing.

<div class="warning">
  Embedded Rust is a result of a work of many exremely talented and hardworking people.
  I have my utmost respect for them and for what they achieved.
  This book is not about complaining about problems of the ecosystem,
  but rather about providing some of the missing pieces.
</div>

My intrusive thoughts revolve around the following ideas (in no particular order):

* Tooling improvements - make common tasks easy (measure binary size, bss use),
crash inspections, logs inspections.
* Explanation of async on embedded by developing a simple async executor.
* Exploration of intrusive linked list as an alternative to static or fixed size allocation.
* Tracing for embedded async.
* Standardization of reading and writing of firmware metadata.
* Developing best practices for panic/hardfault handling and post-mortem debugging.
* Developing a limited example RP2350 HAL with primitives for more low level DMA
and drivers (something like lilos's Notify).

Some of the aforementioned rough edges IMHO are:

* It is unclear how to do some common things
(e.g. `static mut` handling, especially in the context of 2024 edition changes).
* Writing hardware independent/HAL independent drivers requires
a lot of "infectious" generics.
* HALs lock the users into a specific ways of using peripherals, because it is
often impractical to implement all of the peripheral IP features.
As a result of this, making highly special things is hard -
an example of this is abusing the double buffered DMA
to support reading from the DCMI peripheral on STM32 to allow for DMA reads
consisting of more than 65535 transfers.
* Debugging of why things don't work (for example even before defmt is available)
is not well documented.
