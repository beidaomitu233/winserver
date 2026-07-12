mod app_state;
mod commands;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // Get the exe directory and resource directory from Tauri
            let app_dir = std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();

            let resource_dir = app.path().resource_dir()
                .unwrap_or_else(|_| app_dir.clone());

            let app_state = Arc::new(app_state::init_app(app_dir.clone(), resource_dir.clone()));
            let pm = app_state.process_manager.clone();

            let managed = app_state.clone();
            app.manage(app_state);

            // Start health monitor
            tauri::async_runtime::spawn(async move {
                pm.start_health_monitor_async().await;
            });

            // Kick off the heavier local-service detection + bundled runtime
            // install on a background task so the window renders immediately.
            // The frontend polls `app_init_status` and shows a splash until ready.
            app_state::run_auto_setup_async(&managed, app_dir, resource_dir);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_init_status,
            commands::get_state,
            commands::service_start,
            commands::service_stop,
            commands::service_restart,
            commands::suite_start,
            commands::suite_stop,
            commands::service_toggle_auto,
            commands::site_create,
            commands::site_delete,
            commands::site_update,
            commands::site_switch_php,
            commands::site_enable,
            commands::site_disable,
            commands::get_settings,
            commands::update_settings,
            commands::get_system_resource,
            commands::check_port,
            commands::get_logs,
            commands::clear_logs,
            commands::get_config_file,
            commands::save_config_file,
            commands::redis_config_get,
            commands::redis_config_save,
            commands::minio_config_get,
            commands::minio_config_save,
            commands::minio_buckets_list,
            commands::minio_bucket_set_policy,
            commands::db_create,
            commands::db_delete,
            commands::db_change_password,
            commands::db_root_password,
            commands::db_export,
            commands::db_import,
            commands::db_backups,
            commands::db_delete_backup,
            commands::software_install,
            commands::software_uninstall,
            commands::software_download_install,
            commands::software_download_progress,
            commands::software_install_bundled,
            commands::software_detect_local,
            commands::runtime_import,
            commands::open_folder,
            commands::kill_process,
            commands::open_url,
            commands::open_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
