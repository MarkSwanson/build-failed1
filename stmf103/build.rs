
use std::fs::File;
use std::{io::Write, path::PathBuf};
use std::env;

fn main() -> Result<(), ()> {
    
    // Put the linker script somewhere the linker can find it
    let out_dir = &PathBuf::from(env::var("OUT_DIR").expect("Missing environment variable \"OUT_DIR\""));

    File::create(out_dir.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());

    Ok(())
}

