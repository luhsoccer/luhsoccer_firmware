#![no_std]

mod commands;
pub mod definitions;
pub mod error;

use core::{hint, marker::PhantomData};

use commands::{
    ClearIrqStatus, Command, GetIrqStatus, GetRxBufferStatus, GetStatus, ReadBuffer, ReadRegister,
    SetAutoFs, SetBufferBaseAddress, SetDioIrqParams, SetModulationParams, SetPacketParams,
    SetPacketType, SetRegulatorMode, SetRfFrequency, SetStandby, SetTx, SetTxParams, WriteBuffer,
    WriteRegister,
};
use definitions::{
    CommandStatus, FlrcBitrateBandwidth, FlrcCodingRate, FlrcCrcLength, FlrcModulationShaping,
    FlrcPacketLength, FlrcSyncWordLength, FlrcWhitening, GfskFlrcPacketType,
    GfskFlrcPreambleLength, GfskFlrcSyncWordMatch, IrqReader, IrqWriter, PacketType, PeriodBase,
    RampTime, Register, StandbyMode, StatusByte,
};
use defmt::{error, info, panic, unwrap, warn};
use embedded_hal::{
    blocking::{delay::DelayMs, spi::Transfer},
    digital::v2::{InputPin, OutputPin},
};
use embedded_hal_async::{
    delay::DelayUs as AsyncDelayUs, digital::Wait, spi::SpiDevice as AsyncSpiDevice,
};
use error::Result;

pub struct SimpleSpiDevice<B, CS> {
    pub bus: B,
    pub cs: CS,
}

impl<B, CS, W> SpiDevice<W> for SimpleSpiDevice<B, CS>
where
    B: Transfer<W>,
    CS: OutputPin,
{
    type Error = B::Error;

    fn transfer<'w>(&mut self, words: &'w mut [W]) -> core::result::Result<&'w [W], Self::Error> {
        if self.cs.set_low().is_err() {
            panic!("unable to set chipselect low");
        }
        let res = self.bus.transfer(words);
        if self.cs.set_high().is_err() {
            panic!("unable to set chipselect high");
        }
        res
    }
}

pub trait SpiDevice<W> {
    type Error;

    fn transfer<'w>(&mut self, words: &'w mut [W]) -> core::result::Result<&'w [W], Self::Error>;
}

pub struct None;
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Flrc {
    preamble_length: GfskFlrcPreambleLength,
    sync_word_length: FlrcSyncWordLength,
    sync_word_match: GfskFlrcSyncWordMatch,
    header_type: GfskFlrcPacketType,
    crc_length: FlrcCrcLength,
    tx_base_address: u8,
}

impl Default for Flrc {
    fn default() -> Self {
        Self {
            preamble_length: GfskFlrcPreambleLength::PreambleLength24Bits,
            sync_word_length: FlrcSyncWordLength::SyncWordLengthP32S,
            sync_word_match: GfskFlrcSyncWordMatch::SyncWordOff,
            header_type: GfskFlrcPacketType::PacketLengthFixed,
            crc_length: FlrcCrcLength::Crc2Bytes,
            tx_base_address: 128,
        }
    }
}

pub struct Blocking;
pub struct Async;

mod sealed {
    pub trait Engine {}
    impl Engine for super::None {}
    impl Engine for super::Flrc {}

    pub trait Type {}
    impl Type for super::Blocking {}
    impl Type for super::Async {}
}

use fugit::Rate;
use heapless::Vec;
pub use sealed::{Engine, Type};

use crate::{
    commands::{GetPacketStatus, SetRx},
    definitions::{ErrorPacketStatusByte, SyncPacketStatusByte},
};

pub struct Sx1280<S, R, B, E: Engine, W, T: Type> {
    spi: S,
    reset: R,
    busy: B,
    engine: E,
    _marker: PhantomData<(W, T)>,
}

impl<S, R, B, W, T: Type> Sx1280<S, R, B, None, W, T> {
    pub fn new(spi: S, reset: R, busy: B) -> Self {
        Self {
            spi,
            reset,
            busy,
            engine: None,
            _marker: PhantomData,
        }
    }
}

impl<S, R, B, W> Sx1280<S, R, B, None, W, Async>
where
    S: AsyncSpiDevice<W>,
    R: OutputPin,
    B: Wait,
    W: From<u8> + Copy + 'static,
    u8: From<W>,
{
    pub async fn init(
        &mut self,
        delay: &mut impl AsyncDelayUs,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        // Reset the sx1280 chip. Timings based on official c-driver
        delay.delay_ms(20).await;
        self.reset.set_low().map_err(error::Error::ResetPin)?;
        delay.delay_ms(50).await;
        self.reset.set_high().map_err(error::Error::ResetPin)?;
        delay.delay_ms(20).await;

        self.set_standby_rc().await?;
        self.set_regulator_mode(true).await?;
        self.set_standby_xosc().await?;

        let status = self.status().await?;
        info!("Sx1280 Init Finished. Status: {}", status);
        Ok(())
    }
}

impl<S, R, B, W> Sx1280<S, R, B, None, W, Blocking>
where
    S: SpiDevice<W>,
    R: OutputPin,
    B: InputPin,
    W: From<u8> + Copy + 'static,
    u8: From<W>,
{
    pub fn init(
        &mut self,
        delay: &mut impl DelayMs<u32>,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        // Reset the sx1280 chip. Timings based on official c-driver
        delay.delay_ms(20);
        self.reset.set_low().map_err(error::Error::ResetPin)?;
        delay.delay_ms(50);
        self.reset.set_high().map_err(error::Error::ResetPin)?;
        delay.delay_ms(20);

        self.set_standby_rc()?;
        self.set_regulator_mode(true)?;
        self.set_standby_xosc()?;

        let status = self.status()?;
        info!("Sx1280 Init Finished. Status: {}", status);
        Ok(())
    }
}

#[maybe_async_cfg::maybe(
    keep_self,
    idents(
        Async(sync = "Blocking", async),
        AsyncSpiDevice(sync = "SpiDevice", async),
        Wait(sync = "InputPin", async)
    ),
    async(),
    sync()
)]
impl<S, R, B, E, W> Sx1280<S, R, B, E, W, Async>
where
    S: AsyncSpiDevice<W>,
    R: OutputPin,
    B: Wait,
    E: Engine,
    W: From<u8> + Copy + 'static,
    u8: From<W>,
{
    pub async fn into_flrc(mut self) -> Sx1280<S, R, B, Flrc, W, Async> {
        if self
            .transfer::<_, 2>(SetPacketType::new(PacketType::Flrc))
            .await
            .is_err()
        {
            error!("unable to switch to flrc packet engine");
        }
        Sx1280 {
            spi: self.spi,
            reset: self.reset,
            busy: self.busy,
            engine: Flrc::default(),
            _marker: PhantomData,
        }
    }
}

impl<S, R, B, W, T: Type> Sx1280<S, R, B, Flrc, W, T> {
    pub fn set_preamble_length(&mut self, length: GfskFlrcPreambleLength) {
        self.engine.preamble_length = length;
    }

    pub fn set_sync_word_length(&mut self, length: FlrcSyncWordLength) {
        self.engine.sync_word_length = length;
    }

    pub fn set_sync_word_match(&mut self, sync_match: GfskFlrcSyncWordMatch) {
        self.engine.sync_word_match = sync_match;
    }

    pub fn set_packet_type(&mut self, packet_type: GfskFlrcPacketType) {
        self.engine.header_type = packet_type;
    }

    pub fn set_crc_length(&mut self, length: FlrcCrcLength) {
        self.engine.crc_length = length;
    }
}

#[maybe_async_cfg::maybe(
    keep_self,
    idents(
        Async(sync = "Blocking", async),
        AsyncSpiDevice(sync = "SpiDevice", async),
        Wait(sync = "InputPin", async)
    ),
    async(),
    sync()
)]
impl<S, R, B, W> Sx1280<S, R, B, Flrc, W, Async>
where
    R: OutputPin,
    B: Wait,
    S: AsyncSpiDevice<W>,
    W: From<u8> + Copy + 'static,
    u8: From<W>,
{
    pub async fn set_frequency(
        &mut self,
        frequency: Rate<u32, 52_000_000, 262_144>,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 4>(SetRfFrequency::new(frequency))
            .await?;
        Ok(())
    }

    pub async fn set_buffer_base_address(
        &mut self,
        tx: u8,
        rx: u8,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 3>(SetBufferBaseAddress::new(tx, rx))
            .await?;
        self.engine.tx_base_address = tx;
        Ok(())
    }

    pub async fn set_modulation_params(
        &mut self,
        bitrate_bandwidth: FlrcBitrateBandwidth,
        coding_rate: FlrcCodingRate,
        modulation_shaping: FlrcModulationShaping,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 4>(SetModulationParams::flrc(
            bitrate_bandwidth,
            coding_rate,
            modulation_shaping,
        ))
        .await?;
        Ok(())
    }

    pub async fn set_sync_word1(&mut self, word: u32) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 7>(WriteRegister::new(
            Register::SyncAddress1Byte3,
            &word.to_be_bytes(),
        ))
        .await?;
        Ok(())
    }

    pub async fn set_sync_word2(&mut self, word: u32) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 7>(WriteRegister::new(
            Register::SyncAddress2Byte3,
            &word.to_be_bytes(),
        ))
        .await?;
        Ok(())
    }

    pub async fn set_sync_word3(&mut self, word: u32) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 7>(WriteRegister::new(
            Register::SyncAddress3Byte3,
            &word.to_be_bytes(),
        ))
        .await?;
        Ok(())
    }

    pub async fn set_tx_param(
        &mut self,
        power: i8,
        ramp: RampTime,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 3>(SetTxParams::new(power, ramp)).await?;
        Ok(())
    }

    pub async fn send_packet<const N: usize>(
        &mut self,
        data: &[u8],
        timeout_base: PeriodBase,
        timeout_count: u16,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        if data.len() < 6 {
            warn!("payload length {} < 6. Extending data", data.len());
        }
        if data.len() > 127 {
            error!("payload length {} > 127. Clipping data", data.len());
        }
        let payload_length = unwrap!(u8::try_from(data.len().clamp(6, 127)), "clamped");
        self.transfer::<_, 8>(SetPacketParams::flrc(
            self.engine.preamble_length,
            self.engine.sync_word_length,
            self.engine.sync_word_match,
            self.engine.header_type,
            FlrcPacketLength(payload_length),
            self.engine.crc_length,
            FlrcWhitening::WhiteningDisable,
        ))
        .await?;
        self.transfer::<_, N>(WriteBuffer::new(self.engine.tx_base_address, data))
            .await?;
        self.clear_interrupts().await?;
        self.transfer::<_, 4>(SetTx::new(timeout_base, timeout_count))
            .await?;
        let (status, _) = self.transfer::<_, 4>(GetStatus).await?;
        if matches!(
            status.command_status,
            CommandStatus::CommandTimeOut
                | CommandStatus::CommandParsingError
                | CommandStatus::CommandExecuteFailure
        ) {
            Err(error::Error::Other)
        } else {
            Ok(())
        }
    }

    pub async fn start_receive_packet(
        &mut self,
        length: u8,
        timeout_base: PeriodBase,
        timeout_count: u16,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        if length < 6 {
            warn!("payload length {} < 6. Extending data", length);
        }
        if length > 127 {
            error!("payload length {} > 127. Clipping data", length);
        }
        let payload_length = length.clamp(6, 127);
        self.transfer::<_, 8>(SetPacketParams::flrc(
            self.engine.preamble_length,
            self.engine.sync_word_length,
            self.engine.sync_word_match,
            self.engine.header_type,
            FlrcPacketLength(payload_length),
            self.engine.crc_length,
            FlrcWhitening::WhiteningDisable,
        ))
        .await?;
        self.clear_interrupts().await?;
        self.transfer::<_, 4>(SetRx::new(timeout_base, timeout_count))
            .await?;

        let status = self.status().await?;
        if matches!(
            status.command_status,
            CommandStatus::CommandTimeOut
                | CommandStatus::CommandParsingError
                | CommandStatus::CommandExecuteFailure
        ) {
            error!("got erronous status: {}", status);
            Err(error::Error::Other)
        } else {
            Ok(())
        }
    }

    #[allow(clippy::type_complexity)]
    pub async fn packet_status(
        &mut self,
    ) -> Result<(i32, ErrorPacketStatusByte, SyncPacketStatusByte), S::Error, R::Error, B::Error>
    {
        let (_, (_rfu, rssi_sync, errors, _status, sync)) =
            self.transfer::<_, 7>(GetPacketStatus).await?;
        let power = -i32::from(rssi_sync / 2);
        let errors = ErrorPacketStatusByte::from(errors);
        let sync =
            SyncPacketStatusByte::try_from(sync).unwrap_or(SyncPacketStatusByte::SyncAddress1);
        Ok((power, errors, sync))
    }

    pub async fn read_packet<const N: usize>(
        &mut self,
    ) -> Result<Vec<u8, N>, S::Error, R::Error, B::Error> {
        let (_, buffer_status) = self.transfer::<_, 4>(GetRxBufferStatus).await?;
        let (_, buffer) = self
            .transfer::<_, N>(ReadBuffer::new(
                buffer_status.rx_start_buffer_pointer,
                buffer_status.payload_length,
            ))
            .await?;

        Ok(buffer)
    }
}

pub struct FlrcPacket<const N: usize> {
    pub data: Vec<u8, N>,
    pub rssi: i32,
    pub error: ErrorPacketStatusByte,
    pub sync: SyncPacketStatusByte,
}

#[maybe_async_cfg::maybe(
    keep_self,
    idents(
        Async(sync = "Blocking", async),
        AsyncSpiDevice(sync = "SpiDevice", async),
        Wait(sync = "InputPin", async)
    ),
    async(),
    sync()
)]
impl<S, R, B, E, W> Sx1280<S, R, B, E, W, Async>
where
    R: OutputPin,
    B: Wait,
    E: Engine,
    S: AsyncSpiDevice<W>,
    W: From<u8> + Copy + 'static,
    u8: From<W>,
{
    pub async fn status(&mut self) -> Result<StatusByte, S::Error, R::Error, B::Error> {
        self.transfer::<_, 2>(GetStatus)
            .await
            .map(|(status, _)| status)
    }

    pub async fn firmware_version(&mut self) -> Result<u16, S::Error, R::Error, B::Error> {
        let command = ReadRegister::from_range(
            Register::FirmwareVersionByte1..=Register::FirmwareVersionByte0,
        );
        let bytes = self.transfer::<_, 6>(command).await?.1;
        Ok(u16::from_be_bytes(unwrap!(bytes
            .into_array()
            .map_err(|v| v.len()))))
    }

    pub async fn set_auto_fs(&mut self, auto_fs: bool) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 2>(SetAutoFs::new(auto_fs)).await?;
        Ok(())
    }

    pub async fn enable_interrupts(
        &mut self,
        general: IrqWriter,
        dio1: IrqWriter,
        dio2: IrqWriter,
        dio3: IrqWriter,
    ) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 9>(SetDioIrqParams::new(general, dio1, dio2, dio3))
            .await?;
        Ok(())
    }

    pub async fn irq_status(&mut self) -> Result<IrqReader, S::Error, R::Error, B::Error> {
        self.transfer::<_, 4>(GetIrqStatus)
            .await
            .map(|(_, reader)| reader)
    }

    pub async fn clear_interrupts(&mut self) -> Result<(), S::Error, R::Error, B::Error> {
        self.transfer::<_, 3>(ClearIrqStatus::new(IrqWriter::new().all()))
            .await?;
        Ok(())
    }

    pub async fn set_standby_rc(&mut self) -> Result<StatusByte, S::Error, R::Error, B::Error> {
        let command = SetStandby::new(StandbyMode::StandbyRc);
        self.transfer::<_, 2>(command)
            .await
            .map(|(status, _)| status)
    }

    pub async fn set_standby_xosc(&mut self) -> Result<StatusByte, S::Error, R::Error, B::Error> {
        let command = SetStandby::new(StandbyMode::StandbyXOsc);
        self.transfer::<_, 2>(command)
            .await
            .map(|(status, _)| status)
    }

    pub async fn set_regulator_mode(
        &mut self,
        enable_dcdc: bool,
    ) -> Result<StatusByte, S::Error, R::Error, B::Error> {
        let command = SetRegulatorMode::new(enable_dcdc);
        self.transfer::<_, 2>(command)
            .await
            .map(|(status, _)| status)
    }
}

impl<S, R, B, E, W> Sx1280<S, R, B, E, W, Async>
where
    R: OutputPin,
    B: Wait,
    E: Engine,
    S: AsyncSpiDevice<W>,
    W: From<u8> + Copy + 'static,
    u8: From<W>,
{
    async fn transfer<C: Command, const N: usize>(
        &mut self,
        command: C,
    ) -> Result<(StatusByte, C::Result<N>), S::Error, R::Error, B::Error> {
        let mut words = command.encode::<W, N>();
        self.busy
            .wait_for_low()
            .await
            .map_err(error::Error::BusyPin)?;
        self.spi
            .transfer_in_place(&mut words)
            .await
            .map_err(error::Error::Spi)?;
        Ok(command.decode::<_, N>(&words[..]))
    }
}

impl<S, R, B, E, W> Sx1280<S, R, B, E, W, Blocking>
where
    R: OutputPin,
    B: InputPin,
    E: Engine,
    S: SpiDevice<W>,
    W: From<u8> + Copy,
    u8: From<W>,
{
    #[allow(clippy::type_complexity)]
    fn transfer<C: Command, const N: usize>(
        &mut self,
        command: C,
    ) -> Result<(StatusByte, <C as Command>::Result<N>), S::Error, R::Error, B::Error> {
        let mut words = command.encode::<W, N>();
        while self.is_busy()? {
            hint::spin_loop();
        }
        let words = self.spi.transfer(&mut words).map_err(error::Error::Spi)?;
        Ok(command.decode::<_, N>(words))
    }
}

impl<S, R, B, E, W, T> Sx1280<S, R, B, E, W, T>
where
    B: InputPin,
    E: Engine,
    T: Type,
{
    pub fn is_busy<SE, RE>(&mut self) -> Result<bool, SE, RE, B::Error> {
        self.busy.is_high().map_err(error::Error::BusyPin)
    }
}
