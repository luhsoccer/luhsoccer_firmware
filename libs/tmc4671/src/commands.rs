use defmt::Format;
use tmc4671_macros::{TMC4671Command, TMC4671Field};

use blanket::blanket;

#[blanket(derive(Ref, Mut))]
pub(crate) trait TMC4671WriteCommand {
    /// Serialize a write command for the TMC4671.
    /// The first byte is the address of the Register.
    /// The rest are the actual data to be written in order highest to lowest.
    fn serialize_write(&self) -> [u8; 5];
}

pub(crate) trait TMC4671Command
where
    Self: Sized,
{
    /// Serialize the address for a read command of this register.
    fn serialize_read() -> u8;

    /// Deserialize a register from the TMC4671.
    /// The bytes are in order highest to lowest
    ///
    /// # Errors
    ///
    /// This function will return an error if the given bytes can't be deserialized into a valid
    /// representation of the data.
    fn deserialize(input: [u8; 4]) -> Result<Self, Error>;
}

pub(crate) trait TMC4671Field
where
    Self: Sized,
{
    /// Serialize a field to write it into a register
    fn serialize_field<const OFFSET: u8, const SIZE: u8>(&self, buffer: &mut [u8; 5]);

    /// Deserialize a field from a read register
    ///
    /// # Errors
    ///
    /// This function will return an error if the read value is not valid and can't be represented
    /// in rust
    fn deserialize_field<const OFFSET: u8, const SIZE: u8>(input: [u8; 4]) -> Result<Self, Error>;
}

macro_rules! tmc4671field_impl_inum {
    ($($t:ty)*) => ($(
        impl TMC4671Field for $t {
            fn serialize_field<const OFFSET: u8, const SIZE: u8>(&self, buffer: &mut [u8; 5]) {
                let mask = (if SIZE >= core::mem::size_of::<i32>() as u8 * 8 {-1} else {(1 << SIZE as u32) - 1}) << OFFSET;
                let a = ((*self as i32) << OFFSET) & mask;
                let a = a.to_be_bytes();
                for i in 0..4 {
                    buffer[i + 1] |= a[i];
                }
            }

            fn deserialize_field<const OFFSET: u8, const SIZE: u8>(input: [u8; 4]) -> Result<Self, Error> {
                const I32_BITSIZE: usize = core::mem::size_of::<i32>() * 8;
                let a = i32::from_be_bytes(input);
                let a = a << (I32_BITSIZE - OFFSET as usize - SIZE as usize);
                let a = a >> (I32_BITSIZE - SIZE as usize);
                Ok(a as Self)
            }
        }
    )*)
}

tmc4671field_impl_inum!(i8 i16 i32 i64 isize);

macro_rules! tmc4671field_impl_unum {
    ($($t:ty)*) => ($(
        impl TMC4671Field for $t {
            fn serialize_field<const OFFSET: u8, const SIZE: u8>(&self, buffer: &mut [u8; 5]) {
                let mask = (if SIZE >= core::mem::size_of::<u32>() as u8 * 8 {u32::MAX} else {(1 << SIZE as u32) - 1}) << OFFSET;
                let a = ((*self as u32) << OFFSET) & mask;
                let a = a.to_be_bytes();
                for i in 0..4 {
                    buffer[i + 1] |= a[i];
                }
            }

            fn deserialize_field<const OFFSET: u8, const SIZE: u8>(input: [u8; 4]) -> Result<Self, Error> {
                const U32_BITSIZE: usize = core::mem::size_of::<u32>() * 8;
                let a = u32::from_be_bytes(input);
                let a = a << (U32_BITSIZE - OFFSET as usize - SIZE as usize);
                let a = a >> (U32_BITSIZE - SIZE as usize);
                Ok(a as Self)
            }
        }
    )*)
}

tmc4671field_impl_unum!(u8 u16 u32 u64 usize);

impl TMC4671Field for bool {
    fn serialize_field<const OFFSET: u8, const SIZE: u8>(&self, buffer: &mut [u8; 5]) {
        let index = OFFSET / 8;
        let shift = OFFSET % 8;
        buffer[usize::from(4 - index)] |= u8::from(*self) << shift;
    }

    fn deserialize_field<const OFFSET: u8, const SIZE: u8>(input: [u8; 4]) -> Result<Self, Error> {
        let index = OFFSET / 8;
        let shift = OFFSET % 8;
        Ok((input[usize::from(3 - index)] & 1 << shift) != 0)
    }
}

impl TMC4671Field for char {
    fn serialize_field<const OFFSET: u8, const SIZE: u8>(&self, buffer: &mut [u8; 5]) {
        let ascii: u8 = (*self).try_into().expect("only ascii allowed");
        ascii.serialize_field::<OFFSET, SIZE>(buffer);
    }

    fn deserialize_field<const OFFSET: u8, const SIZE: u8>(input: [u8; 4]) -> Result<Self, Error> {
        u8::deserialize_field::<OFFSET, SIZE>(input).map(Self::from)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Format)]
pub enum Error {
    InvalidState,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum Direction {
    #[default]
    Positive,
    Negative,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum ChipinfoDataType {
    #[default]
    Type,
    Version,
    Date,
    Time,
    Variant,
    Build,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum AdcRawDataType {
    #[default]
    I0I1,
    VmAgpiA,
    AgpiBAencUx,
    AencVnAencWy,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum CfgDsmodulatorType {
    #[default]
    IntDsmod,
    ExtMclkInput,
    ExtMclkOutput,
    ExtCmp,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum AdcI01Select {
    #[default]
    I0Raw,
    I1Raw,
    I0Ext,
    I1Ext,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum AdcIUVWSelect {
    #[default]
    I0,
    I1,
    I2,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum AencSelectType {
    #[default]
    UX,
    VN,
    WY,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum AnalogInputStageCfg {
    #[default]
    InpVsInn,
    GndVsInn,
    VddDiv4,
    Vdd3Div4,
    InpVsGnd,
    VddDiv2,
    VddDiv42,
    Vdd3Div42,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum PwmChopperMode {
    #[default]
    OffFreeRunning,
    OffLowSideOn,
    OffHighSideOn,
    OffFreeRunning2,
    OffFreeRunning3,
    LowSide,
    HighSide,
    Centered,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Format, TMC4671Field)]
pub enum MotorType {
    #[default]
    NoMotor,
    SinglePhaseDc,
    TwoPhaseStepper,
    ThreePhaseBldc,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum Representation {
    #[default]
    Q8_8,
    Q4_12,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum ConfigAddrType {
    #[default]
    #[val(1)]
    BiquadXA1,
    BiquadXA2,
    #[val(4)]
    BiquadXB0,
    BiquadXB1,
    BiquadXB2,
    BiquadXEnable,
    #[val(9)]
    BiquadVA1,
    BiquadVA2,
    #[val(12)]
    BiquadVB0,
    BiquadVB1,
    BiquadVB2,
    BiquadVEnable,
    #[val(17)]
    BiquadTA1,
    BiquadTA2,
    #[val(20)]
    BiquadTB0,
    BiquadTB1,
    BiquadTB2,
    BiquadTEnable,
    #[val(25)]
    BiquadFA1,
    BiquadFA2,
    #[val(28)]
    BiquadFB0,
    BiquadFB1,
    BiquadFB2,
    BiquadFEnable,
    PrbsAmplitude,
    PrbsDownSamplingRatio,
    #[val(51)]
    RefSwitchConfig,
    EncoderInitHallEnable,
    #[val(60)]
    SinglePinIfStatusCfg,
    SinglePinIfScaleOffset,
    AdvancedPiRepresent,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum PhiSelection {
    PhiESelection,
    #[default]
    PhiEExt,
    PhiEOpenloop,
    PhiEAbn,
    #[val(5)]
    PhiEHal,
    PhiEAenc,
    PhiAAenc,
    #[val(9)]
    PhiMAbn,
    PhiMAbn2,
    PhiMAenc,
    PhiMHal,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum VelocityMeterType {
    #[default]
    Default,
    Advanced,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum PhiESelectionType {
    #[default]
    #[val(1)]
    PhiEExt,
    PhiEOpenloop,
    PhiEAbn,
    #[val(5)]
    PhiEHal,
    PhiEAenc,
    PhiAAenc,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum ModeMotion {
    #[default]
    Stopped,
    Torque,
    Velocity,
    Position,
    PrbsFlux,
    PrbsTorque,
    PrbsVelocity,
    PrbsPosition,
    UqUdExt,
    #[val(10)]
    AgpiATorque,
    AgpiAVelocity,
    AgpiAPosition,
    PwmITorque,
    PwmiVelocity,
    PwmIPosition,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum PidType {
    #[default]
    Parallel,
    Sequential,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum PidErrorType {
    #[default]
    Torque,
    Flux,
    Velocity,
    Position,
    TorqueSum,
    FluxSum,
    VelocitySum,
    PositionSum,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Format, TMC4671Field)]
pub enum InterimDataType {
    #[default]
    PidinTargetTorque,
    PidinTargetFlux,
    PidinTargetVelocity,
    PidinTargetPosition,
    PidoutTargetTorque,
    PidoutTargetFlux,
    PidoutTargetVelocity,
    PidoutTargetPosition,
    FocIwyIux,
    FocIv,
    FocIbIa,
    FocIqId,
    FocUqUd,
    FocUqUdLimited,
    FocUbUa,
    FocUwyUux,
    FocUv,
    PwmWyUx,
    PwmUv,
    AdcI1I0,
    PidTorqueTargetFluxTargetTorqueActualFluxActualDiv256,
    PidTorqueTargetTorqueActual,
    PidFluxTargetFluxActual,
    PidVelocityTargetVelocityActualDiv256,
    PidVelocityTargetVelocityActual,
    PidPositionTargetPositionActualDiv256,
    PidPositionTargetPositionActual,
    FfVelocity,
    FfTorque,
    ActualVelocityPptm,
    RefSwitchStatus,
    HomePosition,
    LeftPosition,
    RightPosition,
    EncInitHallStatus,
    EncInitHallPhiEAbnOffset,
    EncInitHallPhiEAencOffset,
    EncInitHallPhiAAencOffset,
    #[val(42)]
    SinglePinIfPwmDutyCycleTorqueTarget,
    SinglePinIfVelocityTarget,
    SinglePinIfPositionTarget,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x00)]
#[readonly]
pub(crate) struct ChipinfoDataSiType {
    #[size(8)]
    pub fourth: char,
    #[size(8)]
    pub third: char,
    #[size(8)]
    pub second: char,
    #[size(8)]
    pub first: char,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x00)]
#[readonly]
pub(crate) struct ChipinfoDataSiVersion {
    pub minor: u16,
    pub major: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x00)]
#[readonly]
pub(crate) struct ChipinfoDataSiDate {
    #[size(4)]
    pub day_1: u8,
    #[size(4)]
    pub day_2: u8,
    #[size(4)]
    pub month_1: u8,
    #[size(4)]
    pub month_2: u8,
    #[size(4)]
    pub year_1: u8,
    #[size(4)]
    pub year_2: u8,
    #[size(4)]
    pub year_3: u8,
    #[size(4)]
    pub year_4: u8,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x00)]
#[readonly]
pub(crate) struct ChipinfoDataSiTime {
    #[size(4)]
    pub second_1: u8,
    #[size(4)]
    pub second_2: u8,
    #[size(4)]
    pub minute_1: u8,
    #[size(4)]
    pub minute_2: u8,
    #[size(4)]
    pub hour_1: u8,
    #[size(4)]
    pub hour_2: u8,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x00)]
#[readonly]
pub(crate) struct ChipinfoDataSiVariant {
    pub variant: u32,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x00)]
#[readonly]
pub(crate) struct ChipinfoDataSiBuild {
    pub build: u32,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x01)]
pub(crate) struct ChipinfoAddr {
    #[size(8)]
    pub addr: ChipinfoDataType,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x02)]
#[readonly]
pub(crate) struct AdcRawDataI0I1 {
    pub i0_raw: u16,
    pub i1_raw: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x02)]
#[readonly]
pub(crate) struct AdcRawDataVmAgpiA {
    pub vm_raw: u16,
    pub agpi_a: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x02)]
#[readonly]
pub(crate) struct AdcRawDataAgpiBAencUx {
    pub agpi_b: u16,
    pub aenc_ux: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x02)]
#[readonly]
pub(crate) struct AdcRawDataAencVnAencWy {
    pub aenc_vn: u16,
    pub aenc_wy: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x03)]
pub(crate) struct AdcRawAddr {
    #[size(8)]
    pub addr: AdcRawDataType,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x04)]
pub(crate) struct DsAdcMcfgBMcfgA {
    #[size(2)]
    pub cfg_dsmodulator_a: CfgDsmodulatorType,
    pub mclk_polarity_a: bool,
    pub mdat_polarity_a: bool,
    pub set_nclk_mclk_i_a: bool,
    #[offset(8)]
    pub blanking_a: u8,
    #[size(2)]
    pub cfg_dsmodulator_b: CfgDsmodulatorType,
    pub mclk_polarity_b: bool,
    pub mdat_polarity_b: bool,
    pub set_nclk_mclk_i_b: bool,
    #[offset(24)]
    pub blanking_b: u8,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x05)]
pub(crate) struct DsAdcMclkA {
    pub mclk: u32,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x06)]
pub(crate) struct DsAdcMclkB {
    pub mclk: u32,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x07)]
pub(crate) struct DsAdcMdecBMdecA {
    pub mdec_a: u16,
    pub mdec_b: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x08)]
pub(crate) struct AdcI1ScaleOffset {
    pub offset: u16,
    pub scale: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x09)]
pub(crate) struct AdcI0ScaleOffset {
    pub offset: u16,
    pub scale: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x0A)]
pub(crate) struct AdcISelect {
    #[size(8)]
    pub i0_select: AdcI01Select,
    #[size(8)]
    pub i1_select: AdcI01Select,
    #[size(2)]
    #[offset(24)]
    pub iux_select: AdcIUVWSelect,
    #[size(2)]
    pub iv_select: AdcIUVWSelect,
    #[size(2)]
    pub iwy_select: AdcIUVWSelect,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x0B)]
pub(crate) struct AdcI1I0Ext {
    pub i0: u16,
    pub i1: u16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x0C)]
pub(crate) struct DsAnalogInputStageCfg {
    #[size(4)]
    pub i0: AnalogInputStageCfg,
    #[size(4)]
    pub i1: AnalogInputStageCfg,
    #[size(4)]
    pub vm: AnalogInputStageCfg,
    #[size(4)]
    pub agpi_a: AnalogInputStageCfg,
    #[size(4)]
    pub agpi_b: AnalogInputStageCfg,
    #[size(4)]
    pub aenc_ux: AnalogInputStageCfg,
    #[size(4)]
    pub aenc_vn: AnalogInputStageCfg,
    #[size(4)]
    pub aenc_wy: AnalogInputStageCfg,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x0D)]
pub(crate) struct Aenc0ScaleOffset {
    pub offset: u16,
    pub scale: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x0E)]
pub(crate) struct Aenc1ScaleOffset {
    pub offset: u16,
    pub scale: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x0F)]
pub(crate) struct Aenc2ScaleOffset {
    pub offset: u16,
    pub scale: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x11)]
pub(crate) struct AencSelect {
    #[size(8)]
    pub aenc_0_select: AencSelectType,
    #[size(8)]
    pub aenc_1_select: AencSelectType,
    #[size(8)]
    pub aenc_2_select: AencSelectType,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x12)]
#[readonly]
pub(crate) struct AdcIwyIux {
    pub iux: i16,
    pub iwy: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x13)]
#[readonly]
pub(crate) struct AdcIv {
    pub iv: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x15)]
#[readonly]
pub(crate) struct AencWyUx {
    pub ux: i16,
    pub wy: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x16)]
#[readonly]
pub(crate) struct AencVn {
    pub vn: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x17)]
pub(crate) struct PwmPolarities {
    pub lowside: bool,
    pub highside: bool,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x18)]
pub(crate) struct PwmMaxcnt {
    #[size(12)]
    pub maxcnt: u16, // frequency = 100MHz / (maxcnt + 1)
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x19)]
pub(crate) struct PwmBbmHBbmL {
    pub low: u8,  // 10ns
    pub high: u8, // 10ns
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x1A)]
pub(crate) struct PwmSvChop {
    #[size(8)]
    pub chop: PwmChopperMode,
    pub space_vector: bool,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x1B)]
pub(crate) struct MotorTypeNPolePairs {
    pub pole_pairs: u16,
    #[size(8)]
    pub motor_type: MotorType,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x1C)]
pub(crate) struct PhiEExt {
    pub phi_e: i16,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x1F)]
pub(crate) struct OpenloopMode {
    #[offset(12)]
    pub phi_direction: Direction,
}

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, TMC4671Command)]
#[addr(0x20)]
pub(crate) struct OpenloopAcceleration {
    pub acceleration: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x21)]
pub(crate) struct OpenloopVelocityTarget {
    pub velocity: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x22)]
pub(crate) struct OpenloopVelocityActual {
    pub velocity: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x23)]
pub(crate) struct OpenloopPhi {
    pub phi: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x24)]
pub(crate) struct UqUdExt {
    pub ud: i16,
    pub uq: i16,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x25)]
pub(crate) struct AbnDecoderMode {
    pub apol: bool,
    pub bpol: bool,
    pub npol: bool,
    pub use_abn_as_n: bool,
    #[offset(8)]
    pub cln: bool,
    #[size(1)]
    #[offset(12)]
    pub direction: Direction,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x26)]
pub(crate) struct AbnDecoderPpr {
    #[size(24)]
    pub ppr: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x27)]
pub(crate) struct AbnDecoderCount {
    #[size(24)]
    pub count: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x28)]
pub(crate) struct AbnDecoderCountN {
    #[size(24)]
    pub count: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x29)]
pub(crate) struct AbnDecoderPhiEPhiMOffset {
    pub phi_m_offset: i16,
    pub phi_e_offset: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x2A)]
#[readonly]
pub(crate) struct AbnDecoderPhiEPhiM {
    pub phi_m: i16,
    pub phi_e: i16,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x2C)]
pub(crate) struct Abn2DecoderMode {
    pub apol: bool,
    pub bpol: bool,
    pub npol: bool,
    pub use_abn_as_n: bool,
    #[offset(8)]
    pub cln: bool,
    #[offset(12)]
    pub direction: Direction,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x2D)]
pub(crate) struct Abn2DecoderPpr {
    #[size(24)]
    pub ppr: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x2E)]
pub(crate) struct Abn2DecoderCount {
    #[size(24)]
    pub count: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x2F)]
pub(crate) struct Abn2DecoderCountN {
    #[size(24)]
    pub count: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x30)]
pub(crate) struct Abn2DecoderPhiMOffset {
    pub phi_m_offset: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x31)]
#[readonly]
pub(crate) struct Abn2DecoderPhiM {
    pub phi_m: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x33)]
pub(crate) struct HallMode {
    pub polarity: bool,
    #[offset(4)]
    pub synchronous_pwm_sampling: bool,
    #[offset(8)]
    pub interpolation: bool,
    #[offset(12)]
    pub direction: Direction,
    #[size(12)]
    #[offset(16)]
    pub hall_blank: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x34)]
pub(crate) struct HallPosition060000 {
    pub position_000: i16,
    pub position_060: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x35)]
pub(crate) struct HallPosition180120 {
    pub position_120: i16,
    pub position_180: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x36)]
pub(crate) struct HallPosition300240 {
    pub position_300: i16,
    pub position_240: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x37)]
pub(crate) struct HallPhiEPhiMOffset {
    pub phi_m_offset: i16,
    pub phi_e_offset: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x38)]
pub(crate) struct HallDphiMax {
    pub dphi_max: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x39)]
#[readonly]
pub(crate) struct HallPhiEInterpolatedPhiE {
    pub phi_e: i16,
    pub phi_e_interpolated: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x3A)]
#[readonly]
pub(crate) struct HallPhiM {
    pub phi_m: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x3B)]
pub(crate) struct AencDecoderMode {
    pub n90deg_120deg: bool,
    #[size(1)]
    #[offset(12)]
    pub direction: Direction,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x3C)]
pub(crate) struct AencDecoderNThreshold {
    pub n_threshold: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x3D)]
#[readonly]
pub(crate) struct AencDecoderPhiARaw {
    pub phi_a: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x3E)]
pub(crate) struct AencDecoderPhiAOffset {
    pub offset: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x3F)]
#[readonly]
pub(crate) struct AencDecoderPhiA {
    pub phi_a: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x40)]
pub(crate) struct AencDecoderPpr {
    pub ppr: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x41)]
#[readonly]
pub(crate) struct AencDecoderCount {
    pub count: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x42)]
pub(crate) struct AencDecoderCountN {
    pub count: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x45)]
pub(crate) struct AencDecoderPhiEPhiMOffset {
    pub phi_m_offset: i16,
    pub phi_e_offset: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x46)]
#[readonly]
pub(crate) struct AencDecoderPhiEPhiM {
    pub phi_m: i16,
    pub phi_e: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadXA1 {
    pub biquad_x_a_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadXA2 {
    pub biquad_x_a_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadXB1 {
    pub biquad_x_b_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadXB2 {
    pub biquad_x_b_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadXB3 {
    pub biquad_x_b_3: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadXEnable {
    #[offset(31)]
    pub biquad_x_enable: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadVA1 {
    pub biquad_v_a_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadVA2 {
    pub biquad_v_a_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadVB1 {
    pub biquad_v_b_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadVB2 {
    pub biquad_v_b_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadVB3 {
    pub biquad_v_b_3: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadVEnable {
    #[offset(31)]
    pub biquad_v_enable: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadTA1 {
    pub biquad_t_a_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadTA2 {
    pub biquad_t_a_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadTB1 {
    pub biquad_t_b_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadTB2 {
    pub biquad_t_b_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadTB3 {
    pub biquad_t_b_3: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadTEnable {
    #[offset(31)]
    pub biquad_t_enable: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadFA1 {
    pub biquad_f_a_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadFA2 {
    pub biquad_f_a_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadFB1 {
    pub biquad_f_b_1: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadFB2 {
    pub biquad_f_b_2: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadFB3 {
    pub biquad_f_b_3: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataBiquadFEnable {
    #[offset(31)]
    pub biquad_f_enable: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataPrbsAmplitude {
    pub prbs_amplitude: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataPrbsDownSamplingRation {
    pub prbs_down_sampling_ration: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataRefSwitchConfig {
    pub ref_switchconfig: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataEncoderInitHallEnable {
    pub encoder_init_hall_enable: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataSinglePinIfCfgStatus {
    pub cfg: u8,
    #[offset(16)]
    pub status: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataSinglePinIfOffsetScale {
    pub offset: u16,
    pub scale: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4D)]
pub(crate) struct ConfigDataCurrentVelocityPositionRepresentation {
    #[size(1)]
    pub current_i: Representation,
    #[size(1)]
    pub current_p: Representation,
    #[size(1)]
    pub velocity_i: Representation,
    #[size(1)]
    pub velocity_p: Representation,
    #[size(1)]
    pub position_i: Representation,
    #[size(1)]
    pub position_p: Representation,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x4E)]
pub(crate) struct ConfigAddr {
    #[size(32)]
    pub addr: ConfigAddrType,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x50)]
pub(crate) struct VelocitySelection {
    #[size(8)]
    pub velocity_selection: PhiSelection,
    #[size(8)]
    pub velocity_meter_selection: VelocityMeterType,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x51)]
pub(crate) struct PositionSelection {
    #[size(8)]
    pub position_selecion: PhiSelection,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x52)]
pub(crate) struct PhiESelection {
    #[size(8)]
    pub phi_e_selection: PhiESelectionType,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x53)]
#[readonly]
pub(crate) struct PhiE {
    pub phi_e: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x54)]
pub(crate) struct PidFluxPI {
    pub i: i16,
    pub p: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x56)]
pub(crate) struct PidTorquePI {
    pub i: i16,
    pub p: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x58)]
pub(crate) struct PidVelocityPI {
    pub i: i16,
    pub p: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x5A)]
pub(crate) struct PidPositionPI {
    pub i: i16,
    pub p: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x5D)]
pub(crate) struct PidoutUqUdLimits {
    pub limit: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x5E)]
pub(crate) struct PidTorqueFluxLimits {
    pub limit: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x60)]
pub(crate) struct PidVelocityLimit {
    pub limit: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x61)]
pub(crate) struct PidPositionLimitLow {
    pub limit: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x62)]
pub(crate) struct PidPositionLimitHigh {
    pub limit: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x63)]
pub(crate) struct ModeRampModeMotion {
    #[size(8)]
    pub mode_motion: ModeMotion,
    #[size(7)]
    #[offset(24)]
    pub mode_pid_smpl: u8,
    #[size(1)]
    pub mode_pid_type: PidType,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x64)]
pub(crate) struct PidTorqueFluxTarget {
    pub flux_target: i16,
    pub torque_target: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x65)]
pub(crate) struct PidTorqueFluxOffset {
    pub flux_offset: i16,
    pub torque_offset: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x66)]
pub(crate) struct PidVelocityTarget {
    pub target: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x67)]
pub(crate) struct PidVelocityOffset {
    pub offset: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x68)]
pub(crate) struct PidPositionTarget {
    pub target: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x69)]
#[readonly]
pub(crate) struct PidTorqueFluxActual {
    pub flux: i16,
    pub torque: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6A)]
#[readonly]
pub(crate) struct PidVelocityActual {
    pub velocity: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6B)]
pub(crate) struct PidPositionActual {
    pub position: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataTorque {
    pub torque: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataFlux {
    pub flux: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataVelocity {
    pub velocity: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataPosition {
    pub position: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataTorqueSum {
    pub torque_sum: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataFluxSum {
    pub flux_sum: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
pub(crate) struct PidErrorDataVelocitySum {
    pub velocity_sum: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6C)]
#[readonly]
pub(crate) struct PidErrorDataPositionSum {
    pub position_sum: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6D)]
pub(crate) struct PidErrorAddr {
    #[size(8)]
    pub addr: PidErrorType,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6E)]
pub(crate) struct InterimDataI32 {
    pub value: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6E)]
pub(crate) struct InterimDataI16I16 {
    pub value1: i16,
    pub value2: i16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6E)]
pub(crate) struct InterimDataI8I8I8I8 {
    pub value1: i8,
    pub value2: i8,
    pub value3: i8,
    pub value4: i8,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6E)]
pub(crate) struct InterimDataU16 {
    pub value: u16,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x6F)]
pub(crate) struct InterimAddr {
    #[size(8)]
    pub addr: InterimDataType,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x75)]
pub(crate) struct AdcVmLimits {
    pub low: u16,
    pub high: u16,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x76)]
#[readonly]
pub(crate) struct Tmc4671InputsRaw {
    pub abn_a: bool,
    pub abn_b: bool,
    pub abn_n: bool,
    #[offset(4)]
    pub abn2_a: bool,
    pub abn2_b: bool,
    pub abn2_n: bool,
    #[offset(8)]
    pub hall_ux: bool,
    pub hall_v: bool,
    pub hall_wy: bool,
    #[offset(12)]
    pub ref_sw_r: bool,
    pub ref_sw_h: bool,
    pub ref_sw_l: bool,
    pub enable_in: bool,
    pub dirstp_stp: bool,
    pub dirstp_dir: bool,
    pub pwm_in: bool,
    #[offset(20)]
    pub hall_ux_filt: bool,
    pub hall_v_filt: bool,
    pub hall_wy_filt: bool,
    #[offset(28)]
    pub pwm_idle_l: bool,
    pub pwm_idle_h: bool,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x77)]
#[readonly]
pub(crate) struct Tmc4671OutputsRaw {
    pub pwm_ux1_l: bool,
    pub pwm_ux1_h: bool,
    pub pwm_vx2_l: bool,
    pub pwm_vx2_h: bool,
    pub pwm_wy1_l: bool,
    pub pwm_wy1_h: bool,
    pub pwm_y2_l: bool,
    pub pwm_y2_h: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x78)]
pub(crate) struct StepWidth {
    pub step_width: i32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x79)]
pub(crate) struct UartBps {
    #[size(24)]
    pub bps: u32,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x7B)]
pub(crate) struct GpioDsAdciConfig {
    pub ndbgspim_gpi: bool,
    pub ngpio_dsadcs_a: bool,
    pub ngpio_dsadcs_b: bool,
    pub gpio_group_a_nin_out: bool,
    pub gpio_group_b_nin_out: bool,
    pub group_a_dsadcs_nclkin_clkout: bool,
    pub group_b_dsadcs_nclkin_clkout: bool,
    #[offset(16)]
    pub gpo: u8,
    pub gpi: u8,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x7C)]
pub(crate) struct StatusFlags {
    pub pid_x_target_limit: bool,
    pub pid_x_target_ddt_limit: bool,
    pub pid_x_errsum_limit: bool,
    pub pid_x_output_limit: bool,
    pub pid_v_target_limit: bool,
    pub pid_v_target_ddt_limit: bool,
    pub pid_v_errsum_limit: bool,
    pub pid_v_output_limit: bool,
    pub pid_id_target_limit: bool,
    pub pid_id_target_ddt_limit: bool,
    pub pid_id_errsum_limit: bool,
    pub pid_id_output_limit: bool,
    pub pid_iq_target_limit: bool,
    pub pid_iq_target_ddt_limit: bool,
    pub pid_iq_errsum_limit: bool,
    pub pid_iq_output_limit: bool,
    pub ipark_cirlim_limit_u_d: bool,
    pub ipark_cirlim_limit_u_q: bool,
    pub ipark_cirlim_limit_u_r: bool,
    pub not_pll_locked: bool,
    pub ref_sw_r: bool,
    pub ref_sw_h: bool,
    pub ref_sw_l: bool,
    pub pwm_min: bool,
    pub pwm_max: bool,
    pub adc_i_clipped: bool,
    pub aenc_clipped: bool,
    pub enc_n: bool,
    pub enc_2_n: bool,
    pub aenc_n: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, TMC4671Command)]
#[addr(0x7D)]
pub(crate) struct StatusMask {
    pub mask: u32,
}
