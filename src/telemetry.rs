use serde::Serialize;

#[derive(Serialize, Copy, Clone, Default, Debug)]
pub struct Telemetry {
    pub dummy: u32,
}
