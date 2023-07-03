use super::{rx::Descriptor as RxDescriptor, DescriptorTableT};

#[derive(Debug)]
pub enum Error {}

pub struct Receiver {
    pub(super) descriptors: &'static mut (dyn DescriptorTableT<RxDescriptor> + Send),
}

impl Receiver {
    pub fn new(descriptors: &'static mut (dyn DescriptorTableT<RxDescriptor> + Send)) -> Self {
        Receiver { descriptors }
    }

    pub fn can_receive(&self) -> bool {
        self.descriptors.next_descriptor().read().owned()
    }

    pub fn receive(&self, buffer: &mut [u8]) -> nb::Result<usize, Error> {
        // Check if the next entry is still being used by the GMAC...if so,
        // indicate there's no more entries and the client has to wait for one to
        // become available.
        let (next_descriptor, next_buffer) = self.descriptors.next_descriptor_pair();
        let descriptor_properties = next_descriptor.read();
        if !descriptor_properties.owned() {
            return Err(nb::Error::WouldBlock);
        }

        let buffer_size = descriptor_properties.buffer_size() as usize;
        let descriptor_buffer = next_buffer.borrow();

        buffer[..buffer_size].clone_from_slice(&descriptor_buffer[..buffer_size]);

        // Indicate that the descriptor is no longer owned by software and is available
        // for the GMAC to write into.
        next_descriptor.modify(|w| w.clear_owned());

        // This entry has been consumed, indicate this.
        self.descriptors.consume_next_descriptor();

        Ok(buffer_size)
    }
}
