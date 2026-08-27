use std::collections::BTreeMap;

use crate::{
    BackdropPolicy, ColorAdjustments, ColorTreatment, CoreError, PALETTE_SCHEMA, Palette,
    RgbaImage, conversion::cleaned_visible_pixels,
};

#[derive(Clone)]
struct WeightedColor {
    rgb: [u8; 3],
    count: u32,
}

/// Derives a deterministic limited palette from visible source colours.
///
/// The requested count is the maximum number of visible colours and must be 2..=32.
///
/// # Errors
///
/// Returns [`CoreError`] when the source, backdrop, or colour count is invalid.
pub fn derive_source_palette(
    source: &RgbaImage,
    backdrop: &BackdropPolicy,
    color_count: u8,
    treatment: ColorTreatment,
    adjustments: ColorAdjustments,
) -> Result<Palette, CoreError> {
    if !(2..=64).contains(&color_count) {
        return Err(CoreError::InvalidDerivedColorCount);
    }
    let mut counts = BTreeMap::<[u8; 3], u32>::new();
    for pixel in cleaned_visible_pixels(source, backdrop)? {
        let pixel = adjustments.apply(treatment.apply(pixel));
        *counts.entry([pixel[0], pixel[1], pixel[2]]).or_default() += 1;
    }
    if counts.is_empty() {
        return Err(CoreError::EmptySource);
    }
    let colors = counts
        .into_iter()
        .map(|(rgb, count)| WeightedColor { rgb, count })
        .collect::<Vec<_>>();
    let mut boxes = vec![colors];
    while boxes.len() < usize::from(color_count) {
        let Some(index) = boxes
            .iter()
            .enumerate()
            .filter(|(_, colors)| colors.len() > 1)
            .max_by_key(|(_, colors)| box_priority(colors))
            .map(|(index, _)| index)
        else {
            break;
        };
        let colors = boxes.remove(index);
        let (left, right) = split_box(colors);
        boxes.push(left);
        boxes.push(right);
    }
    let mut resolved = boxes
        .iter()
        .map(|colors| representative(colors))
        .collect::<Vec<_>>();
    resolved.sort_by_key(|color| {
        (
            u32::from(color[0]) * 299 + u32::from(color[1]) * 587 + u32::from(color[2]) * 114,
            *color,
        )
    });
    let mut palette = vec![[0, 0, 0, 0]];
    palette.extend(
        resolved
            .into_iter()
            .map(|rgb| [rgb[0], rgb[1], rgb[2], 255]),
    );
    Ok(Palette {
        schema: PALETTE_SCHEMA.to_owned(),
        name: "Source Colours".to_owned(),
        transparent_index: 0,
        colors: palette,
    })
}

fn box_priority(colors: &[WeightedColor]) -> (u8, u32, usize) {
    let ranges = channel_ranges(colors);
    (
        *ranges.iter().max().unwrap_or(&0),
        colors.iter().map(|color| color.count).sum(),
        colors.len(),
    )
}

fn split_box(mut colors: Vec<WeightedColor>) -> (Vec<WeightedColor>, Vec<WeightedColor>) {
    let ranges = channel_ranges(&colors);
    let channel = ranges
        .iter()
        .enumerate()
        .max_by_key(|(channel, range)| (**range, std::cmp::Reverse(*channel)))
        .map_or(0, |(channel, _)| channel);
    colors.sort_by_key(|color| (color.rgb[channel], color.rgb));
    let total = colors.iter().map(|color| color.count).sum::<u32>();
    let mut cumulative = 0;
    let mut split = 1;
    for (index, color) in colors.iter().enumerate().take(colors.len() - 1) {
        cumulative += color.count;
        split = index + 1;
        if cumulative * 2 >= total {
            break;
        }
    }
    let right = colors.split_off(split);
    (colors, right)
}

fn channel_ranges(colors: &[WeightedColor]) -> [u8; 3] {
    let mut minimum = [u8::MAX; 3];
    let mut maximum = [0; 3];
    for color in colors {
        for channel in 0..3 {
            minimum[channel] = minimum[channel].min(color.rgb[channel]);
            maximum[channel] = maximum[channel].max(color.rgb[channel]);
        }
    }
    [
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ]
}

fn representative(colors: &[WeightedColor]) -> [u8; 3] {
    let total = u64::from(colors.iter().map(|color| color.count).sum::<u32>());
    let channel = |index| {
        let sum = colors
            .iter()
            .map(|color| u64::from(color.rgb[index]) * u64::from(color.count))
            .sum::<u64>();
        u8::try_from((sum + total / 2) / total).expect("weighted u8 average fits u8")
    };
    [channel(0), channel(1), channel(2)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_palette_is_limited_stable_and_ignores_cleaned_background() {
        let source = RgbaImage {
            width: 4,
            height: 2,
            pixels: vec![
                [255, 255, 255, 255],
                [220, 30, 30, 255],
                [230, 40, 40, 255],
                [255, 255, 255, 255],
                [255, 255, 255, 255],
                [20, 30, 210, 255],
                [30, 40, 220, 255],
                [255, 255, 255, 255],
            ],
        };
        let backdrop = BackdropPolicy::BorderConnected {
            color: [255, 255, 255],
            tolerance: 0,
            alpha_threshold: 0,
        };

        let first = derive_source_palette(
            &source,
            &backdrop,
            3,
            ColorTreatment::Original,
            ColorAdjustments::default(),
        )
        .expect("palette");
        let second = derive_source_palette(
            &source,
            &backdrop,
            3,
            ColorTreatment::Original,
            ColorAdjustments::default(),
        )
        .expect("palette");

        assert_eq!(first, second);
        assert_eq!(first.colors.len(), 4);
        assert_eq!(first.colors[0], [0, 0, 0, 0]);
        assert!(!first.colors.contains(&[255, 255, 255, 255]));

        let alternate = RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![[20, 180, 60, 255], [240, 190, 20, 255]],
        };
        let alpha = BackdropPolicy::Alpha { alpha_threshold: 0 };
        let alternate_original = derive_source_palette(
            &alternate,
            &alpha,
            3,
            ColorTreatment::Original,
            ColorAdjustments::default(),
        )
        .expect("palette");
        let alternate = derive_source_palette(
            &alternate,
            &alpha,
            3,
            ColorTreatment::Warm,
            ColorAdjustments::default(),
        )
        .expect("palette");
        assert_ne!(first.colors, alternate.colors);
        assert_ne!(alternate_original.colors, alternate.colors);
    }
}
