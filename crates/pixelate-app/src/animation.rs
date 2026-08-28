use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use pixelate_core::{
    DEFAULT_FRAME_DURATION_MS, FrameOperation, IndexedFrame, IndexedSequence, Operation,
    RECIPE_SCHEMA, Recipe, RgbaImage, SEQUENCE_SCHEMA, ValidationCheck, convert_reference,
    derive_source_palette_batch, sha256_hex, stable_json,
};
use pixelate_project::{ProjectStore, RevisionSnapshot};
use serde::Deserialize;

use crate::{AppError, CommitSequence, RevisionResult, commit_sequence};

#[derive(Debug, Deserialize)]
pub struct FrameMutation {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub action: FrameMutationAction,
    #[serde(default = "default_actor")]
    pub actor: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FrameMutationAction {
    AddBlank {
        position: Option<usize>,
        duration_ms: Option<u32>,
    },
    Duplicate {
        frame_id: String,
        position: Option<usize>,
    },
    Delete {
        frame_id: String,
    },
    Reorder {
        frame_id: String,
        position: usize,
    },
    SetDuration {
        frame_id: String,
        duration_ms: u32,
    },
    Rename {
        frame_id: String,
        name: String,
    },
    ImportFrame {
        file: PathBuf,
        position: Option<usize>,
        duration_ms: Option<u32>,
    },
}

#[derive(Debug, Deserialize)]
pub struct ImportImageSequence {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub files: Vec<PathBuf>,
    #[serde(default = "default_duration")]
    pub duration_ms: u32,
    #[serde(default = "default_actor")]
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportSpritesheet {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub file: PathBuf,
    pub frame_width: u32,
    pub frame_height: u32,
    pub order: Vec<usize>,
    #[serde(default = "default_duration")]
    pub duration_ms: u32,
    #[serde(default = "default_actor")]
    pub actor: String,
}

/// Applies one frame-level mutation as a whole-sequence immutable revision.
///
/// # Errors
/// Returns [`AppError`] for an invalid parent, target, position, image, or sequence.
pub fn mutate_frames(request: FrameMutation) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let mut sequence = parent.sequence.clone();
    let action = match request.action {
        FrameMutationAction::AddBlank {
            position,
            duration_ms,
        } => add_blank(&mut sequence, position, duration_ms)?,
        FrameMutationAction::Duplicate { frame_id, position } => {
            duplicate_frame(&mut sequence, frame_id, position)?
        }
        FrameMutationAction::Delete { frame_id } => {
            if sequence.frames.len() == 1 {
                return Err(AppError::CannotDeleteFinalFrame);
            }
            let index = frame_index(&sequence, &frame_id)?;
            sequence.frames.remove(index);
            FrameOperation::Delete { frame_id }
        }
        FrameMutationAction::Reorder { frame_id, position } => {
            if position >= sequence.frames.len() {
                return Err(AppError::InvalidFramePosition(position));
            }
            let index = frame_index(&sequence, &frame_id)?;
            let frame = sequence.frames.remove(index);
            sequence.frames.insert(position, frame);
            FrameOperation::Reorder { frame_id, position }
        }
        FrameMutationAction::SetDuration {
            frame_id,
            duration_ms,
        } => {
            find_frame_mut(&mut sequence, &frame_id)?.duration_ms = duration_ms;
            FrameOperation::SetDuration {
                frame_id,
                duration_ms,
            }
        }
        FrameMutationAction::Rename { frame_id, name } => {
            let name = name.trim().to_owned();
            find_frame_mut(&mut sequence, &frame_id)?.name = Some(name.clone());
            FrameOperation::Rename { frame_id, name }
        }
        FrameMutationAction::ImportFrame {
            file,
            position,
            duration_ms,
        } => import_frame(
            &store,
            &request.asset,
            &mut sequence,
            &file,
            position,
            duration_ms,
        )?,
    };
    commit_animation(
        &store,
        &request.asset,
        &request.parent,
        parent,
        sequence,
        action,
        request.actor,
    )
}

/// Replaces the current sequence from explicitly ordered images using one shared palette.
///
/// # Errors
/// Returns [`AppError`] when the ordered list, images, conversion, or commit is invalid.
pub fn import_image_sequence(request: ImportImageSequence) -> Result<RevisionResult, AppError> {
    if request.files.is_empty() {
        return Err(AppError::EmptyFrameImport);
    }
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let style = style(&store, &request.asset)?;
    let sources = request
        .files
        .iter()
        .map(read_image)
        .collect::<Result<Vec<_>, _>>()?;
    let palette = derive_source_palette_batch(
        &sources,
        &style.settings.backdrop,
        style.color_count,
        style.settings.color_treatment,
        style.settings.color_adjustments,
    )?;
    let frames = convert_sources(&sources, &palette, &style.settings, request.duration_ms)?;
    let frame_ids = frames.iter().map(|frame| frame.id.clone()).collect();
    let first = convert_reference(&sources[0], &palette, &style.settings)?.raster;
    let sequence = IndexedSequence {
        schema: SEQUENCE_SCHEMA.to_owned(),
        width: first.width,
        height: first.height,
        palette,
        frames,
        pivot: first.pivot,
        metadata: first.metadata,
    };
    commit_animation(
        &store,
        &request.asset,
        &request.parent,
        parent,
        sequence,
        FrameOperation::ImportSequence { frame_ids },
        request.actor,
    )
}

/// Imports explicitly selected cells from a regular spritesheet in supplied order.
///
/// # Errors
/// Returns [`AppError`] when the grid or selected order is invalid.
pub fn import_spritesheet(request: ImportSpritesheet) -> Result<RevisionResult, AppError> {
    let sheet = read_image(&request.file)?;
    if request.frame_width == 0
        || request.frame_height == 0
        || sheet.width % request.frame_width != 0
        || sheet.height % request.frame_height != 0
        || request.order.is_empty()
    {
        return Err(AppError::InvalidSpritesheetGrid);
    }
    let columns = sheet.width / request.frame_width;
    let rows = sheet.height / request.frame_height;
    let cells = usize::try_from(columns * rows).map_err(|_| AppError::InvalidSpritesheetGrid)?;
    let sources = request
        .order
        .iter()
        .map(|index| {
            if *index >= cells {
                return Err(AppError::SpritesheetFrameOutOfBounds(*index));
            }
            let index = u32::try_from(*index).map_err(|_| AppError::InvalidSpritesheetGrid)?;
            crop(
                &sheet,
                index % columns,
                index / columns,
                request.frame_width,
                request.frame_height,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    import_image_sources(
        &request.start,
        &request.asset,
        &request.parent,
        &sources,
        request.duration_ms,
        request.actor,
        true,
    )
}

fn import_image_sources(
    start: &Path,
    asset: &str,
    parent_id: &str,
    sources: &[RgbaImage],
    duration_ms: u32,
    actor: String,
    spritesheet: bool,
) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(start)?;
    let parent = store.revision(asset, parent_id)?;
    let style = style(&store, asset)?;
    let palette = derive_source_palette_batch(
        sources,
        &style.settings.backdrop,
        style.color_count,
        style.settings.color_treatment,
        style.settings.color_adjustments,
    )?;
    let frames = convert_sources(sources, &palette, &style.settings, duration_ms)?;
    let frame_ids = frames
        .iter()
        .map(|frame| frame.id.clone())
        .collect::<Vec<_>>();
    let first = convert_reference(&sources[0], &palette, &style.settings)?.raster;
    let sequence = IndexedSequence {
        schema: SEQUENCE_SCHEMA.into(),
        width: first.width,
        height: first.height,
        palette,
        frames,
        pivot: first.pivot,
        metadata: first.metadata,
    };
    let action = if spritesheet {
        FrameOperation::ImportSpritesheet { frame_ids }
    } else {
        FrameOperation::ImportSequence { frame_ids }
    };
    commit_animation(&store, asset, parent_id, parent, sequence, action, actor)
}

fn add_blank(
    sequence: &mut IndexedSequence,
    position: Option<usize>,
    duration_ms: Option<u32>,
) -> Result<FrameOperation, AppError> {
    let id = sequence.next_frame_id();
    let position = insertion_position(position, sequence.frames.len())?;
    let pixels = vec![sequence.palette.transparent_index; frame_pixel_count(sequence)?];
    sequence.frames.insert(
        position,
        IndexedFrame {
            id: id.clone(),
            name: None,
            duration_ms: duration_ms.unwrap_or(DEFAULT_FRAME_DURATION_MS),
            pixels,
        },
    );
    Ok(FrameOperation::AddBlank {
        frame_id: id,
        position,
    })
}

fn duplicate_frame(
    sequence: &mut IndexedSequence,
    source_id: String,
    position: Option<usize>,
) -> Result<FrameOperation, AppError> {
    let source = find_frame(sequence, &source_id)?.clone();
    let id = sequence.next_frame_id();
    let position = insertion_position(position, sequence.frames.len())?;
    sequence.frames.insert(
        position,
        IndexedFrame {
            id: id.clone(),
            name: source.name,
            duration_ms: source.duration_ms,
            pixels: source.pixels,
        },
    );
    Ok(FrameOperation::Duplicate {
        source_frame_id: source_id,
        frame_id: id,
        position,
    })
}

fn import_frame(
    store: &ProjectStore,
    asset: &str,
    sequence: &mut IndexedSequence,
    file: &PathBuf,
    position: Option<usize>,
    duration_ms: Option<u32>,
) -> Result<FrameOperation, AppError> {
    let source = read_image(file)?;
    let style = style(store, asset)?;
    let raster = convert_reference(&source, &sequence.palette, &style.settings)?.raster;
    let id = sequence.next_frame_id();
    let position = insertion_position(position, sequence.frames.len())?;
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

fn commit_animation(
    store: &ProjectStore,
    asset: &str,
    parent_id: &str,
    parent: RevisionSnapshot,
    sequence: IndexedSequence,
    action: FrameOperation,
    actor: String,
) -> Result<RevisionResult, AppError> {
    sequence.validate()?;
    let input_hash = sha256_hex(&stable_json(&parent.sequence)?);
    let palette_hash = sha256_hex(&stable_json(&sequence.palette)?);
    let detail = format!("{} frames", sequence.frames.len());
    commit_sequence(
        store,
        CommitSequence {
            asset: asset.to_owned(),
            sequence,
            recipe: Recipe {
                schema: RECIPE_SCHEMA.into(),
                input_sha256: input_hash.clone(),
                palette_sha256: palette_hash.clone(),
                operations: vec![Operation::EditFrames { action }],
            },
            brief: parent.brief,
            actor,
            input_hashes: BTreeMap::from([
                ("palette".into(), palette_hash),
                ("parent_pixels".into(), input_hash),
            ]),
            additional_checks: vec![ValidationCheck {
                name: "frame_mutation".into(),
                passed: true,
                detail,
            }],
            parent: Some(parent_id.to_owned()),
            style: None,
        },
    )
}

fn convert_sources(
    sources: &[RgbaImage],
    palette: &pixelate_core::Palette,
    settings: &pixelate_core::ConversionSettings,
    duration_ms: u32,
) -> Result<Vec<IndexedFrame>, AppError> {
    sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            Ok(IndexedFrame {
                id: format!("frame-{:04}", index + 1),
                name: None,
                duration_ms,
                pixels: convert_reference(source, palette, settings)?.raster.pixels,
            })
        })
        .collect()
}

fn style(store: &ProjectStore, asset: &str) -> Result<pixelate_project::AssetStyle, AppError> {
    store.asset(asset)?.style.ok_or_else(|| {
        AppError::UnsupportedConversion(
            "frame image import requires an established pixelization style".to_owned(),
        )
    })
}

fn read_image(path: &PathBuf) -> Result<RgbaImage, AppError> {
    let bytes = fs::read(path).map_err(|source| AppError::Read {
        path: path.clone(),
        source,
    })?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| AppError::Image(error.to_string()))?
        .into_rgba8();
    Ok(RgbaImage {
        width: image.width(),
        height: image.height(),
        pixels: image.pixels().map(|pixel| pixel.0).collect(),
    })
}

fn crop(
    source: &RgbaImage,
    column: u32,
    row: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, AppError> {
    let mut pixels = Vec::with_capacity(usize::try_from(width * height).unwrap_or(0));
    for y in row * height..(row + 1) * height {
        for x in column * width..(column + 1) * width {
            let offset = usize::try_from(u64::from(y) * u64::from(source.width) + u64::from(x))
                .map_err(|_| AppError::InvalidSpritesheetGrid)?;
            pixels.push(source.pixels[offset]);
        }
    }
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

fn frame_pixel_count(sequence: &IndexedSequence) -> Result<usize, AppError> {
    usize::try_from(u64::from(sequence.width) * u64::from(sequence.height))
        .map_err(|_| pixelate_core::CoreError::DimensionOverflow.into())
}
fn insertion_position(position: Option<usize>, len: usize) -> Result<usize, AppError> {
    let position = position.unwrap_or(len);
    if position > len {
        Err(AppError::InvalidFramePosition(position))
    } else {
        Ok(position)
    }
}
fn frame_index(sequence: &IndexedSequence, id: &str) -> Result<usize, AppError> {
    sequence
        .frames
        .iter()
        .position(|frame| frame.id == id)
        .ok_or_else(|| pixelate_core::CoreError::FrameNotFound(id.to_owned()).into())
}
fn find_frame<'a>(sequence: &'a IndexedSequence, id: &str) -> Result<&'a IndexedFrame, AppError> {
    Ok(&sequence.frames[frame_index(sequence, id)?])
}
fn find_frame_mut<'a>(
    sequence: &'a mut IndexedSequence,
    id: &str,
) -> Result<&'a mut IndexedFrame, AppError> {
    let index = frame_index(sequence, id)?;
    Ok(&mut sequence.frames[index])
}
fn default_duration() -> u32 {
    DEFAULT_FRAME_DURATION_MS
}
fn default_actor() -> String {
    "agent".to_owned()
}
