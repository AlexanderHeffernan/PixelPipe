use std::fs;

use pixelate_app::{
    BakeRig, CreateRig, MutateRig, RigDefinition, RigMutation, bake_rig, create_rig, mutate_rig,
};
use serde_json::json;

use crate::args::RigCommand;

pub(crate) fn run_rig(
    command: RigCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let result = match command {
        RigCommand::Create {
            root,
            asset,
            parent,
            source_frame,
            definition,
            actor,
        } => create_rig(CreateRig {
            start: root,
            asset,
            parent,
            source_frame_id: source_frame,
            definition: serde_json::from_slice::<RigDefinition>(&fs::read(definition)?)?,
            actor,
        })?,
        RigCommand::Mutate {
            root,
            asset,
            parent,
            mutation,
            actor,
        } => mutate_rig(MutateRig {
            start: root,
            asset,
            parent,
            action: serde_json::from_slice::<RigMutation>(&fs::read(mutation)?)?,
            actor,
        })?,
        RigCommand::Bake {
            root,
            asset,
            parent,
            actor,
        } => bake_rig(BakeRig {
            start: root,
            asset,
            parent,
            actor,
        })?,
    };
    Ok(json!({ "ok": true, "revision": result }))
}
