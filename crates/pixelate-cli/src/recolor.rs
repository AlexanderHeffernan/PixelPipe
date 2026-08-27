use std::path::PathBuf;

use pixelate_app::{
    InspectRevision, RemapRevisionDocument, load_revision_view, remap_revision_document,
};
use pixelate_core::{PALETTE_REMAP_SCHEMA, PaletteRemap};
use serde_json::json;

pub(crate) fn recolor_command(
    root: PathBuf,
    asset: String,
    parent: Option<String>,
    replacements: &[String],
    actor: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let view = load_revision_view(InspectRevision {
        start: root.clone(),
        asset: asset.clone(),
        revision: parent,
    })?;
    let parent = view.metadata.revision;
    let mut palette = view.metadata.palette;
    let mut changed = Vec::new();
    for replacement in replacements {
        let (index, color) = parse_palette_replacement(replacement)?;
        if index == palette.transparent_index {
            return Err("the transparent palette colour cannot be recoloured".into());
        }
        let entry = palette
            .colors
            .get_mut(usize::from(index))
            .ok_or_else(|| format!("palette index {index} does not exist"))?;
        *entry = [color[0], color[1], color[2], entry[3]];
        changed.push(json!({ "index": index, "rgba": entry }));
    }
    let index_map = (0..palette.colors.len())
        .map(u8::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let revision = remap_revision_document(RemapRevisionDocument {
        start: root,
        asset,
        parent,
        remap: PaletteRemap {
            schema: PALETTE_REMAP_SCHEMA.to_owned(),
            palette,
            index_map,
            structure: None,
        },
        brief: None,
        preview_scale: None,
        actor,
    })?;
    Ok(json!({ "ok": true, "changed": changed, "revision": revision }))
}

fn parse_palette_replacement(value: &str) -> Result<(u8, [u8; 3]), String> {
    let (index, color) = value
        .split_once('=')
        .ok_or_else(|| format!("invalid colour '{value}'; expected INDEX=#RRGGBB"))?;
    let index = index
        .parse::<u8>()
        .map_err(|_| format!("invalid palette index '{index}'"))?;
    let color = color.strip_prefix('#').unwrap_or(color);
    if color.len() != 6 || !color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("invalid colour '{color}'; expected #RRGGBB"));
    }
    let channel = |start| u8::from_str_radix(&color[start..start + 2], 16).unwrap_or_default();
    Ok((index, [channel(0), channel(2), channel(4)]))
}
