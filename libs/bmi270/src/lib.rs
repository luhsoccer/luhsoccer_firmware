#![no_std]

pub mod data;
mod definitions;
mod error;

use crate::data::{Accel, Gyro, Temperature};
use crate::definitions::{Cmd, Register, StatusMessage};
use core::marker::PhantomData;
use defmt::info;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::{FullDuplex, Mode, MODE_0};
use fixed::types::{I16F16, I23F9};
use nb::block;

use crate::error::{Error, Result};

pub const SPI_MODE: Mode = MODE_0;

pub struct Bmi270<S, P, W, D> {
    spi: S,
    cs: P,
    delay: D,
    delay_time: u16,
    word: PhantomData<W>,
    offsets: (I16F16, I16F16),
}

impl<S, P, W, D> Bmi270<S, P, W, D>
where
    S: FullDuplex<W>,
    P: OutputPin,
    W: From<u8>,
    u8: From<W>,
    D: DelayMs<u16> + DelayUs<u16>,
{
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(delay: D, spi: S, cs: P) -> Result<Self, S, W, P> {
        let mut result = Self {
            spi,
            cs,
            delay,
            delay_time: 450,
            word: PhantomData,
            offsets: (I16F16::ZERO, I16F16::ZERO),
        };
        // Set cs high by default
        result.cs.set_high().map_err(Error::ChipSelectPin)?;

        // Dummy read to init spi
        let _ = result.read_register(Register::ChipId)?;

        let chip_ip = result.read_register(Register::ChipId)?;

        if chip_ip != 0x24 {
            return Err(Error::InvalidChipId);
        }
        result.write_register(Register::PwrConf, 0x00)?;
        result.write_register(Register::Cmd, Cmd::SoftReset)?; // Perform soft reset
        result.delay.delay_ms(1000); // Reset the chip. When the was already reset this does nothing
        let _ = result.read_register(Register::ChipId)?;
        result.write_register(Register::PwrConf, 0x00)?;
        result.delay_time = 2;
        result.write_register(Register::InitCtrl, 0x00)?;
        result.burst_config()?;
        result.write_register(Register::InitCtrl, 0x01)?;
        result.delay.delay_ms(100);

        loop {
            let status = result.read_register(Register::InternalStatus)? & 0b1111;
            let status = status.try_into();

            if status == Ok(StatusMessage::InitOk) {
                break;
            }
            info!("Got invalid status: {}. Waiting again", status);
            result.delay.delay_ms(100);
        }

        result.write_register(Register::PwrCtrl, 0x0E)?; // Enable all sensors except the aux interface
        result.write_register(Register::AccConf, 0xA9)?;
        result.write_register(Register::AccRange, 0x01)?;
        result.write_register(Register::GyrConf, 0xE9)?;
        result.write_register(Register::GyrRange, 0x08)?;
        result.write_register(Register::IntMapData, 0x44)?;
        result.write_register(Register::Int1IoCtrl, 0x0A)?;
        result.write_register(Register::Int2IoCtrl, 0x0A)?;
        info!("Successfully init device");

        // calibrate offsets
        /*
        result.write_register(Register::Offset0, 0)?;
        result.write_register(Register::Offset1, 0)?;
        result.delay.delay_ms(10);
        let mut sum_x: i32 = 0;
        let mut sum_y: i32 = 0;
        for _ in 0..1024 {
            let offsets = result.read_accel_data()?;
            sum_x -= i32::from(offsets.x);
            sum_y -= i32::from(offsets.y);
            result.delay.delay_us(1_250);
        }
        sum_x /= 1024 * 32;
        sum_y /= 1024 * 32;
        if let Ok(sum_x) = i8::try_from(sum_x) {
            #[allow(clippy::cast_sign_loss)]
            result.write_register(Register::Offset0, sum_x as u8)?;
        }
        if let Ok(sum_y) = i8::try_from(sum_y) {
            #[allow(clippy::cast_sign_loss)]
            result.write_register(Register::Offset1, sum_y as u8)?;
        }
        result.write_register(Register::NvConf, 0b1001)?;
        */
        const SAMPLES: i32 = 2000;
        let mut sum_x: I16F16 = I16F16::ZERO;
        let mut sum_y: I16F16 = I16F16::ZERO;
        for _ in 0..SAMPLES {
            let offsets = result.read_accel_data()?;
            sum_x += offsets.x;
            sum_y += offsets.y;
            result.delay.delay_us(5_000);
        }
        result.offsets = (sum_x / SAMPLES, sum_y / SAMPLES);
        result.write_register(Register::Features5, 0b010)?;
        result.write_register(Register::Offset6, 0b0100_0000)?;

        Ok(result)
    }

    /// Returns the read temperature of this [`Bmi270<S, P, W, D>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn read_temperature(&mut self) -> Result<Temperature, S, W, P> {
        let data = self.burst_read(Register::Temperature0)?;
        let data = i16::from_le_bytes(data);

        Ok(Temperature(
            I23F9::from_bits(data.into()) + I23F9::unwrapped_from_num(23),
        ))
    }

    /// Returns the read accel data of this [`Bmi270<S, P, W, D>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn read_accel_data(&mut self) -> Result<Accel, S, W, P> {
        let data: [u8; 6] = self.burst_read(Register::Data8)?;

        Ok(Accel::from_bytes(&data, self.offsets))
    }

    /// Returns the read gyro data of this [`Bmi270<S, P, W, D>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn read_gyro_data(&mut self) -> Result<Gyro, S, W, P> {
        let data: [u8; 6] = self.burst_read(Register::Data14)?;

        Ok(Gyro::from_bytes(&data))
    }

    /// Returns the read accel and gyro data of this [`Bmi270<S, P, W, D>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn read_accel_and_gyro_data(&mut self) -> Result<(Accel, Gyro), S, W, P> {
        let data: [u8; 12] = self.burst_read(Register::Data8)?;
        Ok((
            Accel::from_bytes(&data[0..6], self.offsets),
            Gyro::from_bytes(&data[6..12]),
        ))
    }

    fn write_register<T>(&mut self, reg: Register, value: T) -> Result<(), S, W, P>
    where
        T: Into<u8>,
    {
        let reg = reg as u8 & 0b0111_1111;
        let value = value.into();
        self.with_cs(|s| {
            block!(s.spi.send(reg.into())).map_err(Error::Spi)?;
            let _ = block!(s.spi.read()).map_err(Error::Spi)?;
            block!(s.spi.send(value.into())).map_err(Error::Spi)?;
            let _ = block!(s.spi.read()).map_err(Error::Spi)?;

            Ok(())
        })
    }

    fn read_register(&mut self, reg: Register) -> Result<u8, S, W, P> {
        self.with_cs(|s| {
            let reg = reg as u8 | 0b1000_0000;

            block!(s.spi.send(reg.into())).map_err(Error::Spi)?;

            for _ in 0..2 {
                let _ = block!(s.spi.read()).map_err(Error::Spi)?;
                block!(s.spi.send(0x00.into())).map_err(Error::Spi)?;
            }

            let result = block!(s.spi.read()).map_err(Error::Spi)?;

            Ok(result.into())
        })
    }

    fn burst_write(&mut self, reg: Register, data: &[u8]) -> Result<(), S, W, P> {
        let reg = reg as u8;
        self.with_cs(|s| {
            block!(s.spi.send(reg.into())).map_err(Error::Spi)?;
            block!(s.spi.read()).map_err(Error::Spi)?;

            for byte in data {
                block!(s.spi.send((*byte).into())).map_err(Error::Spi)?;
                block!(s.spi.read()).map_err(Error::Spi)?;
            }

            Ok(())
        })
    }

    fn burst_read<const LENGTH: usize>(&mut self, reg: Register) -> Result<[u8; LENGTH], S, W, P> {
        let reg = reg as u8 | 0b1000_0000;
        let mut result = [0u8; LENGTH];
        self.with_cs(|s| {
            block!(s.spi.send(reg.into())).map_err(Error::Spi)?;
            block!(s.spi.read()).map_err(Error::Spi)?;

            block!(s.spi.send(0x00.into())).map_err(Error::Spi)?;
            block!(s.spi.read()).map_err(Error::Spi)?;

            for byte in &mut result {
                block!(s.spi.send(0x00.into())).map_err(Error::Spi)?;
                *byte = block!(s.spi.read()).map_err(Error::Spi)?.into();
            }

            Ok(result)
        })
    }

    fn burst_config(&mut self) -> Result<(), S, W, P> {
        let chunk_size = 1024;
        let chunks = definitions::BMI270_CONFIG_FILE.chunks_exact(chunk_size);
        let chunk_count = chunks.len();

        for (idx, chunk) in chunks.enumerate() {
            info!("Write chunk {}/{} ", idx, chunk_count);
            let address = (idx * chunk_size) / 2;
            let high = address & 0b1111_1111_0000;
            let high = u8::try_from(high >> 4).expect("masked and shifted");
            let low = u8::try_from(address & 0b0000_0000_1111).expect("masked");

            self.write_register(Register::InitAddr1, high)?;
            self.write_register(Register::InitAddr0, low)?;
            self.burst_write(Register::InitData, chunk)?;
        }
        Ok(())
    }

    fn with_cs<F, R>(&mut self, f: F) -> Result<R, S, W, P>
    where
        F: FnOnce(&mut Self) -> Result<R, S, W, P>,
    {
        self.cs.set_low().map_err(Error::ChipSelectPin)?;
        let result = f(self);
        self.cs.set_high().map_err(Error::ChipSelectPin)?;
        self.delay.delay_us(self.delay_time);
        result
    }
}
