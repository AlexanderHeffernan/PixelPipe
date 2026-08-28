use std::{fs, path::Path};

use pixelate_app::{
    AdoptPixelArt, AdoptProjectImage, BrowseProject, DeleteProjectImage, MoveAsset,
    MoveProjectImage, ProjectFileStatus, SetProjectImageIgnored, UpdateLinkedSource,
    adopt_pixel_art, adopt_project_image, browse_project, delete_project_image, move_asset,
    move_project_image, set_project_image_ignored, update_linked_source,
};
use pixelate_project::ProjectStore;

#[test]
fn catalog_deduplicates_adopted_images_and_reports_external_state() {
    let game = tempfile::tempdir().expect("game");
    let store = ProjectStore::init(game.path(), "Fixture").expect("project");
    fs::create_dir(game.path().join("art")).expect("folder");
    let file = game.path().join("art/hero.png");
    write_png(&file, [10, 20, 30, 255]);

    let initial = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(initial.catalog.len(), 1);
    assert_eq!(initial.catalog[0].asset_id, None);

    adopt_pixel_art(AdoptPixelArt {
        start: game.path().to_path_buf(),
        path: "art/hero.png".to_owned(),
        asset: "hero".to_owned(),
        brief: "Hero".to_owned(),
        actor: "test".to_owned(),
    })
    .expect("adopt");
    let adopted = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(adopted.catalog.len(), 1);
    assert_eq!(adopted.catalog[0].asset_id.as_deref(), Some("hero"));
    assert_eq!(format!("{:?}", adopted.catalog[0].status), "Current");

    write_png(&file, [200, 20, 30, 255]);
    let modified = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(format!("{:?}", modified.catalog[0].status), "Modified");
    update_linked_source(UpdateLinkedSource {
        start: game.path().to_path_buf(),
        asset: "hero".to_owned(),
    })
    .expect("update");
    let current = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(format!("{:?}", current.catalog[0].status), "Current");

    fs::remove_file(file).expect("remove");
    let missing = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(format!("{:?}", missing.catalog[0].status), "Missing");
    assert_eq!(store.asset("hero").expect("asset").id, "hero");
}

#[test]
fn reference_adoption_hides_source_and_plans_a_distinct_output() {
    let game = tempfile::tempdir().expect("game");
    ProjectStore::init(game.path(), "Fixture").expect("project");
    fs::create_dir(game.path().join("art")).expect("folder");
    write_png(&game.path().join("art/concept.png"), [40, 80, 120, 255]);

    let asset = adopt_project_image(AdoptProjectImage {
        start: game.path().to_path_buf(),
        path: "art/concept.png".to_owned(),
        asset: "hero".to_owned(),
        brief: "Hero".to_owned(),
        destination: "art/hero.png".to_owned(),
    })
    .expect("adopt reference");
    assert_eq!(asset.project_path.as_deref(), Some("art/hero.png"));
    assert!(asset.selected_reference.is_some());
    assert!(game.path().join("art/concept.png").is_file());
    assert!(!game.path().join("art/hero.png").exists());

    let catalog = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(catalog.catalog.len(), 1);
    assert_eq!(catalog.catalog[0].path, "art/hero.png");
    assert_eq!(catalog.catalog[0].status, ProjectFileStatus::Unexported);

    set_project_image_ignored(SetProjectImageIgnored {
        start: game.path().to_path_buf(),
        path: "art/concept.png".to_owned(),
        ignored: false,
    })
    .expect("restore");
    let restored = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(restored.catalog.len(), 2);
    assert!(
        restored
            .catalog
            .iter()
            .any(|entry| entry.path == "art/concept.png")
    );
}

#[test]
fn deleting_a_linked_project_image_retains_asset_history_and_reports_missing() {
    let game = tempfile::tempdir().expect("game");
    let store = ProjectStore::init(game.path(), "Fixture").expect("project");
    write_png(&game.path().join("hero.png"), [40, 80, 120, 255]);
    adopt_pixel_art(AdoptPixelArt {
        start: game.path().to_path_buf(),
        path: "hero.png".to_owned(),
        asset: "hero".to_owned(),
        brief: "Hero".to_owned(),
        actor: "test".to_owned(),
    })
    .expect("adopt");

    delete_project_image(DeleteProjectImage {
        start: game.path().to_path_buf(),
        path: "hero.png".to_owned(),
    })
    .expect("delete project image");
    assert!(store.asset("hero").expect("asset retained").head.is_some());
    let browser = browse_project(&BrowseProject {
        start: game.path().to_path_buf(),
    })
    .expect("browse");
    assert_eq!(browser.catalog[0].status, ProjectFileStatus::Missing);
}

#[test]
fn moving_a_legacy_pathless_asset_plans_its_real_folder_location() {
    let game = tempfile::tempdir().expect("game");
    let store = ProjectStore::init(game.path(), "Fixture").expect("project");
    fs::create_dir(game.path().join("sprites")).expect("folder");
    store.create_asset("hero", "Hero").expect("asset");

    let moved = move_asset(MoveAsset {
        start: game.path().to_path_buf(),
        asset: "hero".to_owned(),
        destination: "sprites/hero.png".to_owned(),
    })
    .expect("move");
    assert_eq!(moved.project_path.as_deref(), Some("sprites/hero.png"));
    assert!(!game.path().join("sprites/hero.png").exists());
}

#[test]
fn moving_an_unmanaged_image_refuses_managed_sources() {
    let game = tempfile::tempdir().expect("game");
    ProjectStore::init(game.path(), "Fixture").expect("project");
    fs::create_dir(game.path().join("sprites")).expect("folder");
    write_png(&game.path().join("concept.png"), [40, 80, 120, 255]);

    move_project_image(MoveProjectImage {
        start: game.path().to_path_buf(),
        source: "concept.png".to_owned(),
        destination: "sprites/concept.png".to_owned(),
    })
    .expect("move unmanaged");
    assert!(game.path().join("sprites/concept.png").is_file());

    adopt_pixel_art(AdoptPixelArt {
        start: game.path().to_path_buf(),
        path: "sprites/concept.png".to_owned(),
        asset: "concept".to_owned(),
        brief: "Concept".to_owned(),
        actor: "test".to_owned(),
    })
    .expect("adopt");
    assert!(
        move_project_image(MoveProjectImage {
            start: game.path().to_path_buf(),
            source: "sprites/concept.png".to_owned(),
            destination: "concept.png".to_owned(),
        })
        .is_err()
    );
}

fn write_png(path: &Path, color: [u8; 4]) {
    let file = fs::File::create(path).expect("png");
    let mut encoder = png::Encoder::new(file, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("header")
        .write_image_data(&color.repeat(4))
        .expect("pixels");
}
