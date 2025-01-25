#![no_main]
#![no_std]

use core::convert::Infallible;

use lilos::exec::Interrupts;
use liltcp as _;
use smoltcp::{
    iface::{Interface, SocketSet, SocketStorage},
    storage::RingBuffer,
    wire::IpCidr,
};
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

    // ANCHOR: interface_init
    let config = smoltcp::iface::Config::new(liltcp::MAC.into());
    let mut interface = Interface::new(config, &mut eth_dma, liltcp::smoltcp_lilos::smol_now());
    interface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(
            liltcp::IP_ADDR.into_address(),
            liltcp::PREFIX_LEN,
        ));
    });

    let mut storage = [SocketStorage::EMPTY; 1];
    let mut sockets = SocketSet::new(&mut storage[..]);
    // ANCHOR_END: interface_init

    unsafe {
        liltcp::enable_eth_interrupt(&mut cp.NVIC);

        // ANCHOR: spawn
        lilos::exec::run_tasks_with_preemption(
            &mut [
                core::pin::pin!(liltcp::led_task(gpio.led)),
                core::pin::pin!(poll_smoltcp(
                    interface,
                    eth_dma,
                    &mut sockets,
                    lan8742a,
                    gpio.link_led
                )),
            ],
            lilos::exec::ALL_TASKS,
            Interrupts::Filtered(liltcp::NVIC_BASEPRI),
        );
        // ANCHOR_END: spawn
    }
}

// ANCHOR: net_task
async fn poll_smoltcp<'a>(
    mut interface: Interface,
    mut dev: ethernet::EthernetDMA<4, 4>,
    sockets: &mut SocketSet<'a>,
    mut phy: LAN8742A<impl StationManagement>,
    mut link_led: ErasedPin<Output>,
) -> Infallible {
    static mut RX: [u8; 1024] = [0u8; 1024];
    static mut TX: [u8; 1024] = [0u8; 1024];

    let rx_buffer = unsafe { RingBuffer::new(&mut RX[..]) };
    let tx_buffer = unsafe { RingBuffer::new(&mut TX[..]) };

    let client = smoltcp::socket::tcp::Socket::new(rx_buffer, tx_buffer);

    let handle = sockets.add(client);

    let mut eth_up = false;

    loop {
        'worker: {
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
                break 'worker;
            }

            let ready = interface.poll(liltcp::smoltcp_lilos::smol_now(), &mut dev, sockets);

            if !ready {
                break 'worker;
            }

            let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
            if !socket.is_open() {
                defmt::info!("not open, issuing connect");
                defmt::unwrap!(socket.connect(
                    interface.context(),
                    liltcp::REMOTE_ENDPOINT,
                    liltcp::LOCAL_ENDPOINT,
                ));

                break 'worker;
            }

            let mut buffer = [0u8; 10];
            if socket.can_recv() {
                let len = defmt::unwrap!(socket.recv_slice(&mut buffer));
                defmt::info!("recvd: {} bytes {}", len, buffer[..len]);
            }
            if socket.can_send() {
                defmt::unwrap!(socket.send_slice(b"world"));
            }
        }

        // NOTE: Not performant, doesn't handle interrupt signal, cancel the wait on IRQ, etc.
        // NOTE: In async code, this will be replaced with a more elaborate calling of poll_at.
        lilos::time::sleep_for(lilos::time::Millis(1)).await;
    }
}
// ANCHOR_END: net_task

// ANCHOR: eth_irq
#[cortex_m_rt::interrupt]
fn ETH() {
    unsafe {
        ethernet::interrupt_handler();
    }
}
// ANCHOR_END: eth_irq
