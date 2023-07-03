use core::ops::RangeInclusive;

use defmt::{debug, error, info, unwrap, warn, Format};
use embassy_executor::task;
use embassy_futures::select::select;
use embassy_rp::{
    adc::Adc,
    gpio::{Input, Level, Output, Pin, Pull},
    peripherals::{ADC, PIN_12, PIN_13, PIN_28, PIN_29},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
    mutex::Mutex,
    signal::Signal,
};
use embassy_time::{Duration, Ticker, Timer};
use embedded_hal::{
    adc::Channel,
    digital::v2::{InputPin, OutputPin},
};
use embedded_hal_async::digital::Wait;
use fixed::types::U16F16;
use fixed_macro::types::U16F16;
use sync::observable::Observable;

#[task]
pub async fn power_switch_task(
    switch: PIN_13,
    not_shutdown: PIN_12,
    shutdown: &'static Signal<CriticalSectionRawMutex, ()>,
) {
    let switch = Input::new(switch, Pull::None);
    let not_shutdown = Output::new(not_shutdown, Level::High);
    power_control(switch, not_shutdown, shutdown).await;
}

async fn power_control(
    mut switch: impl Wait + InputPin,
    mut not_shutdown: impl OutputPin,
    shutdown: &Signal<impl RawMutex, ()>,
) {
    if unwrap!(switch.is_low(), "Infallible") {
        unwrap!(not_shutdown.set_low(), "Infallible");
        info!("The user is not pressing the button. Assuming this is a shutdown");
        return;
    }

    not_shutdown.set_high().ok();
    info!("waiting for power button to be released");
    if switch.wait_for_low().await.is_err() {
        error!("unable to wait for power switch released");
    }
    debug!("power button released");
    Timer::after(Duration::from_millis(100)).await;

    select(
        async {
            if switch.wait_for_high().await.is_err() {
                error!("unable to wait for power switch pressed");
            }
            info!("power button pressed!");
        },
        shutdown.wait(),
    )
    .await;
    if switch.wait_for_low().await.is_err() {
        error!("unable to wait for power switch released");
    }
    info!("shutting down!");
    while not_shutdown.set_low().is_err() {
        error!("cannot turn of robot");
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Format, Clone, Copy)]
pub enum BatteryState {
    Usb,
    Critical,
    Low,
    Nominal,
    Full,
    Over,
}

impl BatteryState {
    /// Get the voltage range associated with the state. The ranges are for 6s LiPo batteries. The
    /// ranges might overlap to prevent fast switching between states.
    fn range(&self) -> RangeInclusive<U16F16> {
        match self {
            Self::Usb => U16F16::MIN..=U16F16!(5.0),
            Self::Critical => U16F16!(5.0)..=U16F16!(18.0),
            Self::Low => U16F16!(17.9)..=U16F16!(20.5),
            Self::Nominal => U16F16!(20.0)..=U16F16!(24.0),
            Self::Full => U16F16!(23.0)..=U16F16!(25.3),
            Self::Over => U16F16!(25.2)..=U16F16::MAX,
        }
    }

    /// Given a state, select the state with the next higher voltage range.
    fn next_up(&self) -> Self {
        match self {
            Self::Usb => Self::Critical,
            Self::Critical => Self::Low,
            Self::Low => Self::Nominal,
            Self::Nominal => Self::Full,
            Self::Full | Self::Over => Self::Over,
        }
    }

    /// Given a state, select the state with the next lower voltage range.
    fn next_down(&self) -> Self {
        match self {
            Self::Usb | Self::Critical => Self::Usb,
            Self::Low => Self::Critical,
            Self::Nominal => Self::Low,
            Self::Full => Self::Nominal,
            Self::Over => Self::Over,
        }
    }
}

#[task]
pub async fn measure_task(
    current_sense: PIN_28,
    voltage_sense: PIN_29,
    adc: ADC,
    shutdown: &'static Signal<CriticalSectionRawMutex, ()>,
    voltage_state: &'static Observable<CriticalSectionRawMutex, BatteryState, 8>,
    voltage_mutex: &'static Mutex<CriticalSectionRawMutex, U16F16>,
) {
    let adc = Adc::new(adc, crate::Irqs, Default::default());
    measure(
        current_sense,
        voltage_sense,
        adc,
        shutdown,
        voltage_state,
        voltage_mutex,
    )
    .await;
}

async fn measure<'d, const SUBS: usize>(
    mut current_sense: impl Channel<Adc<'d>, ID = u8> + Pin,
    mut voltage_sense: impl Channel<Adc<'d>, ID = u8> + Pin,
    mut adc: Adc<'d>,
    shutdown: &Signal<impl RawMutex, ()>,
    voltage_state: &Observable<impl RawMutex, BatteryState, SUBS>,
    voltage_mutex: &Mutex<impl RawMutex, U16F16>,
) {
    const USB_THRESHOLD: U16F16 = U16F16!(5.0);

    let mut state = BatteryState::Full;
    let mut ticker = Ticker::every(Duration::from_hz(100));
    loop {
        let current = adc.read(&mut current_sense).await;
        let voltage = adc.read(&mut voltage_sense).await;
        // ignore this for now because the PCB designer did a bad job and made the opamp positive
        // feedback
        let _current = convert_to_amp(current);
        let voltage = convert_to_volt(voltage);

        // Adjust the battery state until it fits the voltage. This while loop is not critical,
        // since it only looks for the next fiting state. There can be a maximum of 5 iterations
        // and in practice the voltage will almost never jump, so there is only one iteration most
        // of the time.
        while !state.range().contains(&voltage) {
            if *state.range().end() < voltage {
                state = state.next_up();
            } else {
                state = state.next_down();
            }
            voltage_state.set(state);
        }

        if state == BatteryState::Critical && voltage > USB_THRESHOLD {
            shutdown.signal(());
            warn!("Battery critically low");
        } else if state == BatteryState::Over {
            warn!("Battery overcharged");
        }

        {
            *voltage_mutex.lock().await = voltage;
        }
        ticker.next().await;
    }
}

const ADC_MAX: U16F16 = U16F16!(0x0fff);
const REFERENCE_VOLTAGE: U16F16 = U16F16!(3.3);
const BIT_PER_VOLT: U16F16 = ADC_MAX.unwrapped_div(REFERENCE_VOLTAGE);

fn convert_to_amp(current: u16) -> U16F16 {
    const SHUNT_RESISTANCE: U16F16 = U16F16!(0.01);
    const MEASSURE_RESISTANCE: U16F16 = U16F16!(22); // *100Ohm
    const FEEDBACK_RESISTANCE: U16F16 = U16F16!(680); // *100Ohm
    const VOLT_PER_AMP: U16F16 = (U16F16::ONE
        .unwrapped_add(FEEDBACK_RESISTANCE.unwrapped_div(MEASSURE_RESISTANCE)))
    .unwrapped_mul(SHUNT_RESISTANCE);
    const BIT_PER_AMP: U16F16 = VOLT_PER_AMP.unwrapped_mul(BIT_PER_VOLT);
    U16F16::from_num(current) / BIT_PER_AMP
}

fn convert_to_volt(voltage: u16) -> U16F16 {
    const LOWER_RESISTANCE: U16F16 = U16F16!(10); // *100Ohm
    const UPPER_RESISTANCE: U16F16 = U16F16!(68); //*100Ohm
    const VIN_PER_VOUT: U16F16 = LOWER_RESISTANCE
        .unwrapped_add(UPPER_RESISTANCE)
        .unwrapped_div(LOWER_RESISTANCE);
    const BIT_PER_VIN: U16F16 = BIT_PER_VOLT.unwrapped_div(VIN_PER_VOUT);
    U16F16::from_num(voltage) / BIT_PER_VIN
}
