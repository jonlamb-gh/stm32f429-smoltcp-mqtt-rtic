use log::info;
use smoltcp::wire::{EthernetAddress, Ipv4Address};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub mac_address: EthernetAddress,
    pub ip_address: Ipv4Address,
    pub broker_ip_address: Ipv4Address,
}

impl Config {
    /// export MAC_ADDRESS="02:00:00:03:02:00"
    /// export IP_ADDRESS="a.b.c.d"
    /// export BROKER_IP_ADDRESS="a.b.c.d"
    pub fn load_from_env() -> Self {
        let cfg = Self {
            mac_address: env!("MAC_ADDRESS").parse().unwrap(),
            ip_address: env!("IP_ADDRESS").parse().unwrap(),
            broker_ip_address: env!("BROKER_IP_ADDRESS").parse().unwrap(),
        };
        info!("MAC address: {}", cfg.mac_address);
        info!("IP address: {}", cfg.ip_address);
        info!("Broker IP address: {}", cfg.broker_ip_address);
        cfg
    }
}
