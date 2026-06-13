use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ServerOptions, NamedPipeServer};
use tracing::{error, info, debug};
use shared::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::handler::RequestHandler;

const PIPE_NAME: &str = r"\\.\pipe\WinServer.Core";

pub struct PipeServer {
    handler: Arc<RequestHandler>,
}

impl PipeServer {
    pub fn new(handler: Arc<RequestHandler>) -> Self {
        Self { handler }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Named Pipe server listening at {}", PIPE_NAME);

        loop {
            let server = ServerOptions::new()
                .first_pipe_instance(false)
                .create(PIPE_NAME)?;

            info!("Waiting for client connection...");
            server.connect().await?;
            info!("Client connected");

            let handler = self.handler.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(server, handler).await {
                    if e.to_string().contains("broken pipe") || e.to_string().contains("closed") {
                        info!("Client disconnected");
                    } else {
                        error!("Connection error: {}", e);
                    }
                }
            });
        }
    }
}

async fn handle_connection(
    pipe: NamedPipeServer,
    handler: Arc<RequestHandler>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = tokio::io::split(pipe);
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!("Received: {}", trimmed);

        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(request) => handler.handle(request).await,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                JsonRpcResponse::error("unknown", -32700, "Parse error")
            }
        };

        let response_json = serde_json::to_string(&response)?;
        debug!("Sending: {}", response_json);
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}
