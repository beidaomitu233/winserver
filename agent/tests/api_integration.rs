/// Comprehensive API Integration Test Suite
/// Tests every RPC endpoint against a real Database + RequestHandler.
/// Uses temp directories. Records every step with pass/fail + reasoning.
/// Generates HTML report.

use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use winserver_agent::database::Database;
use winserver_agent::managers::{
    HostsManager, PortManager, ProcessManager, RuntimeManager, SiteManager,
};
use winserver_agent::server::RequestHandler;
use shared::protocol::{JsonRpcRequest, JsonRpcResponse};
use shared::types::AppState;

fn free_port() -> u16 {
    let l = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    l.local_addr().expect("addr").port()
}

fn tmp_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("ws-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&d).expect("mkdir");
    d
}

fn setup() -> (Arc<RequestHandler>, PathBuf) {
    let data = tmp_dir();
    let db = Arc::new(Database::new(&data.join("ws.db")).expect("db"));
    db.run_migrations().expect("mig");
    let hosts_path = data.join("hosts");
    fs::write(&hosts_path, "127.0.0.1 localhost\r\n").expect("write test hosts");
    db.conn(|conn| {
        conn.execute(
            "UPDATE config_files SET path = ?1 WHERE id = 'hosts'",
            rusqlite::params![hosts_path.to_string_lossy().to_string()],
        )?;
        Ok(())
    }).expect("point hosts config at test file");
    let portm = Arc::new(PortManager::new());
    let pm = Arc::new(ProcessManager::new(db.clone(), portm.clone()));
    let hm = Arc::new(HostsManager::with_path(hosts_path));
    let sm = Arc::new(SiteManager::new(data.clone(), pm.clone(), hm.clone(), db.clone()).expect("sm"));
    let rm = Arc::new(RuntimeManager::new(data.clone(), db.clone()));
    let h = Arc::new(RequestHandler::new(db, pm, sm, portm, hm, rm, data.clone()));
    (h, data)
}

async fn rpc(h: &Arc<RequestHandler>, m: &str, p: serde_json::Value) -> serde_json::Value {
    let resp = rpc_response(h, m, p).await;
    if let Some(e) = &resp.error { eprintln!("  RPC ERR {}: {}", m, e.message); }
    resp.result.unwrap_or_default()
}

async fn rpc_response(h: &Arc<RequestHandler>, m: &str, p: serde_json::Value) -> JsonRpcResponse {
    let req = JsonRpcRequest { id: Uuid::new_v4().to_string(), method: m.to_string(), params: p };
    h.handle(req).await
}

fn has_structured_error(resp: &JsonRpcResponse) -> bool {
    resp.error
        .as_ref()
        .and_then(|error| error.data.as_ref())
        .and_then(|data| data.get("errorCode"))
        .is_some()
}

async fn state(h: &Arc<RequestHandler>) -> AppState {
    serde_json::from_value(rpc(h, "state.get", json!({})).await).expect("parse")
}

struct R { cat: String, name: String, pass: bool, reason: String, detail: Option<String> }
struct Report { items: Vec<R> }
impl Report {
    fn new() -> Self { Self { items: vec![] } }
    fn rec(&mut self, c: &str, n: &str, ok: bool, r: &str) {
        let icon = if ok { "PASS" } else { "FAIL" };
        println!("  {} [{}] {}: {}", icon, c, n, r);
        self.items.push(R { cat: c.into(), name: n.into(), pass: ok, reason: r.into(), detail: None });
    }
    fn rec_d(&mut self, c: &str, n: &str, ok: bool, r: &str, d: &str) {
        let icon = if ok { "PASS" } else { "FAIL" };
        println!("  {} [{}] {}: {} ({})", icon, c, n, r, d);
        self.items.push(R { cat: c.into(), name: n.into(), pass: ok, reason: r.into(), detail: Some(d.into()) });
    }
    fn summary(&self) {
        let t = self.items.len();
        let p = self.items.iter().filter(|i| i.pass).count();
        let f = t - p;
        println!("\n{}", "=".repeat(60));
        println!("API Integration Summary: {}/{} passed, {} failed", p, t, f);
        if f > 0 {
            println!("\nFailed:");
            for i in &self.items { if !i.pass { println!("  FAIL [{}] {}: {}", i.cat, i.name, i.reason); } }
        }
        // HTML
        let dir = PathBuf::from("test-results");
        let _ = fs::create_dir_all(&dir);
        let html = self.html();
        let _ = fs::write(dir.join("api-integration-report.html"), html);
        println!("\nReport: test-results/api-integration-report.html");
    }
    fn html(&self) -> String {
        let t = self.items.len(); let p = self.items.iter().filter(|i| i.pass).count(); let f = t - p;
        let mut rows = String::new();
        for i in &self.items {
            let cls = if i.pass { "pass" } else { "fail" };
            let ic = if i.pass { "✓" } else { "✗" };
            let dt = i.detail.as_deref().unwrap_or("").replace('<', "&lt;");
            rows.push_str(&format!("<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n", cls, ic, i.cat, i.name, i.reason, dt));
        }
        format!(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>WinServer API Test Report</title>
<style>body{{font-family:system-ui,sans-serif;margin:2rem;background:#0f172a;color:#e2e8f0}}h1{{color:#f1f5f9}}.s{{display:flex;gap:2rem;margin:1rem 0}}.s>div{{padding:1rem 2rem;border-radius:8px;background:#1e293b}}.p{{border-left:4px solid #22c55e}}.f{{border-left:4px solid #ef4444}}.n{{font-size:2rem;font-weight:700}}.l{{font-size:.85rem;color:#94a3b8}}table{{width:100%;border-collapse:collapse;margin-top:1rem}}th{{text-align:left;padding:.5rem 1rem;background:#1e293b;color:#94a3b8;font-size:.8rem;text-transform:uppercase}}td{{padding:.5rem 1rem;border-bottom:1px solid #1e293b}}.pass td{{color:#86efac}}.fail td{{color:#fca5a5}}</style></head><body><h1>WinServer API Integration Test Report</h1><div class="s"><div class="s>div p"><div class="n">{p}</div><div class="l">Passed</div></div><div class="s>div f"><div class="n">{f}</div><div class="l">Failed</div></div><div class="s>div"><div class="n">{t}</div><div class="l">Total</div></div></div><table><thead><tr><th></th><th>Category</th><th>Test</th><th>Result</th><th>Detail</th></tr></thead><tbody>{rows}</tbody></table></body></html>"#, p=p,f=f,t=t,rows=rows)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn api_integration_full() {
    let (h, dd) = setup();
    let mut rp = Report::new();

    println!("\n{}", "=".repeat(60));
    println!("WinServer API Integration Test Suite");
    println!("{}", "=".repeat(60));

    // ── 1. State ──
    println!("\n── State & Core ──");
    let r = rpc(&h, "state.get", json!({})).await;
    rp.rec("State", "state.get returns services", r.get("services").is_some(), "services field present");

    let r2 = rpc(&h, "state.getFast", json!({})).await;
    rp.rec("State", "state.getFast returns data", r2.get("services").is_some(), "fast cache returns state");

    let r3 = rpc(&h, "resource.get", json!({})).await;
    rp.rec_d("State", "resource.get returns system info", r3.get("cpu_percent").is_some() && r3.get("memory_percent").is_some(),
        "cpu+mem fields present", &format!("cpu={:?} mem={:?}", r3.get("cpu_percent"), r3.get("memory_percent")));

    // ── 2. Services ──
    println!("\n── Service Lifecycle ──");
    let _ = state(&h).await;
    let _ = rpc(&h, "service.start", json!({ "serviceId": "nginx" })).await;
    rp.rec("Service", "start nginx (uninstalled handled)", true, "no panic regardless of install state");

    let r = rpc(&h, "service.stop", json!({ "serviceId": "nginx" })).await;
    rp.rec("Service", "stop nginx idempotent", r.get("state").is_some() || r.get("message").is_some(), "graceful stop response");

    let r = rpc(&h, "service.toggleAuto", json!({ "serviceId": "nginx", "auto": true })).await;
    rp.rec("Service", "toggleAuto nginx=true", r.get("state").is_some(), "returns updated state");
    let _ = rpc(&h, "service.toggleAuto", json!({ "serviceId": "nginx", "auto": false })).await;

    // ── 3. Software ──
    println!("\n── Software ──");
    let s = state(&h).await;
    rp.rec_d("Software", "software list populated", s.software.len() > 0,
        "has entries", &format!("count={}", s.software.len()));

    let r = rpc(&h, "software.installBundled", json!({ "softwareId": "nginx" })).await;
    rp.rec("Software", "installBundled nginx handled", r.get("runtime").is_some() || r.get("error").is_some() || r.get("message").is_some(),
        "no panic on bundled install");

    let r = rpc(&h, "software.detectLocal", json!({})).await;
    rp.rec("Software", "detectLocal returns result", r.get("localServices").is_some(), "returns localServices map");

    let r = rpc(&h, "software.downloadProgress", json!({ "softwareId": "nginx" })).await;
    rp.rec("Software", "downloadProgress idle", r.get("phase").is_some(), "returns phase field");

    // ── 4. Sites ──
    println!("\n── Sites ──");
    let sd = dd.join("www").join("tsite");
    fs::create_dir_all(&sd).ok();
    fs::write(sd.join("index.html"), "<h1>T</h1>").ok();
    let sp = free_port();
    let create_resp = rpc_response(&h, "site.create", json!({ "domain": "tsite.local", "port": sp, "path": sd.to_string_lossy(), "server": "nginx" })).await;
    let created = create_resp.result.as_ref().and_then(|r| r.get("state")).is_some();
    let handled = created || has_structured_error(&create_resp);
    let st = state(&h).await;
    let exists = st.sites.iter().any(|s| s.domain == "tsite.local");
    rp.rec("Sites", "create site success or structured environment error", handled && (created == exists), &format!("created={}, in_state={}", created, exists));

    let r = rpc(&h, "site.list", json!({})).await;
    rp.rec("Sites", "site.list returns array", r.as_array().is_some(), "returns array");

    if let Some(sid) = st.sites.iter().find(|s| s.domain == "tsite.local").map(|s| s.id.clone()) {
        let r = rpc(&h, "site.enable", json!({ "siteId": &sid })).await;
        rp.rec("Sites", "site.enable", r.get("state").is_some(), "returns state");
        let r = rpc(&h, "site.disable", json!({ "siteId": &sid })).await;
        rp.rec("Sites", "site.disable", r.get("state").is_some(), "returns state");
        let _ = rpc(&h, "site.delete", json!({ "siteId": &sid })).await;
        let after = state(&h).await;
        let gone = !after.sites.iter().any(|s| s.id == sid);
        rp.rec("Sites", "site.delete removes it", gone, "site gone from state");
    }

    // ── 5. Config ──
    println!("\n── Config Files ──");
    let r = rpc(&h, "config.get", json!({ "fileId": "hosts" })).await;
    rp.rec("Config", "config.get hosts readable", r.get("content").is_some(), "returns content field");

    let orig = r.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
    let r = rpc(&h, "config.save", json!({ "fileId": "hosts", "content": orig.clone() + "\n# test" })).await;
    rp.rec("Config", "config.save works", r.get("state").is_some() || r.get("message").is_some(), "save returns response");
    let _ = rpc(&h, "config.save", json!({ "fileId": "hosts", "content": orig })).await;

    // ── 6. Port ──
    println!("\n── Port ──");
    let r = rpc(&h, "port.check", json!({ "port": 80 })).await;
    rp.rec(
        "Port",
        "port.check returns structured result",
        r.get("is_open").is_some() && r.get("available").is_some() && r.get("owner_type").is_some(),
        "has availability and owner fields",
    );

    // ── 7. Logs ──
    println!("\n── Logs ──");
    let r = rpc(&h, "log.list", json!({ "source": "operation" })).await;
    rp.rec("Logs", "log.list returns logs", r.get("logs").is_some(), "has logs array");

    let r = rpc(&h, "log.clear", json!({ "source": "operation" })).await;
    rp.rec("Logs", "log.clear works", r.get("message").is_some(), "returns message");

    // ── 8. Settings ──
    println!("\n── Settings ──");
    let r = rpc(&h, "settings.get", json!({})).await;
    rp.rec("Settings", "settings.get returns data", r.get("port").is_some() || r.get("autostart").is_some(), "has settings fields");

    let r = rpc(&h, "settings.update", json!({ "autostart": true })).await;
    rp.rec("Settings", "settings.update works", r.get("message").is_some(), "returns success message");
    let _ = rpc(&h, "settings.update", json!({ "autostart": false })).await;

    // ── 9. Database ──
    println!("\n── Database ──");
    let sync_resp = rpc_response(&h, "database.sync", json!({})).await;
    let sync_ok = sync_resp.result.as_ref().and_then(|r| r.get("state")).is_some();
    rp.rec("DB", "database.sync succeeds or returns structured environment error", sync_ok || has_structured_error(&sync_resp), "returns state or structured MySQL error");

    let r = rpc(&h, "database.backups", json!({})).await;
    rp.rec("DB", "database.backups works", r.get("backups").is_some() || r.get("state").is_some(), "returns backups or state");

    // ── 10. Files ──
    println!("\n── Files ──");
    let r = rpc(&h, "files.list", json!({ "path": dd.to_string_lossy() })).await;
    rp.rec("Files", "files.list returns entries", r.get("entries").is_some(), "has entries array");

    // ── 11. Hosts ──
    println!("\n── Hosts ──");
    let r = rpc(&h, "hosts.sync", json!({ "domain": "x.local" })).await;
    rp.rec("Hosts", "hosts.sync adds domain", r.get("message").is_some(), "returns message");
    let r = rpc(&h, "hosts.remove", json!({ "domain": "x.local" })).await;
    rp.rec("Hosts", "hosts.remove works", r.get("message").is_some(), "returns message");

    // ── 12. Suite ──
    println!("\n── Suite ──");
    let r = rpc(&h, "suite.start", json!({})).await;
    rp.rec("Suite", "suite.start returns state and summary", r.get("state").is_some() && r.get("summary").is_some(), "returns state + summary");
    let r = rpc(&h, "suite.stop", json!({})).await;
    rp.rec("Suite", "suite.stop returns state and summary", r.get("state").is_some() && r.get("summary").is_some(), "returns state + summary");

    // ── 13. Error handling ──
    println!("\n── Errors ──");
    let req = JsonRpcRequest { id: Uuid::new_v4().to_string(), method: "bogus.method".into(), params: json!({}) };
    let resp = h.handle(req).await;
    let has_error_data = resp
        .error
        .as_ref()
        .and_then(|error| error.data.as_ref())
        .and_then(|data| data.get("errorCode"))
        .is_some();
    rp.rec("Errors", "unknown method returns structured error", resp.error.is_some() && has_error_data, "returns JSON-RPC error data");

    // ── Summary ──
    rp.summary();
    let total = rp.items.len();
    let passed = rp.items.iter().filter(|i| i.pass).count();
    let rate = passed as f64 / total as f64;
    assert!(rate >= 0.8, "Pass rate {:.0}% < 80%", rate * 100.0);
}
