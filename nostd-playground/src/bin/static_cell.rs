#![no_main]
#![no_std]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::{future::Future, pin::Pin};

use demo as _; // global logger + panicking-behavior + memory layout
use embedded_hal::digital::OutputPin;
use grounded::uninit::GroundedCell;
use nrf52840_hal::{self as hal, gpio::Level};
use static_cell::StaticCell;

static DATA: StaticCell<()> = StaticCell::new();

static DATA3: GroundedCell<()> = GroundedCell::const_init();

struct LinkedStaticCell<T> {
    inner: UnsafeCell<MaybeUninit<T>>,
}

impl<T> LinkedStaticCell<T> {
    const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    unsafe fn init(&'static self, t: T) -> &'static mut T {
        self.inner.get().write(t)
    }
}

unsafe impl<T> Send for LinkedStaticCell<T> where T: Send {}
unsafe impl<T> Sync for LinkedStaticCell<T> where T: Sync {}

static DATA2: LinkedStaticCell<()> = LinkedStaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = hal::pac::Peripherals::take().unwrap();
    let port1 = hal::gpio::p1::Parts::new(p.P1);
    let mut led = port1.p1_15.into_push_pull_output(Level::Low);

    defmt::error!("Hello, world!");

    loop {}
}
