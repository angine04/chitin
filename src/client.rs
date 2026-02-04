use crate::protocol::JsonRpcResponse;
use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::env;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

struct SpinnerGuard {
    pb: ProgressBar,
}

impl SpinnerGuard {
    fn new() -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.green} {msg}")
                .expect("template"),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message("Thinking...");
        Self { pb }
    }
}

impl Drop for SpinnerGuard {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
    }
}

pub async fn run(prompt: String, pwd: String) -> Result<()> {
    // 1. Setup Spinner
    let result: Result<Vec<u8>> = {
        let _spinner = SpinnerGuard::new();

        // 2. Connect to Socket
        let config = crate::config::Config::load();
        let socket_path = config.server.socket_path;

        if !Path::new(&socket_path).exists() {
            return Err(anyhow!(
                "Chitin daemon is not running (socket not found at {socket_path})"
            ));
        }

        let mut stream = UnixStream::connect(socket_path).await?;

        // 3. Build Request
        let session_id = env::var("CHITIN_SESSION_ID")
            .or_else(|_| env::var("USER"))
            .unwrap_or_else(|_| "default".to_string());

        let request_id = get_time_id();

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "chitin.input",
            "params": {
                "prompt": prompt,
                "pwd": pwd,
                "session_id": session_id
            }
        });

        // 4. Send Request
        let request_bytes = serde_json::to_vec(&payload)?;
        stream.write_all(&request_bytes).await?;
        stream.shutdown().await?;

        // 5. Read Response
        let mut response_bytes = Vec::new();
        stream.read_to_end(&mut response_bytes).await?;

        // _spinner is dropped here automatically at end of scope 'result', clearing logic
        Ok(response_bytes)
    };

    // Process result after spinner is cleared
    let response_bytes = result?;

    if response_bytes.is_empty() {
        return Err(anyhow!("Empty response from daemon"));
    }

    let response: JsonRpcResponse = serde_json::from_slice(&response_bytes)?;

    if let Some(error) = response.error {
        eprintln!("Error: {}", error.message);
        std::process::exit(1);
    }

    if let Some(result) = response.result {
        // Output result to stdout for capture by zsh
        print!("{}", result.command);
        std::io::stdout().flush()?;
    }

    Ok(())
}

fn get_time_id() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    now.to_string()
}
