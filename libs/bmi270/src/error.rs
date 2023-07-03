use defmt::Format;
pub use embedded_hal::digital::v2::{InputPin, OutputPin};
pub use embedded_hal::spi::FullDuplex;

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum Error<S, P> {
    Spi(S),
    ChipSelectPin(P),
    InvalidConfig,
    InvalidChipId,
    Other,
}

pub type Err<S, Word, P> = Error<<S as FullDuplex<Word>>::Error, <P as OutputPin>::Error>;

pub type Result<T, S, Word, P> = ::core::result::Result<T, Err<S, Word, P>>;
