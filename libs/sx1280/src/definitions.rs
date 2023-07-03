use defmt::{warn, Format};

// Zero sized structs to represent states of the rf module
pub mod states {
    pub struct StandbyRc;
    pub struct StandbyXOsc;
    pub struct FrequencySynthesis;
    pub struct Transmit;
    pub struct Receive;
}

// Zero sized structs to represent the selected packet engine of the rf module
pub mod engines {
    pub struct None;

    pub trait EnabledEngine {}
    pub trait TxEngine {}

    pub struct Gfsk;

    impl EnabledEngine for Gfsk {}
    impl TxEngine for Gfsk {}

    pub struct LoRa;

    impl EnabledEngine for LoRa {}
    impl TxEngine for LoRa {}
    pub struct Ranging;

    impl EnabledEngine for Ranging {}
    // Ranging can't transmit packets

    pub struct Flrc;

    impl EnabledEngine for Flrc {}
    impl TxEngine for Flrc {}
    pub struct Ble;

    impl EnabledEngine for Ble {}
    impl TxEngine for Ble {}
}

macro_rules! into_u8 {
    ($s:ty) => {
        impl From<$s> for u8 {
            fn from(value: $s) -> Self {
                value as u8
            }
        }
    };
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[allow(dead_code)] // Enum is just a list of register addresses
#[repr(u16)]
pub enum Register {
    RxGain = 0x891,
    ManualGainSetting = 0x895,
    LNAGainValue = 0x89E,
    LNAGainControl = 0x89F,
    SynchPeakAttenuation = 0x8C2,
    PayloadLength = 0x901,
    LoRaHeaderMode = 0x903,
    RangingRequestAddressByte3 = 0x912,
    RangingRequestAddressByte2 = 0x913,
    RangingRequestAddressByte1 = 0x914,
    RangingRequestAddressByte0 = 0x915,
    RangingDeviceAddressByte3 = 0x916,
    RangingDeviceAddressByte2 = 0x917,
    RangingDeviceAddressByte1 = 0x918,
    RangingDeviceAddressByte0 = 0x919,
    RangingFilterWindowSize = 0x91E,
    ResetRangingFilter = 0x923,
    RangingResultMUX = 0x924,
    SFAdditionalConfiguration = 0x925,
    RangingCalibrationByte2 = 0x92B,
    RangingCalibrationByte1 = 0x92C,
    RangingCalibrationByte0 = 0x92D,
    RangingIDCheckLength = 0x931,
    FrequencyErrorCorrection = 0x93C,
    LoRaSynchWord0 = 0x944,
    LoRaSynchWord1 = 0x945, // Documentations says 0x955 but 0x955 is already reversed for FEIByte1. 0x945 is a wild guess based on the register format
    FEIByte2 = 0x954,
    FEIByte1 = 0x955,
    FEIByte0 = 0x956,
    RangingResultByte2 = 0x961,
    RangingResultByte1 = 0x962,
    RangingResultByte0 = 0x963,
    RangingRSSI = 0x964,
    FreezeRangingResult = 0x97F,
    PacketPreambleSettings = 0x9C1,
    WhiteningInitialValue = 0x9C2,      // Documentation says 0x9C5
    CRCPolynomialDefinitionMSB = 0x9C3, // Documentation says 0x9C6
    CRCPolynomialDefinitionLSB = 0x9C4, // Documentation says 0x9C7
    CRCPolynomialSeedByte2 = 0x9C5,     // Documentation says 0x9C7
    CRCPolynomialSeedByte1 = 0x9C6,     // Documentation says 0x9C8
    CRCPolynomialSeedByte0 = 0x9C7,     // Documentation says 0x9C9
    CRCInitialValueMSB = 0x9C8,
    CRCInitialValueLSB = 0x9C9,
    SynchAddressControl = 0x9CD,
    SyncAddress1Byte4 = 0x9CE,
    SyncAddress1Byte3 = 0x9CF,
    SyncAddress1Byte2 = 0x9D0,
    SyncAddress1Byte1 = 0x9D1,
    SyncAddress1Byte0 = 0x9D2,
    SyncAddress2Byte4 = 0x9D3,
    SyncAddress2Byte3 = 0x9D4,
    SyncAddress2Byte2 = 0x9D5,
    SyncAddress2Byte1 = 0x9D6,
    SyncAddress2Byte0 = 0x9D7,
    SyncAddress3Byte4 = 0x9D8,
    SyncAddress3Byte3 = 0x9D9,
    SyncAddress3Byte2 = 0x9DA,
    SyncAddress3Byte1 = 0x9DB,
    SyncAddress3Byte0 = 0x9DC,
    FirmwareVersionByte1 = 0x153,
    FirmwareVersionByte0 = 0x154,
}

impl From<Register> for u16 {
    fn from(value: Register) -> Self {
        value as u16
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum CircuitMode {
    Reserved,
    StandbyRC,
    StandbyXOsc,
    Fs,
    Rx,
    Tx,
}

impl From<u8> for CircuitMode {
    fn from(value: u8) -> Self {
        match value {
            0x2 => Self::StandbyRC,
            0x3 => Self::StandbyXOsc,
            0x4 => Self::Fs,
            0x5 => Self::Rx,
            0x6 => Self::Tx,
            _ => Self::Reserved,
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum CommandStatus {
    Reserved,
    CommandSuccess,
    DataAvailable,
    CommandTimeOut,
    CommandParsingError,
    CommandExecuteFailure,
    TransmissionTerminated,
}

impl From<u8> for CommandStatus {
    fn from(value: u8) -> Self {
        match value {
            0x1 => Self::CommandSuccess,
            0x2 => Self::DataAvailable,
            0x3 => Self::CommandTimeOut,
            0x4 => Self::CommandParsingError,
            0x5 => Self::CommandExecuteFailure,
            0x6 => Self::TransmissionTerminated,
            _ => Self::Reserved,
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct StatusByte {
    pub circuit_mode: CircuitMode,
    pub command_status: CommandStatus,
}

impl From<u8> for StatusByte {
    fn from(value: u8) -> Self {
        let cir = (value & 0b1110_0000) >> 5;
        let com = (value & 0b0001_1100) >> 2;

        Self {
            circuit_mode: cir.into(),
            command_status: com.into(),
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SleepMode {
    pub buffer_retention: bool,
    pub ram_retention: bool,
}

impl From<SleepMode> for u8 {
    fn from(value: SleepMode) -> Self {
        (if value.ram_retention { 1 } else { 0 }) + (if value.buffer_retention { 2 } else { 0 })
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum StandbyMode {
    StandbyRc = 0x00,
    StandbyXOsc = 0x01,
}

into_u8!(StandbyMode);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum PacketType {
    Gfsk = 0x00,
    LoRa = 0x01,
    Ranging = 0x02,
    Flrc = 0x03,
    Ble = 0x04,
}

into_u8!(PacketType);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct ErrorPacketStatusByte {
    pub sync_error: bool,
    pub length_error: bool,
    pub crc_error: bool,
    pub abort_error: bool,
    pub header_received: bool,
    pub packet_received: bool,
    pub packet_ctrl_busy: bool,
}

impl From<u8> for ErrorPacketStatusByte {
    fn from(value: u8) -> Self {
        Self {
            sync_error: value & 0b01000000 != 0,
            length_error: value & 0b00100000 != 0,
            crc_error: value & 0b00010000 != 0,
            abort_error: value & 0b00001000 != 0,
            header_received: value & 0b00000100 != 0,
            packet_received: value & 0b00000010 != 0,
            packet_ctrl_busy: value & 0b00000001 != 0,
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum SyncPacketStatusByte {
    SyncAddress1,
    SyncAddress2,
    SyncAddress3,
}

impl TryFrom<u8> for SyncPacketStatusByte {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value & 0b001 != 0 {
            Ok(Self::SyncAddress1)
        } else if value & 0b010 != 0 {
            Ok(Self::SyncAddress2)
        } else if value & 0b100 != 0 {
            Ok(Self::SyncAddress3)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskBleBitrateBandwidth {
    Bitrate2000Bandwidth24 = 0x04,
    Bitrate1600Bandwidth24 = 0x28,
    Bitrate1000Bandwidth24 = 0x4C,
    Bitrate1000Bandwidth12 = 0x45,
    Bitrate0800Bandwidth24 = 0x70,
    Bitrate0800Bandwidth12 = 0x69,
    Bitrate0500Bandwidth12 = 0x8D,
    Bitrate0500Bandwidth06 = 0x86,
    Bitrate0400Bandwidth12 = 0xB1,
    Bitrate0400Bandwidth06 = 0xAA,
    Bitrate0250Bandwidth06 = 0xCE,
    Bitrate0250Bandwidth03 = 0xC7,
    Bitrate0125Bandwidth03 = 0xEF,
}

into_u8!(GfskBleBitrateBandwidth);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskBleModulationIndex {
    Index035 = 0x00,
    Index050 = 0x01,
    Index075 = 0x02,
    Index100 = 0x03,
    Index125 = 0x04,
    Index150 = 0x05,
    Index175 = 0x06,
    Index200 = 0x07,
    Index225 = 0x08,
    Index250 = 0x09,
    Index275 = 0x0A,
    Index300 = 0x0B,
    Index325 = 0x0C,
    Index350 = 0x0D,
    Index375 = 0x0E,
    Index400 = 0x0F,
}

into_u8!(GfskBleModulationIndex);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskBleModulationShaping {
    BtOff = 0x00,
    Bt010 = 0x10,
    Bt005 = 0x20,
}

into_u8!(GfskBleModulationShaping);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum FlrcBitrateBandwidth {
    Bitrate1300Bandwidth12 = 0x45,
    Bitrate1000Bandwidth12 = 0x69,
    Bitrate0650Bandwidth06 = 0x86,
    Bitrate0520Bandwidth06 = 0xAA,
    Bitrate0325Bandwidth03 = 0xC7,
    Bitrate0260Bandwidth03 = 0xEB,
}

into_u8!(FlrcBitrateBandwidth);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum FlrcCodingRate {
    CodingRate12 = 0x00,
    CodingRate34 = 0x02,
    CodingRate11 = 0x04,
}

into_u8!(FlrcCodingRate);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum FlrcModulationShaping {
    BtOff = 0x00,
    Bt010 = 0x10,
    Bt005 = 0x20,
}

into_u8!(FlrcModulationShaping);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LoRaSpreadingFactor {
    SpreadingFactor05 = 0x50,
    SpreadingFactor06 = 0x60,
    SpreadingFactor07 = 0x70,
    SpreadingFactor08 = 0x80,
    SpreadingFactor09 = 0x90,
    SpreadingFactor10 = 0xA0,
    SpreadingFactor11 = 0xB0,
    SpreadingFactor12 = 0xC0,
}

into_u8!(LoRaSpreadingFactor);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LoRaBandwidth {
    Bandwidth1600 = 0x0A,
    Bandwidth0800 = 0x18,
    Bandwidth0400 = 0x26,
    Bandwidth0200 = 0x34,
}

into_u8!(LoRaBandwidth);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LoRaCodingRate {
    CodingRate45 = 0x01,
    CodingRate46 = 0x02,
    CodingRate47 = 0x03,
    CodingRate48 = 0x04,
    CodingRateLongInterleaving45 = 0x05,
    CodingRateLongInterleaving46 = 0x06,
    CodingRateLongInterleaving47 = 0x07,
}

into_u8!(LoRaCodingRate);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RangingSpreadingFactor {
    SpreadingFactor05 = 0x50,
    SpreadingFactor06 = 0x60,
    SpreadingFactor07 = 0x70,
    SpreadingFactor08 = 0x80,
    SpreadingFactor09 = 0x90,
    SpreadingFactor10 = 0xA0,
}

into_u8!(RangingSpreadingFactor);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RangingBandwidth {
    Bandwidth1600 = 0x0A,
    Bandwidth0800 = 0x18,
    Bandwidth0400 = 0x26,
}

into_u8!(RangingBandwidth);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RangingCodingRate {
    CodingRate45 = 0x01,
    CodingRate46 = 0x02,
    CodingRate47 = 0x03,
    CodingRate48 = 0x04,
}

into_u8!(RangingCodingRate);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskFlrcPreambleLength {
    PreambleLength04Bits = 0x00,
    PreambleLength08Bits = 0x10,
    PreambleLength12Bits = 0x20,
    PreambleLength16Bits = 0x30,
    PreambleLength20Bits = 0x40,
    PreambleLength24Bits = 0x50,
    PreambleLength28Bits = 0x60,
    PreambleLength32Bits = 0x70,
}

into_u8!(GfskFlrcPreambleLength);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskSyncWordLength {
    SyncWordLength1Bits = 0x00,
    SyncWordLength2Bits = 0x02,
    SyncWordLength3Bits = 0x04,
    SyncWordLength4Bits = 0x06,
    SyncWordLength5Bits = 0x08,
}

into_u8!(GfskSyncWordLength);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskFlrcSyncWordMatch {
    SyncWordOff = 0x00,
    SyncWord1 = 0x10,
    SyncWord2 = 0x20,
    SyncWord12 = 0x30,
    SyncWord3 = 0x40,
    SyncWord13 = 0x50,
    SyncWord23 = 0x60,
    SyncWord123 = 0x70,
}

into_u8!(GfskFlrcSyncWordMatch);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskFlrcPacketType {
    PacketLengthFixed = 0x00,
    PacketLengthVariable = 0x20,
}

into_u8!(GfskFlrcPacketType);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct GfskPacketLength(pub u8);

impl From<GfskPacketLength> for u8 {
    fn from(value: GfskPacketLength) -> Self {
        value.0
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskCrcLength {
    CrcOff = 0x00,
    Crc1Bytes = 0x10,
    Crc2Bytes = 0x20,
}

into_u8!(GfskCrcLength);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GfskWhitening {
    WhiteningEnable = 0x00,
    WhiteningDisable = 0x08,
}

into_u8!(GfskWhitening);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BleConnectionState {
    PayloadLengthMax031Bytes = 0x00,
    PayloadLengthMax037Bytes = 0x20,
    TxTestMode = 0x40,
    PayloadLengthMax255Bytes = 0x80,
}

into_u8!(BleConnectionState);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BleCrcLength {
    CrcOff = 0x00,
    Crc3Bytes = 0x10,
}

into_u8!(BleCrcLength);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BleTxTestPayload {
    PseudoRandomBinarySequence09Deg = 0x00,
    Eyelong10 = 0x04,
    Eyeshort10 = 0x08,
    PseudoRandomBinarySequence15Deg = 0x0C,
    All1 = 0x10,
    All0 = 0x14,
    Eyelong01 = 0x18,
    Eyeshort01 = 0x1C,
}

into_u8!(BleTxTestPayload);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BleWhitening {
    WhiteningEnable = 0x00,
    WhiteningDisable = 0x08,
}

into_u8!(BleWhitening);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum FlrcSyncWordLength {
    NoSync = 0x00,
    SyncWordLengthP32S = 0x04,
}

into_u8!(FlrcSyncWordLength);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct FlrcPacketLength(pub u8);

impl From<FlrcPacketLength> for u8 {
    fn from(value: FlrcPacketLength) -> Self {
        if !(6..128).contains(&value.0) {
            warn!("invalid packet length: {}", value.0);
        }
        value.0.clamp(6, 127)
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum FlrcCrcLength {
    CrcOff = 0x00,
    Crc2Bytes = 0x10,
    Crc3Bytes = 0x20,
    Crc4Bytes = 0x30,
}

into_u8!(FlrcCrcLength);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum FlrcWhitening {
    WhiteningDisable = 0x08,
}

into_u8!(FlrcWhitening);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct LoRaRangingPreamble {
    pub preamble_mantissa: u8,
    pub preamble_exponent: u8,
}

impl From<LoRaRangingPreamble> for u8 {
    fn from(value: LoRaRangingPreamble) -> Self {
        assert!(
            value.preamble_mantissa >= 1 && value.preamble_mantissa <= 15,
            "Preamble mantissa must be between 1 and 15"
        );
        assert!(
            value.preamble_exponent >= 1 && value.preamble_exponent <= 15,
            "Preamble exponent must be between 1 and 15"
        );

        (value.preamble_exponent << 4) & value.preamble_mantissa
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LoRaRangingPacketType {
    ExplicitHeader = 0x00,
    ImplicitHeader = 0x80,
}

into_u8!(LoRaRangingPacketType);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct LoRaRangingPacketLength(pub u8);

impl From<LoRaRangingPacketLength> for u8 {
    fn from(value: LoRaRangingPacketLength) -> Self {
        if !(1..255).contains(&value.0) {
            warn!("invalid packet length: {}", value.0);
        }
        value.0.clamp(1, 255)
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LoRaRangingCrc {
    CrcEnable = 0x20,
    CrcDisable = 0x00,
}

into_u8!(LoRaRangingCrc);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LoRaRangingIq {
    IqInverted = 0x00,
    IqStandard = 0x40,
}

into_u8!(LoRaRangingIq);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RampTime {
    Ramp02us = 0x00,
    Ramp04us = 0x20,
    Ramp06us = 0x40,
    Ramp08us = 0x60,
    Ramp10us = 0x80,
    Ramp12us = 0xA0,
    Ramp16us = 0xC0,
    Ramp20us = 0xE0,
}

into_u8!(RampTime);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct Power(pub i8);

impl From<Power> for u8 {
    fn from(value: Power) -> Self {
        assert!(
            value.0 >= -18 && value.0 <= 13,
            "Power must be between -18 and 13 dBm"
        );

        // Can't overflow, was checked before
        (18 + value.0)
            .try_into()
            .expect("Can't overflow, was checked before")
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum PeriodBase {
    MicroSeconds15N625 = 0x00,
    MicroSeconds62N5 = 0x01,
    MilliSeconds1 = 0x02,
    MilliSeconds4 = 0x03,
}

into_u8!(PeriodBase);

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct TimeoutDuration {
    pub steps: u16,
    pub base: PeriodBase,
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
#[repr(u16)]
pub enum IrqBit {
    TxDone = 0,
    RxDone = 1,
    SyncWordValid = 2,
    SyncWordError = 3,
    HeaderValid = 4,
    HeaderError = 5,
    CrcError = 6,
    RangingSlaveResponseDone = 7,
    RangingSlaveRequestDiscard = 8,
    RangingMasterResultValid = 9,
    RangingMasterTimeout = 10,
    RangingSlaveRequestValid = 11,
    CadDone = 12,
    CadDetected = 13,
    RxTxTimeout = 14,
    PreambledDetectedAdvancedRangingDone = 15,
}

#[derive(Default, Clone, Copy)]
pub struct IrqWriter(u16);
#[derive(Clone, Copy)]
pub struct IrqReader(u16);

impl IrqWriter {
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn all(self) -> Self {
        Self(u16::MAX)
    }

    #[must_use]
    pub const fn set(self, bit: IrqBit) -> Self {
        Self(self.0 | (1 << (bit as u16)))
    }

    #[must_use]
    pub const fn clear(self, bit: IrqBit) -> Self {
        Self(self.0 & !(1 << (bit as u16)))
    }

    #[must_use]
    pub const fn finish(self) -> u16 {
        self.0
    }
}

impl IrqReader {
    #[must_use]
    pub const fn new(irq: u16) -> Self {
        Self(irq)
    }

    #[must_use]
    pub const fn is_set(&self, bit: IrqBit) -> bool {
        (self.0 & (1 << (bit as u16))) != 0
    }

    #[must_use]
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct RxBufferStatus {
    pub payload_length: u8,
    pub rx_start_buffer_pointer: u8,
}
