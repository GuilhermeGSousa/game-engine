use std::env;
use std::path::{Path, PathBuf};

use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

/// Copies the cooked `res/` directory next to the built binary, so the
/// executable-relative `CookedAssetRoot::Directory` default finds it.
/// `res/` is produced by `cook`; if it does not exist yet, do nothing.
fn main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
    let res_path = manifest_dir.join("res");
    println!("cargo:rerun-if-changed={}", res_path.display());

    if !res_path.exists() {
        return Ok(());
    }

    // OUT_DIR is target/<profile>/build/<pkg>-<hash>/out, so the profile
    // directory — where the binary lands — is three levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = out_dir.ancestors().nth(3).unwrap().to_path_buf();

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
