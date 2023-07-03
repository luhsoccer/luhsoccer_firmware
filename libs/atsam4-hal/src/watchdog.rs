use crate::pac::WDT;
use hal::watchdog;
pub use hal::watchdog::{WatchdogDisable, WatchdogEnable};

pub struct Watchdog {
    wdt: WDT,
}

impl Watchdog {
    pub fn new(wdt: WDT) -> Self {
        Self { wdt }
    }
}

impl watchdog::Watchdog for Watchdog {
    /// Feeds an existing watchdog to ensure the processor isn't reset.
    /// Sometimes commonly referred to as "kicking" or "refreshing".
    fn feed(&mut self) {
        unsafe {
            self.wdt
                .cr
                .write_with_zero(|w| w.key().passwd().wdrstt().set_bit());
        }
    }
}

/// Disables a running watchdog timer so the processor won't be reset.
/// This register can only be written once, so you must make a choice betting disabling or enabling.
/// e.g. If a bootloader enables the Watchdog, this will be a no-op
impl watchdog::WatchdogDisable for Watchdog {
    fn disable(&mut self) {
        // Disable the watchdog timer.
        self.wdt.mr.modify(|_, w| w.wddis().set_bit());
    }
}

impl watchdog::WatchdogEnable for Watchdog {
    type Time = u16;

    /// Enables a watchdog timer to reset the processor if software is frozen
    /// or stalled.
    /// This register can only be written once, so you must make a choice betting disabling or enabling.
    /// period sets the WDD register
    /// Do not call this within 3 slow clock cycles of reseting the MCU.
    /// TODO: Use board frequency to calculate us for parameter?
    fn start<T>(&mut self, period: T)
    where
        T: Into<Self::Time>,
    {
        // Enable watchdog
        self.wdt.mr.modify(|_, w| unsafe {
            w.wdrsten()
                .set_bit()
                .wdv()
                .bits(0x0fff)
                .wdd()
                .bits(period.into())
        });
    }
}
