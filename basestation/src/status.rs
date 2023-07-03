use crate::usb_serial::UsbSerial;
use core::fmt::Write;

#[derive(Default)]
pub struct Status {
    pub ip: Option<[u8; 4]>,
}

impl Status {
    pub fn write_to_serial(&self, serial: &mut UsbSerial<'static>) {
        let mut info = atsam4_hal::heapless::String::<32>::new();
        info.push_str("IP: ").unwrap();

        match self.ip {
            None => info.push_str("None\n\r").unwrap(),
            Some(ip) => {
                write!(info, "{}.{}.{}.{}\n\r", ip[0], ip[1], ip[2], ip[3]).unwrap();
            }
        }

        if serial.serial.dtr() {
            let _ = serial.serial.write(&info.as_bytes()[..info.len()]);
        }
    }
}
