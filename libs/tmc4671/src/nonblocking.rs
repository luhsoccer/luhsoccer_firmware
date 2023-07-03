#![allow(clippy::future_not_send)]

use defmt::{debug, error, info, trace, warn};
use embedded_hal_async::{delay::DelayUs, spi::SpiDevice};
use fugit::Duration;
use paste::paste;

#[allow(clippy::wildcard_imports)]
use crate::commands::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy, defmt::Format)]
pub enum Error<S> {
    Spi(S),
    Deserialization(crate::commands::Error),
    CalibrationValidation,
}

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct Controller<S: SpiDevice> {
    device: S,
}

impl<S> Controller<S>
where
    S: SpiDevice,
{
    pub const fn new(device: S) -> Self {
        Self { device }
    }

    /// Calibrate the encoder by turning it to a known position. After Initialization
    /// the Motor is turned to a second known position and checked if it is at that position
    /// with a given wiggle room.
    ///
    /// # Errors
    ///
    /// This function will return an error if the SPI transaction didn't succeed or the motor
    /// didn't turn to the right position after calibration.
    pub async fn calibrate_encoder(
        &mut self,
        force: i16,
        delay: &mut impl DelayUs,
        wiggle_room: u8,
    ) -> Result<(), Error<S::Error>> {
        info!("initializing encoder");
        self.set_openloop_speed(0).await?;
        self.set_phi_e_ext(0).await?;
        self.set_phi_e_selection(PhiESelectionType::PhiEExt).await?;
        self.set_openloop_torque_flux((0, force)).await?;
        let result = self.calibrate_encoder_try(delay, wiggle_room).await;
        self.set_mode(ModeMotion::Stopped).await?;
        self.set_openloop_torque_flux((0, 0)).await?;
        result
    }

    async fn calibrate_encoder_try(
        &mut self,
        delay: &mut impl DelayUs,
        wiggle_room_deg: u8,
    ) -> Result<(), Error<S::Error>> {
        #[allow(clippy::cast_possible_wrap)]
        const TEST_POSITION: i16 = (u16::MAX / 3) as i16;

        self.set_mode(ModeMotion::UqUdExt).await?;
        debug!("driving motor to 0 position");
        self.wait_still(delay).await?;
        self.set_decoder_count(0).await?;

        // rotate motor 120° electrical to check calibration
        let wiggle_room: i16 = i16::MAX / 180 * i16::from(wiggle_room_deg);
        self.set_phi_e_ext(TEST_POSITION).await?;
        debug!("driving motor to test position");
        self.wait_still(delay).await?;
        let encoder_phi = self.decoder_phi_e().await?;
        if ((TEST_POSITION - wiggle_room)..=(TEST_POSITION + wiggle_room)).contains(&encoder_phi) {
            Ok(())
        } else {
            error!(
                "the motor didn't move to the expected position ({})",
                encoder_phi
            );
            Err(Error::CalibrationValidation)
        }
    }

    async fn wait_still(&mut self, delay: &mut impl DelayUs) -> Result<(), Error<S::Error>> {
        const MAX_WAIT_TIME_MS: u32 = 10_000; // 10s
        const LOOP_TIME_MS: u32 = 1;
        const MAX_NUM_LOOPS: u32 = MAX_WAIT_TIME_MS / LOOP_TIME_MS;
        let mut last_encoder_count = self.decoder_count().await?;
        let mut still_counter = 0;
        let mut loop_count = 0;
        delay.delay_ms(200).await;
        while still_counter < 128 {
            delay.delay_ms(LOOP_TIME_MS).await;
            let encoder_count = self.decoder_count().await?;
            if encoder_count == last_encoder_count {
                still_counter += 1;
            } else {
                still_counter = 0;
            }
            last_encoder_count = encoder_count;
            loop_count += 1;
            if loop_count >= MAX_NUM_LOOPS {
                warn!("motor didn't stand still after {}ms", MAX_WAIT_TIME_MS);
                return Err(Error::CalibrationValidation);
            }
        }
        delay.delay_ms(1000).await;
        Ok(())
    }
}

macro_rules! field_impl {
    ($name:ident, $transform:expr, $result:ty) => {
        // implements read operations
        // ``name``: base name for the function
        // ``transform``: transform to apply to get from the raw command to the output value
        // ``result``: value returned by the function
        impl<S> Controller<S>
        where
            S: SpiDevice,
        {
            /// reads from the TMC4671
            ///
            /// # Errors
            ///
            /// Returns an Error if the SPI transaction didn't succeed or the value could not be
            /// deserialized.
            pub async fn $name(&mut self) -> Result<$result, Error<S::Error>> {
                let x = self.read().await?;
                Ok($transform(x))
            }
        }
    };
    ($name:ident, $transform:expr, $result:ty, $pre_command:expr) => {
        // implements read operations for Data registers requiering an matchin Addr register to be
        // set
        // ``name``: base name for the function
        // ``transform``: transform to apply to get from the raw command to the output value
        // ``result``: value returned by the function
        // ``pre_command``: command to send for setting the matching Addr register
        impl<S> Controller<S>
        where
            S: SpiDevice,
        {
            /// reads from the TMC4671
            ///
            /// # Errors
            ///
            /// Returns an Error if the SPI transaction didn't succeed or the value could not be
            /// deserialized.
            pub async fn $name(&mut self) -> Result<$result, Error<S::Error>> {
                self.write($pre_command).await?;
                let x = self.read().await?;
                Ok($transform(x))
            }
        }
    };
    ($name:ident, $transform:expr, $back_transform:expr, $result:ty) => {
        // implements send operations
        // ``name``: base name for the function
        // ``transform``: transform to apply to get from the raw command to the output value
        // ``back_transform``: transform to convert from the input value to the raw command given a
        // raw command.
        // ``result``: value returned by the function
        field_impl!($name, $transform, $result);

        impl<S> Controller<S>
        where
            S: SpiDevice,
        {
            paste! {
                /// Write part of the register on the TMC4671
                ///
                /// # Errors
                ///
                /// Returns an Error if the SPI transaction didn't succeed or the value could not
                /// be deserialized.
                pub async fn [<set_ $name>](&mut self, value: $result) -> Result<(), Error<S::Error>> {
                    let mut x = self.read().await?;
                    $back_transform(&mut x, value);
                    self.write(x).await
                }
            }
        }
    };
    ($name:ident, $transform:expr, $back_transform:expr, $result:ty, single) => {
        // implements send operations where the whole raw command is set by the input
        // ``name``: base name for the function
        // ``transform``: transform to apply to get from the raw command to the output value
        // ``back_transform``: transform to convert from the input value to the raw command given a
        // raw command.
        // ``result``: value returned by the function
        field_impl!($name, $transform, $result);

        impl<S> Controller<S>
        where
            S: SpiDevice,
        {
            paste! {
                /// Write register on the TMC4671
                ///
                /// # Errors
                ///
                /// Returns an Error if the SPI transaction didn't succeed.
                pub async fn [<set_ $name>](&mut self, value: $result) -> Result<(), Error<S::Error>> {
                    let mut x = Default::default();
                    $back_transform(&mut x, value);
                    self.write(x).await
                }
            }
        }
    };
}

field_impl!(
    hardware_type,
    |x: ChipinfoDataSiType| [x.first, x.second, x.third, x.fourth],
    [char; 4],
    ChipinfoAddr {
        addr: ChipinfoDataType::Type
    }
);
field_impl!(
    hardware_version,
    |x: ChipinfoDataSiVersion| (x.major, x.minor),
    (u16, u16),
    ChipinfoAddr {
        addr: ChipinfoDataType::Version
    }
);
field_impl!(
    hardware_date,
    |x: ChipinfoDataSiDate| (
        u16::from(x.year_4) * 1000
            + u16::from(x.year_3) * 100
            + u16::from(x.year_2) * 10
            + u16::from(x.year_1),
        x.month_2 * 10 + x.month_1,
        x.day_2 * 10 + x.day_1
    ),
    (u16, u8, u8),
    ChipinfoAddr {
        addr: ChipinfoDataType::Date
    }
);
field_impl!(
    hardware_time,
    |x: ChipinfoDataSiTime| (
        x.hour_2 * 10 + x.hour_1,
        x.minute_2 * 10 + x.minute_1,
        x.second_2 * 10 + x.second_1
    ),
    (u8, u8, u8),
    ChipinfoAddr {
        addr: ChipinfoDataType::Time
    }
);
field_impl!(
    hardware_variant,
    |x: ChipinfoDataSiVariant| x.variant,
    u32,
    ChipinfoAddr {
        addr: ChipinfoDataType::Variant
    }
);
field_impl!(
    hardware_build,
    |x: ChipinfoDataSiBuild| x.build,
    u32,
    ChipinfoAddr {
        addr: ChipinfoDataType::Build
    }
);

field_impl!(velocity, |x: PidVelocityActual| x.velocity, i32);
field_impl!(
    mode,
    |x: ModeRampModeMotion| x.mode_motion,
    |x: &mut ModeRampModeMotion, v| x.mode_motion = v,
    ModeMotion
);

field_impl!(
    mclka_polarity,
    |x: DsAdcMcfgBMcfgA| x.mclk_polarity_a,
    |x: &mut DsAdcMcfgBMcfgA, v| x.mclk_polarity_a = v,
    bool
);

field_impl!(
    velocity_target,
    |x: PidVelocityTarget| x.target,
    |x: &mut PidVelocityTarget, v| x.target = v,
    i32,
    single
);
field_impl!(
    motor_type,
    |x: MotorTypeNPolePairs| x.motor_type,
    |x: &mut MotorTypeNPolePairs, v| x.motor_type = v,
    MotorType
);
field_impl!(
    pole_pairs,
    |x: MotorTypeNPolePairs| x.pole_pairs,
    |x: &mut MotorTypeNPolePairs, v| x.pole_pairs = v,
    u16
);
field_impl!(
    motor_type_pole_pairs,
    |x: MotorTypeNPolePairs| (x.motor_type, x.pole_pairs),
    |x: &mut MotorTypeNPolePairs, v: (MotorType, u16)| {
        x.motor_type = v.0;
        x.pole_pairs = v.1;
    },
    (MotorType, u16),
    single
);
field_impl!(
    bbm,
    |x: PwmBbmHBbmL| Duration::<u32, 1, 100_000_000>::from_ticks(u32::from(x.low)),
    |x: &mut PwmBbmHBbmL, v: Duration<u32, 1, 100_000_000>| {
        x.low = v.ticks().try_into().expect("bbm to high");
        x.high = v.ticks().try_into().expect("bbm to high");
    },
    fugit::Duration<u32, 1, 100_000_000>,
    single
);
field_impl!(
    pwm_mode,
    |x: PwmSvChop| x.chop,
    |x: &mut PwmSvChop, v| x.chop = v,
    PwmChopperMode
);
field_impl!(
    decoder_ppr,
    |x: AbnDecoderPpr| x.ppr,
    |x: &mut AbnDecoderPpr, v| x.ppr = v,
    u32,
    single
);
field_impl!(
    decoder_direction,
    |x: AbnDecoderMode| x.direction,
    |x: &mut AbnDecoderMode, v| x.direction = v,
    Direction
);
field_impl!(
    openloop_speed,
    |x: OpenloopVelocityTarget| x.velocity,
    |x: &mut OpenloopVelocityTarget, v| x.velocity = v,
    i32,
    single
);
field_impl!(
    phi_e_selection,
    |x: PhiESelection| x.phi_e_selection,
    |x: &mut PhiESelection, v| x.phi_e_selection = v,
    PhiESelectionType,
    single
);
field_impl!(
    phi_e_ext,
    |x: PhiEExt| x.phi_e,
    |x: &mut PhiEExt, v| x.phi_e = v,
    i16,
    single
);
field_impl!(
    openloop_torque_flux,
    |x: UqUdExt| (x.uq, x.ud),
    |x: &mut UqUdExt, v: (i16, i16)| {
        x.uq = v.0;
        x.ud = v.1;
    },
    (i16, i16),
    single
);
field_impl!(
    decoder_count,
    |x: AbnDecoderCount| x.count,
    |x: &mut AbnDecoderCount, v| x.count = v,
    u32,
    single
);
field_impl!(decoder_phi_e, |x: AbnDecoderPhiEPhiM| x.phi_e, i16);

impl<S> Controller<S>
where
    S: SpiDevice,
{
    async fn write(
        &mut self,
        command: impl TMC4671WriteCommand + Send,
    ) -> Result<(), Error<S::Error>> {
        trace!("sending packet to tmc4671");
        let bytes = command.serialize_write();
        self.device.write(&bytes).await.map_err(Error::Spi)
    }

    async fn read<C: TMC4671Command>(&mut self) -> Result<C, Error<S::Error>> {
        let byte = C::serialize_read();
        let mut buffer = [byte, 0, 0, 0, 0];
        self.device
            .transfer_in_place(&mut buffer)
            .await
            .map_err(Error::Spi)?;
        let res = C::deserialize(buffer[1..].try_into().expect("size is 4"))
            .map_err(Error::Deserialization);
        trace!("read packet from tmc4671");
        res
    }
}
