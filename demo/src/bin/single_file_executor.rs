#![no_main]
#![no_std]

use core::mem::MaybeUninit;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::{future::Future, pin::Pin};

use demo as _; // global logger + panicking-behavior + memory layout
use embedded_hal::digital::OutputPin;
use nrf52840_hal::{self as hal, gpio::Level};

unsafe fn clone_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

unsafe fn wake(data: *const ()) {}

unsafe fn wake_by_ref(data: *const ()) {}

unsafe fn drop_waker(data: *const ()) {}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = hal::pac::Peripherals::take().unwrap();
    let port1 = hal::gpio::p1::Parts::new(p.P1);
    let mut led = port1.p1_15.into_push_pull_output(Level::Low);

    defmt::error!("Hello, world!");

    let mut blinker = Blinker {};
    let blinker_waker = unsafe { &Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(blinker_waker);

    loop {
        let res = Pin::new(&mut blinker).poll(&mut context);
        match res {
            Poll::Ready(out) => defmt::error!("ready {}", out),
            Poll::Pending => defmt::error!("pending"),
        };
        led.set_high().unwrap();
        cortex_m::asm::delay(1_000_000);
        led.set_low().unwrap();
        cortex_m::asm::delay(1_000_000);
    }
}

struct Blinker {}

impl Future for Blinker {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        defmt::error!("polled");
        Poll::Ready(())
    }
}
