use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;
use std::path::{Path, PathBuf};

fn get_output_path() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let path = Path::new(&manifest_dir_string)
        .join("target")
        .join(build_type);
    return PathBuf::from(path);
}

fn main() -> anyhow::Result<()> {
    // This tells Cargo to rerun this script if something in /res/ changes.
    println!("cargo:rerun-if-changed=res/*");

    let output_path = get_output_path();
    println!(
        "cargo:warning=Calculated build path: {}",
        output_path.to_str().unwrap()
    );

    let input_path = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("res");
    let output_path = Path::new(&output_path);
    //let res = std::fs::copy(input_path, output_path);
    copy_items(
        &[input_path],
        &output_path,
        &CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )?;

    Ok(())
}
