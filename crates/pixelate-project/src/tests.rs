use std::fs;

use pixelate_core::sha256_hex;
use tempfile::tempdir;

use crate::{ProjectError, ProjectStore, assets::validate_asset_id};

#[test]
fn init_and_discover_project() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    let nested = temp.path().join("src/deep");
    fs::create_dir_all(&nested).expect("nested directory");

    let discovered = ProjectStore::discover(&nested).expect("discover");
    assert_eq!(discovered.root(), store.root());
    assert_eq!(
        discovered.manifest().expect("manifest").name,
        "Fixture Game"
    );
}

#[test]
fn rejects_path_like_asset_ids() {
    assert!(matches!(
        validate_asset_id("../escape"),
        Err(ProjectError::InvalidAssetId(_))
    ));
}

#[test]
fn deletes_only_the_requested_asset() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    store.create_asset("first", "first").expect("first asset");
    store
        .create_asset("second", "second")
        .expect("second asset");

    store.delete_asset("first").expect("delete first");

    assert!(
        store
            .optional_asset("first")
            .expect("first lookup")
            .is_none()
    );
    assert!(
        store
            .optional_asset("second")
            .expect("second lookup")
            .is_some()
    );
}

#[test]
fn imports_references_by_content_hash_without_overwriting() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    let bytes = b"synthetic PNG fixture bytes";
    let first = store
        .import_reference("test-sprite", bytes)
        .expect("import");
    let second = store
        .import_reference("test-sprite", bytes)
        .expect("repeat import");

    assert_eq!(first, second);
    assert_eq!(fs::read(&first.path).expect("stored reference"), bytes);
    assert_eq!(first.sha256, sha256_hex(bytes));
}
