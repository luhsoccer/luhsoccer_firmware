use defmt::{info, unwrap};
use embassy_executor::task;
use embassy_rp::{
    peripherals::{PIN_20, PWM_CH2},
    pwm::{Channel, Config, Pwm},
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
#[cfg(feature = "test_dribbler")]
use embassy_time::{Duration, Timer};
use fixed::types::U12F4;
use sync::observable::Observable;

#[task]
pub async fn dribbler_task(
    pin: PIN_20,
    pwm: PWM_CH2,
    command_speed: &'static Observable<CriticalSectionRawMutex, u16, 8>,
) {
    let pwm = Pwm::new_output_a(pwm, pin, Config::default());

    dribbler(pwm, command_speed).await;
}

const fn u16_to_dutycycle<const MIN: u16, const MAX: u16>(value: u16) -> u16 {
    ((value - u16::MIN) as u32 * (MAX - MIN) as u32 / (u16::MAX - u16::MIN) as u32 + (MIN) as u32)
        as u16
}

async fn dribbler<const SUBS: usize>(
    mut pwm: Pwm<'_, impl Channel>,
    command_speed: &Observable<impl RawMutex, u16, SUBS>,
) {
    const PWM_FREQUENCY: u32 = 500;
    const SYSTEM_FREQUENCY: u32 = 125_000_000;
    const DIVIDER: u32 = 4;
    const TOP: u16 = (SYSTEM_FREQUENCY / (DIVIDER * PWM_FREQUENCY)) as u16;
    const MIN_DUTY_CYCLE: u16 = TOP / 2;
    const MAX_DUTY_CYCLE: u16 = TOP;

    let mut config = Config::default();
    config.divider = U12F4::from_num(DIVIDER);
    config.compare_a = MIN_DUTY_CYCLE;
    config.top = TOP;
    pwm.set_config(&config);

    let mut command_speed_sub = unwrap!(command_speed.subscriber());

    loop {
        let new_speed = command_speed_sub.next_value().await;
        config.compare_a = u16_to_dutycycle::<MIN_DUTY_CYCLE, MAX_DUTY_CYCLE>(new_speed);
        pwm.set_config(&config);
        info!(
            "set new dribbler_speed to {}%",
            u16_to_dutycycle::<0, 100>(new_speed)
        );
    }
}

#[cfg(feature = "test_dribbler")]
#[task]
pub async fn dribbler_test_task(
    speed_signal: &'static Observable<CriticalSectionRawMutex, u16, 8>,
) {
    test_dribbler(speed_signal).await;
}

#[cfg(feature = "test_dribbler")]
pub async fn test_dribbler<const SUBS: usize>(speed: &Observable<impl RawMutex, u16, SUBS>) {
    const MAX: u16 = u16::MAX / 5;
    Timer::after(Duration::from_secs(5)).await;
    loop {
        speed.set(0);
        Timer::after(Duration::from_secs(5)).await;
        for i in (0..=MAX).step_by(usize::from(MAX) / 16) {
            speed.set(i);
            Timer::after(Duration::from_secs(1)).await;
        }
        for i in (0..=MAX).step_by(usize::from(MAX) / 16).rev() {
            speed.set(i);
            Timer::after(Duration::from_secs(1)).await;
        }
        for _ in 0..10 {
            speed.set(MAX);
            Timer::after(Duration::from_secs(1)).await;
            speed.set(0);
            Timer::after(Duration::from_secs(1)).await;
        }
        Timer::after(Duration::from_secs(10)).await;
    }
}
