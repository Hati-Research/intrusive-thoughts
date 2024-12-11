# liltcp

> liltcp's aim is to write minimal, yet functional and performant async smoltcp
wrapper on top of the stm32h7xx-hal crate and lilos.
This should serve as a basis for figuring out how-to glue those two together,
similarly to embassy-net, but much more simpler.

## Basic blinky

To validate our environment and other unpredictable things (cables, devkit, etc.),
we first define a basic hello world type program that blinks an LED,
but uses the critical parts of the software stack (stm32h7xx-hal and lilos).

```rust
{{#include ../../liltcp/src/bin/hello.rs}}
```

If the code works you should see a blinking LED (amber on the Nucleo devkit).

## Basic blocking example from the HAL
