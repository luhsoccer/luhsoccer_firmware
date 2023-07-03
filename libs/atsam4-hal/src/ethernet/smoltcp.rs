use super::{Controller, Receiver, Transmitter, MTU};
use crate::pac::GMAC;
use smoltcp::phy::{
    ChecksumCapabilities, Device, DeviceCapabilities, Medium, RxToken, TxToken,
};
use smoltcp::time::Instant;

impl Device for Controller {
    type RxToken<'a> = EthRxToken<'a> where Self: 'a;
    type TxToken<'a> = EthTxToken<'a> where Self: 'a;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.checksum = ChecksumCapabilities::default();
        // TODO check why this is not working
        //caps.checksum.ipv4 = Checksum::None;
        //caps.checksum.tcp = Checksum::None;
        //caps.checksum.udp = Checksum::None;
        caps.max_transmission_unit = MTU as usize;
        caps.max_burst_size = Some(1);
        caps
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        return if self.tx.can_transmit() && self.rx.can_receive() {
            Some((EthRxToken(&self.rx), EthTxToken(&self.tx, &self.gmac)))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        return if self.tx.can_transmit() {
            Some(EthTxToken(&self.tx, &self.gmac))
        } else {
            None
        }
    }
}

trait SmolTcpReceiver {
    fn receive_smoltcp<R, F: FnOnce(&mut [u8]) -> R>(
        &self,
        f: F,
    ) -> R;
}

impl SmolTcpReceiver for Receiver {
    fn receive_smoltcp<R, F: FnOnce(&mut [u8]) -> R>(
        &self,
        f: F,
    ) -> R {
        let mut buffer: [u8; MTU] = [0; MTU];

        // NOTE(unwrap): This method should only be called when rx.can_receive() is true.
        let size = self.receive(&mut buffer[..]).unwrap();

        f(&mut buffer[..size])
    }
}

trait SmolTcpTransmitter {
    fn send_smoltcp<R, F: FnOnce(&mut [u8]) -> R>(
        &self,
        gmac: &GMAC,
        size: usize,
        f: F,
    ) -> R;
}

impl SmolTcpTransmitter for Transmitter {
    fn send_smoltcp<R, F: FnOnce(&mut [u8]) -> R>(
        &self,
        gmac: &GMAC,
        size: usize,
        f: F,
    ) -> R {
        let mut buffer: [u8; MTU] = [0; MTU];
        let r = f(&mut buffer[..size]);

        // NOTE(unwrap): This method should only be called when rx.can_receive() is true.
        self.send(&gmac, &buffer[..size]).unwrap();

        r
    }
}

pub struct EthRxToken<'rxtx>(&'rxtx Receiver);

impl<'rxtx> RxToken for EthRxToken<'rxtx> {
    fn consume<R, F>(self, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
    {
        self.0.receive_smoltcp(f)
    }
}

pub struct EthTxToken<'rxtx>(&'rxtx Transmitter, &'rxtx GMAC);

impl<'rxtx> TxToken for EthTxToken<'rxtx> {
    fn consume<R, F>(self, size: usize, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
    {
        self.0.send_smoltcp(self.1, size, f)
    }
}
