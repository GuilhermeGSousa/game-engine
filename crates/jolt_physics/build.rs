fn main() {
    // Jolt and JoltC are compiled as C++ (via `joltc-sys`), but that crate only
    // links the `Jolt`/`joltc` static archives — not the C++ standard library
    // those archives depend on. Link it here so binaries (including this
    // crate's tests and any downstream consumer) resolve the C++ runtime.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" | "ios" => println!("cargo:rustc-link-lib=dylib=c++"),
        // Linux/Android and the GNU toolchain on Windows use libstdc++.
        _ => println!("cargo:rustc-link-lib=dylib=stdc++"),
    }
}
