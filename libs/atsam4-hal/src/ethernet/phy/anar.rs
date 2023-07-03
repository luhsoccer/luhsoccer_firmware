enum BitNumber {
    Supports802Dot3 = 0,
    Speed10MbpsHalfDuplex = 5,  // 10BASE-T Half Duplex Support
    Speed10MbpsFullDuplex = 6,  // 10BASE-T Full Duplex Support
    Speed100MbpsHalfDuplex = 7, // 100BASE-TX Half Duplex Support
    Speed100MbpsFullDuplex = 8, // 100BASE-TX Full Duplex Support
}

#[derive(Clone, Copy)]
pub struct Writer(pub(super) u16);
impl Writer {
    pub fn new(initial_value: u16) -> Self {
        Writer(initial_value)
    }

    pub fn set_802_dot_3_supported(self) -> Self {
        Self(self.0 | (1 << BitNumber::Supports802Dot3 as u32))
    }

    pub fn set_10mbps_half_duplex_supported(self) -> Self {
        Self(self.0 | (1 << BitNumber::Speed10MbpsHalfDuplex as u32))
    }

    pub fn set_10mbps_full_duplex_supported(self) -> Self {
        Self(self.0 | (1 << BitNumber::Speed10MbpsFullDuplex as u32))
    }

    pub fn set_100mbps_half_duplex_supported(self) -> Self {
        Self(self.0 | (1 << BitNumber::Speed100MbpsHalfDuplex as u32))
    }

    pub fn set_100mbps_full_duplex_supported(self) -> Self {
        Self(self.0 | (1 << BitNumber::Speed100MbpsFullDuplex as u32))
    }
}
