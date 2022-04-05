pub mod eth;
pub mod gpio;
pub mod net;
pub mod network_clock;
pub mod phy;

pub type NetworkStack = smoltcp_nal::NetworkStack<
    'static,
    stm32_eth::Eth<'static, 'static>,
    network_clock::NetworkClock,
>;

pub type NetworkManager = smoltcp_nal::shared::NetworkManager<
    'static,
    stm32_eth::Eth<'static, 'static>,
    network_clock::NetworkClock,
>;
