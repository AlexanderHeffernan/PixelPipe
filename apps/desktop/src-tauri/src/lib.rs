mod commands;

use commands::{
    cli_install::{cli_installation_status, install_cli, uninstall_cli},
    preferences::{recent_project, remember_project},
    project::{
        adopt_pixel_art, adopt_project_image, browse_project, commit_composition,
        convert_selected_reference, create_folder, delete_asset, delete_folder, export_asset,
        export_asset_file, import_reference, initialize_asset, load_project_image, move_asset,
        move_folder, open_project, preview_composition, preview_selected_reference, relink_asset,
        rename_asset, set_project_image_ignored, update_asset_brief, update_linked_source,
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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(TerminalSessions::default())
        .invoke_handler(tauri::generate_handler![
            browse_project,
            open_project,
            initialize_asset,
            delete_asset,
            update_asset_brief,
            rename_asset,
            adopt_project_image,
            adopt_pixel_art,
            set_project_image_ignored,
            relink_asset,
            update_linked_source,
            create_folder,
            move_folder,
            delete_folder,
            move_asset,
            load_project_image,
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
            cli_installation_status,
            install_cli,
            uninstall_cli,
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
