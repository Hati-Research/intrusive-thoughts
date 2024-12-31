# Introduction

> I've been having some intrusive thoughts - that while async embedded Rust is great,
it could also be better, more transparent and best practices should be documented.

<div class="warning">
    This is more of a notebook for my ideas
    and their exploration than a source of truth.
    The ideas presented here may be in a very raw form.
    In no way are they meant to be insulting to anyone or offensive.</br>
    This project builds on top of the work of many others,
    the author will try to attribute them whenever needed,
    but feel free to let them know if something is wrong.
</div>

The intrusive thoughts revolved around the following ideas:

* Improve tooling - make common tasks easy (measure binary size, bss use),
crash inspections, logs inspections
* develop RP2350 HAL with primitives for more low level DMA and drivers
(something like lilos's Notify)
* develop a simple async executor
* try to utilize intrusive linked lists instead of static allocations
* formalize a way to do tracing
* standardized firmware metadata storage
* add best practices for panic/hardfault handling and post-mortem debugging

Embedded Rust is great and is result of the work of many awesome
and talented people, but there are some rough edges:

* it is unclear how to do things
* too many generics, especially when trying to achieve independence on hardware
* embassy hals are too high level
  (writing driver for DCMI should be possible with double buffering, without hacking the HAL itself)
* It is unclear why things go wrong (panic handling, hardfault handling, task monitoring)
* post mortem debugging is difficult (maybe even unexplored)
