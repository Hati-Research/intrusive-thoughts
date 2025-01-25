#![no_main]
#![no_std]

use liltcp as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32h7xx_hal::pac::Peripherals::take().unwrap();

    let ccdr = liltcp::initialize_clock(dp.PWR, dp.RCC, &dp.SYSCFG);

    let gpio = liltcp::init_gpio(
        dp.GPIOA,
        ccdr.peripheral.GPIOA,
        dp.GPIOB,
        ccdr.peripheral.GPIOB,
        dp.GPIOC,
        ccdr.peripheral.GPIOC,
        dp.GPIOE,
        ccdr.peripheral.GPIOE,
        dp.GPIOG,
        ccdr.peripheral.GPIOG,
    );

    lilos::time::initialize_sys_tick(&mut cp.SYST, ccdr.clocks.sysclk().to_Hz());
    lilos::exec::run_tasks(
        &mut [core::pin::pin!(liltcp::led_task(gpio.led))],
        lilos::exec::ALL_TASKS,
    )
}
