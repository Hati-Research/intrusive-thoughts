#![no_main]
#![no_std]

use core::convert::Infallible;

use lilos::{
    exec::Interrupts,
    time::{Millis, PeriodicGate},
};
use liltcp as _;
use stm32h7xx_hal::{
    ethernet::{self, phy::LAN8742A, StationManagement, PHY as _},
    gpio::{ErasedPin, Output},
    interrupt, pac,
};

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

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

    // ANCHOR: eth_init
    let (_eth_dma, eth_mac) = ethernet::new(
        dp.ETHERNET_MAC,
        dp.ETHERNET_MTL,
        dp.ETHERNET_DMA,
        gpio.eth_pins,
        unsafe { liltcp::take_des_ring() },
        liltcp::MAC,
        ccdr.peripheral.ETH1MAC,
        &ccdr.clocks,
    );

    let mut lan8742a = ethernet::phy::LAN8742A::new(eth_mac.set_phy_addr(0));
    lan8742a.phy_reset();
    lan8742a.phy_init();
    // ANCHOR_END: eth_init

    lilos::time::initialize_sys_tick(&mut cp.SYST, ccdr.clocks.sysclk().to_Hz());

    // ANCHOR: spawn
    unsafe {
        liltcp::enable_eth_interrupt(&mut cp.NVIC);

        lilos::exec::run_tasks_with_preemption(
            &mut [
                core::pin::pin!(liltcp::led_task(gpio.led)),
                core::pin::pin!(poll_link(lan8742a, gpio.link_led)),
            ],
            lilos::exec::ALL_TASKS,
            Interrupts::Filtered(liltcp::NVIC_BASEPRI),
        );
    }
    // ANCHOR_END: spawn
}

// ANCHOR: poll_link
// Periodically poll if the link is up or down
async fn poll_link<MAC: StationManagement>(
    mut phy: LAN8742A<MAC>,
    mut link_led: ErasedPin<Output>,
) -> Infallible {
    let mut gate = PeriodicGate::from(Millis(1000));
    let mut eth_up = false;
    loop {
        gate.next_time().await;

        let eth_last = eth_up;
        eth_up = phy.poll_link();

        link_led.set_state(eth_up.into());

        if eth_up != eth_last {
            if eth_up {
                defmt::info!("UP");
            } else {
                defmt::info!("DOWN");
            }
        }
    }
}
// ANCHOR_END: poll_link

// ANCHOR: eth_irq
#[cortex_m_rt::interrupt]
fn ETH() {
    unsafe {
        ethernet::interrupt_handler();
    }
}
// ANCHOR_END: eth_irq
