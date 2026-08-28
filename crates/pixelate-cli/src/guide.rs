use std::path::Path;

use pixelate_project::ProjectStore;
use serde_json::json;

pub(crate) fn agent_guide(root: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let store = ProjectStore::discover(root)?;
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
            "Generate animation references as one still image per pose. Never ask an image generator for a spritesheet, multi-panel image, or complete animation in one image.",
            "Plan the complete ordered motion cycle and loop closure before generation. Generate each pose using the first accepted frame as the identity anchor and the immediately previous frame as the motion anchor.",
            "Import each accepted pose with 'pixelate frame import' in reviewed playback order. This keeps the established canvas and palette fixed. Batch and explicit-grid imports are ingestion tools for existing files, not source-generation workflows.",
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
            { "action": "plan_motion", "instruction": "Write the complete ordered pose list before generating anything: contact, down, passing, up, opposite contact, and the matching return phases as appropriate. Include the final-to-first transition." },
            { "action": "create_first_pose", "instruction": "Generate one smooth still image, convert it through the normal static workflow, and accept it as the permanent identity, scale, lighting, camera, and ground-line anchor." },
            { "action": "create_next_pose", "instruction": "Generate exactly one smooth still. Reference both the first accepted pose for identity and the immediately previous pose for a small intentional motion step. Keep non-moving details, lighting, camera, scale, pivot, and ground line unchanged." },
            { "action": "import_next_pose", "command": "pixelate frame import --root . --asset <asset-id> --file <next-pose.png> --position <zero-based-position>" },
            { "action": "inspect_frames", "command": "pixelate revision inspect --root . --asset <asset-id>", "success": "frames lists stable IDs, names, and durations in playback order; motion.transitions reports exact changed, silhouette, opaque-colour, and overlap pixel counts; motion.warnings is empty." },
            { "action": "reject_inconsistent_pose", "instruction": "Treat every motion.warning as a failed consistency gate. Regenerate the warning's to_frame_id pose, replace it with the command below, then inspect again. Do not export or describe the animation as complete while warnings remain. If intentionally broad motion cannot clear a warning, stop and ask the human to review it; never silently ignore the warning or generate a retry without importing it.", "command": "pixelate frame replace --root . --asset <asset-id> --frame <to-frame-id> --file <replacement-pose.png>" },
            { "action": "set_timing", "instruction": "Set one uniform frame duration for the complete clip.", "command": "pixelate frame duration --root . --asset <asset-id> --duration 100" },
            { "action": "inspect_motion", "instruction": "The preview is a nearest-neighbour horizontal contact sheet; inspect it and use desktop playback to judge cadence and final-to-first closure. A contact sheet alone does not prove smooth timing.", "command": "pixelate revision preview --root . --asset <asset-id> --output /tmp/<asset-id>-frames.png" },
            { "action": "export_animation", "instruction": "Multi-frame export writes the canonical horizontal PNG sheet and companion timing/rectangle JSON.", "command": "pixelate asset export --root . --asset <asset-id> --destination <folder> --overwrite" }
        ],
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
            "undo_redo": "pixelate revision set-head --root . --asset <asset-id> --revision <revision-id>",
            "export_bundle": "pixelate asset export --root . --asset <asset-id> --destination <folder> --overwrite",
            "export_image": "pixelate asset export-file --root . --asset <asset-id> --destination <name.png|name.webp> --overwrite"
        }
    }))
}
