use std::path::PathBuf;

use pixelate_core::{
    DEFAULT_FRAME_DURATION_MS, FrameOperation, IndexedFrame, IndexedSequence, convert_reference,
};
use pixelate_project::ProjectStore;

use crate::{AppError, animation};

pub(crate) fn default_duration() -> u32 {
    DEFAULT_FRAME_DURATION_MS
}

pub(crate) fn default_actor() -> String {
    "agent".to_owned()
}

pub(crate) fn import_frame(
    store: &ProjectStore,
    asset: &str,
    sequence: &mut IndexedSequence,
    file: &PathBuf,
    position: Option<usize>,
    duration_ms: Option<u32>,
) -> Result<FrameOperation, AppError> {
    let source = animation::read_image(file)?;
    let style = animation::style(store, asset)?;
    let raster = convert_reference(&source, &sequence.palette, &style.settings)?.raster;
    let id = sequence.next_frame_id();
    let position = animation::insertion_position(position, sequence.frames.len())?;
    sequence.frames.insert(
        position,
        IndexedFrame {
            id: id.clone(),
            name: None,
            duration_ms: duration_ms.unwrap_or(DEFAULT_FRAME_DURATION_MS),
            pixels: raster.pixels,
        },
    );
    Ok(FrameOperation::ImportFrame {
        frame_id: id,
        position,
    })
}

pub(crate) fn replace_frame(
    store: &ProjectStore,
    asset: &str,
    sequence: &mut IndexedSequence,
    frame_id: &str,
    file: &PathBuf,
) -> Result<FrameOperation, AppError> {
    let source = animation::read_image(file)?;
    let style = animation::style(store, asset)?;
    let raster = convert_reference(&source, &sequence.palette, &style.settings)?.raster;
    animation::find_frame_mut(sequence, frame_id)?.pixels = raster.pixels;
    Ok(FrameOperation::ReplaceFrame {
        frame_id: frame_id.to_owned(),
    })
}
