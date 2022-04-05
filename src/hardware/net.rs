use smoltcp::{
    iface::{Neighbor, Route, SocketStorage},
    socket::UdpPacketMetadata,
    wire::{IpAddress, IpCidr, Ipv4Address, Ipv4Cidr},
};

// TODO - use option_env! and FromStr impl to tweak these
// or put them in flash for config
pub const SRC_MAC: [u8; 6] = [0x02, 0x00, 0x05, 0x06, 0x07, 0x08];
pub const SRC_IP: [u8; 4] = [192, 168, 1, 39];

const NUM_TCP_SOCKETS: usize = 4;
const NUM_UDP_SOCKETS: usize = 1;
const NUM_SOCKETS: usize = NUM_UDP_SOCKETS + NUM_TCP_SOCKETS;

const UDP_RX_SOCKET_BUFFER_SIZE: usize = 512;
const UDP_TX_SOCKET_BUFFER_SIZE: usize = 512;
const UDP_SOCKET_METADATA_COUNT: usize = 10;

const TCP_RX_SOCKET_BUFFER_SIZE: usize = 512;
const TCP_TX_SOCKET_BUFFER_SIZE: usize = 512;

const NUM_NEIGHBOR_CACHE_ENTRIES: usize = 8;
const NUM_ROUTING_TABLE_ENTRIES: usize = 8;

pub struct NetStorage {
    pub ip_addrs: [IpCidr; 1],
    pub sockets: [SocketStorage<'static>; NUM_SOCKETS],
    pub tcp_socket_storage: [TcpSocketStorage; NUM_TCP_SOCKETS],
    pub udp_socket_storage: [UdpSocketStorage; NUM_UDP_SOCKETS],
    pub neighbor_cache: [Option<(IpAddress, Neighbor)>; NUM_NEIGHBOR_CACHE_ENTRIES],
    pub routes_cache: [Option<(IpCidr, Route)>; NUM_ROUTING_TABLE_ENTRIES],
}

impl NetStorage {
    pub const fn new() -> Self {
        Self {
            ip_addrs: [IpCidr::Ipv4(Ipv4Cidr::new(Ipv4Address(SRC_IP), 24)); 1],
            sockets: [SocketStorage::EMPTY; NUM_SOCKETS],
            tcp_socket_storage: [TcpSocketStorage::INIT; NUM_TCP_SOCKETS],
            udp_socket_storage: [UdpSocketStorage::new(); NUM_UDP_SOCKETS],
            neighbor_cache: [None; NUM_NEIGHBOR_CACHE_ENTRIES],
            routes_cache: [None; NUM_ROUTING_TABLE_ENTRIES],
        }
    }
}

pub struct UdpSocketStorage {
    pub rx_storage: [u8; UDP_RX_SOCKET_BUFFER_SIZE],
    pub tx_storage: [u8; UDP_TX_SOCKET_BUFFER_SIZE],
    pub rx_metadata: [UdpPacketMetadata; UDP_SOCKET_METADATA_COUNT],
    pub tx_metadata: [UdpPacketMetadata; UDP_SOCKET_METADATA_COUNT],
}

impl UdpSocketStorage {
    const fn new() -> Self {
        Self {
            rx_storage: [0; UDP_RX_SOCKET_BUFFER_SIZE],
            tx_storage: [0; UDP_TX_SOCKET_BUFFER_SIZE],
            rx_metadata: [UdpPacketMetadata::EMPTY; UDP_SOCKET_METADATA_COUNT],
            tx_metadata: [UdpPacketMetadata::EMPTY; UDP_SOCKET_METADATA_COUNT],
        }
    }
}

pub struct TcpSocketStorage {
    pub rx_storage: [u8; TCP_RX_SOCKET_BUFFER_SIZE],
    pub tx_storage: [u8; TCP_TX_SOCKET_BUFFER_SIZE],
}

impl TcpSocketStorage {
    const INIT: Self = Self::new();

    const fn new() -> Self {
        Self {
            rx_storage: [0; TCP_RX_SOCKET_BUFFER_SIZE],
            tx_storage: [0; TCP_TX_SOCKET_BUFFER_SIZE],
        }
    }
}
