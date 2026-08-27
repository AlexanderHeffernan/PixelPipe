use std::{collections::BTreeMap, fs};

use pixelate_core::{
    BackdropPolicy, ComponentExpectation, ConversionSettings, IndexedRaster, Palette,
    RASTER_SCHEMA, Registration, render, sha256_hex, stable_json,
};
use pixelate_project::{
    AssetKind, AssetState, AssetStyle, CONVERSION_RECIPE_SCHEMA, ConversionRecipeDocument,
    ProjectError, ProjectStore, RevisionManifest, StoredConversionMode,
};
use tempfile::tempdir;

use crate::*;

#[test]
fn use_case_creates_immutable_revision_chain() {
    let temp = tempdir().expect("tempdir");
    ProjectStore::init(temp.path(), "Test").expect("init");
    let input = temp.path().join("pixels.json");
    let raster = IndexedRaster {
        schema: RASTER_SCHEMA.to_owned(),
        width: 2,
        height: 1,
        palette: Palette::new("fixture", 0, vec![[0, 0, 0, 0], [255, 0, 0, 255]]),
        pixels: vec![0, 1],
        pivot: None,
        metadata: BTreeMap::new(),
    };
    fs::write(&input, stable_json(&raster).expect("json")).expect("write fixture");

    let create = || CreateRevision {
        start: temp.path().to_path_buf(),
        asset: "test-sprite".to_owned(),
        kind: AssetKind::Sprite,
        raster_path: input.clone(),
        brief_path: None,
        preview_scale: Some(2),
        actor: "test".to_owned(),
    };
    let first = create_revision(create()).expect("first revision");
    let first_native = fs::read(first.revision_path.join("native.png")).expect("first PNG");
    let second = create_revision(create()).expect("second revision");

    assert_eq!(first.revision, "r000001");
    assert_eq!(second.revision, "r000002");
    assert_eq!(second.parent.as_deref(), Some("r000001"));
    assert_eq!(
        fs::read(first.revision_path.join("native.png")).expect("first PNG after second"),
        first_native
    );
    assert_eq!(first.native_sha256, second.native_sha256);
    assert_eq!(first.preview_sha256, second.preview_sha256);

    let browser = browse_project(&BrowseProject {
        start: temp.path().join("nested"),
    })
    .expect("browse project from descendant");
    assert_eq!(browser.assets.len(), 1);
    assert_eq!(browser.assets[0].revisions.len(), 2);
    assert_eq!(browser.assets[0].asset.head.as_deref(), Some("r000002"));

    let view = load_revision_view(InspectRevision {
        start: temp.path().to_path_buf(),
        asset: "test-sprite".to_owned(),
        revision: Some("r000001".to_owned()),
    })
    .expect("verified revision view");
    assert_eq!(view.metadata.revision, "r000001");
    assert_eq!(view.metadata.palette.name, "fixture");
    assert_eq!(view.native_png, first_native);

    record_review(RecordReview {
        start: temp.path().to_path_buf(),
        asset: "test-sprite".to_owned(),
        revision: "r000001".to_owned(),
        actor: "reviewer".to_owned(),
        actor_kind: ReviewActorKind::Human,
        decision: ReviewDecision::Reviewed,
        note: "native size inspected".to_owned(),
    })
    .expect("record review");
    assert_eq!(
        ProjectStore::discover(temp.path())
            .expect("store")
            .asset("test-sprite")
            .expect("asset")
            .head
            .as_deref(),
        Some("r000002")
    );

    let manifest: RevisionManifest = serde_json::from_slice(
        &fs::read(first.revision_path.join("revision.json")).expect("revision manifest"),
    )
    .expect("revision manifest JSON");
    for (name, expected_hash) in manifest.files {
        let contents = fs::read(first.revision_path.join(name)).expect("hashed payload");
        assert_eq!(sha256_hex(&contents), expected_hash);
    }
}

#[test]
fn pre_revision_flow_snapshots_resources_and_creates_first_revision_atomically() {
    let (temp, store, brief) = selected_m6_project();
    let (palette, recipe, settings) = m6_resources(&store);
    let mut impossible = recipe.clone();
    impossible.id = "impossible".to_owned();
    impossible.mode = StoredConversionMode::Reference {
        settings: ConversionSettings {
            components: ComponentExpectation { min: 2, max: 2 },
            ..settings.clone()
        },
    };
    store
        .store_conversion_recipe(&impossible)
        .expect("impossible recipe is structurally valid");
    assert!(
        convert_selected_reference(ConvertSelectedReference {
            start: temp.path().to_path_buf(),
            asset: "signal-flare".to_owned(),
            recipe: impossible.id,
            palette: None,
            color_count: None,
            palette_overrides: Vec::new(),
            settings: None,
            auto_background: false,
            actor: "fixture".to_owned(),
        })
        .is_err()
    );
    assert!(store.asset("signal-flare").expect("asset").head.is_none());
    assert!(
        store
            .revisions("signal-flare")
            .expect("revisions")
            .is_empty()
    );

    let result = convert_selected_reference(ConvertSelectedReference {
        start: temp.path().to_path_buf(),
        asset: "signal-flare".to_owned(),
        recipe: recipe.id.clone(),
        palette: None,
        color_count: None,
        palette_overrides: Vec::new(),
        settings: None,
        auto_background: false,
        actor: "fixture".to_owned(),
    })
    .expect("first revision");
    assert_eq!(result.revision, "r000001");
    let asset = store.asset("signal-flare").expect("revisioned asset");
    assert_eq!(asset.state, AssetState::Revisioned);
    assert_eq!(asset.head.as_deref(), Some("r000001"));
    assert_eq!(
        asset.style,
        Some(AssetStyle {
            recipe: recipe.id.clone(),
            palette: None,
            color_count: Some(16),
            settings: settings.clone(),
        })
    );
    let before = store.revision("signal-flare", "r000001").expect("snapshot");
    assert_eq!(before.brief, brief);
    assert!(before.provenance.inputs.contains_key("brief"));
    assert!(
        before
            .provenance
            .inputs
            .contains_key("project_recipe:flare-reference")
    );

    update_asset_brief(UpdateAssetBrief {
        start: temp.path().to_path_buf(),
        asset: "signal-flare".to_owned(),
        brief: "Changed project brief".to_owned(),
    })
    .expect("edit project brief");
    let mut changed_palette = palette;
    changed_palette.name = "changed".to_owned();
    store
        .store_palette("flare", &changed_palette)
        .expect("edit palette resource");
    let mut changed_recipe = recipe;
    changed_recipe.preview_scale = 7;
    store
        .store_conversion_recipe(&changed_recipe)
        .expect("edit recipe resource");
    let after = store
        .revision("signal-flare", "r000001")
        .expect("unchanged snapshot");
    assert_eq!(before, after);
}

fn selected_m6_project() -> (tempfile::TempDir, ProjectStore, &'static str) {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "M6 Fixture").expect("init");
    let draft = initialize_asset(InitializeAsset {
        start: temp.path().to_path_buf(),
        asset: "signal-flare".to_owned(),
        kind: AssetKind::Sprite,
        brief: String::new(),
    })
    .expect("draft");
    assert_eq!(draft.state, AssetState::Draft);
    assert!(matches!(
        create_revision(CreateRevision {
            start: temp.path().to_path_buf(),
            asset: "signal-flare".to_owned(),
            kind: AssetKind::Sprite,
            raster_path: temp.path().join("must-not-be-read.json"),
            brief_path: None,
            preview_scale: None,
            actor: "fixture".to_owned(),
        }),
        Err(AppError::Project(ProjectError::AssetNotReady { .. }))
    ));
    assert!(matches!(
        inspect_revision(InspectRevision {
            start: temp.path().to_path_buf(),
            asset: "signal-flare".to_owned(),
            revision: None,
        }),
        Err(AppError::NoHead(_))
    ));
    let brief = "Strict overhead signal flare with one connected silhouette.";
    let awaiting = update_asset_brief(UpdateAssetBrief {
        start: temp.path().to_path_buf(),
        asset: "signal-flare".to_owned(),
        brief: brief.to_owned(),
    })
    .expect("brief");
    assert_eq!(awaiting.state, AssetState::AwaitingReference);
    let raster: IndexedRaster = serde_json::from_slice(include_bytes!(
        "../../pixelate-core/tests/fixtures/m1/tiny-raster.json"
    ))
    .expect("synthetic source raster");
    let bytes = render(&raster, 1).expect("synthetic source PNG").native_png;
    store
        .select_imported_reference("signal-flare", &bytes)
        .expect("imported reference");
    assert_eq!(
        store.asset("signal-flare").expect("selected asset").state,
        AssetState::SelectedReference
    );
    (temp, store, brief)
}

fn m6_resources(store: &ProjectStore) -> (Palette, ConversionRecipeDocument, ConversionSettings) {
    let palette = Palette::new(
        "m6-synthetic",
        0,
        vec![
            [0, 0, 0, 0],
            [92, 28, 24, 255],
            [238, 76, 36, 255],
            [255, 224, 112, 255],
        ],
    );
    store.store_palette("flare", &palette).expect("palette");
    let settings = ConversionSettings {
        width: 4,
        height: 4,
        color_treatment: pixelate_core::ColorTreatment::Original,
        color_adjustments: pixelate_core::ColorAdjustments::default(),
        margin: 0,
        subject_scale_percent: 100,
        offset_x: 0,
        offset_y: 0,
        coverage_percent: 1,
        backdrop: BackdropPolicy::Alpha { alpha_threshold: 0 },
        registration: Registration::Center,
        components: ComponentExpectation { min: 1, max: 1 },
    };
    let recipe = ConversionRecipeDocument {
        schema: CONVERSION_RECIPE_SCHEMA.to_owned(),
        id: "flare-reference".to_owned(),
        kind: AssetKind::Sprite,
        palette: "flare".to_owned(),
        preview_scale: 4,
        mode: StoredConversionMode::Reference {
            settings: settings.clone(),
        },
    };
    store.store_conversion_recipe(&recipe).expect("recipe");
    (palette, recipe, settings)
}
