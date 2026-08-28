use std::{fs, path::Path};

use pixelate_app::{
    AdoptPixelArt, AdoptProjectImage, BrowseProject, ProjectFileStatus, SetProjectImageIgnored,
    UpdateLinkedSource, adopt_pixel_art, adopt_project_image, browse_project,
    set_project_image_ignored, update_linked_source,
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
