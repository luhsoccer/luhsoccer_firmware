use super::DescriptorT;
use vcell::VolatileCell;

enum Word0BitNumbers {
    Owned = 0,
    Wrap = 1,
}

enum Word1BitNumbers {
    Checksum0 = 22,
    Checksum1 = 23,
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
        d.write(|w| w.set_address(buffer_address).clear_owned());

        if last_entry {
            d.modify(|w| w.set_wrap());
        }

        d
    }
}

pub struct Reader(u32, u32);
impl Reader {
    pub fn owned(&self) -> bool {
        self.0 & (1 << Word0BitNumbers::Owned as u32) != 0x0
    }

    pub fn buffer_size(&self) -> u16 {
        //!todo - If jumbo frames are enabled, this needs to take into account the 13th bit as well.
        (self.1 & 0x0000_0FFF) as u16
    }

    pub fn checksum_checked(&self) -> bool {
        let c0 = self.1 & (1 << Word1BitNumbers::Checksum0 as u32) != 0x00;
        let c1 = self.1 & (1 << Word1BitNumbers::Checksum0 as u32) != 0x00;

        c0 || c1
    }
}

pub struct Writer(u32, u32);
impl Writer {
    pub fn set_address(self, address: *const u8) -> Self {
        if (address as u32) & 0x0000_0003 != 0 {
            panic!("Specified address is not 32 bit aligned");
        }
        Writer(self.0 | ((address as u32) & !0x03), self.1)
    }

    pub fn set_owned(self) -> Self {
        Writer(self.0 | (1 << Word0BitNumbers::Owned as u32), self.1)
    }

    pub fn clear_owned(self) -> Self {
        Writer(self.0 & !(1 << Word0BitNumbers::Owned as u32), self.1)
    }

    pub fn set_wrap(self) -> Self {
        Writer(self.0 | (1 << Word0BitNumbers::Wrap as u32), self.1)
    }
}
