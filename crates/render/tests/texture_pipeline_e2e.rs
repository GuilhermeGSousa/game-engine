//! End-to-end proof that a texture can be cooked and then read at its
//! deterministic ID-keyed location — no source image decode at load time.
use asset_cook::{cooked_file_path_for_id, run_cook, CookOptions, Importer};
use essential::assets::AssetId;
use render::importers::image_importer::ImageImporter;

#[test]
fn cooked_texture_is_reachable_by_its_deterministic_id() {
    let temp_dir = std::env::temp_dir().join(format!("texture-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(source_root.join("textures")).unwrap();

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([9, 9, 9, 255]));
    img.save(source_root.join("textures/swatch.png"))
        .expect("failed to save fixture texture");

    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(
        &manifest_path,
        "[[assets]]\npath = \"textures/swatch.png\"\n",
    )
    .expect("failed to write manifest");

    let importers: Vec<Box<dyn Importer>> = vec![Box::new(ImageImporter)];
    let options = CookOptions {
        manifest_path,
        source_root,
        output_root: output_root.clone(),
    };
    let report = run_cook(&importers, &options);
    assert!(
        report.errors.is_empty(),
        "cooking the fixture texture must succeed: {:?}",
        report.errors
    );

    let expected_id = AssetId::from_path("textures/swatch.png#main");
    let cooked_path = cooked_file_path_for_id(&output_root, expected_id);
    assert!(
        cooked_path.exists(),
        "the cooked texture must be reachable purely from its AssetId"
    );

    let cooked_bytes = std::fs::read(&cooked_path).expect("failed to read cooked texture file");
    let cooked: render::assets::cooked_texture::CookedTexture =
        bincode::deserialize(&cooked_bytes).expect("failed to deserialize cooked texture");
    assert_eq!(
        cooked.pixels,
        vec![9, 9, 9, 255],
        "cooked texture pixel data must match fixture image"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
