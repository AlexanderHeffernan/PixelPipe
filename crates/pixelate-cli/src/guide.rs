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
            "Do not inspect internal project files or guess commands. Use 'pixelate asset list --root .' for discovery and 'pixelate --help' only if a listed command fails.",
            "Project image folders are real game-project folders. Pixelate never creates .gitkeep files; empty folders are not retained by Git.",
            "Reference adoption imports a verified internal source copy, hides the reference from the asset catalog, and assigns a distinct project output path without rewriting either file. Exact pixel-art adoption links the existing image and starts at the editing step.",
            "Before exact pixel-art adoption, run 'project inspect-image' and continue only when pixel_art_importable is true; larger images or images with more than 256 exact RGBA colours should use the reference conversion workflow.",
            "Every new asset needs a project-relative output path. 'asset init' defaults to the project root as '<asset-id>.png'; pass '--path <folder>/<name>.png' when another real folder is intended.",
            "Moving assets or folders refuses collisions and never changes stable asset IDs or revision history.",
            "When downloading a generated image, always choose a new descriptive filename. 'amp files get' intentionally refuses to overwrite an existing destination.",
            "For colour-only feedback, inspect the current revision palette and use 'pixelate revision recolor'. Never regenerate or replace the source just to change colours.",
            "After every conversion or edit, run 'pixelate revision preview' and inspect the resulting enlarged PNG with your vision tool before deciding the work is complete.",
            "A task is complete only after structured inspection reports a head revision and visual inspection confirms the enlarged preview is good enough. The desktop watches for external changes."
        ],
        "fast_path": "Run the steps in order without dependency installation or intermediate image processing. A flat-background source goes directly into Pixelate.",
        "steps": [
            { "action": "choose_identity", "instruction": "Choose a lowercase-hyphen asset ID and a concise creative brief." },
            { "action": "create_asset", "command": "pixelate asset init --root . --asset <asset-id> --brief '<brief>' --path <project-relative-output.png>" },
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
        "capabilities": {
            "version": "pixelate version",
            "update_cli": "pixelate update",
            "list": "pixelate asset list --root .",
            "show_project": "pixelate project show --root .",
            "project_catalog": "pixelate project catalog --root .",
            "inspect_project_image": "pixelate project inspect-image --root . --path <project-relative-image>",
            "adopt_reference_image": "pixelate asset adopt --root . --path <project-relative-reference> --destination <project-relative-output.png> --asset <asset-id> --brief '<brief>'",
            "adopt_existing_pixel_art": "pixelate asset adopt-pixel-art --root . --path <project-relative-image> --asset <asset-id> --brief '<brief>' --actor agent",
            "hide_project_image": "pixelate project hide-image --root . --path <project-relative-image>",
            "restore_project_image": "pixelate project show-image --root . --path <project-relative-image>",
            "relink_project_image": "pixelate asset relink --root . --asset <asset-id> --path <project-relative-image>",
            "accept_external_image_change": "pixelate asset update-linked-source --root . --asset <asset-id>",
            "create_real_folder": "pixelate project create-folder --root . --path <project-relative-folder>",
            "rename_or_move_real_folder": "pixelate project move-folder --root . --source <folder> --destination <folder>",
            "delete_empty_real_folder": "pixelate project delete-folder --root . --path <folder>",
            "delete_project_image": "pixelate project delete-image --root . --path <project-relative-image>",
            "move_unmanaged_project_image": "pixelate project move-image --root . --source <project-relative-image> --destination <project-relative-image>",
            "move_linked_asset": "pixelate asset move --root . --asset <asset-id> --destination <project-relative-image>",
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
            "undo_redo": "pixelate revision set-head --root . --asset <asset-id> --revision <revision-id>",
            "export_bundle": "pixelate asset export --root . --asset <asset-id> --destination <folder> --overwrite",
            "export_image": "pixelate asset export-file --root . --asset <asset-id> --destination <name.png|name.webp> --overwrite"
        }
    }))
}
