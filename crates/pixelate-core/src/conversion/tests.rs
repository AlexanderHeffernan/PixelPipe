use png::ColorType;
use serde::Deserialize;

use super::{
    backdrop::visible_bounds,
    convert_reference, decode_rgba_png, detect_border_color,
    model::{
        BackdropPolicy, Bounds, ComponentExpectation, ConversionSettings, Registration, RgbaImage,
    },
};
use crate::{
    ColorAdjustments, ColorTreatment, IndexedRaster, PALETTE_SCHEMA, Palette, render, sha256_hex,
    stable_json,
};

#[derive(Deserialize)]
struct RgbaFixture {
    width: u32,
    height: u32,
    pixels: Vec<Vec<[u8; 4]>>,
}

fn palette() -> Palette {
    Palette {
        schema: PALETTE_SCHEMA.to_owned(),
        name: "synthetic".to_owned(),
        transparent_index: 0,
        colors: vec![
            [0, 0, 0, 0],
            [24, 24, 28, 255],
            [220, 60, 40, 255],
            [248, 220, 96, 255],
            [240, 240, 240, 255],
        ],
    }
}

fn settings(registration: Registration) -> ConversionSettings {
    ConversionSettings {
        width: 8,
        height: 8,
        color_treatment: ColorTreatment::Original,
        color_adjustments: ColorAdjustments::default(),
        margin: 1,
        subject_scale_percent: 100,
        offset_x: 0,
        offset_y: 0,
        coverage_percent: 25,
        backdrop: BackdropPolicy::Alpha { alpha_threshold: 0 },
        registration,
        components: ComponentExpectation { min: 1, max: 1 },
    }
}

#[test]
fn decodes_rgba_png() {
    let pixels = [255, 0, 0, 255, 0, 255, 0, 128];
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(&pixels).expect("PNG pixels");
        writer.finish().expect("PNG finish");
    }
    let decoded = decode_rgba_png(&bytes).expect("decode PNG");
    assert_eq!(decoded.width, 2);
    assert_eq!(decoded.pixels, vec![[255, 0, 0, 255], [0, 255, 0, 128]]);
}

#[test]
fn border_cleanup_preserves_enclosed_dark_subject_pixels() {
    let mut pixels = vec![[0, 0, 0, 255]; 25];
    for y in 1..=3 {
        for x in 1..=3 {
            pixels[y * 5 + x] = [240, 240, 240, 255];
        }
    }
    pixels[2 * 5 + 2] = [20, 20, 20, 255];
    let source = RgbaImage {
        width: 5,
        height: 5,
        pixels,
    };
    let mut settings = settings(Registration::Center);
    settings.backdrop = BackdropPolicy::BorderConnected {
        color: [0, 0, 0],
        tolerance: 4,
        alpha_threshold: 0,
    };
    let converted = convert_reference(&source, &palette(), &settings).expect("convert");

    assert!(converted.raster.pixels.contains(&1));
    assert!(converted.raster.pixels.contains(&4));
}

#[test]
fn detects_a_softly_varied_dominant_border_colour() {
    let source = RgbaImage {
        width: 3,
        height: 3,
        pixels: vec![
            [101, 149, 199, 255],
            [102, 150, 200, 255],
            [100, 151, 201, 255],
            [99, 150, 200, 255],
            [220, 30, 40, 255],
            [103, 148, 202, 255],
            [100, 150, 198, 255],
            [101, 152, 200, 255],
            [102, 149, 201, 255],
        ],
    };

    assert_eq!(
        detect_border_color(&source, 0, 8).unwrap(),
        Some([101, 150, 200])
    );
}

#[test]
fn does_not_treat_an_edge_to_edge_tile_as_background() {
    let source = RgbaImage {
        width: 8,
        height: 8,
        pixels: vec![[36, 142, 58, 255]; 64],
    };

    assert_eq!(detect_border_color(&source, 0, 28).unwrap(), None);
}

#[test]
fn bottom_registration_uses_shared_baseline() {
    let source = RgbaImage {
        width: 2,
        height: 3,
        pixels: vec![[220, 60, 40, 255]; 6],
    };
    let converted =
        convert_reference(&source, &palette(), &settings(Registration::Bottom)).expect("convert");
    let bounds = visible_bounds_from_raster(&converted.raster);
    assert_eq!(bounds.y + bounds.height, 7);
    assert_eq!(converted.raster.pivot, Some([4, 7]));
}

#[test]
fn subject_scale_and_offsets_allow_intentional_canvas_clipping() {
    let source = RgbaImage {
        width: 2,
        height: 2,
        pixels: vec![[220, 60, 40, 255]; 4],
    };
    let mut settings = settings(Registration::Center);
    settings.subject_scale_percent = 50;
    settings.offset_x = 4;
    settings.offset_y = 3;

    let converted = convert_reference(&source, &palette(), &settings).expect("convert");

    let bounds = visible_bounds_from_raster(&converted.raster);
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (6, 0, 2, 2)
    );
    assert_eq!(converted.raster.metadata["placements"], "6,0,2,2");
}

#[test]
fn subject_scale_can_crop_beyond_the_fitted_canvas() {
    let source = RgbaImage {
        width: 2,
        height: 2,
        pixels: vec![[220, 60, 40, 255]; 4],
    };
    let mut settings = settings(Registration::Center);
    settings.subject_scale_percent = 150;

    let converted = convert_reference(&source, &palette(), &settings).expect("convert");
    let bounds = visible_bounds_from_raster(&converted.raster);

    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (0, 0, 8, 8)
    );
    assert_eq!(converted.raster.metadata["placements"], "0,0,8,8");
}

#[test]
fn dominant_ties_choose_lower_palette_index() {
    let source = RgbaImage {
        width: 2,
        height: 1,
        pixels: vec![[220, 60, 40, 255], [248, 220, 96, 255]],
    };
    let mut settings = settings(Registration::Center);
    settings.width = 3;
    settings.height = 3;
    settings.margin = 1;
    let converted = convert_reference(&source, &palette(), &settings).expect("convert");
    assert!(converted.raster.pixels.contains(&2));
    assert!(!converted.raster.pixels.contains(&3));
}

#[test]
fn palette_distance_ties_choose_lower_palette_index() {
    let palette = Palette::new("tie", 0, vec![[0, 0, 0, 0], [0, 0, 0, 255], [2, 0, 0, 255]]);
    let source = RgbaImage {
        width: 1,
        height: 1,
        pixels: vec![[1, 0, 0, 255]],
    };
    let mut settings = settings(Registration::Center);
    settings.width = 3;
    settings.height = 3;
    settings.margin = 1;
    let converted = convert_reference(&source, &palette, &settings).expect("convert");
    assert!(converted.raster.pixels.contains(&1));
    assert!(!converted.raster.pixels.contains(&2));
}

#[test]
fn synthetic_fixture_matches_golden_hashes() {
    let source = fixture_image(include_bytes!(
        "../../tests/fixtures/m2/reference.rgba.json"
    ));
    let palette: Palette =
        serde_json::from_slice(include_bytes!("../../tests/fixtures/m2/palette.json"))
            .expect("fixture palette");
    let settings: ConversionSettings = serde_json::from_slice(include_bytes!(
        "../../tests/fixtures/m2/reference.settings.json"
    ))
    .expect("fixture settings");
    let converted = convert_reference(&source, &palette, &settings).expect("fixture conversion");
    let raster_hash = sha256_hex(&stable_json(&converted.raster).expect("canonical raster"));
    let rendered = render(&converted.raster, 8).expect("fixture render");

    assert_eq!(
        raster_hash,
        "fd568715dfa76ac11ae6e2a2299ecf99f45c7b0b4094044c4260b2fd0e53be9f"
    );
    assert_eq!(
        sha256_hex(&rendered.native_png),
        "9e1345c3b488327bb6839c177830c0f50f5121b21450de9eda42e1d923e4721e"
    );
    assert_eq!(
        sha256_hex(&rendered.preview_png),
        "22d22ac34a6764531e972636f4c0d10e17c01e9752ad01b3b2e9c4a44305f201"
    );
}

fn fixture_image(bytes: &[u8]) -> RgbaImage {
    let fixture: RgbaFixture = serde_json::from_slice(bytes).expect("RGBA fixture");
    RgbaImage {
        width: fixture.width,
        height: fixture.height,
        pixels: fixture.pixels.into_iter().flatten().collect(),
    }
}

fn visible_bounds_from_raster(raster: &IndexedRaster) -> Bounds {
    let image = RgbaImage {
        width: raster.width,
        height: raster.height,
        pixels: raster
            .pixels
            .iter()
            .map(|index| raster.palette.colors[usize::from(*index)])
            .collect(),
    };
    visible_bounds(&image).expect("visible raster")
}
