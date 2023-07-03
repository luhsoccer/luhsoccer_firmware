use crate::hal::timer::{CountDown, Periodic};
use crate::BorrowUnchecked;
use core::marker::PhantomData;
use cortex_m::{interrupt, peripheral::DWT};
use fugit::{
    HertzU32 as Hertz, RateExtU32, TimerDurationU32 as TimerDuration, TimerRateU32 as TimerRate,
};
use void::Void;

use crate::pac::TC0;
#[cfg(any(feature = "atsam4e_e", feature = "atsam4n_c", feature = "atsam4s_c"))]
use crate::pac::TC1;
#[cfg(feature = "atsam4e_e")]
use crate::pac::TC2;

use crate::clock::{Enabled, Tc0Clock, Tc1Clock, Tc2Clock};
#[cfg(any(feature = "atsam4e_e", feature = "atsam4n_c", feature = "atsam4s_c"))]
use crate::clock::{Tc3Clock, Tc4Clock, Tc5Clock};
#[cfg(feature = "atsam4e_e")]
use crate::clock::{Tc6Clock, Tc7Clock, Tc8Clock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum ClockSource {
    MckDiv2 = 0,
    MckDiv8 = 1,
    MckDiv32 = 2,
    MckDiv128 = 3,
    Slck32768Hz = 4,
}

impl ClockSource {
    /// Determine divider using ClockSource
    pub const fn div(&self) -> u32 {
        match self {
            ClockSource::MckDiv2 => 2,
            ClockSource::MckDiv8 => 8,
            ClockSource::MckDiv32 => 32,
            ClockSource::MckDiv128 => 128,
            ClockSource::Slck32768Hz => {
                panic!("Invalid, must set frequency manually");
            }
        }
    }
}

/// Hardware timers for atsam4 can be 16 or 32-bit
/// depending on the hardware..
/// It is also possible to chain TC (timer channels)
/// within a Timer Module to create larger timer
/// registers (not currently implemented in this hal).
/// TimerCounter implements both the `Periodic` and
/// the `CountDown` embedded_hal timer traits.
/// Before a hardware timer can be used, it must first
/// have a clock configured.
pub struct TimerCounter<TC> {
    _tc: TC,
}

pub struct TimerCounterChannels<
    TC,
    CLK1,
    CLK2,
    CLK3,
    const FREQ1: u32,
    const FREQ2: u32,
    const FREQ3: u32,
> {
    pub ch0: TimerCounterChannel<TC, CLK1, 0, FREQ1>,
    pub ch1: TimerCounterChannel<TC, CLK2, 1, FREQ2>,
    pub ch2: TimerCounterChannel<TC, CLK3, 2, FREQ3>,
}

pub struct TimerCounterChannel<TC, CLK, const CH: u8, const FREQ: u32> {
    freq: Hertz,
    source: ClockSource,
    _clock: PhantomData<CLK>,
    _mode: PhantomData<TC>,
}

macro_rules! tc {
    ($($TYPE:ident: ($TC:ident, $clock1:ident, $clock2:ident, $clock3:ident),)+) => {
        $(
pub type $TYPE = TimerCounter<$TC>;

impl TimerCounter<$TC>
{
    /// Configure this timer counter block.
    /// Each TC block has 3 channels
    /// The clock is obtained from the `ClockController` instance
    /// and its frequency impacts the resolution and maximum range of
    /// the timeout values that can be passed to the `start` method.
    ///
    /// Example
    /// ```
    /// let clocks = ClockController::new(
    ///     cx.device.PMC,
    ///     &cx.device.SUPC,
    ///     &cx.device.EFC0,
    ///     MainClock::Crystal12Mhz,
    ///     SlowClock::RcOscillator32Khz,
    /// );
    ///
    /// let mut tc0 = TimerCounter::new(TC0);
    /// let tc0_chs = tc0.split(
    ///     clocks.peripheral_clocks.tc_0.into_enabled_clock(),
    ///     clocks.peripheral_clocks.tc_1.into_enabled_clock(),
    ///     clocks.peripheral_clocks.tc_2.into_enabled_clock(),
    /// );
    ///
    /// let mut tcc0 = tc0_chs.ch0;
    /// tcc0.clock_input(ClockSource::Slck32768Hz);
    /// tcc0.start(500_u32.millis());
    /// while !tcc0.wait().is_ok() {}
    ///
    /// let mut tcc1 = tc0_chs.ch1;
    /// tcc1.clock_input(ClockSource::MckDiv2);
    /// tcc1.start(17_u32.nanos()); // Assuming MCK is 120 MHz or faster
    /// while !tcc1.wait().is_ok() {}
    /// ```
    pub fn new(tc: $TC) -> Self {
        unsafe {
        // Disable write-protect mode
        tc.wpmr.write_with_zero(|w| w.wpkey().passwd().wpen().clear_bit());

        // Disable timer channels while reconfiguring
        tc.ccr0.write_with_zero(|w| w.clkdis().set_bit());
        tc.ccr1.write_with_zero(|w| w.clkdis().set_bit());
        tc.ccr2.write_with_zero(|w| w.clkdis().set_bit());
        }

        Self {
            _tc: tc,
        }
    }

    /// Splits the TimerCounter module into 3 channels
    /// Defaults to MckDiv2 clock source
    pub fn split<const FREQ1: u32, const FREQ2: u32, const FREQ3: u32>(self, clock1: $clock1<Enabled>, _clock2: $clock2<Enabled>, _clock3: $clock3<Enabled>) -> TimerCounterChannels<$TC, $clock1<Enabled>, $clock2<Enabled>, $clock3<Enabled>, FREQ1, FREQ2, FREQ3> {
        let freq = clock1.frequency();
        let source = ClockSource::MckDiv2;
        TimerCounterChannels::<$TC, $clock1<Enabled>, $clock2<Enabled>, $clock3<Enabled>, FREQ1, FREQ2, FREQ3> {
            ch0: TimerCounterChannel { _clock: PhantomData, freq, source, _mode: PhantomData },
            ch1: TimerCounterChannel { _clock: PhantomData, freq, source, _mode: PhantomData },
            ch2: TimerCounterChannel { _clock: PhantomData, freq, source, _mode: PhantomData },
        }
    }
}

impl<CLK, const CH: u8, const FREQ: u32> TimerCounterChannel<$TC, CLK, CH, FREQ> {
    /// Set the input clock
    pub fn clock_input(&mut self, source: ClockSource) {
        self.source = source;

        // Setup divider
        match CH {
            0 => $TC::borrow_unchecked(|tc| tc.cmr0().modify(|_, w| w.tcclks().bits(source as u8))),
            1 => $TC::borrow_unchecked(|tc| tc.cmr1().modify(|_, w| w.tcclks().bits(source as u8))),
            2 => $TC::borrow_unchecked(|tc| tc.cmr2().modify(|_, w| w.tcclks().bits(source as u8))),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }
    }

    /// Enable the interrupt for this TimerCounterChannel
    /// NOTE: The interrupt used will be TC * 3 + CH
    ///       e.g. TC:1 CH:2 => 1 * 3 + 2 = 5
    pub fn enable_interrupt(&mut self) {
        match CH {
            0 => $TC::borrow_unchecked(|tc| unsafe { tc.ier0.write_with_zero(|w| w.cpcs().set_bit())}),
            1 => $TC::borrow_unchecked(|tc| unsafe { tc.ier1.write_with_zero(|w| w.cpcs().set_bit())}),
            2 => $TC::borrow_unchecked(|tc| unsafe { tc.ier2.write_with_zero(|w| w.cpcs().set_bit())}),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }
    }

    /// Disables the interrupt for this TimerCounterChannel
    pub fn disable_interrupt(&mut self) {
        match CH {
            0 => $TC::borrow_unchecked(|tc| unsafe { tc.idr0.write_with_zero(|w| w.cpcs().set_bit())}),
            1 => $TC::borrow_unchecked(|tc| unsafe { tc.idr1.write_with_zero(|w| w.cpcs().set_bit())}),
            2 => $TC::borrow_unchecked(|tc| unsafe { tc.idr2.write_with_zero(|w| w.cpcs().set_bit())}),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }
    }

    /// Clear interrupt status
    pub fn clear_interrupt_flags(&mut self) -> bool {
        match CH {
            0 => $TC::borrow_unchecked(|tc| tc.sr0.read().cpcs().bit()),
            1 => $TC::borrow_unchecked(|tc| tc.sr1.read().cpcs().bit()),
            2 => $TC::borrow_unchecked(|tc| tc.sr2.read().cpcs().bit()),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }
    }
}
impl<CLK, const CH: u8, const FREQ: u32> Periodic for TimerCounterChannel<$TC, CLK, CH, FREQ> {}
impl<CLK, const CH: u8, const FREQ: u32> CountDown for TimerCounterChannel<$TC, CLK, CH, FREQ> {
    type Time = TimerDuration<FREQ>;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Self::Time>,
    {
        // Determine the cycle count
        let timeout: TimerDuration<FREQ> = timeout.into();
        let rate: Hertz = timeout.into_rate();

        // Make sure the divider is set correctly for the given frequency
        let freq: TimerRate<FREQ> = FREQ.Hz();
        match self.source {
            ClockSource::MckDiv2 => {
                let div_freq = self.freq / 2;
                assert_eq!(freq, div_freq, "FREQ({}) != self.freq / 2 ({})", freq, div_freq);
            }
            ClockSource::MckDiv8 => {
                let div_freq = self.freq / 8;
                assert_eq!(freq, div_freq, "FREQ({}) != self.freq / 8 ({})", freq, div_freq);
            }
            ClockSource::MckDiv32 => {
                let div_freq = self.freq / 32;
                assert_eq!(freq, div_freq, "FREQ({}) != self.freq / 32 ({})", freq, div_freq);
            }
            ClockSource::MckDiv128 => {
                let div_freq = self.freq / 128;
                assert_eq!(freq, div_freq, "FREQ({}) != self.freq / 128 ({})", freq, div_freq);
            }
            ClockSource::Slck32768Hz => {
                let div_freq = 32768_u32.Hz::<1, 1>();
                assert_eq!(freq, div_freq, "FREQ({}) != {}", freq, div_freq);
            }
        }

        // Check if timeout is too fast
        if rate > freq {
            panic!("{} is too fast. Max {}", rate, freq);
        }

        // atsam4e supports 32-bits clock timers
        #[cfg(feature = "atsam4e")]
        let max_counter = u32::max_value();
        // atsam4n and atsam4s support 16-bit clock timers
        #[cfg(any(feature = "atsam4n", feature = "atsam4s"))]
        let max_counter: u32 = u16::max_value() as u32;

        // Compute cycles
        let cycles = freq / rate;

        // Check if timeout too slow
        if cycles > max_counter.into() {
            let min_freq: TimerRate<FREQ> = freq / max_counter;
            panic!("{} Hz is too slow. Min {} Hz.", rate, min_freq);
        }

        defmt::trace!("{}->{} Cycles:{} ClockSource:{}", core::stringify!($TC), CH, cycles, self.source);

        // Setup divider
        match CH {
            0 => $TC::borrow_unchecked(|tc| tc.cmr0().modify(|_, w| w.tcclks().bits(self.source as u8).cpctrg().set_bit())),
            1 => $TC::borrow_unchecked(|tc| tc.cmr1().modify(|_, w| w.tcclks().bits(self.source as u8).cpctrg().set_bit())),
            2 => $TC::borrow_unchecked(|tc| tc.cmr2().modify(|_, w| w.tcclks().bits(self.source as u8).cpctrg().set_bit())),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }

        // Setup count-down value
        match CH {
            0 => $TC::borrow_unchecked(|tc| unsafe { tc.rc0.write_with_zero(|w| w.rc().bits(cycles) )}),
            1 => $TC::borrow_unchecked(|tc| unsafe { tc.rc1.write_with_zero(|w| w.rc().bits(cycles) )}),
            2 => $TC::borrow_unchecked(|tc| unsafe { tc.rc2.write_with_zero(|w| w.rc().bits(cycles) )}),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }

        // Clear the interrupt status
        self.clear_interrupt_flags();

        // Enable timer and start using software trigger
        match CH {
            0 => $TC::borrow_unchecked(|tc| unsafe { tc.ccr0.write_with_zero(|w| w.clken().set_bit().swtrg().set_bit())}),
            1 => $TC::borrow_unchecked(|tc| unsafe { tc.ccr1.write_with_zero(|w| w.clken().set_bit().swtrg().set_bit())}),
            2 => $TC::borrow_unchecked(|tc| unsafe { tc.ccr2.write_with_zero(|w| w.clken().set_bit().swtrg().set_bit())}),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        }
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if match CH {
            0 => $TC::borrow_unchecked(|tc| tc.sr0.read().cpcs().bit()),
            1 => $TC::borrow_unchecked(|tc| tc.sr1.read().cpcs().bit()),
            2 => $TC::borrow_unchecked(|tc| tc.sr2.read().cpcs().bit()),
            _ => panic!("Invalid TimerCounterChannel: {}", CH),
        } {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}
        )+
    }
}

tc! {
    TimerCounter0: (TC0, Tc0Clock, Tc1Clock, Tc2Clock),
}

#[cfg(any(feature = "atsam4e_e", feature = "atsam4n_c", feature = "atsam4s_c"))]
tc! {
    TimerCounter1: (TC1, Tc3Clock, Tc4Clock, Tc5Clock),
}

#[cfg(feature = "atsam4e_e")]
tc! {
    TimerCounter2: (TC2, Tc6Clock, Tc7Clock, Tc8Clock),
}

// Adapted from https://github.com/BlackbirdHQ/atat/blob/master/atat/examples/common/timer.rs
pub struct DwtTimer<const TIMER_HZ: u32> {
    end_time: Option<fugit::TimerInstantU32<TIMER_HZ>>,
}

impl<const TIMER_HZ: u32> DwtTimer<TIMER_HZ> {
    pub fn new() -> Self {
        Self { end_time: None }
    }

    pub fn now() -> u64 {
        static mut DWT_OVERFLOWS: u32 = 0;
        static mut OLD_DWT: u32 = 0;

        interrupt::free(|_| {
            // Safety: These static mut variables are accessed in an interrupt free section.
            let (overflows, last_cnt) = unsafe { (&mut DWT_OVERFLOWS, &mut OLD_DWT) };

            let cyccnt = DWT::cycle_count();

            if cyccnt <= *last_cnt {
                *overflows += 1;
            }

            let ticks = (*overflows as u64) << 32 | (cyccnt as u64);
            *last_cnt = cyccnt;

            ticks
        })
    }
}

impl<const TIMER_HZ: u32> Default for DwtTimer<TIMER_HZ> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const TIMER_HZ: u32> fugit_timer::Timer<TIMER_HZ> for DwtTimer<TIMER_HZ> {
    type Error = core::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        fugit::TimerInstantU32::from_ticks(Self::now() as u32)
    }

    fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        let end = self.now() + duration;
        self.end_time.replace(end);
        Ok(())
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.end_time.take();
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        let now = self.now();
        match self.end_time {
            Some(end) if end <= now => Ok(()),
            _ => Err(nb::Error::WouldBlock),
        }
    }
}
