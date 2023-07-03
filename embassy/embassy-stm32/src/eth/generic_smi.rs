//! Generic SMI Ethernet PHY

use super::{StationManagement, PHY};

#[allow(dead_code)]
mod phy_consts {
    pub const PHY_REG_BCR: u8 = 0x00;
    pub const PHY_REG_BSR: u8 = 0x01;
    pub const PHY_REG_ID1: u8 = 0x02;
    pub const PHY_REG_ID2: u8 = 0x03;
    pub const PHY_REG_ANTX: u8 = 0x04;
    pub const PHY_REG_ANRX: u8 = 0x05;
    pub const PHY_REG_ANEXP: u8 = 0x06;
    pub const PHY_REG_ANNPTX: u8 = 0x07;
    pub const PHY_REG_ANNPRX: u8 = 0x08;
    pub const PHY_REG_CTL: u8 = 0x0D; // Ethernet PHY Register Control
    pub const PHY_REG_ADDAR: u8 = 0x0E; // Ethernet PHY Address or Data

    pub const PHY_REG_WUCSR: u16 = 0x8010;

    pub const PHY_REG_BCR_COLTEST: u16 = 1 << 7;
    pub const PHY_REG_BCR_FD: u16 = 1 << 8;
    pub const PHY_REG_BCR_ANRST: u16 = 1 << 9;
    pub const PHY_REG_BCR_ISOLATE: u16 = 1 << 10;
    pub const PHY_REG_BCR_POWERDN: u16 = 1 << 11;
    pub const PHY_REG_BCR_AN: u16 = 1 << 12;
    pub const PHY_REG_BCR_100M: u16 = 1 << 13;
    pub const PHY_REG_BCR_LOOPBACK: u16 = 1 << 14;
    pub const PHY_REG_BCR_RESET: u16 = 1 << 15;

    pub const PHY_REG_BSR_JABBER: u16 = 1 << 1;
    pub const PHY_REG_BSR_UP: u16 = 1 << 2;
    pub const PHY_REG_BSR_FAULT: u16 = 1 << 4;
    pub const PHY_REG_BSR_ANDONE: u16 = 1 << 5;
}
use self::phy_consts::*;

/// Generic SMI Ethernet PHY
pub struct GenericSMI;

unsafe impl PHY for GenericSMI {
    /// Reset PHY and wait for it to come out of reset.
    fn phy_reset<S: StationManagement>(sm: &mut S) {
        sm.smi_write(PHY_REG_BCR, PHY_REG_BCR_RESET);
        while sm.smi_read(PHY_REG_BCR) & PHY_REG_BCR_RESET == PHY_REG_BCR_RESET {}
    }

    /// PHY initialisation.
    fn phy_init<S: StationManagement>(sm: &mut S) {
        // Clear WU CSR
        Self::smi_write_ext(sm, PHY_REG_WUCSR, 0);

        // Enable auto-negotiation
        sm.smi_write(PHY_REG_BCR, PHY_REG_BCR_AN | PHY_REG_BCR_ANRST | PHY_REG_BCR_100M);
    }

    fn poll_link<S: StationManagement>(sm: &mut S) -> bool {
        let bsr = sm.smi_read(PHY_REG_BSR);

        // No link without autonegotiate
        if bsr & PHY_REG_BSR_ANDONE == 0 {
            return false;
        }
        // No link if link is down
        if bsr & PHY_REG_BSR_UP == 0 {
            return false;
        }

        // Got link
        true
    }
}

/// Public functions for the PHY
impl GenericSMI {
    // Writes a value to an extended PHY register in MMD address space
    fn smi_write_ext<S: StationManagement>(sm: &mut S, reg_addr: u16, reg_data: u16) {
        sm.smi_write(PHY_REG_CTL, 0x0003); // set address
        sm.smi_write(PHY_REG_ADDAR, reg_addr);
        sm.smi_write(PHY_REG_CTL, 0x4003); // set data
        sm.smi_write(PHY_REG_ADDAR, reg_data);
    }
}
