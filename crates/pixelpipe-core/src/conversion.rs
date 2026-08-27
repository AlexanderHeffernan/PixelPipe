mod backdrop;
mod components;
mod frame;
mod image;
mod model;
mod sheet;

pub use backdrop::detect_border_color;
pub use components::{validate_component_expectation, validate_sheet_component_expectation};
pub use frame::convert_reference;
pub use image::decode_rgba_png;
pub use model::{
    BackdropPolicy, ComponentExpectation, ConversionResult, ConversionSettings, Registration,
    RgbaImage, SheetSettings,
};
pub use sheet::convert_sheet;

pub(crate) use backdrop::cleaned_visible_pixels;

#[cfg(test)]
mod tests;
