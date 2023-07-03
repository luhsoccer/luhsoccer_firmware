use prost_build::compile_protos;

fn main() -> Result<(), std::io::Error> {
    // This is necessary so that the user doesn't have to install protoc
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());

    compile_protos(
        &[
            "files/luhsoccer/luhsoccer_basestation.proto",
            "files/ssl_vision/ssl_vision_wrapper.proto",
        ],
        &[
            "files/luhsoccer",
            "files/ssl_simulation",
            "files/ssl_vision",
        ],
    )
}
