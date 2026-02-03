# Chitin

Native Zsh agent that turns `@` prompts into executable shell commands by refilling the next command buffer.

## Quick Start

1. Build and run the daemon:

```bash
cargo run
```

2. Source the Zsh plugin:

```bash
source /path/to/chitin/shell/chitin.zsh
```

Requires either `nc` or `socat` to be available in your PATH.

3. Type a prompt in Zsh:

```bash
@find all large logs
```

## Environment

- `CHITIN_API_KEY` (required for openai provider)
- `CHITIN_API_BASE` (default: https://api.openai.com)
- `CHITIN_MODEL` (default: gpt-4.1-mini)
- `CHITIN_PROVIDER` (default: openai; also supports noop)
- `CHITIN_SOCKET_PATH` (default: /tmp/chitin.sock)
- `CHITIN_SESSION_ID` (default: $USER)

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
