use std::{fs, io, path::Path, process::Command};

pub fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src/slice_dst_macros_impl.wasm",);
    let impl_dir_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/impl"));
    assert!(impl_dir_path.is_dir());
    println!("cargo:rerun-if-changed={}", impl_dir_path.display());
    let mut to_flag: Vec<_> = fs::read_dir(&impl_dir_path)?.collect();
    while let Some(entry) = to_flag.pop() {
        let entry = entry?;
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            to_flag.extend(fs::read_dir(path)?);
        }
    }

    println!(
        "{}> cargo build --release --target wasm32-unknown-unknown --target-dir target",
        impl_dir_path.display(),
    );
    let mut child = match Command::new("cargo")
        .args("build --release --target wasm32-unknown-unknown --target-dir target".split(' '))
        .current_dir(impl_dir_path)
        .spawn()
    {
        Ok(child) => child,
        Err(e) => panic!("failed to spawn `cargo` subprocess (source: {})", e),
    };

    match child.wait() {
        Ok(status) if status.success() => match fs::copy(
            impl_dir_path.join("target/wasm32-unknown-unknown/release/slice_dst_macros_impl.wasm"), 
            impl_dir_path.join("../src/slice_dst_macros_impl.wasm")
        ) {
            Ok(_) => Ok(()),
            Err(e) => panic!("failed to copy wasm to final destination: {}", e),
        },
        Ok(_) => match fs::canonicalize("src/slice_dst_macros_impl.wasm") {
            Ok(_) => Ok(()), // succeed if the wasm is there already
            Err(_) => panic!("failed to build wasm; you need the `wasm32-unknown-unknown` target to build slice-dst macros from source"),
        },
        Err(e) => panic!("failed to wait for `cargo` subprocess (source: {})", e),
    }
}
