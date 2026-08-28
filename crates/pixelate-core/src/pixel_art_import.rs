use std::collections::{BTreeMap, BTreeSet};

use crate::{CoreError, IndexedRaster, Palette, RASTER_SCHEMA, RgbaImage};

/// Converts an RGBA image to exact indexed pixels without scaling or colour conversion.
///
/// # Errors
///
/// Returns an error when dimensions are invalid or the source has more than 256 RGBA colours.
pub fn import_pixel_art(source: &RgbaImage) -> Result<IndexedRaster, CoreError> {
    if source.width == 0 || source.height == 0 || source.width > 8192 || source.height > 8192 {
        return Err(CoreError::InvalidDimensions);
    }
    let mut colors = source.pixels.iter().copied().collect::<BTreeSet<_>>();
    colors.insert([0, 0, 0, 0]);
    if colors.len() > 256 {
        return Err(CoreError::TooManyColors(colors.len()));
    }
    let colors = colors.into_iter().collect::<Vec<_>>();
    let indices = colors
        .iter()
        .copied()
        .zip(0..=u8::MAX)
        .collect::<BTreeMap<_, _>>();
    let transparent_index = indices[&[0, 0, 0, 0]];
    let raster = IndexedRaster {
        schema: RASTER_SCHEMA.to_owned(),
        width: source.width,
        height: source.height,
        palette: Palette::new("Imported Pixel Art", transparent_index, colors),
        pixels: source.pixels.iter().map(|pixel| indices[pixel]).collect(),
        pivot: None,
        metadata: BTreeMap::new(),
    };
    raster.validate()?;
    Ok(raster)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_exact_pixels_and_dimensions() {
        let source = RgbaImage {
            width: 2,
            height: 2,
            pixels: vec![
                [0, 0, 0, 0],
                [20, 10, 5, 255],
                [200, 30, 4, 255],
                [20, 10, 5, 255],
            ],
        };
        let raster = import_pixel_art(&source).expect("import");
        assert_eq!((raster.width, raster.height), (2, 2));
        let restored = raster
            .pixels
            .iter()
            .map(|index| raster.palette.colors[usize::from(*index)])
            .collect::<Vec<_>>();
        assert_eq!(restored, source.pixels);
    }
}
