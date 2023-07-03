use defmt::info;
use embassy_executor::task;
use embassy_rp::{peripherals::WATCHDOG, watchdog::Watchdog};
use embassy_time::{Duration, Timer};

#[task]
pub async fn watchdog_task(watchdog: WATCHDOG) {
    let mut watchdog = Watchdog::new(watchdog);
    info!("starting watchdog");
    watchdog.start(Duration::from_millis(750));

    loop {
        Timer::after(Duration::from_millis(500)).await;
        watchdog.feed();
    }
}
