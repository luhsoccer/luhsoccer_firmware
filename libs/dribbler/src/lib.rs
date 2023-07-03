#![no_std]
pub mod dshot;

use embedded_hal::PwmPin;

pub trait Dribbler<Speed> {
    fn send(&mut self, speed: Speed);
}

macro_rules! pwm_impl {
    ($($duty: ty)*) => {$(
        impl<P: PwmPin<Duty = $duty>> Dribbler<$duty> for P {
            fn send(&mut self, speed: $duty) {
                let duty = speed / (<$duty>::MAX / self.get_max_duty()) / 2 + self.get_max_duty() / 2;
                self.set_duty(duty);
            }
        })*
    };
}

pwm_impl!(u8 u16 u32 u64);

pub struct PwmDribbler<P> {
    pin: P,
}

impl<P> PwmDribbler<P>
where
    P: PwmPin<Duty = u16>,
{
    /// Creates a new dribbler controller.
    /// The `PwmPin` must be set to a frequency of 500Hz
    pub fn new(mut pin: P) -> Self {
        pin.enable();
        Self { pin }
    }

    pub fn set_speed(&mut self, speed: u16) {
        // needs to be calibrated for every motor and is not even accurate
        self.pin.set_duty(speed);
    }
}

impl<P> Dribbler<u16> for PwmDribbler<P>
where
    P: PwmPin<Duty = u16>,
{
    fn send(&mut self, speed: u16) {
        self.set_speed(speed);
    }
}
