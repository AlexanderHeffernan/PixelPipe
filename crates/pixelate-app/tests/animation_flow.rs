use std::{fs, path::Path};

use pixelate_app::{
    ConvertSelectedReference, ExportAsset, FrameMutation, FrameMutationAction, ImportImageSequence,
    ImportReference, ImportSpritesheet, InitializeAsset, InspectRevision, OpenProject,
    PatchRevisionDocument, SetAssetHead, convert_selected_reference, export_asset,
    import_image_sequence, import_reference, import_spritesheet, initialize_asset,
    load_revision_view, mutate_frames, open_project, patch_revision_document, set_asset_head,
};
use pixelate_core::{PATCH_SCHEMA, PixelPatch, PixelPatchSet, sha256_hex, stable_json};
use pixelate_project::ProjectStore;

#[test]
fn frame_mutations_are_parent_linked_and_restore_as_a_whole() {
    let (game, base) = converted_project();
    let root = game.path().to_path_buf();
    let add = mutate_frames(FrameMutation {
        start: root.clone(),
        asset: "hero".into(),
        parent: base.clone(),
        action: FrameMutationAction::AddBlank {
            position: None,
            duration_ms: Some(90),
        },
        actor: "test".into(),
    })
    .unwrap();
    let blank_view = load_revision_view(InspectRevision {
        start: root.clone(),
        asset: "hero".into(),
        revision: Some(add.revision.clone()),
        frame_id: Some("frame-0002".into()),
    })
    .unwrap();
    assert_eq!(blank_view.metadata.inspection.visible_bounds, None);
    let duplicate = mutate_frames(FrameMutation {
        start: root.clone(),
        asset: "hero".into(),
        parent: add.revision.clone(),
        action: FrameMutationAction::Duplicate {
            frame_id: "frame-0001".into(),
            position: Some(1),
        },
        actor: "test".into(),
    })
    .unwrap();
    let duration = mutate_frames(FrameMutation {
        start: root.clone(),
        asset: "hero".into(),
        parent: duplicate.revision.clone(),
        action: FrameMutationAction::SetDuration {
            frame_id: "frame-0002".into(),
            duration_ms: 175,
        },
        actor: "test".into(),
    })
    .unwrap();
    let reordered = mutate_frames(FrameMutation {
        start: root.clone(),
        asset: "hero".into(),
        parent: duration.revision.clone(),
        action: FrameMutationAction::Reorder {
            frame_id: "frame-0002".into(),
            position: 0,
        },
        actor: "test".into(),
    })
    .unwrap();

    let (deleted_revision, imported_revision) = import_then_delete(&root, reordered.revision);

    let store = ProjectStore::discover(&root).unwrap();
    let snapshot = store.revision("hero", &deleted_revision).unwrap();
    assert_eq!(
        snapshot.manifest.parent.as_deref(),
        Some(imported_revision.as_str())
    );
    assert_eq!(
        snapshot
            .sequence
            .frames
            .iter()
            .map(|frame| frame.id.as_str())
            .collect::<Vec<_>>(),
        ["frame-0002", "frame-0004", "frame-0003"]
    );
    assert_eq!(snapshot.sequence.frames[0].duration_ms, 175);
    assert_eq!(snapshot.sequence.frames[1].duration_ms, 110);
    verify_targeted_edit_and_undo(&root, &store, &snapshot, &deleted_revision);

    let single_delete = mutate_frames(FrameMutation {
        start: root.clone(),
        asset: "hero".into(),
        parent: base,
        action: FrameMutationAction::Delete {
            frame_id: "frame-0001".into(),
        },
        actor: "test".into(),
    });
    assert!(
        single_delete
            .unwrap_err()
            .to_string()
            .contains("final remaining frame")
    );
}

fn import_then_delete(root: &Path, parent: String) -> (String, String) {
    let imported_source = root.join("imported-frame.png");
    write_image(&imported_source, 12, 12, [220, 30, 30, 255]);
    let imported = mutate_frames(FrameMutation {
        start: root.to_path_buf(),
        asset: "hero".into(),
        parent,
        action: FrameMutationAction::ImportFrame {
            file: imported_source,
            position: Some(2),
            duration_ms: Some(110),
        },
        actor: "test".into(),
    })
    .unwrap();
    let deleted = mutate_frames(FrameMutation {
        start: root.to_path_buf(),
        asset: "hero".into(),
        parent: imported.revision.clone(),
        action: FrameMutationAction::Delete {
            frame_id: "frame-0001".into(),
        },
        actor: "test".into(),
    })
    .unwrap();
    (deleted.revision, imported.revision)
}

fn verify_targeted_edit_and_undo(
    root: &Path,
    store: &ProjectStore,
    snapshot: &pixelate_project::RevisionSnapshot,
    revision: &str,
) {
    let patched = patch_revision_document(PatchRevisionDocument {
        start: root.to_path_buf(),
        asset: "hero".into(),
        parent: revision.to_owned(),
        patch: PixelPatchSet {
            schema: PATCH_SCHEMA.into(),
            edits: vec![PixelPatch {
                x: 0,
                y: 0,
                index: 0,
            }],
            structure: None,
        },
        frame_id: Some("frame-0003".into()),
        brief: None,
        actor: "test".into(),
    })
    .unwrap();
    let edited = store.revision("hero", &patched.revision).unwrap();
    assert_ne!(
        edited.sequence.frames[2].pixels,
        snapshot.sequence.frames[2].pixels
    );
    assert_eq!(
        edited.sequence.frames[0].pixels,
        snapshot.sequence.frames[0].pixels
    );

    set_asset_head(SetAssetHead {
        start: root.to_path_buf(),
        asset: "hero".into(),
        revision: revision.to_owned(),
    })
    .unwrap();
    assert_eq!(store.asset("hero").unwrap().head.as_deref(), Some(revision));
    assert_eq!(
        store.revision("hero", revision).unwrap().sequence,
        snapshot.sequence
    );
}

#[test]
fn ordered_imports_share_one_palette_and_export_deterministic_sheet_metadata() {
    let (game, base) = converted_project();
    let red = game.path().join("red.png");
    let blue = game.path().join("blue.png");
    write_image(&red, 12, 12, [220, 30, 30, 255]);
    write_image(&blue, 12, 12, [30, 40, 220, 255]);
    let request = || ImportImageSequence {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        parent: base.clone(),
        files: vec![blue.clone(), red.clone()],
        duration_ms: 75,
        actor: "test".into(),
    };
    let first = import_image_sequence(request()).unwrap();
    let second = import_image_sequence(request()).unwrap();
    let store = ProjectStore::discover(game.path()).unwrap();
    let first_snapshot = store.revision("hero", &first.revision).unwrap();
    let second_snapshot = store.revision("hero", &second.revision).unwrap();
    assert_eq!(first_snapshot.sequence, second_snapshot.sequence);
    assert_eq!(first_snapshot.sequence.frames.len(), 2);
    assert_eq!(first_snapshot.sequence.frames[0].duration_ms, 75);
    assert_ne!(
        first_snapshot.sequence.frames[0].pixels,
        first_snapshot.sequence.frames[1].pixels
    );

    set_asset_head(SetAssetHead {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        revision: first.revision.clone(),
    })
    .unwrap();
    let output = game.path().join("export");
    fs::create_dir(&output).unwrap();
    let exported = export_asset(ExportAsset {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        destination: output,
        overwrite: false,
    })
    .unwrap();
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(exported.metadata).unwrap()).unwrap();
    assert_eq!(metadata["schema"], "pixelate.spritesheet/v1");
    assert_eq!(metadata["sheet"]["width"], 64);
    assert_eq!(metadata["frames"][0]["x"], 0);
    assert_eq!(metadata["frames"][1]["x"], 32);
    assert_eq!(metadata["frames"][0]["duration_ms"], 75);
    let sheet = image::load_from_memory(&fs::read(exported.png).unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!((sheet.width(), sheet.height()), (64, 32));
    assert_sheet_pixels(&first_snapshot.sequence, &sheet);
}

fn assert_sheet_pixels(sequence: &pixelate_core::IndexedSequence, sheet: &image::RgbaImage) {
    for (order, frame) in sequence.frames.iter().enumerate() {
        for y in 0..sequence.height {
            for x in 0..sequence.width {
                let source =
                    usize::try_from(u64::from(y) * u64::from(sequence.width) + u64::from(x))
                        .unwrap();
                let expected = sequence.palette.colors[usize::from(frame.pixels[source])];
                assert_eq!(
                    sheet
                        .get_pixel(u32::try_from(order).unwrap() * sequence.width + x, y)
                        .0,
                    expected
                );
            }
        }
    }
}

#[test]
fn spritesheet_import_requires_explicit_valid_grid_order() {
    let (game, base) = converted_project();
    let sheet = game.path().join("sheet.png");
    write_sheet(&sheet);
    let imported = import_spritesheet(ImportSpritesheet {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        parent: base.clone(),
        file: sheet.clone(),
        frame_width: 8,
        frame_height: 8,
        order: vec![1, 0],
        duration_ms: 60,
        actor: "test".into(),
    })
    .unwrap();
    let snapshot = ProjectStore::discover(game.path())
        .unwrap()
        .revision("hero", &imported.revision)
        .unwrap();
    assert_eq!(snapshot.sequence.frames.len(), 2);
    assert_ne!(
        snapshot.sequence.frames[0].pixels,
        snapshot.sequence.frames[1].pixels
    );
    let invalid = import_spritesheet(ImportSpritesheet {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        parent: base,
        file: sheet,
        frame_width: 8,
        frame_height: 8,
        order: vec![2],
        duration_ms: 60,
        actor: "test".into(),
    });
    assert!(
        invalid
            .unwrap_err()
            .to_string()
            .contains("outside the explicit grid")
    );
}

#[test]
fn legacy_raster_revision_loads_without_changing_pixels_or_native_png() {
    let (game, revision) = converted_project();
    let store = ProjectStore::discover(game.path()).unwrap();
    let snapshot = store.revision("hero", &revision).unwrap();
    let original_png = snapshot.native_png.clone();
    let revision_path = snapshot.path;
    let raster_bytes = stable_json(&snapshot.raster).unwrap();
    fs::write(revision_path.join("pixels.json"), &raster_bytes).unwrap();
    let manifest_path = revision_path.join("revision.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"]["pixels.json"] = sha256_hex(&raster_bytes).into();
    fs::write(&manifest_path, stable_json(&manifest).unwrap()).unwrap();

    let loaded = store.revision("hero", &revision).unwrap();
    assert_eq!(loaded.sequence.frames.len(), 1);
    assert_eq!(loaded.raster, snapshot.raster);
    assert_eq!(loaded.native_png, original_png);
    assert_eq!(loaded.manifest.parent, snapshot.manifest.parent);
}

fn converted_project() -> (tempfile::TempDir, String) {
    let game = tempfile::tempdir().unwrap();
    open_project(OpenProject {
        start: game.path().to_path_buf(),
    })
    .unwrap();
    initialize_asset(InitializeAsset {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        brief: "Animated test hero".into(),
    })
    .unwrap();
    let source = game.path().join("source.png");
    write_image(&source, 16, 16, [80, 140, 180, 255]);
    import_reference(ImportReference {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        file: source,
    })
    .unwrap();
    let revision = convert_selected_reference(ConvertSelectedReference {
        start: game.path().to_path_buf(),
        asset: "hero".into(),
        color_count: Some(4),
        palette_overrides: vec![],
        settings: None,
        auto_background: false,
        actor: "test".into(),
    })
    .unwrap()
    .revision;
    (game, revision)
}

fn write_image(path: &Path, width: u32, height: u32, color: [u8; 4]) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&color.repeat((width * height) as usize))
        .unwrap();
}

fn write_sheet(path: &Path) {
    let mut pixels = Vec::new();
    for _y in 0..8 {
        for x in 0..16 {
            pixels.extend_from_slice(if x < 8 {
                &[220, 30, 30, 255]
            } else {
                &[30, 40, 220, 255]
            });
        }
    }
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, 16, 8);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
}
