use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    if target.starts_with("thumbv") {
        // Static library used for efc C functions
        fs::copy(format!("bin/{target}.a"), out_dir.join("libatsam4-hal.a")).unwrap();
        println!("cargo:rustc-link-lib=static=atsam4-hal");
    }

    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed=build.rs");
}
