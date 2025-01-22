#![no_main]
#![no_std]

use core::{cell::RefCell, convert::Infallible};

use cortex_m::interrupt::Mutex;
use embassy_futures::select;
use grounded::uninit::GroundedCell;
use lilos::exec::Interrupts;
use liltcp::stack::{Stack, StackState};
use liltcp::tcp::TcpClient;
use liltcp::{self as _, smoltcp_lilos::smol_now};

use smoltcp::{
    iface::{Interface, SocketSet, SocketStorage},
    time::Duration,
    wire::{IpAddress, IpCidr},
};
use stm32h7xx_hal::{
    ethernet::{self, PHY as _},
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
    let mut interface = Interface::new(config, &mut eth_dma, liltcp::smoltcp_lilos::smol_now());
    interface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::v4(10, 106, 0, 251), 24));
    });

    static mut STORAGE: [SocketStorage<'static>; 1] = [SocketStorage::EMPTY; 1];

    static STACK: GroundedCell<Mutex<RefCell<StackState<'static>>>> = GroundedCell::uninit();

    let inner_stack = unsafe {
        STACK.get().write(Mutex::new(RefCell::new(StackState {
            sockets: unsafe { SocketSet::new(&mut STORAGE[..]) },
            interface,
        })));
        STACK.get().as_ref().unwrap()
    };

    let stack = Stack {
        inner: &inner_stack,
    };

    // ANCHOR_END: interface_init

    // ANCHOR: spawn
    unsafe {
        lilos::exec::run_tasks_with_preemption(
            &mut [
                core::pin::pin!(liltcp::led_task(led)),
                //core::pin::pin!(poll_link(lan8742a, link_led)),
                core::pin::pin!(net_task(stack)),
                core::pin::pin!(poll_smoltcp(&stack, eth_dma)),
            ],
            lilos::exec::ALL_TASKS,
            Interrupts::Filtered(0x80),
        );
    }
    // ANCHOR_END: spawn
}

async fn net_task<'a>(stack: Stack<'a>) -> Infallible {
    static mut TX: [u8; 1024] = [0u8; 1024];
    static mut RX: [u8; 1024] = [0u8; 1024];

    let mut client = TcpClient::new(stack, unsafe { &mut RX[..] }, unsafe { &mut TX[..] });

    // connect
    client.connect().await.unwrap();

    let buff = [0x55; 1024];
    loop {
        // loopback
        // let mut buffer = [0u8; 5];
        // let len = defmt::unwrap!(client.recv(&mut buffer).await);
        match client.send(&buff).await {
            Ok(_) => {}
            Err(_) => todo!(),
        }
    }
}

static SYNCER: lilos::exec::Notify = lilos::exec::Notify::new();

// ANCHOR: poll_smoltcp
async fn poll_smoltcp<'a>(stack: &Stack<'a>, mut dev: ethernet::EthernetDMA<4, 4>) -> Infallible {
    loop {
        let poll_delay = cortex_m::interrupt::free(|cs| {
            stack.with(cs, |(sockets, interface)| {
                interface
                    .poll_delay(smol_now(), sockets)
                    .unwrap_or(Duration::from_millis(1))
            })
        });

        match embassy_futures::select::select(
            lilos::time::sleep_for(lilos::time::Millis(poll_delay.millis())),
            SYNCER.until_next(),
        )
        .await
        {
            select::Either::First(_) => {}
            select::Either::Second(_) => {}
        }

        cortex_m::interrupt::free(|cs| {
            stack.with(cs, |(sockets, interface)| {
                interface.poll(smol_now(), &mut dev, sockets)
            })
        });
    }
}
// ANCHOR_END: poll_smoltcp

#[cortex_m_rt::interrupt]
fn ETH() {
    // TODO: embassy_net wakes polling task any time RX or TX tokens are consumed, resulting in 3x
    // throughput
    unsafe {
        SYNCER.notify();
        ethernet::interrupt_handler();
    }
}
