#![no_std]

use core::{convert::Infallible, marker::PhantomData};
use defmt::panic;
use embedded_hal::digital::v2::OutputPin;

pub use sealed::Mode;

pub struct SleepMode;
pub struct ReceiveLnaMode;
pub struct TransmitHighPowerMode;
pub struct TransmitLowPowerMode;
pub struct ReceiveBypassMode;
pub struct TransmitBypassMode;
pub struct Undefined;

mod sealed {
    pub trait Mode {}

    impl Mode for crate::SleepMode {}
    impl Mode for crate::ReceiveLnaMode {}
    impl Mode for crate::TransmitHighPowerMode {}
    impl Mode for crate::TransmitLowPowerMode {}
    impl Mode for crate::ReceiveBypassMode {}
    impl Mode for crate::TransmitBypassMode {}
    impl Mode for crate::Undefined {}
}

pub trait HighSettable {
    type Error;

    fn set_high(&mut self) -> Result<(), Self::Error>;
}

impl<T: OutputPin> HighSettable for T {
    type Error = T::Error;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_high(self)
    }
}

pub struct TiedHigh;
impl HighSettable for TiedHigh {
    type Error = Infallible;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub trait LowSettable {
    type Error;

    fn set_low(&mut self) -> Result<(), Self::Error>;
}

impl<T: OutputPin> LowSettable for T {
    type Error = T::Error;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_low(self)
    }
}

pub struct TiedLow;
impl LowSettable for TiedLow {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[allow(non_camel_case_types)]
pub struct Sky66112<M: Mode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
    csd: CSD,
    cps: CPS,
    crx: CRX,
    ctx: CTX,
    chl: CHL,
    ant_sel: ANT_SEL,
    _marker: PhantomData<M>,
}

macro_rules! unwrap {
    ($res: expr) => {
        if $res.is_err() {
            panic!("unable to set pin");
        }
    };
}

#[allow(non_camel_case_types)]
impl<M: Mode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
    fn change_mode<N: Mode>(self) -> Sky66112<N, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        Sky66112 {
            csd: self.csd,
            cps: self.cps,
            crx: self.crx,
            ctx: self.ctx,
            chl: self.chl,
            ant_sel: self.ant_sel,
            _marker: PhantomData,
        }
    }
}

#[allow(non_camel_case_types)]
impl<CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<Undefined, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
    pub fn new(csd: CSD, cps: CPS, crx: CRX, ctx: CTX, chl: CHL, ant_sel: ANT_SEL) -> Self {
        Self {
            csd,
            cps,
            crx,
            ctx,
            chl,
            ant_sel,
            _marker: PhantomData,
        }
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: LowSettable,
{
    pub fn into_sleep_mode(mut self) -> Sky66112<SleepMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_low());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: HighSettable,
    CPS: LowSettable,
    CRX: HighSettable,
    CTX: LowSettable,
{
    pub fn into_receive_lna_mode(
        mut self,
    ) -> Sky66112<ReceiveLnaMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_high());
        unwrap!(self.cps.set_low());
        unwrap!(self.crx.set_high());
        unwrap!(self.ctx.set_low());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: HighSettable,
    CPS: LowSettable,
    CTX: HighSettable,
    CHL: HighSettable,
{
    pub fn into_transmit_high_power_mode(
        mut self,
    ) -> Sky66112<TransmitHighPowerMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_high());
        unwrap!(self.cps.set_low());
        unwrap!(self.ctx.set_high());
        unwrap!(self.chl.set_high());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: HighSettable,
    CPS: LowSettable,
    CTX: HighSettable,
    CHL: LowSettable,
{
    pub fn into_transmit_low_power_mode(
        mut self,
    ) -> Sky66112<TransmitLowPowerMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_high());
        unwrap!(self.cps.set_low());
        unwrap!(self.ctx.set_high());
        unwrap!(self.chl.set_low());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: HighSettable,
    CPS: HighSettable,
    CRX: HighSettable,
    CTX: LowSettable,
{
    pub fn into_receive_bypass_mode(
        mut self,
    ) -> Sky66112<ReceiveBypassMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_high());
        unwrap!(self.cps.set_high());
        unwrap!(self.crx.set_high());
        unwrap!(self.ctx.set_low());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: HighSettable,
    CPS: HighSettable,
    CTX: HighSettable,
{
    pub fn into_transmit_bypass_mode(
        mut self,
    ) -> Sky66112<TransmitBypassMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_high());
        unwrap!(self.cps.set_high());
        unwrap!(self.ctx.set_high());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    CSD: HighSettable,
    CRX: LowSettable,
    CTX: LowSettable,
{
    pub fn into_sleep_mode2(mut self) -> Sky66112<SleepMode, CSD, CPS, CRX, CTX, CHL, ANT_SEL> {
        unwrap!(self.csd.set_high());
        unwrap!(self.crx.set_low());
        unwrap!(self.ctx.set_low());
        self.change_mode()
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    ANT_SEL: LowSettable,
{
    pub fn use_antenna_1(&mut self) {
        unwrap!(self.ant_sel.set_low());
    }
}

#[allow(non_camel_case_types)]
impl<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL> Sky66112<M, CSD, CPS, CRX, CTX, CHL, ANT_SEL>
where
    M: Mode,
    ANT_SEL: HighSettable,
{
    pub fn use_antenna_2(&mut self) {
        unwrap!(self.ant_sel.set_high());
    }
}
