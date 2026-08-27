use std::collections::{BTreeMap, VecDeque};

use super::{
    image::{source_offset, validate_source},
    model::{BackdropPolicy, Bounds, RgbaImage},
};
use crate::CoreError;

/// Detects the dominant opaque colour around the source image perimeter.
///
/// Nearby RGB values are grouped before averaging so compressed or softly
/// shaded solid backgrounds still resolve predictably.
///
/// # Errors
///
/// Returns [`CoreError`] when the source image is invalid.
pub fn detect_border_color(
    source: &RgbaImage,
    alpha_threshold: u8,
    tolerance: u8,
) -> Result<Option<[u8; 3]>, CoreError> {
    validate_source(source)?;
    let mut groups = BTreeMap::<[u8; 3], (u32, [u64; 3])>::new();
    let mut opaque_edges = 0_u32;
    let mut record = |x, y| -> Result<(), CoreError> {
        let pixel = source.pixels[source_offset(source.width, x, y)?];
        if pixel[3] <= alpha_threshold {
            return Ok(());
        }
        opaque_edges += 1;
        let key = [pixel[0] >> 4, pixel[1] >> 4, pixel[2] >> 4];
        let entry = groups.entry(key).or_default();
        entry.0 += 1;
        for (sum, value) in entry.1.iter_mut().zip(pixel) {
            *sum += u64::from(value);
        }
        Ok(())
    };
    for x in 0..source.width {
        record(x, 0)?;
        if source.height > 1 {
            record(x, source.height - 1)?;
        }
    }
    for y in 1..source.height.saturating_sub(1) {
        record(0, y)?;
        if source.width > 1 {
            record(source.width - 1, y)?;
        }
    }
    let Some((_, (count, sums))) = groups
        .into_iter()
        .max_by_key(|(key, (count, _))| (*count, std::cmp::Reverse(*key)))
    else {
        return Ok(None);
    };
    if count * 2 < opaque_edges {
        return Ok(None);
    }
    let color = sums.map(|sum| {
        u8::try_from((sum + u64::from(count) / 2) / u64::from(count)).unwrap_or(u8::MAX)
    });
    let mut candidate = clean_backdrop(source, &BackdropPolicy::Alpha { alpha_threshold });
    let visible = candidate.pixels.iter().filter(|pixel| pixel[3] > 0).count();
    let removed = remove_border_connected(&mut candidate, color, tolerance);
    if visible > 0 && removed * 100 >= visible * 96 {
        return Ok(None);
    }
    Ok(Some(color))
}

pub(crate) fn cleaned_visible_pixels(
    source: &RgbaImage,
    policy: &BackdropPolicy,
) -> Result<Vec<[u8; 4]>, CoreError> {
    validate_source(source)?;
    Ok(clean_backdrop(source, policy)
        .pixels
        .into_iter()
        .filter(|pixel| pixel[3] > 0)
        .collect())
}

pub(super) fn clean_backdrop(source: &RgbaImage, policy: &BackdropPolicy) -> RgbaImage {
    let mut cleaned = source.clone();
    let alpha_threshold = match policy {
        BackdropPolicy::Alpha { alpha_threshold }
        | BackdropPolicy::BorderConnected {
            alpha_threshold, ..
        } => *alpha_threshold,
    };
    for pixel in &mut cleaned.pixels {
        if pixel[3] <= alpha_threshold {
            pixel[3] = 0;
        }
    }
    if let BackdropPolicy::BorderConnected {
        color, tolerance, ..
    } = policy
    {
        if is_magenta_key(*color) {
            remove_matching_pixels(&mut cleaned, *color, *tolerance);
        } else {
            remove_border_connected(&mut cleaned, *color, *tolerance);
        }
    }
    cleaned
}

fn remove_matching_pixels(image: &mut RgbaImage, color: [u8; 3], tolerance: u8) -> usize {
    let mut removed = 0;
    for pixel in &mut image.pixels {
        if pixel[3] > 0 && matches_background(*pixel, color, tolerance) {
            pixel[3] = 0;
            removed += 1;
        }
    }
    removed
}

fn remove_border_connected(image: &mut RgbaImage, color: [u8; 3], tolerance: u8) -> usize {
    let Ok(length) = usize::try_from(image.width)
        .and_then(|width| usize::try_from(image.height).map(|height| width.saturating_mul(height)))
    else {
        return 0;
    };
    let mut queued = vec![false; length];
    let mut queue = VecDeque::new();
    for x in 0..image.width {
        enqueue_background(image, x, 0, color, tolerance, &mut queued, &mut queue);
        if image.height > 1 {
            enqueue_background(
                image,
                x,
                image.height - 1,
                color,
                tolerance,
                &mut queued,
                &mut queue,
            );
        }
    }
    for y in 0..image.height {
        enqueue_background(image, 0, y, color, tolerance, &mut queued, &mut queue);
        if image.width > 1 {
            enqueue_background(
                image,
                image.width - 1,
                y,
                color,
                tolerance,
                &mut queued,
                &mut queue,
            );
        }
    }
    let mut removed = 0;
    while let Some((x, y)) = queue.pop_front() {
        let Ok(offset) = source_offset(image.width, x, y) else {
            continue;
        };
        image.pixels[offset][3] = 0;
        removed += 1;
        if x > 0 {
            enqueue_background(image, x - 1, y, color, tolerance, &mut queued, &mut queue);
        }
        if x + 1 < image.width {
            enqueue_background(image, x + 1, y, color, tolerance, &mut queued, &mut queue);
        }
        if y > 0 {
            enqueue_background(image, x, y - 1, color, tolerance, &mut queued, &mut queue);
        }
        if y + 1 < image.height {
            enqueue_background(image, x, y + 1, color, tolerance, &mut queued, &mut queue);
        }
    }
    removed
}

fn enqueue_background(
    image: &RgbaImage,
    x: u32,
    y: u32,
    color: [u8; 3],
    tolerance: u8,
    queued: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
) {
    let Ok(offset) = source_offset(image.width, x, y) else {
        return;
    };
    if queued[offset] {
        return;
    }
    let pixel = image.pixels[offset];
    let matches = pixel[3] > 0 && matches_background(pixel, color, tolerance);
    if matches {
        queued[offset] = true;
        queue.push_back((x, y));
    }
}

fn matches_background(pixel: [u8; 4], color: [u8; 3], tolerance: u8) -> bool {
    if pixel[..3]
        .iter()
        .zip(color)
        .all(|(actual, expected)| actual.abs_diff(expected) <= tolerance)
    {
        return true;
    }
    if !is_magenta_key(color) {
        return false;
    }
    let actual = normalized_chroma([pixel[0], pixel[1], pixel[2]]);
    let expected = normalized_chroma(color);
    match (actual, expected) {
        (Some(actual), Some(expected)) => actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.abs_diff(expected) <= tolerance.saturating_mul(2)),
        _ => false,
    }
}

fn is_magenta_key(color: [u8; 3]) -> bool {
    color[0].saturating_sub(color[1]) >= 64 && color[2].saturating_sub(color[1]) >= 64
}

fn normalized_chroma(color: [u8; 3]) -> Option<[u8; 3]> {
    let minimum = *color.iter().min()?;
    let maximum = *color.iter().max()?;
    let chroma = maximum - minimum;
    if chroma < 48 {
        return None;
    }
    Some(color.map(|channel| {
        u8::try_from(u16::from(channel - minimum) * 255 / u16::from(chroma)).unwrap_or(u8::MAX)
    }))
}

pub(super) fn visible_bounds(image: &RgbaImage) -> Option<Bounds> {
    let mut min_x = image.width;
    let mut min_y = image.height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..image.height {
        for x in 0..image.width {
            if image.pixels[source_offset(image.width, x, y).ok()?][3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    found.then_some(Bounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}
