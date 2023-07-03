//! ADC Implementation
//! Loosely based off of <https://github.com/atsamd-rs/atsamd/blob/master/hal/src/thumbv7em/adc.rs>
//! NOTE: From ASF the following MCUs could be supported with this module
//!       (sam/drivers/adc/adc.c)
//!       atsam3s
//!       atsam3n
//!       atsam3u
//!       atsam3xa
//!       atsam4s (supported)
//!       atsam4c
//!       atsam4cp
//!       atsam4cm
//!
//! TODO
//! - Additional power saving (peripheral clock)
//! - Automatic comparison

use crate::clock::{AdcClock, Enabled};
use crate::gpio::*;
use crate::hal::adc::{Channel, OneShot};
use crate::pac::ADC;
use crate::pdc::*;
use core::marker::PhantomData;
use core::sync::atomic::{compiler_fence, Ordering};
use cortex_m::singleton;
use embedded_dma::WriteBuffer;
use fugit::RateExtU32;

#[derive(PartialEq, Eq, Copy, Clone, Debug, defmt::Format)]
pub enum Powersaving {
    /// ADC core and reference voltage circuitry are kept on between conversions
    Normal,
    /// Voltage reference is kept on, but ADC core is disabled between conversions
    FastWakeup,
    /// ADC core and voltage reference are disabled between conversions
    Sleep,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, defmt::Format)]
pub enum SingleEndedGain {
    /// Single-ended gain = 1
    Gain1x = 1,
    /// Single-ended gain = 2
    Gain2x = 2,
    /// Single-ended gain = 4
    Gain4x = 3,
}

/// An ADC where results are accessible via interrupt servicing.
/// This is based off of atsamd-hal's InterruptAdc interface.
/// It supports both single conversion and free running using an interrupt.
///
/// ```
/// use atsam4_hal::adc::Adc;
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
/// let gpio_ports = Ports::new( // ATSAM4S8B
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
/// let mut adc = Adc::new(
///     peripherals.ADC,
///     clocks.peripheral_clocks.adc.into_enabled_clock(),
/// );
/// // Channels, gain and offset must be enabled and set before starting autocalibration
/// adc.enable_channel(&mut pins.sense1); // pin sense1 = a17<ExFn, into_extra_function>
/// adc.enable_channel(&mut pins.sense2); // pin sense2 = a18<ExFn, into_extra_function>
/// adc.gain(&mut pins.sense1, SingleEndedGain::Gain4x);
/// adc.offset(&mut pins.sense1, true);
/// adc.autocalibration(true); // Waits for autocalibration to complete
///
/// // Convert to an InterruptAdc (either SingleConversion or FreeRunning)
/// let mut adc: InterruptAdc<FreeRunning> = InterruptAdc::from(adc);
/// // NOTE: You must both enable the channel and start the conversion for each pin
/// //       Enabling the pins will start sampling; however, you must call start_conversion
/// //       in order to enable the interrupt.
/// adc.start_conversion(&mut pins.sense1);
/// adc.start_conversion(&mut pins.sense2);
///
/// // RTIC interrupt. adc, sense1 and sense2 are the resource
/// #[task(binds = ADC, resources = [adc, sense1, sense2])]
/// fn adc(cx: adc::Context) {
///     if let Some(values) = cx.resources.adc.service_interrupt_ready(cx.resources.sense1) {
///         defmt::trace!("CH0 {}", values);
///     }
///     if let Some(values) = cx.resources.adc.service_interrupt_ready(cx.resources.sense2) {
///         defmt::trace!("CH1 {}", values);
///     }
/// }
/// ```
pub struct InterruptAdc<C>
where
    C: ConversionMode,
{
    adc: Adc,
    m: core::marker::PhantomData<C>,
}

/// Single shot ADC conversions
/// Implements the embedded-hal ADC OneShot interface
///
/// ```
/// use atsam4_hal::adc::Adc;
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
/// let mut adc = Adc::new(
///     peripherals.ADC,
///     clocks.peripheral_clocks.adc.into_enabled_clock(),
/// );
/// // Channels, gain and offset must be enabled and set before starting autocalibration
/// adc.enable_channel(&mut pins.sense1); // pin sense1 = a17<ExFn, into_extra_function>
/// adc.gain(&mut pins.sense1, SingleEndedGain::Gain4x);
/// adc.offset(&mut pins.sense1, true);
/// adc.autocalibration(true); // Waits for autocalibration to complete
///
/// // Read a single value from the ADC channel
/// let _value: u16 = adc.read(pins.sense1).unwrap();
/// ```
pub struct Adc {
    adc: ADC,
    clock: PhantomData<AdcClock<Enabled>>,
}

pub struct SingleConversion;
pub struct FreeRunning;

/// Describes how an interrupt-driven ADC should finalize the peripheral
/// when the conversion completes.
pub trait ConversionMode {
    fn on_start(adc: &mut Adc);
    fn on_complete<PIN: Channel<ADC, ID = u8>>(adc: &mut Adc, pin: &mut PIN);
    fn on_stop<PIN: Channel<ADC, ID = u8>>(adc: &mut Adc, pin: &mut PIN);
}

impl Adc {
    pub fn new(adc: ADC, clock: AdcClock<Enabled>) -> Self {
        // Clear ADC write-protect
        adc.wpmr
            .modify(|_, w| w.wpkey().passwd().wpen().clear_bit());

        /*
         * Formula: ADCClock = MCK / ((PRESCAL+1) * 2)
         *  MCK = 120MHz, PRESCAL = 2, then:
         *  ADCClock = 120 / ((2+1) * 2) = 20MHz;
         *  sam4s max ADCClock = 22 MHz
         *
         * Formula:
         *     Startup  Time = startup value / ADCClock
         *     Startup time = 64 / 20MHz = 3.2 us (4)
         *     Startup time = 80 / 20MHz = 4 us (5)
         *     Startup time = 96 / 20MHz = 4.8 us (6)
         *     Startup time = 112 / 20MHz = 5.6 us (7)
         *     Startup time = 512 / 20MHz = 25.6 us (8)
         *     sam4s Min Startup Time = 4 us (max 12 us)
         *
         * adc_init(ADC, sysclk_get_cpu_hz(), 20000000, ADC_STARTUP_TIME_5);
         */
        unsafe {
            // Reset the controller (simulates hardware reset)
            adc.cr.write_with_zero(|w| w.swrst().set_bit());

            // Reset mode register (set all fields to 0)
            adc.mr.write_with_zero(|w| w);

            // Reset PDC transfer
            adc.ptcr
                .write_with_zero(|w| w.rxtdis().set_bit().txtdis().set_bit());
        }

        // Setup prescalar and startup time
        let prescaler = (clock.frequency() / (2 * 20_u32.MHz::<1, 1>()) - 1) as u8;
        adc.mr
            .modify(|_, w| unsafe { w.prescal().bits(prescaler).startup().sut80() });

        /* Set ADC timing.
         * Formula:
         *
         *     Ttrack minimum = 0.054 * Zsource + 205
         *     Ttrack minimum = 0.054 * 1.5k + 205 = 286 ns
         *     Ttrack minimum = 0.054 * 10k + 205 = 745 ns
         *     Ttrack minimum = 0.054 * 20k + 205 = 1285 ns
         *     20MHz -> 50 ns * 15 cycles = 750 ns
         *     750 ns > 286 ns -> Tracktim can be set to 0
         *     750 ns > 745 ns -> Tracktim can be set to 0
         *     750 ns < 1285 ns -> Tracktim can be set to 10 => 750 ns + 550 ns (10) = 1300 ns
         *     See sam4s datasheet Figure 44-21 and Table 44-41 for details
         *
         *     Transfer Time = (TRANSFER * 2 + 3) / ADCClock
         *     Tracking Time = (TRACKTIM + 1) / ADCClock
         *     Settling Time = settling value / ADCClock
         *
         *     Hold Time
         *     Transfer Time = (0 * 2 + 3) / 20MHz = 150 ns
         *     Transfer Time = (1 * 2 + 3) / 20MHz = 250 ns
         *     Transfer Time = (2 * 2 + 3) / 20MHz = 350 ns
         *     Transfer Time = (3 * 2 + 3) / 20MHz = 450 ns
         *
         *     Track Time
         *     Tracking Time = (0 + 1) / 20MHz = 50 ns
         *     Tracking Time = (1 + 1) / 20MHz = 100 ns
         *     Tracking Time = (2 + 1) / 20MHz = 150 ns
         *     Tracking Time = (3 + 1) / 20MHz = 200 ns
         *     Tracking Time = (4 + 1) / 20MHz = 250 ns
         *     Tracking Time = (5 + 1) / 20MHz = 300 ns
         *     Tracking Time = (6 + 1) / 20MHz = 350 ns
         *     Tracking Time = (7 + 1) / 20MHz = 400 ns
         *     Tracking Time = (8 + 1) / 20MHz = 450 ns
         *     Tracking Time = (9 + 1) / 20MHz = 500 ns
         *     Tracking Time = (10 + 1) / 20MHz = 550 ns
         *     Tracking Time = (11 + 1) / 20MHz = 600 ns
         *     Tracking Time = (12 + 1) / 20MHz = 650 ns
         *     Tracking Time = (13 + 1) / 20MHz = 700 ns
         *     Tracking Time = (14 + 1) / 20MHz = 750 ns
         *     Tracking Time = (15 + 1) / 20MHz = 800 ns
         *
         *     Analog Settling Time
         *     (TODO May need to tune this)
         *     Settling Time = 3 / 20MHz = 150 ns (0)
         *     Settling Time = 5 / 20MHz = 250 ns (1)
         *     Settling Time = 9 / 20MHz = 450 ns (2)
         *     Settling Time = 17 / 20MHz = 850 ns (3)
         *
         * const uint8_t tracking_time = 10;
         * const uint8_t transfer_period = 2; // Recommended to be set to 2 by datasheet (42.7.2)
         * adc_configure_timing(ADC, tracking_time, ADC_SETTLING_TIME_1, transfer_period);
         */
        let tracking_time = 10;
        let transfer_period = 2;
        adc.mr.modify(|_, w| unsafe {
            w.transfer()
                .bits(transfer_period)
                .tracktim()
                .bits(tracking_time)
                .settling()
                .ast5()
        });

        // Enable temperature sensor
        adc.acr.modify(|_, w| w.tson().set_bit());

        // Allow different gain/offset values for each channel
        adc.mr.modify(|_, w| w.anach().allowed());

        Self {
            adc,
            clock: PhantomData,
        }
    }

    /// Enables channel number tags
    /// Can be used for DMA to tag channel data.
    /// Not necessary, as by default the ordering is by the enabled channels
    /// or per the set sequence. Mainly for convenience.
    pub fn enable_tags(&mut self) {
        // Enable channel number tag for LCDR
        self.adc.emr.modify(|_, w| w.tag().set_bit());
    }

    /// Disables channel number tags
    pub fn disable_tags(&mut self) {
        // Enable channel number tag for LCDR
        self.adc.emr.modify(|_, w| w.tag().clear_bit());
    }

    /// Reterns a PIN-like reference for the temperature sensor
    /// Used to enabled and start sampling of the temperature channel (15)
    /// Cannot use pins as there is no pin reference.
    /// ```
    /// let mut adc = Adc::new(
    ///     peripherals.ADC,
    ///     clocks.peripheral_clocks.adc.into_enabled_clock(),
    /// );
    /// let temp_sensor = adc.temp_sensor();
    /// adc.enable_channel(temp_sensor);
    /// ```
    pub fn temp_sensor(&mut self) -> &'static mut TempSensor {
        singleton!(: TempSensor = TempSensor {}).unwrap()
    }

    /// Sets the channel read sequence
    /// Used with the PDC
    pub fn sequence(&mut self, channels: &[u8]) {
        for (pos, ch) in channels.iter().enumerate() {
            match pos {
                0 => self.adc.seqr1.modify(|_, w| unsafe { w.usch1().bits(*ch) }),
                1 => self.adc.seqr1.modify(|_, w| unsafe { w.usch2().bits(*ch) }),
                2 => self.adc.seqr1.modify(|_, w| unsafe { w.usch3().bits(*ch) }),
                3 => self.adc.seqr1.modify(|_, w| unsafe { w.usch4().bits(*ch) }),
                4 => self.adc.seqr1.modify(|_, w| unsafe { w.usch5().bits(*ch) }),
                5 => self.adc.seqr1.modify(|_, w| unsafe { w.usch6().bits(*ch) }),
                6 => self.adc.seqr1.modify(|_, w| unsafe { w.usch7().bits(*ch) }),
                7 => self.adc.seqr1.modify(|_, w| unsafe { w.usch8().bits(*ch) }),
                8 => self.adc.seqr2.modify(|_, w| unsafe { w.usch9().bits(*ch) }),
                9 => self
                    .adc
                    .seqr2
                    .modify(|_, w| unsafe { w.usch10().bits(*ch) }),
                10 => self
                    .adc
                    .seqr2
                    .modify(|_, w| unsafe { w.usch11().bits(*ch) }),
                11 => self
                    .adc
                    .seqr2
                    .modify(|_, w| unsafe { w.usch12().bits(*ch) }),
                12 => self
                    .adc
                    .seqr2
                    .modify(|_, w| unsafe { w.usch13().bits(*ch) }),
                13 => self
                    .adc
                    .seqr2
                    .modify(|_, w| unsafe { w.usch14().bits(*ch) }),
                14 => self
                    .adc
                    .seqr2
                    .modify(|_, w| unsafe { w.usch15().bits(*ch) }),
                _ => {
                    panic!("Invalid sequence position: {}", pos);
                }
            }
        }
    }

    /// Enable ADC sequencing
    /// When enabled, the ADC channel sequence register is used to determine the ADC read order
    pub fn enable_sequencing(&mut self) {
        self.adc.mr.modify(|_, w| w.useq().set_bit());
    }

    /// Disable ADC sequencing
    /// When disabled (default), the ADC channels are converted by (numerical) channel order
    pub fn disable_sequencing(&mut self) {
        self.adc.mr.modify(|_, w| w.useq().clear_bit());
    }

    /// Set the powersaving mode
    /// By default set to Normal (no powersaving)
    pub fn powersaving(&mut self, ps: Powersaving) {
        match ps {
            Powersaving::Normal => self.adc.mr.modify(|_, w| w.sleep().normal()),
            Powersaving::FastWakeup => self.adc.mr.modify(|_, w| w.sleep().sleep().fwup().on()),
            Powersaving::Sleep => self.adc.mr.modify(|_, w| w.sleep().sleep().fwup().off()),
        }
    }

    /// Set channel single-ended gain
    /// See Section 42.6.10 in ATSAM4S datasheet for details
    /// NOTE: You must run calibration if gain settings are changed
    pub fn gain<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN, gain: SingleEndedGain) {
        match PIN::channel() {
            0 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain0().bits(gain as u8) }),
            1 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain1().bits(gain as u8) }),
            2 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain2().bits(gain as u8) }),
            3 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain3().bits(gain as u8) }),
            4 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain4().bits(gain as u8) }),
            5 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain5().bits(gain as u8) }),
            6 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain6().bits(gain as u8) }),
            7 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain7().bits(gain as u8) }),
            8 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain8().bits(gain as u8) }),
            9 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain9().bits(gain as u8) }),
            10 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain10().bits(gain as u8) }),
            11 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain11().bits(gain as u8) }),
            12 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain12().bits(gain as u8) }),
            13 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain13().bits(gain as u8) }),
            14 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain14().bits(gain as u8) }),
            15 => self
                .adc
                .cgr
                .modify(|_, w| unsafe { w.gain15().bits(gain as u8) }),
            _ => {
                panic!("Invalid channel: {}", PIN::channel());
            }
        }
    }

    /// Set voltage offset
    /// See Section 42.6.10 in ATSAM4S datasheet for details
    /// NOTE: You must run calibration if offset settings are changed
    pub fn offset<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN, offset: bool) {
        match PIN::channel() {
            0 => self.adc.cor.modify(|_, w| w.off0().bit(offset)),
            1 => self.adc.cor.modify(|_, w| w.off1().bit(offset)),
            2 => self.adc.cor.modify(|_, w| w.off2().bit(offset)),
            3 => self.adc.cor.modify(|_, w| w.off3().bit(offset)),
            4 => self.adc.cor.modify(|_, w| w.off4().bit(offset)),
            5 => self.adc.cor.modify(|_, w| w.off5().bit(offset)),
            6 => self.adc.cor.modify(|_, w| w.off6().bit(offset)),
            7 => self.adc.cor.modify(|_, w| w.off7().bit(offset)),
            8 => self.adc.cor.modify(|_, w| w.off8().bit(offset)),
            9 => self.adc.cor.modify(|_, w| w.off9().bit(offset)),
            10 => self.adc.cor.modify(|_, w| w.off10().bit(offset)),
            11 => self.adc.cor.modify(|_, w| w.off11().bit(offset)),
            12 => self.adc.cor.modify(|_, w| w.off12().bit(offset)),
            13 => self.adc.cor.modify(|_, w| w.off13().bit(offset)),
            14 => self.adc.cor.modify(|_, w| w.off14().bit(offset)),
            15 => self.adc.cor.modify(|_, w| w.off15().bit(offset)),
            _ => {
                panic!("Invalid channel: {}", PIN::channel());
            }
        }
    }

    /// Autocalibration
    /// Wait will block until autocalibration is complete
    pub fn autocalibration(&mut self, wait: bool) {
        unsafe { self.adc.cr.write_with_zero(|w| w.autocal().set_bit()) };

        if wait {
            while !self.calibration_ready() {}
        }
    }

    /// Checks if calibration is ready
    /// Will return false if calibration is not ready, or calibration has not been requested
    pub fn calibration_ready(&self) -> bool {
        self.adc.isr.read().eocal().bit()
    }

    fn enable_freerunning(&mut self) {
        self.adc.mr.modify(|_, w| w.freerun().on());
    }

    fn disable_freerunning(&mut self) {
        self.adc.mr.modify(|_, w| w.freerun().off());
    }

    fn synchronous_convert<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN) -> u16 {
        self.start_conversion();
        // Poll End-of-Conversions status bit
        while !match PIN::channel() {
            0 => self.adc.isr.read().eoc0().bit(),
            1 => self.adc.isr.read().eoc1().bit(),
            2 => self.adc.isr.read().eoc2().bit(),
            3 => self.adc.isr.read().eoc3().bit(),
            4 => self.adc.isr.read().eoc4().bit(),
            5 => self.adc.isr.read().eoc5().bit(),
            6 => self.adc.isr.read().eoc6().bit(),
            7 => self.adc.isr.read().eoc7().bit(),
            8 => self.adc.isr.read().eoc8().bit(),
            9 => self.adc.isr.read().eoc9().bit(),
            10 => self.adc.isr.read().eoc10().bit(),
            11 => self.adc.isr.read().eoc11().bit(),
            12 => self.adc.isr.read().eoc12().bit(),
            13 => self.adc.isr.read().eoc13().bit(),
            14 => self.adc.isr.read().eoc14().bit(),
            15 => self.adc.isr.read().eoc15().bit(),
            _ => {
                panic!("Invalid channel: {}", PIN::channel());
            }
        } {}
        // Read data register for the specified channel
        self.adc.cdr[PIN::channel() as usize].read().data().bits()
    }

    fn start_conversion(&mut self) {
        unsafe { self.adc.cr.write_with_zero(|w| w.start().set_bit()) };
    }

    /// Enables the ADC pin channel
    /// A channel, if used, must be enabled before autocalibration
    /// This is needed to determine the various gain+offset settings used
    /// See Section in 42.6.12 in the ATSAM4S datasheet for more details
    pub fn enable_channel<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN) {
        unsafe {
            match PIN::channel() {
                0 => self.adc.cher.write_with_zero(|w| w.ch0().set_bit()),
                1 => self.adc.cher.write_with_zero(|w| w.ch1().set_bit()),
                2 => self.adc.cher.write_with_zero(|w| w.ch2().set_bit()),
                3 => self.adc.cher.write_with_zero(|w| w.ch3().set_bit()),
                4 => self.adc.cher.write_with_zero(|w| w.ch4().set_bit()),
                5 => self.adc.cher.write_with_zero(|w| w.ch5().set_bit()),
                6 => self.adc.cher.write_with_zero(|w| w.ch6().set_bit()),
                7 => self.adc.cher.write_with_zero(|w| w.ch7().set_bit()),
                8 => self.adc.cher.write_with_zero(|w| w.ch8().set_bit()),
                9 => self.adc.cher.write_with_zero(|w| w.ch9().set_bit()),
                10 => self.adc.cher.write_with_zero(|w| w.ch10().set_bit()),
                11 => self.adc.cher.write_with_zero(|w| w.ch11().set_bit()),
                12 => self.adc.cher.write_with_zero(|w| w.ch12().set_bit()),
                13 => self.adc.cher.write_with_zero(|w| w.ch13().set_bit()),
                14 => self.adc.cher.write_with_zero(|w| w.ch14().set_bit()),
                15 => self.adc.cher.write_with_zero(|w| w.ch15().set_bit()),
                _ => {
                    panic!("Invalid channel: {}", PIN::channel());
                }
            }
        }
    }

    pub fn disable_channel<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN) {
        unsafe {
            match PIN::channel() {
                0 => self.adc.chdr.write_with_zero(|w| w.ch0().set_bit()),
                1 => self.adc.chdr.write_with_zero(|w| w.ch1().set_bit()),
                2 => self.adc.chdr.write_with_zero(|w| w.ch2().set_bit()),
                3 => self.adc.chdr.write_with_zero(|w| w.ch3().set_bit()),
                4 => self.adc.chdr.write_with_zero(|w| w.ch4().set_bit()),
                5 => self.adc.chdr.write_with_zero(|w| w.ch5().set_bit()),
                6 => self.adc.chdr.write_with_zero(|w| w.ch6().set_bit()),
                7 => self.adc.chdr.write_with_zero(|w| w.ch7().set_bit()),
                8 => self.adc.chdr.write_with_zero(|w| w.ch8().set_bit()),
                9 => self.adc.chdr.write_with_zero(|w| w.ch9().set_bit()),
                10 => self.adc.chdr.write_with_zero(|w| w.ch10().set_bit()),
                11 => self.adc.chdr.write_with_zero(|w| w.ch11().set_bit()),
                12 => self.adc.chdr.write_with_zero(|w| w.ch12().set_bit()),
                13 => self.adc.chdr.write_with_zero(|w| w.ch13().set_bit()),
                14 => self.adc.chdr.write_with_zero(|w| w.ch14().set_bit()),
                15 => self.adc.chdr.write_with_zero(|w| w.ch15().set_bit()),
                _ => {
                    panic!("Invalid channel: {}", PIN::channel());
                }
            }
        }
    }

    /// Enables interrupts each channel
    /// This does not use DRDY as DRDY under freerunning mode using interrupts
    /// has the tendency to lose samples. Instead each channel interrupt is used instead
    /// so that no samples are lost.
    fn enable_interrupts<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN) {
        unsafe {
            match PIN::channel() {
                0 => self.adc.ier.write_with_zero(|w| w.eoc0().set_bit()),
                1 => self.adc.ier.write_with_zero(|w| w.eoc1().set_bit()),
                2 => self.adc.ier.write_with_zero(|w| w.eoc2().set_bit()),
                3 => self.adc.ier.write_with_zero(|w| w.eoc3().set_bit()),
                4 => self.adc.ier.write_with_zero(|w| w.eoc4().set_bit()),
                5 => self.adc.ier.write_with_zero(|w| w.eoc5().set_bit()),
                6 => self.adc.ier.write_with_zero(|w| w.eoc6().set_bit()),
                7 => self.adc.ier.write_with_zero(|w| w.eoc7().set_bit()),
                8 => self.adc.ier.write_with_zero(|w| w.eoc8().set_bit()),
                9 => self.adc.ier.write_with_zero(|w| w.eoc9().set_bit()),
                10 => self.adc.ier.write_with_zero(|w| w.eoc10().set_bit()),
                11 => self.adc.ier.write_with_zero(|w| w.eoc11().set_bit()),
                12 => self.adc.ier.write_with_zero(|w| w.eoc12().set_bit()),
                13 => self.adc.ier.write_with_zero(|w| w.eoc13().set_bit()),
                14 => self.adc.ier.write_with_zero(|w| w.eoc14().set_bit()),
                15 => self.adc.ier.write_with_zero(|w| w.eoc15().set_bit()),
                _ => {
                    panic!("Invalid channel: {}", PIN::channel());
                }
            }
        }
    }

    /// Disables the interrupts.
    fn disable_interrupts<PIN: Channel<ADC, ID = u8>>(&mut self, _pin: &mut PIN) {
        unsafe {
            match PIN::channel() {
                0 => self.adc.idr.write_with_zero(|w| w.eoc0().set_bit()),
                1 => self.adc.idr.write_with_zero(|w| w.eoc1().set_bit()),
                2 => self.adc.idr.write_with_zero(|w| w.eoc2().set_bit()),
                3 => self.adc.idr.write_with_zero(|w| w.eoc3().set_bit()),
                4 => self.adc.idr.write_with_zero(|w| w.eoc4().set_bit()),
                5 => self.adc.idr.write_with_zero(|w| w.eoc5().set_bit()),
                6 => self.adc.idr.write_with_zero(|w| w.eoc6().set_bit()),
                7 => self.adc.idr.write_with_zero(|w| w.eoc7().set_bit()),
                8 => self.adc.idr.write_with_zero(|w| w.eoc8().set_bit()),
                9 => self.adc.idr.write_with_zero(|w| w.eoc9().set_bit()),
                10 => self.adc.idr.write_with_zero(|w| w.eoc10().set_bit()),
                11 => self.adc.idr.write_with_zero(|w| w.eoc11().set_bit()),
                12 => self.adc.idr.write_with_zero(|w| w.eoc12().set_bit()),
                13 => self.adc.idr.write_with_zero(|w| w.eoc13().set_bit()),
                14 => self.adc.idr.write_with_zero(|w| w.eoc14().set_bit()),
                15 => self.adc.idr.write_with_zero(|w| w.eoc15().set_bit()),
                _ => {
                    panic!("Invalid channel: {}", PIN::channel());
                }
            }
        }
    }

    /// Checks for a ready value on the specified pin
    fn service_interrupt_ready<PIN: Channel<ADC, ID = u8>>(
        &mut self,
        _pin: &mut PIN,
    ) -> Option<u16> {
        // If interrupt is not enable for this channel, don't bother checking
        if !match PIN::channel() {
            0 => self.adc.imr.read().eoc0().bit(),
            1 => self.adc.imr.read().eoc1().bit(),
            2 => self.adc.imr.read().eoc2().bit(),
            3 => self.adc.imr.read().eoc3().bit(),
            4 => self.adc.imr.read().eoc4().bit(),
            5 => self.adc.imr.read().eoc5().bit(),
            6 => self.adc.imr.read().eoc6().bit(),
            7 => self.adc.imr.read().eoc7().bit(),
            8 => self.adc.imr.read().eoc8().bit(),
            9 => self.adc.imr.read().eoc9().bit(),
            10 => self.adc.imr.read().eoc10().bit(),
            11 => self.adc.imr.read().eoc11().bit(),
            12 => self.adc.imr.read().eoc12().bit(),
            13 => self.adc.imr.read().eoc13().bit(),
            14 => self.adc.imr.read().eoc14().bit(),
            15 => self.adc.imr.read().eoc15().bit(),
            _ => {
                panic!("Invalid channel: {}", PIN::channel());
            }
        } {
            return None;
        }

        // Check to see if the channel is ready before reading
        if match PIN::channel() {
            0 => self.adc.isr.read().eoc0().bit(),
            1 => self.adc.isr.read().eoc1().bit(),
            2 => self.adc.isr.read().eoc2().bit(),
            3 => self.adc.isr.read().eoc3().bit(),
            4 => self.adc.isr.read().eoc4().bit(),
            5 => self.adc.isr.read().eoc5().bit(),
            6 => self.adc.isr.read().eoc6().bit(),
            7 => self.adc.isr.read().eoc7().bit(),
            8 => self.adc.isr.read().eoc8().bit(),
            9 => self.adc.isr.read().eoc9().bit(),
            10 => self.adc.isr.read().eoc10().bit(),
            11 => self.adc.isr.read().eoc11().bit(),
            12 => self.adc.isr.read().eoc12().bit(),
            13 => self.adc.isr.read().eoc13().bit(),
            14 => self.adc.isr.read().eoc14().bit(),
            15 => self.adc.isr.read().eoc15().bit(),
            _ => {
                panic!("Invalid channel: {}", PIN::channel());
            }
        } {
            Some(self.adc.cdr[PIN::channel() as usize].read().data().bits())
        } else {
            None
        }
    }
}

impl ConversionMode for SingleConversion {
    fn on_start(_adc: &mut Adc) {}

    fn on_complete<PIN: Channel<ADC, ID = u8>>(adc: &mut Adc, pin: &mut PIN) {
        adc.disable_interrupts(pin);
    }

    fn on_stop<PIN: Channel<ADC, ID = u8>>(_adc: &mut Adc, _pin: &mut PIN) {}
}

impl ConversionMode for FreeRunning {
    fn on_start(adc: &mut Adc) {
        adc.enable_freerunning();
    }

    fn on_complete<PIN: Channel<ADC, ID = u8>>(_adc: &mut Adc, _pin: &mut PIN) {}

    fn on_stop<PIN: Channel<ADC, ID = u8>>(adc: &mut Adc, pin: &mut PIN) {
        adc.disable_interrupts(pin);
        adc.disable_freerunning();
    }
}

impl<C> InterruptAdc<C>
where
    C: ConversionMode,
{
    pub fn service_interrupt_ready<PIN: Channel<ADC, ID = u8>>(
        &mut self,
        pin: &mut PIN,
    ) -> Option<u16> {
        if let Some(res) = self.adc.service_interrupt_ready(pin) {
            C::on_complete(&mut self.adc, pin);
            Some(res)
        } else {
            None
        }
    }

    /// Starts conversion sampling on specified pin
    /// NOTE: You must enable the channel first before starting the conversion
    ///       If you do not start the conversion on pins that have been enabled
    ///       those pins will read ADC values but will not trigger an interrupt.
    pub fn start_conversion<PIN: Channel<ADC, ID = u8>>(&mut self, pin: &mut PIN) {
        C::on_start(&mut self.adc);
        self.adc.enable_interrupts(pin);
        self.adc.start_conversion();
    }

    pub fn stop_conversion<PIN: Channel<ADC, ID = u8>>(&mut self, pin: &mut PIN) {
        C::on_stop(&mut self.adc, pin);
    }

    /// Reverts the InterruptAdc back to Adc
    pub fn revert(self) -> Adc {
        self.adc
    }
}

impl<C> From<Adc> for InterruptAdc<C>
where
    C: ConversionMode,
{
    fn from(adc: Adc) -> Self {
        Self {
            adc,
            m: PhantomData {},
        }
    }
}

impl<WORD, PIN> OneShot<ADC, WORD, PIN> for Adc
where
    WORD: From<u16>,
    PIN: Channel<ADC, ID = u8>,
{
    type Error = ();

    fn read(&mut self, pin: &mut PIN) -> nb::Result<WORD, Self::Error> {
        // Trigger single shot
        let result = self.synchronous_convert(pin);
        Ok(result.into())
    }
}

macro_rules! adc_pins {
    (
        $(
            $PinId:ident: ($CHAN:literal),
        )+
    ) => {
        $(
            impl Channel<ADC> for $PinId<ExFn> {
               type ID = u8;
               fn channel() -> u8 { $CHAN }
            }
        )+
    }
}

#[cfg(feature = "atsam4s")]
adc_pins! {
    Pa17: (0),
    Pa18: (1),
    Pa19: (2),
    Pa20: (3),
    Pb0: (4),
    Pb1: (5),
    Pb2: (6),
    Pb3: (7),
}

#[cfg(any(feature = "atsam4s_b", feature = "atsam4s_c"))]
adc_pins! {
    Pa21: (8),
    Pa22: (9),
}

#[cfg(feature = "atsam4s_c")]
adc_pins! {
    Pc13: (10),
    Pc15: (11),
    Pc12: (12),
    Pc29: (13),
    Pc30: (14),
}

/// Channel 15 is reserved for the temperature sensor
#[cfg(feature = "atsam4s")]
pub struct TempSensor {}

#[cfg(feature = "atsam4s")]
impl Channel<ADC> for TempSensor {
    type ID = u8;
    fn channel() -> u8 {
        15
    }
}

// Setup PDC Rx functionality
pdc_rx! { Adc: adc, isr }

impl Adc {
    /// Configures the ADC with PDC in single sequence mode
    /// This will take a single sample of all the enable channels.
    /// If sequence() has been set, then only the channels in the sequence will
    /// be read.
    ///
    /// Since this is a single capture, the buffer used must be equal to the number
    /// of channels going to be read (a smaller buffer would also work, but doesn't really
    /// make sense). If the buffer is too large, the DMA transfer will not complete
    /// and wait for the next ADC conversion event.
    ///
    /// Polling example
    /// ```
    /// let mut adc = Adc::new(
    ///     cx.device.ADC,
    ///     clocks.peripheral_clocks.adc.into_enabled_clock(),
    /// );
    ///
    /// // Enable 3 channels
    /// adc.enable_channel(&mut pins.sense1);
    /// adc.enable_channel(&mut pins.sense2);
    /// adc.enable_channel(&mut pins.sense3);
    ///
    /// // Enable DMA mode
    /// let adc = adc.with_pdc();
    ///
    /// // Note that the buffer size is 3 and 3 channels were enabled
    /// let buf = singleton!(: [u16; 4] = [0; 4]).unwrap();
    ///
    /// // Read and wait for the result
    /// let (buf, adc) = adc.read(buf).wait();
    /// defmt::trace!("DMA BUF: {}", buf);
    ///
    /// // Revert AdcDma<SingleSequence> back to Adc
    /// let adc = adc.revert();
    /// ```
    ///
    /// Interrupt example (RTIC)
    /// ```
    /// #[local]
    /// struct Local {
    ///     adc: Option<Transfer<W, &'static mut [u16; 6], RxDma<AdcPayload<SingleSequence>>>>,
    /// }
    ///
    /// #[init(local = [adc_buf: [u16; 3] = [0; 3]])]
    /// fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
    ///     let mut adc = Adc::new(
    ///         cx.device.ADC,
    ///         clocks.peripheral_clocks.adc.into_enabled_clock(),
    ///     );
    ///
    ///     // Enable 3 channels
    ///     adc.enable_channel(&mut pins.sense1);
    ///     adc.enable_channel(&mut pins.sense2);
    ///     adc.enable_channel(&mut pins.sense3);
    ///
    ///     // Enable DMA mode
    ///     let adc = adc.with_pdc();
    ///     (
    ///         Shared {},
    ///         Local {
    ///             adc: Some(adc.read(cx.local.adc_buf)),
    ///         },
    ///         init::Monotonics {},
    ///     )
    /// }
    ///
    /// #[task(binds = ADC, local = [adc])]
    /// fn adc(cx: adc::Context) {
    ///     let (buf, adc) = cx.local.adc.take().unwrap().wait();
    ///     defmt::trace!("DMA BUF: {}", buf);
    ///     cx.local.adc.replace(adc.read(buf));
    /// }
    /// ```
    pub fn with_pdc(self) -> AdcDma<SingleSequence> {
        let payload = AdcPayload {
            adc: self,
            _mode: PhantomData,
        };
        RxDma { payload }
    }

    /// Configures the ADC with PDC in single sequence mode
    /// This will take a single sample of all the enable channels.
    /// If sequence() has been set, then only the channels in the sequence will
    /// be read.
    ///
    /// Since this is a continous capture (e.g. freerunning), the buffer size may be any
    /// size and the DMA transfer will complete once the buffer is full.
    /// It is recommended to make the buffer a multiple of the number of channels enabled
    /// (or number of channels used in the sequence).
    ///
    /// Polling example
    /// ```
    /// let mut adc = Adc::new(
    ///     cx.device.ADC,
    ///     clocks.peripheral_clocks.adc.into_enabled_clock(),
    /// );
    ///
    /// // Enable 3 channels
    /// adc.enable_channel(&mut pins.sense1);
    /// adc.enable_channel(&mut pins.sense2);
    /// adc.enable_channel(&mut pins.sense3);
    ///
    /// // Reorder sequence (by default, the channel numbers dictate the order)
    /// adc.sequence(&mut [2, 1, 0]);
    /// adc.enable_sequencing();
    ///
    /// // Enable DMA mode
    /// let adc = adc.with_continuous_pdc();
    ///
    /// // Note that the buffer size is 6 and 3 channels were enabled
    /// // Since we're running in freerunning mode the entire will fill up before
    /// // the PDC notes completion.
    /// let buf = singleton!(: [u16; 6] = [0; 6]).unwrap();
    ///
    /// // Read and wait for the result
    /// let (buf, adc) = adc.read(buf).wait();
    /// defmt::trace!("DMA BUF: {}", buf);
    ///
    /// // Revert AdcDma<Continuous> back to Adc
    /// let adc = adc.revert();
    /// ```
    ///
    /// Interrupt example (RTIC)
    /// ```
    /// #[local]
    /// struct Local {
    ///     adc: Option<Transfer<W, &'static mut [u16; 6], RxDma<AdcPayload<Continuous>>>>,
    /// }
    ///
    /// #[init(local = [adc_buf: [u16; 6] = [0; 6]])]
    /// fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
    ///     let mut adc = Adc::new(
    ///         cx.device.ADC,
    ///         clocks.peripheral_clocks.adc.into_enabled_clock(),
    ///     );
    ///
    ///     // Enable 3 channels
    ///     adc.enable_channel(&mut pins.sense1);
    ///     adc.enable_channel(&mut pins.sense2);
    ///     adc.enable_channel(&mut pins.sense3);
    ///
    ///     // Reorder sequence (by default, the channel numbers dictate the order)
    ///     adc.sequence(&mut [2, 1, 0]);
    ///     adc.enable_sequencing();
    ///
    ///     // Enable DMA mode
    ///     let adc = adc.with_continuous_pdc();
    ///     (
    ///         Shared {},
    ///         Local {
    ///             adc: Some(adc.read(cx.local.adc_buf)),
    ///         },
    ///         init::Monotonics {},
    ///     )
    /// }
    ///
    /// #[task(binds = ADC, local = [adc])]
    /// fn adc(cx: adc::Context) {
    ///     let (buf, adc) = cx.local.adc.take().unwrap().wait();
    ///     defmt::trace!("DMA BUF: {}", buf);
    ///     cx.local.adc.replace(adc.read(buf));
    /// }
    /// ```
    pub fn with_continuous_pdc(self) -> AdcDma<Continuous> {
        let payload = AdcPayload {
            adc: self,
            _mode: PhantomData,
        };
        RxDma { payload }
    }
}

/// Continuous mode
pub struct Continuous;
/// SingleSequence mode
pub struct SingleSequence;

pub struct AdcPayload<MODE> {
    adc: Adc,
    _mode: PhantomData<MODE>,
}

pub type AdcDma<MODE> = RxDma<AdcPayload<MODE>>;

impl<MODE> AdcDma<MODE>
where
    Self: TransferPayload,
{
    /// Reverts the AdcDma back to Adc
    pub fn revert(mut self) -> Adc {
        // Disable PDC in case it is still enabled
        self.payload.adc.stop_rx_pdc();

        self.payload.adc
    }
}

impl<MODE> Receive for AdcDma<MODE> {
    type TransmittedWord = u16;
}

impl<B, MODE> ReadDma<B, u16> for AdcDma<MODE>
where
    Self: TransferPayload,
    B: WriteBuffer<Word = u16>,
{
    /// Assigns the buffer, enables PDC and starts ADC conversion
    fn read(mut self, mut buffer: B) -> Transfer<W, B, Self> {
        // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
        // until the end of the transfer.
        let (ptr, len) = unsafe { buffer.write_buffer() };
        self.payload.adc.set_receive_address(ptr as u32);
        self.payload.adc.set_receive_counter(len as u16);

        compiler_fence(Ordering::Release);
        self.start();

        Transfer::w(buffer, self)
    }
}

impl<B, MODE> ReadDmaPaused<B, u16> for AdcDma<MODE>
where
    Self: TransferPayload,
    B: WriteBuffer<Word = u16>,
{
    /// Assigns the buffer, prepares PDC but does not enable the PDC
    /// Useful when there is strict timing on when the ADC conversion should start
    ///
    /// transfer.resume() can be used to start the transfer
    fn read_paused(mut self, mut buffer: B) -> Transfer<W, B, Self> {
        // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
        // until the end of the transfer.
        let (ptr, len) = unsafe { buffer.write_buffer() };
        self.payload.adc.set_receive_address(ptr as u32);
        self.payload.adc.set_receive_counter(len as u16);

        compiler_fence(Ordering::Release);

        Transfer::w(buffer, self)
    }
}

impl TransferPayload for AdcDma<SingleSequence> {
    fn start(&mut self) {
        self.payload.adc.start_rx_pdc();
        self.payload.adc.start_conversion(); // Start ADC conversions
    }
    fn stop(&mut self) {
        self.payload.adc.stop_rx_pdc();
    }
    fn in_progress(&self) -> bool {
        self.payload.adc.rx_in_progress()
    }
}

impl TransferPayload for AdcDma<Continuous> {
    fn start(&mut self) {
        self.payload.adc.start_rx_pdc();
        self.payload.adc.enable_freerunning();
        self.payload.adc.start_conversion(); // Start ADC conversions
    }
    fn stop(&mut self) {
        self.payload.adc.disable_freerunning();
        self.payload.adc.stop_rx_pdc();
    }
    fn in_progress(&self) -> bool {
        self.payload.adc.rx_in_progress()
    }
}
