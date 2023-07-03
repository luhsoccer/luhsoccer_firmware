#![allow(clippy::upper_case_acronyms)]
#![cfg(any(feature = "atsam4_c", feature = "atsam4e_e"))]

use {
    crate::clock::{Enabled, SmcClock},
    crate::gpio::*,
    crate::pac::{smc, SMC},
    core::marker::PhantomData,
    paste::paste,
};

// Chip Select Mode
pub struct Uninitialized;
pub struct Configured;

#[derive(Copy, Clone, defmt::Format)]
pub enum WaitMode {
    Frozen,
    Ready,
}

#[derive(Copy, Clone, defmt::Format)]
pub enum PageSize {
    FourBytes,
    EightBytes,
    SixteenBytes,
    ThirtyTwoBytes,
}

#[derive(Copy, Clone, defmt::Format)]
pub enum AccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

pub struct ChipSelectConfiguration {
    // 27.16 in SAM4E datasheet for details.

    // Setup parameters
    pub nwe_setup_length: u8,
    pub ncs_write_setup_length: u8,
    pub nrd_setup_length: u8,
    pub ncs_read_setup_length: u8,

    // Pulse parameters
    pub nwe_pulse_length: u8,
    pub ncs_write_pulse_length: u8,
    pub nrd_pulse_length: u8,
    pub ncs_read_pulse_length: u8,

    pub nwe_total_cycle_length: u16,
    pub nrd_total_cycle_length: u16,

    pub access_mode: AccessMode,

    pub wait_mode: Option<WaitMode>, // If some(), wait mode is as specified, otherwise disabled.

    pub data_float_time: u8,

    pub tdf_optimization: bool,
    pub page_size: Option<PageSize>, // If some(), page mode is enabled with the given size.
}

macro_rules! chip_select {
    (
        $ChipSelectType:ident,
        $cs:expr
    ) => {
        pub struct $ChipSelectType<MODE> {
            _mode: PhantomData<MODE>,
        }

        paste! {
            impl<MODE> $ChipSelectType<MODE> {
                pub(crate) fn setup(&mut self) -> &smc::[<SETUP $cs>] {
                    unsafe { &(*SMC::ptr()).[<setup $cs>] }
                }

                pub(crate) fn pulse(&mut self) -> &smc::[<PULSE $cs>] {
                    unsafe { &(*SMC::ptr()).[<pulse $cs>] }
                }

                pub(crate) fn cycle(&mut self) -> &smc::[<CYCLE $cs>] {
                    unsafe { &(*SMC::ptr()).[<cycle $cs>] }
                }

                pub(crate) fn mode(&mut self) -> &smc::[<MODE $cs>] {
                    unsafe { &(*SMC::ptr()).[<mode $cs>] }
                }

                pub fn into_configured_state(mut self, config: &ChipSelectConfiguration) -> $ChipSelectType<Configured> {
                    self.setup().write(|w| unsafe {
                        w.nwe_setup().bits(config.nwe_setup_length).
                          ncs_wr_setup().bits(config.ncs_write_setup_length).
                          nrd_setup().bits(config.nrd_setup_length).
                          ncs_rd_setup().bits(config.ncs_read_setup_length)
                    });

                    self.pulse().write(|w| unsafe {
                        w.nwe_pulse().bits(config.nwe_pulse_length).
                          ncs_wr_pulse().bits(config.ncs_write_pulse_length).
                          nrd_pulse().bits(config.nrd_pulse_length).
                          ncs_rd_pulse().bits(config.ncs_read_pulse_length)
                    });

                    self.cycle().write(|w| unsafe {
                        w.nwe_cycle().bits(config.nwe_total_cycle_length).
                          nrd_cycle().bits(config.nrd_total_cycle_length)
                    });

                    // WARNING: Mode register *must* be writen after the above registers in order to
                    // 'validate' the new configuration.    See 27.11.3.1 in datasheet.
                    self.mode().write(|w| unsafe {
                        match config.access_mode {
                            AccessMode::ReadOnly => w.read_mode().set_bit(),
                            AccessMode::WriteOnly => w.write_mode().set_bit(),
                            AccessMode::ReadWrite => w.read_mode().set_bit().write_mode().set_bit(),
                        };

                        if let Some(wait_mode) = config.wait_mode {
                            let mode = match wait_mode {
                                WaitMode::Frozen => 2,
                                WaitMode::Ready => 3,
                            };
                            w.exnw_mode().bits(mode);
                        }
                        else {
                            w.exnw_mode().bits(0);
                        }

                        w.tdf_cycles().bits(config.data_float_time);

                        if (config.tdf_optimization) {
                            w.tdf_mode().set_bit();
                        }
                        else {
                            w.tdf_mode().clear_bit();
                        }

                        if let Some(page_size) = config.page_size {
                            let value = match page_size {
                                PageSize::FourBytes => 0,
                                PageSize::EightBytes => 1,
                                PageSize::SixteenBytes => 2,
                                PageSize::ThirtyTwoBytes => 3,
                            };

                            w.pmen().set_bit().ps().bits(value);
                        }
                        else {
                            w.pmen().clear_bit().ps().bits(0);
                        }

                        w
                    });

                    $ChipSelectType { _mode: PhantomData }
                }
            }
        }
    }
}

chip_select!(ChipSelect0, 0);
chip_select!(ChipSelect1, 1);
chip_select!(ChipSelect2, 2);
chip_select!(ChipSelect3, 3);

pub struct Smc {
    pub chip_select0: ChipSelect0<Uninitialized>,
    pub chip_select1: ChipSelect1<Uninitialized>,
    pub chip_select2: ChipSelect2<Uninitialized>,
    pub chip_select3: ChipSelect3<Uninitialized>,
}

pub enum NCS1 {
    C15(Pc15<PfA>),

    #[cfg(feature = "atsam4e")]
    D18(Pd18<PfA>),
}

pub enum NCS3 {
    C12(Pc12<PfA>),

    #[cfg(feature = "atsam4e")]
    D19(Pd19<PfA>),
}

type DataLines = (
    Pc0<PfA>,
    Pc1<PfA>,
    Pc2<PfA>,
    Pc3<PfA>,
    Pc4<PfA>,
    Pc5<PfA>,
    Pc6<PfA>,
    Pc7<PfA>,
);

type AddressLines = (
    Pc18<PfA>,
    Pc19<PfA>,
    Pc20<PfA>,
    Pc21<PfA>,
    Pc22<PfA>,
    Pc23<PfA>,
    Pc24<PfA>,
    Pc25<PfA>,
    Pc26<PfA>,
    Pc27<PfA>,
    Pc28<PfA>,
    Pc29<PfA>,
    Pc30<PfA>,
    Pc31<PfA>,
    Pa18<PfC>,
    Pa19<PfC>,
    Pa20<PfC>,
    Pa0<PfC>,
    Pa1<PfC>,
    Pa23<PfC>,
    Pa24<PfC>,
    Pc16<PfA>,
    Pc17<PfA>,
    Pa25<PfC>,
);

impl Smc {
    pub fn new(
        _clock: SmcClock<Enabled>,

        _ncs1: NCS1,
        _ncs3: NCS3,

        _nrd: Pc11<PfA>,
        _nwe: Pc8<PfA>,

        _data_lines: DataLines,
        _address_lines: AddressLines,
    ) -> Self {
        Smc {
            chip_select0: ChipSelect0::<Uninitialized> { _mode: PhantomData },
            chip_select1: ChipSelect1::<Uninitialized> { _mode: PhantomData },
            chip_select2: ChipSelect2::<Uninitialized> { _mode: PhantomData },
            chip_select3: ChipSelect3::<Uninitialized> { _mode: PhantomData },
        }
    }

    pub fn base_address(&self, chip_select: u8) -> usize {
        match chip_select {
            0 => 0x6000_0000,
            1 => 0x6100_0000,
            2 => 0x6200_0000,
            3 => 0x6300_0000,
            _ => panic!("Unrecognized chip select provided: {}", chip_select),
        }
    }
}
