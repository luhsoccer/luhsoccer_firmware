#[cfg(feature = "fixed")]
#[allow(clippy::wildcard_imports)]
use fixed::types::*;
use typenum::Integer;

use crate::{
    types::{
        Ampere, AmperePerMetre, AmperePerSquareMetre, Becquerel, Candela, CandelaPerSquareMetre,
        Coulomb, CoulombPerCubicMetre, CoulombPerKilogram, CoulombPerSquareMetre, CubicMetre,
        CubicMetrePerKilogram, Farad, FaradPerMetre, Gray, GrayPerSecond, Henry, HenryPerMetre,
        Herz, Joule, JoulePerCubicMetre, JoulePerKelvin, JoulePerKilogram, JoulePerKilogramKelvin,
        JoulePerMole, JoulePerMoleKelvin, Katal, KatalPerCubicMetre, Kelvin, Kilogram,
        KilogramPerCubicMetre, KilogramPerSquareMetre, Lumen, Lux, Metre, MetrePerCubeSecond,
        MetrePerSecond, MetrePerSquareSecond, Mole, MolePerCubicMetre, Newton, NewtonMetre,
        NewtonPerMetre, Ohm, Pascal, PascalSecond, Radian, RadianPerCubeSecond, RadianPerSecond,
        RadianPerSquareSecond, ReciprocalMetre, Second, Siemens, Sievert, SquareMetre, Steradian,
        Tesla, Unit, Volt, VoltPerMetre, Watt, WattPerMetreKelvin, WattPerSquareMetre,
        WattPerSquareMetreSteradian, WattPerSteradian, Weber,
    },
    SiUnit,
};

pub trait IntoUnit
where
    Self: Sized,
{
    fn into_unit<Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela>(
        self,
    ) -> SiUnit<Self, Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela>
    where
        Second: Integer,
        Metre: Integer,
        Kilogram: Integer,
        Ampere: Integer,
        Kelvin: Integer,
        Mole: Integer,
        Candela: Integer;

    // Base units
    fn unit(self) -> Unit<Self> {
        self.into_unit()
    }
    fn second(self) -> Second<Self> {
        self.into_unit()
    }
    fn s(self) -> Second<Self> {
        self.into_unit()
    }
    fn metre(self) -> Metre<Self> {
        self.into_unit()
    }
    fn m(self) -> Metre<Self> {
        self.into_unit()
    }
    fn kilogram(self) -> Kilogram<Self> {
        self.into_unit()
    }
    fn kg(self) -> Kilogram<Self> {
        self.into_unit()
    }
    fn ampere(self) -> Ampere<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn A(self) -> Ampere<Self> {
        self.into_unit()
    }
    fn kelvin(self) -> Kelvin<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn K(self) -> Kelvin<Self> {
        self.into_unit()
    }
    fn mole(self) -> Mole<Self> {
        self.into_unit()
    }
    fn mol(self) -> Mole<Self> {
        self.into_unit()
    }
    fn candela(self) -> Candela<Self> {
        self.into_unit()
    }
    fn cd(self) -> Candela<Self> {
        self.into_unit()
    }

    // Derived units with special names
    fn radian(self) -> Radian<Self> {
        self.into_unit()
    }
    fn rad(self) -> Radian<Self> {
        self.into_unit()
    }
    fn steradian(self) -> Steradian<Self> {
        self.into_unit()
    }
    fn sr(self) -> Steradian<Self> {
        self.into_unit()
    }
    fn herz(self) -> Herz<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Hz(self) -> Herz<Self> {
        self.into_unit()
    }
    fn newton(self) -> Newton<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn N(self) -> Newton<Self> {
        self.into_unit()
    }
    fn pascal(self) -> Pascal<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Pa(self) -> Pascal<Self> {
        self.into_unit()
    }
    fn joule(self) -> Joule<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn J(self) -> Joule<Self> {
        self.into_unit()
    }
    fn watt(self) -> Watt<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn W(self) -> Watt<Self> {
        self.into_unit()
    }
    fn coulomb(self) -> Coulomb<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn C(self) -> Coulomb<Self> {
        self.into_unit()
    }
    fn volt(self) -> Volt<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn V(self) -> Volt<Self> {
        self.into_unit()
    }
    fn farad(self) -> Farad<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn F(self) -> Farad<Self> {
        self.into_unit()
    }
    fn ohm(self) -> Ohm<Self> {
        self.into_unit()
    }
    fn siemens(self) -> Siemens<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn S(self) -> Siemens<Self> {
        self.into_unit()
    }
    fn weber(self) -> Weber<Self> {
        self.into_unit()
    }
    fn wb(self) -> Weber<Self> {
        self.into_unit()
    }
    fn tesla(self) -> Tesla<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn T(self) -> Tesla<Self> {
        self.into_unit()
    }
    fn henry(self) -> Henry<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn H(self) -> Henry<Self> {
        self.into_unit()
    }
    fn lumen(self) -> Lumen<Self> {
        self.into_unit()
    }
    fn lm(self) -> Lumen<Self> {
        self.into_unit()
    }
    fn lux(self) -> Lux<Self> {
        self.into_unit()
    }
    fn lx(self) -> Lux<Self> {
        self.into_unit()
    }
    fn becquerel(self) -> Becquerel<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Bq(self) -> Becquerel<Self> {
        self.into_unit()
    }
    fn gray(self) -> Gray<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Gy(self) -> Gray<Self> {
        self.into_unit()
    }
    fn sievert(self) -> Sievert<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Sv(self) -> Sievert<Self> {
        self.into_unit()
    }
    fn katal(self) -> Katal<Self> {
        self.into_unit()
    }
    fn kat(self) -> Katal<Self> {
        self.into_unit()
    }

    // derived units without special names
    fn square_metre(self) -> SquareMetre<Self> {
        self.into_unit()
    }
    fn sqm(self) -> SquareMetre<Self> {
        self.into_unit()
    }
    fn cubic_metre(self) -> CubicMetre<Self> {
        self.into_unit()
    }
    fn metre_per_second(self) -> MetrePerSecond<Self> {
        self.into_unit()
    }
    fn mps(self) -> MetrePerSecond<Self> {
        self.into_unit()
    }
    fn metre_per_square_second(self) -> MetrePerSquareSecond<Self> {
        self.into_unit()
    }
    fn mpsqs(self) -> MetrePerSquareSecond<Self> {
        self.into_unit()
    }
    fn metre_per_cube_second(self) -> MetrePerCubeSecond<Self> {
        self.into_unit()
    }
    fn reciprocal_metre(self) -> ReciprocalMetre<Self> {
        self.into_unit()
    }
    fn kilogram_per_cubic_metre(self) -> KilogramPerCubicMetre<Self> {
        self.into_unit()
    }
    fn kilogram_per_square_metre(self) -> KilogramPerSquareMetre<Self> {
        self.into_unit()
    }
    fn kgpsqm(self) -> KilogramPerSquareMetre<Self> {
        self.into_unit()
    }
    fn cubic_metre_per_kilogram(self) -> CubicMetrePerKilogram<Self> {
        self.into_unit()
    }
    fn ampere_per_square_metre(self) -> AmperePerSquareMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Apsqm(self) -> AmperePerSquareMetre<Self> {
        self.into_unit()
    }
    fn ampere_per_metre(self) -> AmperePerMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Apm(self) -> AmperePerMetre<Self> {
        self.into_unit()
    }
    fn mole_per_cubic_metre(self) -> MolePerCubicMetre<Self> {
        self.into_unit()
    }
    fn candela_per_square_metre(self) -> CandelaPerSquareMetre<Self> {
        self.into_unit()
    }
    fn cdpsqm(self) -> CandelaPerSquareMetre<Self> {
        self.into_unit()
    }

    // Derived units including special names
    fn pascal_second(self) -> PascalSecond<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Pas(self) -> PascalSecond<Self> {
        self.into_unit()
    }
    fn newton_metre(self) -> NewtonMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Nm(self) -> NewtonMetre<Self> {
        self.into_unit()
    }
    fn newton_per_metre(self) -> NewtonPerMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Npm(self) -> NewtonPerMetre<Self> {
        self.into_unit()
    }
    fn radian_per_second(self) -> RadianPerSecond<Self> {
        self.into_unit()
    }
    fn rps(self) -> RadianPerSecond<Self> {
        self.into_unit()
    }
    fn radian_per_square_second(self) -> RadianPerSquareSecond<Self> {
        self.into_unit()
    }
    fn rpsqs(self) -> RadianPerSquareSecond<Self> {
        self.into_unit()
    }
    fn radian_per_cube_second(self) -> RadianPerCubeSecond<Self> {
        self.into_unit()
    }
    fn watt_per_square_metre(self) -> WattPerSquareMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Wpsqm(self) -> WattPerSquareMetre<Self> {
        self.into_unit()
    }
    fn joule_per_kelvin(self) -> JoulePerKelvin<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn JpK(self) -> JoulePerKelvin<Self> {
        self.into_unit()
    }
    fn joule_per_kilogram_kelvin(self) -> JoulePerKilogramKelvin<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn JpkgK(self) -> JoulePerKilogramKelvin<Self> {
        self.into_unit()
    }
    fn joule_per_kilogram(self) -> JoulePerKilogram<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Jpkg(self) -> JoulePerKilogram<Self> {
        self.into_unit()
    }
    fn watt_per_metre_kelvin(self) -> WattPerMetreKelvin<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn WpmK(self) -> WattPerMetreKelvin<Self> {
        self.into_unit()
    }
    fn joule_per_cubic_metre(self) -> JoulePerCubicMetre<Self> {
        self.into_unit()
    }
    fn volt_per_metre(self) -> VoltPerMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Vpm(self) -> VoltPerMetre<Self> {
        self.into_unit()
    }
    fn coulomb_per_cubic_metre(self) -> CoulombPerCubicMetre<Self> {
        self.into_unit()
    }
    fn coulomb_per_square_metre(self) -> CoulombPerSquareMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Cpsqm(self) -> CoulombPerSquareMetre<Self> {
        self.into_unit()
    }
    fn farad_per_metre(self) -> FaradPerMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Fpm(self) -> FaradPerMetre<Self> {
        self.into_unit()
    }
    fn henry_per_metre(self) -> HenryPerMetre<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Hpm(self) -> HenryPerMetre<Self> {
        self.into_unit()
    }
    fn joule_per_mole(self) -> JoulePerMole<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Jpmol(self) -> JoulePerMole<Self> {
        self.into_unit()
    }
    fn joule_per_mole_kelvin(self) -> JoulePerMoleKelvin<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn JpmolK(self) -> JoulePerMoleKelvin<Self> {
        self.into_unit()
    }
    fn coulomb_per_kilogram(self) -> CoulombPerKilogram<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Cpkg(self) -> CoulombPerKilogram<Self> {
        self.into_unit()
    }
    fn gray_per_second(self) -> GrayPerSecond<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Gyps(self) -> GrayPerSecond<Self> {
        self.into_unit()
    }
    fn watt_per_steradian(self) -> WattPerSteradian<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Wpsr(self) -> WattPerSteradian<Self> {
        self.into_unit()
    }
    fn watt_per_square_metre_steradian(self) -> WattPerSquareMetreSteradian<Self> {
        self.into_unit()
    }
    #[allow(non_snake_case)]
    fn Wpsqmsr(self) -> WattPerSquareMetreSteradian<Self> {
        self.into_unit()
    }
    fn katal_per_cubic_metre(self) -> KatalPerCubicMetre<Self> {
        self.into_unit()
    }
}

macro_rules! impl_into_unit {
    ($($ty: ty)*) => {
        $(
        impl IntoUnit for $ty {
            fn into_unit<Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela>(
                self,
            ) -> SiUnit<Self, Second, Metre, Kilogram, Ampere, Kelvin, Mole, Candela>
            where
                Second: Integer,
                Metre: Integer,
                Kilogram: Integer,
                Ampere: Integer,
                Kelvin: Integer,
                Mole: Integer,
                Candela: Integer,
            {
                SiUnit::new(self)
            }
        }
        )*
    };
}

impl_into_unit!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128);
impl_into_unit!(f32 f64);
#[cfg(feature = "fixed")]
impl_into_unit!(
    I0F8 I0F16 I0F32 I0F64 I0F128 I1F7 I1F15 I1F31 I1F63 I1F127 I2F6 I2F14 I2F30
    I2F62 I2F126 I3F5 I3F13 I3F29 I3F61 I3F125 I4F4 I4F12 I4F28 I4F60 I4F124 I5F3
    I5F11 I5F27 I5F59 I5F123 I6F2 I6F10 I6F26 I6F58 I6F122 I7F1 I7F9 I7F25 I7F57
    I7F121 I8F0 I8F8 I8F24 I8F56 I8F120 I9F7 I9F23 I9F55 I9F119 I10F6 I10F22 I10F54
    I10F118 I11F5 I11F21 I11F53 I11F117 I12F4 I12F20 I12F52 I12F116 I13F3 I13F19 I13F51
    I13F115 I14F2 I14F18 I14F50 I14F114 I15F1 I15F17 I15F49 I15F113 I16F0 I16F16 I16F48
    I16F112 I17F15 I17F47 I17F111 I18F14 I18F46 I18F110 I19F13 I19F45 I19F109 I20F12
    I20F44 I20F108 I21F11 I21F43 I21F107 I22F10 I22F42 I22F106 I23F9 I23F41 I23F105
    I24F8 I24F40 I24F104 I25F7 I25F39 I25F103 I26F6 I26F38 I26F102 I27F5 I27F37 I27F101
    I28F4 I28F36 I28F100 I29F3 I29F35 I29F99 I30F2 I30F34 I30F98 I31F1 I31F33 I31F97
    I32F0 I32F32 I32F96 I33F31 I33F95 I34F30 I34F94 I35F29 I35F93 I36F28 I36F92 I37F27
    I37F91 I38F26 I38F90 I39F25 I39F89 I40F24 I40F88 I41F23 I41F87 I42F22 I42F86 I43F21
    I43F85 I44F20 I44F84 I45F19 I45F83 I46F18 I46F82 I47F17 I47F81 I48F16 I48F80 I49F15
    I49F79 I50F14 I50F78 I51F13 I51F77 I52F12 I52F76 I53F11 I53F75 I54F10 I54F74 I55F9
    I55F73 I56F8 I56F72 I57F7 I57F71 I58F6 I58F70 I59F5 I59F69 I60F4 I60F68 I61F3
    I61F67 I62F2 I62F66 I63F1 I63F65 I64F0 I64F64 I65F63 I66F62 I67F61 I68F60 I69F59
    I70F58 I71F57 I72F56 I73F55 I74F54 I75F53 I76F52 I77F51 I78F50 I79F49 I80F48 I81F47
    I82F46 I83F45 I84F44 I85F43 I86F42 I87F41 I88F40 I89F39 I90F38 I91F37 I92F36 I93F35
    I94F34 I95F33 I96F32 I97F31 I98F30 I99F29 I100F28 I101F27 I102F26 I103F25 I104F24
    I105F23 I106F22 I107F21 I108F20 I109F19 I110F18 I111F17 I112F16 I113F15 I114F14
    I115F13 I116F12 I117F11 I118F10 I119F9 I120F8 I121F7 I122F6 I123F5 I124F4 I125F3
    I126F2 I127F1 I128F0 U0F8 U0F16 U0F32 U0F64 U0F128 U1F7 U1F15 U1F31 U1F63 U1F127
    U2F6 U2F14 U2F30 U2F62 U2F126 U3F5 U3F13 U3F29 U3F61 U3F125 U4F4 U4F12 U4F28
    U4F60 U4F124 U5F3 U5F11 U5F27 U5F59 U5F123 U6F2 U6F10 U6F26 U6F58 U6F122 U7F1
    U7F9 U7F25 U7F57 U7F121 U8F0 U8F8 U8F24 U8F56 U8F120 U9F7 U9F23 U9F55 U9F119
    U10F6 U10F22 U10F54 U10F118 U11F5 U11F21 U11F53 U11F117 U12F4 U12F20 U12F52 U12F116
    U13F3 U13F19 U13F51 U13F115 U14F2 U14F18 U14F50 U14F114 U15F1 U15F17 U15F49 U15F113
    U16F0 U16F16 U16F48 U16F112 U17F15 U17F47 U17F111 U18F14 U18F46 U18F110 U19F13
    U19F45 U19F109 U20F12 U20F44 U20F108 U21F11 U21F43 U21F107 U22F10 U22F42 U22F106
    U23F9 U23F41 U23F105 U24F8 U24F40 U24F104 U25F7 U25F39 U25F103 U26F6 U26F38 U26F102
    U27F5 U27F37 U27F101 U28F4 U28F36 U28F100 U29F3 U29F35 U29F99 U30F2 U30F34 U30F98
    U31F1 U31F33 U31F97 U32F0 U32F32 U32F96 U33F31 U33F95 U34F30 U34F94 U35F29 U35F93
    U36F28 U36F92 U37F27 U37F91 U38F26 U38F90 U39F25 U39F89 U40F24 U40F88 U41F23 U41F87
    U42F22 U42F86 U43F21 U43F85 U44F20 U44F84 U45F19 U45F83 U46F18 U46F82 U47F17 U47F81
    U48F16 U48F80 U49F15 U49F79 U50F14 U50F78 U51F13 U51F77 U52F12 U52F76 U53F11 U53F75
    U54F10 U54F74 U55F9 U55F73 U56F8 U56F72 U57F7 U57F71 U58F6 U58F70 U59F5 U59F69
    U60F4 U60F68 U61F3 U61F67 U62F2 U62F66 U63F1 U63F65 U64F0 U64F64 U65F63 U66F62
    U67F61 U68F60 U69F59 U70F58 U71F57 U72F56 U73F55 U74F54 U75F53 U76F52 U77F51 U78F50
    U79F49 U80F48 U81F47 U82F46 U83F45 U84F44 U85F43 U86F42 U87F41 U88F40 U89F39 U90F38
    U91F37 U92F36 U93F35 U94F34 U95F33 U96F32 U97F31 U98F30 U99F29 U100F28 U101F27
    U102F26 U103F25 U104F24 U105F23 U106F22 U107F21 U108F20 U109F19 U110F18 U111F17
    U112F16 U113F15 U114F14 U115F13 U116F12 U117F11 U118F10 U119F9 U120F8 U121F7 U122F6
    U123F5 U124F4 U125F3 U126F2 U127F1 U128F0
);
