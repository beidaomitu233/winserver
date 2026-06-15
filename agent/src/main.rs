use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;
use winserver_agent::server::{PipeServer, RequestHandler};
use winserver_agent::managers::{
    HostsManager, PortManager, ProcessManager, SiteManager, RuntimeManager,
};
use winserver_agent::database::Database;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = env::args().collect();

    if args.contains(&"--service".to_string()) {
        info!("Windows Service mode requested; running in console mode for the current build");
    }

    run_console().await
}

async fn run_console() -> anyhow::Result<()> {
    info!("Starting WinServer Agent in console mode");

    let app_dir = env::current_dir()?;
    let data_dir = app_dir.join("data");
    std::fs::create_dir_all(&data_dir)?;

    let db = Arc::new(Database::new(&data_dir.join("winserver.db"))?);
    db.run_migrations()?;

    let port_manager = Arc::new(PortManager::new());
    let process_manager = Arc::new(ProcessManager::new(db.clone(), port_manager.clone()));
    let hosts_manager = Arc::new(HostsManager::new());
    let site_manager = Arc::new(SiteManager::new(
        data_dir.clone(),
        process_manager.clone(),
        hosts_manager.clone(),
        db.clone(),
    )?);
    let runtime_manager = Arc::new(RuntimeManager::new(data_dir.clone(), db.clone()));

    let handler = Arc::new(RequestHandler::new(
        db,
        process_manager,
        site_manager,
        port_manager,
        hosts_manager,
        runtime_manager,
        data_dir.join("runtime"),
    ));

    let server = PipeServer::new(handler);
    info!("Agent ready, starting Named Pipe server");
    server.run().await
}
