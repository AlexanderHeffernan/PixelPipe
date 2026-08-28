use std::{fs, path::Path};

use pixelate_app::{
    AdoptProjectImage, BrowseProject, UpdateLinkedSource, adopt_project_image, browse_project,
    update_linked_source,
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

    adopt_project_image(AdoptProjectImage {
        start: game.path().to_path_buf(),
        path: "art/hero.png".to_owned(),
        asset: "hero".to_owned(),
        brief: "Hero".to_owned(),
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
