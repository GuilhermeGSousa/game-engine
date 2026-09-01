use std::path::PathBuf;

use asset_cook::{run_cook, CookOptions, Importer};
use gltf_loader::gltf_importer::GltfImporter;
use obj_loader::obj_importer::ObjImporter;
use render::importers::image_importer::ImageImporter;

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![
        Box::new(ImageImporter),
        Box::new(GltfImporter),
        Box::new(ObjImporter),
    ]
}

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: cook <manifest.toml> <source_root> <output_root>");
        std::process::exit(2);
    }

    let options = CookOptions {
        manifest_path: PathBuf::from(&args[1]),
        source_root: PathBuf::from(&args[2]),
        output_root: PathBuf::from(&args[3]),
    };

    let report = run_cook(&registered_importers(), &options);
    println!(
        "cooked: {}, skipped: {}, errors: {}",
        report.cooked.len(),
        report.skipped.len(),
        report.errors.len()
    );
    for error in &report.errors {
        eprintln!("error: {error:?}");
    }

    if !report.errors.is_empty() {
        std::process::exit(1);
    }
}
