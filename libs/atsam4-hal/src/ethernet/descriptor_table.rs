use super::MTU;
use core::cell::Cell;
use core::cell::RefCell;
use heapless::Vec;

// In order to keep the buffers 32 bit aligned (required by the hardware), we adjust
// the size here to be the next 4 byte multiple greater than the requested MTU.
const BUFFERSIZE: usize = (MTU & !3) + 4;

pub trait DescriptorT {
    fn new(buffer_address: *const u8, last_entry: bool) -> Self;
}

pub trait DescriptorTableT<DESCRIPTOR> {
    fn initialize(&mut self);
    fn base_address(&self) -> u32;
    fn next_descriptor(&self) -> &DESCRIPTOR;
    fn next_descriptor_pair(&self) -> (&DESCRIPTOR, &RefCell<Vec<u8, BUFFERSIZE>>);
    fn consume_next_descriptor(&self);
}

#[repr(C)]
pub struct DescriptorTable<DESCRIPTOR, const COUNT: usize> {
    descriptors: Vec<DESCRIPTOR, COUNT>,
    buffers: Vec<RefCell<Vec<u8, BUFFERSIZE>>, COUNT>,
    //    buffers: [RefCell<[u8; BUFFERSIZE]>; COUNT],
    next_entry: Cell<usize>, // Index of next entry to read/write
}

impl<DESCRIPTOR, const COUNT: usize> DescriptorTable<DESCRIPTOR, COUNT> {
    pub const fn new() -> Self {
        DescriptorTable {
            descriptors: Vec::new(),
            //            buffers: [RefCell::new([0; BUFFERSIZE]); COUNT],
            buffers: Vec::new(),
            next_entry: Cell::new(0),
        }
    }
}

impl<DESCRIPTOR: DescriptorT, const COUNT: usize> DescriptorTableT<DESCRIPTOR>
    for DescriptorTable<DESCRIPTOR, COUNT>
{
    fn initialize(&mut self) {
        self.descriptors.truncate(0);
        for i in 0..COUNT {
            // Create the new buffer and fill it with 0.
            self.buffers.push(RefCell::new(Vec::new())).ok();
            self.buffers[i].borrow_mut().resize(BUFFERSIZE, 0).ok();

            let buffer_address = &self.buffers[i].borrow()[0];
            let descriptor = DESCRIPTOR::new(buffer_address, i == COUNT - 1);
            self.descriptors.push(descriptor).ok();
        }
    }

    fn base_address(&self) -> u32 {
        let address: *const DESCRIPTOR = &self.descriptors[0];
        let a = address as u32;
        if a & 0x0000_0003 != 0 {
            panic!("Unaligned buffer address in descriptor table")
        }
        a
    }

    fn next_descriptor(&self) -> &DESCRIPTOR {
        &self.descriptors[self.next_entry.get()]
    }

    fn next_descriptor_pair(&self) -> (&DESCRIPTOR, &RefCell<Vec<u8, BUFFERSIZE>>) {
        let next_entry = self.next_entry.get();
        (&self.descriptors[next_entry], &self.buffers[next_entry])
    }

    fn consume_next_descriptor(&self) {
        let next_entry = self.next_entry.get();
        if next_entry == COUNT - 1 {
            self.next_entry.set(0);
        } else {
            self.next_entry.set(next_entry + 1);
        }
    }
}
