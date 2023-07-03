#![allow(clippy::upper_case_acronyms)]

use crate::pac::{pmc, PMC, SUPC};

#[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
use crate::pac::EFC;

#[cfg(feature = "atsam4s")]
use crate::pac::EFC0;

#[cfg(feature = "atsam4sd")]
use crate::pac::EFC1;

use core::marker::PhantomData;
use fugit::{HertzU32 as Hertz, RateExtU32};

static mut MASTER_CLOCK_FREQUENCY: Hertz = Hertz::from_raw(0);

#[cfg(all(feature = "atsam4s", feature = "usb"))]
static mut PLLB_MULTIPLIER: u16 = 0;

// NOTE: More frequencies and crystals can be added
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainClock {
    #[cfg(not(feature = "atsam4n"))]
    RcOscillator4Mhz, // USB Unsupported
    RcOscillator8Mhz,  // USB Unsupported
    RcOscillator12Mhz, // USB Unsupported
    Crystal12Mhz,      // USB Supported
    Crystal16Mhz,
                       // Crystal11289Khz, // USB Supported - Not implemented
                       // Crystal16MHz     // USB Supported - Not implemented
                       // Crystal18432Khz, // USB Supported - Not implemented
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlowClock {
    RcOscillator32Khz,
    Crystal32Khz,
    OscillatorBypass32Khz,
}

pub fn get_master_clock_frequency() -> Hertz {
    unsafe { MASTER_CLOCK_FREQUENCY }
}

fn setup_slow_clock(supc: &SUPC, slow_clock: SlowClock) -> Hertz {
    match slow_clock {
        // Nothing to do, defaults to 32 kHz
        // You cannot set the Slow Clock back to RC until VDDIO is reset
        SlowClock::RcOscillator32Khz => {}
        SlowClock::Crystal32Khz => {
            // Enable crystal oscillator (also disables Slow RC Oscillator)
            unsafe {
                supc.cr
                    .write_with_zero(|w| w.key().passwd().xtalsel().crystal_sel());
            }
        }
        SlowClock::OscillatorBypass32Khz => {
            // Enable bypass mode
            supc.mr.modify(|_, w| w.key().passwd().oscbypass().bypass());
            // Enable crystal oscillator (also disables Slow RC Oscillator)
            unsafe {
                supc.cr
                    .write_with_zero(|w| w.key().passwd().xtalsel().crystal_sel());
            }
        }
    }
    // 32.768 kHz
    32768.Hz()
}

fn setup_main_clock(pmc: &PMC, main_clock: MainClock) -> Hertz {
    let prescaler = match main_clock {
        #[cfg(not(feature = "atsam4n"))]
        MainClock::RcOscillator4Mhz => {
            switch_main_clock_to_fast_rc_4mhz(pmc);

            // Set up the PLL for 120Mhz operation (4Mhz RC * (30 / 1) = 120Mhz)
            let multiplier: u16 = 30;
            let divider: u8 = 1;
            enable_plla_clock(pmc, multiplier, divider);

            // 0 = no prescaling
            0
        }
        #[cfg(not(feature = "atsam4n"))]
        MainClock::RcOscillator8Mhz => {
            switch_main_clock_to_fast_rc_8mhz(pmc);

            // Set up the PLL for 120Mhz operation (8Mhz RC * (15 / 1) = 120Mhz)
            let multiplier: u16 = 15;
            let divider: u8 = 1;
            enable_plla_clock(pmc, multiplier, divider);

            // 0 = no prescaling
            0
        }
        #[cfg(feature = "atsam4n")]
        MainClock::RcOscillator8Mhz => {
            switch_main_clock_to_fast_rc_8mhz(pmc);

            // Set up the PLL for 100Mhz operation (8Mhz RC * (25 / 2) = 100Mhz)
            let multiplier: u16 = 25;
            let divider: u8 = 2;
            enable_plla_clock(pmc, multiplier, divider);

            // 0 = no prescaling
            0
        }
        #[cfg(not(feature = "atsam4n"))]
        MainClock::RcOscillator12Mhz => {
            switch_main_clock_to_fast_rc_12mhz(pmc);

            // Set up the PLL for 120Mhz operation (12Mhz RC * (10 / 1) = 120Mhz)
            let multiplier: u16 = 10;
            let divider: u8 = 1;
            enable_plla_clock(pmc, multiplier, divider);

            // 0 = no prescaling
            0
        }
        #[cfg(feature = "atsam4n")]
        MainClock::RcOscillator12Mhz => {
            switch_main_clock_to_fast_rc_12mhz(pmc);

            // Set up the PLL for 100Mhz operation (12Mhz RC * (25 / 3) = 100Mhz)
            let multiplier: u16 = 25;
            let divider: u8 = 3;
            enable_plla_clock(pmc, multiplier, divider);

            // 0 = no prescaling
            0
        }
        #[cfg(feature = "atsam4e")]
        MainClock::Crystal12Mhz => {
            switch_main_clock_to_external_12mhz(pmc);

            #[cfg(feature = "usb")]
            {
                // Set up the PLL for 240MHz operation (12 MHz * (20 / 1) = 240 MHz)
                // 240 MHz can be used to generate both master 120 MHz clock and USB 48 MHz clock
                let multiplier: u16 = 20;
                let divider: u8 = 1;
                enable_plla_clock(pmc, multiplier, divider);

                // 1 = /2 prescaling
                1
            }

            #[cfg(not(feature = "usb"))]
            {
                // Set up the PLL for 120MHz operation (12 MHz * (10 / 1) = 120 MHz)
                // Uses less power than running the PLL at 240 MHz
                let multiplier: u16 = 10;
                let divider: u8 = 1;
                enable_plla_clock(pmc, multiplier, divider);

                // 0 = no prescaling
                0
            }
        }
        #[cfg(feature = "atsam4e")]
        MainClock::Crystal16Mhz => {
            switch_main_clock_to_external_12mhz(pmc);

            //#[cfg(feature = "usb")]
            {
                // Set up the PLL for 240MHz operation (16 MHz * (15 / 1) = 240 MHz)
                // 240 MHz can be used to generate both master 120 MHz clock and USB 48 MHz clock
                let multiplier: u16 = 15;
                let divider: u8 = 1;
                enable_plla_clock(pmc, multiplier, divider);

                // 1 = /2 prescaling
                1
            }
        }
        #[cfg(feature = "atsam4n")]
        MainClock::Crystal12Mhz => {
            switch_main_clock_to_external_12mhz(pmc);

            // Set up the PLL for 100MHz operation (12 MHz * (25 / 3) = 100 MHz)
            let multiplier: u16 = 25;
            let divider: u8 = 3;
            enable_plla_clock(pmc, multiplier, divider);

            // 0 = no prescaling
            0
        }

        #[cfg(feature = "atsam4s")]
        MainClock::Crystal12Mhz => {
            switch_main_clock_to_external_12mhz(pmc);

            // Setup PLLA for 120 MHz operation (12 MHz * (10 / 1) = 120 MHz)
            let multiplier: u16 = 10;
            let divider: u8 = 1;
            enable_plla_clock(pmc, multiplier, divider);

            #[cfg(feature = "usb")]
            {
                // Setup PLLB for 96 MHz operation (12 MHz * (8 / 1) = 96 MHz)
                // 96 MHz will be /2 to get 48 MHz
                let multiplier: u16 = 8;
                let divider: u8 = 1;
                enable_pllb_clock(pmc, multiplier, divider);
            }

            // 0 = no prescaling
            0
        }
    };
    wait_for_main_clock_ready(pmc);

    wait_for_plla_lock(pmc);

    switch_master_clock_to_plla(pmc, prescaler);

    calculate_master_clock_frequency(pmc)
}

fn calculate_master_clock_frequency(pmc: &PMC) -> Hertz {
    let mut mclk_freq = match pmc.pmc_mckr.read().css().bits() {
        0 => {
            // Slow clock
            panic!("Unsupported clock source: Slow clock.")
        }
        1 => {
            // Main clock
            panic!("Unsupported clock source: Main clock.")
        }
        2 => {
            // PLL
            let mut mclk_freq = match pmc.ckgr_mor.read().moscsel().bit_is_set() {
                true => 16_u32.MHz(),
                false => {
                    if pmc.ckgr_mor.read().moscrcf().is_12_mhz() {
                        12_u32.MHz()
                    } else if pmc.ckgr_mor.read().moscrcf().is_8_mhz() {
                        8_u32.MHz()
                    } else if pmc.ckgr_mor.read().moscrcf().is_4_mhz() {
                        4_u32.MHz()
                    } else {
                        panic!("Unexpected value detected read from pmc.ckgr_mor.moscrcf")
                    }
                }
            };

            let plla_clock_source: u8 = 2; // 2 = PLLA
            if pmc.pmc_mckr.read().css().bits() == plla_clock_source {
                mclk_freq *= (pmc.ckgr_pllar.read().mula().bits() + 1) as u32;
                mclk_freq /= (pmc.ckgr_pllar.read().diva().bits()) as u32;
            }

            mclk_freq
        }
        _ => panic!("Invalid value found in PMC_MCKR.CSS"),
    };

    // Factor in the prescaler
    mclk_freq = match pmc.pmc_mckr.read().pres().bits() {
        7 => mclk_freq / 3, // Special case for a 3 prescaler
        prescaler => (mclk_freq.raw() >> prescaler).Hz(),
    };

    mclk_freq
}

fn get_flash_wait_states_for_clock_frequency(clock_frequency: Hertz) -> u8 {
    match clock_frequency {
        c if c < 20_u32.MHz::<1, 1>() => 0,
        c if c < 40_u32.MHz::<1, 1>() => 1,
        c if c < 60_u32.MHz::<1, 1>() => 2,
        c if c < 80_u32.MHz::<1, 1>() => 3,
        c if c < 100_u32.MHz::<1, 1>() => 4,
        c if c < 123_u32.MHz::<1, 1>() => 5,
        _ => panic!(
            "Invalid frequency provided to get_flash_wait_states(): {} ",
            clock_frequency
        ),
    }
}

#[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
fn set_flash_wait_states_to_maximum(efc: &EFC) {
    efc.fmr
        .modify(|_, w| unsafe { w.fws().bits(5).cloe().set_bit() });
}

#[cfg(all(feature = "atsam4s", not(feature = "atsam4sd")))]
fn set_flash_wait_states_to_maximum(efc0: &EFC0) {
    efc0.fmr
        .modify(|_, w| unsafe { w.fws().bits(5).cloe().set_bit() });
}

#[cfg(feature = "atsam4sd")]
fn set_flash_wait_states_to_maximum(efc0: &EFC0, efc1: &EFC1) {
    efc0.fmr
        .modify(|_, w| unsafe { w.fws().bits(5).cloe().set_bit() });
    efc1.fmr
        .modify(|_, w| unsafe { w.fws().bits(5).cloe().set_bit() });
}

#[cfg(any(feature = "atsam4n", feature = "atsam4e"))]
fn set_flash_wait_states_to_match_frequency(efc: &EFC, clock_frequency: Hertz) {
    let wait_state_count = get_flash_wait_states_for_clock_frequency(clock_frequency);

    efc.fmr
        .modify(|_, w| unsafe { w.fws().bits(wait_state_count).cloe().set_bit() });
}

#[cfg(feature = "atsam4s")]
fn set_flash_wait_states_to_match_frequency(
    efc0: &EFC0,
    #[cfg(feature = "atsam4sd")] efc1: &EFC1,
    clock_frequency: Hertz,
) {
    let wait_state_count = get_flash_wait_states_for_clock_frequency(clock_frequency);

    efc0.fmr
        .modify(|_, w| unsafe { w.fws().bits(wait_state_count).cloe().set_bit() });
    #[cfg(feature = "atsam4sd")]
    efc1.fmr
        .modify(|_, w| unsafe { w.fws().bits(wait_state_count).cloe().set_bit() });
}

fn switch_main_clock_to_external_12mhz(pmc: &PMC) {
    // Activate external oscillator
    // As we are clocking the core from internal Fast RC, we keep the bit CKGR_MOR_MOSCRCEN.
    // Main Crystal Oscillator Start-up Time (CKGR_MOR_MOSCXTST) is set to maximum value.
    // Then, we wait the startup time to be finished by checking PMC_SR_MOSCXTS in PMC_SR.
    activate_crystal_oscillator(pmc);
    wait_for_main_crystal_ready(pmc);

    // Switch the MAINCK to the main crystal oscillator
    // We add the CKGR_MOR_MOSCSEL bit.
    // Then we wait for the selection to be done by checking PMC_SR_MOSCSELS in PMC_SR.
    change_main_clock_to_crystal(pmc);
    wait_for_main_clock_ready(pmc);
}

fn activate_crystal_oscillator(pmc: &PMC) {
    // ATSAM4S Datasheet 38.5.3
    // Maximum crystal startup time is 62 ms (worst-case)
    // Slow clock is 32 kHz
    // MOSCXTST is the number of slow clocks x8
    // 62 ms / (1 / 32 kHz) / 8 = 248
    //let crystal_startup_cycles = 248;

    // From the datasheet 8 MHz and 16 MHz crystals take between 4 and 2.5 ms to start
    // Using 4 ms for 12 MHz should be sufficient
    // 4 ms / (1 / 32 kHz) / 8 = 16
    let crystal_startup_cycles = 16;

    pmc.ckgr_mor.modify(|_, w| unsafe {
        w.key()
            .passwd()
            .moscrcen()
            .set_bit()
            .moscxten()
            .set_bit()
            .moscxtst()
            .bits(crystal_startup_cycles)
    });
}

fn is_main_crystal_ready(pmc: &PMC) -> bool {
    pmc.pmc_sr.read().moscxts().bit_is_set()
}

fn wait_for_main_crystal_ready(pmc: &PMC) {
    while !is_main_crystal_ready(pmc) {}
}

fn change_main_clock_to_crystal(pmc: &PMC) {
    // Switch to fast crystal
    // Disable RC oscillator
    pmc.ckgr_mor
        .modify(|_, w| w.key().passwd().moscrcen().clear_bit().moscsel().set_bit());
}

#[cfg(not(feature = "atsam4n"))]
fn switch_main_clock_to_fast_rc_4mhz(pmc: &PMC) {
    enable_fast_rc_oscillator(pmc);
    wait_for_fast_rc_oscillator_to_stabilize(pmc);
    change_fast_rc_oscillator_to_4mhz(pmc);
    wait_for_fast_rc_oscillator_to_stabilize(pmc);
    switch_to_fast_rc_oscillator(pmc);
}

fn switch_main_clock_to_fast_rc_8mhz(pmc: &PMC) {
    enable_fast_rc_oscillator(pmc);
    wait_for_fast_rc_oscillator_to_stabilize(pmc);
    change_fast_rc_oscillator_to_8mhz(pmc);
    wait_for_fast_rc_oscillator_to_stabilize(pmc);
    switch_to_fast_rc_oscillator(pmc);
}

fn switch_main_clock_to_fast_rc_12mhz(pmc: &PMC) {
    enable_fast_rc_oscillator(pmc);
    wait_for_fast_rc_oscillator_to_stabilize(pmc);
    change_fast_rc_oscillator_to_12mhz(pmc);
    wait_for_fast_rc_oscillator_to_stabilize(pmc);
    switch_to_fast_rc_oscillator(pmc);
}

fn enable_fast_rc_oscillator(pmc: &PMC) {
    pmc.ckgr_mor
        .modify(|_, w| w.key().passwd().moscrcen().set_bit());
}

#[cfg(not(feature = "atsam4n"))]
fn change_fast_rc_oscillator_to_4mhz(pmc: &PMC) {
    pmc.ckgr_mor
        .modify(|_, w| w.key().passwd().moscrcf()._4_mhz());
}

fn change_fast_rc_oscillator_to_8mhz(pmc: &PMC) {
    pmc.ckgr_mor
        .modify(|_, w| w.key().passwd().moscrcf()._8_mhz());
}

fn change_fast_rc_oscillator_to_12mhz(pmc: &PMC) {
    pmc.ckgr_mor
        .modify(|_, w| w.key().passwd().moscrcf()._12_mhz());
}

fn switch_to_fast_rc_oscillator(pmc: &PMC) {
    pmc.ckgr_mor
        .modify(|_, w| w.key().passwd().moscsel().clear_bit());
}

fn wait_for_fast_rc_oscillator_to_stabilize(pmc: &PMC) {
    while pmc.pmc_sr.read().moscrcs().bit_is_clear() {}
}

fn is_main_clock_ready(pmc: &PMC) -> bool {
    pmc.pmc_sr.read().moscsels().bit_is_set()
}

fn wait_for_main_clock_ready(pmc: &PMC) {
    while !is_main_clock_ready(pmc) {}
}

fn enable_plla_clock(pmc: &PMC, multiplier: u16, divider: u8) {
    disable_plla_clock(pmc);

    // Per ATSAM4S 44.6 and ATSAM4E16 46.5
    // PLL settling time is between 60 and 150 us
    // (1 / 32 kHz) = 31.25 us
    // 60  / (1 / 32 kHz) = 1.92 -> 2
    // 150 / (1 / 32 kHz) = 4.8 -> 5
    let settling_cycles = 5;

    // NOTE: the datasheet indicates the multplier used it MULA + 1 - hence the subtraction when setting the multiplier.
    pmc.ckgr_pllar.modify(|_, w| unsafe {
        w.one()
            .set_bit()
            .pllacount()
            .bits(settling_cycles)
            .mula()
            .bits(multiplier - 1)
            .diva()
            .bits(divider)
    });
}

fn disable_plla_clock(pmc: &PMC) {
    pmc.ckgr_pllar
        .modify(|_, w| unsafe { w.one().set_bit().mula().bits(0) });
}

fn is_plla_locked(pmc: &PMC) -> bool {
    pmc.pmc_sr.read().locka().bit_is_set()
}

fn wait_for_plla_lock(pmc: &PMC) {
    while !is_plla_locked(pmc) {}
}

fn switch_master_clock_to_plla(pmc: &PMC, prescaler: u8) {
    // Set the master clock prescaler
    pmc.pmc_mckr.modify(|_, w| w.pres().bits(prescaler));

    wait_for_master_clock_ready(pmc);

    // Set the master clock source to PLLA
    // BUGBUG: What requires the 'unsafe' on SAM4?  SVD issue?
    let clock_source: u8 = 2; // 2 = PLLA
    #[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
    pmc.pmc_mckr
        .modify(|_, w| unsafe { w.css().bits(clock_source) });

    #[cfg(feature = "atsam4s")]
    pmc.pmc_mckr.modify(|_, w| w.css().bits(clock_source));

    wait_for_master_clock_ready(pmc);
}

fn is_master_clock_ready(pmc: &PMC) -> bool {
    pmc.pmc_sr.read().mckrdy().bit_is_set()
}

fn wait_for_master_clock_ready(pmc: &PMC) {
    while !is_master_clock_ready(pmc) {}
}

#[cfg(all(feature = "atsam4s", feature = "usb"))]
fn enable_pllb_clock(pmc: &PMC, multiplier: u16, divider: u8) {
    disable_pllb_clock(pmc);

    // Per ATSAM4S 44.6 and ATSAM4E16 46.5
    // PLL settling time is between 60 and 150 us
    // (1 / 32 kHz) = 31.25 us
    // 60  / (1 / 32 kHz) = 1.92 -> 2
    // 150 / (1 / 32 kHz) = 4.8 -> 5
    let settling_cycles = 5;

    // NOTE: the datasheet indicates the multplier used it MULB + 1 - hence the subtraction when setting the multiplier.
    pmc.ckgr_pllbr.modify(|_, w| unsafe {
        w.pllbcount()
            .bits(settling_cycles)
            .mulb()
            .bits(multiplier - 1)
            .divb()
            .bits(divider)
    });

    unsafe { PLLB_MULTIPLIER = multiplier - 1 }; // Save for reenable_pllb_clock
}

/// Used to re-enable pllb clock if disabled later at runtime
/// For use with UDP suspend
#[cfg(all(feature = "atsam4s", feature = "usb"))]
pub fn reenable_pllb_clock(pmc: &PMC) {
    pmc.ckgr_pllbr
        .modify(|_, w| unsafe { w.mulb().bits(PLLB_MULTIPLIER) });
}

#[cfg(all(feature = "atsam4s", feature = "usb"))]
pub fn disable_pllb_clock(pmc: &PMC) {
    pmc.ckgr_pllbr.modify(|_, w| unsafe { w.mulb().bits(0) });
}

#[cfg(all(feature = "atsam4s", feature = "usb"))]
fn is_pllb_locked(pmc: &PMC) -> bool {
    pmc.pmc_sr.read().lockb().bit_is_set()
}

#[cfg(all(feature = "atsam4s", feature = "usb"))]
pub fn wait_for_pllb_lock(pmc: &PMC) {
    while !is_pllb_locked(pmc) {}
}

// Peripheral Clock State
#[derive(Default)]
pub struct Enabled;

#[derive(Default)]
pub struct Disabled;

#[derive(Default)]
pub struct PeripheralClock<STATE> {
    _state: PhantomData<STATE>,
}

macro_rules! peripheral_clocks {
    (
        $($PeripheralType:ident, $peripheral_ident:ident, $i:expr,)+
    ) => {
        #[derive(Default)]
        pub struct PeripheralClocks {
            $(
                pub $peripheral_ident: $PeripheralType<Disabled>,
            )+
        }

        impl PeripheralClocks {
            pub fn new() -> Self {
                PeripheralClocks {
                    $(
                        $peripheral_ident: $PeripheralType { _state: PhantomData },
                    )+
                }
            }
        }

        $(
            #[derive(Default)]
            pub struct $PeripheralType<STATE> {
                _state: PhantomData<STATE>,
            }

            impl<STATE> $PeripheralType<STATE> {
                pub(crate) fn pcer0(&mut self) -> &pmc::PMC_PCER0 {
                    unsafe { &(*PMC::ptr()).pmc_pcer0 }
                }

                #[cfg(not(feature = "atsam4n"))]
                pub(crate) fn pcer1(&mut self) -> &pmc::PMC_PCER1 {
                    unsafe { &(*PMC::ptr()).pmc_pcer1 }
                }

                pub(crate) fn pcdr0(&mut self) -> &pmc::PMC_PCDR0 {
                    unsafe { &(*PMC::ptr()).pmc_pcdr0 }
                }

                #[cfg(not(feature = "atsam4n"))]
                pub(crate) fn pcdr1(&mut self) -> &pmc::PMC_PCDR1 {
                    unsafe { &(*PMC::ptr()).pmc_pcdr1 }
                }

                #[cfg(not(feature = "atsam4n"))]
                pub fn into_enabled_clock(&mut self) -> $PeripheralType<Enabled> {
                    if $i <= 31 {
                        let shift = $i;
                        unsafe {self.pcer0().write_with_zero(|w| w.bits(1 << shift) )};
                    }
                    else {
                        let shift = ($i - 32);
                        unsafe {self.pcer1().write_with_zero(|w| w.bits(1 << shift) )};
                    }
                    $PeripheralType { _state: PhantomData }
                }

                #[cfg(feature = "atsam4n")]
                pub fn into_enabled_clock(&mut self) -> $PeripheralType<Enabled> {
                    let shift = $i;
                    unsafe {self.pcer0().write_with_zero(|w| w.bits(1 << shift) )};
                    $PeripheralType { _state: PhantomData }
                }

                #[cfg(not(feature = "atsam4n"))]
                pub fn into_disabled_clock(&mut self) -> $PeripheralType<Disabled> {
                    if $i <= 31 {
                        let shift = $i;
                        unsafe {self.pcdr0().write_with_zero(|w| w.bits(1 << shift) )};
                    }
                    else {
                        let shift = ($i - 32);
                        unsafe {self.pcdr1().write_with_zero(|w| w.bits(1 << shift) )};
                    }
                    $PeripheralType { _state: PhantomData }
                }

                #[cfg(feature = "atsam4n")]
                pub fn into_disabled_clock(&mut self) -> $PeripheralType<Disabled> {
                    let shift = $i;
                    unsafe {self.pcdr0().write_with_zero(|w| w.bits(1 << shift) )};
                    $PeripheralType { _state: PhantomData }
                }

                pub fn frequency(&self) -> Hertz {
                    get_master_clock_frequency()
                }
            }
        )+
    }
}

#[cfg(feature = "atsam4e")]
peripheral_clocks!(
    Uart0Clock,
    uart_0,
    7,
    SmcClock,
    smc,
    8,
    PioAClock,
    pio_a,
    9,
    PioBClock,
    pio_b,
    10,
    PioCClock,
    pio_c,
    11,
    PioDClock,
    pio_d,
    12,
    PioEClock,
    pio_e,
    13,
    Usart0Clock,
    usart_0,
    14,
    Usart1Clock,
    usart_1,
    15,
    HsmciClock,
    hsmci,
    16,
    Twi0Clock,
    twi_0,
    17,
    Twi1Clock,
    twi_1,
    18,
    SpiClock,
    spi,
    19,
    DmacClock,
    dmac,
    20,
    Tc0Clock,
    tc_0,
    21,
    Tc1Clock,
    tc_1,
    22,
    Tc2Clock,
    tc_2,
    23,
    Tc3Clock,
    tc_3,
    24,
    Tc4Clock,
    tc_4,
    25,
    Tc5Clock,
    tc_5,
    26,
    Tc6Clock,
    tc_6,
    27,
    Tc7Clock,
    tc_7,
    28,
    Tc8Clock,
    tc_8,
    29,
    Afec0Clock,
    afec_0,
    30,
    Afec1Clock,
    afec_1,
    31,
    DaccClock,
    dacc,
    32,
    AccClock,
    acc,
    33,
    UdpClock,
    udp,
    35,
    PwmClock,
    pwm,
    36,
    Can0Clock,
    can_0,
    37,
    Can1Clock,
    can_1,
    38,
    AesClock,
    aes,
    39,
    GmacClock,
    gmac,
    44,
    Uart1Clock,
    uart_1,
    45,
);

#[cfg(feature = "atsam4n")]
peripheral_clocks!(
    Uart0Clock,
    uart_0,
    8,
    Uart1Clock,
    uart_1,
    9,
    Uart2Clock,
    uart_2,
    10,
    PioAClock,
    pio_a,
    11,
    PioBClock,
    pio_b,
    12,
    PioCClock,
    pio_c,
    13,
    Usart0Clock,
    usart_0,
    14,
    Usart1Clock,
    usart_1,
    15,
    Uart3Clock,
    uart_3,
    16,
    Usart2Clock,
    usart_2,
    17,
    Twi0Clock,
    twi_0,
    19,
    Twi1Clock,
    twi_1,
    20,
    SpiClock,
    spi,
    21,
    Twi2Clock,
    twi_2,
    22,
    Tc0Clock,
    tc_0,
    23,
    Tc1Clock,
    tc_1,
    24,
    Tc2Clock,
    tc_2,
    25,
    Tc3Clock,
    tc_3,
    26,
    Tc4Clock,
    tc_4,
    27,
    Tc5Clock,
    tc_5,
    28,
    AdcClock,
    adc,
    29,
    DaccClock,
    dacc,
    30,
    PwmClock,
    pwm,
    31,
);

#[cfg(feature = "atsam4s")]
peripheral_clocks!(
    Uart0Clock,
    uart_0,
    8,
    Uart1Clock,
    uart_1,
    9,
    SmcClock,
    smc,
    10,
    PioAClock,
    pio_a,
    11,
    PioBClock,
    pio_b,
    12,
    PioCClock,
    pio_c,
    13,
    Usart0Clock,
    usart_0,
    14,
    Usart1Clock,
    usart_1,
    15,
    HsmciClock,
    hsmci,
    18,
    Twi0Clock,
    twi_0,
    19,
    Twi1Clock,
    twi_1,
    20,
    SpiClock,
    spi,
    21,
    SscClock,
    ssc,
    22,
    Tc0Clock,
    tc_0,
    23,
    Tc1Clock,
    tc_1,
    24,
    Tc2Clock,
    tc_2,
    25,
    Tc3Clock,
    tc_3,
    26,
    Tc4Clock,
    tc_4,
    27,
    Tc5Clock,
    tc_5,
    28,
    AdcClock,
    adc,
    29,
    DaccClock,
    dacc,
    30,
    PwmClock,
    pwm,
    31,
    CrccuClock,
    crccu,
    32,
    AccClock,
    acc,
    33,
    UdpClock,
    udp,
    34,
);

pub struct ClockController {
    pub peripheral_clocks: PeripheralClocks,
    pub pmc: PMC,
    master_clock: Hertz,
    slow_clock: Hertz,
}

impl ClockController {
    pub fn new(
        pmc: PMC,
        supc: &SUPC,
        #[cfg(any(feature = "atsam4e", feature = "atsam4n"))] efc: &EFC,
        #[cfg(feature = "atsam4s")] efc0: &EFC0,
        #[cfg(feature = "atsam4sd")] efc1: &EFC1,
        main_clock: MainClock,
        slow_clock: SlowClock,
    ) -> Self {
        // Make sure write protection has been disabled
        pmc.pmc_wpmr
            .modify(|_, w| w.wpkey().passwd().wpen().clear_bit());

        set_flash_wait_states_to_maximum(
            #[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
            efc,
            #[cfg(feature = "atsam4s")]
            efc0,
            #[cfg(feature = "atsam4sd")]
            efc1,
        );
        let slow_clock_frequency = setup_slow_clock(supc, slow_clock);
        let master_clock_frequency = setup_main_clock(&pmc, main_clock);
        set_flash_wait_states_to_match_frequency(
            #[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
            efc,
            #[cfg(feature = "atsam4s")]
            efc0,
            #[cfg(feature = "atsam4sd")]
            efc1,
            master_clock_frequency,
        );

        // Setup USB clock
        #[cfg(feature = "usb")]
        match main_clock {
            // TODO (HaaTa): Does USB even work without a crystal oscillator?
            //               The bootloader requires an external oscillator for USB to work.
            #[cfg(not(feature = "atsam4n"))]
            MainClock::RcOscillator4Mhz => {}
            MainClock::RcOscillator8Mhz => {}
            MainClock::RcOscillator12Mhz => {}
            MainClock::Crystal12Mhz | MainClock::Crystal16Mhz => {
                // PLLA
                // 240 MHz / 5 = 48 MHz
                // This works for both sam4s and sam4e as sam4e only has 1 pll (sam4s has 2)
                // However, using plla and pllb, lower current usage can be achieved on sam4s
                // Per the datasheet ~1 mA
                // NOTE: the datasheet indicates divider is USBDIV + 1
                #[cfg(feature = "atsam4e")]
                {
                    let usbdiv = 5;
                    pmc.pmc_usb
                        .modify(|_, w| unsafe { w.usbdiv().bits(usbdiv - 1) });
                }

                // Use PLLB for sam4s
                // 96 MHz / 2 = 48 MHz
                // NOTE: the datasheet indicates divider is USBDIV + 1
                #[cfg(feature = "atsam4s")]
                {
                    wait_for_pllb_lock(&pmc);

                    let usbdiv = 2;
                    pmc.pmc_usb
                        .modify(|_, w| unsafe { w.usbs().set_bit().usbdiv().bits(usbdiv - 1) });
                }
            }
        }

        unsafe {
            MASTER_CLOCK_FREQUENCY = master_clock_frequency;
        }

        ClockController {
            peripheral_clocks: PeripheralClocks::new(),
            pmc,
            master_clock: master_clock_frequency,
            slow_clock: slow_clock_frequency,
        }
    }

    pub fn master_clock(self) -> Hertz {
        self.master_clock
    }

    pub fn slow_clock(self) -> Hertz {
        self.slow_clock
    }
}
