use std::{fs, path::Path};

use pixelpipe_app::{
    BrowseProject, ImportReference, InitializeAsset, OpenProject, PreviewSelectedReference,
    browse_project, import_reference, initialize_asset, open_project, preview_selected_reference,
};
use pixelpipe_project::{AssetKind, StoredConversionMode};

#[test]
fn conversion_preview_is_ephemeral_and_accepts_settings_overrides() {
    let (game, recipe, mut settings) = selected_reference_project();
    settings.width = 16;
    settings.height = 16;

    let before = project_bytes(game.path());
    let preview = preview_selected_reference(PreviewSelectedReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        recipe,
        settings: Some(settings),
    })
    .unwrap();

    assert_eq!(
        (preview.inspection.width, preview.inspection.height),
        (16, 16)
    );
    assert_eq!(&preview.native_png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(project_bytes(game.path()), before);
    let browser = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .unwrap();
    assert!(browser.assets[0].asset.head.is_none());
    assert!(browser.assets[0].revisions.is_empty());
}

#[test]
fn invalid_preview_does_not_change_project_state() {
    let (game, recipe, mut settings) = selected_reference_project();
    settings.width = 0;
    let before = project_bytes(game.path());

    assert!(
        preview_selected_reference(PreviewSelectedReference {
            start: game.path().to_path_buf(),
            asset: "field-medic".to_owned(),
            recipe,
            settings: Some(settings),
        })
        .is_err()
    );
    assert_eq!(project_bytes(game.path()), before);
}

fn selected_reference_project() -> (
    tempfile::TempDir,
    String,
    pixelpipe_core::ConversionSettings,
) {
    let game = tempfile::tempdir().unwrap();
    let reference = game.path().join("medic-reference.png");
    write_reference(&reference);
    let opened = open_project(OpenProject {
        start: game.path().to_path_buf(),
    })
    .unwrap();
    initialize_asset(InitializeAsset {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        kind: AssetKind::Sprite,
        brief: "Strict overhead field medic".to_owned(),
    })
    .unwrap();
    import_reference(ImportReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        file: reference,
    })
    .unwrap();
    let recipe = opened
        .recipes
        .into_iter()
        .find(|entry| entry.id == "sprite-32")
        .unwrap();
    let settings = match recipe.mode {
        StoredConversionMode::Reference { settings } => settings,
        StoredConversionMode::Sheet { .. } => unreachable!(),
    };
    (game, recipe.id, settings)
}

fn project_bytes(root: &Path) -> Vec<u8> {
    fs::read(root.join(".pixelpipe/assets/field-medic/asset.toml")).unwrap()
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
                pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4]
                    .copy_from_slice(&[70, 120, 150, 255]);
            }
        }
    }
    writer.write_image_data(&pixels).unwrap();
}
