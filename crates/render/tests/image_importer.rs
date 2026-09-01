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

#[test]
fn texture_from_cooked_preserves_dimensions_and_pixels() {
    let cooked = CookedTexture {
        width: 2,
        height: 1,
        srgb: true,
        pixels: vec![10, 20, 30, 255, 40, 50, 60, 255],
    };
    let texture = render::assets::texture::Texture::from_cooked(cooked);
    assert_eq!(
        texture.size().width,
        2,
        "from_cooked should carry through the cooked width"
    );
    assert_eq!(
        texture.size().height,
        1,
        "from_cooked should carry through the cooked height"
    );
    assert_eq!(
        texture.data(),
        &[10, 20, 30, 255, 40, 50, 60, 255],
        "from_cooked should move the cooked pixels through unchanged"
    );
}
