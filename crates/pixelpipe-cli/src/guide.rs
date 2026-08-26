use std::path::Path;

use pixelpipe_project::ProjectStore;
use serde_json::json;

pub(crate) fn agent_guide(root: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let store = ProjectStore::discover(root)?;
    let recipes = store
        .conversion_recipes()?
        .into_iter()
        .map(|recipe| recipe.id)
        .collect::<Vec<_>>();
    Ok(json!({
        "ok": true,
        "workflow": "coding_agent_sprite",
        "project_root": store.root(),
        "available_recipes": recipes,
        "rules": [
            "You are already the coding agent. Do not run 'pixelpipe agent run' or launch another agent.",
            "Do not install Python packages, rembg, image converters, or other dependencies. PixelPipe accepts PNG, JPEG, and WebP and performs deterministic background removal itself.",
            "Use your existing image-generation or drawing tool once to create one smooth local source image, then immediately run the import, convert, and inspect commands below.",
            "If the image has no alpha, keep one simple connected flat background and use '--background auto'. Do not remove that background yourself.",
            "Use only PixelPipe CLI commands to mutate .pixelpipe project state; never edit manifests or revisions directly.",
            "Do not inspect internal project files or guess commands. Use 'pixelpipe asset list --root .' for discovery and 'pixelpipe --help' only if a listed command fails.",
            "When downloading a generated image, always choose a new descriptive filename. 'amp files get' intentionally refuses to overwrite an existing destination.",
            "For colour-only feedback, inspect the current revision palette and use 'pixelpipe revision recolor'. Never regenerate or replace the source just to change colours.",
            "A task is complete only after 'pixelpipe asset inspect' reports a head revision. The desktop watches for these external changes."
        ],
        "fast_path": "Run the steps in order without dependency installation or intermediate image processing. A flat-background source goes directly into PixelPipe.",
        "steps": [
            { "action": "choose_identity", "instruction": "Choose a lowercase-hyphen asset ID and a concise creative brief." },
            { "action": "create_asset", "command": "pixelpipe asset init --root . --asset <asset-id> --kind sprite --brief '<brief>'" },
            { "action": "create_source", "instruction": "Create a smooth PNG/JPEG/WebP source as a normal local file. Alpha is ideal; otherwise use one connected flat contrasting background. Do not pre-pixelate or remove the background. If an Amp image tool returns an attachment URL, download it with 'amp files get <url> -o <source-image>'." },
            { "action": "import_source", "command": "pixelpipe reference import --root . --asset <asset-id> --file <source-image>" },
            { "action": "convert", "instruction": "Use 32px when resolution is unspecified. The direct command exposes colour mood, fine tuning, and background controls through --help.", "command": "pixelpipe revision pixelize --root . --asset <asset-id> --resolution 32 --colors 16 --background auto --actor agent" },
            { "action": "verify", "command": "pixelpipe asset inspect --root . --asset <asset-id>", "success": "The JSON asset.head is a revision such as r000001." }
        ],
        "update_existing_sprite": [
            { "action": "choose_update", "instruction": "For colour-only feedback, use revision inspect followed by revision recolor. Replace the source only when the requested shape, pose, content, or silhouette changed." },
            { "action": "create_replacement_source", "instruction": "For a genuine content change, create a new smooth source image from the user's feedback." },
            { "action": "replace_and_reconvert", "command": "pixelpipe asset update-source --root . --asset <asset-id> --file <source-image> --actor agent" },
            { "action": "verify", "command": "pixelpipe asset inspect --root . --asset <asset-id>", "success": "asset.head changed to the newly created revision." }
        ],
        "capabilities": {
            "list": "pixelpipe asset list --root .",
            "rename": "pixelpipe asset rename --root . --asset <asset-id> --name '<display-name>'",
            "delete": "pixelpipe asset delete --root . --asset <asset-id>",
            "replace_source": "pixelpipe asset update-source --root . --asset <asset-id> --file <source-image>",
            "pixelize": "pixelpipe revision pixelize --root . --asset <asset-id> --resolution 32 --colors 16 --mood <original|warm|cool|vivid|muted> --brightness <value> --contrast <value> --saturation <value> --warmth <value> --background <auto|none|color> --actor agent",
            "canvas_placement": "pixelpipe revision compose --root . --asset <asset-id> --width <px> --height <px> --offset-x <px> --offset-y <px> --actor agent",
            "advanced_palette_remap": "pixelpipe revision remap --help",
            "inspect_colours": "pixelpipe revision inspect --root . --asset <asset-id>",
            "recolor": "pixelpipe revision recolor --root . --asset <asset-id> --set '1=#2455D6' --set '2=#6E9CFF' --actor agent",
            "pencil_or_eraser": "pixelpipe revision draw --root . --asset <asset-id> --pixel '12,8=3' --pixel '13,8=3' --actor agent",
            "fill": "pixelpipe revision fill --root . --asset <asset-id> --x <px> --y <px> --index <palette-index> --actor agent",
            "undo_redo": "pixelpipe revision set-head --root . --asset <asset-id> --revision <revision-id>",
            "export": "pixelpipe asset export --root . --asset <asset-id> --destination <folder> --overwrite"
        }
    }))
}
