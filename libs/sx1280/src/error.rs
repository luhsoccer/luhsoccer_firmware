use defmt::Format;
pub use embedded_hal::digital::v2::{InputPin, OutputPin};
pub use embedded_hal::spi::FullDuplex;

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum Error<S, R, B> {
    Spi(S),
    ResetPin(R),
    BusyPin(B),
    Timeout,
    Other,
}

pub(crate) type Result<T, S, R, B> = core::result::Result<T, Error<S, R, B>>;
