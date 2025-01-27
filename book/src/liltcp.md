# liltcp

`liltcp` is a demo project concerned with developing a basic glue library for
connecting together `smoltcp`, HAL and and an async runtime.
The name is sort of a pun on both smoltcp and cliffle's lilos,
because both of these are used as a basis for the glue.
The goal of the project is to be able to produce a working yet very basic
alternative to `embassy-net`, therefore documenting how it works and how
to use `smoltcp`'s async capabilities.
To avoid depdency on `embassy` itself, `stm32h7xx-hal` is used as a HAL.

The demo project is developed for the STM32H743ZI Nucleo devkit,
but it should work with any other H7 board, providing pin mappings are corrected.

## Getting started

Before diving into developing the networking code,
let's first make a LED blinking smoke test.
This is just to make sure that the environment is set up correctly
and there are no broken things (devkit, cables, etc).
The smoke test also makes sure that we have `lilos` working together with the HAL.

The code below implements such a smoke test.

```rust
{{#include ../../liltcp/src/bin/hello.rs}}
```

First, it initializes the clock, then the GPIO.
These are initialized with functions created to allow for easier code sharing,
so these include more code than necessary.
Next, lilos initializes SYSTICK and spawns a LED blinking task.

The LED blinking task itself is pretty bare:

```rust
{{#include ../../liltcp/src/lib.rs:led_task}}
```

If everything went well you should see a blinking LED (amber on the Nucleo devkit).
We can now move to initializing the ethernet peripheral
to do some basic link state polling.
