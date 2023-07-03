use defmt::{debug, error, info, unwrap, warn};
use embassy_executor::task;
use embassy_rp::{
    gpio::{Input, Pull},
    peripherals::{PIN_11, PWM_CH5},
    pwm::{self, Channel, Pwm},
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_time::{with_timeout, Duration};
use embedded_hal_async::digital::Wait;
use fixed::types::U12F4;
use sync::observable::Observable;

#[task]
pub async fn servo_input_task(
    pin: PIN_11,
    pwm: PWM_CH5,
    throttle: &'static Observable<CriticalSectionRawMutex, u16, 8>,
) {
    let input_pin = Input::new(&pin, Pull::Down);
    let pwm = Pwm::new_input(pwm, &pin, pwm::InputMode::Level, pwm::Config::default());
    servo_input(pwm, input_pin, throttle).await;
}

async fn servo_input<const SUBS: usize>(
    mut pwm: Pwm<'_, impl Channel>,
    mut pin: impl Wait,
    throttle: &Observable<impl RawMutex, u16, SUBS>,
) {
    const CLOCK_FREQUENCY: u32 = 125_000_000;
    const DIVIDER: u16 = 4;
    const FIXED_DIVIDER: U12F4 = U12F4::const_from_int(DIVIDER);
    const PWM_FREQUENCY: u32 = CLOCK_FREQUENCY / DIVIDER as u32;
    const LOW_COUNT: u16 = (PWM_FREQUENCY / 1_000) as u16;
    const HIGH_COUNT: u16 = (PWM_FREQUENCY / 500) as u16;

    pwm.set_counter(0);
    let mut config = pwm::Config::default();
    config.enable = true;
    config.divider = FIXED_DIVIDER;
    unwrap!(pin.wait_for_low().await, "Infallible");
    pwm.set_config(&config);

    let mut value = 0;
    loop {
        unwrap!(pin.wait_for_low().await, "Infallible");
        pwm.set_counter(0);
        if with_timeout(Duration::from_millis(200), pin.wait_for_falling_edge())
            .await
            .is_err()
        {
            warn!("No input for more than 200ms. Disabling dribbler");
            throttle.set(0);
            value = 0;
            unwrap!(pin.wait_for_falling_edge().await, "Infallible");
            pwm.set_counter(0);
            continue;
        }
        let counter = pwm.counter();
        if !(LOW_COUNT..HIGH_COUNT).contains(&counter) {
            error!("input out of range {}", counter);
            continue;
        }

        let new_value = (counter - LOW_COUNT) * (u16::MAX / (HIGH_COUNT - LOW_COUNT));
        if within_range(new_value, value, 0xf) {
            throttle.set(new_value);
            info!("new servo signal set {} -> {}", value, new_value);
            value = new_value;
        }

        let microseconds = u32::from(counter) * 1_000 / (PWM_FREQUENCY / 1_000);
        debug!("pwm counter: {}us", microseconds);
    }
}

fn within_range(a: u16, b: u16, tolerance: u16) -> bool {
    let a = i32::from(a);
    let b = i32::from(b);
    let tolerance = i32::from(tolerance);
    !((a - tolerance)..(a + tolerance)).contains(&b)
}
