mod backdrop;
mod components;
mod frame;
mod image;
mod model;

pub use backdrop::detect_border_color;
pub use components::validate_component_expectation;
pub use frame::convert_reference;
pub use image::decode_rgba_png;
pub use model::{
    BackdropPolicy, ComponentExpectation, ConversionResult, ConversionSettings, Registration,
    RgbaImage,
};

pub(crate) use backdrop::cleaned_visible_pixels;

#[cfg(test)]
mod tests;
