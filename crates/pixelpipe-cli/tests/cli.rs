use std::{path::Path, process::Command};

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
