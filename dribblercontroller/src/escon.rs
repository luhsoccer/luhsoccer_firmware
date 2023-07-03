use defmt::{debug, error, info, unwrap};
use embassy_executor::task;
use embassy_futures::select::select;
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    peripherals::{PIN_0, PIN_1, PIN_2, PIN_3, PWM_CH1},
    pwm::{self, Channel, Pwm},
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::v2::OutputPin;
use embedded_hal_async::digital::Wait;
use fixed_macro::types::U12F4;
use sync::observable::Observable;

#[task]
pub async fn escon_task(
    _dio4: PIN_0,
    dio3: PIN_1,
    dio2: PIN_2,
    dio1: PIN_3,
    pwm: PWM_CH1,
    throttle: &'static Observable<CriticalSectionRawMutex, u16, 8>,
) {
    let ready = Output::new(dio2, Level::Low);
    let error = Input::new(dio3, Pull::Up);
    let pwm = Pwm::new_output_b(pwm, dio1, pwm::Config::default());
    escon(ready, error, pwm, throttle).await;
}

async fn escon<const SUBS: usize>(
    mut ready: impl OutputPin,
    mut error: impl Wait,
    mut pwm: Pwm<'_, impl Channel>,
    throttle: &Observable<impl RawMutex, u16, SUBS>,
) {
    // 10%
    const PWM_LOW: u16 = u16::MAX / 10;
    // 90%
    const PWM_HIGH: u16 = u16::MAX - PWM_LOW;

    let mut config = pwm::Config::default();
    config.divider = U12F4!(4);
    config.enable = true;
    config.compare_a = PWM_LOW;
    config.compare_b = PWM_LOW;
    config.top = u16::MAX;
    pwm.set_config(&config);
    unwrap!(ready.set_high(), "Infallible");

    let mut throttle_sub = unwrap!(throttle.subscriber());

    loop {
        match select(error.wait_for_high(), throttle_sub.next_value()).await {
            embassy_futures::select::Either::First(_) => {
                info!("ESCON has an error. trying to clear it");
                unwrap!(ready.set_low(), "Infallible");
                config.compare_a = PWM_LOW;
                config.compare_b = PWM_LOW;
                pwm.set_config(&config);
                if with_timeout(Duration::from_millis(10), error.wait_for_low())
                    .await
                    .is_err()
                {
                    error!("setting pwm to defined low didn't reset the error");
                }
                info!("error successfully cleared");
                unwrap!(ready.set_high(), "Infallible");
                Timer::after(Duration::from_millis(10)).await;
            }
            embassy_futures::select::Either::Second(value) => {
                let value = unwrap!(u16::try_from(
                    u32::from(value) * u32::from(PWM_HIGH - PWM_LOW) / u32::from(u16::MAX)
                )) + PWM_LOW;
                debug!("setting dribbler to {}", value);
                config.compare_a = value;
                config.compare_b = value;
                pwm.set_config(&config);
            }
        }
    }
}
