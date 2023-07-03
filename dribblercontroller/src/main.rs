#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod escon;
mod servo;

use cortex_m_rt::entry;
use defmt_rtt as _;
use embassy_executor::Executor;
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use panic_reset as _;
use static_cell::StaticCell;
use sync::observable::Observable;

use crate::{escon::escon_task, servo::servo_input_task};

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

static mut CORE1_STACK: Stack<4096> = Stack::new();

#[entry]
fn main() -> ! {
    static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
    static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
    static COMMAND_THROTTLE: Observable<CriticalSectionRawMutex, u16, 8> = Observable::new(0);

    unsafe { spinlock_reset() }
    let p = embassy_rp::init(embassy_rp::config::Config::default());

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor = EXECUTOR1.init(Executor::new());
        executor.run(|spawner| {
            spawner.must_spawn(escon_task(
                p.PIN_0,
                p.PIN_1,
                p.PIN_2,
                p.PIN_3,
                p.PWM_CH1,
                &COMMAND_THROTTLE,
            ))
        });
    });

    let executor = EXECUTOR0.init(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(servo_input_task(p.PIN_11, p.PWM_CH5, &COMMAND_THROTTLE));
    });
}
