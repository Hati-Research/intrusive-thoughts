# Initializing and polling the ethernet peripheral

At this point, we know that the devkit is able to run our code,
but it doesn't yet do anything network related, so let's change that.

First, we need to initialize the ethernet peripheral driver from the HAL.

```rust
{{#include ../../liltcp/src/bin/bare_eth.rs:eth_init}}
```

The initialization itself is pretty bare,
the only remotely interesting part is the initialization of the PHY
on the address 0.

The ethernet peripheral internally sets up DMA for receiving and transmitting data and lets the user know that something happened using an interrupt handler.

```rust
{{#include ../../liltcp/src/bin/bare_eth.rs:eth_irq}}
```

The interrupt must also be enabled in NVIC, which is done using the following function, called just before `lilos` spawns tasks.

```rust
{{#include ../../liltcp/src/lib.rs:enable_eth_interrupt}}
```

Once this is done, the peripheral is ready to send and receive data.
That, however, is a topic for the next chapter.
For now, we only want to check if the link is up.
This is done by polling the PHY.
Let's now add a new async task, which will periodically poll the PHY
and print the link state on change. To also see the link state on the devkit,
let's also turn the LED on, when the link is UP.

```rust
{{#include ../../liltcp/src/bin/bare_eth.rs:poll_link}}
```

The final thing left to do is to spawn the task and run the binary on our devkit.

```rust
{{#include ../../liltcp/src/bin/bare_eth.rs:spawn}}
```

When you plug in an ethernet cable, there should be a log visible
in the terminal and also an LED should light up.

We are now ready to move on to actually receiving and transmitting data via the ethernet.
