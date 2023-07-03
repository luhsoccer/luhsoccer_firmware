use defmt::Format;
use embassy_executor::task;
use embassy_rp::{
    clocks::RoscRng,
    peripherals::{PIN_21, PIO0},
    pio::{self as pio_mod, Common, Direction, Instance, PioPin, StateMachine},
    relocate::RelocatedProgram,
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_time::{Duration, Timer};
use rand_distr::{uniform::Uniform, Distribution};
use sync::observable::Observable;

use crate::power::BatteryState;

#[task]
pub async fn buzzer_task(
    pin: PIN_21,
    mut pio: Common<'static, PIO0>,
    mut sm: StateMachine<'static, PIO0, 0>,
    voltage_state: &'static Observable<CriticalSectionRawMutex, BatteryState, 8>,
) {
    setup_buzzer_pio(&mut pio, &mut sm, pin);
    buzzer(&mut sm, voltage_state).await;
}

fn setup_buzzer_pio<'a, PIO: Instance, const SM: usize>(
    pio: &mut Common<'a, PIO>,
    sm: &mut StateMachine<'a, PIO, SM>,
    pin: impl PioPin,
) {
    let prg = pio_proc::pio_asm!(
        ".wrap_target",
        "mov pins, isr",
        "mov isr, ~isr",
        "pull noblock",
        "mov x, osr",
        "mov y, x",
        "loop:",
        "jmp y--, loop",
        ".wrap",
    )
    .program;
    let relocated = RelocatedProgram::new(&prg);
    let mut cfg = pio_mod::Config::default();
    cfg.use_program(&pio.load_program(&relocated), &[]);
    // set 1MHz clock
    cfg.clock_divider = 125u8.into();
    let pin = pio.make_pio_pin(pin);
    cfg.set_out_pins(&[&pin]);
    sm.set_config(&cfg);
    sm.set_pin_dirs(Direction::Out, &[&pin]);
    sm.set_enable(true);
}

async fn buzzer<const SM: usize, const SUBS: usize>(
    sm: &mut StateMachine<'_, impl Instance, SM>,
    voltage_state: &Observable<impl RawMutex, BatteryState, SUBS>,
) {
    use Tone::*;
    const STARTUP: [(Tone, u64); 8] = [
        (Off, 1000),
        (Ab5, 200),
        (B5, 200),
        (Db6, 200),
        (Off, 1000),
        (Ab5, 450),
        (Off, 200),
        (Eb6, 500),
    ];
    const STARTUP0: [(Tone, u64); 4] = [(Off, 1000), (Ab5, 200), (B5, 200), (Db6, 200)];
    const STARTUP1: [(Tone, u64); 38] = [
        (D5, 4),
        (E5, 4),
        (G5, 4),
        (D5, 4),
        (B5, 2),
        (Off, 4),
        (B5, 2),
        (Off, 4),
        (A5, 1),
        (Off, 2),
        (D5, 4),
        (E5, 4),
        (G5, 4),
        (D5, 4),
        (A5, 2),
        (Off, 4),
        (A5, 2),
        (Off, 4),
        (G5, 2),
        (G5, 4),
        (Gb5, 4),
        (E5, 2),
        (D5, 4),
        (E5, 4),
        (G5, 4),
        (E5, 4),
        (G5, 1),
        (A5, 2),
        (Gb5, 2),
        (Gb5, 4),
        (E5, 4),
        (D5, 2),
        (D5, 4),
        (Off, 4),
        (A5, 2),
        (A5, 4),
        (Off, 4),
        (Gb5, 1),
    ];

    let uniform = Uniform::from(0..1000);
    match uniform.sample(&mut RoscRng) {
        0 => {
            play_sequence(
                sm,
                STARTUP1
                    .iter()
                    .map(|(tone, divider)| (*tone, Duration::from_millis(500 / divider))),
            )
            .await
        }
        1..=10 => {
            play_sequence(
                sm,
                STARTUP0
                    .iter()
                    .map(|(tone, duration)| (*tone, Duration::from_millis(*duration))),
            )
            .await
        }
        11 => {
            play_sequence(
                sm,
                STARTUP
                    .iter()
                    .rev()
                    .map(|(tone, duration)| (*tone, Duration::from_millis(*duration))),
            )
            .await
        }
        12 => {
            // Don't play any startup sound
        }
        _ => {
            play_sequence(
                sm,
                STARTUP
                    .iter()
                    .map(|(tone, duration)| (*tone, Duration::from_millis(*duration))),
            )
            .await
        }
    }

    loop {
        if matches!(
            voltage_state.get(),
            BatteryState::Low | BatteryState::Critical
        ) {
            play(sm, A5);
            Timer::after(Duration::from_millis(500)).await;
            play(sm, Off);
            Timer::after(Duration::from_millis(500)).await;
        } else {
            play(sm, Off);
            Timer::after(Duration::from_hz(10)).await;
        }
    }
}

fn play<const SM: usize>(sm: &mut StateMachine<'_, impl Instance, SM>, tone: Tone) {
    if tone == Tone::Off {
        sm.set_enable(false);
    } else {
        if !sm.is_enabled() {
            sm.restart();
        }
        sm.tx().push(counts(tone as u32));
        sm.set_enable(true);
    }
}

async fn play_sequence<const SM: usize>(
    sm: &mut StateMachine<'_, impl Instance, SM>,
    tones: impl IntoIterator<Item = (Tone, Duration)>,
) {
    for (tone, delay) in tones.into_iter() {
        play(sm, tone);
        Timer::after(delay).await;
    }
    play(sm, Tone::Off);
}

const fn counts(frequency: u32) -> u32 {
    const CLOCK: u32 = 500_000;
    const MIN_CYCLES: u32 = 6;
    if frequency == 0 {
        0
    } else {
        let counts = CLOCK * 100 / frequency;
        counts.saturating_sub(MIN_CYCLES)
    }
}

#[allow(dead_code)]
#[repr(u32)]
#[derive(Debug, Format, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Tone {
    Off = 0,
    C0 = 1635,
    Db0 = 1732,
    D0 = 1835,
    Eb0 = 1945,
    E0 = 2060,
    F0 = 2183,
    Gb0 = 2312,
    G0 = 2450,
    Ab0 = 2596,
    A0 = 2750,
    Bb0 = 2914,
    B0 = 3087,
    C1 = 3270,
    Db1 = 3465,
    D1 = 3671,
    Eb1 = 3889,
    E1 = 4120,
    F1 = 4365,
    Gb1 = 4625,
    G1 = 4900,
    Ab1 = 5191,
    A1 = 5500,
    Bb1 = 5827,
    B1 = 6174,
    C2 = 6541,
    Db2 = 6930,
    D2 = 7342,
    Eb2 = 7778,
    E2 = 8241,
    F2 = 8731,
    Gb2 = 9250,
    G2 = 9800,
    Ab2 = 10383,
    A2 = 11000,
    Bb2 = 11654,
    B2 = 12347,
    C3 = 13081,
    Db3 = 13859,
    D3 = 14683,
    Eb3 = 15556,
    E3 = 16481,
    F3 = 17461,
    Gb3 = 18500,
    G3 = 19600,
    Ab3 = 20765,
    A3 = 22000,
    Bb3 = 23308,
    B3 = 24694,
    C4 = 26163,
    Db4 = 27718,
    D4 = 29366,
    Eb4 = 31113,
    E4 = 32963,
    F4 = 34923,
    Gb4 = 36999,
    G4 = 39200,
    Ab4 = 41530,
    A4 = 44000,
    Bb4 = 46616,
    B4 = 49388,
    C5 = 52325,
    Db5 = 55437,
    D5 = 58733,
    Eb5 = 62225,
    E5 = 65925,
    F5 = 69846,
    Gb5 = 73999,
    G5 = 78399,
    Ab5 = 83061,
    A5 = 88000,
    Bb5 = 93233,
    B5 = 98777,
    C6 = 104650,
    Db6 = 110873,
    D6 = 117466,
    Eb6 = 124451,
    E6 = 131851,
    F6 = 139691,
    Gb6 = 147998,
    G6 = 156798,
    Ab6 = 166122,
    A6 = 176000,
    Bb6 = 186466,
    B6 = 197553,
    C7 = 209300,
    Db7 = 221746,
    D7 = 234932,
    Eb7 = 248902,
    E7 = 263702,
    F7 = 279383,
    Gb7 = 295996,
    G7 = 313596,
    Ab7 = 332244,
    A7 = 352000,
    Bb7 = 372931,
    B7 = 395107,
    C8 = 418601,
    Db8 = 443492,
    D8 = 469863,
    Eb8 = 497803,
    E8 = 527404,
    F8 = 558765,
    Gb8 = 591991,
    G8 = 627193,
    Ab8 = 664488,
    A8 = 704000,
    Bb8 = 745862,
    B8 = 790213,
}
