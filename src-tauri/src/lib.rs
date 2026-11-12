mod agents;
mod context;
mod handoff;
mod history;
mod oauth;
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
            agents::list_project_agents,
            agents::add_custom_agent,
            agents::remove_custom_agent,
            projects::list_projects,
            projects::add_project,
            projects::set_project_favorite,
            projects::set_project_archived,
            projects::list_terminal_sessions,
            projects::get_app_settings,
            projects::update_app_settings,
            projects::set_project_terminal_sync,
            projects::set_project_files_sync,
            sync::set_cloud_passphrase,
            sync::get_cloud_status,
            sync::connect_cloud,
            sync::disconnect_cloud,
            sync::test_cloud_connection,
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
            context::get_project_context,
            context::refresh_project_context_command,
            terminal::launch_terminal,
            terminal::send_terminal_input,
            terminal::resize_terminal,
            terminal::close_terminal
        ])
        .build(tauri::generate_context!())
        .expect("error while building Eureka42")
        .run(|_app_handle, _event| {
            // Padrão macOS: com a janela fechada (escondida), clicar no ícone
            // na Dock reabre a janela. ⌘Q continua encerrando o app de verdade.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                if let Some(window) = _app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}
