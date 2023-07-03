//! General Purpose Input / Output
use {
    core::convert::Infallible,
    core::marker::PhantomData,
    hal::digital::v2::{toggleable, InputPin, IoPin, OutputPin, PinState, StatefulOutputPin},
    paste::paste,
};

#[cfg(feature = "atsam4e")]
use {
    crate::clock::PioDClock,
    crate::pac::{piod, PIOD},
};

#[cfg(feature = "atsam4e_e")]
use {
    crate::clock::{PioCClock, PioEClock},
    crate::pac::{pioc, pioe, PIOC, PIOE},
};

#[cfg(any(feature = "atsam4e", feature = "atsam4n", feature = "atsam4s"))]
use {
    crate::clock::{Enabled, PioAClock, PioBClock},
    crate::pac::MATRIX,
    crate::pac::{pioa, piob, PIOA, PIOB},
};

#[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c"))]
use {
    crate::clock::PioCClock,
    crate::pac::{pioc, PIOC},
};

/// The GpioExt trait allows splitting the PORT hardware into
/// its constituent pin parts.
pub trait GpioExt {
    type Parts;

    /// Consume and split the device into its constituent parts
    fn split(self) -> Self::Parts;
}
pub struct Ports {
    pioa: PhantomData<(PIOA, PioAClock<Enabled>)>,
    piob: PhantomData<(PIOB, PioBClock<Enabled>)>,
    #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
    pioc: PhantomData<(PIOC, PioCClock<Enabled>)>,
    #[cfg(feature = "atsam4e")]
    piod: PhantomData<(PIOD, PioDClock<Enabled>)>,
    #[cfg(feature = "atsam4e_e")]
    pioe: PhantomData<(PIOE, PioEClock<Enabled>)>,
}

impl Ports {
    pub fn new(
        _pioa: (PIOA, PioAClock<Enabled>),
        _piob: (PIOB, PioBClock<Enabled>),
        #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))] _pioc: (
            PIOC,
            PioCClock<Enabled>,
        ),
        #[cfg(feature = "atsam4e")] _piod: (PIOD, PioDClock<Enabled>),
        #[cfg(feature = "atsam4e_e")] _pioe: (PIOE, PioEClock<Enabled>),
    ) -> Self {
        // The above arguments are consumed here...never to be seen again.
        Ports {
            pioa: PhantomData,
            piob: PhantomData,
            #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
            pioc: PhantomData,
            #[cfg(feature = "atsam4e")]
            piod: PhantomData,
            #[cfg(feature = "atsam4e_e")]
            pioe: PhantomData,
        }
    }
}

/// Represents a pin configured for input.
/// The MODE type is typically one of `Floating`, `PullDown` or
/// `PullUp`.
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Represents a pin configured for output.
/// The MODE type is typically one of `PushPull`, or
/// `OpenDrain`.
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Peripheral Function A
pub struct PfA;
/// Peripheral Function B
pub struct PfB;
/// Peripheral Function C
pub struct PfC;
/// Peripheral Function D
pub struct PfD;
/// System Function
pub struct SysFn;
/// Extra Function
pub struct ExFn;

/// Floating Input
pub struct Floating;
/// Pulled down Input
pub struct PullDown;
/// Pulled up Input
pub struct PullUp;

/// Totem Pole aka Push-Pull
pub struct PushPull;
/// Open drain output
pub struct OpenDrain;

macro_rules! pins {
    ([
        $($PinTypeA:ident: ($pin_identA:ident, $pin_noA:expr, $extfnA:ident),)*
    ],[
        $($PinTypeB:ident: ($pin_identB:ident, $pin_noB:expr, $extfnB:ident, $sysioB:ident),)*
    ],[
        $($PinTypeC:ident: ($pin_identC:ident, $pin_noC:expr, $extfnC:ident),)*
    ],[
        $($PinTypeD:ident: ($pin_identD:ident, $pin_noD:expr),)*
    ],[
        $($PinTypeE:ident: ($pin_identE:ident, $pin_noE:expr),)*
    ]) => {
        /// Holds the GPIO broken out pin instances (consumes the Ports object)
        pub struct Pins {
            $(
                /// Pin $pin_identA
                pub $pin_identA: $PinTypeA<Input<Floating>>,
            )*
            $(
                /// Pin $pin_identB
                pub $pin_identB: $PinTypeB<Input<Floating>>,
            )*
            $(
                /// Pin $pin_identC
                #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
                pub $pin_identC: $PinTypeC<Input<Floating>>,
            )*
            $(
                /// Pin $pin_identD
                #[cfg(feature = "atsam4e")]
                pub $pin_identD: $PinTypeD<Input<Floating>>,
            )*
            $(
                /// Pin $pin_identE
                #[cfg(feature = "atsam4e_e")]
                pub $pin_identE: $PinTypeE<Input<Floating>>,
            )*
        }

        impl GpioExt for Ports {
            type Parts = Pins;

            /// Split the PORT peripheral into discrete pins
            fn split(self) -> Pins {
                Pins {
                    $(
                        $pin_identA: $PinTypeA { _mode: PhantomData },
                    )*
                    $(
                        $pin_identB: $PinTypeB { _mode: PhantomData },
                    )*
                    $(
                        #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
                        $pin_identC: $PinTypeC { _mode: PhantomData },
                    )*
                    $(
                        #[cfg(feature = "atsam4e")]
                        $pin_identD: $PinTypeD { _mode: PhantomData },
                    )*
                    $(
                        #[cfg(feature = "atsam4e_e")]
                        $pin_identE: $PinTypeE { _mode: PhantomData },
                    )*
                }
            }
        }

        $(
            pin!($PinTypeA, $pin_identA, $pin_noA, PIOA, pioa);
            pin_sysio!($PinTypeA, $pin_noA, false);
            pin_extrafn!($PinTypeA, $extfnA);
        )*
        pin_generic!(PIOA);
        $(
            pin!($PinTypeB, $pin_identB, $pin_noB, PIOB, piob);
            pin_sysio!($PinTypeB, $pin_noB, $sysioB);
            pin_extrafn!($PinTypeB, $extfnB);
        )*
        pin_generic!(PIOB);
        $(
            #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
            pin!($PinTypeC, $pin_identC, $pin_noC, PIOC, pioc);
            #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
            pin_sysio!($PinTypeC, $pin_noC, false);
            #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
            pin_extrafn!($PinTypeC, $extfnC);
        )*
        #[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e_e"))]
        pin_generic!(PIOC);
        $(
            #[cfg(feature = "atsam4e")]
            pin!($PinTypeD, $pin_identD, $pin_noD, PIOD, piod);
            #[cfg(feature = "atsam4e")]
            pin_sysio!($PinTypeD, $pin_noD, false);
        )*
        #[cfg(feature = "atsam4e")]
        pin_generic!(PIOD);
        $(
            #[cfg(feature = "atsam4e_e")]
            pin!($PinTypeE, $pin_identE, $pin_noE, PIOE, pioe);
            #[cfg(feature = "atsam4e_e")]
            pin_sysio!($PinTypeE, $pin_noE, false);
        )*
        #[cfg(feature = "atsam4e_e")]
        pin_generic!(PIOE);
    };
}

/// Extra Function
/// These function do not do any setup, they are solely for assigning Rust ownership to specific
/// pins based on their intended use. To simplify, all functions (even if there are multiple)
/// are defined as ExFn.
macro_rules! pin_extrafn {
    (
        $PinType:ident,
        true
    ) => {
        impl<MODE> $PinType<MODE> {
            pub fn into_extra_function(self, _matrix: &MATRIX) -> $PinType<ExFn> {
                $PinType { _mode: PhantomData }
            }
        }
    };
    (
        $PinType:ident,
        false
    ) => {};
}

/// System I/O Configuration setup
/// Some IO pins have an extra configuration used to disable some special default peripheral config
/// This must be set in order to use those pins as GPIO (or other peripherals).
/// The macro here adds the necessary register calls to make this happen for those pins.
macro_rules! pin_sysio {
    (
        $PinType:ident,
        $i:expr,
        false
    ) => {
        impl<MODE> $PinType<MODE> {
            fn prepare_pin_for_function_use(&mut self) {
                unsafe {
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable Pullup
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable Pulldown
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable Multi-drive (open drain)
                    self.ifscdr().write_with_zero(|w| w.bits(1 << $i)); // Disable Glitch filter (Debounce)
                }
            }

            fn prepare_pin_for_input_use(&mut self) {
                self.disable_pin_interrupt(); // Disable interrupt
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.odr().write_with_zero(|w| w.bits(1 << $i)); // Disable output mode
                }
            }

            pub fn into_peripheral_function_a(mut self, _matrix: &MATRIX) -> $PinType<PfA> {
                self.prepare_pin_for_function_use();
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) });
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_peripheral_function_b(mut self, _matrix: &MATRIX) -> $PinType<PfB> {
                self.prepare_pin_for_function_use();
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) }); // Set up peripheral function
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_peripheral_function_c(mut self, _matrix: &MATRIX) -> $PinType<PfC> {
                self.prepare_pin_for_function_use();
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) }); // Set up peripheral function
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_peripheral_function_d(mut self, _matrix: &MATRIX) -> $PinType<PfD> {
                self.prepare_pin_for_function_use();
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) }); // Set up peripheral function
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_floating_input(mut self, _matrix: &MATRIX) -> $PinType<Input<Floating>> {
                self.prepare_pin_for_input_use();
                unsafe {
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-up
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-down
                }
                self.enable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_pull_down_input(mut self, _matrix: &MATRIX) -> $PinType<Input<PullDown>> {
                self.prepare_pin_for_input_use();
                unsafe {
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-up (this must happen first when enabling pull-down resistors)
                    self.ppder().write_with_zero(|w| w.bits(1 << $i)); // Enable pull-down
                }
                self.enable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_pull_up_input(mut self, _matrix: &MATRIX) -> $PinType<Input<PullUp>> {
                self.prepare_pin_for_input_use();
                unsafe {
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-down
                    self.puer().write_with_zero(|w| w.bits(1 << $i)); // Enable pull-up
                }
                self.enable_pin();

                $PinType { _mode: PhantomData }
            }

            /// Configures the pin to operate as an open drain output
            pub fn into_open_drain_output(
                mut self,
                _matrix: &MATRIX,
            ) -> $PinType<Output<OpenDrain>> {
                self.disable_pin_interrupt();
                unsafe {
                    self.mder().write_with_zero(|w| w.bits(1 << $i)); // Enable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                }
                self.enable_pin(); // Enable pio mode (disables peripheral control of pin)

                $PinType { _mode: PhantomData }
            }

            /// Configures the pin to operate as a push-pull output
            pub fn into_push_pull_output(mut self, _matrix: &MATRIX) -> $PinType<Output<PushPull>> {
                self.disable_pin_interrupt();
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                }

                $PinType { _mode: PhantomData }
            }
        }
    };

    (
        $PinType:ident,
        $i:expr,
        true
    ) => {
        impl<MODE> $PinType<MODE> {
            paste! {
                /// Clears bit to enable system function
                pub fn into_system_function(self, matrix: &MATRIX) -> $PinType<SysFn> {
                    matrix.ccfg_sysio.modify(|_, w| w.[<sysio $i>]().clear_bit());

                    $PinType { _mode: PhantomData }
                }

                /// Sets bit to disable system function
                fn disable_system_function(&self, matrix: &MATRIX) {
                    matrix.ccfg_sysio.modify(|_, w| w.[<sysio $i>]().set_bit());
                }
            }

            fn prepare_pin_for_function_use(&mut self) {
                unsafe {
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable Pullup
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable Pulldown
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable Multi-drive (open drain)
                    self.ifscdr().write_with_zero(|w| w.bits(1 << $i)); // Disable Glitch filter (Debounce)
                }
            }

            fn prepare_pin_for_input_use(&mut self) {
                self.disable_pin_interrupt(); // Disable interrupt
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.odr().write_with_zero(|w| w.bits(1 << $i)); // Disable output mode
                }
            }

            pub fn into_peripheral_function_a(mut self, matrix: &MATRIX) -> $PinType<PfA> {
                self.prepare_pin_for_function_use();
                self.disable_system_function(matrix);
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) });
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_peripheral_function_b(mut self, matrix: &MATRIX) -> $PinType<PfB> {
                self.prepare_pin_for_function_use();
                self.disable_system_function(matrix);
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) }); // Set up peripheral function
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_peripheral_function_c(mut self, matrix: &MATRIX) -> $PinType<PfC> {
                self.prepare_pin_for_function_use();
                self.disable_system_function(matrix);
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << $i)) }); // Set up peripheral function
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_peripheral_function_d(mut self, matrix: &MATRIX) -> $PinType<PfD> {
                self.prepare_pin_for_function_use();
                self.disable_system_function(matrix);
                self.abcdsr1()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) }); // Set up peripheral function
                self.abcdsr2()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << $i)) });
                self.disable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_floating_input(mut self, matrix: &MATRIX) -> $PinType<Input<Floating>> {
                self.prepare_pin_for_input_use();
                self.disable_system_function(matrix);
                unsafe {
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-up
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-down
                }
                self.enable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_pull_down_input(mut self, matrix: &MATRIX) -> $PinType<Input<PullDown>> {
                self.prepare_pin_for_input_use();
                self.disable_system_function(matrix);
                unsafe {
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-up (this must happen first when enabling pull-down resistors)
                    self.ppder().write_with_zero(|w| w.bits(1 << $i)); // Enable pull-down
                }
                self.enable_pin();

                $PinType { _mode: PhantomData }
            }

            pub fn into_pull_up_input(mut self, matrix: &MATRIX) -> $PinType<Input<PullUp>> {
                self.prepare_pin_for_input_use();
                self.disable_system_function(matrix);
                unsafe {
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-down
                    self.puer().write_with_zero(|w| w.bits(1 << $i)); // Enable pull-up
                }
                self.enable_pin();

                $PinType { _mode: PhantomData }
            }

            /// Configures the pin to operate as an open drain output
            pub fn into_open_drain_output(
                mut self,
                matrix: &MATRIX,
            ) -> $PinType<Output<OpenDrain>> {
                self.disable_pin_interrupt();
                self.disable_system_function(matrix);
                unsafe {
                    self.mder().write_with_zero(|w| w.bits(1 << $i)); // Enable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                }
                self.enable_pin(); // Enable pio mode (disables peripheral control of pin)

                $PinType { _mode: PhantomData }
            }

            /// Configures the pin to operate as a push-pull output
            pub fn into_push_pull_output(mut self, matrix: &MATRIX) -> $PinType<Output<PushPull>> {
                self.disable_pin_interrupt();
                self.disable_system_function(matrix);
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                }

                $PinType { _mode: PhantomData }
            }
        }
    };
}

macro_rules! pin {
    (
        $PinType:ident,
        $pin_ident:ident,
        $i:expr,
        $PIO:ident,
        $pio:ident
    ) => {
        pub struct $PinType<MODE> {
            _mode: PhantomData<MODE>,
        }

        impl<MODE> $PinType<MODE> {
            pub(crate) fn puer(&mut self) -> &$pio::PUER {
                unsafe { &(*$PIO::ptr()).puer }
            }

            pub(crate) fn pudr(&mut self) -> &$pio::PUDR {
                unsafe { &(*$PIO::ptr()).pudr }
            }

            pub(crate) fn _ier(&mut self) -> &$pio::IER {
                unsafe { &(*$PIO::ptr()).ier }
            }

            pub(crate) fn idr(&mut self) -> &$pio::IDR {
                unsafe { &(*$PIO::ptr()).idr }
            }

            pub(crate) fn ppder(&mut self) -> &$pio::PPDER {
                unsafe { &(*$PIO::ptr()).ppder }
            }

            pub(crate) fn ppddr(&mut self) -> &$pio::PPDDR {
                unsafe { &(*$PIO::ptr()).ppddr }
            }

            pub(crate) fn abcdsr1(&mut self) -> &$pio::ABCDSR {
                unsafe { &(*$PIO::ptr()).abcdsr[0] }
            }

            pub(crate) fn abcdsr2(&mut self) -> &$pio::ABCDSR {
                unsafe { &(*$PIO::ptr()).abcdsr[1] }
            }

            pub(crate) fn mder(&mut self) -> &$pio::MDER {
                unsafe { &(*$PIO::ptr()).mder }
            }

            pub(crate) fn mddr(&mut self) -> &$pio::MDDR {
                unsafe { &(*$PIO::ptr()).mddr }
            }

            pub(crate) fn oer(&mut self) -> &$pio::OER {
                unsafe { &(*$PIO::ptr()).oer }
            }

            pub(crate) fn odr(&mut self) -> &$pio::ODR {
                unsafe { &(*$PIO::ptr()).odr }
            }

            pub(crate) fn per(&mut self) -> &$pio::PER {
                unsafe { &(*$PIO::ptr()).per }
            }

            pub(crate) fn pdr(&mut self) -> &$pio::PDR {
                unsafe { &(*$PIO::ptr()).pdr }
            }

            pub(crate) fn sodr(&mut self) -> &$pio::SODR {
                unsafe { &(*$PIO::ptr()).sodr }
            }

            pub(crate) fn codr(&mut self) -> &$pio::CODR {
                unsafe { &(*$PIO::ptr()).codr }
            }

            pub(crate) fn ifscdr(&mut self) -> &$pio::IFSCDR {
                unsafe { &(*$PIO::ptr()).ifscdr }
            }

            pub(crate) fn odsr(&self) -> &$pio::ODSR {
                unsafe { &(*$PIO::ptr()).odsr }
            }

            pub(crate) fn pdsr(&self) -> &$pio::PDSR {
                unsafe { &(*$PIO::ptr()).pdsr }
            }

            fn enable_pin(&mut self) {
                unsafe { self.per().write_with_zero(|w| w.bits(1 << $i)) };
            }

            fn disable_pin(&mut self) {
                unsafe { self.pdr().write_with_zero(|w| w.bits(1 << $i)) };
            }

            fn _enable_pin_interrupt(&mut self) {
                unsafe { self._ier().write_with_zero(|w| w.bits(1 << $i)) };
            }

            fn disable_pin_interrupt(&mut self) {
                unsafe { self.idr().write_with_zero(|w| w.bits(1 << $i)) };
            }
        }

        impl<MODE> InputPin for $PinType<Input<MODE>> {
            type Error = Infallible;

            fn is_high(&self) -> Result<bool, Self::Error> {
                Ok(self.pdsr().read().bits() & (1 << $i) != 0)
            }

            fn is_low(&self) -> Result<bool, Self::Error> {
                Ok(self.pdsr().read().bits() & (1 << $i) == 0)
            }
        }

        impl<MODE> OutputPin for $PinType<Output<MODE>> {
            type Error = Infallible;

            fn set_high(&mut self) -> Result<(), Self::Error> {
                unsafe { self.sodr().write_with_zero(|w| w.bits(1 << $i)) };
                Ok(())
            }

            fn set_low(&mut self) -> Result<(), Self::Error> {
                unsafe { self.codr().write_with_zero(|w| w.bits(1 << $i)) };
                Ok(())
            }
        }

        impl<MODE> StatefulOutputPin for $PinType<Output<MODE>> {
            fn is_set_high(&self) -> Result<bool, Self::Error> {
                Ok(self.odsr().read().bits() & (1 << $i) != 0)
            }

            fn is_set_low(&self) -> Result<bool, Self::Error> {
                Ok(self.odsr().read().bits() & (1 << $i) == 0)
            }
        }

        /// Software toggle (uses StatefulOutputPin and OutputPin)
        impl<MODE> toggleable::Default for $PinType<Output<MODE>> {}

        impl IoPin<$PinType<Input<Floating>>, Self> for $PinType<Output<PushPull>> {
            type Error = Infallible;
            fn into_input_pin(mut self) -> Result<$PinType<Input<Floating>>, Self::Error> {
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.odr().write_with_zero(|w| w.bits(1 << $i)); // Disable output mode
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-up
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-down
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                }

                Ok($PinType { _mode: PhantomData })
            }
            fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                self.set_state(state).unwrap();
                Ok(self)
            }
        }

        impl IoPin<$PinType<Input<PullDown>>, Self> for $PinType<Output<PushPull>> {
            type Error = Infallible;
            fn into_input_pin(mut self) -> Result<$PinType<Input<PullDown>>, Self::Error> {
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.odr().write_with_zero(|w| w.bits(1 << $i)); // Disable output mode
                    self.pudr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-up
                    self.ppder().write_with_zero(|w| w.bits(1 << $i)); // Enable pull-down
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                }

                Ok($PinType { _mode: PhantomData })
            }
            fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                self.set_state(state).unwrap();
                Ok(self)
            }
        }

        impl IoPin<$PinType<Input<PullUp>>, Self> for $PinType<Output<PushPull>> {
            type Error = Infallible;
            fn into_input_pin(mut self) -> Result<$PinType<Input<PullUp>>, Self::Error> {
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.odr().write_with_zero(|w| w.bits(1 << $i)); // Disable output mode
                    self.puer().write_with_zero(|w| w.bits(1 << $i)); // Enable pull-up
                    self.ppddr().write_with_zero(|w| w.bits(1 << $i)); // Disable pull-down
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                }

                Ok($PinType { _mode: PhantomData })
            }
            fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                self.set_state(state).unwrap();
                Ok(self)
            }
        }

        impl IoPin<Self, $PinType<Output<PushPull>>> for $PinType<Input<Floating>> {
            type Error = Infallible;
            fn into_input_pin(self) -> Result<Self, Self::Error> {
                Ok(self)
            }
            fn into_output_pin(
                mut self,
                state: PinState,
            ) -> Result<$PinType<Output<PushPull>>, Self::Error> {
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                    match state {
                        PinState::Low => {
                            self.codr().write_with_zero(|w| w.bits(1 << $i));
                        }
                        PinState::High => {
                            self.sodr().write_with_zero(|w| w.bits(1 << $i));
                        }
                    }
                }

                Ok($PinType { _mode: PhantomData })
            }
        }

        impl IoPin<Self, $PinType<Output<PushPull>>> for $PinType<Input<PullDown>> {
            type Error = Infallible;
            fn into_input_pin(self) -> Result<Self, Self::Error> {
                Ok(self)
            }
            fn into_output_pin(
                mut self,
                state: PinState,
            ) -> Result<$PinType<Output<PushPull>>, Self::Error> {
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                    match state {
                        PinState::Low => {
                            self.codr().write_with_zero(|w| w.bits(1 << $i));
                        }
                        PinState::High => {
                            self.sodr().write_with_zero(|w| w.bits(1 << $i));
                        }
                    }
                }

                Ok($PinType { _mode: PhantomData })
            }
        }

        impl IoPin<Self, $PinType<Output<PushPull>>> for $PinType<Input<PullUp>> {
            type Error = Infallible;
            fn into_input_pin(self) -> Result<Self, Self::Error> {
                Ok(self)
            }
            fn into_output_pin(
                mut self,
                state: PinState,
            ) -> Result<$PinType<Output<PushPull>>, Self::Error> {
                unsafe {
                    self.mddr().write_with_zero(|w| w.bits(1 << $i)); // Disable open-drain/multi-drive
                    self.oer().write_with_zero(|w| w.bits(1 << $i)); // Enable output mode
                    self.per().write_with_zero(|w| w.bits(1 << $i)); // Enable pio mode (disables peripheral control of pin)
                    match state {
                        PinState::Low => {
                            self.codr().write_with_zero(|w| w.bits(1 << $i));
                        }
                        PinState::High => {
                            self.sodr().write_with_zero(|w| w.bits(1 << $i));
                        }
                    }
                }

                Ok($PinType { _mode: PhantomData })
            }
        }

        paste! {
            impl<MODE> $PinType<MODE> {
                /// Erases the pin number from the type
                #[inline]
                fn into_generic(self) -> [<$PIO Generic>]<MODE> {
                    [<$PIO Generic>] {
                        i: $i,
                        _mode: PhantomData,
                    }
                }

                /// Erases the pin number and port from the type
                ///
                /// This is useful when you want to collect the pins into an array where you
                /// need all the elements to have the same type
                pub fn downgrade(self) -> PioX<MODE> {
                    self.into_generic().downgrade()
                }
            }
        }
    };
}

macro_rules! pin_generic {
    (
        $port:ident
    ) => {
        paste! {
            pub struct [<$port Generic>]<MODE> {
                i: u8,
                _mode: PhantomData<MODE>,
            }

            impl<MODE> [<$port Generic>]<MODE> {
                pub fn downgrade(self) -> PioX<MODE> {
                    PioX::$port(self)
                }
            }

            impl<MODE> OutputPin for [<$port Generic>]<Output<MODE>> {
                type Error = Infallible;
                fn set_high(&mut self) -> Result<(), Self::Error> {
                    Ok(unsafe { (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << self.i) ) })
                }

                fn set_low(&mut self) -> Result<(), Self::Error> {
                    Ok(unsafe { (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << self.i) ) })
                }
            }

            impl<MODE> InputPin for [<$port Generic>]<Input<MODE>> {
                type Error = Infallible;
                fn is_high(&self) -> Result<bool, Self::Error> {
                    Ok(unsafe { (*$port::ptr()).pdsr.read().bits() & (1 << self.i)} != 0)
                }

                fn is_low(&self) -> Result<bool, Self::Error> {
                    Ok(unsafe { (*$port::ptr()).pdsr.read().bits() & (1 << self.i)} == 0)
                }
            }

            impl <MODE> StatefulOutputPin for [<$port Generic>]<Output<MODE>> {
                fn is_set_high(&self) -> Result<bool, Self::Error> {
                    Ok(unsafe { (*$port::ptr()).odsr.read().bits() & (1 << self.i)} != 0)
                }

                fn is_set_low(&self) -> Result<bool, Self::Error> {
                    Ok(unsafe { (*$port::ptr()).odsr.read().bits() & (1 << self.i)} == 0)
                }
            }

            impl <MODE> toggleable::Default for [<$port Generic>]<Output<MODE>> {}

            impl IoPin<[<$port Generic>]<Input<Floating>>, Self> for [<$port Generic>]<Output<PushPull>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<[<$port Generic>]<Input<Floating>>, Self::Error> {
                    unsafe {
                        (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable open-drain/multi-drive
                        (*$port::ptr()).odr.write_with_zero(|w| w.bits(1 << self.i)); // Disable output mode
                        (*$port::ptr()).pudr.write_with_zero(|w| w.bits(1 << self.i)); // Disable pull-up
                        (*$port::ptr()).ppddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable pull-down
                        (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << self.i)); // Enable pio mode (disables peripheral control of pin)
                    }

                    Ok([<$port Generic>] { i: self.i, _mode: PhantomData })
                }
                fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                    self.set_state(state).unwrap();
                    Ok(self)
                }
            }

            impl IoPin<[<$port Generic>]<Input<PullDown>>, Self> for [<$port Generic>]<Output<PushPull>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<[<$port Generic>]<Input<PullDown>>, Self::Error> {
                    unsafe {
                        (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable open-drain/multi-drive
                        (*$port::ptr()).odr.write_with_zero(|w| w.bits(1 << self.i)); // Disable output mode
                        (*$port::ptr()).pudr.write_with_zero(|w| w.bits(1 << self.i)); // Disable pull-up
                        (*$port::ptr()).ppder.write_with_zero(|w| w.bits(1 << self.i)); // Enable pull-down
                        (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << self.i)); // Enable pio mode (disables peripheral control of pin)
                    }

                    Ok([<$port Generic>] { i: self.i, _mode: PhantomData })
                }
                fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                    self.set_state(state).unwrap();
                    Ok(self)
                }
            }

            impl IoPin<[<$port Generic>]<Input<PullUp>>, Self> for [<$port Generic>]<Output<PushPull>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<[<$port Generic>]<Input<PullUp>>, Self::Error> {
                    unsafe {
                        (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable open-drain/multi-drive
                        (*$port::ptr()).odr.write_with_zero(|w| w.bits(1 << self.i)); // Disable output mode
                        (*$port::ptr()).puer.write_with_zero(|w| w.bits(1 << self.i)); // Enable pull-up
                        (*$port::ptr()).ppddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable pull-down
                        (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << self.i)); // Enable pio mode (disables peripheral control of pin)
                    }

                    Ok([<$port Generic>] { i: self.i, _mode: PhantomData })
                }
                fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                    self.set_state(state).unwrap();
                    Ok(self)
                }
            }

            impl IoPin<Self, [<$port Generic>]<Output<PushPull>>> for [<$port Generic>]<Input<Floating>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<Self, Self::Error> {
                    Ok(self)
                }
                fn into_output_pin(self, state: PinState) -> Result<[<$port Generic>]<Output<PushPull>>, Self::Error> {
                    unsafe {
                        (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable open-drain/multi-drive
                        (*$port::ptr()).oer.write_with_zero(|w| w.bits(1 << self.i)); // Enable output mode
                        (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << self.i)); // Enable pio mode (disables peripheral control of pin)
                        match state {
                            PinState::Low => {
                                (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << self.i) );
                            }
                            PinState::High => {
                                (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << self.i) );
                            }
                        }
                    }

                    Ok( [<$port Generic>] { i: self.i, _mode: PhantomData } )
                }
            }

            impl IoPin<Self, [<$port Generic>]<Output<PushPull>>> for [<$port Generic>]<Input<PullDown>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<Self, Self::Error> {
                    Ok(self)
                }
                fn into_output_pin(self, state: PinState) -> Result<[<$port Generic>]<Output<PushPull>>, Self::Error> {
                    unsafe {
                        (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable open-drain/multi-drive
                        (*$port::ptr()).oer.write_with_zero(|w| w.bits(1 << self.i)); // Enable output mode
                        (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << self.i)); // Enable pio mode (disables peripheral control of pin)
                        match state {
                            PinState::Low => {
                                (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << self.i) );
                            }
                            PinState::High => {
                                (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << self.i) );
                            }
                        }
                    }

                    Ok( [<$port Generic>] { i: self.i, _mode: PhantomData } )
                }
            }

            impl IoPin<Self, [<$port Generic>]<Output<PushPull>>> for [<$port Generic>]<Input<PullUp>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<Self, Self::Error> {
                    Ok(self)
                }
                fn into_output_pin(self, state: PinState) -> Result<[<$port Generic>]<Output<PushPull>>, Self::Error> {
                    unsafe {
                        (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << self.i)); // Disable open-drain/multi-drive
                        (*$port::ptr()).oer.write_with_zero(|w| w.bits(1 << self.i)); // Enable output mode
                        (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << self.i)); // Enable pio mode (disables peripheral control of pin)
                        match state {
                            PinState::Low => {
                                (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << self.i) );
                            }
                            PinState::High => {
                                (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << self.i) );
                            }
                        }
                    }

                    Ok( [<$port Generic>] { i: self.i, _mode: PhantomData } )
                }
            }
        }
    };
}

#[cfg(feature = "atsam4e")]
pins!([
    Pa0: (pa0, 0, true), // WKUP0
    Pa1: (pa1, 1, true), // WKUP1
    Pa2: (pa2, 2, true), // WKUP2
    Pa3: (pa3, 3, false),
    Pa4: (pa4, 4, true), // WKUP3
    Pa5: (pa5, 5, true), // WKUP4
    Pa6: (pa6, 6, false),
    Pa7: (pa7, 7, false),
    Pa8: (pa8, 8, true), // WKUP5
    Pa9: (pa9, 9, true), // WKUP6
    Pa10: (pa10, 10, false),
    Pa11: (pa11, 11, true), // WKUP7
    Pa12: (pa12, 12, false),
    Pa13: (pa13, 13, false),
    Pa14: (pa14, 14, true), // WKUP8
    Pa15: (pa15, 15, true), // WKUP14/PIODCEN1
    Pa16: (pa16, 16, true), // WKUP15/PIODCEN2
    Pa17: (pa17, 17, true), // AFE0_AD0
    Pa18: (pa18, 18, true), // AFE0_AD1
    Pa19: (pa19, 19, true), // AFE0_AD2/WKUP9
    Pa20: (pa20, 20, true), // AFE0_AD3/WKUP10
    Pa21: (pa21, 21, true), // AFE1_AD2
    Pa22: (pa22, 22, true), // AFE1_AD3
    Pa23: (pa23, 23, true), // PIODCCLK
    Pa24: (pa24, 24, true), // PIODC0
    Pa25: (pa25, 25, true), // PIODC1
    Pa26: (pa26, 26, true), // PIODC2
    Pa27: (pa27, 27, true), // PIODC3
    Pa28: (pa28, 28, true), // PIODC4
    Pa29: (pa29, 29, true), // PIODC5
    Pa30: (pa30, 30, true), // WKUP11/PIODC6
    Pa31: (pa31, 31, true), // PIODC7
],[
    Pb0: (pb0, 0, true, false), // AFE0_AD4/RTCOUT0
    Pb1: (pb1, 1, true, false), // AFE0_AD5/RTCOUT1
    Pb2: (pb2, 2, true, false), // AFE1_AD0/WKUP12
    Pb3: (pb3, 3, true, false), // AFE1_AD1
    Pb4: (pb4, 4, false, true), // | SYSIO4 - TDI
    Pb5: (pb5, 5, true, true), // WKUP13 | SYSIO5 - TDO/TRACESWO
    Pb6: (pb6, 6, false, true), // | SYSIO6 - TMS/SWDIO
    Pb7: (pb7, 7, false, true), // | SYSIO7 - TCK/SWCLK
    Pb8: (pb8, 8, false, false),
    Pb9: (pb9, 9, false, false),
    Pb10: (pb10, 10, false, true), // | SYSIO10 - DDM
    Pb11: (pb11, 11, false, true), // | SYSIO11 - DDP
    Pb12: (pb12, 12, false, true), // | SYSIO12 - ERASE
    Pb13: (pb13, 13, true, false), // DAC0
    Pb14: (pb14, 14, true, false), // DAC1

    // PB15-31 do not exist.
],
[
    Pc0: (pc0, 0, true), // AFE0_AD14
    Pc1: (pc1, 1, true), // AFE1_AD4
    Pc2: (pc2, 2, true), // AFE1_AD5
    Pc3: (pc3, 3, true), // AFE1_AD6
    Pc4: (pc4, 4, true), // AFE1_AD7
    Pc5: (pc5, 5, false),
    Pc6: (pc6, 6, false),
    Pc7: (pc7, 7, false),
    Pc8: (pc8, 8, false),
    Pc9: (pc9, 9, false),
    Pc10: (pc10, 10, false),
    Pc11: (pc11, 11, false),
    Pc12: (pc12, 12, true), // AFE0_AD8
    Pc13: (pc13, 13, true), // AFE0_AD6
    Pc14: (pc14, 14, false),
    Pc15: (pc15, 15, true), // AFE0_AD7
    Pc16: (pc16, 16, false),
    Pc17: (pc17, 17, false),
    Pc18: (pc18, 18, false),
    Pc19: (pc19, 19, false),
    Pc20: (pc20, 20, false),
    Pc21: (pc21, 21, false),
    Pc22: (pc22, 22, false),
    Pc23: (pc23, 23, false),
    Pc24: (pc24, 24, false),
    Pc25: (pc25, 25, false),
    Pc26: (pc26, 26, true), // AFE0_AD12
    Pc27: (pc27, 27, true), // AFE0_AD13
    Pc28: (pc28, 28, false),
    Pc29: (pc29, 29, true), // AFE0_AD9
    Pc30: (pc30, 30, true), // AFE0_AD10
    Pc31: (pc31, 31, true), // AFE0_AD11
],
[
    Pd0: (pd0, 0),
    Pd1: (pd1, 1),
    Pd2: (pd2, 2),
    Pd3: (pd3, 3),
    Pd4: (pd4, 4),
    Pd5: (pd5, 5),
    Pd6: (pd6, 6),
    Pd7: (pd7, 7),
    Pd8: (pd8, 8),
    Pd9: (pd9, 9),
    Pd10: (pd10, 10),
    Pd11: (pd11, 11),
    Pd12: (pd12, 12),
    Pd13: (pd13, 13),
    Pd14: (pd14, 14),
    Pd15: (pd15, 15),
    Pd16: (pd16, 16),
    Pd17: (pd17, 17),
    Pd18: (pd18, 18),
    Pd19: (pd19, 19),
    Pd20: (pd20, 20),
    Pd21: (pd21, 21),
    Pd22: (pd22, 22),
    Pd23: (pd23, 23),
    Pd24: (pd24, 24),
    Pd25: (pd25, 25),
    Pd26: (pd26, 26),
    Pd27: (pd27, 27),
    Pd28: (pd28, 28),
    Pd29: (pd29, 29),
    Pd30: (pd30, 30),
    Pd31: (pd31, 31),
],
[
    Pe0: (pe0, 0),
    Pe1: (pe1, 1),
    Pe2: (pe2, 2),
    Pe3: (pe3, 3),
    Pe4: (pe4, 4),
    Pe5: (pe5, 5),

    // Pe6-31 do not exist.
]);

#[cfg(feature = "atsam4n")]
pins!([
    Pa0: (pa0, 0, true), // WKUP0
    Pa1: (pa1, 1, true), // WKUP1
    Pa2: (pa2, 2, true), // WKUP2
    Pa3: (pa3, 3, false),
    Pa4: (pa4, 4, true), // WKUP3
    Pa5: (pa5, 5, true), // WKUP4
    Pa6: (pa6, 6, false),
    Pa7: (pa7, 7, false),
    Pa8: (pa8, 8, true), // WKUP5
    Pa9: (pa9, 9, true), // WKUP6
    Pa10: (pa10, 10, false),
    Pa11: (pa11, 11, true), // WKUP7
    Pa12: (pa12, 12, false),
    Pa13: (pa13, 13, false),
    Pa14: (pa14, 14, true), // WKUP8
    Pa15: (pa15, 15, true), // WKUP14
    Pa16: (pa16, 16, true), // WKUP15
    Pa17: (pa17, 17, true), // AD0
    Pa18: (pa18, 18, true), // AD1
    Pa19: (pa19, 19, true), // AD2/WKUP9
    Pa20: (pa20, 20, true), // AD3/WKUP10
    Pa21: (pa21, 21, true), // AD8
    Pa22: (pa22, 22, true), // AD9
    Pa23: (pa23, 23, false),
    Pa24: (pa24, 24, false),
    Pa25: (pa25, 25, false),
    Pa26: (pa26, 26, false),
    Pa27: (pa27, 27, false),
    Pa28: (pa28, 28, false),
    Pa29: (pa29, 29, false),
    Pa30: (pa30, 30, true), // WKUP11
    Pa31: (pa31, 31, false),
],[
    Pb0: (pb0, 0, true, false), // AD4
    Pb1: (pb1, 1, true, false), // AD5
    Pb2: (pb2, 2, true, false), // AD6/WKUP12
    Pb3: (pb3, 3, true, false), // AD7
    Pb4: (pb4, 4, false, true), // | SYSIO4 - TDI
    Pb5: (pb5, 5, true, true), // WKUP13 | SYSIO5 - TDO/TRACESWO
    Pb6: (pb6, 6, false, true), // | SYSIO6 - TMS/SWDIO
    Pb7: (pb7, 7, false, true), // | SYSIO7 - TCK/SWCLK
    Pb8: (pb8, 8, false, false),
    Pb9: (pb9, 9, false, false),
    Pb10: (pb10, 10, false, false),
    Pb11: (pb11, 11, false, false),
    Pb12: (pb12, 12, false, true), // | SYSIO12 - ERASE
    Pb13: (pb13, 13, true, false), // DAC0
    Pb14: (pb14, 14, false, false),

    // PB15-31 do not exist.
],
[
    Pc0: (pc0, 0, false),
    Pc1: (pc1, 1, false),
    Pc2: (pc2, 2, false),
    Pc3: (pc3, 3, false),
    Pc4: (pc4, 4, false),
    Pc5: (pc5, 5, false),
    Pc6: (pc6, 6, false),
    Pc7: (pc7, 7, false),
    Pc8: (pc8, 8, false),
    Pc9: (pc9, 9, false),
    Pc10: (pc10, 10, false),
    Pc11: (pc11, 11, false),
    Pc12: (pc12, 12, true), // AD12
    Pc13: (pc13, 13, true), // AD10
    Pc14: (pc14, 14, false),
    Pc15: (pc15, 15, true), // AD11
    Pc16: (pc16, 16, false),
    Pc17: (pc17, 17, false),
    Pc18: (pc18, 18, false),
    Pc19: (pc19, 19, false),
    Pc20: (pc20, 20, false),
    Pc21: (pc21, 21, false),
    Pc22: (pc22, 22, false),
    Pc23: (pc23, 23, false),
    Pc24: (pc24, 24, false),
    Pc25: (pc25, 25, false),
    Pc26: (pc26, 26, false),
    Pc27: (pc27, 27, false),
    Pc28: (pc28, 28, false),
    Pc29: (pc29, 29, true), // AD13
    Pc30: (pc30, 30, true), // AD14
    Pc31: (pc31, 31, true), // AD15
], [], []);

#[cfg(feature = "atsam4s")]
pins!([
    Pa0: (pa0, 0, true), // WKUP0
    Pa1: (pa1, 1, true), // WKUP1
    Pa2: (pa2, 2, true), // WKUP2
    Pa3: (pa3, 3, false),
    Pa4: (pa4, 4, true), // WKUP3
    Pa5: (pa5, 5, true), // WKUP4
    Pa6: (pa6, 6, false),
    Pa7: (pa7, 7, false),
    Pa8: (pa8, 8, true), // WKUP5
    Pa9: (pa9, 9, true), // WKUP6
    Pa10: (pa10, 10, false),
    Pa11: (pa11, 11, true), // WKUP7
    Pa12: (pa12, 12, false),
    Pa13: (pa13, 13, false),
    Pa14: (pa14, 14, true), // WKUP8
    Pa15: (pa15, 15, true), // WKUP14/PIODCEN1
    Pa16: (pa16, 16, true), // WKUP15/PIODCEN2
    Pa17: (pa17, 17, true), // AD0
    Pa18: (pa18, 18, true), // AD1
    Pa19: (pa19, 19, true), // AD2/WKUP9
    Pa20: (pa20, 20, true), // AD3/WKUP10
    Pa21: (pa21, 21, true), // AD8
    Pa22: (pa22, 22, true), // AD9
    Pa23: (pa23, 23, true), // PIODCCLK
    Pa24: (pa24, 24, true), // PIODC0
    Pa25: (pa25, 25, true), // PIODC1
    Pa26: (pa26, 26, true), // PIODC2
    Pa27: (pa27, 27, true), // PIODC3
    Pa28: (pa28, 28, true), // PIODC4
    Pa29: (pa29, 29, true), // PIODC5
    Pa30: (pa30, 30, true), // WKUP11/PIODC6
    Pa31: (pa31, 31, true), // PIODC7
],[
    Pb0: (pb0, 0, true, false), // AD4/RTCOUT0
    Pb1: (pb1, 1, true, false), // AD5/RTCOUT1
    Pb2: (pb2, 2, true, false), // AD6/WKUP12
    Pb3: (pb3, 3, true, false), // AD7
    Pb4: (pb4, 4, false, true), // | SYSIO4 - TDI
    Pb5: (pb5, 5, true, true), // WKUP13 | SYSIO5 - TDO/TRACESWO
    Pb6: (pb6, 6, false, true), // | SYSIO6 - TMS/SWDIO
    Pb7: (pb7, 7, false, true), // | SYSIO7 - TCK/SWCLK
    Pb8: (pb8, 8, false, false),
    Pb9: (pb9, 9, false, false),
    Pb10: (pb10, 10, false, true), // | SYSIO10 - DDM
    Pb11: (pb11, 11, false, true), // | SYSIO11 - DDP
    Pb12: (pb12, 12, false, true), // | SYSIO12 - ERASE
    Pb13: (pb13, 13, true, false), // DAC0
    Pb14: (pb14, 14, true, false), // DAC1

    // PB15-31 do not exist.
],
[
    Pc0: (pc0, 0, false),
    Pc1: (pc1, 1, false),
    Pc2: (pc2, 2, false),
    Pc3: (pc3, 3, false),
    Pc4: (pc4, 4, false),
    Pc5: (pc5, 5, false),
    Pc6: (pc6, 6, false),
    Pc7: (pc7, 7, false),
    Pc8: (pc8, 8, false),
    Pc9: (pc9, 9, false),
    Pc10: (pc10, 10, false),
    Pc11: (pc11, 11, false),
    Pc12: (pc12, 12, true), // AD12
    Pc13: (pc13, 13, true), // AD10
    Pc14: (pc14, 14, false),
    Pc15: (pc15, 15, true), // AD11
    Pc16: (pc16, 16, false),
    Pc17: (pc17, 17, false),
    Pc18: (pc18, 18, false),
    Pc19: (pc19, 19, false),
    Pc20: (pc20, 20, false),
    Pc21: (pc21, 21, false),
    Pc22: (pc22, 22, false),
    Pc23: (pc23, 23, false),
    Pc24: (pc24, 24, false),
    Pc25: (pc25, 25, false),
    Pc26: (pc26, 26, false),
    Pc27: (pc27, 27, false),
    Pc28: (pc28, 28, false),
    Pc29: (pc29, 29, true), // AD13
    Pc30: (pc30, 30, true), // AD14
    Pc31: (pc31, 31, false),
], [], []);

#[macro_export]
macro_rules! define_pin_map {
    ($(#[$topattr:meta])* struct $Type:ident,
     $( $(#[$attr:meta])* pin $name:ident = $pin_ident:ident<$pin_type:ty, $into_method:ident>),+ , ) => {

        paste! {
            $(#[$topattr])*
            pub struct $Type {
                $(
                    $(#[$attr])*
                    pub $name: [<P $pin_ident>]<$pin_type>
                ),+
            }
        }

        impl $Type {
            /// Returns the pins for the device
            paste! {
                pub fn new(ports: Ports, matrix: &MATRIX) -> Self {
                    let pins = ports.split();
                    // Create local pins with the correct type so we can put them into the
                    // pin structure below.
                    $(
                        let [<new_pin $pin_ident>] = pins.[<p $pin_ident>].$into_method(matrix);
                    )+
                    $Type {
                        $(
                            $name: [<new_pin $pin_ident>]
                        ),+
                    }
                }
            }
        }
    }
}

macro_rules! impl_pxx {
    ($(($port:ident)),*) => {
        paste! {
            pub enum PioX<MODE> {
                $(
                    $port([<$port Generic>]<MODE>)
                ),*
            }

            impl<MODE> OutputPin for PioX<Output<MODE>> {
                type Error = Infallible;
                fn set_high(&mut self) -> Result<(), Infallible> {
                    match self {
                        $(PioX::$port(pin) => pin.set_high()),*
                    }
                }

                fn set_low(&mut self) -> Result<(), Infallible> {
                    match self {
                        $(PioX::$port(pin) => pin.set_low()),*
                    }
                }
            }

            impl<MODE> StatefulOutputPin for PioX<Output<MODE>> {
                fn is_set_high(&self) -> Result<bool, Self::Error> {
                    match self {
                        $(PioX::$port(pin) => pin.is_set_high()),*
                    }
                }

                fn is_set_low(&self) -> Result<bool, Self::Error> {
                    match self {
                        $(PioX::$port(pin) => pin.is_set_low()),*
                    }
                }
            }

            impl<MODE> InputPin for PioX<Input<MODE>> {
                type Error = Infallible;
                fn is_high(&self) -> Result<bool, Infallible> {
                    match self {
                        $(PioX::$port(pin) => pin.is_high()),*
                    }
                }

                fn is_low(&self) -> Result<bool, Infallible> {
                    match self {
                        $(PioX::$port(pin) => pin.is_low()),*
                    }
                }
            }

            impl <MODE> toggleable::Default for PioX<Output<MODE>> {}

            impl IoPin<PioX<Input<Floating>>, Self> for PioX<Output<PushPull>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<PioX<Input<Floating>>, Self::Error> {
                    unsafe {
                        match self {
                            $(PioX::$port(pin) => {
                                (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable open-drain/multi-drive
                                (*$port::ptr()).odr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable output mode
                                (*$port::ptr()).pudr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable pull-up
                                (*$port::ptr()).ppddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable pull-down
                                (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pio mode (disables peripheral control of pin)

                                Ok(PioX::$port([<$port Generic>] { i: pin.i, _mode: PhantomData }))
                            })*
                        }
                    }

                }
                fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                    self.set_state(state).unwrap();
                    Ok(self)
                }
            }

            impl IoPin<PioX<Input<PullDown>>, Self> for PioX<Output<PushPull>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<PioX<Input<PullDown>>, Self::Error> {
                    unsafe {
                        match self {
                            $(PioX::$port(pin) => {
                                (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable open-drain/multi-drive
                                (*$port::ptr()).odr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable output mode
                                (*$port::ptr()).pudr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable pull-up
                                (*$port::ptr()).ppder.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pull-down
                                (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pio mode (disables peripheral control of pin)

                                Ok(PioX::$port([<$port Generic>] { i: pin.i, _mode: PhantomData }))
                            })*
                        }
                    }

                }
                fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                    self.set_state(state).unwrap();
                    Ok(self)
                }
            }

            impl IoPin<PioX<Input<PullUp>>, Self> for PioX<Output<PushPull>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<PioX<Input<PullUp>>, Self::Error> {
                    unsafe {
                        match self {
                            $(PioX::$port(pin) => {
                                (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable open-drain/multi-drive
                                (*$port::ptr()).odr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable output mode
                                (*$port::ptr()).puer.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pull-up
                                (*$port::ptr()).ppddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable pull-down
                                (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pio mode (disables peripheral control of pin)

                                Ok(PioX::$port([<$port Generic>] { i: pin.i, _mode: PhantomData }))
                            })*
                        }
                    }

                }
                fn into_output_pin(mut self, state: PinState) -> Result<Self, Self::Error> {
                    self.set_state(state).unwrap();
                    Ok(self)
                }
            }

            impl IoPin<Self, PioX<Output<PushPull>>> for PioX<Input<Floating>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<Self, Self::Error> {
                    Ok(self)
                }
                fn into_output_pin(self, state: PinState) -> Result<PioX<Output<PushPull>>, Self::Error> {
                    unsafe {
                        match self {
                            $(PioX::$port(pin) => {
                                (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable open-drain/multi-drive
                                (*$port::ptr()).oer.write_with_zero(|w| w.bits(1 << pin.i)); // Enable output mode
                                (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pio mode (disables peripheral control of pin)
                                match state {
                                    PinState::Low => {
                                        (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << pin.i) );
                                    }
                                    PinState::High => {
                                        (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << pin.i) );
                                    }
                                }

                                Ok(PioX::$port([<$port Generic>] { i: pin.i, _mode: PhantomData }))
                            })*
                        }
                    }
                }
            }

            impl IoPin<Self, PioX<Output<PushPull>>> for PioX<Input<PullDown>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<Self, Self::Error> {
                    Ok(self)
                }
                fn into_output_pin(self, state: PinState) -> Result<PioX<Output<PushPull>>, Self::Error> {
                    unsafe {
                        match self {
                            $(PioX::$port(pin) => {
                                (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable open-drain/multi-drive
                                (*$port::ptr()).oer.write_with_zero(|w| w.bits(1 << pin.i)); // Enable output mode
                                (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pio mode (disables peripheral control of pin)
                                match state {
                                    PinState::Low => {
                                        (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << pin.i) );
                                    }
                                    PinState::High => {
                                        (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << pin.i) );
                                    }
                                }

                                Ok(PioX::$port([<$port Generic>] { i: pin.i, _mode: PhantomData }))
                            })*
                        }
                    }
                }
            }

            impl IoPin<Self, PioX<Output<PushPull>>> for PioX<Input<PullUp>> {
                type Error = Infallible;
                fn into_input_pin(self) -> Result<Self, Self::Error> {
                    Ok(self)
                }
                fn into_output_pin(self, state: PinState) -> Result<PioX<Output<PushPull>>, Self::Error> {
                    unsafe {
                        match self {
                            $(PioX::$port(pin) => {
                                (*$port::ptr()).mddr.write_with_zero(|w| w.bits(1 << pin.i)); // Disable open-drain/multi-drive
                                (*$port::ptr()).oer.write_with_zero(|w| w.bits(1 << pin.i)); // Enable output mode
                                (*$port::ptr()).per.write_with_zero(|w| w.bits(1 << pin.i)); // Enable pio mode (disables peripheral control of pin)
                                match state {
                                    PinState::Low => {
                                        (*$port::ptr()).codr.write_with_zero(|w| w.bits(1 << pin.i) );
                                    }
                                    PinState::High => {
                                        (*$port::ptr()).sodr.write_with_zero(|w| w.bits(1 << pin.i) );
                                    }
                                }

                                Ok(PioX::$port([<$port Generic>] { i: pin.i, _mode: PhantomData }))
                            })*
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(any(feature = "atsam4n_c", feature = "atsam4s_c", feature = "atsam4e")))]
impl_pxx! {
    (PIOA),
    (PIOB)
}

#[cfg(any(feature = "atsam4n_c", feature = "atsam4s_c"))]
impl_pxx! {
    (PIOA),
    (PIOB),
    (PIOC)
}

#[cfg(feature = "atsam4e_c")]
impl_pxx! {
    (PIOA),
    (PIOB),
    (PIOD)
}

#[cfg(feature = "atsam4e_e")]
impl_pxx! {
    (PIOA),
    (PIOB),
    (PIOC),
    (PIOD),
    (PIOE)
}
