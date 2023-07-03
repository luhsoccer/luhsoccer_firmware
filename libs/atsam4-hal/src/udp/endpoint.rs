use crate::pac::udp::csr;
use crate::pac::udp::csr::EPTYPE_A;
use crate::pac::UDP;
use crate::udp::{frm_num, UdpEndpointAddress, UdpEndpointType, UdpUsbDirection};
use crate::BorrowUnchecked;
use paste::paste;
use usb_device::{
    bus::PollResult,
    endpoint::{EndpointAddress, EndpointType},
    UsbDirection,
};

/// Needed to support ping-pong buffers
/// If interrupts are processed too slowly, it's possible that both Rx banks have been filled.
/// There is no way from the register interface to know which buffer is first so we have to
/// keep track of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
enum NextBank {
    Bank0,
    Bank1,
}

/// atsam4 has two registers dedicated to each endpoint
/// CSRx and FDRx
/// Most of the relevant information about the endpoint can be queried directly from the registers.
/// TODO: Isochronous not implemented
pub struct Endpoint {
    index: u8,
    interval: u8,
    max_packet_size: u16,
    ep_type: EndpointType,
    ep_dir: UsbDirection,
    next_bank: NextBank,
    allocated: bool,
    txbanks_free: u8,
    /// Used to keep track of stalled state, we cannot use the registers to entirely track the
    /// status
    stalled: bool,
    /// During a Control Write transaction it's possible to detect incoming zlp early
    /// This flag will make the next read() a zlp without checking FDR
    next_out_zlp: bool,
}

macro_rules! clear_ep {
    (
        $udp:ident,
        $ep:ident,
        $epnum:expr
    ) => {{
        // Set
        $udp.rst_ep.modify(|_, w| w.$ep().set_bit());
        // Wait for clear to finish
        while !$udp.rst_ep.read().$ep().bit() && $udp.csr()[$epnum].read().rxbytecnt().bits() != 0 {
        }
        // Clear
        $udp.rst_ep.modify(|_, w| w.$ep().clear_bit());
    }};
}

/// Creates a csr field set function $field_set(endpoint index)
/// or a csr field clear function $field_clear(endpoint index)
/// See ATSAM4S 40.7.10 warning on why this is necessary.
///
/// Uses $field_$eptype_dir_set(endpoint index) and $field_$eptype_dir_clear(endpoint index)
/// for the eptype field.
macro_rules! csr_wait {
    (
        $field:ident,
        set
    ) => {
        paste! {
            pub fn [<$field _set>](index: u8) {
                UDP::borrow_unchecked(|udp| {
                    // Set bit
                    udp.csr()[index as usize]
                        .modify(|_, w| csr_no_change(w).$field().set_bit());
                    // Wait for bit to set (See Warning 40.7.10 atsam4s datasheet)
                    while !udp.csr()[index as usize].read().$field().bit() {}
                });
            }
        }
    };
    (
        $field:ident,
        clear
    ) => {
        paste! {
            pub fn [<$field _clear>](index: u8) {
                UDP::borrow_unchecked(|udp| {
                    // Clear bit
                    udp.csr()[index as usize]
                        .modify(|_, w| csr_no_change(w).$field().clear_bit());
                    // Wait for bit to clear (See Warning 40.7.10 atsam4s datasheet)
                    while udp.csr()[index as usize].read().$field().bit() {}
                });
            }
        }
    };
    (
        $field:ident,
        $eptype_dir:ident,
        eptype
    ) => {
        paste! {
            pub fn [<$field _ $eptype_dir>](index: u8) {
                UDP::borrow_unchecked(|udp| {
                    // Set bit
                    udp.csr()[index as usize]
                        .modify(|_, w| csr_no_change(w).$field().$eptype_dir());
                    // Wait for bit to set (See Warning 40.7.10 atsam4s datasheet)
                    while !udp.csr()[index as usize].read().$field().[<is_ $eptype_dir>]() {}
                });
            }
        }
    };
}

/// Sets all fields (except fields that need to be set to 1 for no action) to 0
fn csr_clear(index: u8) {
    UDP::borrow_unchecked(|udp| {
        // Set bit
        udp.csr()[index as usize].modify(|_, w| {
            csr_no_change(w)
                .dir()
                .clear_bit()
                .epeds()
                .clear_bit()
                .forcestall()
                .clear_bit()
                .txpktrdy()
                .clear_bit()
                .eptype()
                .ctrl()
        });
        // Wait for bit to set (See Warning 40.7.10 atsam4s datasheet)
        loop {
            let reg = udp.csr()[index as usize].read();
            if !reg.dir().bit()
                && !reg.epeds().bit()
                && !reg.forcestall().bit()
                && !reg.txpktrdy().bit()
                && reg.eptype().is_ctrl()
            {
                break;
            }
        }
    });
}

/// The UDP CSR register is a bit strange in that for a modification to have no unexpected
/// side-effects you must set a number of bits.
/// - rx_data_bk0
/// - rx_data_bk1
/// - stallsent
/// - rxsetup
/// - txcomp
/// It's not sufficient to use a reset value as other bits (such as epeds), must not change
/// from the set value.
/// This is a convenience function to handle setting of these bits.
fn csr_no_change(w: &mut csr::W) -> &mut csr::W {
    w.rx_data_bk0()
        .set_bit()
        .rx_data_bk1()
        .set_bit()
        .stallsent()
        .set_bit()
        .rxsetup()
        .set_bit()
        .txcomp()
        .set_bit()
}

// Generate CSR set functions
csr_wait!(dir, set);
csr_wait!(epeds, set);
csr_wait!(forcestall, set);
csr_wait!(txpktrdy, set);

// Generate CSR clear functions
csr_wait!(dir, clear);
csr_wait!(epeds, clear);
csr_wait!(forcestall, clear);
csr_wait!(rx_data_bk0, clear);
csr_wait!(rx_data_bk1, clear);
csr_wait!(rxsetup, clear);
csr_wait!(stallsent, clear);
csr_wait!(txcomp, clear);
csr_wait!(txpktrdy, clear);

// Generate CSR eptype functions
csr_wait!(eptype, bulk_in, eptype);
csr_wait!(eptype, bulk_out, eptype);
csr_wait!(eptype, ctrl, eptype);
csr_wait!(eptype, int_in, eptype);
csr_wait!(eptype, int_out, eptype);
csr_wait!(eptype, iso_in, eptype);
csr_wait!(eptype, iso_out, eptype);

impl Endpoint {
    pub fn new(index: u8) -> Self {
        // TODO Figure out how to given ownership to specific CSR and FDR registers
        //      These are effectively owned by this struct, but I'm not sure how to do
        //      this with how svd2rust generated

        Self {
            index,
            interval: 1,
            max_packet_size: 8,
            ep_type: EndpointType::Interrupt,
            ep_dir: UsbDirection::Out,
            next_bank: NextBank::Bank0,
            allocated: false,
            txbanks_free: 0,
            stalled: false,
            next_out_zlp: false,
        }
    }

    /// Allocates the endpoint
    /// Since atsam4 uses registers for the buffer and configuration no memory
    /// is allocated. However there is a finite number of endpoints so we still
    /// need to do allocation and configuration.
    pub fn alloc(
        &mut self,
        ep_type: EndpointType,
        ep_dir: UsbDirection,
        max_packet_size: u16,
        interval: u8,
    ) -> usb_device::Result<EndpointAddress> {
        let address = EndpointAddress::from_parts(self.index as usize, ep_dir);

        // ep0 must be Control
        if ep_type != EndpointType::Control && self.index == 0 {
            return Err(usb_device::UsbError::InvalidEndpoint);
        }

        // Already allocated
        if self.allocated {
            // Ignore allocation check for Control endpoints
            if ep_type == EndpointType::Control {
                defmt::trace!(
                    "{} Endpoint{}::alloc() -> {:?}",
                    frm_num(),
                    self.index,
                    UdpEndpointAddress {
                        inner: Some(address)
                    },
                );
                return Ok(address);
            }
            return Err(usb_device::UsbError::InvalidEndpoint);
        }

        defmt::trace!(
            "{} Endpoint{}::alloc({:?}, {:?}, {}, {})",
            frm_num(),
            self.index,
            UdpEndpointType { inner: ep_type },
            UdpUsbDirection { inner: ep_dir },
            max_packet_size,
            interval
        );
        self.reset();

        // Check if max_packet_size will fit on this endpoint
        self.max_packet_size = max_packet_size;
        if max_packet_size > self.max_packet_size() {
            return Err(usb_device::UsbError::EndpointMemoryOverflow);
        }

        // Check if endpoint type can be allocated and set register
        match ep_type {
            EndpointType::Bulk => {}
            EndpointType::Control => {
                // Control endpoints are only valid on ep0 and ep3 (non-dual bank)
                if self.dual_bank() {
                    return Err(usb_device::UsbError::Unsupported);
                }
            }
            EndpointType::Interrupt => {}
            EndpointType::Isochronous => {
                // Must have dual banks for isochronous support
                if !self.dual_bank() {
                    return Err(usb_device::UsbError::Unsupported);
                }
            }
        }
        self.ep_type = ep_type;
        self.ep_dir = ep_dir;

        // Set free tx banks
        self.txbanks_free = if self.dual_bank() { 2 } else { 1 };

        self.allocated = true;
        self.interval = interval;
        defmt::trace!(
            "{} Endpoint{}::alloc() -> {:?}",
            frm_num(),
            self.index,
            UdpEndpointAddress {
                inner: Some(address)
            },
        );
        Ok(address)
    }

    /// Gets the endpoint address including direction bit.
    pub fn address(&self) -> EndpointAddress {
        EndpointAddress::from_parts(self.index as usize, self.ep_dir)
    }

    /// Gets the maximum packet size for the endpoint.
    pub fn max_packet_size(&self) -> u16 {
        let hardware = match self.index {
            0..=3 | 6 | 7 => 64,
            4 | 5 => 512, // Only really useful for isochronous
            _ => 0,       // Invalid
        };
        core::cmp::min(hardware, self.max_packet_size)
    }

    /// Check if endpoint is dual-buffered
    fn dual_bank(&self) -> bool {
        !matches!(self.index, 0 | 3)
    }

    /// Check endpoint interrupt
    fn interrupt(&self) -> bool {
        let isr = UDP::borrow_unchecked(|udp| udp.isr.read());
        match self.index {
            0 => isr.ep0int().bit(),
            1 => isr.ep1int().bit(),
            2 => isr.ep2int().bit(),
            3 => isr.ep3int().bit(),
            4 => isr.ep4int().bit(),
            5 => isr.ep5int().bit(),
            6 => isr.ep6int().bit(),
            7 => isr.ep7int().bit(),
            _ => false, // Invalid
        }
    }

    /// Check if endpoint is enabled
    fn interrupt_enabled(&self) -> bool {
        let imr = UDP::borrow_unchecked(|udp| udp.imr.read());
        match self.index {
            0 => imr.ep0int().bit(),
            1 => imr.ep1int().bit(),
            2 => imr.ep2int().bit(),
            3 => imr.ep3int().bit(),
            4 => imr.ep4int().bit(),
            5 => imr.ep5int().bit(),
            6 => imr.ep6int().bit(),
            7 => imr.ep7int().bit(),
            _ => false, // Invalid
        }
    }

    /// Set interrupt (enable/disable)
    fn interrupt_set(&self, enable: bool) {
        // Enable interrupt for endpoint
        unsafe {
            UDP::borrow_unchecked(|udp| {
                if enable {
                    match self.index {
                        0 => udp.ier.write_with_zero(|w| w.ep0int().set_bit()),
                        1 => udp.ier.write_with_zero(|w| w.ep1int().set_bit()),
                        2 => udp.ier.write_with_zero(|w| w.ep2int().set_bit()),
                        3 => udp.ier.write_with_zero(|w| w.ep3int().set_bit()),
                        4 => udp.ier.write_with_zero(|w| w.ep4int().set_bit()),
                        5 => udp.ier.write_with_zero(|w| w.ep5int().set_bit()),
                        6 => udp.ier.write_with_zero(|w| w.ep6int().set_bit()),
                        7 => udp.ier.write_with_zero(|w| w.ep7int().set_bit()),
                        _ => {} // Invalid
                    }
                } else {
                    match self.index {
                        0 => udp.idr.write_with_zero(|w| w.ep0int().set_bit()),
                        1 => udp.idr.write_with_zero(|w| w.ep1int().set_bit()),
                        2 => udp.idr.write_with_zero(|w| w.ep2int().set_bit()),
                        3 => udp.idr.write_with_zero(|w| w.ep3int().set_bit()),
                        4 => udp.idr.write_with_zero(|w| w.ep4int().set_bit()),
                        5 => udp.idr.write_with_zero(|w| w.ep5int().set_bit()),
                        6 => udp.idr.write_with_zero(|w| w.ep6int().set_bit()),
                        7 => udp.idr.write_with_zero(|w| w.ep7int().set_bit()),
                        _ => {} // Invalid
                    }
                }
            });
        }
    }

    /// Gets the poll interval for interrupt endpoints.
    pub fn interval(&self) -> u8 {
        self.interval
    }

    /// Sets the STALL condition for the endpoint.
    pub fn stall(&mut self) {
        if !self.stalled {
            defmt::debug!("{} Endpoint{}::stall()", frm_num(), self.index);
            forcestall_set(self.index);
            self.stalled = true;
        }
    }

    /// Clears the STALL condition of the endpoint.
    pub fn unstall(&mut self) {
        defmt::trace!("{} Endpoint{}::unstall()", frm_num(), self.index);
        forcestall_clear(self.index);
        self.stalled = false;

        // Recharge free banks after a stall clear
        self.txbanks_free = if self.dual_bank() { 2 } else { 1 };
    }

    /// Check if STALL has been set
    pub fn is_stalled(&self) -> bool {
        self.stalled
    }

    /// Enable endpoint
    pub fn enable(&self) {
        // Only enable if the endpoint has been allocated
        if self.allocated {
            epeds_set(self.index);
        }
    }

    /// Disable endpoint
    pub fn disable(&self) {
        epeds_clear(self.index);
    }

    /// Clear fifo (two step, set then clear)
    pub fn clear_fifo(&self) {
        UDP::borrow_unchecked(|udp| {
            match self.index {
                0 => clear_ep!(udp, ep0, 0),
                1 => clear_ep!(udp, ep1, 1),
                2 => clear_ep!(udp, ep2, 2),
                3 => clear_ep!(udp, ep3, 3),
                4 => clear_ep!(udp, ep4, 4),
                5 => clear_ep!(udp, ep5, 5),
                6 => clear_ep!(udp, ep6, 6),
                7 => clear_ep!(udp, ep7, 7),
                _ => {} // Invalid
            }
        });
    }

    /// Reset endpoint to allocated settings
    pub fn reset(&mut self) {
        if !self.allocated {
            return;
        }

        // Clear CSR
        csr_clear(self.index);

        // Toggle TXPKTRDY to force a FIFO flush
        txpktrdy_set(self.index);
        txpktrdy_clear(self.index);
        if self.dual_bank() {
            txpktrdy_set(self.index);
            txpktrdy_clear(self.index);
        }

        // Reset endpoint FIFO
        self.clear_fifo();

        // Clear free tx banks
        self.txbanks_free = if self.dual_bank() { 2 } else { 1 };

        // Setup CSR
        match self.ep_type {
            EndpointType::Bulk => match self.ep_dir {
                UsbDirection::In => {
                    eptype_bulk_in(self.index);
                }
                UsbDirection::Out => {
                    eptype_bulk_out(self.index);
                }
            },
            EndpointType::Control => {
                // Control Endpoint must start out configured in the OUT direction
                eptype_ctrl(self.index);
                dir_clear(self.index);
            }
            EndpointType::Interrupt => match self.ep_dir {
                UsbDirection::In => {
                    eptype_int_in(self.index);
                }
                UsbDirection::Out => {
                    eptype_int_out(self.index);
                }
            },
            EndpointType::Isochronous => match self.ep_dir {
                UsbDirection::In => {
                    eptype_iso_in(self.index);
                }
                UsbDirection::Out => {
                    eptype_iso_out(self.index);
                }
            },
        }

        // Enable endpoint
        self.enable();

        defmt::trace!(
            "{} Endpoint{}::reset() CSR:{:X}",
            frm_num(),
            self.index,
            UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read()).bits()
        );

        // Enable interrupt
        self.interrupt_set(true);
    }

    /// Poll endpoint
    pub fn poll(&mut self) -> PollResult {
        if !self.allocated {
            return PollResult::None;
        }

        // Check if interupt is enabled (except for Ctrl endpoints)
        if !self.interrupt_enabled() && self.ep_type != EndpointType::Control {
            return PollResult::None;
        }

        // Check endpoint interrupt
        if self.interrupt() {
            let csr = UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read());

            // STALLed
            if csr.stallsent().bit() {
                // Ack STALL
                stallsent_clear(self.index);
                self.stalled = false;
                defmt::debug!(
                    "{} Endpoint{}::Poll() -> Ack STALL CSR:{:X}",
                    frm_num(),
                    self.index,
                    UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read()).bits(),
                );
                return PollResult::None;
            }

            // Determine endpoint type
            match self.ep_type {
                EndpointType::Control => {
                    // SETUP packet received
                    if csr.rxsetup().bit() {
                        defmt::debug!(
                            "{} Endpoint{}::Poll(Ctrl) -> SETUP CSR:{:X}",
                            frm_num(),
                            self.index,
                            UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read())
                                .bits()
                        );
                        return PollResult::Data {
                            ep_out: 0,
                            ep_in_complete: 0,
                            ep_setup: 1 << self.index,
                        };
                    }
                    // IN packet sent
                    if csr.txcomp().bit() {
                        defmt::debug!(
                            "{} Endpoint{}::Poll(Ctrl) -> IN CSR:{:X}",
                            frm_num(),
                            self.index,
                            UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read())
                                .bits()
                        );
                        // Ack TXCOMP
                        txcomp_clear(self.index);
                        self.txbanks_free += 1;
                        return PollResult::Data {
                            ep_out: 0,
                            ep_in_complete: 1 << self.index,
                            ep_setup: 0,
                        };
                    }
                    // OUT packet received
                    if csr.rx_data_bk0().bit() {
                        // If this is a ZLP (from a Control Write transaction), we can ACK it right
                        // away.
                        if csr.rxbytecnt().bits() == 0 {
                            self.next_out_zlp = true;
                            rx_data_bk0_clear(self.index);
                            defmt::debug!(
                                "{} Endpoint{}::Poll(Ctrl) -> Status OUT CSR:{:X}",
                                frm_num(),
                                self.index,
                                UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read())
                                    .bits()
                            );
                        } else {
                            defmt::debug!(
                                "{} Endpoint{}::Poll(Ctrl) -> OUT CSR:{:X}",
                                frm_num(),
                                self.index,
                                csr.bits(),
                            );
                        }
                        return PollResult::Data {
                            ep_out: 1 << self.index,
                            ep_in_complete: 0,
                            ep_setup: 0,
                        };
                    }
                }
                EndpointType::Bulk | EndpointType::Interrupt | EndpointType::Isochronous => {
                    // RXOUT: Full packet received
                    let ep_out = if csr.rx_data_bk0().bit() || csr.rx_data_bk1().bit() {
                        defmt::trace!(
                            "{} Endpoint{}::Poll({:?}) -> OUT CSR:{:X}",
                            frm_num(),
                            self.index,
                            UdpEndpointType {
                                inner: self.ep_type
                            },
                            csr.bits()
                        );
                        1 << self.index
                    } else {
                        0
                    };
                    // TXIN: Packet sent
                    let ep_in_complete = if csr.txcomp().bit() {
                        // Ack TXCOMP
                        txcomp_clear(self.index);
                        self.txbanks_free += 1;
                        defmt::trace!(
                            "{} Endpoint{}::Poll({:?}) -> IN CSR:{:X}",
                            frm_num(),
                            self.index,
                            UdpEndpointType {
                                inner: self.ep_type
                            },
                            UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read())
                                .bits()
                        );
                        1 << self.index
                    } else {
                        0
                    };

                    // Return if we found any data status flags
                    if (ep_in_complete | ep_out) > 0 {
                        return PollResult::Data {
                            ep_out,
                            ep_in_complete,
                            ep_setup: 0,
                        };
                    }
                }
            }
        }

        PollResult::None
    }

    /// Writes a single packet of data to the specified endpoint and returns number of bytes
    /// actually written. The buffer must not be longer than the `max_packet_size` specified when
    /// allocating the endpoint.
    ///
    /// # Errors
    ///
    /// Note: USB bus implementation errors are directly passed through, so be prepared to handle
    /// other errors as well.
    ///
    /// * [`WouldBlock`](usb_device::UsbError::WouldBlock) - The transmission buffer of the USB
    ///   peripheral is full and the packet cannot be sent now. A peripheral may or may not support
    ///   concurrent transmission of packets.
    /// * [`BufferOverflow`](usb_device::UsbError::BufferOverflow) - The data is longer than the
    ///   `max_packet_size` specified when allocating the endpoint. This is generally an error in
    ///   the class implementation.
    pub fn write(&mut self, data: &[u8]) -> usb_device::Result<usize> {
        // -- Data IN Transaction --
        // * Check for FIFO ready by polling TXPKTRDY in CSR
        // * Write packet data to FDR
        // * Notify FIFO ready to send by setting TXPKTRDY
        // * FIFO has been released when TXCOMP is set (clear TXCOMP)
        // * Write next packet to FDR
        // * Notify FIFO ready to send by setting TXPKTRDY
        // * After the last packet is sent, clear TXCOMP
        // -- Data IN Transaction (/w Ping-pong) --
        // Isochronous must use Ping-pong for Data IN
        // * Check for FIFO ready by polling TXPKTRDY in CSR
        // * Write packet data to FDR (Bank 0)
        // * Notify FIFO ready to send by setting TXPKTRDY
        // * Immediately write next packet to FDR (Bank 1)
        // * Bank 0 FIFO has been released when TXCOMP is set (clear TXCOMP)
        // * Write next packet to FDR (Bank 0)
        // * Notify FIFO ready to send by setting TXPKTRDY
        // * After the last packet is sent, clear TXCOMP

        // Make sure endpoint has been allocated
        if !self.allocated {
            return Err(usb_device::UsbError::InvalidEndpoint);
        }

        // Make sure FIFO is ready
        // This counter takes into account Ctrl vs Non-Ctrl endpoints
        let csr = UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read());
        if self.txbanks_free == 0 || (self.txbanks_free == 1 && csr.txpktrdy().bit()) {
            return Err(usb_device::UsbError::WouldBlock);
        }

        // Make sure we don't overflow the endpoint fifo
        // Each EP has a different size and is not configurable
        if data.len() > self.max_packet_size() as usize {
            return Err(usb_device::UsbError::EndpointMemoryOverflow);
        }

        // Check to see if data has been received on this endpoint (Ctrl-only)
        // and abort.
        if csr.rx_data_bk0().bit() {
            // Send ZLP
            txcomp_clear(self.index);
            return Err(usb_device::UsbError::InvalidState);
        }

        // While copying data to FIFO do not interrupt
        cortex_m::interrupt::free(|_| {
            // Write data to fifo
            UDP::borrow_unchecked(|udp| {
                for byte in data {
                    unsafe {
                        udp.fdr[self.index as usize].write_with_zero(|w| w.fifo_data().bits(*byte))
                    }
                }
            });
            self.txbanks_free -= 1;

            // Set TXPKTRDY
            txpktrdy_set(self.index);
        });

        defmt::debug!(
            "{} Endpoint{}::write() -> {} CSR:{:X} Banks:{}",
            frm_num(),
            self.index,
            data.len(),
            UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read()).bits(),
            self.txbanks_free,
        );
        Ok(data.len())
    }

    /// Reads a single packet of data from the specified endpoint and returns the actual length of
    /// the packet. The buffer should be large enough to fit at least as many bytes as the
    /// `max_packet_size` specified when allocating the endpoint.
    ///
    /// # Errors
    ///
    /// Note: USB bus implementation errors are directly passed through, so be prepared to handle
    /// other errors as well.
    ///
    /// * [`WouldBlock`](usb_device::UsbError::WouldBlock) - There is no packet to be read. Note that
    ///   this is different from a received zero-length packet, which is valid and significant in
    ///   USB. A zero-length packet will return `Ok(0)`.
    /// * [`BufferOverflow`](usb_device::UsbError::BufferOverflow) - The received packet is too long to
    ///   fit in `data`. This is generally an error in the class implementation.
    pub fn read(&mut self, data: &mut [u8]) -> usb_device::Result<usize> {
        let csr = UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read());
        defmt::debug!(
            "{} Endpoint{}::read() CSR:{:X}",
            frm_num(),
            self.index,
            csr.bits()
        );

        // Make sure endpoint has been allocated
        if !self.allocated {
            return Err(usb_device::UsbError::InvalidEndpoint);
        }

        // Determine if we've been configured as a control endpoint
        if csr.eptype().variant() == Some(EPTYPE_A::CTRL) {
            // -- Setup Transaction --
            // * Hardware automatically acknowledges
            // * RXSETUP is set in CSR
            // * Interrupt until RXSETUP is cleared
            // * If SETUP IN, set the DIR bit
            // -- Data OUT Transaction --
            // * Until FIFO is ready, hardware sends NAKs automatically
            // * After data is written to FIFO, ACK automatically sent
            // * RX_DATA_BK0 is set in CSR
            // * Interrupt until RX_DATA_BK0 is cleared
            // * RXBYTECNT has the number of bytes in the FIFO
            // * Read FDR for FIFO data
            // * Clear RX_DATA_BK0 to indicate finished

            // Check if we have a queued zlp
            if self.next_out_zlp {
                self.next_out_zlp = false;
                return Ok(0);
            }

            // Check for RXSETUP
            if csr.rxsetup().bit() {
                // Check incoming data length, make sure our buffer is big enough
                let rxbytes = csr.rxbytecnt().bits() as usize;
                if rxbytes > data.len() {
                    // Clear RXSETUP, to continue after the overflow
                    rxsetup_clear(self.index);
                    return Err(usb_device::UsbError::BufferOverflow);
                }

                // All setup transactions have 8 bytes, invalid otherwise
                if rxbytes != 8 {
                    // Clear RXSETUP, to continue after the error
                    rxsetup_clear(self.index);
                    return Err(usb_device::UsbError::InvalidState);
                }

                // Copy fifo into buffer
                for byte in data.iter_mut().take(rxbytes) {
                    *byte = UDP::borrow_unchecked(|udp| {
                        udp.fdr[self.index as usize].read().fifo_data().bits()
                    });
                }

                // We can determine the direction by looking at the first byte
                let dir: UsbDirection = data[0].into();
                if dir == UsbDirection::In {
                    dir_set(self.index);
                } else {
                    dir_clear(self.index);
                }

                // Clear RXSETUP
                rxsetup_clear(self.index);
                defmt::debug!(
                    "{} Endpoint{}::read({}, {:02X}) SETUP {:?} CSR:{:X}",
                    frm_num(),
                    self.index,
                    rxbytes,
                    &data[0..rxbytes],
                    UdpUsbDirection { inner: dir },
                    UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read()).bits()
                );
                return Ok(rxbytes);
            }

            // Check for OUT packet in Bank0
            if csr.rx_data_bk0().bit() {
                // Check incoming data length, make sure our buffer is big enough
                let rxbytes = csr.rxbytecnt().bits() as usize;
                if rxbytes > data.len() {
                    // Clear RX_DATA_BK0, to continue after the overflow
                    rx_data_bk0_clear(self.index);
                    return Err(usb_device::UsbError::BufferOverflow);
                }

                // Copy fifo into buffer
                for byte in data.iter_mut().take(rxbytes) {
                    *byte = UDP::borrow_unchecked(|udp| {
                        udp.fdr[self.index as usize].read().fifo_data().bits()
                    });
                }

                // Clear RX_DATA_BK0
                rx_data_bk0_clear(self.index);
                defmt::debug!(
                    "{} Endpoint{}::read({}, {:02X}) OUT CSR:{:X}",
                    frm_num(),
                    self.index,
                    rxbytes,
                    &data[0..rxbytes],
                    UDP::borrow_unchecked(|udp| udp.csr()[self.index as usize].read()).bits()
                );
                return Ok(rxbytes);
            }

            // No data
            return Err(usb_device::UsbError::WouldBlock);
        }

        // Make sure this is an Out endpoint
        match csr.eptype().variant() {
            Some(EPTYPE_A::BULK_OUT) | Some(EPTYPE_A::INT_OUT) | Some(EPTYPE_A::ISO_OUT) => {}
            _ => {
                return Err(usb_device::UsbError::InvalidEndpoint);
            }
        }

        // -- Data OUT Transaction (/w Ping-pong) --
        // Isochronous must use Ping-pong for Data OUT
        // NOTE: Must keep track of which bank should be next as there's no way
        //       to determine which bank should be next if the interrupt was slow.
        // * Until FIFO is ready, hardware sends NAKs automatically
        // * After data is written to FIFO, ACK automatically sent
        //   - Host can immediately start sending data to Bank 1 after ACK
        // * RX_DATA_BK0 is set in CSR
        // * Interrupt until RX_DATA_BK0 is cleared
        // * RXBYTECNT has the number of bytes in the FIFO
        // * Read FDR for FIFO data
        // * Clear RX_DATA_BK0 to indicate finished
        //   - Host can begin sending data to Bank 0

        // Determine which bank to read
        let bank = if csr.rx_data_bk0().bit() && csr.rx_data_bk1().bit() {
            // Both banks are ready, use prior state to decide
            self.next_bank
        } else if csr.rx_data_bk0().bit() {
            NextBank::Bank0
        // EP0 and EP3 are not dual-buffered
        } else if self.dual_bank() && csr.rx_data_bk1().bit() {
            NextBank::Bank1
        } else {
            // No data
            return Err(usb_device::UsbError::WouldBlock);
        };

        // Check incoming data length, make sure our buffer is big enough
        let rxbytes = csr.rxbytecnt().bits() as usize;
        if rxbytes > data.len() {
            // Clear bank fifo, to continue after the overflow
            match bank {
                NextBank::Bank0 => {
                    rx_data_bk0_clear(self.index);
                    self.next_bank = NextBank::Bank1;
                }
                NextBank::Bank1 => {
                    rx_data_bk1_clear(self.index);
                    self.next_bank = NextBank::Bank0;
                }
            }
            return Err(usb_device::UsbError::BufferOverflow);
        }

        // Copy fifo into buffer
        UDP::borrow_unchecked(|udp| {
            for byte in data.iter_mut().take(rxbytes) {
                *byte = udp.fdr[self.index as usize].read().fifo_data().bits();
            }
        });

        // Clear bank fifo to indicate finished
        match bank {
            NextBank::Bank0 => {
                rx_data_bk0_clear(self.index);
                self.next_bank = NextBank::Bank1;
            }
            NextBank::Bank1 => {
                rx_data_bk1_clear(self.index);
                self.next_bank = NextBank::Bank0;
            }
        }
        Ok(rxbytes)
    }
}
