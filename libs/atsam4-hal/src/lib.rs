//! HAL for the ATSAM4 series of microcontrollers
//!
//! This is an implementation of the [`embedded-hal`] traits for the ATSAM4 microcontrollers
//!
//! [`embedded-hal`]: https://github.com/japaric/embedded-hal
//!
//! # Requirements
//!
//! This crate requires `arm-none-eabi-gcc` to be installed and available in `$PATH` to build.
//!
//! # Usage
//!
//! To build applications (binary crates) using this crate follow the [cortex-m-quickstart]
//! instructions and add this crate as a dependency in step number 5 and make sure you enable the
//! "rt" Cargo feature of this crate.
//!
//! [cortex-m-quickstart]: https://docs.rs/cortex-m-quickstart/~0.3
//!

//#![deny(missing_docs)]
#![no_std]

pub extern crate embedded_hal as hal;
pub use hal::digital::v2::*;

#[cfg(feature = "atsam4e16c")]
pub use atsam4e16c_pac as pac;
#[cfg(feature = "atsam4e16e")]
pub use atsam4e16e_pac as pac;
#[cfg(feature = "atsam4e8c")]
pub use atsam4e8c_pac as pac;
#[cfg(feature = "atsam4e8e")]
pub use atsam4e8e_pac as pac;

#[cfg(feature = "atsam4n16b")]
pub use atsam4n16b_pac as pac;
#[cfg(feature = "atsam4n16c")]
pub use atsam4n16c_pac as pac;
#[cfg(feature = "atsam4n8a")]
pub use atsam4n8a_pac as pac;
#[cfg(feature = "atsam4n8b")]
pub use atsam4n8b_pac as pac;
#[cfg(feature = "atsam4n8c")]
pub use atsam4n8c_pac as pac;

#[cfg(feature = "atsam4s2a")]
pub use atsam4s2a_pac as pac;
#[cfg(feature = "atsam4s2b")]
pub use atsam4s2b_pac as pac;
#[cfg(feature = "atsam4s2c")]
pub use atsam4s2c_pac as pac;
#[cfg(feature = "atsam4s4a")]
pub use atsam4s4a_pac as pac;
#[cfg(feature = "atsam4s4b")]
pub use atsam4s4b_pac as pac;
#[cfg(feature = "atsam4s4c")]
pub use atsam4s4c_pac as pac;
#[cfg(feature = "atsam4s8b")]
pub use atsam4s8b_pac as pac;
#[cfg(feature = "atsam4s8c")]
pub use atsam4s8c_pac as pac;
#[cfg(feature = "atsam4sa16b")]
pub use atsam4sa16b_pac as pac;
#[cfg(feature = "atsam4sa16c")]
pub use atsam4sa16c_pac as pac;
#[cfg(feature = "atsam4sd16b")]
pub use atsam4sd16b_pac as pac;
#[cfg(feature = "atsam4sd16c")]
pub use atsam4sd16c_pac as pac;
#[cfg(feature = "atsam4sd32b")]
pub use atsam4sd32b_pac as pac;
#[cfg(feature = "atsam4sd32c")]
pub use atsam4sd32c_pac as pac;

use core::mem;

// NOTE: In ASF atsam4s uses sam/drivers/adc/adc.c whereas atsam4n uses sam/drivers/adc/adc2.c
#[cfg(feature = "atsam4s")]
pub mod adc;
pub mod chipid;
pub mod clock;
pub mod delay;
pub mod efc;
#[cfg(all(any(feature = "atsam4e")))]
pub mod ethernet;
pub mod gpio;
pub mod pdc;
pub mod prelude;
pub mod rtt;
pub mod serial;
pub mod spi;
pub mod static_memory_controller;
pub mod timer;
#[cfg(all(feature = "usb", any(feature = "atsam4e", feature = "atsam4s")))]
pub mod udp;
pub use heapless;
pub use smoltcp;

pub mod watchdog;

mod sealed;

/// Borrows a peripheral without checking if it has already been taken
/// # Safety
unsafe trait BorrowUnchecked {
    fn borrow_unchecked<T>(f: impl FnOnce(&mut Self) -> T) -> T;
}

macro_rules! borrow_unchecked {
    ($($peripheral:ident),*) => {
        $(
            unsafe impl BorrowUnchecked for pac::$peripheral {
                fn borrow_unchecked<T>(f: impl FnOnce(&mut Self) -> T) -> T {
                    let mut p = unsafe { mem::transmute(()) };
                    f(&mut p)
                }
            }
        )*
    }
}

borrow_unchecked!(TC0);

#[cfg(any(feature = "atsam4e_e", feature = "atsam4n_c", feature = "atsam4s_c"))]
borrow_unchecked!(TC1);

#[cfg(feature = "atsam4e_e")]
borrow_unchecked!(TC2);

#[cfg(all(feature = "usb", any(feature = "atsam4e", feature = "atsam4s")))]
borrow_unchecked!(PMC, UDP);
