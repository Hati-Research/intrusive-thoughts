#![no_main]
#![no_std]

pub mod smoltcp_lilos;
pub mod stack;
pub mod tcp;

use core::{
    convert::Infallible,
    sync::atomic::{self, AtomicBool, AtomicUsize, Ordering},
};

use cortex_m_semihosting::debug;

use defmt_rtt as _; // global logger

use grounded::uninit::GroundedCell;
use lilos::time::PeriodicGate;
use smoltcp::wire::{IpEndpoint, Ipv4Address};
use stm32h7xx_hal::{
    self as _,
    ethernet::{self, PinsRMII},
    gpio::{ErasedPin, Output},
    pac,
    prelude::*,
    rcc,
};

use panic_probe as _;

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", COUNT.fetch_add(1, Ordering::Relaxed));

pub const MAC: smoltcp::wire::EthernetAddress =
    smoltcp::wire::EthernetAddress([0x12, 0x00, 0x00, 0x00, 0x00, 0x00]);

pub const IP_ADDR: Ipv4Address = Ipv4Address::new(10, 106, 0, 251);
pub const PREFIX_LEN: u8 = 24;
pub const REMOTE_ENDPOINT: IpEndpoint =
    IpEndpoint::new(Ipv4Address::new(10, 106, 0, 198).into_address(), 8001);
pub const LOCAL_ENDPOINT: u16 = 55128;

pub fn initialize_clock(
    pwr: pac::PWR,
    rcc: pac::RCC,
    syscfg: &pac::SYSCFG,
) -> stm32h7xx_hal::rcc::Ccdr {
    let pwrcfg = pwr.constrain().vos1().freeze();

    // we use SRAM3 for storing descriptor ring
    rcc.ahb2enr.modify(|_, w| w.sram3en().enabled());

    rcc.constrain()
        .sys_ck(400.MHz())
        .hclk(200.MHz())
        .pll1_r_ck(100.MHz())
        .freeze(pwrcfg, syscfg)
}

pub struct Gpio<Rmii: PinsRMII> {
    pub led: ErasedPin<Output>,
    pub link_led: ErasedPin<Output>,
    pub eth_pins: Rmii,
}

pub fn init_gpio(
    gpioa: pac::GPIOA,
    clocka: rcc::rec::Gpioa,
    gpiob: pac::GPIOB,
    clockb: rcc::rec::Gpiob,
    gpioc: pac::GPIOC,
    clockc: rcc::rec::Gpioc,
    gpioe: pac::GPIOE,
    clocke: rcc::rec::Gpioe,
    gpiog: pac::GPIOG,
    clockg: rcc::rec::Gpiog,
) -> Gpio<impl PinsRMII> {
    let gpioa = gpioa.split(clocka);
    let gpiob = gpiob.split(clockb);
    let gpioc = gpioc.split(clockc);
    let gpioe = gpioe.split(clocke);
    let gpiog = gpiog.split(clockg);

    let rmii_ref_clk = gpioa.pa1.into_alternate();
    let rmii_mdio = gpioa.pa2.into_alternate();
    let rmii_mdc = gpioc.pc1.into_alternate();
    let rmii_crs_dv = gpioa.pa7.into_alternate();
    let rmii_rxd0 = gpioc.pc4.into_alternate();
    let rmii_rxd1 = gpioc.pc5.into_alternate();
    let rmii_tx_en = gpiog.pg11.into_alternate();
    let rmii_txd0 = gpiog.pg13.into_alternate();
    let rmii_txd1 = gpiob.pb13.into_alternate();

    Gpio {
        led: gpioe.pe1.into_push_pull_output().erase(),
        link_led: gpiob.pb0.into_push_pull_output().erase(),
        eth_pins: (
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
    }
}

#[link_section = ".sram3.eth"]
static DES_RING: GroundedCell<ethernet::DesRing<4, 4>> = GroundedCell::uninit();

static DES_RING_TAKEN: AtomicBool = AtomicBool::new(false);

pub unsafe fn take_des_ring() -> &'static mut ethernet::DesRing<4, 4> {
    if DES_RING_TAKEN.swap(true, atomic::Ordering::SeqCst) {
        panic!("take_des_ring called multiple times");
    }
    DES_RING.get().write(ethernet::DesRing::new());

    &mut *DES_RING.get()
}

pub const NVIC_BASEPRI: u8 = 0x80;

pub unsafe fn enable_eth_interrupt(nvic: &mut pac::NVIC) {
    ethernet::enable_interrupt();
    nvic.set_priority(stm32h7xx_hal::stm32::Interrupt::ETH, NVIC_BASEPRI - 1);
    cortex_m::peripheral::NVIC::unmask(stm32h7xx_hal::stm32::Interrupt::ETH);
}

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
