enum BitNumbers {
    _ExtendedCapability = 0,
    _JabberDetected = 1,
    LinkDetected = 2,
    _AutoNegotiationCapable = 3,
    _RemoteFaultDetected = 4,
    AutoNegotiationComplete = 5,
    _PreambleSuppressionCapable = 6,
    _HalfDuplex10BaseTCapable = 11,
    _FullDuplex10BaseTCapable = 12,
    HalfDuplex100BaseTXCapable = 13,
    FullDuplex100BaseTXCapable = 14,
}

#[derive(Clone, Copy)]
pub struct Reader(u16);
impl Reader {
    pub fn new(initial_value: u16) -> Self {
        Reader(initial_value)
    }

    pub fn _has_extended_capability(&self) -> bool {
        self.0 & (1 << BitNumbers::_ExtendedCapability as u32) != 0
    }

    pub fn _jabber_detected(&self) -> bool {
        self.0 & (1 << BitNumbers::_JabberDetected as u32) != 0
    }

    pub fn link_detected(&self) -> bool {
        self.0 & (1 << BitNumbers::LinkDetected as u32) != 0
    }

    pub fn _auto_negotiation_capable(&self) -> bool {
        self.0 & (1 << BitNumbers::_AutoNegotiationCapable as u32) != 0
    }

    pub fn _remote_fault_detected(&self) -> bool {
        self.0 & (1 << BitNumbers::_RemoteFaultDetected as u32) != 0
    }

    pub fn auto_negotiation_complete(&self) -> bool {
        self.0 & (1 << BitNumbers::AutoNegotiationComplete as u32) != 0
    }

    pub fn _preamble_suppression_capable(&self) -> bool {
        self.0 & (1 << BitNumbers::_PreambleSuppressionCapable as u32) != 0
    }

    pub fn _half_duplex_10base_t_capable(&self) -> bool {
        self.0 & (1 << BitNumbers::_HalfDuplex10BaseTCapable as u32) != 0
    }

    pub fn _full_duplex_10base_t_capable(&self) -> bool {
        self.0 & (1 << BitNumbers::_FullDuplex10BaseTCapable as u32) != 0
    }

    pub fn _half_duplex_100base_tx_capable(&self) -> bool {
        self.0 & (1 << BitNumbers::HalfDuplex100BaseTXCapable as u32) != 0
    }

    pub fn _full_duplex_100base_tx_capable(&self) -> bool {
        self.0 & (1 << BitNumbers::FullDuplex100BaseTXCapable as u32) != 0
    }

    pub fn _is_full_duplex(&self) -> bool {
        (self.0
            & (1 << BitNumbers::_FullDuplex10BaseTCapable as u32)
            & (1 << BitNumbers::FullDuplex100BaseTXCapable as u32))
            != 0
    }

    pub fn _is_10mbit(&self) -> bool {
        (self.0
            & ((1 << BitNumbers::_HalfDuplex10BaseTCapable as u32)
                | (1 << BitNumbers::_FullDuplex10BaseTCapable as u32)))
            != 0
    }

    pub fn is_100mbit(&self) -> bool {
        (self.0
            & ((1 << BitNumbers::HalfDuplex100BaseTXCapable as u32)
                | (1 << BitNumbers::FullDuplex100BaseTXCapable as u32)))
            != 0
    }

    pub fn _speed(&self) -> u32 {
        if self._is_10mbit() {
            10
        } else if self.is_100mbit() {
            100
        } else {
            0
        }
    }
}
