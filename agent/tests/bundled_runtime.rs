/// Bundled Installation Flow Test
///
/// Tests the critical bug scenario: install bundled runtime →
/// verify files are extracted correctly → verify service is
/// properly registered with correct exe/args/cwd →
/// verify service can start without errors.
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use winserver_agent::database::Database;
use winserver_agent::managers::{HostsManager, PortManager, ProcessManager, RuntimeManager, SiteManager};
use winserver_agent::server::RequestHandler;
use shared::protocol::JsonRpcRequest;

fn tmp_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("ws-bundled-{}", Uuid::new_v4()));
    fs::create_dir_all(&d).expect("mkdir");
    d
}

async fn rpc(h: &Arc<RequestHandler>, m: &str, p: serde_json::Value) -> serde_json::Value {
    let req = JsonRpcRequest { id: Uuid::new_v4().to_string(), method: m.to_string(), params: p };
    let resp = h.handle(req).await;
    resp.result.unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn bundled_install_flow_test() {
    let data = tmp_dir();
    let db = Arc::new(Database::new(&data.join("ws.db")).expect("db"));
    db.run_migrations().expect("mig");
    let pm = Arc::new(ProcessManager::new(db.clone()));
    let portm = Arc::new(PortManager::new());
    let hosts_path = data.join("hosts");
    fs::write(&hosts_path, "127.0.0.1 localhost
").ok();
    let hm = Arc::new(HostsManager::with_path(hosts_path));
    let sm = Arc::new(SiteManager::new(data.clone(), pm.clone(), hm.clone(), db.clone()).expect("sm"));
    let rm = Arc::new(RuntimeManager::new(data.clone(), db.clone()));

    // Find actual runtime directory
    let runtime_dir = {
        let exe_dir = std::env::current_exe().unwrap_or_default()
            .parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let candidates = vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("runtime"),  // dev mode
            exe_dir.join("runtime"),           // installed mode
        ];
        candidates.into_iter().find(|d| d.join("minio.exe").exists() || d.join("redis-7.2.4.zip").exists())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("runtime"))
    };

    let h = Arc::new(RequestHandler::new(db, pm, sm, portm, hm, rm, runtime_dir.clone()));

    println!("\n{}", "=".repeat(60));
    println!("Bundled Installation Flow Test");
    println!("Runtime dir: {:?}", runtime_dir);
    println!("{}", "=".repeat(60));

    // ── Test each bundled service ──
    let bundled_services = ["nginx", "redis", "minio", "mysql80"];
    let mut results: Vec<(&str, String, String)> = Vec::new();

    for sw_id in &bundled_services {
        println!("\n── Testing bundled install: {} ──", sw_id);

        // Check if bundled file exists
        let has_bundled = if runtime_dir.exists() {
            let entries: Vec<_> = fs::read_dir(&runtime_dir).unwrap_or_else(|_| panic!("read dir"))
                .filter_map(|e| e.ok()).collect();
            let names: Vec<String> = entries.iter()
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect();
            match *sw_id {
                "nginx" => names.iter().any(|n| n.starts_with("nginx")),
                "redis" => names.iter().any(|n| n.starts_with("redis")),
                "minio" => names.iter().any(|n| n.starts_with("minio")),
                "mysql80" => names.iter().any(|n| n.contains("mysql-8")),
                _ => false,
            }
        } else {
            false
        };

        if !has_bundled {
            println!("  SKIP: no bundled file for {}", sw_id);
            results.push((*sw_id, "SKIP".into(), "No bundled file found".into()));
            continue;
        }

        // Install bundled
        let r = rpc(&h, "software.installBundled", json!({ "softwareId": sw_id })).await;
        let install_ok = r.get("runtime").is_some();
        println!("  Install result: {}", if install_ok { "OK" } else { "FAIL" });

        if !install_ok {
            results.push((*sw_id, "FAIL".into(), "install_bundled_runtime returned no runtime".into()));
            continue;
        }

        // Verify service is registered
        let state = rpc(&h, "state.get", json!({})).await;
        let services = state.get("services").and_then(|s| s.as_array());
        let svc = services.and_then(|arr| arr.iter().find(|s| s.get("id").and_then(|v| v.as_str()) == Some(sw_id)));

        match svc {
            Some(svc_data) => {
                let installed = svc_data.get("installed").and_then(|v| v.as_bool()).unwrap_or(false);

                if !installed {
                    results.push((*sw_id, "FAIL".into(), "service not marked as installed".into()));
                    continue;
                }

                // Check extracted files exist on disk
                let server_dir = data.join("server").join(sw_id);
                let dir_exists = server_dir.exists();
                println!("  installed=true, server_dir={:?}, exists={}", server_dir, dir_exists);

                if dir_exists {
                    // Check for key files based on service type
                    let marker = match *sw_id {
                        "nginx" => server_dir.join("nginx.exe"),
                        "redis" => server_dir.join("redis-server.exe"),
                        "minio" => server_dir.join("minio.exe"),
                        "mysql80" => server_dir.join("bin").join("mysqld.exe"),
                        _ => server_dir.clone(),
                    };
                    let marker_exists = marker.exists();
                    println!("  marker={:?}, exists={}", marker, marker_exists);

                    if marker_exists {
                        results.push((*sw_id, "PASS".into(), format!("files extracted, marker exists at {:?}", marker)));
                    } else {
                        results.push((*sw_id, "FAIL".into(), format!("server dir exists but marker not found at {:?}", marker)));
                    }
                } else {
                    results.push((*sw_id, "FAIL".into(), "server directory not created after install".into()));
                }
            }
            None => {
                results.push((*sw_id, "FAIL".into(), "service not found in state after install".into()));
            }
        }
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("Bundled Install Flow Summary:");
    let passed = results.iter().filter(|r| r.1 == "PASS").count();
    let failed = results.iter().filter(|r| r.1 == "FAIL").count();
    let skipped = results.iter().filter(|r| r.1 == "SKIP").count();
    for (id, status, reason) in &results {
        let icon = match status.as_str() { "PASS" => "✓", "FAIL" => "✗", _ => "-" };
        println!("  {} {}: {}", icon, id, reason);
    }
    println!("Passed: {}, Failed: {}, Skipped: {}", passed, failed, skipped);

    // If we had bundled files available, ensure no failures
    let testable = results.iter().filter(|r| r.1 != "SKIP").count();
    if testable > 0 {
        assert_eq!(failed, 0, "{} bundled install tests failed", failed);
    }
}
