use std::io::Cursor;

use png::{ColorType, Transformations};

use super::model::RgbaImage;
use crate::CoreError;

/// Decodes a PNG into deterministic row-major RGBA8 pixels.
///
/// # Errors
///
/// Returns a [`CoreError`] for malformed PNG data, unsupported decoder output,
/// or inconsistent dimensions.
pub fn decode_rgba_png(bytes: &[u8]) -> Result<RgbaImage, CoreError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    let data = &buffer[..info.buffer_size()];
    let pixels = match info.color_type {
        ColorType::Rgba => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
            .collect(),
        ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        ColorType::Grayscale => data
            .iter()
            .map(|value| [*value, *value, *value, 255])
            .collect(),
        ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        ColorType::Indexed => return Err(CoreError::InvalidSourceImage),
    };
    let image = RgbaImage {
        width: info.width,
        height: info.height,
        pixels,
    };
    validate_source(&image)?;
    Ok(image)
}

pub(super) fn validate_source(source: &RgbaImage) -> Result<(), CoreError> {
    if source.width == 0
        || source.height == 0
        || source.width > 16384
        || source.height > 16384
        || source.pixels.len() != pixel_count(source.width, source.height)?
    {
        return Err(CoreError::InvalidSourceImage);
    }
    Ok(())
}

pub(super) fn source_offset(width: u32, x: u32, y: u32) -> Result<usize, CoreError> {
    let offset = u64::from(y)
        .checked_mul(u64::from(width))
        .and_then(|row| row.checked_add(u64::from(x)))
        .ok_or(CoreError::DimensionOverflow)?;
    usize::try_from(offset).map_err(|_| CoreError::DimensionOverflow)
}

pub(super) fn pixel_count(width: u32, height: u32) -> Result<usize, CoreError> {
    usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| CoreError::DimensionOverflow)
}
