#![cfg_attr(any(not(test), target_arch = "arm"), no_std)]

pub mod commands;
pub mod controller;
#[cfg(feature = "async")]
pub mod nonblocking;

#[cfg(not(any(not(test), target_arch = "arm")))]
mod tests {
    use super::commands::{
        ChipinfoAddr, ChipinfoDataSiType, ChipinfoDataType, TMC4671Command, TMC4671Field,
        TMC4671WriteCommand,
    };

    #[test]
    fn serialize_field() {
        let a = -10i16;
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<8, 16>(&mut buf);
        assert_eq!(buf, [0, 0, 0b1111_1111, 0b1111_0110, 0]);
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<6, 12>(&mut buf);
        assert_eq!(buf, [0, 0, 0b0000_0011, 0b1111_1101, 0b1000_0000]);

        let a = 10u16;
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<8, 16>(&mut buf);
        assert_eq!(buf, [0, 0, 0, 0b000_1010, 0]);
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<6, 12>(&mut buf);
        assert_eq!(buf, [0, 0, 0, 0b0000_0010, 0b1000_0000]);

        let a = true;
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<8, 16>(&mut buf);
        assert_eq!(buf, [0, 0, 0, 0b0000_0001, 0]);
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<6, 12>(&mut buf);
        assert_eq!(buf, [0, 0, 0, 0, 0b0100_0000]);

        let a = 'a';
        let mut buf = [0, 0, 0, 0, 0];
        a.serialize_field::<8, 8>(&mut buf);
        assert_eq!(buf, [0, 0, 0, 0x61, 0]);
    }

    #[test]
    fn deserialize_field() {
        // random bytes
        let a = [0b1011_1100, 0b1001_1011, 0b0011_1111, 0b0010_0100];
        #[allow(clippy::cast_possible_wrap)]
        let expected = 0b1001_1011_0011_1111_u16 as i16;
        assert_eq!(i16::deserialize_field::<8, 16>(a), Ok(expected));
        #[allow(clippy::cast_possible_wrap)]
        let expected = 0b1111_1100_1111_1100_u16 as i16;
        assert_eq!(i16::deserialize_field::<6, 12>(a), Ok(expected));

        assert_eq!(
            u16::deserialize_field::<8, 16>(a),
            Ok(0b1001_1011_0011_1111)
        );
        assert_eq!(
            u16::deserialize_field::<6, 12>(a),
            Ok(0b0000_1100_1111_1100)
        );

        assert_eq!(bool::deserialize_field::<7, 1>(a), Ok(false));
        assert_eq!(bool::deserialize_field::<10, 1>(a), Ok(true));

        assert_eq!(char::deserialize_field::<0, 8>(a), Ok('$'));
    }

    #[test]
    fn serialize_write() {
        let a = ChipinfoAddr {
            addr: ChipinfoDataType::Time,
        };
        assert_eq!(a.serialize_write(), [0b1000_0001, 0, 0, 0, 0x03]);
    }

    #[test]
    fn serialize_read() {
        assert_eq!(ChipinfoDataSiType::serialize_read(), 0);
    }

    #[test]
    fn deserialize() {
        let a = [0x34, 0x36, 0x37, 0x31];
        let b = ChipinfoDataSiType {
            first: '4',
            second: '6',
            third: '7',
            fourth: '1',
        };
        assert_eq!(ChipinfoDataSiType::deserialize(a), Ok(b));
    }
}
