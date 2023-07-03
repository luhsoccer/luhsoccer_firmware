use defmt::{info, unwrap, warn};
use embassy_executor::task;
use embassy_futures::select::{select3, Either3};
use embassy_rp::{
    adc::Adc,
    gpio::{Input, Level, Output, Pull},
    peripherals::{
        PIN_0, PIN_1, PIN_10, PIN_11, PIN_12, PIN_13, PIN_14, PIN_15, PIN_2, PIN_29, PIN_3, PIN_4,
        PIN_5, PIN_6, PIN_7, PIN_8, PIN_9, PIO0,
    },
    pio::{Common, Instance, StateMachine},
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_time::{Duration, Timer};
use fixed::types::I16F16;
use kicker::asynch::PioDac;
use sync::observable::Observable;
use units::types::Volt;

const CAP_TOP_RESISTANCE: f64 = 4_990_000.0;
const CAP_BOTTOM_RESISTANCE: f64 = 36_500.0;
const CAP_TOTAL_RESISTANCE: f64 = CAP_TOP_RESISTANCE + CAP_BOTTOM_RESISTANCE;
const CAP_GAIN: f64 = CAP_BOTTOM_RESISTANCE / CAP_TOTAL_RESISTANCE;

const DAC_MAX: f64 = 0x03FF as f64;
const DAC_BIT_PER_VOLT: f64 = DAC_MAX / 3.3;
const DAC_BIT_PER_CAP_VOLT: f64 = DAC_BIT_PER_VOLT * CAP_GAIN;
pub(crate) const DAC_230V_POINT: u16 = (DAC_BIT_PER_CAP_VOLT * 230.0) as u16;

const ADC_MAX: f64 = 0x0FFF as f64;
const ADC_BIT_PER_VOLT: f64 = ADC_MAX / 3.3;
const ADC_BIT_PER_CAP_VOLT: f64 = ADC_BIT_PER_VOLT * CAP_GAIN;
pub(crate) const ADC_230V_POINT: u16 = (ADC_BIT_PER_CAP_VOLT * 230.0) as u16;

struct Kicker<'d, PIO: Instance, const SM: usize> {
    triggers: (Output<'d, PIN_0>, Output<'d, PIN_1>),
    _not_fault: Input<'d, PIN_12>,
    _not_done: Input<'d, PIN_13>,
    clear: Output<'d, PIN_14>,
    _charge: Input<'d, PIN_15>,
    dac: PioDac<'d, PIO, SM>,
    _adc: Adc<'d>,
    _cap_voltage: PIN_29,
}

impl<'d, PIO: Instance, const SM: usize> Kicker<'d, PIO, SM> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        triggers: (PIN_0, PIN_1),
        not_fault: PIN_12,
        not_done: PIN_13,
        clear: PIN_14,
        charge: PIN_15,
        cap_voltage: PIN_29,
        dac_pins: (
            PIN_2,
            PIN_3,
            PIN_4,
            PIN_5,
            PIN_6,
            PIN_7,
            PIN_8,
            PIN_9,
            PIN_10,
            PIN_11,
        ),
        adc: Adc<'d>,
        sm: StateMachine<'d, PIO, SM>,
        pio: &mut Common<'d, PIO>,
    ) -> Self {
        let dac = PioDac::new(sm, pio, dac_pins);
        let clear = Output::new(clear, Level::Low);
        let triggers = (
            Output::new(triggers.0, Level::Low),
            Output::new(triggers.1, Level::Low),
        );
        let charge = Input::new(charge, Pull::Down);
        let not_fault = Input::new(not_fault, Pull::None);
        let not_done = Input::new(not_done, Pull::None);
        if not_fault.is_low() {
            warn!("Kicker error at creation.");
        }
        Self {
            triggers,
            _not_fault: not_fault,
            _not_done: not_done,
            clear,
            _charge: charge,
            dac,
            _adc: adc,
            _cap_voltage: cap_voltage,
        }
    }

    fn discharge(&mut self) {
        info!("discharging");
        self.dac.set(0);
        self.clear.set_low();
        self.triggers.0.set_low();
        self.triggers.1.set_low();
    }

    async fn kick(&mut self, time: Duration) {
        warn!("kicking!!!");
        self.triggers.0.set_high();
        self.triggers.1.set_high();
        Timer::after(time).await;
        self.triggers.0.set_low();
        self.triggers.1.set_low();
    }

    fn charge(&mut self, voltage: u16) {
        info!("charging");
        self.clear.set_high();
        self.dac.set(voltage);
    }
}

#[task]
#[allow(clippy::too_many_arguments)]
pub async fn kicker_task(
    has_ball: &'static Observable<CriticalSectionRawMutex, bool, 8>,
    set_voltage: &'static Observable<CriticalSectionRawMutex, Volt<u8>, 8>,
    speed: &'static Observable<CriticalSectionRawMutex, u16, 8>,
    kicker_raw_duration: &'static Observable<CriticalSectionRawMutex, Duration, 8>,
    triggers: (PIN_0, PIN_1),
    not_fault: PIN_12,
    not_done: PIN_13,
    clear: PIN_14,
    charge: PIN_15,
    cap_voltage: PIN_29,
    dac_pins: (
        PIN_2,
        PIN_3,
        PIN_4,
        PIN_5,
        PIN_6,
        PIN_7,
        PIN_8,
        PIN_9,
        PIN_10,
        PIN_11,
    ),
    adc: Adc<'static>,
    sm: StateMachine<'static, PIO0, 0>,
    mut pio: Common<'static, PIO0>,
    config: &'static crate::Config<CriticalSectionRawMutex>,
) {
    let kicker_obj = Kicker::new(
        triggers,
        not_fault,
        not_done,
        clear,
        charge,
        cap_voltage,
        dac_pins,
        adc,
        sm,
        &mut pio,
    );
    kicker(
        has_ball,
        set_voltage,
        speed,
        kicker_raw_duration,
        kicker_obj,
        config,
    )
    .await;
    warn!("unexpected return of kicker loop.");
}

async fn kicker<
    const SUBS1: usize,
    const SUBS2: usize,
    const SUBS3: usize,
    const SUBS4: usize,
    const SM: usize,
>(
    has_ball: &Observable<impl RawMutex, bool, SUBS1>,
    set_voltage: &Observable<impl RawMutex, Volt<u8>, SUBS2>,
    speed: &Observable<impl RawMutex, u16, SUBS3>,
    kicker_raw_duration: &Observable<impl RawMutex, Duration, SUBS4>,
    mut kicker: Kicker<'_, impl Instance, SM>,
    config: &crate::Config<impl RawMutex>,
) {
    let mut has_ball_sub = unwrap!(has_ball.subscriber());
    let mut set_voltage_sub = unwrap!(set_voltage.subscriber());
    let mut speed_sub = unwrap!(speed.subscriber());
    loop {
        match select3(
            has_ball_sub.next_value(),
            set_voltage_sub.next_value(),
            speed_sub.next_value(),
        )
        .await
        {
            Either3::First(has_ball) => {
                info!("got ball update {}", has_ball);
                let timing = kicker_raw_duration.get();
                if has_ball && timing != Duration::MIN {
                    kicker.kick(timing).await;
                    kicker_raw_duration.set(Duration::MIN);
                }
            }
            Either3::Second(voltage) if voltage == Volt::new(0) => {
                info!("discharging");
                kicker.discharge();
            }
            Either3::Second(voltage) => {
                info!("setting voltage {}", voltage);
                let value_230v = config.kicker_cap_dac_230v.get();
                let value = unwrap!(u16::try_from(
                    (u32::from(voltage.raw()) * u32::from(value_230v) / 230)
                        .clamp(0, DAC_MAX as u32),
                ));
                kicker.charge(value);
            }
            Either3::Third(speed) => {
                if speed == 0 {
                    info!("commanded to not kick");
                    kicker_raw_duration.set(Duration::MIN);
                } else {
                    let timing = calc_kick_time(speed, config);
                    info!(
                        "commanded to kick with {}mm/s, {}us",
                        speed,
                        timing.as_micros()
                    );
                    if has_ball.get() {
                        info!("we currently have the ball. kicking");
                        kicker.kick(timing).await;
                        kicker_raw_duration.set(Duration::MIN);
                    } else {
                        kicker_raw_duration.set_if_different(timing);
                    }
                }
            }
        }
    }
}

fn calc_kick_time(speed: u16, config: &crate::Config<impl RawMutex>) -> Duration {
    let speed = I16F16::from_num(speed) / 1000;
    let poli4 = config.kicker_poli4.get();
    let poli3 = config.kicker_poli3.get();
    let poli2 = config.kicker_poli2.get();
    let poli1 = config.kicker_poli1.get();
    let poli0 = config.kicker_poli0.get();
    let microseconds = (((poli4 * speed + poli3) * speed + poli2) * speed + poli1) * speed + poli0;
    Duration::from_micros_floor(microseconds.to_num())
}

#[cfg(feature = "test_kicker")]
#[task]
pub async fn kicker_test_task(
    set_voltage: &'static Observable<CriticalSectionRawMutex, Volt<u8>, 8>,
    has_ball: &'static Observable<CriticalSectionRawMutex, bool, 8>,
) {
    kicker_test(has_ball, set_voltage).await;
}

#[cfg(feature = "test_kicker")]
async fn kicker_test<const SUBS1: usize, const SUBS2: usize>(
    has_ball: &Observable<impl RawMutex, bool, SUBS1>,
    set_voltage: &Observable<impl RawMutex, Volt<u8>, SUBS2>,
) {
    loop {
        has_ball.set(false);
        set_voltage.set(Volt::new(0));
        info!("Discharging kicker for 5s");
        Timer::after(Duration::from_secs(5)).await;
        for voltage in (10..=200).step_by(10).map(Volt::new) {
            info!("increasing kicker voltage to {}", voltage);
            set_voltage.set(voltage);
            Timer::after(Duration::from_secs(2)).await;
        }
        for voltage in (0..=190).step_by(10).map(Volt::new).rev() {
            info!("reducing kicker voltage to {}", voltage);
            set_voltage.set(voltage);
            Timer::after(Duration::from_secs(2)).await;
        }
        for voltage in (50..=200).step_by(50).map(Volt::new) {
            info!("kicking with {}", voltage);
            set_voltage.set(voltage);
            Timer::after(Duration::from_secs(2)).await;
            has_ball.set(true);
            Timer::after(Duration::from_millis(1)).await;
            has_ball.set(false);
        }
    }
}
