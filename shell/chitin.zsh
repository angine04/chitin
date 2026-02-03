#!/usr/bin/env zsh

if [[ -n ${CHITIN_ZSH_LOADED:-} ]]; then
  return 0
fi
CHITIN_ZSH_LOADED=1

typeset -g CHITIN_SOCKET_PATH=${CHITIN_SOCKET_PATH:-/tmp/chitin.sock}
typeset -g CHITIN_CLIENT_TIMEOUT=${CHITIN_CLIENT_TIMEOUT:-10}

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
    print -s -- "$raw_prompt"

    local escaped_prompt=$(_chitin_json_escape "$raw_prompt")
    local escaped_pwd=$(_chitin_json_escape "$PWD")
    local session_id=${CHITIN_SESSION_ID:-$USER}
    local escaped_session=$(_chitin_json_escape "$session_id")
    local request_id=$(_chitin_request_id)

    local payload="{\"jsonrpc\":\"2.0\",\"id\":\"${request_id}\",\"method\":\"chitin.input\",\"params\":{\"prompt\":\"${escaped_prompt}\",\"pwd\":\"${escaped_pwd}\",\"session_id\":\"${escaped_session}\"}}"

    local response
    response=$(_chitin_send_json "$payload")
    if [[ -n ${CHITIN_DEBUG:-} ]]; then
      print -u2 -- "Chitin debug: response: $response"
    fi

    if [[ -n "$response" ]]; then
      local command
      command=$(print -r -- "$response" | _chitin_extract_command)
      if [[ -n "$command" ]]; then
        BUFFER=""
        CURSOR=0
        print -z -- "$command"
        zle redisplay
        return 0
      fi
    fi
    print -u2 -- "Chitin: no response from daemon"
    BUFFER="$raw_prompt"
    CURSOR=${#BUFFER}
    zle redisplay
    return 0
  fi

  zle chitin-original-accept-line
}

if [[ $- == *i* ]]; then
  if ! zle -l chitin-original-accept-line >/dev/null 2>&1; then
    zle -A accept-line chitin-original-accept-line
  fi
  zle -N accept-line _chitin_accept_line
fi
