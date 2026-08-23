use std::{fs, path::Path};

use pixelpipe_app::{
    ConvertSelectedReference, ExportAsset, ImportReference, InitializeAsset, OpenProject,
    convert_selected_reference, export_asset, import_reference, initialize_asset, open_project,
};
use pixelpipe_project::AssetKind;

#[test]
fn folder_to_export_uses_starter_resources_without_manual_json() {
    let game = tempfile::tempdir().unwrap();
    let reference = game.path().join("medic-reference.png");
    write_reference(&reference);

    let opened = open_project(OpenProject {
        start: game.path().to_path_buf(),
    })
    .unwrap();
    assert_eq!(opened.recipes.len(), 3);

    initialize_asset(InitializeAsset {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        kind: AssetKind::Sprite,
        brief: "Strict overhead field medic with a compact silhouette".to_owned(),
    })
    .unwrap();
    import_reference(ImportReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        file: reference,
    })
    .unwrap();
    let converted = convert_selected_reference(ConvertSelectedReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        recipe: "sprite-32".to_owned(),
        actor: "user".to_owned(),
    })
    .unwrap();

    let output = game.path().join("runtime-assets");
    fs::create_dir(&output).unwrap();
    let exported = export_asset(ExportAsset {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        destination: output,
        overwrite: false,
    })
    .unwrap();
    assert_eq!(exported.revision, converted.revision);
    assert!(exported.png.is_file());
    assert!(exported.metadata.is_file());
    assert_eq!(&fs::read(&exported.png).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
}

fn write_reference(path: &Path) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, 64, 64);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let mut pixels = vec![255; 64 * 64 * 4];
    for y in 10_usize..58 {
        for x in 8_usize..56 {
            if x.abs_diff(32) + y.abs_diff(34) < 25 {
                let offset = (y * 64 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&[70, 120, 150, 255]);
            }
        }
    }
    writer.write_image_data(&pixels).unwrap();
}
