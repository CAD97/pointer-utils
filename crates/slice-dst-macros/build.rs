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
            impl_dir_path.join("../src/slice_dst_macros_impl.wasm"),
        ) {
            Ok(_) => Ok(()),
            Err(e) => panic!("failed to copy wasm to final destination: {}", e),
        },
        Ok(_) => match fs::canonicalize("src/slice_dst_macros_impl.wasm") {
            Ok(_) => Ok(()), // succeed if the wasm is there already
            Err(_) => error_could_not_build_wasm(impl_dir_path),
        },
        Err(e) => panic!("failed to wait for `cargo` subprocess (source: {})", e),
    }
}

fn error_could_not_build_wasm(impl_dir_path: &Path) -> ! {
    let this_toolchain = match Command::new("rustup")
        .args("show active-toolchain".split(' '))
        .current_dir(impl_dir_path)
        .output()
    {
        Ok(this_toolchain) => this_toolchain,
        Err(_) => panic!(
            "

Could not build slice-dst macros, and you don't have `rustup` available.
The command
    build --release --target wasm32-unknown-unknown --target-dir target
must succeed when run in the directory
    {}
to build slice-dst's macros from source. Either do what's required to make
that command succeed in your installation of cargo, or duplicate a copy of
`slice_dst_macros_impl.wasm` provided with distributions of this crate not
from source (e.g. from crates-io) to the path
    {}
at which point building slice-dst's macros will work properly.

",
            impl_dir_path.display(),
            impl_dir_path
                .join("../src/slice_dst_macros_impl.wasm")
                .display(),
        ),
    };

    assert!(this_toolchain.status.success());
    let this_toolchain = String::from_utf8_lossy(&this_toolchain.stdout);
    panic!(
        "

Could not build slice-dst macros, likely because you don't have the
`wasm32-unknown-unknown` target installed in your default profile.
When building from distribution, the macro implementation wasm is provied
precompiled, but when building from source, it's built using the default
profile (and not an override for wherever the build command was issued).
The toolchain being used to build the slice-dst macros is
    {}
and you should be able to fix this error by running
    rustup target add wasm32-unknown-unknown --toolchain {}

",
        this_toolchain,
        this_toolchain.split(' ').next().unwrap(),
    );
}
