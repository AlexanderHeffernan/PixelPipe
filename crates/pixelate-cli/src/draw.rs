use std::path::PathBuf;

use pixelate_app::{
    InspectRevision, PatchRevisionDocument, load_revision_view, patch_revision_document,
};
use pixelate_core::{PATCH_SCHEMA, PixelPatch, PixelPatchSet};
use serde_json::json;

pub(crate) fn draw_command(
    root: PathBuf,
    asset: String,
    parent: Option<String>,
    pixels: &[String],
    actor: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let parent = load_revision_view(InspectRevision {
        start: root.clone(),
        asset: asset.clone(),
        revision: parent,
    })?
    .metadata
    .revision;
    let edits = pixels
        .iter()
        .map(|value| parse_pixel(value))
        .collect::<Result<Vec<_>, _>>()?;
    let revision = patch_revision_document(PatchRevisionDocument {
        start: root,
        asset,
        parent,
        patch: PixelPatchSet {
            schema: PATCH_SCHEMA.to_owned(),
            edits,
            structure: None,
        },
        brief: None,
        actor,
    })?;
    Ok(json!({ "ok": true, "revision": revision }))
}

fn parse_pixel(value: &str) -> Result<PixelPatch, String> {
    let (coordinate, index) = value
        .split_once('=')
        .ok_or_else(|| format!("invalid pixel '{value}'; expected X,Y=INDEX"))?;
    let (x, y) = coordinate
        .split_once(',')
        .ok_or_else(|| format!("invalid coordinate '{coordinate}'; expected X,Y"))?;
    Ok(PixelPatch {
        x: x.parse()
            .map_err(|_| format!("invalid horizontal coordinate '{x}'"))?,
        y: y.parse()
            .map_err(|_| format!("invalid vertical coordinate '{y}'"))?,
        index: index
            .parse()
            .map_err(|_| format!("invalid palette index '{index}'"))?,
    })
}
