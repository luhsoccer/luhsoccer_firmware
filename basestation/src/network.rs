use atsam4_hal::smoltcp::iface::Config;
use atsam4_hal::{
    heapless::Vec,
    smoltcp::{
        iface::{Interface, SocketHandle, SocketSet, SocketStorage},
        phy::Device,
        socket::{dhcpv4, udp},
        time::Instant,
        wire::{DhcpOption, EthernetAddress, HardwareAddress, IpCidr, IpEndpoint, Ipv4Address},
    },
};
use defmt::{error, info, warn};
use prost::Message;
use protobuf::proto::{
    luhsoccer::{FromBasestationWrapper, ToBasestationWrapper},
    ssl_vision::SslWrapperPacket,
};

use crate::status::Status;
use crate::{app::monotonics::Monotonic, HEAP};

const MAX_SOCKETS: usize = 10;
const MAX_DHCP_OPTIONS: usize = 1;
const MAX_VISION_TX_METADATA: usize = 1;
const MAX_VISION_TX_DATA: usize = 16;
const MAX_VISION_RX_METADATA: usize = 10;
const MAX_VISION_RX_DATA: usize = 1024;
const MAX_SERVER_TX_METADATA: usize = 10;
const MAX_SERVER_TX_DATA: usize = 1024;
const MAX_SERVER_RX_METADATA: usize = 10;
const MAX_SERVER_RX_DATA: usize = 1024;

pub struct Storage<'a> {
    sockets: [SocketStorage<'a>; MAX_SOCKETS],
    dhcp_options: [DhcpOption<'a>; MAX_DHCP_OPTIONS],
    vision_tx_metadata: [udp::PacketMetadata; MAX_VISION_TX_METADATA],
    vision_tx_data: [u8; MAX_VISION_TX_DATA],
    vision_rx_metadata: [udp::PacketMetadata; MAX_VISION_RX_METADATA],
    vision_rx_data: [u8; MAX_VISION_RX_DATA],
    server_tx_metadata: [udp::PacketMetadata; MAX_SERVER_TX_METADATA],
    server_tx_data: [u8; MAX_SERVER_TX_DATA],
    server_rx_metadata: [udp::PacketMetadata; MAX_SERVER_RX_METADATA],
    server_rx_data: [u8; MAX_SERVER_RX_DATA],
}

impl Default for Storage<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage<'_> {
    pub const fn new() -> Self {
        Self {
            sockets: [SocketStorage::EMPTY; MAX_SOCKETS],
            dhcp_options: [DhcpOption {
                kind: 12,
                data: b"luhbots-bs",
            }],
            vision_tx_metadata: [udp::PacketMetadata::EMPTY; MAX_VISION_TX_METADATA],
            vision_tx_data: [0u8; MAX_VISION_TX_DATA],
            vision_rx_metadata: [udp::PacketMetadata::EMPTY; MAX_VISION_RX_METADATA],
            vision_rx_data: [0u8; MAX_VISION_RX_DATA],
            server_tx_metadata: [udp::PacketMetadata::EMPTY; MAX_SERVER_TX_METADATA],
            server_tx_data: [0u8; MAX_SERVER_TX_DATA],
            server_rx_metadata: [udp::PacketMetadata::EMPTY; MAX_SERVER_RX_METADATA],
            server_rx_data: [0u8; MAX_SERVER_RX_DATA],
        }
    }
}

pub struct Network<'s, D>
where
    D: Device,
{
    device: D,
    interface: Interface,
    sockets: SocketSet<'s>,
    dhcp_handle: SocketHandle,
    vision_handle: SocketHandle,
    server_handle: SocketHandle,
}

const SSL_VISION_MULTICAST: Ipv4Address = Ipv4Address::new(224, 5, 23, 2);
const SSL_VISION_MULTICAST_PORT: u16 = 10006;
const SERVER_PORT: u16 = 0xb45e;

impl<'s, D> Network<'s, D>
where
    D: Device,
{
    pub fn from_device(
        mut device: D,
        hardware_address: [u8; 6],
        storage: &'s mut Storage<'s>,
    ) -> Self {
        let mut config = Config::new();
        config.hardware_addr = Some(HardwareAddress::Ethernet(EthernetAddress::from_bytes(
            &hardware_address,
        )));
        let interface = Interface::new(config, &mut device);

        let mut sockets = SocketSet::new(&mut storage.sockets[..]);

        let mut dhcp = dhcpv4::Socket::new();
        dhcp.set_outgoing_options(&storage.dhcp_options);
        let dhcp_handle = sockets.add(dhcp);

        let vision_rx = udp::PacketBuffer::new(
            &mut storage.vision_rx_metadata[..],
            &mut storage.vision_rx_data[..],
        );
        let vision_tx = udp::PacketBuffer::new(
            &mut storage.vision_tx_metadata[..],
            &mut storage.vision_tx_data[..],
        );
        let vision = udp::Socket::new(vision_rx, vision_tx);
        let vision_handle = sockets.add(vision);

        let server_rx = udp::PacketBuffer::new(
            &mut storage.server_rx_metadata[..],
            &mut storage.server_rx_data[..],
        );
        let server_tx = udp::PacketBuffer::new(
            &mut storage.server_tx_metadata[..],
            &mut storage.server_tx_data[..],
        );
        let server = udp::Socket::new(server_rx, server_tx);
        let server_handle = sockets.add(server);

        Self {
            device,
            interface,
            sockets,
            dhcp_handle,
            vision_handle,
            server_handle,
        }
    }

    pub fn poll(
        &mut self,
        on_vision: impl FnOnce(SslWrapperPacket),
        on_server: impl FnOnce(ToBasestationWrapper, IpEndpoint),
        status: &mut Status,
    ) {
        #[allow(clippy::cast_possible_wrap)]
        let now = Instant::from_millis(Monotonic::now().duration_since_epoch().to_millis() as i64);
        if self
            .interface
            .poll(now, &mut self.device, &mut self.sockets)
        {
            match self
                .sockets
                .get_mut::<dhcpv4::Socket>(self.dhcp_handle)
                .poll()
            {
                Some(dhcpv4::Event::Configured(config)) => {
                    update_with_dhcp(&mut self.interface, &config, status);
                    self.interface
                        .join_multicast_group(&mut self.device, SSL_VISION_MULTICAST, now)
                        .unwrap();
                    info!("Joined multicast group");
                }
                Some(dhcpv4::Event::Deconfigured) => {
                    info!("Dhcp deconfigured!");
                    self.interface.update_ip_addrs(Vec::clear);
                    let _ = self.interface.routes_mut().remove_default_ipv4_route();
                }
                _ => {}
            }

            let vision = self.sockets.get_mut::<udp::Socket>(self.vision_handle);
            if !vision.is_open() {
                info!("Open ssl vision socket");
                vision.bind(SSL_VISION_MULTICAST_PORT).ok();
            }
            if vision.can_recv() {
                vision.recv().map_or_else(
                    |_| {
                        info!("Error while reading tcp data");
                    },
                    |(data, _sender)| {
                        SslWrapperPacket::decode(data).map_or_else(
                            |_| error!("decoding protobuf packet from vision"),
                            on_vision,
                        );
                        if HEAP.used() != 0 {
                            // Memory should always free after this
                            warn!("Memory leak! Something bad will happen");
                        }
                    },
                );
            }

            let server = self.sockets.get_mut::<udp::Socket>(self.server_handle);
            if !server.is_open() {
                for cidr in self.interface.ip_addrs() {
                    info!("Opening server socket");
                    server
                        .bind(IpEndpoint::new(cidr.address(), SERVER_PORT))
                        .ok();
                }
            }
            if server.can_recv() {
                server.recv().map_or_else(
                    |_| {
                        error!("while reading udp server dada");
                    },
                    |(data, sender)| {
                        if let Ok(packet) = ToBasestationWrapper::decode(data) {
                            on_server(packet, sender);
                        } else {
                            warn!("Error while decoding protobuf packet from server");
                        }
                    },
                );
            }
        }
    }

    pub fn send_feedback(&mut self, feedback: &FromBasestationWrapper, endpoint: IpEndpoint) {
        let server = self.sockets.get_mut::<udp::Socket>(self.server_handle);
        if server.can_send() {
            let data = feedback.encode_to_vec();
            match server.send_slice(&data[..], endpoint) {
                Ok(_) => (),
                Err(udp::SendError::Unaddressable) => {
                    error!("unable to address server");
                }
                Err(udp::SendError::BufferFull) => {
                    error!("buffer full");
                }
            }
        } else {
            error!("unable to send feedback because the buffer is full");
        }
    }
}

fn update_with_dhcp(interface: &mut Interface, config: &dhcpv4::Config, status: &mut Status) {
    info!(
        "Got IPv4: {} and default gateway: {}",
        &config.address.address().0,
        &config.router.unwrap().0
    );
    status.ip = Some(config.address.address().0);
    interface.routes_mut().remove_default_ipv4_route();
    interface
        .routes_mut()
        .add_default_ipv4_route(config.router.unwrap())
        .unwrap();
    interface.update_ip_addrs(|addrs| {
        addrs.clear();
        addrs.push(IpCidr::Ipv4(config.address)).unwrap();
    });
}
