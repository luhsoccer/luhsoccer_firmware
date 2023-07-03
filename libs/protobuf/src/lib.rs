#![no_std]
pub mod proto {
    pub mod luhsoccer {
        include!(concat!(env!("OUT_DIR"), "/luhsoccer.proto.basestation.rs"));
    }
    pub mod ssl_vision {
        include!(concat!(env!("OUT_DIR"), "/luhsoccer.proto.ssl_vision.rs"));
    }
}
