//! Covers ImageImporter producing a CookedTexture from a raw image file.
use std::path::Path;

use asset_cook::{CookedAsset, ImportContext, Importer};
use render::assets::cooked_texture::CookedTexture;
use render::importers::image_importer::ImageImporter;

fn write_test_png(path: &Path) {
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
    img.save(path).expect("failed to write test fixture PNG");
}

#[test]
fn import_produces_one_main_sub_asset_with_correct_pixels() {
    let temp_dir = std::env::temp_dir().join(format!("image-importer-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let source_path = temp_dir.join("swatch.png");
    write_test_png(&source_path);

    let mut ctx = ImportContext::new(std::path::PathBuf::from("swatch.png"));
    ImageImporter
        .import(&source_path, &mut ctx)
        .expect("importing a valid PNG should succeed");
    let outputs = ctx.into_parts();

    assert_eq!(outputs.sub_assets.len(), 1);
    assert_eq!(outputs.sub_assets[0].name, "main");
    assert_eq!(outputs.sub_assets[0].type_name, CookedTexture::TYPE_NAME);

    let cooked: CookedTexture = bincode::deserialize(&outputs.sub_assets[0].bytes).unwrap();
    assert_eq!(cooked.width, 2);
    assert_eq!(cooked.height, 2);
    assert_eq!(cooked.pixels.len(), 2 * 2 * 4);
    assert_eq!(&cooked.pixels[0..4], &[255, 0, 0, 255]);

    std::fs::remove_dir_all(&temp_dir).ok();
}
