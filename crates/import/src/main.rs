use std::path::PathBuf;

use anyhow::bail;

use import::config::{project_root_of, ContentConfig};
use import::import_source;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let mut source: Option<PathBuf> = None;
    let mut config_path: Option<PathBuf> = None;
    let mut extension: Option<String> = None;
    let mut content_root: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_path = args.next().map(PathBuf::from),
            "--ext" => extension = args.next(),
            "--content-root" => content_root = args.next(),
            other if other.starts_with("--") => bail!("unknown flag '{other}'"),
            other => source = Some(PathBuf::from(other)),
        }
    }

    let Some(source) = source else {
        eprintln!(
            "usage: import <source> [--config <content.toml>] [--ext <ext>] [--content-root <dir>]"
        );
        std::process::exit(2);
    };

    let config_path = config_path.unwrap_or_else(|| PathBuf::from("content.toml"));
    let project_root = project_root_of(&config_path);
    let mut config = ContentConfig::load_or_default(&project_root)?;
    if let Some(extension) = extension {
        config.extension = extension;
    }
    if let Some(root) = content_root {
        config.root = root;
    }

    let written = import_source(&source, &project_root, &config)?;
    println!("imported {} -> {} assets", source.display(), written.len());
    for address in &written {
        println!("  {address}");
    }
    Ok(())
}
