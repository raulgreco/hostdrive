mod agents;
mod handoff;
mod history;
mod projects;
mod sync;
mod terminal;
mod workspace;

use tauri::Manager;

#[tauri::command]
fn health_check() -> &'static str {
    "ok"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let repository = projects::ProjectRepository::new(data_dir.join("hostdrive.sqlite"));
            repository.initialize()?;
            app.manage(repository);
            Ok(())
        })
        .manage(terminal::TerminalManager::default())
        .invoke_handler(tauri::generate_handler![
            health_check,
            agents::detect_agents,
            projects::list_projects,
            projects::add_project,
            projects::set_project_favorite,
            projects::set_project_archived,
            projects::list_terminal_sessions,
            projects::get_app_settings,
            projects::update_app_settings,
            projects::set_project_terminal_sync,
            sync::has_nextcloud_credentials,
            sync::set_nextcloud_credentials,
            sync::clear_nextcloud_credentials,
            sync::test_nextcloud_connection,
            sync::sync_project_now,
            workspace::list_project_memory,
            workspace::save_project_memory,
            workspace::list_project_tasks,
            workspace::add_project_task,
            workspace::set_project_task_completed,
            workspace::get_git_status,
            handoff::prepare_handoff,
            history::get_terminal_session_events,
            history::export_terminal_session,
            terminal::launch_terminal,
            terminal::send_terminal_input,
            terminal::resize_terminal,
            terminal::close_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running HostDrive");
}
