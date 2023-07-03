mod builder;
pub use builder::Builder as ControllerBuilder;

mod controller;
pub use controller::Controller;

mod descriptor_table;
use descriptor_table::{DescriptorT, DescriptorTable, DescriptorTableT};

mod eui48;
pub use eui48::Identifier as EthernetAddress;

mod phy;

mod receiver;
use receiver::Receiver;

mod rx;
pub type RxDescriptorTable<const COUNT: usize> = DescriptorTable<rx::Descriptor, COUNT>;
use rx::Descriptor as RxDescriptor;

mod transmitter;
use transmitter::Transmitter;

mod tx;
pub type TxDescriptorTable<const COUNT: usize> = DescriptorTable<tx::Descriptor, COUNT>;
use tx::Descriptor as TxDescriptor;

const MTU: usize = 1536;

mod smoltcp;
