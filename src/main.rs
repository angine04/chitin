mod client;
mod protocol;
mod provider;
mod session;

use anyhow::Result;
use clap::{Parser, Subcommand};
use protocol::{
    JsonRpcRequest, JsonRpcResponse, ResponseAction, internal_error, invalid_params,
    invalid_request, method_not_found,
};
use provider::{CommandGenerator, Context, build_provider_from_env};
use serde_json::Value;
use session::SessionStore;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{Duration, timeout};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const SOCKET_PATH: &str = "/tmp/chitin.sock";
const HANDSHAKE_TIMEOUT_MS: u64 = 200;

#[derive(Parser)]
#[command(name = "chitin", about = "Natural language shell assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the backend daemon
    Daemon,
    /// Ask the AI for a command
    Ask {
        /// The query/prompt
        prompt: String,
        /// Current working directory
        #[arg(long, default_value = ".")]
        pwd: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging only if daemon or no command
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Ask { prompt, pwd }) => {
            client::run(prompt, pwd).await?;
        }
        Some(Commands::Daemon) | None => {
            // Default to daemon mode
            init_logging();
            run_daemon().await?;
        }
    }

    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_ansi(true)
        .pretty()
        .init();
}

async fn run_daemon() -> Result<()> {
    init_socket()?;
    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("Chitin: listening on {SOCKET_PATH}");

    let session_store = Arc::new(Mutex::new(SessionStore::new(10)));
    let provider = Arc::from(build_provider_from_env()?);

    loop {
        let (stream, _) = listener.accept().await?;
        let store = Arc::clone(&session_store);
        let generator = Arc::clone(&provider);
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, store, generator).await {
                error!("Chitin error: {err}");
            }
        });
    }
}

fn init_socket() -> Result<()> {
    let path = Path::new(SOCKET_PATH);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

async fn handle_connection(
    mut stream: UnixStream,
    sessions: Arc<Mutex<SessionStore>>,
    generator: Arc<dyn CommandGenerator>,
) -> Result<()> {
    let mut buffer = Vec::new();
    let read_result = timeout(
        Duration::from_millis(HANDSHAKE_TIMEOUT_MS),
        stream.read_to_end(&mut buffer),
    )
    .await;
    let read_ok = match read_result {
        Ok(result) => result?,
        Err(_) => {
            send_response(
                &mut stream,
                invalid_request(Value::Null, "timeout waiting for request"),
            )
            .await?;
            return Ok(());
        }
    };

    if read_ok == 0 {
        send_response(&mut stream, invalid_request(Value::Null, "empty request")).await?;
        return Ok(());
    }

    let request: JsonRpcRequest = match serde_json::from_slice(&buffer) {
        Ok(req) => req,
        Err(err) => {
            send_response(
                &mut stream,
                invalid_request(Value::Null, format!("invalid json: {err}")),
            )
            .await?;
            return Ok(());
        }
    };

    let response = handle_request(request, sessions, generator).await;
    if let Err(err) = send_response(&mut stream, response).await {
        if is_broken_pipe(&err) {
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn handle_request(
    request: JsonRpcRequest,
    sessions: Arc<Mutex<SessionStore>>,
    generator: Arc<dyn CommandGenerator>,
) -> JsonRpcResponse {
    if request.jsonrpc != "2.0" {
        return invalid_request(request.id, "jsonrpc must be 2.0");
    }

    if request.method != "chitin.input" {
        return method_not_found(request.id, "unknown method");
    }

    if request.params.prompt.trim().is_empty() {
        return invalid_params(request.id, "prompt is required");
    }

    let session_id = request.params.session_id.clone();
    let prompt = request.params.prompt.clone();
    let pwd = request.params.pwd.clone();

    let snapshot = {
        let mut store = sessions.lock().expect("session lock");
        store.record_input(&session_id, &prompt);
        store.snapshot(&session_id)
    };

    info!("Chitin: generating command...");

    let context = Context {
        prompt,
        pwd,
        session_id,
        history: snapshot.history,
        last_command: snapshot.last_command,
    };

    match generator.generate(context).await {
        Ok(command) => {
            {
                let mut store = sessions.lock().expect("session lock");
                store.record_output(&request.params.session_id, &command);
            }
            info!("Chitin: done");
            JsonRpcResponse::success(
                request.id,
                ResponseAction {
                    action_type: "refill".to_string(),
                    command,
                },
            )
        }
        Err(err) => {
            error!("Chitin: failed - {err}");
            internal_error(request.id, err.to_string())
        }
    }
}

async fn send_response(stream: &mut UnixStream, response: JsonRpcResponse) -> Result<()> {
    let payload = serde_json::to_vec(&response)?;
    stream.write_all(&payload).await?;
    stream.shutdown().await?;
    Ok(())
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
            )
        })
        .unwrap_or(false)
}
