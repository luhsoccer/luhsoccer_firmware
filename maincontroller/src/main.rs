#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod buzzer;
mod configprovider;
mod dribbler;
mod lightbarrier;
mod motorcontroller;
mod power;
mod rf;
mod ui;
mod watchdog;

use cortex_m_rt::entry;
use defmt::Format;
use defmt_rtt as _;
use embassy_executor::{Executor, InterruptExecutor};
use embassy_rp::{
    adc, bind_interrupts, i2c,
    interrupt::{self, Handler},
    pac::Interrupt,
    peripherals::{I2C1, UART0},
    pio::Pio,
    uart,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use fixed::types::U16F16;
use intra_comms::definitions::LocalVelocity;
use panic_probe as _;
use power::BatteryState;
use static_cell::StaticCell;
use sync::observable::Observable;

#[cfg(feature = "test_dribbler")]
use crate::dribbler::dribbler_test_task;
use crate::{
    buzzer::buzzer_task,
    configprovider::{config_task, ConfigV0 as Config},
    dribbler::dribbler_task,
    lightbarrier::lightbarrier_task,
    motorcontroller::motorcontroller_task,
    power::{measure_task, power_switch_task},
    rf::rf_task,
    ui::ui_task,
    watchdog::watchdog_task,
};

bind_interrupts!(struct Irqs {
    UART0_IRQ => uart::BufferedInterruptHandler<UART0>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    SWI_IRQ_0 => ExecutorInterruptHandler;
});

/// Free all spinlocks, regardless of their current status
///
/// RP2040 does not release all spinlocks on reset.
/// The C SDK clears these all during entry, and so do we if you call `hal::entry!`
/// But if someone is using the default cortex-m entry they risk hitting deadlocks so provide *something* to help out
///
/// # Safety
/// Where possible, you should use the `hal::entry` macro attribute on main instead of this.
/// You should call this as soon as possible after reset - preferably as the first entry in fn main(), before *ANY* use of spinlocks, atomics, or `critical_section`
pub unsafe fn spinlock_reset() {
    // Using raw pointers to avoid taking peripherals accidently at startup
    const SIO_BASE: u32 = 0xd000_0000;
    const SPINLOCK0_PTR: *mut u32 = (SIO_BASE + 0x100) as *mut u32;
    const SPINLOCK_COUNT: usize = 32;
    for i in 0..SPINLOCK_COUNT {
        SPINLOCK0_PTR.wrapping_add(i).write_volatile(1);
    }
}

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

struct ExecutorInterruptHandler;

impl<I: interrupt::Interrupt> Handler<I> for ExecutorInterruptHandler {
    unsafe fn on_interrupt() {
        EXECUTOR_HIGH.on_interrupt();
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Format)]
pub enum KickSpeed {
    Velocity(u16),
    Raw(u16),
}

#[entry]
fn main() -> ! {
    static EXECUTOR_LOW: StaticCell<Executor> = StaticCell::new();
    static SHUTDOWN_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
    static SAVE_CONFIG_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
    static DRIBBLER_SPEED: Observable<CriticalSectionRawMutex, u16, 8> = Observable::new(0);
    static HAS_BALL: Observable<CriticalSectionRawMutex, lightbarrier::LightBarrierState, 8> =
        Observable::new(lightbarrier::LightBarrierState::NoBall);
    static VOLTAGE_STATE: Observable<CriticalSectionRawMutex, BatteryState, 8> =
        Observable::new(BatteryState::Nominal);
    static COMMAND_VELOCITY: Observable<CriticalSectionRawMutex, LocalVelocity, 8> =
        Observable::new(LocalVelocity {
            forward: 0,
            left: 0,
            counterclockwise: 0,
        });
    static COMMAND_KICK_SPEED: Observable<CriticalSectionRawMutex, KickSpeed, 8> =
        Observable::new(KickSpeed::Velocity(0));
    static VOLTAGE_MUTEX: Mutex<CriticalSectionRawMutex, U16F16> = Mutex::new(U16F16::ZERO);
    static ACTUAL_VELOCITY: Observable<CriticalSectionRawMutex, LocalVelocity, 8> =
        Observable::new(LocalVelocity {
            forward: 0,
            left: 0,
            counterclockwise: 0,
        });
    static KICKER_VOLTAGE: Observable<CriticalSectionRawMutex, u8, 8> = Observable::new(0);

    static CONFIG: Config<CriticalSectionRawMutex> = Config::new();

    unsafe { spinlock_reset() }
    let p = embassy_rp::init(embassy_rp::config::Config::default());

    let spawner = EXECUTOR_HIGH.start(Interrupt::SWI_IRQ_0);
    spawner.must_spawn(power_switch_task(p.PIN_13, p.PIN_12, &SHUTDOWN_SIGNAL));

    let Pio { common, sm0, .. } = Pio::new(p.PIO0);

    let executor = EXECUTOR_LOW.init(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(watchdog_task(p.WATCHDOG));
        spawner.must_spawn(dribbler_task(p.PIN_20, p.PWM_CH2, &DRIBBLER_SPEED));
        spawner.must_spawn(lightbarrier_task(p.PIN_15, &HAS_BALL, &CONFIG));
        spawner.must_spawn(rf_task(
            p.PIN_0,
            p.PIN_1,
            p.PIN_2,
            p.PIN_3,
            p.PIN_4,
            p.PIN_5,
            p.PIN_6,
            p.PIN_7,
            p.PIN_8,
            p.PIN_14,
            p.SPI0,
            p.DMA_CH0,
            p.DMA_CH1,
            &CONFIG,
            &VOLTAGE_MUTEX,
            &HAS_BALL,
            &DRIBBLER_SPEED,
            &COMMAND_VELOCITY,
            &COMMAND_KICK_SPEED,
            &ACTUAL_VELOCITY,
            &KICKER_VOLTAGE,
        ));
        spawner.must_spawn(motorcontroller_task(
            p.UART0,
            p.PIN_16,
            p.PIN_17,
            p.PIN_18,
            p.PIN_19,
            &HAS_BALL,
            &COMMAND_VELOCITY,
            &COMMAND_KICK_SPEED,
            &ACTUAL_VELOCITY,
            &KICKER_VOLTAGE,
            spawner,
        ));
        spawner.must_spawn(ui_task(
            p.PIN_10,
            p.PIN_11,
            p.I2C1,
            &CONFIG,
            &SAVE_CONFIG_SIGNAL,
            &VOLTAGE_MUTEX,
            &HAS_BALL,
            &COMMAND_KICK_SPEED,
            &DRIBBLER_SPEED,
            &SHUTDOWN_SIGNAL,
        ));
        spawner.must_spawn(buzzer_task(p.PIN_21, common, sm0, &VOLTAGE_STATE));
        spawner.must_spawn(config_task(p.FLASH, &CONFIG, &SAVE_CONFIG_SIGNAL));
        spawner.must_spawn(measure_task(
            p.PIN_28,
            p.PIN_29,
            p.ADC,
            &SHUTDOWN_SIGNAL,
            &VOLTAGE_STATE,
            &VOLTAGE_MUTEX,
        ));
        #[cfg(feature = "test_dribbler")]
        spawner.must_spawn(dribbler_test_task(&DRIBBLER_SPEED));
    });
}
