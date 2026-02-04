# Chitin

Native Zsh agent that turns `@` prompts into executable shell commands by refilling the next command buffer.

## Quick Start

1. Install Chitin:

```bash
cargo install --path .
```

1. Setup the background service:

```bash
# Automatically detects your OS/init system and installs the service
chitin service install
```

1. Setup the shell integration:

```bash
# Installs the shell plugin and updates your ~/.zshrc
chitin shell install

# Reload your shell
source ~/.zshrc
```

1. Type a prompt in Zsh:

```bash
@print current directory
# Should give you: pwd
```

### Manual Installation

If you prefer to configure things manually or use a different init system:

- `chitin service generate <launchd|systemd|openrc>`: Prints the service file to stdout.
- You can find the shell plugin source in `shell/chitin.zsh`.

## Supported Environments

### Shells

- ✅ Zsh
- 🚧 Bash (Planned)
- 🚧 Fish (Planned)

### Operating Systems

- ✅ macOS (via `launchd`)
- ✅ Linux (via `systemd` user services or `openrc`)

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
