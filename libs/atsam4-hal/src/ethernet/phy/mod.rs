pub enum Register {
    Bmcr = 0x00,
    Bmsr = 0x01,
    Anar = 0x04,   // Auto Negotiation Advertisement Register
    Anlpar = 0x05, // Auto Negotiation Link Partner Ability Register
    Pcr1 = 0x1E,   // Phy Control Register #1
}

mod anar; // Auto Negotiation Advertisement Register
pub(super) use anar::Writer as AnarWriter;
mod bmcr; // Basic Mode Control Register
pub(super) use bmcr::Reader as BmcrReader;
pub(super) use bmcr::Writer as BmcrWriter;
mod bmsr; // Basic Mode Status Register
pub(super) use bmsr::Reader as BmsrReader;
use defmt::Format;

mod pcr1;
pub(super) use pcr1::Reader as Pcr1Reader;

mod anlpar; // Auto Negotiation Link Partner Ability Register
pub(super) use anlpar::Reader as AnlparReader;

#[derive(Format)]
pub enum LinkType {
    HalfDuplex10,
    FullDuplex10,
    HalfDuplex100,
    FullDuplex100,
}

pub trait Phy {
    fn reset_phy(&mut self) {
        self.enable_phy_management_port();

        self.write_phy_bmcr_(|w| w.set_reset());

        // Wait for the PHY to be reset
        // !todo - use timeout.
        while self.read_phy_bmcr_().reset().is_set() {}

        self.enable_phy_management_port();
    }

    fn enable_phy_auto_negotiation(&mut self) -> LinkType {
        self.modify_phy_bmcr(|w| {
            w.clear_enable_auto_negotiation()
                .clear_loop_back()
                .clear_power_down()
                .set_isolate()
        });

        self.modify_phy_anar(|w| {
            w.set_802_dot_3_supported()
                .set_10mbps_half_duplex_supported()
                .set_10mbps_full_duplex_supported()
                .set_100mbps_half_duplex_supported()
                .set_100mbps_full_duplex_supported()
        });

        self.modify_phy_bmcr(|w| {
            w.set_speed_100()
                .set_enable_auto_negotiation()
                .set_full_duplex()
        });

        // Restart auto-negotiation (this must be done with managment port enabled)
        {
            self.enable_phy_management_port();
            self.modify_phy_bmcr_(|w| w.set_auto_negotiation_restart().clear_isolate());

            // Wait for auto-negotiation to complete
            // !todo - use timeout.
            while !self.read_phy_bmsr().auto_negotiation_complete() {}
            self.disable_phy_management_port();
        }

        // Get the auto-negotiation partner ability
        let partner = self.read_phy_anlpar();
        if partner.is_100mbps_full_duplex_supported() {
            LinkType::FullDuplex100
        } else if partner.is_100mbps_half_duplex_supported() {
            LinkType::HalfDuplex100
        } else if partner.is_10mbps_full_duplex_supported() {
            LinkType::FullDuplex10
        } else {
            LinkType::HalfDuplex10
        }
    }

    // ANAR
    fn modify_phy_anar<F: FnOnce(AnarWriter) -> AnarWriter>(&mut self, f: F) {
        self.enable_phy_management_port();
        let w = AnarWriter::new(self.read_phy_register(Register::Anar));
        let new_value = f(w);
        self.write_phy_register(Register::Anar, new_value.0);
        self.disable_phy_management_port();
    }

    // BMCR
    fn read_phy_bmcr(&self) -> BmcrReader {
        self.enable_phy_management_port();
        let value = self.read_phy_bmcr_();
        self.disable_phy_management_port();
        value
    }

    fn read_phy_bmcr_(&self) -> BmcrReader {
        BmcrReader::new(self.read_phy_register(Register::Bmcr))
    }

    fn modify_phy_bmcr<F: FnOnce(BmcrWriter) -> BmcrWriter>(&mut self, f: F) {
        self.enable_phy_management_port();
        self.modify_phy_bmcr_(f);
        self.disable_phy_management_port();
    }

    fn modify_phy_bmcr_<F: FnOnce(BmcrWriter) -> BmcrWriter>(&mut self, f: F) {
        let w = BmcrWriter::new(self.read_phy_register(Register::Bmcr));
        let new_value = f(w);
        self.write_phy_register(Register::Bmcr, new_value.0);
    }

    fn write_phy_bmcr<F: FnOnce(BmcrWriter) -> BmcrWriter>(&mut self, f: F) {
        self.enable_phy_management_port();
        self.write_phy_bmcr_(f);
        self.disable_phy_management_port();
    }

    fn write_phy_bmcr_<F: FnOnce(BmcrWriter) -> BmcrWriter>(&mut self, f: F) {
        let w = BmcrWriter::new(0);
        let new_value = f(w);
        self.write_phy_register(Register::Bmcr, new_value.0);
    }

    // BMSR
    fn read_phy_bmsr(&self) -> BmsrReader {
        self.enable_phy_management_port();
        let value = BmsrReader::new(self.read_phy_register(Register::Bmsr));
        self.disable_phy_management_port();
        value
    }

    // PCR1
    fn read_phy_pcr1(&self) -> Pcr1Reader {
        self.enable_phy_management_port();
        let value = Pcr1Reader::new(self.read_phy_register(Register::Pcr1));
        self.disable_phy_management_port();
        value
    }

    // ANLPAR
    fn read_phy_anlpar(&self) -> AnlparReader {
        self.enable_phy_management_port();
        let value = AnlparReader::new(self.read_phy_register(Register::Anlpar));
        self.disable_phy_management_port();
        value
    }

    fn read_phy_register(&self, register: Register) -> u16;
    fn write_phy_register(&mut self, register: Register, new_value: u16);
    fn enable_phy_management_port(&self);
    fn disable_phy_management_port(&self);
}
