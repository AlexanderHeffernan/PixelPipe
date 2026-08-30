use std::path::Path;

use pixelate_project::ProjectStore;
use serde_json::json;

pub(crate) fn agent_guide(root: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let store = ProjectStore::discover(root)?;
    let rig_definition_example = rig_definition_example();
    let rig_mutation_examples = rig_mutation_examples();
    Ok(json!({
        "ok": true,
        "workflow": "coding_agent_sprite",
        "project_root": store.root(),
        "rules": [
            "Pixelate does not launch or manage agents. As the current coding agent, use its CLI directly.",
            "Do not install Python packages, rembg, image converters, or other dependencies. Pixelate accepts PNG, JPEG, and WebP and performs deterministic background removal itself.",
            "Use your existing image-generation or drawing tool once to create one smooth local source image, then immediately run the import, convert, inspect, and visual preview commands below.",
            "If the image has no alpha, keep one simple connected flat background and use '--background auto'. Do not remove that background yourself.",
            "Use only Pixelate CLI commands to mutate .pixelate project state; never edit manifests or revisions directly.",
            "Every asset is one ordered clip with one or more stable-ID frames sharing a canvas, palette, transparency index, and pivot. A static sprite is a one-frame clip.",
            "For a subject that can be assembled from rigid or rotating pieces, prefer the generic rig workflow. Ask the image model for one separated-parts source sheet, never an animation spritesheet: each reusable part appears exactly once, fully visible, non-overlapping, on one flat background.",
            "A Pixelate rig is generic. Do not assume humanoid anatomy or invent semantic joints. Build a connected parent hierarchy that follows the visible articulation: place pivots at visible joints, set each moving segment's parent_id to the segment it bends from, and leave parent_id null only for genuine independent roots. Disconnected nodes cannot show useful joint links or constrained reach in the desktop.",
            "For an animation that cannot be represented by reusable parts, such as smoke or an explosion, generate one reviewed still per pose and import each accepted pose in explicit order. Never ask an image generator for temporal frames arranged as a spritesheet.",
            "For multi-frame drawing and fill, pass the stable --frame ID reported by revision inspect. Shared recolour and canvas placement intentionally affect every frame.",
            "Do not inspect internal project files or guess commands. Use 'pixelate asset list --root .' for discovery and 'pixelate --help' only if a listed command fails.",
            "When downloading a generated image, always choose a new descriptive filename. 'amp files get' intentionally refuses to overwrite an existing destination.",
            "For colour-only feedback, inspect the current revision palette and use 'pixelate revision recolor'. Never regenerate or replace the source just to change colours.",
            "After every conversion or edit, run 'pixelate revision preview' and inspect the resulting enlarged PNG with your vision tool before deciding the work is complete.",
            "A task is complete only after structured inspection reports a head revision and visual inspection confirms the enlarged preview is good enough. The desktop watches for external changes."
        ],
        "fast_path": "Run the steps in order without dependency installation or intermediate image processing. A flat-background source goes directly into Pixelate.",
        "steps": [
            { "action": "choose_identity", "instruction": "Choose a lowercase-hyphen asset ID and a concise creative brief." },
            { "action": "create_asset", "command": "pixelate asset init --root . --asset <asset-id> --brief '<brief>'" },
            { "action": "create_source", "instruction": "Create a smooth PNG/JPEG/WebP source as a normal local file. Alpha is ideal; otherwise use one connected flat contrasting background. Do not pre-pixelate or remove the background. If an Amp image tool returns an attachment URL, download it with 'amp files get <url> -o <source-image>'." },
            { "action": "import_source", "command": "pixelate reference import --root . --asset <asset-id> --file <source-image>" },
            { "action": "convert", "instruction": "Use 32px when resolution is unspecified. The direct command exposes colour mood, fine tuning, and background controls through --help.", "command": "pixelate revision pixelize --root . --asset <asset-id> --resolution 32 --colors 16 --background auto --actor agent" },
            { "action": "verify_state", "command": "pixelate asset inspect --root . --asset <asset-id>", "success": "The JSON asset.head is a revision such as r000001." },
            { "action": "preview_result", "instruction": "Open the output PNG with your vision tool. It is enlarged with exact nearest-neighbour scaling so pixel edges remain faithful. Iterate with revision commands if it is not good enough.", "command": "pixelate revision preview --root . --asset <asset-id> --output /tmp/<asset-id>-preview.png" }
        ],
        "update_existing_sprite": [
            { "action": "choose_update", "instruction": "For colour-only feedback, use revision inspect followed by revision recolor. Replace the source only when the requested shape, pose, content, or silhouette changed." },
            { "action": "create_replacement_source", "instruction": "For a genuine content change, create a new smooth source image from the user's feedback." },
            { "action": "replace_and_reconvert", "command": "pixelate asset update-source --root . --asset <asset-id> --file <source-image> --actor agent" },
            { "action": "verify_state", "command": "pixelate asset inspect --root . --asset <asset-id>", "success": "asset.head changed to the newly created revision." },
            { "action": "preview_result", "command": "pixelate revision preview --root . --asset <asset-id> --output /tmp/<asset-id>-preview.png", "success": "Visually inspect the enlarged PNG before completing the task." }
        ],
        "animation_workflow": [
            { "action": "choose_authoring_route", "instruction": "Use a generic rig when motion can reuse independently moving visual parts. Use ordered full-frame references only for topology-changing organic effects. Do not silently combine the routes." },
            { "action": "plan_manual_poses", "instruction": "Plan only the important manual poses and final-to-first closure. Pixelate can deterministically insert the requested number of derived in-between frames." },
            { "action": "generate_parts_source", "instruction": "For the rig route, generate one normal high-resolution separated-parts sheet. It is source art, not a sprite or animation sheet: parts must be fully visible, non-overlapping, and consistently lit." },
            { "action": "pixelize_parts_source", "instruction": "Import and pixelize the parts sheet through the normal static workflow. Inspect the indexed result and record exact rectangles and pivots for each reusable part." },
            { "action": "write_rig_definition", "instruction": "Write pixelate.rig-definition/v1 JSON using the schema example returned in rig_definition_example. Build connected chains rather than a flat list: each child parent_id names the joint it bends from, child x/y is its fixed local reach from that parent, and pivots sit on the visible connection points. Coordinates and transforms use integer thousandths: 1000 x_millis is one pixel; 90000 rotation_millidegrees is 90 degrees; scale 1000 is 100%. Each manual pose must contain every node exactly once." },
            { "action": "create_rig", "command": "pixelate rig create --root . --asset <asset-id> --parent <head-revision> --definition <rig-definition.json> --actor agent" },
            { "action": "inspect_and_preview", "instruction": "Inspect the structured rig, then inspect the enlarged contact sheet and desktop playback. The human can select parts directly over the pixel art and correct position, rotation, scale, depth, visibility, or part assignment.", "command": "pixelate revision inspect --root . --asset <asset-id>" },
            { "action": "revise_rig", "instruction": "Write one tagged mutation from rig_mutation_examples and commit it. Always use the latest parent revision.", "command": "pixelate rig mutate --root . --asset <asset-id> --parent <head-revision> --mutation <mutation.json> --actor agent" },
            { "action": "bake_for_pixel_cleanup", "instruction": "Bake only after motion is accepted. Baking preserves rendered frames exactly and intentionally removes editable rig authoring data so ordinary per-frame pixel tools can refine the result.", "command": "pixelate rig bake --root . --asset <asset-id> --parent <head-revision> --actor agent" },
            { "action": "export_animation", "instruction": "Multi-frame export writes the canonical horizontal PNG sheet and companion timing/rectangle JSON.", "command": "pixelate asset export --root . --asset <asset-id> --destination <folder> --overwrite" }
        ],
        "rig_definition_example": rig_definition_example,
        "rig_mutation_examples": rig_mutation_examples,
        "capabilities": {
            "version": "pixelate version",
            "update_cli": "pixelate update",
            "list": "pixelate asset list --root .",
            "show_project": "pixelate project show --root .",
            "update_brief": "pixelate asset set-brief --root . --asset <asset-id> --brief '<brief>'",
            "rename": "pixelate asset rename --root . --asset <asset-id> --name '<display-name>'",
            "delete": "pixelate asset delete --root . --asset <asset-id>",
            "replace_source": "pixelate asset update-source --root . --asset <asset-id> --file <source-image>",
            "pixelize": "pixelate revision pixelize --root . --asset <asset-id> --resolution 32 --colors 16 --mood <original|warm|cool|vivid|muted> --brightness <value> --contrast <value> --saturation <value> --warmth <value> --background <auto|none|color> --actor agent",
            "canvas_placement": "pixelate revision compose --root . --asset <asset-id> --width <px> --height <px> --offset-x <px> --offset-y <px> --actor agent",
            "advanced_palette_remap": "pixelate revision remap --help",
            "inspect_colours": "pixelate revision inspect --root . --asset <asset-id>",
            "visual_preview": "pixelate revision preview --root . --asset <asset-id> --output /tmp/<asset-id>-preview.png",
            "recolor": "pixelate revision recolor --root . --asset <asset-id> --set '1=#2455D6' --set '2=#6E9CFF' --actor agent",
            "pencil_or_eraser": "pixelate revision draw --root . --asset <asset-id> --pixel '12,8=3' --pixel '13,8=3' --actor agent",
            "fill": "pixelate revision fill --root . --asset <asset-id> --x <px> --y <px> --index <palette-index> --actor agent",
            "add_blank_frame": "pixelate frame add --root . --asset <asset-id> --duration <ms>",
            "duplicate_frame": "pixelate frame duplicate --root . --asset <asset-id> --frame <frame-id>",
            "import_frame": "pixelate frame import --root . --asset <asset-id> --file <image> --position <zero-based-position>",
            "replace_frame": "pixelate frame replace --root . --asset <asset-id> --frame <frame-id> --file <replacement-image>",
            "import_image_sequence": "pixelate frame import-sequence --root . --asset <asset-id> --file <first> --file <second> --duration <ms>",
            "import_spritesheet": "pixelate frame import-sheet --root . --asset <asset-id> --file <sheet> --frame-width <px> --frame-height <px> --cell <index>",
            "delete_frame": "pixelate frame delete --root . --asset <asset-id> --frame <frame-id>",
            "reorder_frame": "pixelate frame reorder --root . --asset <asset-id> --frame <frame-id> --position <zero-based-position>",
            "set_animation_duration": "pixelate frame duration --root . --asset <asset-id> --duration <ms>",
            "rename_frame": "pixelate frame rename --root . --asset <asset-id> --frame <frame-id> --name '<pose-name>'",
            "create_generic_rig": "pixelate rig create --root . --asset <asset-id> --parent <revision> --definition <pixelate.rig-definition-v1.json>",
            "mutate_generic_rig": "pixelate rig mutate --root . --asset <asset-id> --parent <revision> --mutation <mutation.json>",
            "bake_generic_rig": "pixelate rig bake --root . --asset <asset-id> --parent <revision>",
            "undo_redo": "pixelate revision set-head --root . --asset <asset-id> --revision <revision-id>",
            "export_bundle": "pixelate asset export --root . --asset <asset-id> --destination <folder> --overwrite",
            "export_image": "pixelate asset export-file --root . --asset <asset-id> --destination <name.png|name.webp> --overwrite"
        }
    }))
}

fn rig_definition_example() -> serde_json::Value {
    json!({
        "schema": "pixelate.rig-definition/v1",
        "width": 32,
        "height": 32,
        "parts": [
            { "id": "upper-part", "source": [0, 0, 8, 12], "pivot": [4, 2] },
            { "id": "lower-part", "source": [8, 0, 7, 10], "pivot": [3, 1] }
        ],
        "nodes": [
            { "id": "upper-joint", "parent_id": null },
            { "id": "lower-joint", "parent_id": "upper-joint" }
        ],
        "poses": [
            { "id": "pose-01", "name": "Pose 1", "nodes": [
                { "node_id": "upper-joint", "part_id": "upper-part", "x_millis": 16000, "y_millis": 12000, "rotation_millidegrees": 0, "scale_x_millis": 1000, "scale_y_millis": 1000, "depth": 0, "visible": true },
                { "node_id": "lower-joint", "part_id": "lower-part", "x_millis": 0, "y_millis": 9000, "rotation_millidegrees": 0, "scale_x_millis": 1000, "scale_y_millis": 1000, "depth": 1, "visible": true }
            ] },
            { "id": "pose-02", "name": "Pose 2", "nodes": [
                { "node_id": "upper-joint", "part_id": "upper-part", "x_millis": 16000, "y_millis": 12000, "rotation_millidegrees": 15000, "scale_x_millis": 1000, "scale_y_millis": 1000, "depth": 0, "visible": true },
                { "node_id": "lower-joint", "part_id": "lower-part", "x_millis": 0, "y_millis": 9000, "rotation_millidegrees": -30000, "scale_x_millis": 1000, "scale_y_millis": 1000, "depth": 1, "visible": true }
            ] }
        ],
        "frame_duration_ms": 80,
        "interpolation": { "inbetweens": 2, "looped": true },
        "pivot": [16, 28]
    })
}

fn rig_mutation_examples() -> serde_json::Value {
    json!({
        "move_rotate_or_reassign_one_node": { "type": "update_node", "pose_id": "pose-01", "node_id": "node-a", "x_millis": 17000, "y_millis": null, "rotation_millidegrees": 12000, "scale_x_millis": null, "scale_y_millis": null, "depth": 2, "visible": null, "part_id": null },
        "swap_accidentally_reversed_parts_in_every_pose": { "type": "swap_parts", "first_node_id": "node-a", "second_node_id": "node-b" },
        "set_interpolation": { "type": "set_interpolation", "inbetweens": 2, "looped": true },
        "set_shared_duration": { "type": "set_duration", "duration_ms": 80 },
        "duplicate_pose": { "type": "duplicate_pose", "pose_id": "pose-01", "new_pose_id": "pose-02", "name": "Passing pose" },
        "delete_pose": { "type": "delete_pose", "pose_id": "pose-02" },
        "reorder_pose": { "type": "reorder_pose", "pose_id": "pose-02", "position": 0 },
        "rename_pose": { "type": "rename_pose", "pose_id": "pose-01", "name": "Contact" }
    })
}
