use std::sync::Arc;
use tauri::State;
use crate::app_state::App;
use shared::protocol::JsonRpcRequest;
use serde_json::json;

fn make_request(method: &str, params: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method: method.to_string(),
        params,
    }
}

async fn call_handler(state: &App, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = make_request(method, params);
    let response = state.handler.handle(request).await;
    if let Some(error) = response.error {
        if let Some(data) = error.data {
            if let Some(error_code) = data.get("errorCode").and_then(|value| value.as_str()) {
                return Err(format!("{} ({})", error.message, error_code));
            }
        }
        return Err(error.message);
    }
    Ok(response.result.unwrap_or(json!({})))
}

#[tauri::command]
pub async fn get_state(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "state.get", json!({})).await
}

/// Report background auto-setup progress so the frontend can render an
/// initialization splash instead of freezing the window.
#[tauri::command]
pub async fn app_init_status(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    let phase = state.init_state.phase().await;
    let ready = state.init_state.ready();
    Ok(json!({ "phase": phase, "ready": ready }))
}

#[tauri::command]
pub async fn service_start(state: State<'_, Arc<App>>, service_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "service.start", json!({ "serviceId": service_id })).await
}

#[tauri::command]
pub async fn service_stop(state: State<'_, Arc<App>>, service_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "service.stop", json!({ "serviceId": service_id })).await
}

#[tauri::command]
pub async fn service_restart(state: State<'_, Arc<App>>, service_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "service.restart", json!({ "serviceId": service_id })).await
}

#[tauri::command]
pub async fn suite_start(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "suite.start", json!({})).await
}

#[tauri::command]
pub async fn suite_stop(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "suite.stop", json!({})).await
}

#[tauri::command]
pub async fn service_toggle_auto(state: State<'_, Arc<App>>, service_id: String, auto: bool) -> Result<serde_json::Value, String> {
    call_handler(&state, "service.toggleAuto", json!({ "serviceId": service_id, "auto": auto })).await
}

#[tauri::command]
pub async fn site_create(
    state: State<'_, Arc<App>>,
    domain: String,
    port: u16,
    path: String,
    server: String,
    php_runtime_id: Option<String>,
) -> Result<serde_json::Value, String> {
    call_handler(&state, "site.create", json!({
        "domain": domain,
        "port": port,
        "path": path,
        "server": server,
        "phpRuntimeId": php_runtime_id
    })).await
}

#[tauri::command]
pub async fn site_delete(state: State<'_, Arc<App>>, site_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "site.delete", json!({ "siteId": site_id })).await
}

#[tauri::command]
pub async fn site_update(state: State<'_, Arc<App>>, site_id: String, domain: String, port: u16, path: String, server: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "site.update", json!({ "siteId": site_id, "domain": domain, "port": port, "path": path, "server": server })).await
}

#[tauri::command]
pub async fn site_switch_php(
    state: State<'_, Arc<App>>,
    site_id: String,
    php_runtime_id: String,
) -> Result<serde_json::Value, String> {
    call_handler(&state, "site.switchPhp", json!({ "siteId": site_id, "phpRuntimeId": php_runtime_id })).await
}

#[tauri::command]
pub async fn site_enable(state: State<'_, Arc<App>>, site_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "site.enable", json!({ "siteId": site_id })).await
}

#[tauri::command]
pub async fn site_disable(state: State<'_, Arc<App>>, site_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "site.disable", json!({ "siteId": site_id })).await
}

#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "settings.get", json!({})).await
}

#[tauri::command]
pub async fn update_settings(state: State<'_, Arc<App>>, params: String) -> Result<serde_json::Value, String> {
    let params_value: serde_json::Value = serde_json::from_str(&params).map_err(|e| e.to_string())?;
    call_handler(&state, "settings.update", params_value).await
}

#[tauri::command]
pub async fn get_system_resource(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "resource.get", json!({})).await
}

#[tauri::command]
pub async fn check_port(state: State<'_, Arc<App>>, port: u16) -> Result<serde_json::Value, String> {
    call_handler(&state, "port.check", json!({ "port": port })).await
}

#[tauri::command]
pub async fn get_logs(
    state: State<'_, Arc<App>>,
    source: Option<String>,
    search: Option<String>,
) -> Result<serde_json::Value, String> {
    call_handler(&state, "log.list", json!({ "source": source, "search": search })).await
}

#[tauri::command]
pub async fn clear_logs(
    state: State<'_, Arc<App>>,
    source: Option<String>,
) -> Result<serde_json::Value, String> {
    call_handler(&state, "log.clear", json!({ "source": source })).await
}

#[tauri::command]
pub async fn get_config_file(state: State<'_, Arc<App>>, file_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "config.get", json!({ "fileId": file_id })).await
}

#[tauri::command]
pub async fn save_config_file(state: State<'_, Arc<App>>, file_id: String, content: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "config.save", json!({ "fileId": file_id, "content": content })).await
}

#[tauri::command]
pub async fn redis_config_get(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "redis.config.get", json!({})).await
}

#[tauri::command]
pub async fn redis_config_save(state: State<'_, Arc<App>>, params: String) -> Result<serde_json::Value, String> {
    let params_value: serde_json::Value = serde_json::from_str(&params).map_err(|e| e.to_string())?;
    call_handler(&state, "redis.config.save", params_value).await
}

#[tauri::command]
pub async fn minio_config_get(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "minio.config.get", json!({})).await
}

#[tauri::command]
pub async fn minio_config_save(state: State<'_, Arc<App>>, params: String) -> Result<serde_json::Value, String> {
    let params_value: serde_json::Value = serde_json::from_str(&params).map_err(|e| e.to_string())?;
    call_handler(&state, "minio.config.save", params_value).await
}

#[tauri::command]
pub async fn minio_buckets_list(
    state: State<'_, Arc<App>>,
    params: Option<String>,
) -> Result<serde_json::Value, String> {
    let params_value: serde_json::Value = match params.as_deref() {
        Some(raw) if !raw.trim().is_empty() => {
            serde_json::from_str(raw).map_err(|e| e.to_string())?
        }
        _ => json!({}),
    };
    call_handler(&state, "minio.buckets.list", params_value).await
}

#[tauri::command]
pub async fn minio_bucket_set_policy(
    state: State<'_, Arc<App>>,
    bucket: String,
    policy: String,
    root_user: Option<String>,
    root_password: Option<String>,
    api_port: Option<u16>,
) -> Result<serde_json::Value, String> {
    call_handler(
        &state,
        "minio.bucket.setPolicy",
        json!({
            "bucket": bucket,
            "policy": policy,
            "root_user": root_user,
            "root_password": root_password,
            "api_port": api_port,
        }),
    )
    .await
}

#[tauri::command]
pub async fn db_create(state: State<'_, Arc<App>>, db: String, user: String, pass: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.create", json!({ "db": db, "user": user, "pass": pass })).await
}

#[tauri::command]
pub async fn db_delete(state: State<'_, Arc<App>>, db_name: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.delete", json!({ "db": db_name })).await
}

#[tauri::command]
pub async fn db_change_password(state: State<'_, Arc<App>>, db_name: String, user: String, pass: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.changePassword", json!({ "db": db_name, "user": user, "pass": pass })).await
}

#[tauri::command]
pub async fn db_root_password(state: State<'_, Arc<App>>, current_pass: String, new_pass: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.rootPassword", json!({ "currentPass": current_pass, "pass": new_pass })).await
}

#[tauri::command]
pub async fn db_export(
    state: State<'_, Arc<App>>,
    db_name: String,
    path: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut params = json!({ "db": db_name });
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        params["path"] = json!(path);
    }
    call_handler(&state, "database.export", params).await
}

#[tauri::command]
pub async fn db_import(state: State<'_, Arc<App>>, db_name: String, path: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.import", json!({ "db": db_name, "path": path })).await
}

#[tauri::command]
pub async fn db_backups(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.backups", json!({})).await
}

#[tauri::command]
pub async fn db_delete_backup(state: State<'_, Arc<App>>, path: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "database.deleteBackup", json!({ "path": path })).await
}

#[tauri::command]
pub async fn software_install(state: State<'_, Arc<App>>, software_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "software.install", json!({ "softwareId": software_id })).await
}

#[tauri::command]
pub async fn software_uninstall(state: State<'_, Arc<App>>, software_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "software.uninstall", json!({ "softwareId": software_id })).await
}

#[tauri::command]
pub async fn software_download_install(state: State<'_, Arc<App>>, software_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "software.downloadInstall", json!({ "softwareId": software_id })).await
}

#[tauri::command]
pub async fn software_download_progress(state: State<'_, Arc<App>>, software_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "software.downloadProgress", json!({ "softwareId": software_id })).await
}

#[tauri::command]
pub async fn software_install_bundled(state: State<'_, Arc<App>>, software_id: String) -> Result<serde_json::Value, String> {
    call_handler(&state, "software.installBundled", json!({ "softwareId": software_id })).await
}

#[tauri::command]
pub async fn software_detect_local(state: State<'_, Arc<App>>) -> Result<serde_json::Value, String> {
    call_handler(&state, "software.detectLocal", json!({})).await
}

#[tauri::command]
pub async fn runtime_import(
    state: State<'_, Arc<App>>,
    runtime_type: String,
    install_path: String,
    port: Option<u16>,
) -> Result<serde_json::Value, String> {
    call_handler(
        &state,
        "runtime.import",
        json!({ "runtimeType": runtime_type, "installPath": install_path, "port": port }),
    ).await
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<serde_json::Value, String> {
    let path = path.trim().trim_start_matches(r"\\?\").to_string();
    if path.is_empty() {
        return Err("目录路径为空".into());
    }
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("目录不存在：{}", path));
    }
    // explorer requires a path; spawn without flashing a console (CREATE_NO_WINDOW).
    let mut cmd = std::process::Command::new("explorer.exe");
    cmd.arg(&path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(json!({ "message": format!("已打开: {}", path) }))
}

#[tauri::command]
pub async fn kill_process(state: State<'_, Arc<App>>, pid: u32) -> Result<serde_json::Value, String> {
    call_handler(&state, "port.killProcess", json!({ "pid": pid })).await
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<serde_json::Value, String> {
    std::process::Command::new("cmd.exe").args(["/c", "start", "", &url]).spawn().map_err(|e| e.to_string())?;
    Ok(json!({ "message": format!("已打开: {}", url) }))
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<serde_json::Value, String> {
    std::process::Command::new("notepad.exe").arg(&path).spawn().map_err(|e| e.to_string())?;
    Ok(json!({ "message": format!("已打开: {}", path) }))
}

#[tauri::command]
pub async fn kill_process(state: State<'_, Arc<App>>, pid: u32) -> Result<serde_json::Value, String> {
    call_handler(&state, "port.killProcess", json!({ "pid": pid })).await
}
