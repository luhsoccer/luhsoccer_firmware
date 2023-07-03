//! SPI Implementation
use crate::clock::{get_master_clock_frequency, Enabled, SpiClock};
use crate::gpio::{Pa12, Pa13, Pa14, PfA};
use crate::pac::SPI;
use crate::pdc::*;
use core::marker::PhantomData;
use core::sync::atomic::{compiler_fence, Ordering};
use embedded_dma::{ReadBuffer, WriteBuffer};
use paste::paste;

pub use embedded_hal::spi;
pub use fugit::HertzU32 as Hertz;

/// u8 that can convert back and forth with u16
/// Needed for some of the register bit fields
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct SpiU8(u8);

impl From<u8> for SpiU8 {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

impl From<u16> for SpiU8 {
    // Yes this will lose bits; however in this mode only the first 8bits are used
    fn from(val: u16) -> Self {
        Self(val as _)
    }
}

impl From<SpiU16> for SpiU8 {
    fn from(val: SpiU16) -> Self {
        Self(val.0 as _)
    }
}

impl From<SpiU8> for u8 {
    fn from(val: SpiU8) -> Self {
        val.0 as _
    }
}

/// u16 that can convert back and forth with u8
/// Needed for some of the register bit fields
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct SpiU16(u16);

impl From<u8> for SpiU16 {
    fn from(val: u8) -> Self {
        Self(val as _)
    }
}

impl From<u16> for SpiU16 {
    fn from(val: u16) -> Self {
        Self(val)
    }
}

impl From<SpiU8> for SpiU16 {
    fn from(val: SpiU8) -> Self {
        Self(val.0 as _)
    }
}

impl From<SpiU16> for u16 {
    fn from(val: SpiU16) -> Self {
        val.0 as _
    }
}

/// SPI Error
#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Underrun occurred (slave mode only)
    Underrun,
    /// Mode fault occurred
    ModeFault,
    /// SPI Disabled
    SpiDisabled,
    /// Invalid Chip Select
    InvalidCs(u8),
    /// Fixed Mode Set
    FixedModeSet,
    /// Variable Mode Set
    VariableModeSet,
    /// PCS read unexpected (data, pcs)
    UnexpectedPcs(u16, u8),
}

/// Chip Select Active Settings
/// This enum controls:
///  CNSAAT -> Chip Select Not Active After Transfer
///  CSAAT -> Chip Select Active After Transfer
#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum ChipSelectActive {
    /// csaat = 1, csnaat = 0
    ActiveAfterTransfer,
    /// csaat = 0, csnaat = 0
    ActiveOnConsecutiveTransfers,
    /// csaat = 0, csnaat = 1
    InactiveAfterEachTransfer,
}

/// Transfer Width
/// NOTE: Transfer Widths larger than 8-bits require using 16-bit with send/read
#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum BitWidth {
    Width8Bit = 0,
    Width9Bit = 1,
    Width10Bit = 2,
    Width11Bit = 3,
    Width12Bit = 4,
    Width13Bit = 5,
    Width14Bit = 6,
    Width15Bit = 7,
    Width16Bit = 8,
}

/// Peripheral Select Mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum PeripheralSelectMode {
    /// Fixed Peripheral Select Mode (ps = 0, pcsdec = 0)
    Fixed,
    /// Variable Peripheral Select Mode (ps = 1, pcsdec = 0)
    Variable,
    /// Chip Select Decode (Variable) (ps = 1, pcsdec = 1)
    ChipSelectDecode,
}

/// SPI Chip Select Settings
///
/// CPOL -> MODE
/// NCPHA -> MODE
/// CSNAAT -> CS not active after transfer (ignored if CSAAT = 1) => csa
/// CSAAT -> CS active after transfer => csa
/// BITS -> 8bit through 16bits
/// SCBR -> Serial clock rate (SCBR = f_periph / SPCK bit rate) (0 forbidden)
/// DLYBS -> Delay before SPCK (DLYBS x f_periph)
/// DLYBCT -> Delay between consecutive transfers (DLYBCT x f_periph / 32)
#[derive(Clone, PartialEq, Eq)]
pub struct ChipSelectSettings {
    mode: spi::Mode,
    csa: ChipSelectActive,
    scbr: u8,
    dlybs: u8,
    dlybct: u8,
    bits: BitWidth,
}

impl ChipSelectSettings {
    /// mode:   SPI Mode
    /// csa:    Chip Select behaviour after transfer
    /// bits:   SPI bit width
    /// baud:   SPI speed in Hertz
    /// dlybs:  Cycles to delay from CS to first valid SPCK
    ///         0 is half the SPCK clock period
    ///         Otherwise dlybs = Delay Before SPCK x f_periph
    /// dlybct: Cycles to delay between consecutive transfers
    ///         0 is no delay
    ///         Otherwise dlybct = Delay between consecutive transfers x f_periph / 32
    pub fn new(
        mode: spi::Mode,
        csa: ChipSelectActive,
        bits: BitWidth,
        baud: Hertz,
        dlybs: u8,
        dlybct: u8,
    ) -> ChipSelectSettings {
        let pclk = get_master_clock_frequency();

        // Calculate baud divider
        // (f_periph + baud - 1) / baud
        let scbr = ((pclk.raw() + baud.raw() - 1) / baud.raw()) as u8;
        if scbr < 1 {
            panic!("scbr must be greater than 0: {}", scbr);
        }

        ChipSelectSettings {
            mode,
            csa,
            scbr,
            dlybs,
            dlybct,
            bits,
        }
    }
}

/// SPI Master
///
/// Example on how to individually read/write to SPI CS channels
/// ```
/// use atsam4_hal::clock::{ClockController, MainClock, SlowClock};
/// use atsam4_hal::pac::Peripherals;
///
/// let peripherals = Peripherals::take().unwrap();
/// let clocks = ClockController::new(
///     peripherals.PMC,
///     &peripherals.SUPC,
///     &peripherals.EFC0,
///     MainClock::Crystal12Mhz,
///     SlowClock::RcOscillator32Khz,
/// );
/// let gpio_ports = Ports::new(
///     (
///         peripherals.PIOA,
///         clocks.peripheral_clocks.pio_a.into_enabled_clock(),
///     ),
///     (
///         peripherals.PIOB,
///         clocks.peripheral_clocks.pio_b.into_enabled_clock(),
///     ),
/// );
/// let mut pins = Pins::new(gpio_ports, &peripherals.MATRIX)
///
/// // Setup SPI Master
/// let wdrbt = false; // Wait data read before transfer enabled
/// let llb = false; // Local loopback
///                 // Cycles to delay between consecutive transfers
/// let dlybct = 0; // No delay
/// // SpiU8 can be used as we're only using 8-bit SPI
/// // SpiU16 can be used for 8 to 16-bit SPI
/// let mut spi = SpiMaster::<SpiU8>::new(
///     cx.device.SPI,
///     clocks.peripheral_clocks.spi.into_enabled_clock(),
///     pins.spi_miso,
///     pins.spi_mosi,
///     pins.spi_sck,
///     spi::PeripheralSelectMode::Variable,
///     wdrbt,
///     llb,
///     dlybct,
/// );
///
/// // Setup CS0 channel
/// let mode = spi::spi::MODE_3;
/// let csa = spi::ChipSelectActive::ActiveAfterTransfer;
/// let bits = spi::BitWidth::Width8Bit;
/// let baud = spi::Hertz(12_000_000_u32); // 12 MHz
/// // Cycles to delay from CS to first valid SPCK
/// let dlybs = 0; // Half an SPCK clock period
/// let cs_settings = spi::ChipSelectSettings::new(mode, csa, bits, baud, dlybs, dlybct);
/// spi.cs_setup(0, cs_settings.clone()).unwrap();
///
/// // Enable CS0
/// spi.cs_select(0).unwrap();
///
/// // Write value
/// let val: u8 = 0x39;
/// spi.send(val.into()).unwrap();
///
/// // Read value
/// let val = match spi.read() {
///     Ok(val) => val,
///     _ => {}
/// };
/// ```
pub struct SpiMaster<FRAMESIZE> {
    spi: SPI,
    clock: PhantomData<SpiClock<Enabled>>,
    miso: PhantomData<Pa12<PfA>>,
    mosi: PhantomData<Pa13<PfA>>,
    spck: PhantomData<Pa14<PfA>>,
    cs: u8,
    lastxfer: bool,
    framesize: PhantomData<FRAMESIZE>,
}

impl<FRAMESIZE> SpiMaster<FRAMESIZE> {
    /// Initialize SPI as Master
    /// PSM - Peripheral Select Mode
    /// WDRBT - Wait Data Read Before Transfer Enabled
    /// LLB - Local Loopback
    /// DLYBCS - Delay between chip selects = DLYBCS / f_periph
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spi: SPI,
        _clock: SpiClock<Enabled>,
        _miso: Pa12<PfA>,
        _mosi: Pa13<PfA>,
        _spck: Pa14<PfA>,
        psm: PeripheralSelectMode,
        wdrbt: bool,
        llb: bool,
        dlybcs: u8,
        modfdis: bool,
    ) -> SpiMaster<FRAMESIZE> {
        unsafe {
            // Disable SPI
            spi.cr.write_with_zero(|w| w.spidis().set_bit());

            // Software reset SPI (this will reset SPI into Slave Mode)
            spi.cr.write_with_zero(|w| w.swrst().set_bit());

            // Enable SPI
            spi.cr.write_with_zero(|w| w.spien().set_bit());

            // Determine peripheral select mode
            let (ps, pcsdec) = match psm {
                PeripheralSelectMode::Fixed => (false, false),
                PeripheralSelectMode::Variable => (true, false),
                PeripheralSelectMode::ChipSelectDecode => (true, true),
            };

            // Clear spi protection mode register
            // (needed before writing to SPI_MR and SPI_CSRx)
            spi.wpmr
                .write_with_zero(|w| w.wpkey().bits(0x535049).wpen().clear_bit());

            // Setup SPI Master
            // Master Mode
            // Variable Peripheral Select (more flexible and less initial options to set)
            // Mode Fault Detection Enabled
            spi.mr.write_with_zero(|w| {
                w.mstr()
                    .set_bit()
                    .ps()
                    .bit(ps)
                    .pcsdec()
                    .bit(pcsdec)
                    .modfdis()
                    .bit(modfdis)
                    .wdrbt()
                    .bit(wdrbt)
                    .llb()
                    .bit(llb)
                    .dlybcs()
                    .bits(dlybcs)
            });
        }

        SpiMaster {
            spi,
            clock: PhantomData,
            miso: PhantomData,
            mosi: PhantomData,
            spck: PhantomData,
            cs: 0,           // Default to NPCS0
            lastxfer: false, // Reset to false on each call to send()
            framesize: PhantomData,
        }
    }

    /// Apply settings to a specific channel
    /// Uses cs 0..3 for spi channel settings
    /// When using pcsdec (Chip Decode Select)
    ///  csr0 -> 0..3
    ///  csr1 -> 4..7
    ///  csr2 -> 8..11
    ///  csr3 -> 12..14
    pub fn cs_setup(&mut self, cs: u8, settings: ChipSelectSettings) -> Result<(), Error> {
        // Lookup cs when using pcsdec
        let cs = if self.spi.mr.read().pcsdec().bit_is_set() {
            match cs {
                0..=3 => 0,
                4..=7 => 1,
                8..=11 => 2,
                12..=14 => 3,
                _ => {
                    return Err(Error::InvalidCs(cs));
                }
            }

        // Otherwise validate the cs
        } else if cs > 3 {
            return Err(Error::InvalidCs(cs));
        } else {
            cs
        };

        let cpol = match settings.mode.polarity {
            spi::Polarity::IdleLow => false,
            spi::Polarity::IdleHigh => true,
        };
        let ncpha = match settings.mode.phase {
            spi::Phase::CaptureOnFirstTransition => true,
            spi::Phase::CaptureOnSecondTransition => false,
        };
        let (csaat, csnaat) = match settings.csa {
            ChipSelectActive::ActiveAfterTransfer => (true, false),
            ChipSelectActive::ActiveOnConsecutiveTransfers => (false, false),
            ChipSelectActive::InactiveAfterEachTransfer => (false, true),
        };
        unsafe {
            self.spi.csr[cs as usize].write_with_zero(|w| {
                w.cpol()
                    .bit(cpol)
                    .ncpha()
                    .bit(ncpha)
                    .csnaat()
                    .bit(csnaat)
                    .csaat()
                    .bit(csaat)
                    .bits_()
                    .bits(settings.bits as u8)
                    .scbr()
                    .bits(settings.scbr)
                    .dlybs()
                    .bits(settings.dlybs)
                    .dlybct()
                    .bits(settings.dlybct)
            });
        }

        Ok(())
    }

    /// Select ChipSelect for next read/write FullDuplex trait functions
    /// Works around limitations in the embedded-hal trait
    /// Valid cs:
    ///  0 -> 3 (as long as NPCS0..3 are configured)
    ///  0 -> 15 (uses NPCS0..3 as the input to a 4 to 16 mux), pcsdec must be enabled
    pub fn cs_select(&mut self, cs: u8) -> Result<(), Error> {
        // Map cs to id
        let pcs_id = match cs {
            0 => 0b0000, // xxx0 => NPCS[3:0] = 1110
            1 => 0b0001, // xx01 => NPCS[3:0] = 1101
            2 => 0b0011, // x011 => NPCS[3:0] = 1011
            3 => 0b0111, // 0111 => NPCS[3:0] = 0111
            _ => 0b1111, // Forbidden
        };

        // Fixed mode
        if self.spi.mr.read().ps().bit_is_clear() {
            self.spi.mr.modify(|_, w| unsafe { w.pcs().bits(pcs_id) });

        // Variable Mode
        } else {
            // Check for pcsdec
            if self.spi.mr.read().pcsdec().bit_is_set() {
                if cs > 15 {
                    return Err(Error::InvalidCs(cs));
                }
                self.cs = cs;
            } else {
                if cs > 3 {
                    return Err(Error::InvalidCs(cs));
                }
                // Map cs to id
                self.cs = pcs_id;
            }
        }
        Ok(())
    }

    /// lastxfer set
    /// Fixed Mode
    ///  Sets lastxfer register
    /// Variable Mode
    ///  Use to set lastxfer for the next call to send()
    pub fn lastxfer(&mut self, lastxfer: bool) {
        // Fixed mode
        if self.spi.mr.read().ps().bit_is_clear() {
            unsafe {
                self.spi.cr.write_with_zero(|w| w.lastxfer().set_bit());
            }
        // Variable Mode
        } else {
            self.lastxfer = lastxfer;
        }
    }

    /// Enable Receive Data Register Full (RDRF) interrupt
    /// NOTE: Do not enable this if planning on using PDC as the PDC uses it to load the register
    pub fn enable_rdrf_interrupt(&mut self) {
        unsafe {
            self.spi.ier.write_with_zero(|w| w.rdrf().set_bit());
        }
    }

    /// Disable Receive Data Register Full (RDRF) interrupt
    pub fn disable_rdrf_interrupt(&mut self) {
        unsafe {
            self.spi.idr.write_with_zero(|w| w.rdrf().set_bit());
        }
    }

    /// Enable Transmit Data Register Empty (TDRE) interrupt
    /// NOTE: Do not enable this if planning on using PDC as the PDC uses it to load the register
    pub fn enable_tdre_interrupt(&mut self) {
        unsafe {
            self.spi.ier.write_with_zero(|w| w.tdre().set_bit());
        }
    }

    /// Disable Transmit Data Register Empty (TDRE) interrupt
    pub fn disable_tdre_interrupt(&mut self) {
        unsafe {
            self.spi.idr.write_with_zero(|w| w.tdre().set_bit());
        }
    }

    /// Enable Mode Fault Error (MODF) interrupt
    /// NOTE: Generally used in multi-master SPI environments
    pub fn enable_modf_interrupt(&mut self) {
        unsafe {
            self.spi.ier.write_with_zero(|w| w.modf().set_bit());
        }
    }

    /// Disable Mode Fault Error (MODF) interrupt
    pub fn disable_modf_interrupt(&mut self) {
        unsafe {
            self.spi.idr.write_with_zero(|w| w.modf().set_bit());
        }
    }

    /// Enable Overrun Error Status (OVRES) interrupt
    pub fn enable_ovres_interrupt(&mut self) {
        unsafe {
            self.spi.ier.write_with_zero(|w| w.ovres().set_bit());
        }
    }

    /// Disable Overrun Error Status (OVRES) interrupt
    pub fn disable_ovres_interrupt(&mut self) {
        unsafe {
            self.spi.idr.write_with_zero(|w| w.ovres().set_bit());
        }
    }
}

/// Used to convert from variable pcs to cs
/// See (33.8.4)
/// <https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-11100-32-bit%20Cortex-M4-Microcontroller-SAM4S_Datasheet.pdf>
fn variable_pcs_to_cs(pcs: u8) -> Result<u8, Error> {
    // CS0
    if (pcs & 0x1) == 0 {
        Ok(0)
    } else if (pcs & 0x2) == 0 {
        Ok(1)
    } else if (pcs & 0x4) == 0 {
        Ok(2)
    } else if (pcs & 0x8) == 0 {
        Ok(3)
    } else {
        Err(Error::InvalidCs(0xF))
    }
}

impl<FRAMESIZE> spi::FullDuplex<FRAMESIZE> for SpiMaster<FRAMESIZE>
where
    FRAMESIZE: Copy + From<SpiU16>,
    SpiU16: From<FRAMESIZE> + From<SpiU8>,
    u8: From<FRAMESIZE>,
{
    type Error = Error;

    fn read(&mut self) -> nb::Result<FRAMESIZE, Error> {
        let sr = self.spi.sr.read();
        //defmt::trace!("Read: {}", sr.rdrf().bit_is_set());

        // Check for errors (return error)
        // Check for data to read (and read it)
        // Return WouldBlock if no data available
        Err(if sr.ovres().bit_is_set() {
            defmt::trace!("Send overrun");
            nb::Error::Other(Error::Overrun)
        } else if sr.modf().bit_is_set() {
            defmt::trace!("Mode fault");
            nb::Error::Other(Error::ModeFault)
        } else if sr.spiens().bit_is_clear() {
            defmt::trace!("SPI disabled");
            nb::Error::Other(Error::SpiDisabled)
        } else if sr.rdrf().bit_is_set() {
            let rdr = self.spi.rdr.read();

            // In variable mode, verify pcs is what we expect
            if self.spi.mr.read().ps().bit_is_set()
                && variable_pcs_to_cs(rdr.pcs().bits())? != self.cs
            {
                nb::Error::Other(Error::UnexpectedPcs(rdr.rd().bits(), rdr.pcs().bits()))
            } else {
                return Ok(SpiU16(rdr.rd().bits()).into());
            }
        } else {
            nb::Error::WouldBlock
        })
    }

    fn send(&mut self, byte: FRAMESIZE) -> nb::Result<(), Error> {
        let sr = self.spi.sr.read();
        //let data: u8 = byte.into();
        //defmt::trace!("Send: {} {}", data, sr.tdre().bit_is_set());

        // Check for errors (return error)
        // Make sure buffer is empty (then write if available)
        // Return WouldBlock if buffer is full
        Err(if sr.ovres().bit_is_set() {
            defmt::trace!("Send overrun");
            nb::Error::Other(Error::Overrun)
        } else if sr.modf().bit_is_set() {
            defmt::trace!("Send mode fault");
            nb::Error::Other(Error::ModeFault)
        } else if sr.spiens().bit_is_clear() {
            defmt::trace!("Send spi disabled");
            nb::Error::Other(Error::SpiDisabled)
        } else if sr.tdre().bit_is_set() {
            // Fixed Mode
            if self.spi.mr.read().ps().bit_is_clear() {
                self.write_fixed_data_reg(byte);

            // Variable Mode
            } else {
                self.write_variable_data_reg(byte);
            }
            return Ok(());
        } else {
            nb::Error::WouldBlock
        })
    }
}

impl<FRAMESIZE> crate::hal::blocking::spi::transfer::Default<FRAMESIZE> for SpiMaster<FRAMESIZE>
where
    FRAMESIZE: Copy + From<SpiU16>,
    SpiU16: From<FRAMESIZE> + From<SpiU8>,
    u8: From<FRAMESIZE>,
{
}

impl crate::hal::blocking::spi::Write<SpiU8> for SpiMaster<SpiU8> {
    type Error = Error;

    fn write(&mut self, words: &[SpiU8]) -> Result<(), Error> {
        self.spi_write(words)
    }
}

impl crate::hal::blocking::spi::Write<SpiU16> for SpiMaster<SpiU16> {
    type Error = Error;

    fn write(&mut self, words: &[SpiU16]) -> Result<(), Error> {
        self.spi_write(words)
    }
}

pub trait SpiReadWrite<T> {
    fn read_data_reg(&mut self) -> T;
    fn write_fixed_data_reg(&mut self, data: T);
    fn write_variable_data_reg(&mut self, data: T);
    fn spi_write(&mut self, words: &[T]) -> Result<(), Error>;
}

impl<FRAMESIZE> SpiReadWrite<FRAMESIZE> for SpiMaster<FRAMESIZE>
where
    FRAMESIZE: Copy + From<SpiU16>,
    SpiU16: From<FRAMESIZE> + From<SpiU8>,
{
    fn read_data_reg(&mut self) -> FRAMESIZE {
        let rdr = self.spi.rdr.read();
        SpiU16(rdr.rd().bits()).into()
    }

    fn write_fixed_data_reg(&mut self, data: FRAMESIZE) {
        unsafe {
            let data: SpiU16 = data.into();
            self.spi.tdr.write_with_zero(|w| w.td().bits(data.0));
        }
    }

    fn write_variable_data_reg(&mut self, data: FRAMESIZE) {
        // NOTE: Uses self.cs to write the pcs register field
        unsafe {
            let data: SpiU16 = data.into();
            self.spi.tdr.write_with_zero(|w| {
                w.td()
                    .bits(data.0)
                    .pcs()
                    .bits(self.cs)
                    .lastxfer()
                    .bit(self.lastxfer)
            });
        }
    }

    fn spi_write(&mut self, words: &[FRAMESIZE]) -> Result<(), Error> {
        for word in words {
            loop {
                let sr = self.spi.sr.read();
                if sr.tdre().bit_is_set() {
                    // Fixed Mode
                    if self.spi.mr.read().ps().bit_is_clear() {
                        self.write_fixed_data_reg(*word);

                    // Variable Mode
                    } else {
                        self.write_variable_data_reg(*word);
                    }
                    if sr.modf().bit_is_set() {
                        return Err(Error::ModeFault);
                    }
                }
            }
        }
        Ok(())
    }
}

/// 8-bit fixed mode
/// 8-bit data storage
/// Any SPI settings must be done using the registers
/// See section: 33.7.3.6
/// <https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-11100-32-bit%20Cortex-M4-Microcontroller-SAM4S_Datasheet.pdf>
///
/// or
///
/// 9-16 bit fixed mode
/// 16-bit data storage
/// Any SPI settings must be done using the registers
/// See section: 33.7.3.6
/// <https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-11100-32-bit%20Cortex-M4-Microcontroller-SAM4S_Datasheet.pdf>
pub struct Fixed;

/// Variable mode
/// 8-16 bit transfer sizes
/// Can do per data word setting adjustments using the DMA stream
/// 32-bits store:
/// - data (8-16 bits)
/// - pcs (CS) (4 bits)
/// - lastxfer (1 bit)
///
/// Not as efficient RAM/flash wise, but fewer interrupts and polling loops are required as PDC can
/// handle entire sequences talking to many SPI chips.
///
/// See section: 33.7.3.6
/// <https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-11100-32-bit%20Cortex-M4-Microcontroller-SAM4S_Datasheet.pdf>
pub struct Variable;

pub struct SpiPayload<MODE, FRAMESIZE> {
    spi: SpiMaster<FRAMESIZE>,
    _mode: PhantomData<MODE>,
}

pub type SpiRxDma<MODE, FRAMESIZE> = RxDma<SpiPayload<MODE, FRAMESIZE>>;
pub type SpiTxDma<MODE, FRAMESIZE> = TxDma<SpiPayload<MODE, FRAMESIZE>>;
pub type SpiRxTxDma<MODE, FRAMESIZE> = RxTxDma<SpiPayload<MODE, FRAMESIZE>>;

macro_rules! spi_pdc {
    (
        $Mode:ident, $Framesize:ident
    ) => {
        paste! {
            impl SpiMaster<$Framesize> {
                /// SPI with PDC, Rx only
                pub fn with_pdc_rx(self) -> SpiRxDma<$Mode, $Framesize> {
                    let payload = SpiPayload {
                        spi: self,
                        _mode: PhantomData,
                    };
                    RxDma { payload }
                }

                /// SPI with PDC, Tx only
                pub fn with_pdc_tx(self) -> SpiTxDma<$Mode, $Framesize> {
                    let payload = SpiPayload {
                        spi: self,
                        _mode: PhantomData,
                    };
                    TxDma { payload }
                }

                /// SPI with PDC, Rx+TX
                /// ```
                /// use atsam4_hal::clock::{ClockController, MainClock, SlowClock};
                /// use atsam4_hal::pac::Peripherals;
                ///
                /// let peripherals = Peripherals::take().unwrap();
                /// let clocks = ClockController::new(
                ///     peripherals.PMC,
                ///     &peripherals.SUPC,
                ///     &peripherals.EFC0,
                ///     MainClock::Crystal12Mhz,
                ///     SlowClock::RcOscillator32Khz,
                /// );
                /// let gpio_ports = Ports::new(
                ///     (
                ///         peripherals.PIOA,
                ///         clocks.peripheral_clocks.pio_a.into_enabled_clock(),
                ///     ),
                ///     (
                ///         peripherals.PIOB,
                ///         clocks.peripheral_clocks.pio_b.into_enabled_clock(),
                ///     ),
                /// );
                /// let mut pins = Pins::new(gpio_ports, &peripherals.MATRIX)
                ///
                /// // Setup SPI Master
                /// let wdrbt = false; // Wait data read before transfer enabled
                /// let llb = false; // Local loopback
                ///                 // Cycles to delay between consecutive transfers
                /// let dlybct = 0; // No delay
                /// // SpiU8 can be used as we're only using 8-bit SPI
                /// // SpiU16 can be used for 8 to 16-bit SPI
                /// let mut spi = SpiMaster::<SpiU8>::new(
                ///     cx.device.SPI,
                ///     clocks.peripheral_clocks.spi.into_enabled_clock(),
                ///     pins.spi_miso,
                ///     pins.spi_mosi,
                ///     pins.spi_sck,
                ///     spi::PeripheralSelectMode::Variable,
                ///     wdrbt,
                ///     llb,
                ///     dlybct,
                /// );
                ///
                /// // Setup SPI with pdc
                /// let spi_tx_buf: [u32; 10] = [5; 10],
                /// let spi_rx_buf: [u32; 10] = [0; 10],
                /// let mut spi = spi.with_pdc_rxtx();
                /// // Same as read_write() but use a smaller subset of the given buffer
                /// let txfr = spi.read_write_len(spi_rx_buf, spi_tx_buf, 7);
                /// let ((rx_buf, tx_buf), spi) = txfr.wait();
                /// ```
                pub fn with_pdc_rxtx(self) -> SpiRxTxDma<$Mode, $Framesize> {
                    let payload = SpiPayload {
                        spi: self,
                        _mode: PhantomData,
                    };
                    RxTxDma { payload }
                }
            }

            // Setup PDC Rx/Tx functionality
            pub type [<SpiMaster $Framesize>] = SpiMaster<$Framesize>;
            pdc_rx! { [<SpiMaster $Framesize>]: spi, sr }
            pdc_tx! { [<SpiMaster $Framesize>]: spi, sr }
            pdc_rxtx! { [<SpiMaster $Framesize>]: spi }

            impl Transmit for SpiTxDma<$Mode, $Framesize> {
                type ReceivedWord = $Framesize;
            }

            impl Receive for SpiRxDma<$Mode, $Framesize> {
                type TransmittedWord = $Framesize;
            }

            impl Receive for SpiRxTxDma<$Mode, $Framesize> {
                type TransmittedWord = $Framesize;
            }

            impl Transmit for SpiRxTxDma<$Mode, $Framesize> {
                type ReceivedWord = $Framesize;
            }

            impl SpiRxDma<$Mode, $Framesize> {
                /// Reverts SpiRxDma back to SpiMaster
                pub fn revert(mut self) -> SpiMaster<$Framesize> {
                    self.payload.spi.stop_rx_pdc();
                    self.payload.spi
                }
            }

            impl<B> ReadDma<B, $Framesize> for SpiRxDma<$Mode, $Framesize>
            where
                Self: TransferPayload,
                B: WriteBuffer<Word = $Framesize>,
            {
                /// Assigns the buffer, enables PDC and starts SPI transaction
                fn read(mut self, mut buffer: B) -> Transfer<W, B, Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, len) = unsafe { buffer.write_buffer() };
                    self.payload.spi.set_receive_address(ptr as u32);
                    self.payload.spi.set_receive_counter(len as u16);

                    compiler_fence(Ordering::Release);
                    self.start();

                    Transfer::w(buffer, self)
                }
            }

            impl TransferPayload for SpiRxDma<$Mode, $Framesize> {
                fn start(&mut self) {
                    self.payload.spi.start_rx_pdc();
                }
                fn stop(&mut self) {
                    self.payload.spi.stop_rx_pdc();
                }
                fn in_progress(&self) -> bool {
                    self.payload.spi.rx_in_progress()
                }
            }

            impl SpiTxDma<$Mode, $Framesize> {
                /// Reverts SpiTxDma back to SpiMaster
                pub fn revert(mut self) -> SpiMaster<$Framesize> {
                    self.payload.spi.stop_tx_pdc();
                    self.payload.spi
                }
            }

            impl<B> WriteDma<B, $Framesize> for SpiTxDma<$Mode, $Framesize>
            where
                Self: TransferPayload,
                B: ReadBuffer<Word = $Framesize>,
            {
                /// Assigns the write buffer, enables PDC and starts SPI transaction
                fn write(mut self, buffer: B) -> Transfer<R, B, Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, len) = unsafe { buffer.read_buffer() };
                    self.payload.spi.set_transmit_address(ptr as u32);
                    self.payload.spi.set_transmit_counter(len as u16);

                    compiler_fence(Ordering::Release);
                    self.start();

                    Transfer::r(buffer, self)
                }
            }

            impl TransferPayload for SpiTxDma<$Mode, $Framesize> {
                fn start(&mut self) {
                    self.payload.spi.start_tx_pdc();
                }
                fn stop(&mut self) {
                    self.payload.spi.stop_tx_pdc();
                }
                fn in_progress(&self) -> bool {
                    self.payload.spi.tx_in_progress()
                }
            }

            impl SpiRxTxDma<$Mode, $Framesize> {
                /// Reverts SpiRxTxDma back to SpiMaster
                pub fn revert(mut self) -> SpiMaster<$Framesize> {
                    self.payload.spi.stop_rxtx_pdc();
                    self.payload.spi
                }
            }

            impl<RXB, TXB> ReadWriteDma<RXB, TXB, $Framesize> for SpiRxTxDma<$Mode, $Framesize>
            where
                Self: TransferPayload,
                RXB: WriteBuffer<Word = $Framesize>,
                TXB: ReadBuffer<Word = $Framesize>,
            {
                fn read_write(mut self, mut rx_buffer: RXB, tx_buffer: TXB) -> Transfer<W, (RXB, TXB), Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, rx_len) = unsafe { rx_buffer.write_buffer() };
                    self.payload.spi.set_receive_address(ptr as u32);
                    self.payload.spi.set_receive_counter(rx_len as u16);

                    let (ptr, tx_len) = unsafe { tx_buffer.read_buffer() };
                    self.payload.spi.set_transmit_address(ptr as u32);
                    self.payload.spi.set_transmit_counter(tx_len as u16);

                    if rx_len != tx_len {
                        panic!("rx_len: {} != tx:len: {}", rx_len, tx_len);
                    }

                    compiler_fence(Ordering::Release);
                    self.start();

                    Transfer::w((rx_buffer, tx_buffer), self)
                }
            }

            impl<RXB, TXB> ReadWriteDmaLen<RXB, TXB, $Framesize> for SpiRxTxDma<$Mode, $Framesize>
            where
                Self: TransferPayload,
                RXB: WriteBuffer<Word = $Framesize>,
                TXB: ReadBuffer<Word = $Framesize>,
            {
                /// Same as read_write(), but allows for a specified length
                fn read_write_len(mut self, mut rx_buffer: RXB, rx_buf_len: usize, tx_buffer: TXB, tx_buf_len: usize) -> Transfer<W, (RXB, TXB), Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, rx_len) = unsafe { rx_buffer.write_buffer() };
                    self.payload.spi.set_receive_address(ptr as u32);
                    self.payload.spi.set_receive_counter(rx_buf_len as u16);
                    if rx_len < rx_buf_len {
                        panic!("rx_len: {} < rx_buf_len: {}", rx_len, rx_buf_len);
                    }

                    let (ptr, tx_len) = unsafe { tx_buffer.read_buffer() };
                    self.payload.spi.set_transmit_address(ptr as u32);
                    self.payload.spi.set_transmit_counter(tx_buf_len as u16);
                    if tx_len < tx_buf_len {
                        panic!("tx_len: {} < tx_buf_len: {}", tx_len, tx_buf_len);
                    }

                    compiler_fence(Ordering::Release);
                    self.start();

                    Transfer::w((rx_buffer, tx_buffer), self)
                }
            }

            impl TransferPayload for SpiRxTxDma<$Mode, $Framesize> {
                fn start(&mut self) {
                    self.payload.spi.start_rxtx_pdc();
                }
                fn stop(&mut self) {
                    self.payload.spi.stop_rxtx_pdc();
                }
                fn in_progress(&self) -> bool {
                    self.payload.spi.tx_in_progress() || self.payload.spi.rx_in_progress()
                }
            }
        }
    }
}

// Setup SPI for each of the 3 different datastructures
spi_pdc! { Fixed, u8 }
spi_pdc! { Fixed, u16 }
spi_pdc! { Variable, u32 }
