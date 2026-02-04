#!/usr/bin/env zsh



typeset -g CHITIN_SOCKET_PATH=${CHITIN_SOCKET_PATH:-/tmp/chitin.sock}
typeset -g CHITIN_CLIENT_TIMEOUT=${CHITIN_CLIENT_TIMEOUT:-10}
typeset -g CHITIN_ECHO_PROMPT=${CHITIN_ECHO_PROMPT:-1}
typeset -g CHITIN_SHOW_RESPONSE=${CHITIN_SHOW_RESPONSE:-0}
# Define alias @=':' so that "@ command" behaves like ": command" (no-op)
alias @=':'

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

_chitin_accept_line() {
  if [[ "$BUFFER" == @* ]]; then
    local raw_prompt="$BUFFER"
    # Save the original prompt to history manually since we will clear the execution buffer
    _chitin_save_history "$raw_prompt"

    # Call the Rust client
    # The client prints the spinner to stderr and the result to stdout
    local command
    # We use 'command chitin' to ignore aliases, assuming chitin binary is in path
    if command -v chitin >/dev/null 2>&1; then
      command=$(chitin ask "$raw_prompt" --pwd "$PWD")
    else
      print -u2 "Chitin binary not found in PATH."
    fi

    if [[ -n "$command" ]]; then
       # 1. Push the generated command to the *next* buffer stack
       print -z -- "$command"
    fi
     
    # 2. Modify buffer to "@ ..." so it matches the alias @=':' and runs as no-op
    # This keeps the prompt visible on screen (as "@ print ...") without error.
    BUFFER="@ ${raw_prompt:1}"
    
    # 3. Accept the current line
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
