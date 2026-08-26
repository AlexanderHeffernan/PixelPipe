mod commands;

use commands::{
    agents::{
        AgentTasks, approve_agent_connector, browse_agent_runs, cancel_agent_task,
        detect_agent_connectors, load_agent_candidate, select_agent_candidate, start_agent_task,
    },
    preferences::{recent_project, remember_project},
    project::{
        browse_project, commit_composition, convert_selected_reference, delete_asset, export_asset,
        export_asset_file, import_reference, initialize_asset, open_project, preview_composition,
        preview_selected_reference, rename_asset, store_project_palette, store_project_recipe,
        update_asset_brief,
    },
    revisions::{
        compare_revisions, fill_revision, load_revision, patch_revision, record_review,
        remap_revision, set_asset_head,
    },
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
        .manage(AgentTasks::default())
        .manage(TerminalSessions::default())
        .invoke_handler(tauri::generate_handler![
            browse_project,
            open_project,
            initialize_asset,
            delete_asset,
            update_asset_brief,
            rename_asset,
            import_reference,
            detect_agent_connectors,
            approve_agent_connector,
            export_asset,
            export_asset_file,
            convert_selected_reference,
            preview_selected_reference,
            preview_composition,
            commit_composition,
            store_project_palette,
            store_project_recipe,
            load_revision,
            compare_revisions,
            record_review,
            patch_revision,
            fill_revision,
            set_asset_head,
            remap_revision,
            browse_agent_runs,
            load_agent_candidate,
            select_agent_candidate,
            start_agent_task,
            cancel_agent_task,
            recent_project,
            remember_project,
            start_terminal,
            write_terminal,
            resize_terminal,
            close_terminal
        ])
        .run(tauri::generate_context!())
        .expect("PixelPipe desktop runtime failed");
}
