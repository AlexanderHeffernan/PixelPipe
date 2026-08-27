use serde::{Deserialize, Serialize};

use crate::{ColorAdjustments, ColorTreatment, IndexedRaster, ValidationCheck};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackdropPolicy {
    Alpha {
        alpha_threshold: u8,
    },
    BorderConnected {
        color: [u8; 3],
        tolerance: u8,
        alpha_threshold: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Registration {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentExpectation {
    pub min: u16,
    pub max: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversionSettings {
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "ColorTreatment::is_original")]
    pub color_treatment: ColorTreatment,
    #[serde(default, skip_serializing_if = "ColorAdjustments::is_neutral")]
    pub color_adjustments: ColorAdjustments,
    pub margin: u16,
    #[serde(default = "default_subject_scale")]
    pub subject_scale_percent: u8,
    #[serde(default)]
    pub offset_x: i16,
    #[serde(default)]
    pub offset_y: i16,
    pub coverage_percent: u8,
    pub backdrop: BackdropPolicy,
    pub registration: Registration,
    pub components: ComponentExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetSettings {
    pub columns: u16,
    pub rows: u16,
    pub frame: ConversionSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionResult {
    pub raster: IndexedRaster,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedFrame {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) pixels: Vec<Option<u8>>,
    pub(super) source_bounds: Bounds,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Bounds {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Scale {
    pub(super) numerator: u32,
    pub(super) denominator: u32,
}

const fn default_subject_scale() -> u8 {
    100
}
