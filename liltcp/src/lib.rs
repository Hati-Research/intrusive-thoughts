#![no_main]
#![no_std]

pub mod smoltcp_lilos;
pub mod stack;
pub mod tcp;

use core::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
};

use cortex_m_semihosting::debug;

use defmt_rtt as _; // global logger

use lilos::time::PeriodicGate;
use stm32h7xx_hal::{
    self as _,
    gpio::{ErasedPin, Output},
}; // memory layout

use panic_probe as _;

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", COUNT.fetch_add(1, Ordering::Relaxed));

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cortex_m_rt::exception]
unsafe fn HardFault(_frame: &cortex_m_rt::ExceptionFrame) -> ! {
    loop {
        debug::exit(debug::EXIT_FAILURE);
    }
}

pub async fn led_task(mut led: ErasedPin<Output>) -> Infallible {
    let mut gate = PeriodicGate::from(lilos::time::Millis(500));
    loop {
        led.toggle();
        gate.next_time().await;
    }
}
