pub use atsam4_hal as hal;
use atsam4_hal::pac::MATRIX;
use atsam4_hal::{define_pin_map, gpio::*};
use paste::paste;

define_pin_map! {
    struct Pins,

    pin rf_cps = a5<Output<OpenDrain>, into_open_drain_output>,
    pin rf_ant_sel = a8<Output<OpenDrain>, into_open_drain_output>,
    pin spi_cs = a11<PfA, into_peripheral_function_a>,

}
