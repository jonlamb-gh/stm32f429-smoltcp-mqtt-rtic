//#![deny(warnings, clippy::all)]
//#![forbid(unsafe_code)]
#![no_main]
#![no_std]

//use panic_abort as _; // panic handler
use panic_rtt_target as _; // panic handler

mod hardware;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::built_info;
    use crate::hardware::{
        eth::EthStorage,
        gpio::{LedBluePin, LedGreenPin, LedRedPin, PhyMdcPin, PhyMdioPin},
        net::{NetStorage, SRC_MAC},
        phy::Phy,
    };
    use log::{debug, info};
    use rtt_logger::RTTLogger;
    use rtt_target::rtt_init_print;
    use smoltcp::{
        iface::{Interface, InterfaceBuilder, NeighborCache, Routes},
        socket::{TcpSocket, TcpSocketBuffer, UdpSocket, UdpSocketBuffer},
        time::Instant,
        wire::{EthernetAddress, Ipv4Address},
    };
    use stm32_eth::{Eth, EthPins, FilterMode};
    use stm32f4xx_hal::{gpio::Speed, prelude::*, time::Hertz};
    use systick_monotonic::{ExtU64, Systick};

    const SYS_CLOCK_FREQ: Hertz = Hertz::MHz(180);

    static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Trace);

    #[shared]
    struct Shared {
        #[lock_free]
        net: Interface<'static, &'static mut Eth<'static, 'static>>,
    }

    #[local]
    struct Local {
        _led_r: LedRedPin,
        activity_led: LedBluePin,
        link_led: LedGreenPin,
        mdio_pin: PhyMdioPin,
        mdc_pin: PhyMdcPin,
    }

    #[monotonic(binds = SysTick, default = true)]
    type SysTickMono = Systick<1_000>; // 1ms resolution

    #[init(local = [
        eth_storage: EthStorage = EthStorage::new(),
        net_storage: NetStorage = NetStorage::new(),
        eth: Option<Eth<'static, 'static>> = None,
    ])]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Trace))
            .unwrap();

        info!(
            "{} version {}",
            built_info::PKG_NAME,
            built_info::PKG_VERSION
        );
        info!("{}, {}", built_info::RUSTC_VERSION, built_info::TARGET);

        info!("--- Starting hardware setup");

        // Set up the system clock
        // HCLK must be at least 25MHz to use the ethernet peripheral
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.hclk(64.MHz()).sysclk(SYS_CLOCK_FREQ).freeze();
        debug_assert!(clocks.hclk() >= Hertz::MHz(25));
        debug_assert_eq!(clocks.sysclk(), SYS_CLOCK_FREQ);

        info!("Setup GPIO");
        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();
        let gpiog = ctx.device.GPIOG.split();

        let mut link_led = gpiob.pb0.into_push_pull_output();
        let mut activity_led = gpiob.pb7.into_push_pull_output();
        let mut led_r = gpiob.pb14.into_push_pull_output();
        link_led.set_low();
        activity_led.set_low();
        led_r.set_low();

        info!("Setup Ethernet");
        let mut mdio_pin = gpioa.pa2.into_alternate().set_speed(Speed::VeryHigh);
        let mut mdc_pin = gpioc.pc1.into_alternate().set_speed(Speed::VeryHigh);

        let eth_pins = EthPins {
            ref_clk: gpioa.pa1,
            crs: gpioa.pa7,
            tx_en: gpiog.pg11,
            tx_d0: gpiog.pg13,
            tx_d1: gpiob.pb13,
            rx_d0: gpioc.pc4,
            rx_d1: gpioc.pc5,
        };

        let mut eth = Eth::new(
            ctx.device.ETHERNET_MAC,
            ctx.device.ETHERNET_DMA,
            ctx.device.ETHERNET_MMC,
            &mut ctx.local.eth_storage.rx_ring[..],
            &mut ctx.local.eth_storage.tx_ring[..],
            FilterMode::FilterDest(SRC_MAC),
            //FilterMode::Promiscuous,
            clocks,
            eth_pins,
        )
        .unwrap();

        info!("Setup phy");
        let mut delay = asm_delay::AsmDelay::new(asm_delay::bitrate::Hertz(SYS_CLOCK_FREQ.raw()));
        let phy = Phy::new(eth.smi(&mut mdio_pin, &mut mdc_pin));
        phy.reset(&mut delay);
        phy.setup();
        info!("Waiting for link");
        while !phy.link_status() {
            cortex_m::asm::delay(100000);
        }
        eth.interrupt_handler();
        eth.enable_interrupt();
        ctx.local.eth.replace(eth);

        info!("Setup TCP/IP");
        let mac = EthernetAddress::from_bytes(&SRC_MAC);
        info!(
            "IP: {} MAC: {}",
            ctx.local.net_storage.ip_addrs[0].address(),
            mac
        );
        let neighbor_cache = NeighborCache::new(&mut ctx.local.net_storage.neighbor_cache[..]);
        let mut routes = Routes::new(&mut ctx.local.net_storage.routes_cache[..]);
        routes
            .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
            .unwrap();
        let mut eth_iface = InterfaceBuilder::new(
            ctx.local.eth.as_mut().unwrap(),
            &mut ctx.local.net_storage.sockets[..],
        )
        .hardware_addr(mac.into())
        .ip_addrs(&mut ctx.local.net_storage.ip_addrs[..])
        .neighbor_cache(neighbor_cache)
        .routes(routes)
        .finalize();

        for storage in ctx.local.net_storage.tcp_socket_storage[..].iter_mut() {
            let tcp_socket = {
                let rx_buffer = TcpSocketBuffer::new(&mut storage.rx_storage[..]);
                let tx_buffer = TcpSocketBuffer::new(&mut storage.tx_storage[..]);

                TcpSocket::new(rx_buffer, tx_buffer)
            };

            eth_iface.add_socket(tcp_socket);
        }

        for storage in ctx.local.net_storage.udp_socket_storage[..].iter_mut() {
            let udp_socket = {
                let rx_buffer =
                    UdpSocketBuffer::new(&mut storage.rx_metadata[..], &mut storage.rx_storage[..]);
                let tx_buffer =
                    UdpSocketBuffer::new(&mut storage.tx_metadata[..], &mut storage.tx_storage[..]);

                UdpSocket::new(rx_buffer, tx_buffer)
            };

            eth_iface.add_socket(udp_socket);
        }

        let systick = ctx.core.SYST;
        let mono = Systick::new(systick, clocks.sysclk().raw());
        //let clock = SystemTimer::new(|| monotonics::now().ticks());

        info!("--- Hardware setup done");

        link_status::spawn().unwrap();
        poll_ip_stack::spawn().unwrap();

        (
            Shared { net: eth_iface },
            Local {
                _led_r: led_r,
                activity_led,
                link_led,
                mdio_pin,
                mdc_pin,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [activity_led], shared = [net], priority = 1)]
    fn poll_ip_stack(ctx: poll_ip_stack::Context) {
        let led = ctx.local.activity_led;
        let net = ctx.shared.net;
        let time = Instant::from_millis(monotonics::now().ticks() as i64);
        match net.poll(time) {
            Ok(something_happened) => {
                if something_happened {
                    led.toggle()
                }
            }
            Err(e) => debug!("{:?}", e),
        }
        poll_ip_stack::spawn_after(20_u64.millis()).unwrap();
    }

    #[task(local = [link_led, mdio_pin, mdc_pin], shared = [net], priority = 1)]
    fn link_status(ctx: link_status::Context) {
        let link_led = ctx.local.link_led;
        let mdio = ctx.local.mdio_pin;
        let mdc = ctx.local.mdc_pin;
        let net = ctx.shared.net;

        // Poll link status
        let smi = net.device_mut().smi(mdio, mdc);
        let phy = Phy::new(smi);
        if phy.link_status() {
            link_led.set_high();
        } else {
            link_led.set_low();
            // TODO - close all sockets
        }

        link_status::spawn_after(1_u64.secs()).unwrap();
    }

    #[task(binds = ETH, shared = [net], priority = 1)]
    fn on_eth(ctx: on_eth::Context) {
        let net = ctx.shared.net;
        net.device_mut().interrupt_handler();
    }
}
