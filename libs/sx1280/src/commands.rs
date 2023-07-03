use core::ops::{Bound, RangeBounds};

use crate::definitions::{
    BleConnectionState, BleCrcLength, BleTxTestPayload, BleWhitening, FlrcBitrateBandwidth,
    FlrcCodingRate, FlrcCrcLength, FlrcModulationShaping, FlrcPacketLength, FlrcSyncWordLength,
    FlrcWhitening, GfskBleBitrateBandwidth, GfskBleModulationIndex, GfskBleModulationShaping,
    GfskCrcLength, GfskFlrcPacketType, GfskFlrcPreambleLength, GfskFlrcSyncWordMatch,
    GfskPacketLength, GfskSyncWordLength, GfskWhitening, IrqReader, IrqWriter, LoRaBandwidth,
    LoRaCodingRate, LoRaRangingCrc, LoRaRangingIq, LoRaRangingPacketLength, LoRaRangingPacketType,
    LoRaRangingPreamble, LoRaSpreadingFactor, PacketType, PeriodBase, RampTime, RxBufferStatus,
    SleepMode, StandbyMode,
};
use fugit::Rate;
pub use sealed::Command;

use defmt::{assert, unwrap, Format};
use heapless::Vec;

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct GetStatus;

impl Command for GetStatus {
    type Result<const N: usize> = ();
    const OP_CODE: u8 = 0xC0;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        Vec::new()
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct WriteRegister<'a> {
    address: u16,
    data: &'a [u8],
}

impl<'a> WriteRegister<'a> {
    #[allow(dead_code)]
    pub fn new(address: impl Into<u16>, data: &'a [u8]) -> Self {
        Self {
            address: address.into(),
            data,
        }
    }
}

impl<'a> Command for WriteRegister<'a> {
    type Result<const N: usize> = ();
    const OP_CODE: u8 = 0x18;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let mut vec = unwrap!(
            Vec::from_slice(&self.address.to_be_bytes()[..]),
            "size checked above"
        );
        unwrap!(vec.extend_from_slice(self.data), "size checked above");
        vec
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct ReadRegister {
    address: u16,
    size: u16,
}

impl ReadRegister {
    #[allow(dead_code)]
    pub fn new(address: impl Into<u16>) -> Self {
        Self {
            address: address.into(),
            size: 1,
        }
    }

    #[allow(dead_code)]
    pub fn from_range<T: Into<u16> + Copy>(addresses: impl RangeBounds<T>) -> Self {
        let lower = match addresses.start_bound() {
            Bound::Included(&lower) => lower.into(),
            Bound::Excluded(&lower) => lower.into() + 1,
            Bound::Unbounded => 0x891,
        };
        let upper = match addresses.end_bound() {
            Bound::Included(&upper) => upper.into() - 1,
            Bound::Excluded(&upper) => upper.into(),
            Bound::Unbounded => 0x9DD,
        };
        let size = upper - lower;

        Self {
            address: lower,
            size,
        }
    }

    #[allow(dead_code)]
    pub fn from_size(address: impl Into<u16>, size: u16) -> Self {
        Self {
            address: address.into(),
            size,
        }
    }
}

impl Command for ReadRegister {
    type Result<const N: usize> = Vec<u8, N>;
    const OP_CODE: u8 = 0x18;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let final_size = usize::from(self.size) + 3;
        let mut vec = unwrap!(Vec::from_slice(&self.address.to_be_bytes()));
        unwrap!(vec.resize_default(final_size));
        vec
    }

    fn result<const N: usize>(&self, bytes: Vec<u8, N>) -> Self::Result<N> {
        unwrap!(Vec::from_slice(&bytes[3..]))
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct WriteBuffer<'a> {
    offset: u8,
    data: &'a [u8],
}

impl<'a> WriteBuffer<'a> {
    #[allow(dead_code)]
    pub const fn new(offset: u8, data: &'a [u8]) -> Self {
        Self { offset, data }
    }
}

impl<'a> Command for WriteBuffer<'a> {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x1A;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let mut vec = unwrap!(Vec::from_slice(&[self.offset]));
        unwrap!(vec.extend_from_slice(self.data), "size checked above");
        vec
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct ReadBuffer {
    offset: u8,
    size: u8,
}

impl ReadBuffer {
    #[allow(dead_code)]
    pub const fn new(offset: u8, size: u8) -> Self {
        Self { offset, size }
    }

    #[allow(dead_code)]
    pub fn from_range(range: impl RangeBounds<u8>) -> Self {
        let offset = match range.start_bound() {
            Bound::Included(&lower) => lower,
            Bound::Excluded(&lower) => lower + 1,
            Bound::Unbounded => 0x0,
        };
        let size = match range.end_bound() {
            Bound::Included(&upper) => upper.wrapping_add(1).wrapping_sub(offset),
            Bound::Excluded(&upper) => upper.wrapping_sub(offset),
            Bound::Unbounded => 0xFF,
        };
        Self { offset, size }
    }
}

impl Command for ReadBuffer {
    type Result<const N: usize> = Vec<u8, N>;

    const OP_CODE: u8 = 0x1B;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let final_size = usize::from(self.size) + 2;
        let mut vec = unwrap!(Vec::from_slice(&[self.offset]));
        unwrap!(vec.resize_default(final_size));
        vec
    }

    fn result<const N: usize>(&self, bytes: Vec<u8, N>) -> Self::Result<N> {
        unwrap!(Vec::from_slice(&bytes[2..]))
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetSleep {
    mode: SleepMode,
}

impl SetSleep {
    #[allow(dead_code)]
    pub const fn new(ram_retention: bool, buffer_retention: bool) -> Self {
        Self {
            mode: SleepMode {
                buffer_retention,
                ram_retention,
            },
        }
    }
}

impl Command for SetSleep {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x84;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[self.mode.into()]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetStandby {
    mode: StandbyMode,
}

impl SetStandby {
    #[allow(dead_code)]
    pub const fn new(mode: StandbyMode) -> Self {
        Self { mode }
    }
}

impl Command for SetStandby {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x80;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[self.mode.into()]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetFs;

impl Command for SetFs {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0xC1;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        Vec::new()
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetTx {
    period_base: PeriodBase,
    count: u16,
}

impl SetTx {
    #[allow(dead_code)]
    pub const fn new(period_base: PeriodBase, count: u16) -> Self {
        Self { period_base, count }
    }
}

impl Command for SetTx {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x83;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let mut vec = unwrap!(Vec::from_slice(&[self.period_base.into()]));
        unwrap!(vec.extend_from_slice(&self.count.to_be_bytes()));
        vec
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetRx {
    period_base: PeriodBase,
    count: u16,
}

impl SetRx {
    #[allow(dead_code)]
    pub const fn new(period_base: PeriodBase, count: u16) -> Self {
        Self { period_base, count }
    }
}

impl Command for SetRx {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x82;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let mut vec = unwrap!(Vec::from_slice(&[self.period_base.into()]));
        unwrap!(vec.extend_from_slice(&self.count.to_be_bytes()));
        vec
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetTxContinuousWave;

impl Command for SetTxContinuousWave {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0xD1;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        Vec::new()
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetAutoFs {
    enable: bool,
}

impl SetAutoFs {
    #[allow(dead_code)]
    pub const fn new(enable: bool) -> Self {
        Self { enable }
    }
}

impl Command for SetAutoFs {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x9E;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[if self.enable { 1 } else { 0 }]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetPacketType {
    packettype: PacketType,
}

impl SetPacketType {
    #[allow(dead_code)]
    pub const fn new(packettype: PacketType) -> Self {
        Self { packettype }
    }
}

impl Command for SetPacketType {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x8A;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[self.packettype.into()]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SetRfFrequency {
    frequency: Rate<u32, 52_000_000, 262_144>,
}

impl SetRfFrequency {
    #[allow(dead_code)]
    pub const fn new(frequency: Rate<u32, 52_000_000, 262_144>) -> Self {
        Self { frequency }
    }
}

impl Command for SetRfFrequency {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x86;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&self.frequency.raw().to_be_bytes()[1..]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetTxParams {
    power: u8,
    ramptime: RampTime,
}

impl SetTxParams {
    #[allow(dead_code)]
    pub fn new(power: i8, ramptime: RampTime) -> Self {
        assert!(
            (-18..13).contains(&power),
            "power must be between -18dBm and 13dBm"
        );
        let power = unwrap!(u8::try_from(power + 18), "range checked above");
        Self { power, ramptime }
    }
}

impl Command for SetTxParams {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x8E;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[self.power, self.ramptime.into()]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetBufferBaseAddress {
    tx_base: u8,
    rx_base: u8,
}

impl SetBufferBaseAddress {
    #[allow(dead_code)]
    pub const fn new(tx_base: u8, rx_base: u8) -> Self {
        Self { tx_base, rx_base }
    }
}

impl Command for SetBufferBaseAddress {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x8F;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[self.tx_base, self.rx_base]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetModulationParams(u8, u8, u8);

impl SetModulationParams {
    #[allow(dead_code)]
    fn new(param0: impl Into<u8>, param1: impl Into<u8>, param2: impl Into<u8>) -> Self {
        Self(param0.into(), param1.into(), param2.into())
    }

    #[allow(dead_code)]
    pub fn flrc(
        bitrate_bandwidth: FlrcBitrateBandwidth,
        coding_rate: FlrcCodingRate,
        modulation_shaping: FlrcModulationShaping,
    ) -> Self {
        Self::new(bitrate_bandwidth, coding_rate, modulation_shaping)
    }

    #[allow(dead_code)]
    pub fn ble_gfsk(
        bitrate_bandwidth: GfskBleBitrateBandwidth,
        modulation_index: GfskBleModulationIndex,
        modulation_shaping: GfskBleModulationShaping,
    ) -> Self {
        Self::new(bitrate_bandwidth, modulation_index, modulation_shaping)
    }

    #[allow(dead_code)]
    pub fn lora(
        spreading_factor: LoRaSpreadingFactor,
        bandwidth: LoRaBandwidth,
        coding_rate: LoRaCodingRate,
    ) -> Self {
        Self::new(spreading_factor, bandwidth, coding_rate)
    }
}

impl Command for SetModulationParams {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x8B;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[self.0, self.1, self.2]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetPacketParams(u8, u8, u8, u8, u8, u8, u8);

impl SetPacketParams {
    #[allow(dead_code)]
    fn new(
        param1: impl Into<u8>,
        param2: impl Into<u8>,
        param3: impl Into<u8>,
        param4: impl Into<u8>,
        param5: impl Into<u8>,
        param6: impl Into<u8>,
        param7: impl Into<u8>,
    ) -> Self {
        Self(
            param1.into(),
            param2.into(),
            param3.into(),
            param4.into(),
            param5.into(),
            param6.into(),
            param7.into(),
        )
    }

    #[allow(dead_code)]
    pub fn flrc(
        preamble_length: GfskFlrcPreambleLength,
        sync_word_length: FlrcSyncWordLength,
        sync_word_match: GfskFlrcSyncWordMatch,
        header_type: GfskFlrcPacketType,
        payload_length: FlrcPacketLength,
        crc_length: FlrcCrcLength,
        whitening: FlrcWhitening,
    ) -> Self {
        Self::new(
            preamble_length,
            sync_word_length,
            sync_word_match,
            header_type,
            payload_length,
            crc_length,
            whitening,
        )
    }

    #[allow(dead_code)]
    pub fn gfsk(
        preamble_length: GfskFlrcPreambleLength,
        sync_word_length: GfskSyncWordLength,
        sync_word_match: GfskFlrcSyncWordMatch,
        header_type: GfskFlrcPacketType,
        payload_length: GfskPacketLength,
        crc_length: GfskCrcLength,
        whitening: GfskWhitening,
    ) -> Self {
        Self::new(
            preamble_length,
            sync_word_length,
            sync_word_match,
            header_type,
            payload_length,
            crc_length,
            whitening,
        )
    }

    #[allow(dead_code)]
    pub fn ble(
        connection_state: BleConnectionState,
        crc_length: BleCrcLength,
        test_payload: BleTxTestPayload,
        whitening: BleWhitening,
    ) -> Self {
        Self::new(
            connection_state,
            crc_length,
            test_payload,
            whitening,
            0,
            0,
            0,
        )
    }

    #[allow(dead_code)]
    pub fn lora(
        preamble_length: LoRaRangingPreamble,
        header_type: LoRaRangingPacketType,
        payload_length: LoRaRangingPacketLength,
        crc: LoRaRangingCrc,
        iq: LoRaRangingIq,
    ) -> Self {
        Self::new(preamble_length, header_type, payload_length, crc, iq, 0, 0)
    }
}

impl Command for SetPacketParams {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x8C;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[
            self.0, self.1, self.2, self.3, self.4, self.5, self.6
        ]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct GetRxBufferStatus;

impl Command for GetRxBufferStatus {
    type Result<const N: usize> = RxBufferStatus;

    const OP_CODE: u8 = 0x17;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[0, 0, 0]))
    }

    fn result<const N: usize>(&self, bytes: Vec<u8, N>) -> Self::Result<N> {
        RxBufferStatus {
            payload_length: bytes[1],
            rx_start_buffer_pointer: bytes[2],
        }
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct GetPacketStatus;

impl Command for GetPacketStatus {
    type Result<const N: usize> = (u8, u8, u8, u8, u8);

    const OP_CODE: u8 = 0x1D;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[0, 0, 0, 0, 0, 0]))
    }

    fn result<const N: usize>(&self, bytes: Vec<u8, N>) -> Self::Result<N> {
        (bytes[1], bytes[2], bytes[3], bytes[4], bytes[5])
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct GetIrqStatus;

impl Command for GetIrqStatus {
    type Result<const N: usize> = IrqReader;

    const OP_CODE: u8 = 0x15;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[0, 0, 0]))
    }

    fn result<const N: usize>(&self, mut bytes: Vec<u8, N>) -> Self::Result<N> {
        bytes.remove(0);
        let irq_status = u16::from_be_bytes(unwrap!(bytes.into_array().map_err(|v| v.len())));
        IrqReader::new(irq_status)
    }
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct ClearIrqStatus {
    bits: u16,
}

impl ClearIrqStatus {
    #[allow(dead_code)]
    pub const fn new(writer: IrqWriter) -> Self {
        Self {
            bits: writer.finish(),
        }
    }
}

impl Command for ClearIrqStatus {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x97;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&self.bits.to_be_bytes()))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetDioIrqParams {
    irq: u16,
    dio1: u16,
    dio2: u16,
    dio3: u16,
}

impl SetDioIrqParams {
    #[allow(dead_code)]
    pub const fn new(irq: IrqWriter, dio1: IrqWriter, dio2: IrqWriter, dio3: IrqWriter) -> Self {
        Self {
            irq: irq.finish(),
            dio1: dio1.finish(),
            dio2: dio2.finish(),
            dio3: dio3.finish(),
        }
    }
}

impl Command for SetDioIrqParams {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x8D;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        let mut vec = unwrap!(Vec::from_slice(&self.irq.to_be_bytes()));
        unwrap!(vec.extend_from_slice(&self.dio1.to_be_bytes()));
        unwrap!(vec.extend_from_slice(&self.dio2.to_be_bytes()));
        unwrap!(vec.extend_from_slice(&self.dio3.to_be_bytes()));
        vec
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub struct SetRegulatorMode {
    dcdc: bool,
}

impl SetRegulatorMode {
    #[allow(dead_code)]
    pub const fn new(dcdc: bool) -> Self {
        Self { dcdc }
    }

    #[allow(dead_code)]
    pub const fn ldo() -> Self {
        Self::new(false)
    }

    #[allow(dead_code)]
    pub const fn dcdc() -> Self {
        Self::new(true)
    }
}

impl Command for SetRegulatorMode {
    type Result<const N: usize> = ();

    const OP_CODE: u8 = 0x96;

    fn params<const N: usize>(&self) -> Vec<u8, N> {
        unwrap!(Vec::from_slice(&[if self.dcdc { 1 } else { 0 }]))
    }

    fn result<const N: usize>(&self, _bytes: Vec<u8, N>) -> Self::Result<N> {}
}

mod sealed {
    use defmt::unwrap;
    use heapless::Vec;

    use crate::definitions::StatusByte;

    pub trait Command {
        type Result<const N: usize>;
        const OP_CODE: u8;

        fn params<const N: usize>(&self) -> Vec<u8, N>;

        fn result<const N: usize>(&self, bytes: Vec<u8, N>) -> Self::Result<N>;

        fn encode<W: From<u8>, const N: usize>(&self) -> Vec<W, N> {
            let mut res = Vec::new();
            unwrap!(
                res.push(Self::OP_CODE.into()),
                "vector holding spi data to small"
            );
            for param in self.params::<N>() {
                unwrap!(
                    res.push(param.into()),
                    "vector holding the spi data to small"
                );
            }
            res
        }

        fn decode<W: Into<u8> + Copy, const N: usize>(
            &self,
            bytes: &[W],
        ) -> (StatusByte, Self::Result<N>) {
            let status = bytes[0].into();
            let mut bytes_vec = Vec::<u8, N>::new();
            for byte in &bytes[1..] {
                unwrap!(
                    bytes_vec.push((*byte).into()),
                    "vector holding spi response to small"
                );
            }
            (status.into(), self.result(bytes_vec))
        }
    }
}
