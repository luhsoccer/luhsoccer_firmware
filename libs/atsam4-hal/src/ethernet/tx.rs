use super::DescriptorT;
use super::MTU;
use vcell::VolatileCell;

enum Word1BitNumbers {
    LastBuffer = 15,
    DoNotAppendCRC = 16,

    LateCollision = 26,
    FrameCorrupted = 27,
    Underrun = 28,
    RetryLimitExceeded = 29,
    Wrap = 30,
    Used = 31,
}

#[repr(C)]
pub struct Descriptor(VolatileCell<u32>, VolatileCell<u32>);
impl Descriptor {
    pub fn read(&self) -> Reader {
        Reader(self.0.get(), self.1.get())
    }

    pub fn modify<F: FnOnce(Writer) -> Writer>(&self, f: F) {
        let w = Writer(self.0.get(), self.1.get());
        let result = f(w);
        self.0.set(result.0);
        self.1.set(result.1);
    }

    pub fn write<F: FnOnce(Writer) -> Writer>(&self, f: F) {
        let w = Writer(0, 0);
        let result = f(w);
        self.0.set(result.0);
        self.1.set(result.1);
    }
}

impl DescriptorT for Descriptor {
    fn new(buffer_address: *const u8, last_entry: bool) -> Self {
        let d = Descriptor(VolatileCell::new(0), VolatileCell::new(0));
        d.write(|w| w.set_used().set_address(buffer_address).set_buffer_size(0));

        if last_entry {
            d.modify(|w| w.set_wrap());
        }

        d
    }
}

pub struct Reader(pub u32, pub u32);
impl Reader {
    pub fn collided(&self) -> bool {
        self.1 & (1 << Word1BitNumbers::LateCollision as u32) != 0
    }

    pub fn corrupted(&self) -> bool {
        self.1 & (1 << Word1BitNumbers::FrameCorrupted as u32) != 0
    }

    pub fn underran(&self) -> bool {
        self.1 & (1 << Word1BitNumbers::Underrun as u32) != 0
    }

    pub fn retry_exceeded(&self) -> bool {
        self.1 & (1 << Word1BitNumbers::RetryLimitExceeded as u32) != 0
    }

    pub fn used(&self) -> bool {
        self.1 & (1 << Word1BitNumbers::Used as u32) != 0
    }
}

pub struct Writer(pub u32, pub u32);
impl Writer {
    pub fn set_address(self, address: *const u8) -> Self {
        if (address as u32) & 0x0000_0003 != 0 {
            panic!("Specified address is not 32 bit aligned");
        }
        Writer(address as u32, self.1)
    }

    pub fn set_buffer_size(self, byte_length: u16) -> Self {
        if byte_length as usize > MTU {
            panic!("Specified byte length is larger than 0x1FFFF");
        }
        Writer(self.0, (self.1 & !0x0000_1FFF) | byte_length as u32)
    }

    pub fn set_wrap(self) -> Self {
        Writer(self.0, self.1 | (1 << Word1BitNumbers::Wrap as u32))
    }

    pub fn set_used(self) -> Self {
        Writer(self.0, self.1 | (1 << Word1BitNumbers::Used as u32))
    }

    pub fn clear_used(self) -> Self {
        Writer(self.0, self.1 & !(1 << Word1BitNumbers::Used as u32))
    }

    pub fn set_last(self) -> Self {
        Writer(self.0, self.1 | (1 << Word1BitNumbers::LastBuffer as u32))
    }

    pub fn clear_crc(self) -> Self {
        Writer(
            self.0,
            self.1 & !(1 << Word1BitNumbers::DoNotAppendCRC as u32),
        )
    }
}
