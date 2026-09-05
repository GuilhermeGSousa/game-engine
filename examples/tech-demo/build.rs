use std::env;
use std::path::{Path, PathBuf};

use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

/// Copies the `content/` directory next to the built binary, so the
/// executable-relative `ContentAssetRoot::Directory` default finds it.
fn main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
    let content_path = manifest_dir.join("content");
    println!("cargo:rerun-if-changed={}", content_path.display());

    if !content_path.exists() {
        return Ok(());
    }

    // OUT_DIR is target/<profile>/build/<pkg>-<hash>/out, so the profile
    // directory — where the binary lands — is three levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = out_dir.ancestors().nth(3).unwrap().to_path_buf();

    copy_items(
        &[content_path],
        Path::new(&output_path),
        &CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )?;

    Ok(())
}
