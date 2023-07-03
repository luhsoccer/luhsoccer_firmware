enum BitNumber {
    FullDuplex = 8,
    RestartAutoNegotiation = 9,
    Isolate = 10,
    PowerDown = 11,
    EnableAutoNegotiation = 12,
    Speed100Mbps = 13,
    LoopBack = 14,
    Reset = 15,
}

#[derive(Clone, Copy)]
pub struct Writer(pub(super) u16);
impl Writer {
    pub fn new(initial_value: u16) -> Self {
        Writer(initial_value)
    }

    pub fn set_full_duplex(self) -> Self {
        Self(self.0 | (1 << BitNumber::FullDuplex as u32))
    }

    pub fn set_auto_negotiation_restart(self) -> Self {
        Self(self.0 | (1 << BitNumber::RestartAutoNegotiation as u32))
    }

    pub fn set_isolate(self) -> Self {
        Self(self.0 | (1 << BitNumber::Isolate as u32))
    }

    pub fn clear_isolate(self) -> Self {
        Self(self.0 & !(1 << BitNumber::Isolate as u32))
    }

    pub fn _set_power_down(self) -> Self {
        Self(self.0 | (1 << BitNumber::PowerDown as u32))
    }

    pub fn clear_power_down(self) -> Self {
        Self(self.0 & !(1 << BitNumber::PowerDown as u32))
    }

    pub fn set_enable_auto_negotiation(self) -> Self {
        Self(self.0 | (1 << BitNumber::EnableAutoNegotiation as u32))
    }

    pub fn clear_enable_auto_negotiation(self) -> Self {
        Self(self.0 & !(1 << BitNumber::EnableAutoNegotiation as u32))
    }

    pub fn set_speed_100(self) -> Self {
        Self(self.0 | (1 << BitNumber::Speed100Mbps as u32))
    }

    pub fn _set_loop_back(self) -> Self {
        Self(self.0 | (1 << BitNumber::LoopBack as u32))
    }

    pub fn clear_loop_back(self) -> Self {
        Self(self.0 & !(1 << BitNumber::LoopBack as u32))
    }

    pub fn set_reset(self) -> Self {
        Self(self.0 | (1 << BitNumber::Reset as u32))
    }
}

pub struct Reset(bool);
impl Reset {
    pub fn is_set(&self) -> bool {
        self.0
    }
}

pub struct Reader(u16);
impl Reader {
    pub fn new(value: u16) -> Self {
        Reader(value)
    }

    pub fn reset(&self) -> Reset {
        Reset((self.0 & (1 << BitNumber::Reset as u32)) != 0)
    }
}
