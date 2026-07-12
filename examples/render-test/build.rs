use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap()
}

fn get_output_path() -> PathBuf {
    // The PROFILE env var only ever reports "debug" or "release", which is
    // wrong for custom profiles (e.g. --profile profiling). OUT_DIR is
    // target/<profile>/build/<pkg>-<hash>/out, so the profile directory is
    // three levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    out_dir.ancestors().nth(3).unwrap().to_path_buf()
}

fn main() -> anyhow::Result<()> {
    let res_path = manifest_dir().join("res");
    println!("cargo:rerun-if-changed={}", res_path.display());

    let output_path = get_output_path();
    copy_items(
        &[res_path],
        Path::new(&output_path),
        &CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )?;

    Ok(())
}
