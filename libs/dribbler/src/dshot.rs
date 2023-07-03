//! # Unidirectional DSHOT
//! ## DSHOT is simple.
//!
//! Bits are encoded using pulse length:
//! ```text
//!  ____           _________
//! |    |_________|         |____|
//! |      0       |      1       |
//! ```
//! The pulse of a 1 is double the length of the pulse of a 0. The protocoll can be though of as a bit stream where every bit
//! flanked by a start bit of 1 and an stop bit of 0. The frequency at which bits are transmittet
//! is given by the DSHOT Version in kHz. The stop bit is schorter than the start and data bit. The
//! ratio is 3/2 (start bit / stop bit).
//!
//! A DSHOT frame consists of 3 parts:
//! `vvvv_vvvv_vvvt_ccc`
//! v is the command value send to the ESC.
//! t is a flag telling the ESC to send telemetry over a seperate telemetry wire.
//! c is a checksum over v and t.
//!
//! The value of v gives the throttle for the ESC but the values 1..=47 are maped to special
//! commands. The value 0 is used for disarm. Values in 48..=2047 give the throttle value from 0..2000.
//! The checksum is calculated by calculating the xor of the nibbles `vvvv_vvvv_vvvt`.
//!
//! # Bidirectional DSHOT
//! ## DSHOT is hard.
//!
//! Bits are still encoded using pulse length:
//! ```text
//!       _________           ____
//! |____|         |_________|    |
//! |      0       |      1       |
//! ```
//! The signal is inverted on the line compared to Unidirectional DSHOT. This tells the ESC that
//! Bidirectional DSHOT is used. The frame of Bidirectional DSHOT is the same as the Unidirectional
//! frame.
//!
//! The checksum is also calculating by creating the xor of the nibbles but is then inverted.
//!
//! The frame send back by the ESC encodes the ePeriod in microseconds:
//! `eeem_mmmm_mmmm_cccc`
//! e is the exponent indicating the amount the mantissa needs to be shifted to the left.
//! m is the mantissa.
//! c is the checksum calculated like the checksum of Unidirectional DSHOT.
//!
//! Each nibble is encoded using [GCR encoding][1]:
//!
//! | 0x0  | 0x1  | 0x2  | 0x3  | 0x4  | 0x5  | 0x6  | 0x7  | 0x8  | 0x9  | 0xA  | 0xB  | 0xC  | 0xD  | 0xE  | 0xF  |
//! |------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|
//! | 0x19 | 0x1B | 0x12 | 0x13 | 0x1D | 0x15 | 0x16 | 0x17 | 0x1A | 0x09 | 0x0A | 0x0B | 0x1E | 0x0D | 0x0E | 0x0F |
//!
//! after the transformation there are 20 bits. A 21th bit = 0 is added and the bits are encoded by
//! the change from one bit to the next. So if the current bit is 1 the bit on the line is the
//! inverse of the last bit on the line. If the current bit is 0 the bit on the line is the same As
//! the last bit on the line.
//!
//! This bit sequence is encoded on the line using high and low pulses:
//! ```text
//!             __________
//! |__________|          |
//! |     0    |     1    |
//! ```
//! The length of each bit is 4/5 of the length of a bit when sending to the ESC. This is done to
//! transmit the feedback in the same time it takes to transmit the command.
//!
//! This information is mostly taken from [a nice brushlesswhoop.com blog][2]
//!
//! [1]: https://en.wikipedia.org/wiki/Run-length_limited#GCR:_(0,2)_RLL
//! [2]: https://brushlesswhoop.com/dshot-and-bidirectional-dshot/

use core::marker::PhantomData;

use crate::Dribbler;

use defmt::{trace, warn, Format};
use fixed::{types::extra::U2, FixedU8};
use fugit::{HertzU32, Rate, RateExtU32, RateExtU64};
use pio::{Assembler, InSource, JmpCondition, OutDestination, SetDestination, SideSet, WaitSource};
use rp2040_hal::{
    gpio::{Function, FunctionConfig, Pin, PinId, ValidPinMode},
    pio::{
        Buffers, InstalledProgram, PIOBuilder, PIOExt, PinDir, PinState, Running, Rx,
        ShiftDirection, StateMachine, StateMachineIndex, Tx, UninitStateMachine, PIO,
    },
};
use units::{
    prelude::*,
    types::{Ampere, Volt},
};

#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Format, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum DshotCommand {
    /// Disarm the motor. Can only be send when stopped.
    Disarm = 0,
    /// Beep using the motor. Can only be send when stopped. Wait at least 260ms before next
    /// command.
    Beep1 = 1,
    /// Beep using the motor. Can only be send when stopped. Wait at least 260ms before next
    /// command.
    Beep2 = 2,
    /// Beep using the motor. Can only be send when stopped. Wait at least 260ms before next
    /// command.
    Beep3 = 3,
    /// Beep using the motor. Can only be send when stopped. Wait at least 260ms before next
    /// command.
    Beep4 = 4,
    /// Beep using the motor. Can only be send when stopped. Wait at least 260ms before next
    /// command.
    Beep5 = 5,
    /// Request ESC info. Can only be send when stopped. Wait at least 12ms before next command.
    EscInfo = 6,
    /// Set spin direction to direction 1. Can only be send when stopped. Needs to be send 6 times.
    SpinDirection1 = 7,
    /// Set spin direction to direction 2. Can only be send when stopped. Needs to be send 6 times.
    SpinDirection2 = 8,
    /// Disable 3D mode (spinning in both directions). Can only be send when stopped. Needs to be
    /// send 6 times.
    Disable3DMode = 9,
    /// Enable 3D mode (spinning in both directions). Can only be send when stopped. Needs to be
    /// send 6 times.
    Enable3DMode = 10,
    /// Request current ESC settings. Can only be send when stopped.
    SettingsRequest = 11,
    /// Save the current ESC settings. Can only be send when stopped. Needs to be send 6 times.
    /// Wait at least 35ms before next command.
    SaveSettings = 12,
    /// Enable extended telemetry. Can only be send when stopped. Needs to be send 6 times.
    EnableExtendedTelemetry = 13,
    /// Disable extended telemetry. Can only be send when stopped. Needs to be send 6 times.
    DisableExtendedTelemetry = 14,
    /// Set spin direction to Normal. Can only be send when stopped. Needs to be send 6 times.
    SpinDirectionNormal = 20,
    /// Set spin direction to Reverse. Can only be send when stopped. Needs to be send 6 times.
    SpinDirectionReversed = 21,
    /// Turn Led 0 on. Can only be send when stopped.
    Led0On = 22,
    /// Turn Led 1 on. Can only be send when stopped.
    Led1On = 23,
    /// Turn Led 2 on. Can only be send when stopped.
    Led2On = 24,
    /// Turn Led 3 on. Can only be send when stopped.
    Led3On = 25,
    /// Turn Led 0 off. Can only be send when stopped.
    Led0Off = 26,
    /// Turn Led 1 off. Can only be send when stopped.
    Led1Off = 27,
    /// Turn Led 2 off. Can only be send when stopped.
    Led2Off = 28,
    /// Turn Led 3 off. Can only be send when stopped.
    Led3Off = 29,
    /// Toggle audio stream mode. Can only be send when stopped.
    AudioStreamModeToggle = 30,
    /// Toggle silent mode. Can only be send when stopped.
    SilentModeToggle = 31,
    /// Disables commands 42..=47. Can only be send when stopped. Needs to be send 6 times.
    DisableSignalLineTelemetry = 32,
    /// Enables commands 42..=47. Can only be send when stopped. Needs to be send 6 times.
    EnableSignalLineTelemetry = 33,
    /// Enables commands 42..=47 and sends erpm if normal DSHOT frame. Can only be send when stopped. Needs to be send 6 times.
    SignalLineContinuousErpmTelemetry = 34,
    /// Enables commands 42..=47 and sends erpm period if normal DSHOT frame. Can only be send when stopped. Needs to be send 6 times.
    SignalLineContinuousErpmPeriodTelemetry = 35,
    /// Send temperature telemetry. 1°C per LSB.
    SignalLineTemperatueTelemetry = 42,
    /// Send voltage telemetry. 10mV per LSB.
    SignalLineVoltageTelemetry = 43,
    /// Send current telemetry. 100mA per LSB.
    SignalLineCurrentTelemetry = 44,
    /// Send consumption telemetry. 10mAh per LSB.
    SignalLineConsumptionTelemetry = 45,
    /// Send erpm telemetry. 100erpm per LSB.
    SignalLineErpmTelemetry = 46,
    /// Send erpm period telemetry. 16us per LSB.
    SignalLineErpmPeriodTelemetry = 47,
    /// Throttle value in 0..2000.
    Throttle(u16) = 48,
}

impl From<DshotCommand> for u16 {
    fn from(value: DshotCommand) -> Self {
        match value {
            DshotCommand::Throttle(v) => {
                if (0..2000).contains(&v) {
                    v + 48
                } else {
                    unreachable!()
                }
            }
            DshotCommand::Disarm => 0,
            DshotCommand::Beep1 => 1,
            DshotCommand::Beep2 => 2,
            DshotCommand::Beep3 => 3,
            DshotCommand::Beep4 => 4,
            DshotCommand::Beep5 => 5,
            DshotCommand::EscInfo => 6,
            DshotCommand::SpinDirection1 => 7,
            DshotCommand::SpinDirection2 => 8,
            DshotCommand::Disable3DMode => 9,
            DshotCommand::Enable3DMode => 10,
            DshotCommand::SettingsRequest => 11,
            DshotCommand::SaveSettings => 12,
            DshotCommand::EnableExtendedTelemetry => 13,
            DshotCommand::DisableExtendedTelemetry => 14,
            DshotCommand::SpinDirectionNormal => 20,
            DshotCommand::SpinDirectionReversed => 21,
            DshotCommand::Led0On => 22,
            DshotCommand::Led1On => 23,
            DshotCommand::Led2On => 24,
            DshotCommand::Led3On => 25,
            DshotCommand::Led0Off => 26,
            DshotCommand::Led1Off => 27,
            DshotCommand::Led2Off => 28,
            DshotCommand::Led3Off => 29,
            DshotCommand::AudioStreamModeToggle => 30,
            DshotCommand::SilentModeToggle => 31,
            DshotCommand::DisableSignalLineTelemetry => 32,
            DshotCommand::EnableSignalLineTelemetry => 33,
            DshotCommand::SignalLineContinuousErpmTelemetry => 34,
            DshotCommand::SignalLineContinuousErpmPeriodTelemetry => 35,
            DshotCommand::SignalLineTemperatueTelemetry => 42,
            DshotCommand::SignalLineVoltageTelemetry => 43,
            DshotCommand::SignalLineCurrentTelemetry => 44,
            DshotCommand::SignalLineConsumptionTelemetry => 45,
            DshotCommand::SignalLineErpmTelemetry => 46,
            DshotCommand::SignalLineErpmPeriodTelemetry => 47,
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct DshotFrame {
    command: DshotCommand,
    telemetry: bool,
}

impl From<DshotFrame> for u16 {
    fn from(value: DshotFrame) -> Self {
        let value = Self::from(value.command);
        let value = if value < 48 && value != 0 {
            (value << 5) | (1 << 4)
        } else {
            value << 5
        };
        // xor bytes
        let csum = (value >> 8) ^ value;
        // xor nibbles
        let csum = ((csum >> 4) ^ csum) & 0xF;
        value | csum
    }
}

#[derive(Debug, Format, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Version {
    Dshot150,
    Dshot300,
    Dshot600,
    Dshot1200,
}

impl<const NOM: u32, const DENOM: u32> From<Version> for Rate<u32, NOM, DENOM> {
    fn from(value: Version) -> Self {
        match value {
            Version::Dshot150 => 150u32.kHz(),
            Version::Dshot300 => 300u32.kHz(),
            Version::Dshot600 => 600u32.kHz(),
            Version::Dshot1200 => 1200u32.kHz(),
        }
    }
}

impl<const NOM: u32, const DENOM: u32> From<Version> for Rate<u64, NOM, DENOM> {
    fn from(value: Version) -> Self {
        match value {
            Version::Dshot150 => 150u64.kHz(),
            Version::Dshot300 => 300u64.kHz(),
            Version::Dshot600 => 600u64.kHz(),
            Version::Dshot1200 => 1200u64.kHz(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Telemetry {
    Erpm(u16),
    Temperature(u8),
    Voltage(Volt<FixedU8<U2>>),
    Current(Ampere<u8>),
    Debug1(u8),
    Debug2(u8),
    Debug3(u8),
    StateEvent(u8),
}

/// Marker trait for valid DSHOT modes
pub trait ValidMode {}

/// Marker for Unidirectional DSHOT
#[derive(Debug, Clone, Copy)]
pub struct UnidirectionalMode;
/// Marker for Bidirectional DSHOT
#[derive(Debug, Clone, Copy)]
pub struct BidirectionalMode;

impl ValidMode for UnidirectionalMode {}
impl ValidMode for BidirectionalMode {}

/// Marker trait for valid DSHOT versions
pub trait ValidVersion {
    fn freq() -> HertzU32;
}
/// Marker trait for valid Bidirectional DSHOT versions
pub trait ValidBidirVersion: ValidVersion {}

/// Marker for DSHOT 150
#[derive(Debug, Clone, Copy)]
pub struct Version150;
/// Marker for DSHOT 300
#[derive(Debug, Clone, Copy)]
pub struct Version300;
/// Marker for DSHOT 600
#[derive(Debug, Clone, Copy)]
pub struct Version600;
/// Marker for DSHOT 1200
#[derive(Debug, Clone, Copy)]
pub struct Version1200;

impl ValidVersion for Version150 {
    fn freq() -> HertzU32 {
        150u32.kHz()
    }
}
impl ValidVersion for Version300 {
    fn freq() -> HertzU32 {
        300u32.kHz()
    }
}
impl ValidVersion for Version600 {
    fn freq() -> HertzU32 {
        600u32.kHz()
    }
}
impl ValidVersion for Version1200 {
    fn freq() -> HertzU32 {
        1200u32.kHz()
    }
}
impl ValidBidirVersion for Version300 {}
impl ValidBidirVersion for Version600 {}
impl ValidBidirVersion for Version1200 {}

/// Decodes the 5 LSBs from value using gcr
const fn decode_gcr(value: u32) -> Option<u8> {
    match value & 0x1F {
        0x19 => Some(0x0),
        0x1B => Some(0x1),
        0x12 => Some(0x2),
        0x13 => Some(0x3),
        0x1D => Some(0x4),
        0x15 => Some(0x5),
        0x16 => Some(0x6),
        0x17 => Some(0x7),
        0x1A => Some(0x8),
        0x09 => Some(0x9),
        0x0A => Some(0xA),
        0x0B => Some(0xB),
        0x1E => Some(0xC),
        0x0D => Some(0xD),
        0x0E => Some(0xE),
        0x0F => Some(0xF),
        _ => None,
    }
}

fn decode_telemetry_bits(bits: u32) -> Option<u16> {
    let bits = bits ^ bits >> 1;
    let value = u16::from(decode_gcr(bits)?)
        | u16::from(decode_gcr(bits >> 5)?) << 4
        | u16::from(decode_gcr(bits >> 10)?) << 8
        | u16::from(decode_gcr(bits >> 15)?) << 12;
    let csum = (value >> 8) ^ value;
    let csum = ((csum >> 4) ^ csum) & 0xF;
    if csum != 0xF {
        warn!("checksum failed. got {}", csum);
        return None;
    }
    Some(value >> 4)
}

/// Decodes a non extended telemetry frame. `bits` contains the 21 received bits.
fn decode_telemetry(bits: u32) -> Option<u16> {
    let value = decode_telemetry_bits(bits)?;
    Some((value & 0x1FF) << ((value & 0xE00) >> 9))
}

#[allow(dead_code)]
fn decode_extended_telemetry(bits: u32) -> Option<Telemetry> {
    let value = decode_telemetry_bits(bits)?;
    match value & 0xf00 {
        0x200 => Some(Telemetry::Temperature(
            (value & 0x0ff).try_into().expect("range is checked"),
        )),
        0x400 => Some(Telemetry::Voltage(
            FixedU8::from_bits((value & 0x0ff).try_into().expect("range is checked")).V(),
        )),
        0x600 => Some(Telemetry::Current(
            TryInto::<u8>::try_into(value & 0x0ff)
                .expect("range is checked")
                .A(),
        )),
        0x800 => Some(Telemetry::Debug1(
            (value & 0x0ff).try_into().expect("range is checked"),
        )),
        0xA00 => Some(Telemetry::Debug2(
            (value & 0x0ff).try_into().expect("range is checked"),
        )),
        0xC00 => Some(Telemetry::Debug3(
            (value & 0x0ff).try_into().expect("range is checked"),
        )),
        0xE00 => Some(Telemetry::StateEvent(
            (value & 0x0ff).try_into().expect("range is checked"),
        )),
        _ => Some(Telemetry::Erpm((value & 0x1FF) << ((value & 0xE00) >> 9))),
    }
}

pub struct Dshot<P, S, I, M, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    M: ValidMode,
    V: ValidVersion,
{
    sm: StateMachine<(P, S), Running>,
    tx: Tx<(P, S)>,
    rx: Rx<(P, S)>,
    unused_program: InstalledProgram<P>,
    clock_freq: HertzU32,
    last_erpm: u16,
    last_temp: u8,
    last_volt: Volt<FixedU8<U2>>,
    last_curr: Ampere<u8>,
    pin: Pin<I, Function<P>>,
    mode: PhantomData<M>,
    version: PhantomData<V>,
}

impl<P, S, I, V> Dshot<P, S, I, UnidirectionalMode, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    V: ValidVersion,
{
    /// Creates a new [`Dshot<P, S, I, UnidirectionalMode, V>`].
    ///
    /// # Panics
    ///
    /// Panics if no divider config can be found to support the selected DSHOT version
    #[allow(clippy::similar_names)]
    pub fn new(
        pio: &mut PIO<P>,
        sm: UninitStateMachine<(P, S)>,
        pin: Pin<I, Function<P>>,
        clock_freq: HertzU32,
    ) -> Self {
        const DIVIDER_MAX: u32 = u16::MAX as u32 + 1;

        // first calculate divider to fail fast in case it is not possible to set it correctly
        let freq = V::freq();
        let bit_freq = freq * 8;
        let int = clock_freq / bit_freq;
        let rem = clock_freq - (int * bit_freq);
        let frac = (rem * 256) / bit_freq;
        assert!(
            (1..=DIVIDER_MAX).contains(&int) && (int != DIVIDER_MAX || frac == 0),
            "({}kHz / {}kHz) must be within 1.0..={} but is ({} + {} / 256)",
            clock_freq.to_kHz(),
            bit_freq.to_kHz(),
            DIVIDER_MAX,
            int,
            frac
        );
        let int = if int == DIVIDER_MAX { 0 } else { int };
        let int = u16::try_from(int).expect("range was checked above");
        let frac = u8::try_from(frac).expect("range was checked above");

        let unidir_installed = install_unidir_pio_program(pio);

        let bidir_installed = install_bidir_pio_program(pio, I::DYN.num);

        let (mut sm, rx, tx) = PIOBuilder::from_program(unidir_installed)
            .buffers(Buffers::OnlyTx)
            .side_set_pin_base(I::DYN.num)
            .out_shift_direction(ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(16)
            .clock_divisor_fixed_point(int, frac)
            .build(sm);
        sm.set_pindirs([(I::DYN.num, PinDir::Output)]);
        let sm = sm.start();

        Self {
            sm,
            tx,
            rx,
            unused_program: bidir_installed,
            clock_freq,
            last_erpm: 0,
            last_temp: 0,
            last_volt: FixedU8::<U2>::ZERO.V(),
            last_curr: 0.A(),
            pin,
            mode: PhantomData,
            version: PhantomData,
        }
    }
}

fn install_unidir_pio_program<P>(pio: &mut PIO<P>) -> InstalledProgram<P>
where
    P: PIOExt,
{
    const T_START: u8 = 3;
    const T_MID: u8 = 3;
    const T_STOP: u8 = 2;
    const HIGH: u8 = 1;
    const LOW: u8 = 0;

    // assemble the unidirectional program
    let side_set = SideSet::new(false, 1, false);
    let mut a = Assembler::new_with_side_set(side_set);
    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    let mut do_zero = a.label();
    // Do stop bit
    a.bind(&mut wrap_target);
    a.out_with_delay_and_side_set(OutDestination::X, 1, T_STOP - 1, LOW);
    // Do start bit
    a.jmp_with_delay_and_side_set(JmpCondition::XIsZero, &mut do_zero, T_START - 1, HIGH);
    // Do data bit = 1
    a.jmp_with_delay_and_side_set(JmpCondition::Always, &mut wrap_target, T_MID - 1, HIGH);
    // Do data bit = 0
    a.bind(&mut do_zero);
    a.nop_with_delay_and_side_set(T_MID - 1, LOW);
    a.bind(&mut wrap_source);
    let program = a.assemble_with_wrap(wrap_source, wrap_target);
    pio.install(&program).unwrap()
}

fn install_bidir_pio_program<P>(pio: &mut PIO<P>, pin: u8) -> InstalledProgram<P>
where
    P: PIOExt,
{
    const T_START: u8 = 3 * 5;
    const T_MID: u8 = 3 * 5;
    const T_STOP: u8 = 2 * 5;
    const CYCLES_PER_BIT: u8 = T_START + T_MID + T_STOP;
    const T_RECV: u8 = CYCLES_PER_BIT * 2 / 5;
    const HIGH: u8 = 0;
    const LOW: u8 = 1;

    // assemble the bidirectional program
    let side_set = SideSet::new(false, 1, false);
    let mut a = Assembler::new_with_side_set(side_set);
    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    let mut do_zero = a.label();
    let mut do_start = a.label();
    let mut do_stop = a.label();
    let mut receive = a.label();
    // Set pin direction to output
    a.bind(&mut wrap_target);
    a.set_with_side_set(SetDestination::PINDIRS, 1, LOW);
    // Create 16 bit bit counter
    a.set_with_side_set(SetDestination::Y, 15, LOW);
    // Read bit into x
    a.bind(&mut do_start);
    a.out_with_side_set(OutDestination::X, 1, LOW);
    // Do start bit
    a.jmp_with_delay_and_side_set(JmpCondition::XIsZero, &mut do_zero, T_START - 1, HIGH);
    // Do data bit = 1
    a.jmp_with_delay_and_side_set(JmpCondition::Always, &mut do_stop, T_MID - 1, HIGH);
    // Do data bit = 0
    a.bind(&mut do_zero);
    a.nop_with_delay_and_side_set(T_MID - 1, LOW);
    // Do stop bit and start receiving when all bits are send
    a.bind(&mut do_stop);
    a.jmp_with_delay_and_side_set(JmpCondition::YDecNonZero, &mut do_start, T_STOP - 2, LOW);
    // Create 21 bit bit counter
    a.set_with_side_set(SetDestination::Y, 20, LOW);
    // Set pin directions to input
    a.set_with_side_set(SetDestination::PINDIRS, 0, LOW);
    // Wait for the first bit and wait half a bit to be in the center
    a.wait_with_delay_and_side_set(HIGH, WaitSource::GPIO, pin, false, T_RECV - 1, LOW);
    // Read the bit and delay for half a bit
    a.bind(&mut receive);
    a.in_with_delay_and_side_set(InSource::PINS, 1, T_RECV - 1, LOW);
    // Receive next bit if not all bits where received yet. Delay for another half bit
    a.jmp_with_delay_and_side_set(JmpCondition::YDecNonZero, &mut receive, T_RECV - 1, LOW);
    a.bind(&mut wrap_source);
    let program = a.assemble_with_wrap(wrap_source, wrap_target);
    pio.install(&program).unwrap()
}

impl<P, S, I, M, V> Dshot<P, S, I, M, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    M: ValidMode,
    V: ValidBidirVersion,
    Self: ValidController,
{
    fn change_mode<M2: ValidMode>(
        mut self,
        command: DshotCommand,
        bit_cycles: u32,
        buffers: Buffers,
        initial_state: PinState,
    ) -> Dshot<P, S, I, M2, V> {
        const DIVIDER_MAX: u32 = u16::MAX as u32 + 1;

        // send enble telemetry command 10 times
        for _ in 0..10 {
            self.send_command(command);
        }
        // wait for the sm to finish the commands
        while !self.tx.has_stalled() {}

        let (sm, old_program) = self.sm.uninit(self.rx, self.tx);

        // first calculate divider to fail fast in case it is not possible to set it correctly
        let freq = V::freq();
        let bit_freq = freq * bit_cycles;
        let int = self.clock_freq / bit_freq;
        let rem = self.clock_freq - (int * bit_freq);
        let frac = (rem * 256) / bit_freq;
        assert!(
            (1..=DIVIDER_MAX).contains(&int) && (int != DIVIDER_MAX || frac == 0),
            "({}kHz / {}kHz) must be within 1.0..={} but is ({} + {} / 256)",
            self.clock_freq.to_kHz(),
            bit_freq.to_kHz(),
            DIVIDER_MAX,
            int,
            frac
        );
        let int = if int == DIVIDER_MAX { 0 } else { int };
        let int = u16::try_from(int).expect("range was checked above");
        let frac = u8::try_from(frac).expect("range was checked above");

        let (mut sm, rx, tx) = PIOBuilder::from_program(self.unused_program)
            .buffers(buffers)
            .side_set_pin_base(I::DYN.num)
            .set_pins(I::DYN.num, 1)
            .in_pin_base(I::DYN.num)
            .out_shift_direction(ShiftDirection::Left)
            .in_shift_direction(ShiftDirection::Left)
            .autopull(true)
            .autopush(true)
            .pull_threshold(16)
            .push_threshold(21)
            .clock_divisor_fixed_point(int, frac)
            .build(sm);

        sm.set_pindirs([(I::DYN.num, PinDir::Output)]);
        sm.set_pins([(I::DYN.num, initial_state)]);
        let sm = sm.start();

        Dshot {
            sm,
            tx,
            rx,
            unused_program: old_program,
            clock_freq: self.clock_freq,
            last_erpm: self.last_erpm,
            last_temp: self.last_temp,
            last_volt: self.last_volt,
            last_curr: self.last_curr,
            pin: self.pin,
            mode: PhantomData,
            version: PhantomData,
        }
    }
}

impl<P, S, I, V> Dshot<P, S, I, UnidirectionalMode, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    V: ValidBidirVersion,
{
    /// enables bidirectional dshot telemetry
    ///
    /// # Panics
    ///
    /// Panics if no divider for the speed can be found
    #[must_use]
    pub fn enable_telemetry(self) -> Dshot<P, S, I, BidirectionalMode, V> {
        self.change_mode(
            DshotCommand::EnableSignalLineTelemetry,
            8 * 5,
            Buffers::RxTx,
            PinState::High,
        )
    }
}

impl<P, S, I, V> Dshot<P, S, I, BidirectionalMode, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    V: ValidBidirVersion,
{
    /// disables bidirectional dshot telemetry
    ///
    /// # Panics
    ///
    /// Panics if no divider for the speed can be found
    #[must_use]
    pub fn disable_telemetry(self) -> Dshot<P, S, I, UnidirectionalMode, V> {
        self.change_mode(
            DshotCommand::DisableSignalLineTelemetry,
            8,
            Buffers::OnlyTx,
            PinState::Low,
        )
    }
}

impl<P, S, I, V> Dshot<P, S, I, BidirectionalMode, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    V: ValidBidirVersion,
{
    pub fn erpm_period(&mut self) -> u16 {
        self.last_erpm
    }

    fn read_telemetry(&mut self) {
        let Some(value) = self.rx.read() else {return};
        let Some(telemetry) = decode_telemetry(value) else {return};
        self.last_erpm = telemetry;
    }
}

pub trait ValidController {
    fn send_frame(&mut self, value: u16);
}

impl<P, S, I, V> ValidController for Dshot<P, S, I, UnidirectionalMode, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    V: ValidVersion,
{
    fn send_frame(&mut self, value: u16) {
        trace!("sending unidirectional dshot value {}", value);
        self.tx.write_u16_replicated(value);
    }
}

impl<P, S, I, V> ValidController for Dshot<P, S, I, BidirectionalMode, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    V: ValidBidirVersion,
{
    fn send_frame(&mut self, value: u16) {
        trace!("sending bidirectional dshot value {}", value);
        let value = value ^ 0x000F;
        self.read_telemetry();
        self.sm.restart();
        self.tx.write_u16_replicated(value);
    }
}

impl<P, S, I, M, V> Dshot<P, S, I, M, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    M: ValidMode,
    V: ValidVersion,
    Self: ValidController,
{
    /// Disarm the motor
    pub fn disarm(&mut self) {
        self.send_command(DshotCommand::Disarm);
    }

    /// Beep using the motor
    pub fn beep1(&mut self) {
        self.send_command(DshotCommand::Beep1);
    }

    /// Beep using the motor
    pub fn beep2(&mut self) {
        self.send_command(DshotCommand::Beep2);
    }

    /// Beep using the motor
    pub fn beep3(&mut self) {
        self.send_command(DshotCommand::Beep3);
    }

    /// Beep using the motor
    pub fn beep4(&mut self) {
        self.send_command(DshotCommand::Beep4);
    }

    /// Beep using the motor
    pub fn beep5(&mut self) {
        self.send_command(DshotCommand::Beep5);
    }

    /// Request ESC info
    pub fn esc_info(&mut self) {
        self.send_command(DshotCommand::EscInfo);
    }

    /// Change to motor spin direction 1
    pub fn spin_direction1(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::SpinDirection1);
        }
    }

    /// Change to motor spin direction 1
    pub fn spin_direction2(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::SpinDirection2);
        }
    }

    /// Disable 3D Mode. 3D Mode lets the motor spin in both directions
    pub fn disable_3d_mode(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::Disable3DMode);
        }
    }

    /// Enable 3D Mode. 3D Mode lets the motor spin in both directions
    pub fn enable_3d_mode(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::Enable3DMode);
        }
    }

    /// Request the current settings from the ESC
    pub fn settings_request(&mut self) {
        self.send_command(DshotCommand::SettingsRequest);
    }

    /// Save the current settings to permanent memory
    pub fn save_settings(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::SaveSettings);
        }
    }

    /* these are relatively new (as of Jan. 2023) and we don't support them yet also they should
     * only be used with bidirectional DSHOT
    /// Enable additional telemetry values send over the signal line
    pub fn enable_extended_telemetry(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::EnableExtendedTelemetry);
        }
    }

    /// Disable additional telemetry values send over the signal line
    pub fn disable_extended_telemetry(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::DisableExtendedTelemetry);
        }
    }
    */

    /// Set motor direction to normal
    pub fn spin_direction_normal(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::SpinDirectionNormal);
        }
    }

    /// Set motor direction to reversed
    pub fn spin_direction_reversed(&mut self) {
        for _ in 0..10 {
            self.send_command(DshotCommand::SpinDirectionReversed);
        }
    }

    /// Turn Led number 0 on
    pub fn led0_on(&mut self) {
        self.send_command(DshotCommand::Led0On);
    }

    /// Turn Led number 1 on
    pub fn led1_on(&mut self) {
        self.send_command(DshotCommand::Led1On);
    }

    /// Turn Led number 2 on
    pub fn led2_on(&mut self) {
        self.send_command(DshotCommand::Led2On);
    }

    /// Turn Led number 3 on
    pub fn led3_on(&mut self) {
        self.send_command(DshotCommand::Led3On);
    }

    /// Turn Led number 0 off
    pub fn led0_off(&mut self) {
        self.send_command(DshotCommand::Led0Off);
    }

    /// Turn Led number 1 off
    pub fn led1_off(&mut self) {
        self.send_command(DshotCommand::Led1Off);
    }

    /// Turn Led number 2 off
    pub fn led2_off(&mut self) {
        self.send_command(DshotCommand::Led2Off);
    }

    /// Turn Led number 3 off
    pub fn led3_off(&mut self) {
        self.send_command(DshotCommand::Led3Off);
    }

    /* as far as I know these are not implemented on any esc
    /// Use throttle to stream audio commands instead of throttle commands
    pub fn toggle_audio_stream_mode(&mut self) {
        self.send_command(DshotCommand::AudioStreamModeToggle);
    }

    /// Make the motor quieter
    pub fn toggle_silent_mode(&mut self) {
        self.send_command(DshotCommand::SilentModeToggle);
    }
    */

    /* disable these for now
    pub fn enable_telemetry_continuous_erpm(&mut self) {
        self.send_command(DshotCommand::SignalLineContinuousErpmTelemetry);
    }

    pub fn enable_telemetry_continuous_erpm_period(&mut self) {
        self.send_command(DshotCommand::SignalLineContinuousErpmPeriodTelemetry);
    }

    pub fn telemetry_temperature(&mut self) {
        self.send_command(DshotCommand::SignalLineTemperatueTelemetry);
    }

    pub fn telemetry_voltage(&mut self) {
        self.send_command(DshotCommand::SignalLineVoltageTelemetry);
    }

    pub fn telemetry_current(&mut self) {
        self.send_command(DshotCommand::SignalLineCurrentTelemetry);
    }

    pub fn telemetry_consumption(&mut self) {
        self.send_command(DshotCommand::SignalLineConsumptionTelemetry);
    }

    pub fn telemetry_erpm(&mut self) {
        self.send_command(DshotCommand::SignalLineErpmTelemetry);
    }

    pub fn telemetry_erpm_period(&mut self) {
        self.send_command(DshotCommand::SignalLineErpmPeriodTelemetry);
    }
    */

    /// Set throttle value from 0 to 2000.
    pub fn throttle(&mut self, throttle: u16) {
        if (0..2000).contains(&throttle) {
            self.send_command(DshotCommand::Throttle(throttle));
        }
    }

    fn send_command(&mut self, command: DshotCommand) {
        let frame = DshotFrame {
            command,
            telemetry: false,
        };
        self.send_frame(u16::from(frame));
    }
}

impl<P, S, I, M, V> Dribbler<u16> for Dshot<P, S, I, M, V>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    S: StateMachineIndex,
    M: ValidMode,
    V: ValidVersion,
    Self: ValidController,
{
    fn send(&mut self, speed: u16) {
        let throttle = speed / (u16::MAX / 2000);
        self.throttle(throttle);
    }
}
