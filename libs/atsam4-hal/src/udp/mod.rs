//! UDP (USB Device Port) Implementation
//! NOTE: From ASF the following MCUs could possibly be supported with this module
//!       atsam3s
//!       atsam4e (supported)
//!       atsam4s (supported)
//!       atsamg55

pub use usb_device;

mod bus;
pub use self::bus::UdpBus;

mod endpoint;
pub use self::endpoint::Endpoint;

use crate::pac::UDP;
use crate::BorrowUnchecked;

use usb_device::{
    endpoint::{EndpointAddress, EndpointType},
    UsbDirection,
};

/// Wrapper for defmt
struct UdpEndpointAddress {
    inner: Option<EndpointAddress>,
}

impl defmt::Format for UdpEndpointAddress {
    fn format(&self, fmt: defmt::Formatter) {
        if let Some(ep_addr) = self.inner {
            defmt::write!(fmt, "EndpointAddress({=u8})", ep_addr.into());
        } else {
            defmt::write!(fmt, "EndpointAddress(None)");
        }
    }
}

/// Wrapper for defmt
struct UdpEndpointType {
    inner: EndpointType,
}

impl defmt::Format for UdpEndpointType {
    fn format(&self, fmt: defmt::Formatter) {
        match self.inner {
            EndpointType::Control => defmt::write!(fmt, "EndpointType::Control"),
            EndpointType::Isochronous => defmt::write!(fmt, "EndpointType::Isochronous"),
            EndpointType::Bulk => defmt::write!(fmt, "EndpointType::Bulk"),
            EndpointType::Interrupt => defmt::write!(fmt, "EndpointType::Interrupt"),
        }
    }
}

/// Wrapper for defmt
struct UdpUsbDirection {
    inner: UsbDirection,
}

impl defmt::Format for UdpUsbDirection {
    fn format(&self, fmt: defmt::Formatter) {
        match self.inner {
            UsbDirection::In => defmt::write!(fmt, "UsbDirection::In"),
            UsbDirection::Out => defmt::write!(fmt, "UsbDirection::Out"),
        }
    }
}

/// Retrieve current frame number (updated on SOF_EOP)
pub fn frm_num() -> u16 {
    UDP::borrow_unchecked(|udp| udp.frm_num.read().frm_num().bits())
}
