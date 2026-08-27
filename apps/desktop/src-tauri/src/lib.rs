mod commands;

use commands::{
    preferences::{recent_project, remember_project},
    project::{
        browse_project, commit_composition, convert_selected_reference, delete_asset, export_asset,
        export_asset_file, import_reference, initialize_asset, open_project, preview_composition,
        preview_selected_reference, rename_asset, update_asset_brief,
    },
    revisions::{fill_revision, load_revision, patch_revision, remap_revision, set_asset_head},
    terminal::{TerminalSessions, close_terminal, resize_terminal, start_terminal, write_terminal},
};

/// Starts the desktop runtime and typed command adapter.
///
/// # Panics
///
/// Panics when Tauri cannot initialize or run the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(TerminalSessions::default())
        .invoke_handler(tauri::generate_handler![
            browse_project,
            open_project,
            initialize_asset,
            delete_asset,
            update_asset_brief,
            rename_asset,
            import_reference,
            export_asset,
            export_asset_file,
            convert_selected_reference,
            preview_selected_reference,
            preview_composition,
            commit_composition,
            load_revision,
            patch_revision,
            fill_revision,
            set_asset_head,
            remap_revision,
            recent_project,
            remember_project,
            start_terminal,
            write_terminal,
            resize_terminal,
            close_terminal
        ])
        .run(tauri::generate_context!())
        .expect("Pixelate desktop runtime failed");
}
