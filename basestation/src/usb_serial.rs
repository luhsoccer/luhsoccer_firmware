use atsam4_hal::clock::{Disabled, UdpClock};
use atsam4_hal::gpio::{Pb10, Pb11, SysFn};
use atsam4_hal::pac::UDP;
use atsam4_hal::udp::usb_device::bus::UsbBusAllocator;
use atsam4_hal::udp::usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use atsam4_hal::udp::UdpBus;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

pub struct UsbSerial<'a> {
    pub serial: SerialPort<'a, UdpBus>,
    pub device: UsbDevice<'a, UdpBus>,
}

pub type UsbAllocator = UsbBusAllocator<UdpBus>;

pub fn new_allocator(
    udp: UDP,
    clock: UdpClock<Disabled>,
    ddm: Pb10<SysFn>,
    ddp: Pb11<SysFn>,
) -> UsbAllocator {
    UsbBusAllocator::new(UdpBus::new(udp, clock, ddm, ddp))
}

impl<'a> UsbSerial<'a> {
    pub fn new(allocator: &'a UsbAllocator) -> UsbSerial<'a> {
        let serial = SerialPort::new(allocator);
        let device = UsbDeviceBuilder::new(allocator, UsbVidPid(0x6c62, 0x6273))
            .manufacturer("luhbots soccer")
            .product("Base station")
            .serial_number("2")
            .device_class(USB_CLASS_CDC)
            .build();

        Self { serial, device }
    }

    pub fn on_interrupt(&mut self) -> bool {
        self.device.poll(&mut [&mut self.serial])
    }
}
