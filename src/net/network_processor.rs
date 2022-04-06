use super::{NetworkReference, UpdateState};
use crate::hardware::{
    gpio::{PhyMdcPin, PhyMdioPin},
    phy::Phy,
};
use log::warn;

pub struct NetworkProcessor {
    stack: NetworkReference,
    mdio: PhyMdioPin,
    mdc: PhyMdcPin,
    network_was_reset: bool,
}

impl NetworkProcessor {
    pub fn new(stack: NetworkReference, mdio: PhyMdioPin, mdc: PhyMdcPin) -> Self {
        Self {
            stack,
            mdio,
            mdc,
            network_was_reset: false,
        }
    }

    pub fn handle_link(&mut self) -> bool {
        let link_up = self.stack.lock(|stack| {
            let smi = stack
                .interface_mut()
                .device_mut()
                .smi(&mut self.mdio, &mut self.mdc);
            let phy = Phy::new(smi);
            phy.link_status()
        });
        match (link_up, self.network_was_reset) {
            (true, true) => {
                warn!("Network link UP");
                self.network_was_reset = false;
            }
            (false, false) => {
                warn!("Network link DOWN");
                self.network_was_reset = true;
                self.stack.lock(|stack| stack.handle_link_reset());
            }
            _ => {}
        };
        link_up
    }

    pub fn handle_interrupt(&mut self) {
        self.stack
            .lock(|stack| stack.interface_mut().device_mut().interrupt_handler());
    }

    pub fn update(&mut self) -> UpdateState {
        match self.stack.lock(|stack| stack.poll()) {
            Ok(true) => UpdateState::Updated,
            Ok(false) => UpdateState::NoChange,
            Err(_) => UpdateState::Updated,
        }
    }
}
