use std::{fs, path::Path, process::Command};

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
