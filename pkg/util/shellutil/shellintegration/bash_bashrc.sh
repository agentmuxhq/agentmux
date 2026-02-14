
# Source /etc/profile if it exists
if [ -f /etc/profile ]; then
    . /etc/profile
fi

# Detect portable mode: check if wsh exists in AgentMux app directory
if [ -n "$AGENTMUX" ] && [ "$AGENTMUX" != "1" ]; then
    APP_DIR="$(dirname "$AGENTMUX")"
    if ls "$APP_DIR"/wsh-* >/dev/null 2>&1; then
        AGENTMUX_WSHBINDIR="$APP_DIR"
    else
        AGENTMUX_WSHBINDIR={{.WSHBINDIR}}
    fi
else
    AGENTMUX_WSHBINDIR={{.WSHBINDIR}}
fi

# after /etc/profile which is likely to clobber the path
export PATH="$AGENTMUX_WSHBINDIR:$PATH"

# Source the dynamic script from wsh token
eval "$(wsh token "$AGENTMUX_SWAPTOKEN" bash 2> /dev/null)"
unset AGENTMUX_SWAPTOKEN

# Source the first of ~/.bash_profile, ~/.bash_login, or ~/.profile that exists
if [ -f ~/.bash_profile ]; then
    . ~/.bash_profile
elif [ -f ~/.bash_login ]; then
    . ~/.bash_login
elif [ -f ~/.profile ]; then
    . ~/.profile
fi

if [[ ":$PATH:" != *":$AGENTMUX_WSHBINDIR:"* ]]; then
    export PATH="$AGENTMUX_WSHBINDIR:$PATH"
fi
unset AGENTMUX_WSHBINDIR
if type _init_completion &>/dev/null; then
  source <(wsh completion bash)
fi

# shell integration
_agentmux_si_blocked() {
  [[ -n "$TMUX" || -n "$STY" || "$TERM" == tmux* || "$TERM" == screen* ]]
}

_agentmux_si_urlencode() {
  local s="$1"
  # Escape % first
  s="${s//%/%25}"
  # Common reserved characters in file paths
  s="${s// /%20}"
  s="${s//#/%23}"
  s="${s//\?/%3F}"
  s="${s//&/%26}"
  s="${s//;/%3B}"
  s="${s//+/%2B}"
  printf '%s' "$s"
}

_agentmux_si_osc7() {
  _agentmux_si_blocked && return
  local encoded_pwd=$(_agentmux_si_urlencode "$PWD")
  printf '\033]7;file://%s%s\007' "$HOSTNAME" "$encoded_pwd"
}

# Hook OSC 7 into PROMPT_COMMAND
_AGENTMUX_SI_LAST_AGENT=""

# Escape string for JSON embedding (escape backslashes and quotes)
_agentmux_si_json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"  # Escape backslashes first
  s="${s//\"/\\\"}"  # Escape quotes
  printf '%s' "$s"
}

# Send agent environment for per-pane identification (on every prompt if changed)
_agentmux_si_agent_env() {
  _agentmux_si_blocked && return
  local current_agent=""
  if [[ -n "$WAVEMUX_AGENT_ID" ]]; then
    current_agent="WAVEMUX_AGENT_ID:$WAVEMUX_AGENT_ID"
  fi
  # Only send if changed
  if [[ "$current_agent" != "$_AGENTMUX_SI_LAST_AGENT" ]]; then
    _AGENTMUX_SI_LAST_AGENT="$current_agent"
    if [[ -n "$WAVEMUX_AGENT_ID" ]]; then
      local escaped=$(_agentmux_si_json_escape "$WAVEMUX_AGENT_ID")
      printf '\033]16162;E;{"WAVEMUX_AGENT_ID":"%s"}\007' "$escaped"
    else
      # Agent was cleared - send empty object to clear metadata
      printf '\033]16162;E;{}\007'
    fi
  fi
}

_agentmux_si_prompt_command() {
  _agentmux_si_osc7
  # Send agent environment on every prompt (only if changed)
  _agentmux_si_agent_env
}

# Append _agentmux_si_prompt_command to PROMPT_COMMAND (v3-safe)
_agentmux_si_append_pc() {
  if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == "declare -a"* ]]; then
    PROMPT_COMMAND+=(_agentmux_si_prompt_command)
  else
    PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND$'\n'}_agentmux_si_prompt_command"
  fi
}
_agentmux_si_append_pc