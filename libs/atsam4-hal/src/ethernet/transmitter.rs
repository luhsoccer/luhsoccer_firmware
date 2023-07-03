use super::{tx::Descriptor as TxDescriptor, DescriptorTableT, MTU};
use crate::pac::GMAC;

#[derive(Debug)]
pub enum Error {}

pub struct Transmitter {
    pub(super) descriptors: &'static mut (dyn DescriptorTableT<TxDescriptor> + Send),
}

impl Transmitter {
    pub fn new(descriptors: &'static mut (dyn DescriptorTableT<TxDescriptor> + Send)) -> Self {
        Transmitter { descriptors }
    }

    pub fn can_transmit(&self) -> bool {
        return self.descriptors.next_descriptor().read().used();
    }

    pub fn send(&self, gmac: &GMAC, buffer: &[u8]) -> nb::Result<(), Error> {
        let buffer_length = buffer.len();
        if buffer_length > MTU {
            panic!("ERROR: Requested to send a buffer larger than the MTU")
        }

        // Check if the next entry is still being used by the GMAC...if so,
        // indicate there's no more entries and the client has to wait for one to
        // become available.
        let (next_descriptor, next_buffer) = self.descriptors.next_descriptor_pair();
        if !next_descriptor.read().used() {
            return Err(nb::Error::WouldBlock);
        }

        // Copy the input buffer into the descriptor's buffer.
        let mut descriptor_buffer = next_buffer.borrow_mut();
        descriptor_buffer[..buffer_length].clone_from_slice(&buffer);

        // Set up the descriptor.
        next_descriptor.modify(|w| {
            w.set_buffer_size(buffer_length as u16)
                .clear_used()
                .set_last()
                .clear_crc()
        });

        // Start the transmission
        Self::start_transmission(&gmac);

        Ok(())
    }

    fn start_transmission(gmac: &GMAC) {
        gmac.ncr.modify(|_, w| w.tstart().set_bit());
    }
}
