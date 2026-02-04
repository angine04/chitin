# Chitin

Native Zsh agent that turns `@` prompts into executable shell commands by refilling the next command buffer.

## Quick Start

1. Install Chitin (assuming you have it built or installed):

```bash
# If building from source
cargo install --path .
```

2. Generate and start the background service:

   **macOS (launchd)**
   ```bash
   chitin service launchd > ~/Library/LaunchAgents/com.user.chitin.plist
   launchctl load ~/Library/LaunchAgents/com.user.chitin.plist
   ```

   **Linux (systemd)**
   ```bash
   mkdir -p ~/.config/systemd/user
   chitin service systemd > ~/.config/systemd/user/chitin.service
   systemctl --user enable --now chitin
   ```

3. Source the Zsh plugin in your `.zshrc`:

```bash
# Point to where you have the script
source /path/to/chitin/shell/chitin.zsh
```

4. Type a prompt in Zsh:

```bash
@find all large logs
```

## Configuration

Chitin looks for a configuration file at `~/.config/chitin/config.toml` (or `XDG_CONFIG_HOME`).

You can also use environment variables to override these settings.

```toml
# ~/.config/chitin/config.toml

[server]
# Socket path for client-daemon communication
socket_path = "/tmp/chitin.sock"

[provider]
# "openai" (default), "openai-compatible", or "noop"
type = "openai"

[provider.openai]
# create your key at platform.openai.com
api_key = "sk-..." 
# defaults to gpt-4.1-mini
model = "gpt-4.1-mini"
# optional, for compatible providers (e.g. local LLMs)
# api_base = "http://localhost:8000/v1"
```

### Environment Variables

Environment variables take precedence over the config file.

- `CHITIN_API_KEY`
- `CHITIN_API_BASE`
- `CHITIN_MODEL`
- `CHITIN_PROVIDER`
- `CHITIN_SOCKET_PATH`
- `CHITIN_CONFIG` (custom path to config file)

## Protocol

Chitin speaks JSON-RPC 2.0 over a Unix socket at `/tmp/chitin.sock`.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": "123",
  "method": "chitin.input",
  "params": {
    "prompt": "@find all large logs",
    "pwd": "/Users/me",
    "session_id": "me"
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "123",
  "result": {
    "type": "refill",
    "command": "find . -size +100M"
  }
}
```
