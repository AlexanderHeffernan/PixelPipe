use std::{fs, path::Path, process::Command};

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct RgbaFixture {
    width: u32,
    height: u32,
    pixels: Vec<Vec<[u8; 4]>>,
}

#[test]
fn agent_workflow_pixelizes_previews_and_exports() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path().to_str().expect("project path");
    run(&["init", "--root", root, "--name", "CLI Fixture"]);
    run(&[
        "asset",
        "init",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--brief",
        "Strict overhead signal flare",
    ]);
    let source = project.path().join("source.png");
    write_fixture_png(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../pixelate-core/tests/fixtures/m2/reference.rgba.json"),
        &source,
    );
    run(&[
        "reference",
        "import",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--file",
        source.to_str().expect("source path"),
    ]);
    let converted = run(&[
        "revision",
        "pixelize",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--resolution",
        "32",
        "--colors",
        "12",
        "--background",
        "auto",
    ]);
    assert_eq!(converted["revision"]["revision"], "r000001");
    let revision_path = Path::new(
        converted["revision"]["revision_path"]
            .as_str()
            .expect("revision path"),
    );
    assert!(!revision_path.join("preview.png").exists());
    make_revision_legacy(revision_path);

    let preview_path = project.path().join("preview.png");
    let preview = run(&[
        "revision",
        "preview",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--output",
        preview_path.to_str().expect("preview path"),
    ]);
    assert_eq!(preview["preview"]["revision"], "r000001");
    assert!(preview["preview"]["scale"].as_u64().expect("scale") > 1);
    assert_eq!(
        &fs::read(&preview_path).expect("preview")[..8],
        b"\x89PNG\r\n\x1a\n"
    );

    let export_path = project.path().join("Signal Flare.webp");
    let exported = run(&[
        "asset",
        "export-file",
        "--root",
        root,
        "--asset",
        "signal-flare",
        "--destination",
        export_path.to_str().expect("export path"),
    ]);
    assert_eq!(exported["export"]["format"], "webp");
    assert!(export_path.is_file());
}

#[test]
fn guide_documents_every_agent_workflow_family() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path().to_str().expect("project path");
    run(&["init", "--root", root, "--name", "Guide Fixture"]);
    let guide = run(&["guide", "--root", root]);
    assert!(guide.get("available_recipes").is_none());
    for capability in [
        "version",
        "update_cli",
        "list",
        "show_project",
        "update_brief",
        "rename",
        "delete",
        "replace_source",
        "pixelize",
        "canvas_placement",
        "inspect_colours",
        "visual_preview",
        "recolor",
        "pencil_or_eraser",
        "fill",
        "undo_redo",
        "export_bundle",
        "export_image",
    ] {
        assert!(
            guide["capabilities"].get(capability).is_some(),
            "missing {capability}"
        );
    }
    assert!(
        guide["rules"][0]
            .as_str()
            .expect("rule")
            .contains("does not launch")
    );
}

#[test]
fn version_is_available_without_update_notifications() {
    let version = run(&["version"]);
    assert_eq!(version["version"], env!("CARGO_PKG_VERSION"));
    assert!(version.get("update_available").is_none());
}

fn write_fixture_png(fixture_path: &Path, output_path: &Path) {
    let fixture: RgbaFixture =
        serde_json::from_slice(&fs::read(fixture_path).expect("fixture")).expect("fixture JSON");
    let pixels: Vec<u8> = fixture.pixels.into_iter().flatten().flatten().collect();
    let file = fs::File::create(output_path).expect("PNG fixture");
    let mut encoder = png::Encoder::new(file, fixture.width, fixture.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("PNG header")
        .write_image_data(&pixels)
        .expect("PNG pixels");
}

fn make_revision_legacy(path: &Path) {
    let mut recipe: Value =
        serde_json::from_slice(&fs::read(path.join("recipe.json")).expect("recipe"))
            .expect("recipe JSON");
    recipe["operations"]
        .as_array_mut()
        .expect("operations")
        .push(serde_json::json!({ "type": "render_indexed", "preview_scale": 8 }));
    let recipe_hash = write_json(&path.join("recipe.json"), &recipe);

    let mut validation: Value =
        serde_json::from_slice(&fs::read(path.join("validation.json")).expect("validation"))
            .expect("validation JSON");
    validation["visual_review"] = Value::String("required".to_owned());
    let validation_hash = write_json(&path.join("validation.json"), &validation);

    let preview = fs::read(path.join("native.png")).expect("native PNG");
    fs::write(path.join("preview.png"), &preview).expect("legacy preview");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(path.join("revision.json")).expect("manifest"))
            .expect("manifest JSON");
    let files = manifest["files"].as_object_mut().expect("manifest files");
    files.insert("recipe.json".to_owned(), Value::String(recipe_hash));
    files.insert("validation.json".to_owned(), Value::String(validation_hash));
    files.insert(
        "preview.png".to_owned(),
        Value::String(pixelate_core::sha256_hex(&preview)),
    );
    write_json(&path.join("revision.json"), &manifest);
}

fn write_json(path: &Path, value: &Value) -> String {
    let mut bytes = serde_json::to_vec_pretty(value).expect("JSON");
    bytes.push(b'\n');
    fs::write(path, &bytes).expect("write JSON");
    pixelate_core::sha256_hex(&bytes)
}

fn run(arguments: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_pixelate"))
        .args(arguments)
        .output()
        .expect("run pixelate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI JSON")
}
