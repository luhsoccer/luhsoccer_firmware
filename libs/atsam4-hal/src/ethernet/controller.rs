use super::{
    builder::Builder,
    descriptor_table::DescriptorTableT,
    phy::{LinkType, Phy, Register},
    EthernetAddress, Receiver, RxDescriptor, Transmitter, TxDescriptor,
};
use crate::{
    clock::{get_master_clock_frequency, Enabled, GmacClock},
    gpio::*,
    pac::GMAC,
};
use core::marker::PhantomData;
use fugit::RateExtU32;
use paste::paste;

macro_rules! define_ethernet_address_function {
    (
        $address_number:expr
    ) => {
        paste! {
            fn [<set_ethernet_address $address_number>](&mut self, ethernet_address: &EthernetAddress) {
                let bytes = ethernet_address.as_bytes();
                self.gmac.[<sab $address_number>].write(|w| unsafe {
                    w.bits(
                        (bytes[0] as u32) |
                        (bytes[1] as u32) << 8 |
                        (bytes[2] as u32) << 16 |
                        (bytes[3] as u32) << 24
                    )
                });

                // NOTE: Writing the top bits (e.g. satX) enables the address in the hardware.
                self.gmac.[<sat $address_number>].write(|w| unsafe {
                    w.bits(
                        (bytes[4] as u32) |
                        (bytes[5] as u32) << 8
                    )
                });
            }
        }
    };
}

pub struct Controller {
    pub(super) gmac: GMAC,
    clock: PhantomData<GmacClock<Enabled>>,
    phy_address: u8,
    pub(super) rx: Receiver,
    pub(super) tx: Transmitter,
}

impl Controller {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        gmac: GMAC,
        _: GmacClock<Enabled>,
        _gtxck: Pd0<PfA>,
        _gtxen: Pd1<PfA>,
        _gtx0: Pd2<PfA>,
        _gtx1: Pd3<PfA>,
        _gcrsdv: Pd4<PfA>,
        _grx0: Pd5<PfA>,
        _grx1: Pd6<PfA>,
        _grxer: Pd7<PfA>,
        _gmdc: Pd8<PfA>,
        _gmdio: Pd9<PfA>,
        _gcrs: Pd10<PfA>,
        _grx2: Pd11<PfA>,
        _grx3: Pd12<PfA>,
        _gcol: Pd13<PfA>,
        _grxck: Pd14<PfA>,
        _gtx2: Pd15<PfA>,
        _gtx3: Pd16<PfA>,
        _gtxer: Pd17<PfA>,
        rx: &'static mut (dyn DescriptorTableT<RxDescriptor> + Send),
        tx: &'static mut (dyn DescriptorTableT<TxDescriptor> + Send),
        builder: Builder,
    ) -> Self {
        rx.initialize();
        tx.initialize();

        let rx_base_address = rx.base_address();
        let tx_base_address = tx.base_address();
        let mut e = Controller {
            gmac,
            clock: PhantomData,
            phy_address: builder.phy_address(),
            rx: Receiver::new(rx),
            tx: Transmitter::new(tx),
        };

        // Reset the GMAC to its reset state (with transmit and receive disabled)
        e.reset();

        // Set the GMAC network configuration register value.
        e.gmac.ncfgr.modify(|_, w| {
            w
                // Copy All Frames (Promiscuous Mode) -- TODO: Only accept frame destined for our MAC. For this we need to also receive frames from multicast macs
                .caf()
                .set_bit()
                .rxcoen()
                .set_bit()
                // Allow 1536 byte frames
                .maxfs()
                .set_bit()
                // Set pause-enable - transmission will pause if a non-zero 802.3 classic pause frame is received and PFC has not been negotiated.
                .pen()
                .set_bit();

            // Set up the MDC (Management Data Clock) for the PHY based on the master clock frequency
            let mck = get_master_clock_frequency();
            if mck > 240u32.MHz::<1, 1>() {
                panic!("Invalid master clock frequency")
            } else if mck > 160u32.MHz::<1, 1>() {
                w.clk().mck_96();
            } else if mck > 120u32.MHz::<1, 1>() {
                w.clk().mck_64();
            } else if mck > 80u32.MHz::<1, 1>() {
                w.clk().mck_48();
            } else if mck > 40u32.MHz::<1, 1>() {
                w.clk().mck_32();
            } else if mck > 20u32.MHz::<1, 1>() {
                w.clk().mck_16();
            } else {
                w.clk().mck_8();
            }
            w
        });

        e.reset_phy();

        // Initialize the PHY and set the GMAC's speed and duplex based on returned link type.
        let link = e.enable_phy_auto_negotiation();

        match link {
            LinkType::HalfDuplex10 => e
                .gmac
                .ncfgr
                .modify(|_, w| w.spd().clear_bit().fd().clear_bit()),
            LinkType::FullDuplex10 => e
                .gmac
                .ncfgr
                .modify(|_, w| w.spd().clear_bit().fd().set_bit()),
            LinkType::HalfDuplex100 => e
                .gmac
                .ncfgr
                .modify(|_, w| w.spd().set_bit().fd().clear_bit()),
            LinkType::FullDuplex100 => e.gmac.ncfgr.modify(|_, w| w.spd().set_bit().fd().set_bit()),
        }

        // Ensure MII mode is set (NOTE: it's clear by default)
        e.gmac.ur.modify(|_, w| w.mii().set_bit());

        // Set the MAC addresses into the hardware.
        e.set_ethernet_address1(&builder.ethernet_address());
        for index in 0..builder.alternate_ethernet_address_count() {
            let alternate_address = builder.alternate_ethernet_address(index);
            match index {
                0 => e.set_ethernet_address2(&alternate_address),
                1 => e.set_ethernet_address3(&alternate_address),
                2 => e.set_ethernet_address4(&alternate_address),
                _ => panic!("unexpected alternate mac address offset in 3 element array"),
            }
        }

        // Initialize the receive descriptor table
        e.gmac.rbqb.write(|w| unsafe { w.bits(rx_base_address) });
        e.gmac.tbqb.write(|w| unsafe { w.bits(tx_base_address) });

        // Initialize the DMA configuration register
        e.gmac.dcfgr.modify(|_, w| unsafe {
            w.fbldo()
                .incr4() // set up incr4 (default) transfers
                .esma()
                .clear_bit() // do not swap endianess for management transfer
                .espa()
                .clear_bit() // do not swap endianess for packet transfer
                .drbs()
                .bits(0x18) // Set transfer buffer sizes to 1536
        });

        // Enable receive and transmit circuits
        e.enable_receive();
        e.enable_transmit();

        e
    }

    pub fn link_state(&self) -> Option<u32> {
        let phy_status = self.read_phy_bmsr();
        match phy_status.link_detected() {
            false => None,
            true => {
                if phy_status.is_100mbit() {
                    Some(100)
                } else {
                    Some(10)
                }
            }
        }
    }

    fn reset(&mut self) {
        self.gmac.ncr.reset();
        self.disable_all_interrupts();
        self.clear_statistics();

        // Clear all status bits in the receive status register by setting the four
        // status bits.
        self.gmac.rsr.write(
            |w| {
                w.bna()
                    .set_bit() // Buffer not available
                    .rec()
                    .set_bit() // Frame Received
                    .rxovr()
                    .set_bit() // Receive Overrun
                    .hno()
                    .set_bit()
            }, // HRESP not ok
        );

        // Clear all bits in the transmit status register
        self.gmac.tsr.write(
            |w| {
                w.ubr()
                    .set_bit() // Used bit read
                    .col()
                    .set_bit() // Collision occurred
                    .rle()
                    .set_bit() // Retry limit exceeded
                    .txgo()
                    .set_bit() // Transmit go
                    .tfc()
                    .set_bit() // Transmit frame corruption due to AHB error
                    .txcomp()
                    .set_bit() // Transmit complete
                    .und()
                    .set_bit() // Transmit underrun
                    .hresp()
                    .set_bit()
            }, // HRESP not ok
        );

        // Read the interrupt status register to ensure all interrupts are clear
        self.gmac.isr.read();

        // Reset the configuration register
        self.gmac.ncfgr.reset();

        // Disable both transmit and receive circuits
        self.disable_receive();
        self.disable_transmit();
    }

    fn disable_all_interrupts(&mut self) {
        unsafe {
            self.gmac.idr.write_with_zero(|w| {
                w.mfs()
                    .set_bit()
                    .rcomp()
                    .set_bit()
                    .rxubr()
                    .set_bit()
                    .txubr()
                    .set_bit()
                    .tur()
                    .set_bit()
                    .rlex()
                    .set_bit()
                    .tfc()
                    .set_bit()
                    .tcomp()
                    .set_bit()
                    .rovr()
                    .set_bit()
                    .hresp()
                    .set_bit()
                    .pfnz()
                    .set_bit()
                    .ptz()
                    .set_bit()
                    .pftr()
                    .set_bit()
                    .exint()
                    .set_bit()
                    .drqfr()
                    .set_bit()
                    .sfr()
                    .set_bit()
                    .drqft()
                    .set_bit()
                    .sft()
                    .set_bit()
                    .pdrqfr()
                    .set_bit()
                    .pdrsfr()
                    .set_bit()
                    .pdrqft()
                    .set_bit()
                    .pdrsft()
                    .set_bit()
                    .sri()
                    .set_bit()
                    .wol()
                    .set_bit()
            });
        }
    }

    fn enable_transmit(&self) {
        self.gmac.ncr.modify(|_, w| w.txen().set_bit())
    }

    fn disable_transmit(&self) {
        self.gmac.ncr.modify(|_, w| w.txen().clear_bit())
    }

    fn enable_receive(&self) {
        self.gmac.ncr.modify(|_, w| w.rxen().set_bit())
    }

    fn disable_receive(&self) {
        self.gmac.ncr.modify(|_, w| w.rxen().clear_bit())
    }

    // Hardware/MAC address manipulation
    define_ethernet_address_function!(1);
    define_ethernet_address_function!(2);
    define_ethernet_address_function!(3);
    define_ethernet_address_function!(4);

    // PHY
    fn wait_for_phy_idle(&self) {
        while !self.gmac.nsr.read().idle().bit() {}
    }

    // Statistics
    fn clear_statistics(&mut self) {
        self.gmac.ncr.modify(|_, w| w.clrstat().set_bit())
    }

    fn _increment_statistics(&mut self) {
        self.gmac.ncr.modify(|_, w| w.incstat().set_bit())
    }
}

impl Phy for Controller {
    fn read_phy_register(&self, register: Register) -> u16 {
        self.wait_for_phy_idle();
        self.gmac.man.write(|w| unsafe {
            w.
            wtn().bits(0b10).                   // must always be binary 10 (0x02)
            rega().bits(register as u8).        // phy register to read
            phya().bits(self.phy_address).      // phy address
            op().bits(0b10).                    // write = 0b01, read = 0b10
            cltto().set_bit().
            wzo().clear_bit() // must be set to zero
        });

        // Wait for the shift operation to complete and the register value to be present
        self.wait_for_phy_idle();

        // Read the data portion of the register
        self.gmac.man.read().data().bits()
    }

    fn write_phy_register(&mut self, register: Register, new_value: u16) {
        self.wait_for_phy_idle();
        self.gmac.man.write(|w| unsafe {
            w.
            data().bits(new_value).
            wtn().bits(0b10).                   // must always be binary 10 (0x02)
            rega().bits(register as u8).        // phy register to read/write
            phya().bits(self.phy_address).      // phy address
            op().bits(0b01).                    // write = 0b01, read = 0b10
            cltto().set_bit().
            wzo().clear_bit() // must be set to zero
        });
        self.wait_for_phy_idle();
    }

    fn enable_phy_management_port(&self) {
        self.gmac.ncr.modify(|_, w| w.mpe().set_bit());
    }

    fn disable_phy_management_port(&self) {
        self.gmac.ncr.modify(|_, w| w.mpe().clear_bit());
    }
}
