use crate::hal::timer::{CountDown, Periodic};
use crate::pac::RTT;
use fugit::{ExtU32, TimerDurationU32 as TimerDuration};
use void::Void;

/// RTT (Real-time Timer) can be configured in one of
/// two ways:
/// 1. Use 32.768 kHz (/w 16-bit prescaler) input clock
///    to expire a 32-bit counter. The prescaler has an additional
///    interrupt that can be triggered on incrementing.
///    input clock to expire a 32-bit counter.
/// 2. Use 1 Hz RC clock, 16-bit prescaler is ignored and can be used
///    separately. This requires the RTC module is setup and enabled.
///
/// (1) is independent of (2), except that the 16-bit prescaler is shared.
const SLCK_FREQ: u32 = 32_768;
pub struct RealTimeTimer<const PRESCALER: usize, const RTC1HZ: bool> {
    rtt: RTT,
}

impl<const PRESCALER: usize, const RTC1HZ: bool> Periodic for RealTimeTimer<PRESCALER, RTC1HZ> {}
impl<const PRESCALER: usize, const RTC1HZ: bool> CountDown for RealTimeTimer<PRESCALER, RTC1HZ> {
    // Create a frequency base using the prescaler
    type Time = TimerDuration<SLCK_FREQ>;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Self::Time>,
    {
        // Disable timer during configuration
        self.rtt.mr.modify(|_, w| w.rttdis().set_bit());

        // Check if ALMIEN is set (need to disable, then re-enable)
        let rtt_mr = self.rtt.mr.read();
        let almien = rtt_mr.almien().bit_is_set();
        let rttincien = rtt_mr.rttincien().bit_is_set();
        let timeout: TimerDuration<SLCK_FREQ> = timeout.into();

        // Calculate the prescaler period
        let period: Self::Time = if RTC1HZ {
            // When using RTC1HZ, PRESCALER must be set to 32768
            assert_eq!(
                PRESCALER,
                (u16::MAX / 2) as usize,
                "Prescaler must be set to 32768 for RTC1HZ"
            );
            1.secs()
        } else {
            let slck_duration: TimerDuration<SLCK_FREQ> = TimerDuration::from_ticks(1);
            match PRESCALER {
                0 => slck_duration * 2_u32.pow(16),
                1 | 2 => {
                    panic!("Invalid prescaler");
                }
                _ => slck_duration * PRESCALER as u32,
            }
        };

        // Determine alarm value
        let alarmv = timeout / period;
        defmt::trace!(
            "RTT: timeout:{:?} period:{:?} alarmv:{:?}",
            timeout,
            period,
            alarmv
        );

        // ALMIEN must be disabled when setting a new alarm value
        if almien {
            self.disable_alarm_interrupt();
        }
        if rttincien {
            self.disable_prescaler_interrupt();
        }

        // The alarm value is always alarmv - 1 as RTT_AR is set
        // to 0xFFFF_FFFF on reset
        self.rtt.ar.write(|w| unsafe { w.almv().bits(alarmv) });

        // Re-enable ALMIEN if it was enabled
        if almien {
            self.enable_alarm_interrupt();
        }
        if rttincien {
            self.enable_prescaler_interrupt();
        }

        // Start timer, making sure to start fresh
        // NOTE: This seems to behave better as two calls when prescaler is set to 3
        self.rtt.mr.modify(|_, w| w.rttdis().clear_bit());
        self.rtt.mr.modify(|_, w| w.rttrst().set_bit());
    }

    /// Waits on the 32-bit register alarm flag (ALMS)
    fn wait(&mut self) -> nb::Result<(), Void> {
        // Reading clears the flag, so store it for analysis
        // Double-reading can cause interesting issues where the module
        // doesn't reset the timer correctly.
        let rtt_sr = self.rtt.sr.read();

        // Reading clears the flag
        if rtt_sr.alms().bit_is_set() {
            // Reset the timer (to ensure we're periodic)
            self.rtt.mr.modify(|_, w| w.rttrst().set_bit());
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl<const PRESCALER: usize, const RTC1HZ: bool> RealTimeTimer<PRESCALER, RTC1HZ> {
    /// RTT is simple to initialize as it requires no other setup.
    /// (with the exception of using a 32.768 kHz crystal).
    /// Both the internal RC counters (32.768 kHz and 1 Hz) require
    /// no setup.
    ///
    /// If prescaler is equal to zero, the prescaler period
    /// is equal to 2^16 * SCLK period. If not, the prescaler period
    /// is equal to us_prescaler * SCLK period.
    /// 0         - 2^16 * SCLK
    /// 1, 2      - Forbidden
    /// Otherwise - RTPRES * SLCK
    /// 3 => 32.768 kHz / 3 = 10.92267 kHz (91552.706 ns)
    /// This means our minimum unit of time is ~92 us.
    ///
    /// The maximum amount of time using the minimum unit of time:
    /// 91552.706 ns * 2^32 = 3.932159E14
    ///  393215.9     seconds
    ///    6553.598   minutes
    ///     109.2266  hours
    ///       4.55111 days
    ///
    /// If the RTC1HZ is enabled, a 1 Hz signal is used for the 32-bit
    /// alarm. The prescaler is still active and can be triggered from
    /// the prescaler increment interrupt.
    /// This is a calibrated source and is optimized for 1 Hz (if you don't have
    /// a physical 32.768 Hz crystal).
    ///
    /// ```rust
    /// const PRESCALER: usize = 3;
    /// let mut rtt = RealTimeTimer::<PRESCALER, false>::new(peripherals.RTT);
    /// // Set Wait for 1 second
    /// rtt.start(1_000_000u32.micros());
    /// // Wait for 1 second
    /// while !rtt.wait().is_ok() {}
    /// // Wait for 1 second again
    /// while !rtt.wait().is_ok() {}
    /// ```
    pub fn new(rtt: RTT) -> Self {
        // Compile-time check to make sure the prescaler is not set to 1 or 2
        crate::sealed::not_one_or_two::<PRESCALER>();

        // Compile-time check to make sure prescalar is u16
        crate::sealed::smaller_than_or_eq::<PRESCALER, { u16::MAX as usize }>();

        // Disable timer while reconfiguring and prescaler interrupt before setting RTPRES
        rtt.mr
            .modify(|_, w| w.rttdis().set_bit().rttincien().clear_bit());

        // Set the prescalar, rtc1hz and reset the prescaler
        // NOTE: rtc1hz is write-only on some MCUs
        rtt.mr.modify(|_, w| unsafe {
            w.rtpres()
                .bits(PRESCALER as u16)
                .rtc1hz()
                .bit(RTC1HZ)
                .rttrst()
                .set_bit()
        });

        Self { rtt }
    }

    /// Enable the interrupt generation for the 32-bit register
    /// alarm. This method only sets the clock configuration to
    /// trigger the interrupt; it does not configure the interrupt
    /// controller or define an interrupt handler.
    pub fn enable_alarm_interrupt(&mut self) {
        self.rtt.mr.modify(|_, w| w.almien().set_bit());
    }

    /// Enable the interrupt generation for the 16-bit prescaler
    /// overflow. This method only sets the clock configuration to
    /// trigger the interrupt; it does not configure the interrupt
    /// controller or define an interrupt handler.
    pub fn enable_prescaler_interrupt(&mut self) {
        self.rtt.mr.modify(|_, w| w.rttincien().set_bit());
    }

    /// Disables interrupt generation for the 32-bit register alarm.
    /// This method only sets the clock configuration to prevent
    /// triggering the interrupt; it does not configure the interrupt
    /// controller.
    pub fn disable_alarm_interrupt(&mut self) {
        self.rtt.mr.modify(|_, w| w.almien().clear_bit());
    }

    /// Disables interrupt generation for the 16-bit prescaler overflow.
    /// This method only sets the clock configuration to prevent
    /// triggering the interrupt; it does not configure the interrupt
    /// controller.
    pub fn disable_prescaler_interrupt(&mut self) {
        self.rtt.mr.modify(|_, w| w.rttincien().clear_bit());
    }

    /// Clear interrupt status
    /// This will clear both prescaler and alarm interrupts
    pub fn clear_interrupt_flags(&mut self) {
        let _rtt_sr = self.rtt.sr.read();

        // Reset the timer (to ensure we're periodic)
        self.rtt.mr.modify(|_, w| w.rttrst().set_bit());
    }
}
