//! Example MQTT client with smoltcp stack on RTIC
//!
//! Inspired by https://github.com/quartiq/thermostat-eem

#![deny(warnings, clippy::all)]
#![forbid(unsafe_code)]
#![no_main]
#![no_std]

//use panic_abort as _; // panic handler
use panic_rtt_target as _; // panic handler

mod config;
mod hardware;
mod net;
mod settings;
mod telemetry;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::built_info;
    use crate::hardware::{
        eth::EthStorage,
        gpio::{LedBluePin, LedGreenPin, LedRedPin},
        net::NetStorage,
        network_clock::NetworkClock,
        phy::Phy,
        NetworkManager, NetworkStack,
    };
    use crate::{
        config::Config,
        net::{NetworkState, NetworkUsers},
        settings::Settings,
        telemetry::Telemetry,
    };
    use log::info;
    use rand_core::RngCore;
    use rtt_logger::RTTLogger;
    use rtt_target::rtt_init_print;
    use smoltcp::{
        iface::{InterfaceBuilder, NeighborCache, Routes},
        socket::{TcpSocket, TcpSocketBuffer, UdpSocket, UdpSocketBuffer},
        wire::{IpCidr, Ipv4Address, Ipv4Cidr},
    };
    use stm32_eth::{Eth, EthPins, FilterMode};
    use stm32f4xx_hal::{gpio::Speed, prelude::*, time::Hertz};
    use systick_monotonic::{ExtU64, Systick};

    const SYS_CLOCK_FREQ: Hertz = Hertz::MHz(180);

    static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Trace);

    #[shared]
    struct Shared {
        net: NetworkUsers<Settings, Telemetry>,
        settings: Settings,
        telemetry: Telemetry,
    }

    #[local]
    struct Local {
        led_r: LedRedPin,
        activity_led: LedBluePin,
        link_led: LedGreenPin,
    }

    #[monotonic(binds = SysTick, default = true)]
    type SysTickMono = Systick<1_000>; // 1ms resolution

    #[init(local = [
        eth_storage: EthStorage = EthStorage::new(),
        net_storage: NetStorage = NetStorage::new(),
        eth: Option<Eth<'static, 'static>> = None,
        net_stack_manager: Option<NetworkManager> = None,
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

        let config = Config::load_from_env();

        info!("--- Starting hardware setup");

        // Set up the system clock
        // HCLK must be at least 25MHz to use the ethernet peripheral
        // The RNG requires the PLL48_CLK to be active
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc
            .cfgr
            .hclk(64.MHz())
            .sysclk(SYS_CLOCK_FREQ)
            .require_pll48clk()
            .freeze();
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
            FilterMode::FilterDest(config.mac_address.0),
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
            delay.delay_ms(100_u32);
        }
        eth.interrupt_handler();
        eth.enable_interrupt();
        ctx.local.eth.replace(eth);

        info!("Setup TCP/IP");
        ctx.local.net_storage.ip_addrs[0] = IpCidr::Ipv4(Ipv4Cidr::new(config.ip_address, 24));
        let neighbor_cache = NeighborCache::new(&mut ctx.local.net_storage.neighbor_cache[..]);
        let mut routes = Routes::new(&mut ctx.local.net_storage.routes_cache[..]);
        routes
            .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
            .unwrap();
        let mut eth_iface = InterfaceBuilder::new(
            ctx.local.eth.as_mut().unwrap(),
            &mut ctx.local.net_storage.sockets[..],
        )
        .hardware_addr(config.mac_address.into())
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

        info!("Setup SysTick");
        let systick = ctx.core.SYST;
        let mono = Systick::new(systick, clocks.sysclk().raw());
        let net_clock = NetworkClock::new(|| monotonics::now().ticks());

        info!("Setup network");
        let random_seed = {
            let mut rng = ctx.device.RNG.constrain(&clocks);
            let mut data = [0u8; 4];
            rng.fill_bytes(&mut data);
            data
        };
        let mut net_stack = NetworkStack::new(eth_iface, net_clock);
        net_stack.seed_random_port(&random_seed);
        let stack_manager = NetworkManager::new(net_stack);
        ctx.local.net_stack_manager.replace(stack_manager);
        let net = NetworkUsers::new(
            ctx.local.net_stack_manager.as_mut().unwrap(),
            mdio_pin,
            mdc_pin,
            net_clock,
            env!("CARGO_BIN_NAME"),
            config.mac_address,
            minimq::embedded_nal::Ipv4Addr::from(config.broker_ip_address.0).into(),
        );

        info!("--- Hardware setup done");

        link_status::spawn().unwrap();
        poll_ip_stack::spawn().unwrap();
        settings_update::spawn().unwrap();
        telemetry_task::spawn().unwrap();

        (
            Shared {
                net,
                settings: Settings::default(),
                telemetry: Telemetry::default(),
            },
            Local {
                led_r,
                activity_led,
                link_led,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [led_r], shared = [net, settings], priority = 1)]
    fn settings_update(ctx: settings_update::Context) {
        let led = ctx.local.led_r;
        let mut net = ctx.shared.net;
        let mut settings = ctx.shared.settings;
        let s = net.lock(|n| *n.miniconf.settings());
        settings.lock(|current| *current = s);
        led.set_state(s.led.into());
    }

    #[task(shared = [net, telemetry], priority = 1)]
    fn telemetry_task(ctx: telemetry_task::Context) {
        let mut net = ctx.shared.net;
        let mut telemetry = ctx.shared.telemetry;
        let t: Telemetry = telemetry.lock(|telemetry| {
            telemetry.dummy += 1;
            *telemetry
        });
        net.lock(|n| n.telemetry.publish(&t));
        telemetry_task::spawn_after(1_u64.secs()).unwrap();
    }

    #[task(local = [activity_led], shared = [net], priority = 1)]
    fn poll_ip_stack(ctx: poll_ip_stack::Context) {
        let led = ctx.local.activity_led;
        let mut net = ctx.shared.net;
        match net.lock(|n| n.update()) {
            NetworkState::SettingsChanged => settings_update::spawn().unwrap(),
            NetworkState::Updated => led.toggle(),
            NetworkState::NoChange => {}
        }
        poll_ip_stack::spawn_after(10_u64.millis()).unwrap();
    }

    #[task(local = [link_led], shared = [net], priority = 1)]
    fn link_status(ctx: link_status::Context) {
        let led = ctx.local.link_led;
        let mut net = ctx.shared.net;
        let link_status = net.lock(|n| n.processor.handle_link());
        if link_status {
            led.set_high();
        } else {
            led.set_low();
        }
        link_status::spawn_after(1_u64.secs()).unwrap();
    }

    #[task(binds = ETH, shared = [net], priority = 1)]
    fn on_eth(ctx: on_eth::Context) {
        let mut net = ctx.shared.net;
        net.lock(|n| n.processor.handle_interrupt());
    }
}
