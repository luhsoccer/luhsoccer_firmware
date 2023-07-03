#[cfg(not(test))]
use defmt::info;
use defmt::Format;
use embedded_hal::{blocking::delay::DelayMs, digital::v2::OutputPin, spi::FullDuplex};
use fugit::Duration;
#[cfg(test)]
use log::info;
use nb::block;
use paste::paste;

#[allow(clippy::wildcard_imports)]
use crate::commands::*;

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum Error<S, P> {
    Spi(S),
    Pin(P),
    Deserialization(crate::commands::Error),
    InvalidChipselect,
    CalibrationValidation,
}

impl<S, P> From<crate::commands::Error> for Error<S, P> {
    fn from(err: crate::commands::Error) -> Self {
        Self::Deserialization(err)
    }
}

type Result<R, S, P> =
    core::result::Result<R, Error<<S as FullDuplex<u8>>::Error, <P as OutputPin>::Error>>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Controller<S, P, const N: usize>
where
    S: FullDuplex<u8>,
    P: OutputPin,
{
    spi: S,
    chipselects: [P; N],
}

impl<S, P, const N: usize> Controller<S, P, N>
where
    S: FullDuplex<u8>,
    P: OutputPin,
{
    /// Creates a new motorcontroller controlling N motors over SPI
    ///
    /// # Errors
    ///
    /// This function will return an error if it is unable to set all chipselects high
    pub fn new(spi: S, chipselects: [P; N]) -> Result<Self, S, P> {
        let mut s = Self { spi, chipselects };
        s.disable_all()?;
        Ok(s)
    }

    /// Calibrate encoders of all motors by turning them to a known position. After Initialization
    /// the Motors are turned to a second known position and checked if they are at that position
    /// with a given wiggle room
    ///
    /// # Errors
    ///
    /// This function will return an error if an SPI or Pin error occured, or the Motors did not
    /// end at the second position.
    pub fn calibrate_encoder_all(
        &mut self,
        force: i16,
        delay: &mut impl DelayMs<i32>,
        wiggle_room: u8,
    ) -> Result<(), S, P> {
        self.set_openloop_speed_all(0)?;
        self.set_phi_e_ext_all(0)?;
        self.set_phi_e_selection_all(PhiESelectionType::PhiEExt)?;
        self.set_openloop_torque_flux_all((0, force))?;
        let result = self.calibrate_encoder_all_try(delay, wiggle_room);
        self.set_mode_all(ModeMotion::Stopped)?;
        self.set_openloop_torque_flux_all((0, 0))?;
        result
    }

    fn calibrate_encoder_all_try(
        &mut self,
        delay: &mut impl DelayMs<i32>,
        wiggle_room_deg: u8,
    ) -> Result<(), S, P> {
        #[allow(clippy::cast_possible_wrap)]
        const TEST_POSITION: i16 = (u16::MAX / 3) as i16;

        self.set_mode_all(ModeMotion::UqUdExt)?;
        let modes = self.mode_multi()?;
        info!("modes: {:?}", modes);
        let torque_flux = self.openloop_torque_flux_multi()?;
        info!("torque, flux: {:?}", torque_flux);
        self.wait_still_all(delay)?;
        self.set_decoder_count_all(0)?;

        // rotate motor 120° electrical to check calibration
        let wiggle_room: i16 = i16::MAX / 180 * i16::from(wiggle_room_deg);
        self.set_phi_e_ext_all(TEST_POSITION)?;
        self.wait_still_all(delay)?;
        let encoder_phis = self.decoder_phi_e_multi()?;
        let passed = encoder_phis.iter().all(|phi| {
            ((TEST_POSITION - wiggle_room)..=(TEST_POSITION + wiggle_room)).contains(phi)
        });
        if passed {
            Ok(())
        } else {
            Err(Error::CalibrationValidation)
        }
    }

    fn wait_still_all(&mut self, delay: &mut impl DelayMs<i32>) -> Result<(), S, P> {
        let mut last_encoder_counts = self.decoder_count_multi()?;
        let mut still_counter = 0;
        delay.delay_ms(500);
        info!("waiting for motors to stand still");
        while still_counter < 128 {
            delay.delay_ms(1);
            let encoder_counts = self.decoder_count_multi()?;
            if encoder_counts == last_encoder_counts {
                still_counter += 1;
            } else {
                still_counter = 0;
            }
            last_encoder_counts = encoder_counts;
        }
        info!("waiting another second");
        delay.delay_ms(1000);
        Ok(())
    }

    /// Calibrate the encoder of a motor by turning it to a known position. After Initialization
    /// the Motor is turned to a second known position and checked if it is at that position
    /// with a given wiggle room
    ///
    /// # Errors
    ///
    /// This function will return an error if an SPI or Pin error occured, or the Motor did not
    /// end at the second position.
    pub fn calibrate_encoder(
        &mut self,
        force: i16,
        delay: &mut impl DelayMs<i32>,
        wiggle_room: u8,
        i: usize,
    ) -> Result<(), S, P> {
        info!("initializing encoder for motor {}", i);
        self.set_openloop_speed(0, i)?;
        self.set_phi_e_ext(0, i)?;
        self.set_phi_e_selection(PhiESelectionType::PhiEExt, i)?;
        self.set_openloop_torque_flux((0, force), i)?;
        let result = self.calibrate_encoder_try(delay, wiggle_room, i);
        self.set_mode(ModeMotion::Stopped, i)?;
        self.set_openloop_torque_flux((0, 0), i)?;
        result
    }

    fn calibrate_encoder_try(
        &mut self,
        delay: &mut impl DelayMs<i32>,
        wiggle_room_deg: u8,
        i: usize,
    ) -> Result<(), S, P> {
        #[allow(clippy::cast_possible_wrap)]
        const TEST_POSITION: i16 = (u16::MAX / 3) as i16;

        self.set_mode(ModeMotion::UqUdExt, i)?;
        let mode = self.mode(i)?;
        info!("set mode to {:?}", mode);
        self.wait_still(delay, i)?;
        self.set_decoder_count(0, i)?;

        // rotate motor 120° electrical to check calibration
        let wiggle_room: i16 = i16::MAX / 180 * i16::from(wiggle_room_deg);
        self.set_phi_e_ext(TEST_POSITION, i)?;
        self.wait_still(delay, i)?;
        let encoder_phi = self.decoder_phi_e(i)?;
        if ((TEST_POSITION - wiggle_room)..=(TEST_POSITION + wiggle_room)).contains(&encoder_phi) {
            Ok(())
        } else {
            Err(Error::CalibrationValidation)
        }
    }

    fn wait_still(&mut self, delay: &mut impl DelayMs<i32>, i: usize) -> Result<(), S, P> {
        let mut last_encoder_count = self.decoder_count(i)?;
        let mut still_counter = 0;
        delay.delay_ms(200);
        while still_counter < 128 {
            delay.delay_ms(1);
            let encoder_count = self.decoder_count(i)?;
            if encoder_count == last_encoder_count {
                still_counter += 1;
            } else {
                still_counter = 0;
            }
            last_encoder_count = encoder_count;
        }
        delay.delay_ms(1000);
        Ok(())
    }
}

macro_rules! field_impl {
    ($name:ident, $transform:expr, $result:ty) => {
        /// implements read operations
        /// ``name``: base name for the function
        /// ``transform``: transform to apply to get from the raw command to the output value
        /// ``result``: value returned by the function
        impl<S, P, const N: usize> Controller<S, P, N>
        where
            S: FullDuplex<u8>,
            P: OutputPin,
        {
            /// reads from a motorcontroller
            ///
            /// # Errors
            ///
            /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
            /// deserialized
            pub fn $name(&mut self, i: usize) -> Result<$result, S, P> {
                let x = self.read(i)?;
                Ok($transform(x))
            }

            paste! {
                /// reads from all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
                /// deserialized
                pub fn [<$name _multi>](&mut self) -> Result<[$result; N], S, P> {
                    let xs = self.read_multi()?;
                    Ok(xs.map($transform))
                }
            }
        }
    };
    ($name:ident, $transform:expr, $result:ty, $pre_command:expr) => {
        /// implements read operations for Data registers requiering an matchin Addr register to be
        /// set
        /// ``name``: base name for the function
        /// ``transform``: transform to apply to get from the raw command to the output value
        /// ``result``: value returned by the function
        /// ``pre_command``: command to send for setting the matching Addr register
        impl<S, P, const N: usize> Controller<S, P, N>
        where
            S: FullDuplex<u8>,
            P: OutputPin,
        {
            /// reads from a motorcontroller
            ///
            /// # Errors
            ///
            /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
            /// deserialized
            pub fn $name(&mut self, i: usize) -> Result<$result, S, P> {
                self.send($pre_command, i)?;
                let x = self.read(i)?;
                Ok($transform(x))
            }

            paste! {
                /// reads from all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
                /// deserialized
                pub fn [<$name _multi>](&mut self) -> Result<[$result; N], S, P> {
                    self.send_all($pre_command)?;
                    let xs = self.read_multi()?;
                    Ok(xs.map($transform))
                }
            }
        }
    };
    ($name:ident, $transform:expr, $back_transform:expr, $result:ty) => {
        /// implements send operations
        /// ``name``: base name for the function
        /// ``transform``: transform to apply to get from the raw command to the output value
        /// ``back_transform``: transform to convert from the input value to the raw command given a
        /// raw command.
        /// ``result``: value returned by the function
        impl<S, P, const N: usize> Controller<S, P, N>
        where
            S: FullDuplex<u8>,
            P: OutputPin,
        {
            /// reads from a motorcontroller
            ///
            /// # Errors
            ///
            /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
            /// deserialized
            pub fn $name(&mut self, i: usize) -> Result<$result, S, P> {
                let x = self.read(i)?;
                Ok($transform(x))
            }

            paste! {
                /// reads from all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
                /// deserialized
                pub fn [<$name _multi>](&mut self) -> Result<[$result; N], S, P> {
                    let xs = self.read_multi()?;
                    Ok(xs.map($transform))
                }

                /// Set property for a motorcontroller
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occures.
                pub fn [<set_ $name>](&mut self, value: $result, i: usize) -> Result<(), S, P> {
                    let mut x = self.read(i)?;
                    $back_transform(&mut x, value);
                    self.send(x, i)
                }

                /// Set property for all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occures
                pub fn [<set_ $name _multi>](&mut self, values: [$result; N]) -> Result<(), S, P> {
                    let mut xs = self.read_multi()?;
                    for i in 0..N {
                        $back_transform(&mut xs[i], values[i]);
                    }
                    self.send_multi(&xs)
                }

                /// Set property to the same value for all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occurs.
                pub fn [<set_ $name _all>](&mut self, value: $result) -> Result<(), S, P> {
                    let mut xs = self.read_multi()?;
                    for i in 0..N {
                        $back_transform(&mut xs[i], value);
                    }
                    self.send_multi(&xs)
                }
            }
        }
    };
    ($name:ident, $transform:expr, $back_transform:expr, $result:ty, single) => {
        /// implements send operations where the whole raw command is set by the input
        /// ``name``: base name for the function
        /// ``transform``: transform to apply to get from the raw command to the output value
        /// ``back_transform``: transform to convert from the input value to the raw command given a
        /// raw command.
        /// ``result``: value returned by the function
        impl<S, P, const N: usize> Controller<S, P, N>
        where
            S: FullDuplex<u8>,
            P: OutputPin,
        {
            /// reads from a motorcontroller
            ///
            /// # Errors
            ///
            /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
            /// deserialized
            pub fn $name(&mut self, i: usize) -> Result<$result, S, P> {
                let x = self.read(i)?;
                Ok($transform(x))
            }

            paste! {
                /// reads from all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occurs, or the read value couldn't be
                /// deserialized
                pub fn [<$name _multi>](&mut self) -> Result<[$result; N], S, P> {
                    let xs = self.read_multi()?;
                    Ok(xs.map($transform))
                }

                /// Set property for a motorcontroller
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occures.
                pub fn [<set_ $name>](&mut self, value: $result, i: usize) -> Result<(), S, P> {
                    let mut x = Default::default();
                    $back_transform(&mut x, value);
                    self.send(x, i)
                }

                /// Set property for all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occures
                pub fn [<set_ $name _multi>](&mut self, values: [$result; N]) -> Result<(), S, P> {
                    let mut xs = [Default::default(); N];
                    for i in 0..N {
                        $back_transform(&mut xs[i], values[i]);
                    }
                    self.send_multi(&xs)
                }

                /// Set property to the same value for all motorcontrollers
                ///
                /// # Errors
                ///
                /// Returns an Error if an SPI or Pin error occurs.
                pub fn [<set_ $name _all>](&mut self, value: $result) -> Result<(), S, P> {
                    let mut x = Default::default();
                    $back_transform(&mut x, value);
                    self.send_all(x)
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

impl<S, P, const N: usize> Controller<S, P, N>
where
    S: FullDuplex<u8>,
    P: OutputPin,
{
    fn send(&mut self, command: impl TMC4671WriteCommand, i: usize) -> Result<(), S, P> {
        let bytes = command.serialize_write();
        self.enable(i)?;
        for byte in bytes {
            self.send_byte(byte)?;
        }
        for _ in bytes {
            self.read_byte()?;
        }
        self.disable(i)
    }

    fn send_multi(&mut self, commands: &[impl TMC4671WriteCommand; N]) -> Result<(), S, P> {
        let mut bytes = commands[0].serialize_write();
        for i in 0..N {
            self.enable(i)?;
            for byte in bytes {
                self.send_byte(byte)?;
            }
            if i < N - 1 {
                bytes = commands[i + 1].serialize_write();
            }
            for _ in bytes {
                self.read_byte()?;
            }
            self.disable(i)?;
        }
        Ok(())
    }

    fn send_all(&mut self, command: impl TMC4671WriteCommand + Copy) -> Result<(), S, P> {
        let bytes = command.serialize_write();
        self.enable_all()?;
        for byte in bytes {
            self.send_byte(byte)?;
        }
        for _ in bytes {
            self.read_byte()?;
        }
        self.disable_all()
    }

    fn read<C>(&mut self, i: usize) -> Result<C, S, P>
    where
        C: TMC4671Command,
    {
        let addr = C::serialize_read();
        let mut buf = [0; 4];
        self.enable(i)?;
        self.send_byte(addr)?;
        self.read_byte()?;

        // when writing to the TMC4671 the controller needs a pause of 500ns between the address
        // and the data. This is only needed at > 2MHz but sadly a bit of specific code for the
        // RP2040 at this point.
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();

        for _ in &buf {
            self.send_byte(0)?;
        }
        for byte in &mut buf {
            *byte = self.read_byte()?;
        }
        self.disable(i)?;
        Ok(C::deserialize(buf)?)
    }

    fn read_multi<C>(&mut self) -> Result<[C; N], S, P>
    where
        C: TMC4671Command + Copy + Default,
    {
        let addr = C::serialize_read();
        let mut buf = [0; 4];
        let mut result = [Default::default(); N];
        for i in 0..N {
            self.enable(i)?;
            self.send_byte(addr)?;
            self.read_byte()?;

            // when writing to the TMC4671 the controller needs a pause of 500ns between the address
            // and the data. This is only needed at > 2MHz but sadly a bit of specific code for the
            // RP2040 at this point.
            cortex_m::asm::nop();
            cortex_m::asm::nop();
            cortex_m::asm::nop();
            cortex_m::asm::nop();

            for _ in buf {
                self.send_byte(0)?;
            }
            if i > 0 {
                result[i - 1] = C::deserialize(buf)?;
            }
            for byte in &mut buf {
                *byte = self.read_byte()?;
            }
            self.disable(i)?;
        }
        result[N - 1] = C::deserialize(buf)?;
        // SAFETY: All the values have been set above
        Ok(result)
    }

    fn send_byte(&mut self, byte: u8) -> Result<(), S, P> {
        block!(self.spi.send(byte)).map_err(Error::Spi)
    }

    fn read_byte(&mut self) -> Result<u8, S, P> {
        block!(self.spi.read()).map_err(Error::Spi)
    }

    fn enable(&mut self, i: usize) -> Result<(), S, P> {
        self.chipselects
            .get_mut(i)
            .ok_or(Error::InvalidChipselect)
            .and_then(|cs| cs.set_low().map_err(Error::Pin))
    }

    fn disable(&mut self, i: usize) -> Result<(), S, P> {
        self.chipselects
            .get_mut(i)
            .ok_or(Error::InvalidChipselect)
            .and_then(|cs| cs.set_high().map_err(Error::Pin))
    }

    fn enable_all(&mut self) -> Result<(), S, P> {
        for cs in &mut self.chipselects {
            cs.set_low().map_err(Error::Pin)?;
        }
        Ok(())
    }

    fn disable_all(&mut self) -> Result<(), S, P> {
        for cs in &mut self.chipselects {
            cs.set_high().map_err(Error::Pin)?;
        }
        Ok(())
    }
}
