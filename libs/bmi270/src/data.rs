use fixed::types::{I16F16, I23F9};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Temperature(pub I23F9);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Gyro {
    pub x: I16F16,
    pub y: I16F16,
    pub z: I16F16,
}

impl Gyro {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        const RANGE: I16F16 = I16F16::unwrapped_from_str("2000").div_euclid_int(360);
        let x = i16::from_le_bytes((&bytes[0..2]).try_into().expect("0..2 gives 2 bytes"));
        let y = i16::from_le_bytes((&bytes[2..4]).try_into().expect("2..4 gives 2 bytes"));
        let z = i16::from_le_bytes((&bytes[4..6]).try_into().expect("4..6 gives 2 bytes"));
        let x = I16F16::from_bits(i32::from(x) << 1) * RANGE;
        let y = I16F16::from_bits(i32::from(y) << 1) * RANGE;
        let z = I16F16::from_bits(i32::from(z) << 1) * RANGE;
        Self { x, y, z }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Accel {
    pub x: I16F16,
    pub y: I16F16,
    pub z: I16F16,
}

impl Accel {
    pub(crate) fn from_bytes(bytes: &[u8], (offset_x, offset_y): (I16F16, I16F16)) -> Self {
        const G: I16F16 = I16F16::unwrapped_from_str("9.80665");
        const RANGE: I16F16 = I16F16::unwrapped_from_str("4");
        const FACTOR: I16F16 = RANGE.unwrapped_mul(G);
        let x = i16::from_le_bytes((&bytes[0..2]).try_into().expect("0..2 gives 2 bytes"));
        let y = i16::from_le_bytes((&bytes[2..4]).try_into().expect("2..4 gives 2 bytes"));
        let z = i16::from_le_bytes((&bytes[4..6]).try_into().expect("4..6 gives 2 bytes"));
        let x = (I16F16::from_bits(i32::from(x) << 1) * FACTOR) - offset_x;
        let y = (I16F16::from_bits(i32::from(y) << 1) * FACTOR) - offset_y;
        let z = I16F16::from_bits(i32::from(z) << 1) * FACTOR;
        Self { x, y, z }
    }
}
