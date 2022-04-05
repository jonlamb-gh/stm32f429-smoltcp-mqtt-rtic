// https://www.st.com/resource/en/technical_note/tn1305-network-management-interfaces-stmicroelectronics.pdf

#![allow(dead_code)]

use modular_bitfield::prelude::*;
use stm32_eth::smi::{MdcPin, MdioPin, Smi};
use stm32f4xx_hal::hal::blocking::delay::DelayMs;

const PHY_ADDR: u8 = 0;

/// Basic mode control register
#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug)]
pub struct Bmcr {
    #[skip]
    __: B7,
    collision_test: bool,
    force_fd: bool,
    restart_an: bool,
    isolate: bool,
    power_down: bool,
    an_enable: bool,
    force_100: bool,
    loopback: bool,
    soft_reset: bool,
}

impl Bmcr {
    pub const ADDRESS: u8 = 0x00;
}

/// Basic mode status register
#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug)]
pub struct Bmsr {
    extended_capable: bool,
    jabber_test: bool,
    link_status: bool,
    an_capable: bool,
    remote_fault: bool,
    an_complete: bool,
    #[skip]
    __: B5,
    capable_10_hd: bool,
    capable_10_fd: bool,
    capable_100_hd: bool,
    capable_100_fd: bool,
    capable_t4: bool,
}

impl Bmsr {
    pub const ADDRESS: u8 = 0x01;
}

pub struct Phy<'eth, 'pins, Mdio, Mdc> {
    smi: Smi<'eth, 'pins, Mdio, Mdc>,
}

impl<'eth, 'pins, Mdio, Mdc> Phy<'eth, 'pins, Mdio, Mdc>
where
    Mdio: MdioPin,
    Mdc: MdcPin,
{
    pub fn new(smi: Smi<'eth, 'pins, Mdio, Mdc>) -> Self {
        Self { smi }
    }

    pub fn reset<D: DelayMs<u32>>(&self, delay: &mut D) {
        let mut w = Bmcr::from(self.smi.read(PHY_ADDR, Bmcr::ADDRESS));
        w.set_soft_reset(true);
        self.smi.write(PHY_ADDR, Bmcr::ADDRESS, w.into());
        loop {
            delay.delay_ms(1);
            let r = Bmcr::from(self.smi.read(PHY_ADDR, Bmcr::ADDRESS));
            if !r.soft_reset() {
                break;
            }
        }
    }

    pub fn setup(&self) {
        let mut w = Bmcr::from(self.smi.read(PHY_ADDR, Bmcr::ADDRESS));
        w.set_force_fd(true);
        w.set_force_100(true);
        self.smi.write(PHY_ADDR, Bmcr::ADDRESS, w.into());
    }

    pub fn link_status(&self) -> bool {
        let r = Bmsr::from(self.smi.read(PHY_ADDR, Bmsr::ADDRESS));
        r.link_status()
    }
}
