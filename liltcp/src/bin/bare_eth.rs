#![no_main]
#![no_std]

use core::{convert::Infallible, mem::MaybeUninit};

use lilos::{
    exec::Interrupts,
    time::{Millis, PeriodicGate},
};
use liltcp as _;
use stm32h7xx_hal::{
    ethernet::{self, phy::LAN8742A, StationManagement, PHY as _},
    gpio::{ErasedPin, Output},
    interrupt, pac,
    prelude::*,
    stm32,
};

#[link_section = ".sram3.eth"]
static mut DES_RING: MaybeUninit<ethernet::DesRing<4, 4>> = MaybeUninit::uninit();

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let pwrcfg = dp.PWR.constrain().vos1().freeze();

    dp.RCC.ahb2enr.modify(|_, w| w.sram3en().enabled());

    let ccdr = dp
        .RCC
        .constrain()
        .sys_ck(400.MHz())
        .hclk(200.MHz())
        .pll1_r_ck(100.MHz())
        .freeze(pwrcfg, &dp.SYSCFG);

    defmt::warn!("Hello, world!");

    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpiog = dp.GPIOG.split(ccdr.peripheral.GPIOG);
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);

    let link_led = gpiob.pb0.into_push_pull_output().erase();
    let led = gpioe.pe1.into_push_pull_output().erase();

    let rmii_ref_clk = gpioa.pa1.into_alternate();
    let rmii_mdio = gpioa.pa2.into_alternate();
    let rmii_mdc = gpioc.pc1.into_alternate();
    let rmii_crs_dv = gpioa.pa7.into_alternate();
    let rmii_rxd0 = gpioc.pc4.into_alternate();
    let rmii_rxd1 = gpioc.pc5.into_alternate();
    let rmii_tx_en = gpiog.pg11.into_alternate();
    let rmii_txd0 = gpiog.pg13.into_alternate();
    let rmii_txd1 = gpiob.pb13.into_alternate();

    static MAC: [u8; 6] = [0x12, 0x00, 0x00, 0x00, 0x00, 0x00];

    let mac_addr = smoltcp::wire::EthernetAddress::from_bytes(&MAC);
    let (_eth_dma, eth_mac) = unsafe {
        DES_RING.write(ethernet::DesRing::new());

        ethernet::new(
            dp.ETHERNET_MAC,
            dp.ETHERNET_MTL,
            dp.ETHERNET_DMA,
            (
                rmii_ref_clk,
                rmii_mdio,
                rmii_mdc,
                rmii_crs_dv,
                rmii_rxd0,
                rmii_rxd1,
                rmii_tx_en,
                rmii_txd0,
                rmii_txd1,
            ),
            DES_RING.assume_init_mut(),
            mac_addr,
            ccdr.peripheral.ETH1MAC,
            &ccdr.clocks,
        )
    };

    // Initialise ethernet PHY...
    let mut lan8742a = ethernet::phy::LAN8742A::new(eth_mac.set_phy_addr(0));
    lan8742a.phy_reset();
    lan8742a.phy_init();

    unsafe {
        ethernet::enable_interrupt();
        cp.NVIC.set_priority(stm32::Interrupt::ETH, 0x80 - 1); // Mid prio
        cortex_m::peripheral::NVIC::unmask(stm32::Interrupt::ETH);
    }

    lilos::time::initialize_sys_tick(&mut cp.SYST, ccdr.clocks.sysclk().to_Hz());

    unsafe {
        lilos::exec::run_tasks_with_preemption(
            &mut [
                core::pin::pin!(led_task(led)),
                core::pin::pin!(poll_link(lan8742a, link_led)),
            ],
            lilos::exec::ALL_TASKS,
            Interrupts::Filtered(0x80),
        );
    }
}

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

#[cortex_m_rt::interrupt]
fn ETH() {
    unsafe {
        ethernet::interrupt_handler();
    }
}

async fn led_task(mut led: ErasedPin<Output>) -> Infallible {
    let mut gate = PeriodicGate::from(lilos::time::Millis(500));
    loop {
        led.toggle();
        gate.next_time().await;
    }
}
