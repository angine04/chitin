#!/usr/bin/env zsh



typeset -g CHITIN_SOCKET_PATH=${CHITIN_SOCKET_PATH:-/tmp/chitin.sock}
typeset -g CHITIN_CLIENT_TIMEOUT=${CHITIN_CLIENT_TIMEOUT:-10}
typeset -g CHITIN_ECHO_PROMPT=${CHITIN_ECHO_PROMPT:-1}
typeset -g CHITIN_SHOW_RESPONSE=${CHITIN_SHOW_RESPONSE:-0}

_chitin_save_history() {
  local entry="$1"
  # Add to in-memory history
  print -s -- "$entry"
  # Also write directly to HISTFILE for persistence
  if [[ -n "${HISTFILE:-}" ]]; then
    # Use extended history format if EXTENDED_HISTORY is set
    if [[ -o extendedhistory ]]; then
      print -r -- ": ${EPOCHSECONDS:-$(date +%s)}:0;$entry" >> "$HISTFILE"
    else
      print -r -- "$entry" >> "$HISTFILE"
    fi
  fi
}

_chitin_json_escape() {
  local value="$1"
  value=${value//\\/\\\\}
  value=${value//"/\\"}
  value=${value//$'\n'/\\n}
  value=${value//$'\r'/\\r}
  value=${value//$'\t'/\\t}
  print -r -- "$value"
}

_chitin_request_id() {
  local base=${EPOCHREALTIME//./}
  if [[ -z $base ]]; then
    base=$RANDOM
  fi
  print -r -- "$base"
}

_chitin_send_json() {
  local payload="$1"
  if command -v nc >/dev/null 2>&1; then
    local response
    response=$(printf '%s' "$payload" | nc -U -w "$CHITIN_CLIENT_TIMEOUT" "$CHITIN_SOCKET_PATH" 2>/dev/null)
    if [[ -n "$response" ]]; then
      print -r -- "$response"
      return 0
    fi
    return 1
  fi
  if command -v socat >/dev/null 2>&1; then
    print -r -- "$payload" | socat - UNIX-CONNECT:"$CHITIN_SOCKET_PATH"
    return $?
  fi
  return 1
}

_chitin_extract_command() {
  local json
  json=$(</dev/stdin)
  if [[ $json =~ '"command"[[:space:]]*:[[:space:]]*"([^"]*)"' ]]; then
    local command="${match[1]}"
    command=${command//\\\\/\\}
    command=${command//\\"/"}
    command=${command//\\n/$'\n'}
    command=${command//\\r/$'\r'}
    command=${command//\\t/$'\t'}
    print -r -- "$command"
    return 0
  fi
  return 1
}

_chitin_accept_line() {
  if [[ "$BUFFER" == @* ]]; then
    local raw_prompt="$BUFFER"
    # Save the original prompt to history manually since we will clear the execution buffer
    _chitin_save_history "$raw_prompt"


    local escaped_prompt=$(_chitin_json_escape "$raw_prompt")
    local escaped_pwd=$(_chitin_json_escape "$PWD")
    local session_id=${CHITIN_SESSION_ID:-$USER}
    local escaped_session=$(_chitin_json_escape "$session_id")
    local request_id=$(_chitin_request_id)

    local payload="{\"jsonrpc\":\"2.0\",\"id\":\"${request_id}\",\"method\":\"chitin.input\",\"params\":{\"prompt\":\"${escaped_prompt}\",\"pwd\":\"${escaped_pwd}\",\"session_id\":\"${escaped_session}\"}}"

    zle -M "Chitin: ${raw_prompt}"
    local response
    response=$(_chitin_send_json "$payload")
    if [[ -n ${CHITIN_DEBUG:-} ]]; then
      print -u2 -- "Chitin debug: response: $response"
    fi

    if [[ -n "$response" ]]; then
      local command
      command=$(print -r -- "$response" | _chitin_extract_command)
      if [[ -n "$command" ]]; then
        # Show raw response if enabled
        if [[ "$CHITIN_SHOW_RESPONSE" == "1" ]]; then
           # We need zle -I here because we are technically still in the widget
           # before we accept the line
           zle -I
           print -r -- "Response: $response"
        fi
        
        # 1. Push the generated command to the *next* buffer stack
        print -z -- "$command"
        
        # 2. Clear current buffer to avoid executing anything (but keeps proper Prompt/PWD visual)
        # This creates an "empty" history line in scrollback, but preserves the prompt line.
        BUFFER=""
        
        # 3. Accept the current line (runs empty command, moves to next line)
        zle .accept-line
        return 0
      fi
    fi
    
    # Fallback: just accept the raw prompt (will likely error as command not found, which is fine)
    zle .accept-line
    return 0
  fi

  # Use builtin accept-line to ensure reliable execution, bypassing potential hook conflicts
  zle .accept-line
}

if [[ $- == *i* ]]; then
  # Guard against multiple bindings
  if [[ -z ${CHITIN_ZSH_LOADED:-} ]]; then
    if ! zle -l chitin-original-accept-line >/dev/null 2>&1; then
      zle -A accept-line chitin-original-accept-line
    fi
    zle -N accept-line _chitin_accept_line
    CHITIN_ZSH_LOADED=1
  fi
fi
