use std::collections::BTreeMap;

use png::{BitDepth, ColorType, Compression, Encoder, FilterType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PALETTE_SCHEMA: &str = "pixelpipe.palette/v1";
pub const RASTER_SCHEMA: &str = "pixelpipe.raster/v1";
pub const RECIPE_SCHEMA: &str = "pixelpipe.recipe/v1";
pub const VALIDATION_SCHEMA: &str = "pixelpipe.validation/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub schema: String,
    pub name: String,
    pub transparent_index: u8,
    pub colors: Vec<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedRaster {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub palette: Palette,
    pub pixels: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema: String,
    pub input_sha256: String,
    pub palette_sha256: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    RenderIndexed { preview_scale: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub schema: String,
    pub valid: bool,
    pub checks: Vec<ValidationCheck>,
    pub visual_review: VisualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualReview {
    Required,
    Passed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedRaster {
    pub native_png: Vec<u8>,
    pub preview_png: Vec<u8>,
    pub validation: ValidationReport,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unsupported schema '{actual}', expected '{expected}'")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("raster dimensions must be between 1 and 8192 pixels")]
    InvalidDimensions,
    #[error("pixel count {actual} does not match dimensions ({expected})")]
    PixelCount { expected: usize, actual: usize },
    #[error("palette must contain between 1 and 256 colors")]
    InvalidPaletteSize,
    #[error("transparent index {index} is outside the palette")]
    InvalidTransparentIndex { index: u8 },
    #[error("pixel index {index} at offset {offset} is outside the palette")]
    InvalidPixelIndex { index: u8, offset: usize },
    #[error("preview scale must be between 1 and 64")]
    InvalidPreviewScale,
    #[error("image dimensions overflow the supported range")]
    DimensionOverflow,
    #[error("PNG encoding failed: {0}")]
    Png(#[from] png::EncodingError),
    #[error("JSON encoding failed: {0}")]
    Json(#[from] serde_json::Error),
}

impl Palette {
    #[must_use]
    pub fn new(name: impl Into<String>, transparent_index: u8, colors: Vec<[u8; 4]>) -> Self {
        Self {
            schema: PALETTE_SCHEMA.to_owned(),
            name: name.into(),
            transparent_index,
            colors,
        }
    }
}

impl IndexedRaster {
    /// Validates structural and indexed-palette invariants.
    ///
    /// # Errors
    ///
    /// Returns a [`CoreError`] when a schema, dimension, palette, or pixel index
    /// is invalid.
    pub fn validate(&self) -> Result<ValidationReport, CoreError> {
        ensure_schema(&self.schema, RASTER_SCHEMA)?;
        ensure_schema(&self.palette.schema, PALETTE_SCHEMA)?;

        if self.width == 0 || self.height == 0 || self.width > 8192 || self.height > 8192 {
            return Err(CoreError::InvalidDimensions);
        }
        let expected = usize::try_from(self.width)
            .ok()
            .and_then(|width| {
                usize::try_from(self.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(CoreError::DimensionOverflow)?;
        if self.pixels.len() != expected {
            return Err(CoreError::PixelCount {
                expected,
                actual: self.pixels.len(),
            });
        }
        if self.palette.colors.is_empty() || self.palette.colors.len() > 256 {
            return Err(CoreError::InvalidPaletteSize);
        }
        if usize::from(self.palette.transparent_index) >= self.palette.colors.len() {
            return Err(CoreError::InvalidTransparentIndex {
                index: self.palette.transparent_index,
            });
        }
        if let Some((offset, index)) = self
            .pixels
            .iter()
            .copied()
            .enumerate()
            .find(|(_, index)| usize::from(*index) >= self.palette.colors.len())
        {
            return Err(CoreError::InvalidPixelIndex { index, offset });
        }

        Ok(ValidationReport {
            schema: VALIDATION_SCHEMA.to_owned(),
            valid: true,
            checks: vec![
                ValidationCheck {
                    name: "dimensions".to_owned(),
                    passed: true,
                    detail: format!("{}x{}", self.width, self.height),
                },
                ValidationCheck {
                    name: "pixel_count".to_owned(),
                    passed: true,
                    detail: expected.to_string(),
                },
                ValidationCheck {
                    name: "palette_indices".to_owned(),
                    passed: true,
                    detail: format!("{} colors", self.palette.colors.len()),
                },
                ValidationCheck {
                    name: "indexed_transparency".to_owned(),
                    passed: true,
                    detail: format!("index {}", self.palette.transparent_index),
                },
            ],
            visual_review: VisualReview::Required,
        })
    }
}

/// Encodes an indexed native PNG and exact nearest-neighbour preview.
///
/// # Errors
///
/// Returns a [`CoreError`] when the raster or scale is invalid, dimensions
/// overflow, or PNG encoding fails.
pub fn render(raster: &IndexedRaster, preview_scale: u16) -> Result<RenderedRaster, CoreError> {
    let validation = raster.validate()?;
    if !(1..=64).contains(&preview_scale) {
        return Err(CoreError::InvalidPreviewScale);
    }

    let native_png =
        encode_indexed_png(raster.width, raster.height, &raster.palette, &raster.pixels)?;
    let (preview_width, preview_height, preview_pixels) = nearest_pixels(raster, preview_scale)?;
    let preview_png = encode_indexed_png(
        preview_width,
        preview_height,
        &raster.palette,
        &preview_pixels,
    )?;

    Ok(RenderedRaster {
        native_png,
        preview_png,
        validation,
    })
}

/// Serializes a persisted value as pretty JSON with a trailing newline.
///
/// # Errors
///
/// Returns a [`CoreError`] when `value` cannot be serialized.
pub fn stable_json<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn ensure_schema(actual: &str, expected: &'static str) -> Result<(), CoreError> {
    if actual == expected {
        Ok(())
    } else {
        Err(CoreError::Schema {
            expected,
            actual: actual.to_owned(),
        })
    }
}

fn nearest_pixels(raster: &IndexedRaster, scale: u16) -> Result<(u32, u32, Vec<u8>), CoreError> {
    let scale_u32 = u32::from(scale);
    let width = raster
        .width
        .checked_mul(scale_u32)
        .ok_or(CoreError::DimensionOverflow)?;
    let height = raster
        .height
        .checked_mul(scale_u32)
        .ok_or(CoreError::DimensionOverflow)?;
    if width > 8192 || height > 8192 {
        return Err(CoreError::InvalidDimensions);
    }

    let capacity = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(CoreError::DimensionOverflow)?;
    let source_width = usize::try_from(raster.width).map_err(|_| CoreError::DimensionOverflow)?;
    let scale = usize::from(scale);
    let mut pixels = Vec::with_capacity(capacity);

    for source_row in raster.pixels.chunks_exact(source_width) {
        let mut expanded_row = Vec::with_capacity(usize::try_from(width).unwrap_or(0));
        for index in source_row {
            expanded_row.extend(std::iter::repeat_n(*index, scale));
        }
        for _ in 0..scale {
            pixels.extend_from_slice(&expanded_row);
        }
    }

    Ok((width, height, pixels))
}

fn encode_indexed_png(
    width: u32,
    height: u32,
    palette: &Palette,
    pixels: &[u8],
) -> Result<Vec<u8>, CoreError> {
    let mut output = Vec::new();
    {
        let mut encoder = Encoder::new(&mut output, width, height);
        encoder.set_color(ColorType::Indexed);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Best);
        encoder.set_filter(FilterType::NoFilter);

        let rgb = palette
            .colors
            .iter()
            .flat_map(|color| color[..3].iter().copied())
            .collect::<Vec<_>>();
        let alpha = palette
            .colors
            .iter()
            .map(|color| color[3])
            .collect::<Vec<_>>();
        encoder.set_palette(rgb);
        encoder.set_trns(alpha);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixels)?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> IndexedRaster {
        IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: 2,
            height: 2,
            palette: Palette::new(
                "test",
                0,
                vec![[0, 0, 0, 0], [255, 80, 20, 255], [250, 230, 120, 255]],
            ),
            pixels: vec![0, 1, 2, 1],
            pivot: Some([1, 1]),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn rendering_is_byte_stable() {
        let first = render(&fixture(), 4).expect("first render");
        let second = render(&fixture(), 4).expect("second render");

        assert_eq!(first.native_png, second.native_png);
        assert_eq!(first.preview_png, second.preview_png);
        assert_eq!(first.validation, second.validation);
    }

    #[test]
    fn preview_is_exact_nearest_neighbour() {
        let raster = fixture();
        let (width, height, pixels) = nearest_pixels(&raster, 2).expect("nearest pixels");

        assert_eq!((width, height), (4, 4));
        assert_eq!(pixels, vec![0, 0, 1, 1, 0, 0, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1]);
    }

    #[test]
    fn rejects_out_of_palette_pixels() {
        let mut raster = fixture();
        raster.pixels[3] = 3;

        assert!(matches!(
            raster.validate(),
            Err(CoreError::InvalidPixelIndex {
                index: 3,
                offset: 3
            })
        ));
    }

    #[test]
    fn synthetic_fixture_matches_golden_hashes() {
        let raster: IndexedRaster =
            serde_json::from_str(include_str!("../../../fixtures/m1/tiny-raster.json"))
                .expect("synthetic fixture JSON");
        let golden: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../../fixtures/m1/golden-hashes.json"))
                .expect("golden hash JSON");
        let rendered = render(&raster, 4).expect("golden render");

        assert_eq!(golden["native_png"], sha256_hex(&rendered.native_png));
        assert_eq!(golden["preview_png"], sha256_hex(&rendered.preview_png));
    }
}
