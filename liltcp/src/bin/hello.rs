#![no_main]
#![no_std]

use core::convert::Infallible;

use lilos::time::PeriodicGate;
use liltcp as _;
use stm32h7xx_hal::{
    gpio::{ErasedPin, Output},
    pac,
    prelude::*,
};

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let pwrcfg = dp.PWR.constrain().vos1().freeze();
    let ccdr = dp
        .RCC
        .constrain()
        .sys_ck(400.MHz())
        .freeze(pwrcfg, &dp.SYSCFG);

    defmt::warn!("Hello, world!");

    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);
    let led = gpioe.pe1.into_push_pull_output().erase();

    lilos::time::initialize_sys_tick(&mut cp.SYST, ccdr.clocks.sysclk().to_Hz());

    lilos::exec::run_tasks(
        &mut [core::pin::pin!(led_task(led))],
        lilos::exec::ALL_TASKS,
    )
}

async fn led_task(mut led: ErasedPin<Output>) -> Infallible {
    let mut gate = PeriodicGate::from(lilos::time::Millis(500));
    loop {
        led.toggle();
        gate.next_time().await;
    }
}
