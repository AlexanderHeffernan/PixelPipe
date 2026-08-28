use std::{fs, path::Path};

use pixelate_app::{
    BrowseProject, CommitComposition, ConvertSelectedReference, ImportReference, InitializeAsset,
    OpenProject, PreviewComposition, PreviewSelectedReference, browse_project, commit_composition,
    convert_selected_reference, import_reference, initialize_asset, open_project,
    preview_composition, preview_selected_reference,
};

#[test]
fn conversion_preview_is_ephemeral_and_accepts_settings_overrides() {
    let (game, mut settings) = selected_reference_project();
    settings.width = 16;
    settings.height = 16;

    let before = project_bytes(game.path());
    let low_colour_preview = preview_selected_reference(PreviewSelectedReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        color_count: Some(2),
        palette_overrides: Vec::new(),
        settings: Some(settings.clone()),
        auto_background: false,
    })
    .unwrap();
    let preview = preview_selected_reference(PreviewSelectedReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        color_count: Some(12),
        palette_overrides: Vec::new(),
        settings: Some(settings),
        auto_background: false,
    })
    .unwrap();

    assert_eq!(
        (preview.inspection.width, preview.inspection.height),
        (16, 16)
    );
    assert_eq!(&preview.native_png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(preview.palette_name, "Source Colours");
    assert!(preview.inspection.palette.len() > low_colour_preview.inspection.palette.len());
    assert_ne!(preview.native_png, low_colour_preview.native_png);
    assert_eq!(project_bytes(game.path()), before);
    let browser = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .unwrap();
    assert!(browser.assets[0].asset.head.is_none());
    assert!(browser.assets[0].revisions.is_empty());
}

#[test]
fn preview_applies_source_palette_replacements_without_persisting() {
    let (game, settings) = selected_reference_project();
    let before = project_bytes(game.path());

    let preview = preview_selected_reference(PreviewSelectedReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        color_count: Some(4),
        palette_overrides: vec![pixelate_app::PaletteColorOverride {
            index: 1,
            rgba: [255, 0, 128, 255],
        }],
        settings: Some(settings),
        auto_background: false,
    })
    .unwrap();

    assert!(
        preview
            .inspection
            .palette
            .iter()
            .any(|entry| entry.rgba == [255, 0, 128, 255])
    );
    assert_eq!(project_bytes(game.path()), before);
}

#[test]
fn invalid_preview_does_not_change_project_state() {
    let (game, mut settings) = selected_reference_project();
    settings.width = 0;
    let before = project_bytes(game.path());

    assert!(
        preview_selected_reference(PreviewSelectedReference {
            start: game.path().to_path_buf(),
            asset: "field-medic".to_owned(),
            color_count: Some(12),
            palette_overrides: Vec::new(),
            settings: Some(settings),
            auto_background: false,
        })
        .is_err()
    );
    assert_eq!(project_bytes(game.path()), before);
}

#[test]
fn canvas_composition_previews_ephemerally_then_commits_one_revision() {
    let (game, settings) = selected_reference_project();
    let converted = convert_selected_reference(ConvertSelectedReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        color_count: Some(12),
        palette_overrides: Vec::new(),
        settings: Some(settings),
        auto_background: true,
        actor: "test".to_owned(),
    })
    .unwrap();
    let before = project_bytes(game.path());
    let canvas = pixelate_core::CanvasSettings {
        width: 40,
        height: 36,
        scale_percent: 150,
        offset_x: 7,
        offset_y: -5,
    };

    let preview = preview_composition(PreviewComposition {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        parent: converted.revision.clone(),
        settings: canvas,
    })
    .unwrap();
    assert_eq!(
        (preview.inspection.width, preview.inspection.height),
        (40, 36)
    );
    assert_eq!(project_bytes(game.path()), before);

    let composed = commit_composition(CommitComposition {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        parent: converted.revision,
        settings: canvas,
        actor: "test".to_owned(),
    })
    .unwrap();
    assert_eq!(composed.revision, "r000002");
    let browser = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .unwrap();
    assert_eq!(browser.assets[0].asset.head.as_deref(), Some("r000002"));
    assert_eq!(browser.assets[0].revisions.len(), 2);
}

fn selected_reference_project() -> (tempfile::TempDir, pixelate_core::ConversionSettings) {
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
        brief: "Strict overhead field medic".to_owned(),
        project_path: None,
    })
    .unwrap();
    import_reference(ImportReference {
        start: game.path().to_path_buf(),
        asset: "field-medic".to_owned(),
        file: reference,
    })
    .unwrap();
    (game, opened.pixelization.settings)
}

fn project_bytes(root: &Path) -> Vec<u8> {
    fs::read(root.join(".pixelate/assets/field-medic/asset.toml")).unwrap()
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
                let band = u8::try_from((x - 8) / 4).unwrap();
                pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&[
                    40 + band * 12,
                    70 + band * 9,
                    210_u8.saturating_sub(band * 11),
                    255,
                ]);
            }
        }
    }
    writer.write_image_data(&pixels).unwrap();
}
