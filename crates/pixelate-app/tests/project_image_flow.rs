use std::fs;

use pixelate_app::{LoadProjectImage, load_project_image};
use pixelate_project::ProjectStore;

#[test]
fn selected_image_inspection_uses_exact_import_rules_and_safe_discovery() {
    let game = tempfile::tempdir().expect("game");
    ProjectStore::init(game.path(), "Fixture").expect("project");
    write_png(
        &game.path().join("sprite.png"),
        16,
        16,
        &(0_u8..=255)
            .flat_map(|index| [index.min(254), 0, 0, 255])
            .collect::<Vec<_>>(),
    );

    let view = load_project_image(LoadProjectImage {
        start: game.path().to_path_buf(),
        path: "sprite.png".to_owned(),
    })
    .expect("inspect");
    assert_eq!((view.width, view.height), (16, 16));
    assert!(view.pixel_art_importable);
    assert_eq!(view.path, game.path().join("sprite.png"));

    assert!(
        load_project_image(LoadProjectImage {
            start: game.path().to_path_buf(),
            path: ".pixelate/project.toml".to_owned(),
        })
        .is_err()
    );
}

#[test]
fn exact_import_is_quietly_unavailable_for_large_or_over_palette_images() {
    let game = tempfile::tempdir().expect("game");
    ProjectStore::init(game.path(), "Fixture").expect("project");
    write_png(
        &game.path().join("large.png"),
        257,
        1,
        &[20, 40, 60, 255].repeat(257),
    );
    let many_colors = (0_u16..512)
        .flat_map(|index| {
            let index = index.min(256);
            let [low, high] = index.to_le_bytes();
            [low, high, 0, 255]
        })
        .collect::<Vec<_>>();
    write_png(&game.path().join("many-colors.png"), 256, 2, &many_colors);

    for path in ["large.png", "many-colors.png"] {
        let view = load_project_image(LoadProjectImage {
            start: game.path().to_path_buf(),
            path: path.to_owned(),
        })
        .expect("inspect");
        assert!(!view.pixel_art_importable, "{path}");
    }
}

fn write_png(path: &std::path::Path, width: u32, height: u32, pixels: &[u8]) {
    let file = fs::File::create(path).expect("png");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("header")
        .write_image_data(pixels)
        .expect("pixels");
}
