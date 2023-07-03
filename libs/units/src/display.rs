use core::fmt::{self, Display, Formatter};

use typenum::Integer;

use crate::{
    types::{
        AmperePerMetre, AmperePerSquareMetre, Coulomb, CoulombPerCubicMetre, CoulombPerKilogram,
        CoulombPerSquareMetre, CubicMetrePerKilogram, Farad, FaradPerMetre, Gray, GrayPerSecond,
        Henry, HenryPerMetre, Herz, Joule, JoulePerKelvin, JoulePerKilogramKelvin, JoulePerMole,
        JoulePerMoleKelvin, Katal, KatalPerCubicMetre, KilogramPerCubicMetre,
        KilogramPerSquareMetre, Lux, MetrePerSecond, MetrePerSquareSecond, MolePerCubicMetre,
        Newton, NewtonPerMetre, Ohm, Pascal, PascalSecond, Siemens, Tesla, Volt, VoltPerMetre,
        Watt, WattPerMetreKelvin, WattPerSquareMetre, Weber,
    },
    SiUnit,
};

#[cfg(feature = "defmt")]
macro_rules! display_unit_defmt {
    ($formatter: ident, $param: ident, $symbol: literal, $e1: ident, $e2: ident, $e3: ident, $e4: ident, $e5: ident, $e6: ident) => {
        if $param::I64 != 0 {
            if $e1::I64 == 0
                && $e2::I64 == 0
                && $e3::I64 == 0
                && $e4::I64 == 0
                && $e5::I64 == 0
                && $e6::I64 == 0
            {
                defmt::write!($formatter, $symbol);
            } else {
                defmt::write!($formatter, "*{}", $symbol);
            }
            if $param::I64 != 1 {
                defmt::write!($formatter, "^{}", $param::I64);
            }
        }
    };
}

#[cfg(feature = "defmt")]
macro_rules! display_special_unit_defmt {
    ($formatter: ident, $self: ident, $(($symbol: literal, $other: ty)),* $(,)?) => {
        $(
        if ::core::any::TypeId::of::<$self>() == ::core::any::TypeId::of::<$other>() {
            defmt::write!($formatter, $symbol);
            return;
        }
        )*
    };
}

#[cfg(feature = "defmt")]
impl<T, Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela> defmt::Format
    for SiUnit<T, Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela>
where
    Second: Integer,
    Metre: Integer,
    Kilogram: Integer,
    Ampere: Integer,
    Kelvin: Integer,
    Mole: Integer,
    Candela: Integer,
    T: defmt::Format + 'static,
{
    #[allow(clippy::cognitive_complexity)]
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{}", self.value);
        // derived units with special symbols
        display_special_unit_defmt!(f, Self,
            ("Hz", Herz<T>),
            ("N", Newton<T>),
            ("Pa", Pascal<T>),
            ("J", Joule<T>),
            ("W", Watt<T>),
            ("C", Coulomb<T>),
            ("V", Volt<T>),
            ("F", Farad<T>),
            ("Ohm", Ohm<T>),
            ("S", Siemens<T>),
            ("Wb", Weber<T>),
            ("T", Tesla<T>),
            ("H", Henry<T>),
            ("lx", Lux<T>),
            ("Gy|Sv", Gray<T>),
            ("kat", Katal<T>),
        );
        // derived units
        display_special_unit_defmt!(f, Self,
            ("m/s", MetrePerSecond<T>),
            ("m/s²", MetrePerSquareSecond<T>),
            ("kg/m³", KilogramPerCubicMetre<T>),
            ("kg/m²", KilogramPerSquareMetre<T>),
            ("m³/kg", CubicMetrePerKilogram<T>),
            ("A/m²", AmperePerSquareMetre<T>),
            ("A/m", AmperePerMetre<T>),
            ("mol/m³", MolePerCubicMetre<T>),
        );
        // derived units including special names
        display_special_unit_defmt!(f, Self,
            ("Pa*s", PascalSecond<T>),
            ("N/m", NewtonPerMetre<T>),
            ("W/m²", WattPerSquareMetre<T>),
            ("J/K", JoulePerKelvin<T>),
            ("J/(kg*K)", JoulePerKilogramKelvin<T>),
            ("W/(m*K)", WattPerMetreKelvin<T>),
            ("V/m", VoltPerMetre<T>),
            ("C/m³", CoulombPerCubicMetre<T>),
            ("C/m²", CoulombPerSquareMetre<T>),
            ("F/m", FaradPerMetre<T>),
            ("H/m", HenryPerMetre<T>),
            ("J/mol", JoulePerMole<T>),
            ("J/(mol*K)", JoulePerMoleKelvin<T>),
            ("C/kg", CoulombPerKilogram<T>),
            ("Gy/s", GrayPerSecond<T>),
            ("kat/m³", KatalPerCubicMetre<T>),
        );

        // base units
        display_unit_defmt!(f, Second, "s", Metre, Kilogram, Ampere, Kelvin, Mole, Candela);
        display_unit_defmt!(f, Metre, "m", Second, Kilogram, Ampere, Kelvin, Mole, Candela);
        display_unit_defmt!(f, Kilogram, "kg", Second, Metre, Ampere, Kelvin, Mole, Candela);
        display_unit_defmt!(f, Ampere, "A", Second, Metre, Kilogram, Kelvin, Mole, Candela);
        display_unit_defmt!(f, Kelvin, "K", Second, Metre, Kilogram, Ampere, Mole, Candela);
        display_unit_defmt!(f, Mole, "mol", Second, Metre, Kilogram, Ampere, Kelvin, Candela);
        display_unit_defmt!(f, Candela, "cd", Second, Metre, Kilogram, Ampere, Kelvin, Mole);
    }
}

macro_rules! display_unit {
    ($formatter: ident, $param: ident, $symbol: literal, $e1: ident, $e2: ident, $e3: ident, $e4: ident, $e5: ident, $e6: ident) => {
        if $param::I64 != 0 {
            if $e1::I64 == 0
                && $e2::I64 == 0
                && $e3::I64 == 0
                && $e4::I64 == 0
                && $e5::I64 == 0
                && $e6::I64 == 0
            {
                write!($formatter, $symbol)?;
            } else {
                write!($formatter, "*{}", $symbol)?;
            }
            if $param::I64 != 1 {
                write!($formatter, "^{}", $param::I64)?;
            }
        }
    };
}

macro_rules! display_special_unit {
    ($formatter: ident, $self: ident, $(($symbol: literal, $other: ty)),* $(,)?) => {
        $(
        if ::core::any::TypeId::of::<$self>() == ::core::any::TypeId::of::<$other>() {
            return write!($formatter, $symbol);
        }
        )*
    };
}

impl<T, Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela> Display
    for SiUnit<T, Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela>
where
    Second: Integer,
    Metre: Integer,
    Kilogram: Integer,
    Ampere: Integer,
    Kelvin: Integer,
    Mole: Integer,
    Candela: Integer,
    T: Display + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)?;
        // derived units with special symbols
        display_special_unit!(f, Self,
            ("Hz", Herz<T>),
            ("N", Newton<T>),
            ("Pa", Pascal<T>),
            ("J", Joule<T>),
            ("W", Watt<T>),
            ("C", Coulomb<T>),
            ("V", Volt<T>),
            ("F", Farad<T>),
            ("Ohm", Ohm<T>),
            ("S", Siemens<T>),
            ("Wb", Weber<T>),
            ("T", Tesla<T>),
            ("H", Henry<T>),
            ("lx", Lux<T>),
            ("Gy|Sv", Gray<T>),
            ("kat", Katal<T>),
        );
        // derived units
        display_special_unit!(f, Self,
            ("m/s", MetrePerSecond<T>),
            ("m/s²", MetrePerSquareSecond<T>),
            ("kg/m³", KilogramPerCubicMetre<T>),
            ("kg/m²", KilogramPerSquareMetre<T>),
            ("m³/kg", CubicMetrePerKilogram<T>),
            ("A/m²", AmperePerSquareMetre<T>),
            ("A/m", AmperePerMetre<T>),
            ("mol/m³", MolePerCubicMetre<T>),
        );
        // derived units including special names
        display_special_unit!(f, Self,
            ("Pa*s", PascalSecond<T>),
            ("N/m", NewtonPerMetre<T>),
            ("W/m²", WattPerSquareMetre<T>),
            ("J/K", JoulePerKelvin<T>),
            ("J/(kg*K)", JoulePerKilogramKelvin<T>),
            ("W/(m*K)", WattPerMetreKelvin<T>),
            ("V/m", VoltPerMetre<T>),
            ("C/m³", CoulombPerCubicMetre<T>),
            ("C/m²", CoulombPerSquareMetre<T>),
            ("F/m", FaradPerMetre<T>),
            ("H/m", HenryPerMetre<T>),
            ("J/mol", JoulePerMole<T>),
            ("J/(mol*K)", JoulePerMoleKelvin<T>),
            ("C/kg", CoulombPerKilogram<T>),
            ("Gy/s", GrayPerSecond<T>),
            ("kat/m³", KatalPerCubicMetre<T>),
        );

        // base units
        display_unit!(f, Second, "s", Metre, Kilogram, Ampere, Kelvin, Mole, Candela);
        display_unit!(f, Metre, "m", Second, Kilogram, Ampere, Kelvin, Mole, Candela);
        display_unit!(f, Kilogram, "kg", Second, Metre, Ampere, Kelvin, Mole, Candela);
        display_unit!(f, Ampere, "A", Second, Metre, Kilogram, Kelvin, Mole, Candela);
        display_unit!(f, Kelvin, "K", Second, Metre, Kilogram, Ampere, Mole, Candela);
        display_unit!(f, Mole, "mol", Second, Metre, Kilogram, Ampere, Kelvin, Candela);
        display_unit!(f, Candela, "cd", Second, Metre, Kilogram, Ampere, Kelvin, Mole);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::types::{Ampere, Candela, Kelvin, Kilogram, Metre, Mole, Second, Unit};

    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn debug() {
        let m = Metre::new(2);
        assert_eq!(format!("{m:?}"), "SiUnit { value: 2, _marker: PhantomData<(typenum::int::Z0, typenum::int::PInt<typenum::uint::UInt<typenum::uint::UTerm, typenum::bit::B1>>, typenum::int::Z0, typenum::int::Z0, typenum::int::Z0, typenum::int::Z0, typenum::int::Z0)> }".to_owned());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    #[cfg(feature = "std")]
    fn display() {
        let unit = Unit::new(2);
        let second = Second::new(2);
        let meter = Metre::new(2);
        let kilogram = Kilogram::new(2);
        let ampere = Ampere::new(2);
        let kelvin = Kelvin::new(2);
        let mole = Mole::new(2);
        let candela = Candela::new(2);
        assert_eq!(unit.to_string(), "2");
        assert_eq!(second.to_string(), "2s");
        assert_eq!(meter.to_string(), "2m");
        assert_eq!(kilogram.to_string(), "2kg");
        assert_eq!(ampere.to_string(), "2A");
        assert_eq!(kelvin.to_string(), "2K");
        assert_eq!(mole.to_string(), "2mol");
        assert_eq!(candela.to_string(), "2cd");
        let square_second: SiUnit<i32, _, _, _, _, _, _, _> = second * second;
        let square_meter: SiUnit<i32, _, _, _, _, _, _, _> = meter * meter;
        let square_kilogram: SiUnit<i32, _, _, _, _, _, _, _> = kilogram * kilogram;
        let square_ampere: SiUnit<i32, _, _, _, _, _, _, _> = ampere * ampere;
        let square_kelvin: SiUnit<i32, _, _, _, _, _, _, _> = kelvin * kelvin;
        let square_mole: SiUnit<i32, _, _, _, _, _, _, _> = mole * mole;
        let square_candela: SiUnit<i32, _, _, _, _, _, _, _> = candela * candela;
        assert_eq!(square_second.to_string(), "4s^2");
        assert_eq!(square_meter.to_string(), "4m^2");
        assert_eq!(square_kilogram.to_string(), "4kg^2");
        assert_eq!(square_ampere.to_string(), "4A^2");
        assert_eq!(square_kelvin.to_string(), "4K^2");
        assert_eq!(square_mole.to_string(), "4mol^2");
        assert_eq!(square_candela.to_string(), "4cd^2");

        assert_eq!(Herz::new(2).to_string(), "2Hz");
        assert_eq!(Newton::new(2).to_string(), "2N");
        assert_eq!(Pascal::new(2).to_string(), "2Pa");
        assert_eq!(Joule::new(2).to_string(), "2J");
        assert_eq!(Watt::new(2).to_string(), "2W");
        assert_eq!(Coulomb::new(2).to_string(), "2C");
        assert_eq!(Volt::new(2).to_string(), "2V");
        assert_eq!(Farad::new(2).to_string(), "2F");
        assert_eq!(Ohm::new(2).to_string(), "2Ohm");
        assert_eq!(Siemens::new(2).to_string(), "2S");
        assert_eq!(Weber::new(2).to_string(), "2Wb");
        assert_eq!(Tesla::new(2).to_string(), "2T");
        assert_eq!(Henry::new(2).to_string(), "2H");
        assert_eq!(Lux::new(2).to_string(), "2lx");
        assert_eq!(Gray::new(2).to_string(), "2Gy|Sv");
        assert_eq!(Katal::new(2).to_string(), "2kat");

        assert_eq!(MetrePerSecond::new(2).to_string(), "2m/s");
        assert_eq!(MetrePerSquareSecond::new(2).to_string(), "2m/s²");
        assert_eq!(KilogramPerCubicMetre::new(2).to_string(), "2kg/m³");
        assert_eq!(KilogramPerSquareMetre::new(2).to_string(), "2kg/m²");
        assert_eq!(CubicMetrePerKilogram::new(2).to_string(), "2m³/kg");
        assert_eq!(AmperePerSquareMetre::new(2).to_string(), "2A/m²");
        assert_eq!(AmperePerMetre::new(2).to_string(), "2A/m");
        assert_eq!(MolePerCubicMetre::new(2).to_string(), "2mol/m³");

        assert_eq!(PascalSecond::new(2).to_string(), "2Pa*s");
        assert_eq!(NewtonPerMetre::new(2).to_string(), "2N/m");
        assert_eq!(WattPerSquareMetre::new(2).to_string(), "2W/m²");
        assert_eq!(JoulePerKelvin::new(2).to_string(), "2J/K");
        assert_eq!(JoulePerKilogramKelvin::new(2).to_string(), "2J/(kg*K)");
        assert_eq!(WattPerMetreKelvin::new(2).to_string(), "2W/(m*K)");
        assert_eq!(VoltPerMetre::new(2).to_string(), "2V/m");
        assert_eq!(CoulombPerCubicMetre::new(2).to_string(), "2C/m³");
        assert_eq!(CoulombPerSquareMetre::new(2).to_string(), "2C/m²");
        assert_eq!(FaradPerMetre::new(2).to_string(), "2F/m");
        assert_eq!(HenryPerMetre::new(2).to_string(), "2H/m");
        assert_eq!(JoulePerMole::new(2).to_string(), "2J/mol");
        assert_eq!(JoulePerMoleKelvin::new(2).to_string(), "2J/(mol*K)");
        assert_eq!(CoulombPerKilogram::new(2).to_string(), "2C/kg");
        assert_eq!(GrayPerSecond::new(2).to_string(), "2Gy/s");
        assert_eq!(KatalPerCubicMetre::new(2).to_string(), "2kat/m³");
    }
}
