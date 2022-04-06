use super::{NetworkReference, MQTT_MESSAGE_SIZE_MAX};
use crate::hardware::network_clock::NetworkClock;
use heapless::{String, Vec};
use minimq::embedded_nal::IpAddr;
use minimq::{QoS, Retain};
use serde::Serialize;

const MQTT_MSG_COUNT: usize = 1;

pub struct TelemetryClient<T: Serialize> {
    mqtt: minimq::Minimq<NetworkReference, NetworkClock, MQTT_MESSAGE_SIZE_MAX, MQTT_MSG_COUNT>,
    telemetry_topic: String<128>,
    _telemetry: core::marker::PhantomData<T>,
}

impl<T: Serialize> TelemetryClient<T> {
    pub fn new(
        stack: NetworkReference,
        clock: NetworkClock,
        client_id: &str,
        prefix: &str,
        broker: IpAddr,
    ) -> Self {
        let mqtt = minimq::Minimq::new(broker, client_id, stack, clock).unwrap();

        let mut telemetry_topic: String<128> = String::from(prefix);
        telemetry_topic.push_str("/telemetry").unwrap();

        Self {
            mqtt,
            telemetry_topic,
            _telemetry: core::marker::PhantomData::default(),
        }
    }

    pub fn publish(&mut self, telemetry: &T) {
        let telemetry: Vec<u8, MQTT_MESSAGE_SIZE_MAX> = serde_json_core::to_vec(telemetry).unwrap();
        self.mqtt
            .client
            .publish(
                &self.telemetry_topic,
                &telemetry,
                QoS::AtMostOnce,
                Retain::NotRetained,
                &[],
            )
            .ok();
    }

    pub fn update(&mut self) {
        match self.mqtt.poll(|_client, _topic, _message, _properties| {}) {
            Err(minimq::Error::Network(smoltcp_nal::NetworkError::NoIpAddress)) => {}

            Err(error) => log::info!("Unexpected error: {:?}", error),
            _ => {}
        }
    }
}
