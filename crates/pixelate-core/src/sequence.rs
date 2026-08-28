use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    CoreError, IndexedRaster, Palette, RASTER_SCHEMA, RenderedRaster, VALIDATION_SCHEMA,
    ValidationCheck, ValidationReport, encode_indexed_png, render,
};

pub const SEQUENCE_SCHEMA: &str = "pixelate.sequence/v1";
pub const DEFAULT_FRAME_DURATION_MS: u32 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedFrame {
    pub id: String,
    pub duration_ms: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedSequence {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub palette: Palette,
    pub frames: Vec<IndexedFrame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl IndexedSequence {
    #[must_use]
    pub fn from_raster(raster: IndexedRaster) -> Self {
        Self {
            schema: SEQUENCE_SCHEMA.to_owned(),
            width: raster.width,
            height: raster.height,
            palette: raster.palette,
            frames: vec![IndexedFrame {
                id: "frame-0001".to_owned(),
                duration_ms: DEFAULT_FRAME_DURATION_MS,
                pixels: raster.pixels,
            }],
            pivot: raster.pivot,
            metadata: raster.metadata,
        }
    }

    /// Validates shared canvas/palette and every ordered frame.
    ///
    /// # Errors
    /// Returns [`CoreError`] for an invalid schema, canvas, palette, frame ID,
    /// duration, pixel count, or palette index.
    pub fn validate(&self) -> Result<ValidationReport, CoreError> {
        if self.schema != SEQUENCE_SCHEMA {
            return Err(CoreError::Schema {
                expected: SEQUENCE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.frames.is_empty() {
            return Err(CoreError::EmptySequence);
        }
        let mut ids = BTreeSet::new();
        for frame in &self.frames {
            if frame.id.trim().is_empty() {
                return Err(CoreError::InvalidFrameId);
            }
            if !ids.insert(frame.id.clone()) {
                return Err(CoreError::DuplicateFrameId(frame.id.clone()));
            }
            if frame.duration_ms == 0 {
                return Err(CoreError::InvalidFrameDuration {
                    frame: frame.id.clone(),
                });
            }
            self.raster_for_pixels(frame.pixels.clone()).validate()?;
        }
        Ok(ValidationReport {
            schema: VALIDATION_SCHEMA.to_owned(),
            valid: true,
            checks: vec![
                ValidationCheck {
                    name: "frame_count".to_owned(),
                    passed: true,
                    detail: self.frames.len().to_string(),
                },
                ValidationCheck {
                    name: "frame_ids".to_owned(),
                    passed: true,
                    detail: "unique".to_owned(),
                },
                ValidationCheck {
                    name: "frame_durations".to_owned(),
                    passed: true,
                    detail: "nonzero milliseconds".to_owned(),
                },
            ],
        })
    }

    /// Returns one frame as a compatibility raster with shared properties.
    ///
    /// # Errors
    /// Returns [`CoreError`] when the sequence or frame ID is invalid.
    pub fn raster(&self, frame_id: &str) -> Result<IndexedRaster, CoreError> {
        self.validate()?;
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.id == frame_id)
            .ok_or_else(|| CoreError::FrameNotFound(frame_id.to_owned()))?;
        Ok(self.raster_for_pixels(frame.pixels.clone()))
    }

    /// Returns the first frame as a compatibility raster.
    ///
    /// # Errors
    /// Returns [`CoreError`] when the sequence is invalid.
    pub fn first_raster(&self) -> Result<IndexedRaster, CoreError> {
        let id = self
            .frames
            .first()
            .ok_or(CoreError::EmptySequence)?
            .id
            .clone();
        self.raster(&id)
    }

    #[must_use]
    pub fn next_frame_id(&self) -> String {
        let mut number = 1_u32;
        loop {
            let candidate = format!("frame-{number:04}");
            if self.frames.iter().all(|frame| frame.id != candidate) {
                return candidate;
            }
            number += 1;
        }
    }

    fn raster_for_pixels(&self, pixels: Vec<u8>) -> IndexedRaster {
        IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: self.width,
            height: self.height,
            palette: self.palette.clone(),
            pixels,
            pivot: self.pivot,
            metadata: self.metadata.clone(),
        }
    }
}

/// Renders one-frame sequences byte-identically to legacy rasters and
/// multi-frame sequences as deterministic horizontal indexed PNG sheets.
///
/// # Errors
/// Returns [`CoreError`] when validation, sheet dimensions, or PNG encoding fails.
pub fn render_sequence(sequence: &IndexedSequence) -> Result<RenderedRaster, CoreError> {
    let validation = sequence.validate()?;
    if sequence.frames.len() == 1 {
        return render(&sequence.first_raster()?, 1);
    }
    let frame_count =
        u32::try_from(sequence.frames.len()).map_err(|_| CoreError::DimensionOverflow)?;
    let width = sequence
        .width
        .checked_mul(frame_count)
        .ok_or(CoreError::DimensionOverflow)?;
    if width > 8192 {
        return Err(CoreError::SheetDimensionOverflow {
            width,
            height: sequence.height,
        });
    }
    let row_width = usize::try_from(sequence.width).map_err(|_| CoreError::DimensionOverflow)?;
    let mut pixels = Vec::with_capacity(
        usize::try_from(u64::from(width) * u64::from(sequence.height))
            .map_err(|_| CoreError::DimensionOverflow)?,
    );
    for y in 0..sequence.height {
        let start = usize::try_from(u64::from(y) * u64::from(sequence.width))
            .map_err(|_| CoreError::DimensionOverflow)?;
        for frame in &sequence.frames {
            pixels.extend_from_slice(&frame.pixels[start..start + row_width]);
        }
    }
    let png = encode_indexed_png(width, sequence.height, &sequence.palette, &pixels)?;
    Ok(RenderedRaster {
        native_png: png.clone(),
        preview_png: png,
        validation,
    })
}

/// Renders an exact nearest-neighbour contact sheet for animation inspection.
///
/// # Errors
/// Returns [`CoreError`] when validation, scale, dimensions, or PNG encoding fails.
pub fn render_sequence_preview(
    sequence: &IndexedSequence,
    scale: u16,
) -> Result<Vec<u8>, CoreError> {
    sequence.validate()?;
    if sequence.frames.len() == 1 {
        return Ok(render(&sequence.first_raster()?, scale)?.preview_png);
    }
    if !(1..=64).contains(&scale) {
        return Err(CoreError::InvalidPreviewScale);
    }
    let scale = u32::from(scale);
    let frame_count =
        u32::try_from(sequence.frames.len()).map_err(|_| CoreError::DimensionOverflow)?;
    let width = sequence
        .width
        .checked_mul(frame_count)
        .and_then(|width| width.checked_mul(scale))
        .ok_or(CoreError::DimensionOverflow)?;
    let height = sequence
        .height
        .checked_mul(scale)
        .ok_or(CoreError::DimensionOverflow)?;
    if width > 8192 || height > 8192 {
        return Err(CoreError::SheetDimensionOverflow { width, height });
    }
    let mut pixels = Vec::with_capacity(
        usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_| CoreError::DimensionOverflow)?,
    );
    let source_width = usize::try_from(sequence.width).map_err(|_| CoreError::DimensionOverflow)?;
    for y in 0..sequence.height {
        let start = usize::try_from(u64::from(y) * u64::from(sequence.width))
            .map_err(|_| CoreError::DimensionOverflow)?;
        let mut row = Vec::with_capacity(usize::try_from(width).unwrap_or(0));
        for frame in &sequence.frames {
            for pixel in &frame.pixels[start..start + source_width] {
                row.extend(std::iter::repeat_n(
                    *pixel,
                    usize::try_from(scale).unwrap_or(1),
                ));
            }
        }
        for _ in 0..scale {
            pixels.extend_from_slice(&row);
        }
    }
    encode_indexed_png(width, height, &sequence.palette, &pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence() -> IndexedSequence {
        IndexedSequence {
            schema: SEQUENCE_SCHEMA.to_owned(),
            width: 2,
            height: 1,
            palette: Palette::new("shared", 0, vec![[0, 0, 0, 0], [255, 0, 0, 255]]),
            frames: vec![
                IndexedFrame {
                    id: "a".into(),
                    duration_ms: 80,
                    pixels: vec![0, 1],
                },
                IndexedFrame {
                    id: "b".into(),
                    duration_ms: 120,
                    pixels: vec![1, 0],
                },
            ],
            pivot: Some([1, 1]),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn validates_frame_invariants() {
        assert!(sequence().validate().is_ok());

        let mut value = sequence();
        value.width = 0;
        assert!(matches!(
            value.validate(),
            Err(CoreError::InvalidDimensions)
        ));

        let mut value = sequence();
        value.frames[0].pixels.pop();
        assert!(matches!(
            value.validate(),
            Err(CoreError::PixelCount { .. })
        ));

        let mut value = sequence();
        value.frames[0].pixels[0] = 2;
        assert!(matches!(
            value.validate(),
            Err(CoreError::InvalidPixelIndex { .. })
        ));

        let mut value = sequence();
        value.frames[1].id = "a".into();
        assert!(matches!(
            value.validate(),
            Err(CoreError::DuplicateFrameId(_))
        ));

        let mut value = sequence();
        value.frames[1].duration_ms = 0;
        assert!(matches!(
            value.validate(),
            Err(CoreError::InvalidFrameDuration { .. })
        ));

        let mut value = sequence();
        value.frames.clear();
        assert!(matches!(value.validate(), Err(CoreError::EmptySequence)));
    }

    #[test]
    fn legacy_wrapping_preserves_one_frame_render_bytes() {
        let raster = sequence().first_raster().unwrap();
        let legacy = render(&raster, 1).unwrap();
        let wrapped = render_sequence(&IndexedSequence::from_raster(raster)).unwrap();
        assert_eq!(legacy.native_png, wrapped.native_png);
    }

    #[test]
    fn refuses_sheets_beyond_the_supported_edge() {
        let mut value = sequence();
        value.width = 8192;
        value
            .frames
            .iter_mut()
            .for_each(|frame| frame.pixels = vec![0; 8192]);
        assert!(matches!(
            render_sequence(&value),
            Err(CoreError::SheetDimensionOverflow { .. })
        ));
    }
}
