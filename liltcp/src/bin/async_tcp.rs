#![no_main]
#![no_std]

use core::{cell::RefCell, convert::Infallible};

use embassy_futures::select;
use lilos::exec::Interrupts;
use liltcp::stack::{InnerStack, Stack};
use liltcp::tcp::TcpClient;
use liltcp::{self as _, smoltcp_lilos::smol_now};

use smoltcp::wire::IpCidr;
use smoltcp::{
    iface::{Interface, SocketStorage},
    time::Duration,
};
use stm32h7xx_hal::ethernet::phy::LAN8742A;
use stm32h7xx_hal::ethernet::StationManagement;
use stm32h7xx_hal::gpio::{ErasedPin, Output};
use stm32h7xx_hal::{
    ethernet::{self, PHY as _},
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

    let (mut eth_dma, eth_mac) = ethernet::new(
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

    lilos::time::initialize_sys_tick(&mut cp.SYST, ccdr.clocks.sysclk().to_Hz());

    let config = smoltcp::iface::Config::new(liltcp::MAC.into());
    let mut interface = Interface::new(config, &mut eth_dma, liltcp::smoltcp_lilos::smol_now());
    interface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(
            liltcp::IP_ADDR.into_address(),
            liltcp::PREFIX_LEN,
        ));
    });

    // ANCHOR: stack_init
    let mut storage = [SocketStorage::EMPTY; 1];
    // NOTE: This unnecessarily exposes implementation details of the Stack's shared state.
    // For the purposes of this demo, it is fine, but it should be hidden in production impl.
    let inner_stack = RefCell::new(InnerStack::new(&mut storage, interface));
    let stack = Stack::new(&inner_stack);
    // ANCHOR_END: stack_init

    unsafe {
        liltcp::enable_eth_interrupt(&mut cp.NVIC);

        // ANCHOR: spawn
        lilos::exec::run_tasks_with_preemption(
            &mut [
                core::pin::pin!(liltcp::led_task(gpio.led)),
                core::pin::pin!(tcp_client_task(stack)),
                core::pin::pin!(net_task(stack, eth_dma, lan8742a, gpio.link_led)),
            ],
            lilos::exec::ALL_TASKS,
            Interrupts::Filtered(liltcp::NVIC_BASEPRI),
        );
        // ANCHOR_END: spawn
    }
}

// ANCHOR: tcp_client_task
async fn tcp_client_task(stack: Stack<'_>) -> Infallible {
    static mut TX: [u8; 1024] = [0u8; 1024];
    static mut RX: [u8; 1024] = [0u8; 1024];

    let mut client = TcpClient::new(stack, unsafe { &mut RX[..] }, unsafe { &mut TX[..] });

    client
        .connect(liltcp::REMOTE_ENDPOINT, liltcp::LOCAL_ENDPOINT)
        .await
        .unwrap();

    defmt::info!("Connected.");

    // loopback
    loop {
        let mut buffer = [0u8; 5];
        let len = defmt::unwrap!(client.recv(&mut buffer).await);
        // Let's not care about the number of sent bytes,
        // with the current buffer settings, it should always write full buffer.
        defmt::unwrap!(client.send(&buffer[..len]).await);
    }
}
// ANCHOR_END: tcp_client_task

// ANCHOR: irq_notify
static IRQ_NOTIFY: lilos::exec::Notify = lilos::exec::Notify::new();
// ANCHOR_END: irq_notify

// ANCHOR: net_task
async fn net_task(
    mut stack: Stack<'_>,
    mut dev: ethernet::EthernetDMA<4, 4>,
    mut phy: LAN8742A<impl StationManagement>,
    mut link_led: ErasedPin<Output>,
) -> Infallible {
    let mut eth_up = false;

    loop {
        let poll_delay = stack.with(|(sockets, interface)| {
            interface
                .poll_delay(smol_now(), sockets)
                .unwrap_or(Duration::from_millis(1))
        });

        match embassy_futures::select::select(
            lilos::time::sleep_for(lilos::time::Millis(poll_delay.millis())),
            IRQ_NOTIFY.until_next(),
        )
        .await
        {
            select::Either::First(_) => {}
            select::Either::Second(_) => {}
        }

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
        if !eth_up {
            continue;
        }

        stack.with(|(sockets, interface)| interface.poll(smol_now(), &mut dev, sockets));
    }
}
// ANCHOR_END: net_task

// ANCHOR: eth_irq
#[cortex_m_rt::interrupt]
fn ETH() {
    unsafe {
        ethernet::interrupt_handler();
    }
    // NOTE: embassy_net wakes polling task any time RX or TX tokens are consumed, resulting in 3x
    // throughput
    IRQ_NOTIFY.notify();
}
// ANCHOR_END: eth_irq
