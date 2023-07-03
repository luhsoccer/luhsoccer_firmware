#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![allow(clippy::future_not_send)]

mod configprovider;
mod kicker;
mod maincontroller;
mod odometry;
mod watchdog;

use configprovider::ConfigV0 as Config;
use cortex_m_rt::entry;
#[allow(unused_imports)]
use defmt::{
    assert, assert_eq, assert_ne, bitflags, dbg, debug, debug_assert, debug_assert_eq,
    debug_assert_ne, error, info, intern, panic, println, timestamp, todo, trace, unimplemented,
    unreachable, unwrap, warn, write,
};
use defmt_rtt as _;
use embassy_executor::{Executor, InterruptExecutor};
use embassy_rp::{
    adc::{self, Adc},
    bind_interrupts,
    interrupt::{self, Handler},
    multicore::{spawn_core1, Stack},
    pac::Interrupt,
    peripherals::UART0,
    pio::Pio,
    uart,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
use fixed::types::I24F8;
use panic_probe as _;
use static_cell::StaticCell;
use sync::observable::Observable;
use units::types::{RadianPerSecond, Volt};

#[cfg(feature = "test_kicker")]
use crate::kicker::kicker_test_task;
#[cfg(feature = "test_motors")]
use crate::odometry::motors_test_task;
use crate::watchdog::watchdog_task;
use crate::{configprovider::config_task, odometry::calc_robot_speed_task};
use crate::{
    kicker::kicker_task,
    maincontroller::maincontroller_task,
    odometry::{motors_task, Movement},
};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
    UART0_IRQ => uart::BufferedInterruptHandler<UART0>;
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

// Multicore
static mut CORE1_STACK: Stack<{ 1024 * 4 }> = Stack::new();

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

struct ExecutorInterruptHandler;

impl<I: interrupt::Interrupt> Handler<I> for ExecutorInterruptHandler {
    unsafe fn on_interrupt() {
        EXECUTOR_HIGH.on_interrupt();
    }
}

#[entry]
fn main() -> ! {
    // Executors
    static EXECUTOR_CORE1: StaticCell<Executor> = StaticCell::new();
    static EXECUTOR_LOW: StaticCell<Executor> = StaticCell::new();

    static SAVE_CONFIG: Signal<CriticalSectionRawMutex, ()> = Signal::new();
    static MOVEMENT_SETPOINT: Observable<CriticalSectionRawMutex, Movement, 8> =
        Observable::new(Movement::new());
    static HAS_BALL: Observable<CriticalSectionRawMutex, bool, 8> = Observable::new(false);
    static KICKER_VOLTAGE_SIGNAL: Observable<CriticalSectionRawMutex, Volt<u8>, 8> =
        Observable::new(Volt::new(0));
    static KICKER_CAP_VOLTAGE: Observable<CriticalSectionRawMutex, Volt<u8>, 8> =
        Observable::new(Volt::new(0));
    static KICKER_SPEED: Observable<CriticalSectionRawMutex, u16, 8> = Observable::new(0);
    static WHEEL_SPEEDS: Observable<CriticalSectionRawMutex, [RadianPerSecond<I24F8>; 4], 8> =
        Observable::new([RadianPerSecond::new(I24F8::ZERO); 4]);
    static KICKER_RAW_DURATION: Observable<CriticalSectionRawMutex, Duration, 8> =
        Observable::new(Duration::MIN);
    static ACTUAL_MOVEMENT: Observable<CriticalSectionRawMutex, Movement, 8> =
        Observable::new(Movement::new());

    static CONFIG: Config<CriticalSectionRawMutex> = Config::new();

    // # Safety
    // Nothing uses spinlocks yet.
    unsafe { spinlock_reset() }
    let p = embassy_rp::init(embassy_rp::config::Config::default());

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor = EXECUTOR_CORE1.init(Executor::new());
        executor.run(|spawner| {
            spawner.must_spawn(motors_task(
                p.SPI1,
                p.PIN_26,
                p.PIN_28,
                p.PIN_27,
                (p.PIN_24, p.PIN_22, p.PIN_25, p.PIN_23),
                p.DMA_CH0,
                p.DMA_CH1,
                &MOVEMENT_SETPOINT,
                &WHEEL_SPEEDS,
                &CONFIG,
                spawner,
            ));
        })
    });

    let Pio { common, sm0, .. } = Pio::new(p.PIO0);

    let spawner = EXECUTOR_HIGH.start(Interrupt::SWI_IRQ_0);

    let adc = Adc::new(p.ADC, Irqs, adc::Config::default());
    spawner.must_spawn(kicker_task(
        &HAS_BALL,
        &KICKER_VOLTAGE_SIGNAL,
        &KICKER_SPEED,
        &KICKER_RAW_DURATION,
        (p.PIN_0, p.PIN_1),
        p.PIN_12,
        p.PIN_13,
        p.PIN_14,
        p.PIN_15,
        p.PIN_29,
        (
            p.PIN_2, p.PIN_3, p.PIN_4, p.PIN_5, p.PIN_6, p.PIN_7, p.PIN_8, p.PIN_9, p.PIN_10,
            p.PIN_11,
        ),
        adc,
        sm0,
        common,
        &CONFIG,
    ));

    let executor = EXECUTOR_LOW.init(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(watchdog_task(p.WATCHDOG));
        spawner.must_spawn(maincontroller_task(
            p.UART0,
            p.PIN_16,
            p.PIN_17,
            p.PIN_18,
            p.PIN_19,
            &MOVEMENT_SETPOINT,
            &KICKER_VOLTAGE_SIGNAL,
            &HAS_BALL,
            &KICKER_CAP_VOLTAGE,
            &KICKER_SPEED,
            &KICKER_RAW_DURATION,
            &ACTUAL_MOVEMENT,
            &SAVE_CONFIG,
            &CONFIG,
            spawner,
        ));
        spawner.must_spawn(config_task(p.FLASH, &CONFIG, &SAVE_CONFIG));
        spawner.must_spawn(calc_robot_speed_task(&WHEEL_SPEEDS, &ACTUAL_MOVEMENT));
        #[cfg(feature = "test_motors")]
        spawner.must_spawn(motors_test_task(&MOVEMENT_SETPOINT));
        #[cfg(feature = "test_kicker")]
        spawner.must_spawn(kicker_test_task(&KICKER_VOLTAGE_SIGNAL, &HAS_BALL));
    });
}
