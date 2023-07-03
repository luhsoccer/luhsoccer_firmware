#![no_std]

pub use dac::Dac;
use embedded_hal::{
    adc::{Channel, OneShot},
    digital::v2::OutputPin,
};
use nb::block;
use units::{prelude::*, types::Volt};

pub mod asynch;
pub mod dac;

pub trait Kicker {
    type Error;

    /// Charge the Kicker to a given voltage
    ///
    /// # Errors
    ///
    /// This function will return an error if the kicker is not able to charge
    fn charge(&mut self, setpoint: Volt<u8>) -> Result<(), Self::Error>;

    /// Kick the ball
    ///
    /// # Errors
    ///
    /// This function will return an error if it is not possible to kick
    fn kick(&mut self) -> Result<(), Self::Error>;

    /// Chip the ball
    ///
    /// # Errors
    ///
    /// This function will return an error if it is not possible to chip
    fn chip(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KickerError<D, A, AP, T1, T2, C>
where
    D: Dac,
    A: OneShot<A, u16, AP>,
    AP: Channel<A, ID = u8>,
    T1: OutputPin,
    T2: OutputPin,
    C: OutputPin,
{
    Dac(D::Error),
    Adc(A::Error),
    Kick(T1::Error),
    Chip(T2::Error),
    Clearance(C::Error),
}

pub struct Automatic<D, A, AP, T1, T2, C> {
    dac: D,
    adc: A,
    feedback: AP,
    trig1: T1,
    trig2: T2,
    clear: C,
    setpoint: Volt<u8>,
}

impl<D, A, AP, T1, T2, C> Automatic<D, A, AP, T1, T2, C> {
    /// Creates a new [`Automatic<D, A, AP, T1, T2, C>`]. Charging is controlled using the DAC, ADC
    /// and clear pin. Kicking is done using trig1, chiping is done by trig2.
    pub const fn new(dac: D, adc: A, feedback: AP, trig1: T1, trig2: T2, clear: C) -> Self {
        Self {
            dac,
            adc,
            feedback,
            trig1,
            trig2,
            clear,
            setpoint: Volt::new(0),
        }
    }
}

impl<D, A, AP, T1, T2, C> Automatic<D, A, AP, T1, T2, C>
where
    D: Dac,
    A: OneShot<A, u16, AP>,
    AP: Channel<A, ID = u8>,
    T1: OutputPin,
    T2: OutputPin,
    C: OutputPin,
{
    fn wait_kick(&mut self) -> Result<(), KickerError<D, A, AP, T1, T2, C>> {
        while block!(self.adc.read(&mut self.feedback)).map_err(KickerError::Adc)?
            > 30 * (0xFFF / 300)
        {}
        Ok(())
    }
}

impl<D, A, AP, T1, T2, C> Kicker for Automatic<D, A, AP, T1, T2, C>
where
    D: Dac,
    A: OneShot<A, u16, AP>,
    AP: Channel<A, ID = u8>,
    T1: OutputPin,
    T2: OutputPin,
    C: OutputPin,
{
    type Error = KickerError<D, A, AP, T1, T2, C>;

    fn charge(&mut self, setpoint: Volt<u8>) -> Result<(), Self::Error> {
        // Disable kicker if voltage is set to 0
        if setpoint == 0.V() {
            self.clear.set_low().map_err(KickerError::Clearance)?;
            self.setpoint = setpoint;
            return Ok(());
        }

        // The hardware can only charge the kicker. Discharge in case a lower voltage is requested
        if self.setpoint > setpoint {
            // Remove clearence so the hardware will start discharging through the power resistor
            self.clear.set_low().map_err(KickerError::Clearance)?;
            // Wait for the adc to read a voltage lower than the requested voltage
            while block!(self.adc.read(&mut self.feedback)).map_err(KickerError::Adc)?
                > u16::from(setpoint.raw()) * (0xFFF / 300)
            {}
        }

        // Set requested voltage
        self.dac
            .set(u16::from(setpoint.raw()) * (u16::MAX / 300))
            .map_err(KickerError::Dac)?;

        // Signal the kicker to start charging
        self.clear.set_high().map_err(KickerError::Clearance)?;
        self.setpoint = setpoint;
        Ok(())
    }

    fn kick(&mut self) -> Result<(), Self::Error> {
        // Disable charging so the kicker won't try to charge while kicking
        self.dac.set(0).map_err(KickerError::Dac)?;
        self.trig1.set_high().map_err(KickerError::Kick)?;
        self.wait_kick()?;
        self.trig1.set_low().map_err(KickerError::Kick)
    }

    fn chip(&mut self) -> Result<(), Self::Error> {
        // Disable charging so the kicker won't try to charge while kicking
        self.dac.set(0).map_err(KickerError::Dac)?;
        self.trig2.set_high().map_err(KickerError::Chip)?;
        self.wait_kick()?;
        self.trig2.set_low().map_err(KickerError::Chip)
    }
}
