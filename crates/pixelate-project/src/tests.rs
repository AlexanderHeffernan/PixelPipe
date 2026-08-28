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
fn opens_projects_with_removed_manifest_fields() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    fs::write(
        temp.path().join(".pixelate/project.toml"),
        "schema = \"pixelate.project/v1\"\nname = \"Fixture Game\"\npreview_scale = 8\n",
    )
    .expect("legacy manifest");

    let manifest = store.manifest().expect("manifest");
    assert_eq!(manifest.name, "Fixture Game");
    assert!(manifest.ignored_project_images.is_empty());

    store
        .create_asset("legacy-asset", "Legacy asset")
        .expect("asset");
    fs::write(
        temp.path().join(".pixelate/assets/legacy-asset/asset.toml"),
        concat!(
            "schema = \"pixelate.asset/v2\"\n",
            "id = \"legacy-asset\"\n",
            "kind = \"sprite\"\n",
            "state = \"selected_reference\"\n",
            "approved = \"r000001\"\n",
            "[brief]\n",
            "schema = \"pixelate.asset-brief/v1\"\n",
            "text = \"Legacy asset\"\n",
            "[selected_reference]\n",
            "schema = \"pixelate.reference-selection/v1\"\n",
            "asset = \"legacy-asset\"\n",
            "run = \"import\"\n",
            "candidate = \"local-file\"\n",
            "sha256 = \"legacy-hash\"\n",
            "selected_unix_ms = 1\n",
        ),
    )
    .expect("legacy asset manifest");

    let asset = store.asset("legacy-asset").expect("legacy asset");
    assert_eq!(asset.id, "legacy-asset");
    assert_eq!(asset.project_path, None);
    assert_eq!(
        asset.selected_reference.expect("selection").sha256,
        "legacy-hash"
    );
}

#[test]
fn hidden_project_images_are_persistent_and_reversible() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    fs::create_dir(temp.path().join("art")).expect("folder");
    fs::write(temp.path().join("art/concept.png"), b"image").expect("image");

    let hidden = store.ignore_project_image("art/concept.png").expect("hide");
    assert_eq!(hidden.ignored_project_images, ["art/concept.png"]);
    assert_eq!(
        ProjectStore::discover(temp.path())
            .expect("discover")
            .manifest()
            .expect("manifest")
            .ignored_project_images,
        ["art/concept.png"]
    );
    assert!(matches!(
        store.ignore_project_image("../outside.png"),
        Err(ProjectError::InvalidProjectPath(_))
    ));
    assert!(
        store
            .unignore_project_image("art/concept.png")
            .expect("restore")
            .ignored_project_images
            .is_empty()
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

#[test]
fn discovers_supported_images_with_ignore_and_internal_exclusions() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    fs::create_dir_all(temp.path().join("art/units")).expect("art folder");
    fs::create_dir_all(temp.path().join("node_modules/package")).expect("dependency folder");
    fs::create_dir_all(temp.path().join("ignored")).expect("ignored folder");
    fs::write(temp.path().join(".gitignore"), "ignored/\n").expect("ignore file");
    fs::write(temp.path().join("art/units/hero.PNG"), b"image").expect("hero");
    fs::write(temp.path().join("art/readme.txt"), b"not artwork").expect("text");
    fs::write(
        temp.path().join("node_modules/package/icon.png"),
        b"dependency",
    )
    .expect("dep");
    fs::write(temp.path().join("ignored/concept.webp"), b"ignored").expect("ignored");
    fs::write(temp.path().join(".pixelate/internal.png"), b"internal").expect("internal");

    assert_eq!(
        store.project_images().expect("catalog"),
        vec![crate::ProjectImage {
            path: "art/units/hero.PNG".to_owned()
        }]
    );
}

#[test]
fn links_and_moves_assets_without_changing_identity_or_history_path() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    fs::create_dir_all(temp.path().join("art/units")).expect("folders");
    fs::write(temp.path().join("art/hero.png"), b"original").expect("image");
    store.create_asset("hero", "Hero").expect("asset");
    let revision_root = temp.path().join(".pixelate/assets/hero/revisions");

    let linked = store
        .link_asset_project_path("hero", "art/hero.png")
        .expect("link");
    assert_eq!(linked.project_path.as_deref(), Some("art/hero.png"));
    assert_eq!(
        linked.project_file_sha256.as_deref(),
        Some(sha256_hex(b"original").as_str())
    );

    let moved = store
        .move_asset_file("hero", "art/units/hero.png")
        .expect("move");
    assert_eq!(moved.id, "hero");
    assert_eq!(moved.project_path.as_deref(), Some("art/units/hero.png"));
    assert_eq!(
        revision_root,
        temp.path().join(".pixelate/assets/hero/revisions")
    );
    assert!(temp.path().join("art/units/hero.png").is_file());
}

#[test]
fn moves_unmanaged_project_images_safely_and_refuses_collisions() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    fs::create_dir_all(temp.path().join("art/units")).expect("folders");
    fs::write(temp.path().join("concept.png"), b"concept").expect("image");

    store
        .move_project_image("concept.png", "art/units/concept.png")
        .expect("move image");
    assert!(!temp.path().join("concept.png").exists());
    assert_eq!(
        fs::read(temp.path().join("art/units/concept.png")).expect("moved image"),
        b"concept"
    );

    fs::write(temp.path().join("other.png"), b"other").expect("other");
    assert!(matches!(
        store.move_project_image("other.png", "art/units/concept.png"),
        Err(ProjectError::ProjectPathExists(_))
    ));
    assert!(matches!(
        store.move_project_image("other.png", "../outside.png"),
        Err(ProjectError::InvalidProjectPath(_))
    ));
}

#[test]
fn folder_moves_update_all_linked_manifests_and_refuse_collisions() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    fs::create_dir_all(temp.path().join("art/units")).expect("folders");
    fs::write(temp.path().join("art/units/a.png"), b"a").expect("a");
    fs::write(temp.path().join("art/units/b.webp"), b"b").expect("b");
    for (id, path) in [("a", "art/units/a.png"), ("b", "art/units/b.webp")] {
        store.create_asset(id, id).expect("asset");
        store.link_asset_project_path(id, path).expect("link");
    }
    store
        .ignore_project_image("art/units/a.png")
        .expect("hide source");

    store
        .move_project_folder("art/units", "art/characters")
        .expect("move folder");
    assert_eq!(
        store.asset("a").expect("a").project_path.as_deref(),
        Some("art/characters/a.png")
    );
    assert_eq!(
        store.asset("b").expect("b").project_path.as_deref(),
        Some("art/characters/b.webp")
    );
    assert_eq!(
        store.manifest().expect("project").ignored_project_images,
        ["art/characters/a.png"]
    );
    fs::create_dir(temp.path().join("art/taken")).expect("taken");
    assert!(matches!(
        store.move_project_folder("art/characters", "art/taken"),
        Err(ProjectError::ProjectPathExists(_))
    ));
}

#[test]
fn folder_operations_reject_internal_paths_and_non_empty_deletion() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    assert!(matches!(
        store.create_project_folder(".git/art"),
        Err(ProjectError::ReservedProjectPath(_))
    ));
    store
        .create_project_folder("sprites/units")
        .expect("create nested");
    assert!(temp.path().join("sprites/units").is_dir());
    store.create_project_folder("art").expect("create");
    fs::write(temp.path().join("art/file.txt"), b"occupied").expect("file");
    assert!(matches!(
        store.delete_project_folder("art"),
        Err(ProjectError::FolderNotEmpty(_))
    ));
    fs::remove_file(temp.path().join("art/file.txt")).expect("remove file");
    store.delete_project_folder("art").expect("delete empty");
}

#[test]
fn discovers_empty_asset_folders_and_safely_deletes_only_images() {
    let temp = tempdir().expect("tempdir");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    store
        .create_project_folder("sprites/units")
        .expect("create folder");
    assert_eq!(
        store.project_folders().expect("folders"),
        [crate::ProjectFolder {
            path: "sprites/units".to_owned()
        }]
    );

    fs::write(temp.path().join("sprites/hero.png"), b"image").expect("image");
    store
        .delete_project_image("sprites/hero.png")
        .expect("delete image");
    assert!(!temp.path().join("sprites/hero.png").exists());
    assert!(matches!(
        store.delete_project_image(".pixelate/internal.png"),
        Err(ProjectError::ReservedProjectPath(_))
    ));
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escape_targets() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().expect("tempdir");
    let outside = tempdir().expect("outside");
    let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
    symlink(outside.path(), temp.path().join("outside")).expect("symlink");
    fs::write(outside.path().join("sprite.png"), b"image").expect("outside image");
    assert!(matches!(
        store.create_project_folder("outside/new-folder"),
        Err(ProjectError::SymlinkEscape(_))
    ));
}
