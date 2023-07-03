use crate::SiUnit;
use typenum::consts::{N1, N2, N3, P1, P2, P3, P4, Z0};

macro_rules! unit {
    ($acronym: ident -> $second: ty, $metre: ty, $kilogram: ty, $ampere: ty, $kelvin: ty, $mole: ty, $candela: ty $(; $doc: literal)?) => {
        $(
        #[doc = $doc]
        )?
        pub type $acronym<T> = SiUnit<T, $second, $metre, $kilogram, $ampere, $kelvin, $mole, $candela>;
    };
}

// Base units
unit!(Unit     -> Z0, Z0, Z0, Z0, Z0, Z0, Z0; "unit");
unit!(Second   -> P1, Z0, Z0, Z0, Z0, Z0, Z0; "time");
unit!(Metre    -> Z0, P1, Z0, Z0, Z0, Z0, Z0; "length");
unit!(Kilogram -> Z0, Z0, P1, Z0, Z0, Z0, Z0; "mass");
unit!(Ampere   -> Z0, Z0, Z0, P1, Z0, Z0, Z0; "electric current");
unit!(Kelvin   -> Z0, Z0, Z0, Z0, P1, Z0, Z0; "thermodynamic temperature");
unit!(Mole     -> Z0, Z0, Z0, Z0, Z0, P1, Z0; "amount of substance");
unit!(Candela  -> Z0, Z0, Z0, Z0, Z0, Z0, P1; "luminous intensity");

// Derived units with special names
unit!(Radian    -> Z0, Z0, Z0, Z0, Z0, Z0, Z0; "plane angle");
unit!(Steradian -> Z0, Z0, Z0, Z0, Z0, Z0, Z0; "solid angle");
unit!(Herz      -> N1, Z0, Z0, Z0, Z0, Z0, Z0; "frequency");
unit!(Newton    -> N2, P1, P1, Z0, Z0, Z0, Z0; "force");
unit!(Pascal    -> N2, N1, P1, Z0, Z0, Z0, Z0; "pressure, stress");
unit!(Joule     -> N2, P2, P1, Z0, Z0, Z0, Z0; "energy, work, heat");
unit!(Watt      -> N3, P2, P1, Z0, Z0, Z0, Z0; "power, radiant flux");
unit!(Coulomb   -> P1, Z0, Z0, P1, Z0, Z0, Z0; "electric charge");
unit!(Volt      -> N3, P2, P1, N1, Z0, Z0, Z0; "electric potential, voltage, emf");
unit!(Farad     -> P4, N2, N1, P2, Z0, Z0, Z0; "capacitance");
unit!(Ohm       -> N3, P2, P1, N2, Z0, Z0, Z0; "resistance, impedance, reactance");
unit!(Siemens   -> P3, N2, N1, P2, Z0, Z0, Z0; "electrical conductance");
unit!(Weber     -> N2, P2, P1, N1, Z0, Z0, Z0; "magnetic flux");
unit!(Tesla     -> N2, Z0, P1, N1, Z0, Z0, Z0; "magnetic flux density");
unit!(Henry     -> N2, P2, P1, N2, Z0, Z0, Z0; "inductance");
unit!(Lumen     -> Z0, Z0, Z0, Z0, Z0, Z0, P1; "luminous flux");
unit!(Lux       -> Z0, N2, Z0, Z0, Z0, Z0, P1; "iluminance");
unit!(Becquerel -> N1, Z0, Z0, Z0, Z0, Z0, Z0; "activity referred to a radionuclide");
unit!(Gray      -> N2, P2, Z0, Z0, Z0, Z0, Z0; "absorbed dose");
unit!(Sievert   -> N2, P2, Z0, Z0, Z0, Z0, Z0; "equivalent dose");
unit!(Katal     -> N1, Z0, Z0, Z0, Z0, P1, Z0; "catalytic activity");

// Derived units without special names
unit!(SquareMetre            -> Z0, P2, Z0, Z0, Z0, Z0, Z0; "area");
unit!(CubicMetre             -> Z0, P3, Z0, Z0, Z0, Z0, Z0; "volume");
unit!(MetrePerSecond         -> N1, P1, Z0, Z0, Z0, Z0, Z0; "speed, velocity");
unit!(MetrePerSquareSecond   -> N2, P1, Z0, Z0, Z0, Z0, Z0; "acceleration");
unit!(MetrePerCubeSecond     -> N3, P1, Z0, Z0, Z0, Z0, Z0; "jerk");
unit!(ReciprocalMetre        -> Z0, N1, Z0, Z0, Z0, Z0, Z0; "wavenumber, vergence");
unit!(KilogramPerCubicMetre  -> Z0, N3, P1, Z0, Z0, Z0, Z0; "density, mass concentration");
unit!(KilogramPerSquareMetre -> Z0, N2, P1, Z0, Z0, Z0, Z0; "surface density");
unit!(CubicMetrePerKilogram  -> Z0, P3, N1, Z0, Z0, Z0, Z0; "specific density");
unit!(AmperePerSquareMetre   -> Z0, N2, Z0, P1, Z0, Z0, Z0; "current density");
unit!(AmperePerMetre         -> Z0, N1, Z0, P1, Z0, Z0, Z0; "magnetic field strength");
unit!(MolePerCubicMetre      -> Z0, N3, Z0, Z0, Z0, P1, Z0; "concentration");
unit!(CandelaPerSquareMetre  -> Z0, N2, Z0, Z0, Z0, Z0, P1; "luminance");

// Derived units including special names
unit!(PascalSecond                -> N1, N1, P1, Z0, Z0, Z0, Z0; "dynamic viscosity");
unit!(NewtonMetre                 -> N2, P2, P1, Z0, Z0, Z0, Z0; "moment of force");
unit!(NewtonPerMetre              -> N2, Z0, P1, Z0, Z0, Z0, Z0; "surface tension");
unit!(RadianPerSecond             -> N1, Z0, Z0, Z0, Z0, Z0, Z0; "angular velocity, angular frequency");
unit!(RadianPerSquareSecond       -> N2, Z0, Z0, Z0, Z0, Z0, Z0; "angular acceleration");
unit!(RadianPerCubeSecond         -> N3, Z0, Z0, Z0, Z0, Z0, Z0; "angular jerk");
unit!(WattPerSquareMetre          -> N3, Z0, P1, Z0, Z0, Z0, Z0; "heat flux density, irradiance");
unit!(JoulePerKelvin              -> N2, P2, P1, Z0, N1, Z0, Z0; "entropy, heat capacity");
unit!(JoulePerKilogramKelvin      -> N2, P2, Z0, Z0, N1, Z0, Z0; "specific heat capacity, specific entropy");
unit!(JoulePerKilogram            -> N2, P2, Z0, Z0, Z0, Z0, Z0; "specific energy");
unit!(WattPerMetreKelvin          -> N3, P1, P1, Z0, N1, Z0, Z0; "thermal conductivity");
unit!(JoulePerCubicMetre          -> N2, N1, P1, Z0, Z0, Z0, Z0; "energy density");
unit!(VoltPerMetre                -> N3, P1, P1, N1, Z0, Z0, Z0; "electric field strenght");
unit!(CoulombPerCubicMetre        -> P1, N3, Z0, P1, Z0, Z0, Z0; "electric charge density");
unit!(CoulombPerSquareMetre       -> P1, N2, Z0, P1, Z0, Z0, Z0; "surface charge density, electric flux density, electric displacement");
unit!(FaradPerMetre               -> P4, N3, N1, P2, Z0, Z0, Z0; "permittivity");
unit!(HenryPerMetre               -> N2, P1, P1, N2, Z0, Z0, Z0; "permeability");
unit!(JoulePerMole                -> N2, P2, P1, Z0, Z0, N1, Z0; "molar energy");
unit!(JoulePerMoleKelvin          -> N2, P2, P1, Z0, N1, N1, Z0; "molar entropy");
unit!(CoulombPerKilogram          -> P1, Z0, N1, P1, Z0, Z0, Z0; "exposue (x- and gamma-rays)");
unit!(GrayPerSecond               -> N3, P2, Z0, Z0, Z0, Z0, Z0; "absorbed dose rate");
unit!(WattPerSteradian            -> N3, P2, P1, Z0, Z0, Z0, Z0; "rAdiant intensity");
unit!(WattPerSquareMetreSteradian -> N3, Z0, P1, Z0, Z0, Z0, Z0; "radiance");
unit!(KatalPerCubicMetre          -> N1, N3, Z0, Z0, Z0, P1, Z0; "catalytic activity concentration");
