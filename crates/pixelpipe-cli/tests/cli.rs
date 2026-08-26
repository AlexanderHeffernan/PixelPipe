use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn cli_init_create_and_inspect_share_json_contract() {
    let project = tempdir().expect("project tempdir");
    let init = run(&[
        "init",
        "--root",
        project.path().to_str().expect("UTF-8 project path"),
        "--name",
        "CLI Fixture",
    ]);
    assert_eq!(init["ok"], true);
    assert_eq!(init["schema"], "pixelpipe.project/v1");

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m1/tiny-raster.json");
    let revision = run(&[
        "revision",
        "create",
        "--root",
        project.path().to_str().expect("UTF-8 project path"),
        "--asset",
        "signal-flare",
        "--pixels",
        fixture.to_str().expect("UTF-8 fixture path"),
        "--preview-scale",
        "4",
    ]);
    assert_eq!(revision["ok"], true);
    assert_eq!(revision["revision"]["revision"], "r000001");
    assert_eq!(
        revision["revision"]["native_sha256"],
        "64c879bd5c6f41849d80d3ce0f08705d5909c0c5a8270988072f0b546bfa3bc4"
    );

    let inspect = run(&[
        "asset",
        "inspect",
        "--root",
        project.path().to_str().expect("UTF-8 project path"),
        "--asset",
        "signal-flare",
    ]);
    assert_eq!(inspect["ok"], true);
    assert_eq!(inspect["asset"]["head"], "r000001");
    assert_eq!(inspect["asset"]["kind"], "sprite");
}

#[test]
fn cli_guide_keeps_coding_agents_on_the_direct_local_workflow() {
    let project = tempdir().expect("project tempdir");
    let root = project.path().to_str().expect("UTF-8 project path");
    run(&["init", "--root", root, "--name", "Agent Guide Fixture"]);
    pixelpipe_app::open_project(pixelpipe_app::OpenProject {
        start: project.path().to_path_buf(),
    })
    .expect("starter resources");

    let guide = run(&["guide", "--root", root]);

    assert_eq!(guide["ok"], true);
    assert_eq!(guide["workflow"], "coding_agent_sprite");
    assert!(
        guide["rules"][0]
            .as_str()
            .expect("guide rule")
            .contains("does not launch or manage agents")
    );
    assert!(
        guide["rules"]
            .as_array()
            .expect("guide rules")
            .iter()
            .any(|rule| rule.as_str().is_some_and(|rule| rule.contains("rembg")))
    );
    assert!(
        guide["available_recipes"]
            .as_array()
            .expect("recipe list")
            .iter()
            .any(|recipe| recipe == "sprite-64")
    );
    assert!(
        guide["capabilities"]["replace_source"]
            .as_str()
            .expect("replace source command")
            .contains("asset update-source")
    );
    assert!(
        guide["capabilities"]["recolor"]
            .as_str()
            .expect("recolor command")
            .contains("revision recolor")
    );
    assert!(
        guide["capabilities"]["pencil_or_eraser"]
            .as_str()
            .expect("draw command")
            .contains("revision draw")
    );
    assert!(
        guide["steps"][4]["instruction"]
            .as_str()
            .expect("conversion instruction")
            .contains("32px")
    );

    let assets = run(&["asset", "list", "--root", root]);
    assert_eq!(assets["ok"], true);
    assert_eq!(assets["assets"], serde_json::json!([]));
}

#[test]
fn cli_updates_and_exports_an_existing_sprite_with_its_current_style() {
    let project = tempdir().expect("project tempdir");
    let root = project.path().to_str().expect("UTF-8 project path");
    pixelpipe_app::open_project(pixelpipe_app::OpenProject {
        start: project.path().to_path_buf(),
    })
    .expect("open project");
    run(&[
        "asset",
        "init",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--brief",
        "Top-down field medic",
    ]);
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m2");
    let first_source = project.path().join("first.png");
    let replacement = project.path().join("replacement.png");
    write_fixture_png(&fixtures.join("reference.rgba.json"), &first_source);
    write_fixture_png(&fixtures.join("sheet.rgba.json"), &replacement);
    run(&[
        "reference",
        "import",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--file",
        first_source.to_str().expect("source path"),
    ]);
    run(&[
        "revision",
        "pixelize",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--resolution",
        "32",
        "--colors",
        "8",
        "--mood",
        "vivid",
        "--background",
        "auto",
    ]);

    let updated = run(&[
        "asset",
        "update-source",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--file",
        replacement.to_str().expect("replacement path"),
    ]);
    assert_eq!(updated["update"]["revision"]["revision"], "r000002");
    let inspected = run(&["asset", "inspect", "--root", root, "--asset", "field-medic"]);
    assert_eq!(inspected["asset"]["head"], "r000002");
    assert_eq!(inspected["asset"]["style"]["recipe"], "sprite-32");
    assert_eq!(inspected["asset"]["style"]["color_count"], 8);
    assert_eq!(
        inspected["asset"]["style"]["settings"]["color_treatment"],
        "vivid"
    );

    assert_recolor_and_draw(root);

    let destination = project.path().join("exports");
    fs::create_dir(&destination).expect("export directory");
    let exported = run(&[
        "asset",
        "export",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--destination",
        destination.to_str().expect("export path"),
    ]);
    assert_eq!(exported["export"]["revision"], "r000004");
    assert!(destination.join("field-medic.png").is_file());
}

fn assert_recolor_and_draw(root: &str) {
    let recolored = run(&[
        "revision",
        "recolor",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--set",
        "1=#2455D6",
        "--actor",
        "agent",
    ]);
    assert_eq!(recolored["revision"]["revision"], "r000003");
    assert_eq!(recolored["changed"][0]["index"], 1);
    assert_eq!(
        recolored["changed"][0]["rgba"],
        serde_json::json!([36, 85, 214, 255])
    );

    let drawn = run(&[
        "revision",
        "draw",
        "--root",
        root,
        "--asset",
        "field-medic",
        "--pixel",
        "0,0=0",
        "--actor",
        "agent",
    ]);
    assert_eq!(drawn["revision"]["revision"], "r000004");
}

#[test]
fn cli_initializes_pre_revision_asset_and_project_resources() {
    let project = tempdir().expect("project tempdir");
    run(&[
        "init",
        "--root",
        project.path().to_str().expect("UTF-8 project path"),
        "--name",
        "M6 CLI Fixture",
    ]);
    let root = project.path().to_str().expect("UTF-8 project path");
    let draft = run(&[
        "asset",
        "init",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--kind",
        "sprite",
    ]);
    assert_eq!(draft["asset"]["state"], "draft");
    assert!(draft["asset"].get("head").is_none());
    let awaiting = run(&[
        "asset",
        "set-brief",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--brief",
        "Strict overhead synthetic flare",
    ]);
    assert_eq!(awaiting["asset"]["state"], "awaiting_reference");

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m6");
    let palette = fixtures.join("palette.json");
    let recipe = fixtures.join("recipe.json");
    assert_eq!(
        run(&[
            "project",
            "set-palette",
            "--root",
            root,
            "--id",
            "synthetic-flare",
            "--file",
            palette.to_str().expect("palette path"),
        ])["palette"],
        "synthetic-flare"
    );
    assert_eq!(
        run(&[
            "project",
            "set-recipe",
            "--root",
            root,
            "--file",
            recipe.to_str().expect("recipe path"),
        ])["recipe"],
        "synthetic-flare"
    );
    let show = run(&["project", "show", "--root", root]);
    assert_eq!(show["recipes"][0]["id"], "synthetic-flare");

    assert_selected_reference_preview(project.path());
}

fn assert_selected_reference_preview(project: &Path) {
    let root = project.to_str().expect("project path");
    let reference = project.join("reference.png");
    write_fixture_png(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m2/reference.rgba.json"),
        &reference,
    );
    let imported = run(&[
        "reference",
        "import",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--file",
        reference.to_str().expect("reference path"),
    ]);
    assert_eq!(imported["selection"]["asset"], "signal-flare");

    let preview = project.join("preview.png");
    let previewed = run(&[
        "revision",
        "preview-selected",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--recipe",
        "synthetic-flare",
        "--native",
        preview.to_str().expect("preview path"),
    ]);
    assert_eq!(previewed["preview"]["inspection"]["width"], 4);
    assert_eq!(
        &fs::read(&preview).expect("preview PNG")[..8],
        b"\x89PNG\r\n\x1a\n"
    );
    assert!(
        run(&[
            "asset",
            "inspect",
            "--root",
            root,
            "--asset",
            "signal-flare"
        ])["asset"]
            .get("head")
            .is_none()
    );
}

#[derive(Deserialize)]
struct RgbaFixture {
    width: u32,
    height: u32,
    pixels: Vec<Vec<[u8; 4]>>,
}

#[test]
fn cli_converts_synthetic_png_through_the_application_use_case() {
    let project = tempdir().expect("project tempdir");
    run(&[
        "init",
        "--root",
        project.path().to_str().expect("UTF-8 project path"),
        "--name",
        "Conversion Fixture",
    ]);
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/m2");
    let source_path = project.path().join("synthetic-reference.png");
    write_fixture_png(&fixtures.join("reference.rgba.json"), &source_path);

    let revision = run(&[
        "revision",
        "convert",
        "--root",
        project.path().to_str().expect("UTF-8 project path"),
        "--asset",
        "synthetic-pickup",
        "--source",
        source_path.to_str().expect("UTF-8 source path"),
        "--palette",
        fixtures
            .join("palette.json")
            .to_str()
            .expect("UTF-8 palette path"),
        "--settings",
        fixtures
            .join("reference.settings.json")
            .to_str()
            .expect("UTF-8 settings path"),
        "--preview-scale",
        "8",
    ]);

    assert_eq!(revision["ok"], true);
    assert_eq!(revision["revision"]["revision"], "r000001");
    assert_eq!(
        revision["revision"]["native_sha256"],
        "9e1345c3b488327bb6839c177830c0f50f5121b21450de9eda42e1d923e4721e"
    );
    assert_eq!(
        revision["revision"]["preview_sha256"],
        "22d22ac34a6764531e972636f4c0d10e17c01e9752ad01b3b2e9c4a44305f201"
    );

    let revision_path = Path::new(
        revision["revision"]["revision_path"]
            .as_str()
            .expect("revision path"),
    );
    let recipe: Value =
        serde_json::from_slice(&fs::read(revision_path.join("recipe.json")).expect("read recipe"))
            .expect("recipe JSON");
    assert_eq!(recipe["operations"][0]["type"], "convert_reference");
    assert_eq!(recipe["operations"][1]["type"], "render_indexed");
    let stored_reference = project
        .path()
        .join(".pixelpipe/assets/synthetic-pickup/references/selected")
        .join(format!(
            "{}.png",
            recipe["input_sha256"].as_str().expect("reference hash")
        ));
    assert_eq!(
        fs::read(stored_reference).expect("stored selected reference"),
        fs::read(&source_path).expect("source reference")
    );
    let validation: Value = serde_json::from_slice(
        &fs::read(revision_path.join("validation.json")).expect("read validation"),
    )
    .expect("validation JSON");
    assert!(
        validation["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .any(|check| check["name"] == "connected_components" && check["detail"] == "1")
    );
}

#[test]
fn cli_refines_branches_compares_and_records_review_without_mutating_parents() {
    let project = tempdir().expect("project tempdir");
    let root = project.path().to_str().expect("UTF-8 project path");
    run(&["init", "--root", root, "--name", "M3 Fixture"]);
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raster = workspace.join("fixtures/m1/tiny-raster.json");
    let fixtures = workspace.join("fixtures/m3");
    let created = run(&[
        "revision",
        "create",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--pixels",
        raster.to_str().expect("raster path"),
    ]);
    assert_eq!(created["revision"]["revision"], "r000001");
    let first_path = PathBuf::from(created["revision"]["revision_path"].as_str().expect("path"));
    let first_payloads = revision_payloads(&first_path);

    let patched = run(&[
        "revision",
        "patch",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--parent",
        "r000001",
        "--patch",
        fixtures
            .join("pixel-patch.json")
            .to_str()
            .expect("patch path"),
    ]);
    assert_eq!(patched["revision"]["revision"], "r000002");
    assert_eq!(patched["revision"]["parent"], "r000001");

    let remapped = run(&[
        "revision",
        "remap",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--parent",
        "r000001",
        "--remap",
        fixtures
            .join("palette-remap.json")
            .to_str()
            .expect("remap path"),
    ]);
    assert_eq!(remapped["revision"]["revision"], "r000003");
    assert_eq!(remapped["revision"]["parent"], "r000001");
    assert_eq!(revision_payloads(&first_path), first_payloads);

    assert_m3_inspection_comparison_review(&project, root, &first_path, &first_payloads);
    assert_invalid_patch_is_atomic(&project, root);
}

fn assert_m3_inspection_comparison_review(
    project: &tempfile::TempDir,
    root: &str,
    first_path: &Path,
    first_payloads: &[(String, Vec<u8>)],
) {
    let inspection = run(&[
        "revision",
        "inspect",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--revision",
        "r000002",
    ]);
    assert_eq!(inspection["revision"]["review"], Value::Null);
    assert_eq!(
        inspection["revision"]["inspection"]["text_rows"][3],
        "01 01 01 --"
    );

    let visual_native = project.path().join("diff.png");
    let visual_preview = project.path().join("diff-preview.png");
    let comparison = run(&[
        "revision",
        "compare",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--left",
        "r000001",
        "--right",
        "r000002",
        "--visual-native",
        visual_native.to_str().expect("visual path"),
        "--visual-preview",
        visual_preview.to_str().expect("preview path"),
    ]);
    assert_eq!(
        comparison["comparison"]["diff"]["changed_pixels"]
            .as_array()
            .expect("pixels")
            .len(),
        2
    );
    assert_eq!(
        comparison["comparison"]["visual_native_sha256"],
        "3de8b85fe254add0ac7525bc72732d5692dfaba83f81be3a94b1284ece90bb13"
    );
    assert_eq!(
        comparison["comparison"]["visual_preview_sha256"],
        "f77d9c7a01bd0f341fd56913a598412915cf1022e33aa0613e8f72f13a106302"
    );
    assert!(visual_native.is_file());
    assert!(visual_preview.is_file());

    let review = run(&[
        "revision",
        "review",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--revision",
        "r000002",
        "--decision",
        "changes-requested",
        "--actor-kind",
        "agent",
        "--actor",
        "fixture-agent",
        "--note",
        "native silhouette needs another pass",
    ]);
    assert_eq!(
        review["review"]["events"][0]["decision"],
        "changes_requested"
    );
    let inspected_review = run(&[
        "revision",
        "inspect",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--revision",
        "r000002",
    ]);
    assert_eq!(
        inspected_review["revision"]["review"]["events"][0]["actor_kind"],
        "agent"
    );
    assert_eq!(revision_payloads(first_path), first_payloads);
}

fn assert_invalid_patch_is_atomic(project: &tempfile::TempDir, root: &str) {
    let invalid_patch = project.path().join("invalid-patch.json");
    fs::write(
        &invalid_patch,
        r#"{"schema":"pixelpipe.patch/v1","edits":[{"x":99,"y":0,"index":1}]}"#,
    )
    .expect("invalid patch");
    run_failure(&[
        "revision",
        "patch",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--parent",
        "r000002",
        "--patch",
        invalid_patch.to_str().expect("invalid patch path"),
    ]);
    let asset = run(&[
        "asset",
        "inspect",
        "--root",
        root,
        "--asset",
        "signal-flare",
    ]);
    assert_eq!(asset["asset"]["head"], "r000003");
    assert!(
        !project
            .path()
            .join(".pixelpipe/assets/signal-flare/revisions/r000004")
            .exists()
    );
}

fn write_fixture_png(fixture_path: &Path, output_path: &Path) {
    let fixture: RgbaFixture =
        serde_json::from_slice(&fs::read(fixture_path).expect("read synthetic RGBA fixture"))
            .expect("RGBA fixture JSON");
    let pixels: Vec<u8> = fixture.pixels.into_iter().flatten().flatten().collect();
    let file = fs::File::create(output_path).expect("create PNG fixture");
    let mut encoder = png::Encoder::new(file, fixture.width, fixture.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write PNG header");
    writer.write_image_data(&pixels).expect("write PNG data");
    writer.finish().expect("finish PNG fixture");
}

fn run(arguments: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_pixelpipe"))
        .args(arguments)
        .output()
        .expect("run pixelpipe");
    assert!(
        output.status.success(),
        "pixelpipe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI stdout JSON")
}

fn run_failure(arguments: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_pixelpipe"))
        .args(arguments)
        .output()
        .expect("run pixelpipe");
    assert!(!output.status.success(), "pixelpipe unexpectedly succeeded");
    serde_json::from_slice(&output.stderr).expect("CLI stderr JSON")
}

fn revision_payloads(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut payloads = fs::read_dir(path)
        .expect("revision directory")
        .map(|entry| {
            let entry = entry.expect("revision entry");
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).expect("revision payload"),
            )
        })
        .collect::<Vec<_>>();
    payloads.sort_by(|left, right| left.0.cmp(&right.0));
    payloads
}
