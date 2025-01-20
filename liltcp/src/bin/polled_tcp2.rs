#![no_main]
#![no_std]

use core::convert::Infallible;

use grounded::uninit::GroundedCell;
use lilos::{
    exec::Interrupts,
    time::{sleep_for, Millis, PeriodicGate},
};
use liltcp as _;
use smoltcp::{
    iface::{Interface, SocketSet, SocketStorage},
    storage::RingBuffer,
    wire::{IpAddress, IpCidr},
};
use stm32h7xx_hal::{
    ethernet::{self, phy::LAN8742A, StationManagement, PHY as _},
    gpio::{ErasedPin, Output},
    interrupt, pac,
    prelude::*,
    stm32,
};

#[link_section = ".sram3.eth"]
static DES_RING: GroundedCell<ethernet::DesRing<4, 4>> = GroundedCell::uninit();

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

    static MAC: [u8; 6] = [0x12, 0x01, 0x02, 0x03, 0x04, 0x05];

    // TODO: only safe to be called once, I'd use StaticCell, but it is unsound when used
    // inside linker sections
    let des_ring = unsafe {
        DES_RING.get().write(ethernet::DesRing::new());
        &mut *DES_RING.get()
    };

    let mac_addr = smoltcp::wire::EthernetAddress::from_bytes(&MAC);
    let (mut eth_dma, eth_mac) = ethernet::new(
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
        des_ring,
        mac_addr,
        ccdr.peripheral.ETH1MAC,
        &ccdr.clocks,
    );

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

    // ANCHOR: interface_init
    let config = smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(mac_addr));
    let mut interface = Interface::new(config, &mut eth_dma, smol_now());
    interface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::v4(10, 106, 0, 251), 24));
    });

    static mut STORAGE: [SocketStorage<'static>; 1] = [SocketStorage::EMPTY; 1];

    let mut sockets = unsafe { SocketSet::new(&mut STORAGE[..]) };

    // ANCHOR_END: interface_init

    // ANCHOR: spawn
    unsafe {
        lilos::exec::run_tasks_with_preemption(
            &mut [
                core::pin::pin!(led_task(led)),
                core::pin::pin!(poll_link(lan8742a, link_led)),
                core::pin::pin!(poll_smoltcp(interface, eth_dma, &mut sockets)),
            ],
            lilos::exec::ALL_TASKS,
            Interrupts::Filtered(0x80),
        );
    }
    // ANCHOR_END: spawn
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

// ANCHOR: poll_smoltcp
async fn poll_smoltcp<'a>(
    mut interface: Interface,
    mut dev: ethernet::EthernetDMA<4, 4>,
    sockets: &mut SocketSet<'a>,
) -> Infallible {
    static mut TX: [u8; 1024] = [0u8; 1024];
    static mut RX: [u8; 1024] = [0u8; 1024];

    let rx_buffer = unsafe { RingBuffer::new(&mut RX[..]) };
    let tx_buffer = unsafe { RingBuffer::new(&mut TX[..]) };

    let client = smoltcp::socket::tcp::Socket::new(rx_buffer, tx_buffer);

    let handle = sockets.add(client);
    sleep_for(lilos::time::Millis(3000)).await;
    loop {
        'worker: {
            let ready = interface.poll(smol_now(), &mut dev, sockets);

            if !ready {
                break 'worker;
            }

            let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
            if !socket.is_open() {
                defmt::info!("not open, issuing connect");
                defmt::unwrap!(socket.connect(
                    interface.context(),
                    (IpAddress::v4(10, 106, 0, 198), 8001),
                    52234,
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
// ANCHOR_END: poll_smoltcp

async fn led_task(mut led: ErasedPin<Output>) -> Infallible {
    let mut gate = PeriodicGate::from(lilos::time::Millis(500));
    loop {
        led.toggle();
        gate.next_time().await;
    }
}

#[cortex_m_rt::interrupt]
fn ETH() {
    unsafe {
        ethernet::interrupt_handler();
    }
}

fn smol_now() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_millis(u64::from(lilos::time::TickTime::now()) as i64)
}
