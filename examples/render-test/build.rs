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
    let build_type = env::var("PROFILE").unwrap();
    manifest_dir()
        .join("..")
        .join("..")
        .join("target")
        .join(build_type)
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
