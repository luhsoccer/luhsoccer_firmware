#![no_std]

//! This crate defines the communication interfaces used by the firmware.
//! The graph looks like this:
//!
//! ```text
//! Server                Vision                  Game Controller
//!    |                     |                           |
//!    | Protobuff           | Protobuff                 | Protobuff
//!    |                    \/                           |
//!    \-------------> Basestation <---------------------/
//!                         |
//!                         | Postcard
//!                        \/            Postcard
//!                  Maincontroller <----------------> Motorcontroller
//! ```
//!
//! All the structs used in postcard communication are defined in `definitions`.
//! The Basestation sends `BasestationToRobot` structs to the Maincontroller.
//! The Maincontroller sends `RobotToBasestation` structs to the Basestation.
//! The Maincontroller only sends one packet to the basestation after receiving a packet.

pub use konst;

pub mod definitions;
pub mod uart;

pub const BASESTATION_SYNC_WORD: u32 = 0x9cd6_040c;
pub const BROADCAST_SYNC_WORD: u32 = 0xb9d1_6e9c;
pub const ROBOT_BLUE_SYNC_WORDS: [u32; 16] = [
    0xab60_615e,
    0x1902_97ab,
    0x56e2_4fb8,
    0xbfe5_e129,
    0x1e0c_14e8,
    0x85e9_cb3a,
    0xe0b4_d33f,
    0x01ae_1bb5,
    0x42a3_3fa0,
    0xa273_2908,
    0x6aeb_d021,
    0xbbce_a667,
    0x76e4_a78d,
    0x3ce0_e5d3,
    0xc66e_0d5c,
    0xfafe_4934,
];
pub const ROBOT_YELLOW_SYNC_WORDS: [u32; 16] = [
    0x95da_9603,
    0xd0eb_1461,
    0x15ba_5654,
    0xbb2b_f452,
    0x2c14_0646,
    0x62e6_1ebe,
    0xb2d5_4232,
    0xc692_9a96,
    0x6366_8943,
    0x5e7e_eba1,
    0x254a_8d13,
    0x1e6b_1077,
    0x5fae_4041,
    0x4d45_7592,
    0x320a_fa60,
    0x99fb_1ae9,
];

#[macro_export]
macro_rules! crate_version {
    () => {
        $crate::definitions::SemVersion {
            major: $crate::konst::result::unwrap_ctx!($crate::konst::primitive::parse_u8(env!(
                "CARGO_PKG_VERSION_MAJOR"
            ))),
            minor: $crate::konst::result::unwrap_ctx!($crate::konst::primitive::parse_u8(env!(
                "CARGO_PKG_VERSION_MINOR"
            ))),
            patch: $crate::konst::result::unwrap_ctx!($crate::konst::primitive::parse_u8(env!(
                "CARGO_PKG_VERSION_PATCH"
            ))),
        }
    };
}
pub const VERSION: definitions::SemVersion = crate_version!();

#[cfg(test)]
mod tests {
    use crate::definitions::SemVersion;

    #[test]
    fn version() {
        assert_eq!(
            crate::VERSION,
            SemVersion {
                major: 0,
                minor: 3,
                patch: 0
            }
        )
    }
}
