use crate::hardware::{
    gpio::{PhyMdcPin, PhyMdioPin},
    network_clock::NetworkClock,
    NetworkManager, NetworkStack,
};
use core::fmt::Write;
use heapless::String;
use miniconf::Miniconf;
use minimq::embedded_nal::IpAddr;
use network_processor::NetworkProcessor;
use serde::Serialize;
use telemetry::TelemetryClient;

pub mod network_processor;
pub mod telemetry;

pub const MQTT_MESSAGE_SIZE_MAX: usize = 512;

pub type NetworkReference = smoltcp_nal::shared::NetworkStackProxy<'static, NetworkStack>;

#[derive(Copy, Clone, PartialEq)]
pub enum UpdateState {
    NoChange,
    Updated,
}

#[derive(Copy, Clone, PartialEq)]
pub enum NetworkState {
    SettingsChanged,
    Updated,
    NoChange,
}

pub struct NetworkUsers<S: Default + Miniconf, T: Serialize> {
    pub miniconf: miniconf::MqttClient<S, NetworkReference, NetworkClock, MQTT_MESSAGE_SIZE_MAX>,
    pub processor: NetworkProcessor,
    pub telemetry: TelemetryClient<T>,
}

impl<S, T> NetworkUsers<S, T>
where
    S: Default + Miniconf,
    T: Serialize,
{
    pub fn new(
        stack_manager: &'static mut NetworkManager,
        mdio: PhyMdioPin,
        mdc: PhyMdcPin,
        clock: NetworkClock,
        app: &str,
        mac: smoltcp_nal::smoltcp::wire::EthernetAddress,
        broker: IpAddr,
    ) -> Self {
        let processor = NetworkProcessor::new(stack_manager.acquire_stack(), mdio, mdc);

        let prefix = get_device_prefix(app, mac);

        let settings = miniconf::MqttClient::new(
            stack_manager.acquire_stack(),
            &get_client_id(app, "settings", mac),
            &prefix,
            broker,
            clock,
        )
        .unwrap();

        let telemetry = TelemetryClient::new(
            stack_manager.acquire_stack(),
            clock,
            &get_client_id(app, "tlm", mac),
            &prefix,
            broker,
        );

        NetworkUsers {
            miniconf: settings,
            processor,
            telemetry,
        }
    }

    pub fn update(&mut self) -> NetworkState {
        // Update the MQTT clients.
        self.telemetry.update();

        // Poll for incoming data.
        let poll_result = match self.processor.update() {
            UpdateState::NoChange => NetworkState::NoChange,
            UpdateState::Updated => NetworkState::Updated,
        };

        match self.miniconf.update() {
            Ok(true) => NetworkState::SettingsChanged,
            _ => poll_result,
        }
    }
}

/// Get an MQTT client ID for a client.
///
/// # Args
/// * `app` - The name of the application
/// * `client` - The unique tag of the client
/// * `mac` - The MAC address of the device.
///
/// # Returns
/// A client ID that may be used for MQTT client identification.
fn get_client_id(
    app: &str,
    client: &str,
    mac: smoltcp_nal::smoltcp::wire::EthernetAddress,
) -> String<64> {
    let mut identifier = String::new();
    write!(&mut identifier, "{}-{}-{}", app, mac, client).unwrap();
    identifier
}

/// Get the MQTT prefix of a device.
///
/// # Args
/// * `app` - The name of the application that is executing.
/// * `mac` - The ethernet MAC address of the device.
///
/// # Returns
/// The MQTT prefix used for this device.
pub fn get_device_prefix(
    app: &str,
    mac: smoltcp_nal::smoltcp::wire::EthernetAddress,
) -> String<128> {
    // Note(unwrap): The mac address + binary name must be short enough to fit into this string. If
    // they are defined too long, this will panic and the device will fail to boot.
    let mut prefix: String<128> = String::new();
    write!(&mut prefix, "dt/dummy/{}/{}", app, mac).unwrap();

    prefix
}
