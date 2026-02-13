# add wsh to path, source dynamic script from wsh token
# Detect portable mode: check if wsh exists in AgentMux app directory
if [ -n "$WAVETERM" ]; then
    APP_DIR="$(dirname "$WAVETERM")"
    if ls "$APP_DIR"/wsh-* >/dev/null 2>&1; then
        AGENTMUX_WSHBINDIR="$APP_DIR"
    else
        AGENTMUX_WSHBINDIR={{.WSHBINDIR}}
    fi
else
    AGENTMUX_WSHBINDIR={{.WSHBINDIR}}
fi
export PATH="$AGENTMUX_WSHBINDIR:$PATH"
source <(wsh token "$AGENTMUX_SWAPTOKEN" zsh 2>/dev/null)
unset AGENTMUX_SWAPTOKEN

# Source the original zshrc only if ZDOTDIR has not been changed
if [ "$ZDOTDIR" = "$AGENTMUX_ZDOTDIR" ]; then
  [ -f ~/.zshrc ] && source ~/.zshrc
fi

if [[ ":$PATH:" != *":$AGENTMUX_WSHBINDIR:"* ]]; then
  export PATH="$AGENTMUX_WSHBINDIR:$PATH"
fi
unset AGENTMUX_WSHBINDIR

if [[ -n ${_comps+x} ]]; then
  source <(wsh completion zsh)
fi

typeset -g _AGENTMUX_SI_FIRSTPRECMD=1
typeset -g _AGENTMUX_SI_LAST_AGENT=""

# shell integration
_agentmux_si_blocked() {
  [[ -n "$TMUX" || -n "$STY" || "$TERM" == tmux* || "$TERM" == screen* ]]
}

_agentmux_si_urlencode() {
  if (( $+functions[omz_urlencode] )); then
    omz_urlencode "$1"
  else
    local s="$1"
    # Escape % first
    s=${s//%/%25}
    # Common reserved characters in file paths
    s=${s// /%20}
    s=${s//#/%23}
    s=${s//\?/%3F}
    s=${s//&/%26}
    s=${s//;/%3B}
    s=${s//+/%2B}
    printf '%s' "$s"
  fi
}

_agentmux_si_osc7() {
  _agentmux_si_blocked && return
  local encoded_pwd=$(_agentmux_si_urlencode "$PWD")
  printf '\033]7;file://%s%s\007' "$HOST" "$encoded_pwd"  # OSC 7 - current directory
}

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

_agentmux_si_precmd() {
  local _agentmux_si_status=$?
  _agentmux_si_blocked && return
  # D;status for previous command (skip before first prompt)
  if (( !_AGENTMUX_SI_FIRSTPRECMD )); then
    printf '\033]16162;D;{"exitcode":%d}\007' $_agentmux_si_status
  else
    local uname_info=$(uname -smr 2>/dev/null)
    printf '\033]16162;M;{"shell":"zsh","shellversion":"%s","uname":"%s"}\007' "$ZSH_VERSION" "$uname_info"
    _agentmux_si_osc7
  fi
  # Send agent environment on every prompt (only if changed)
  _agentmux_si_agent_env
  printf '\033]16162;A\007'      # start of new prompt
  _AGENTMUX_SI_FIRSTPRECMD=0
}

_agentmux_si_preexec() {
  _agentmux_si_blocked && return
  local cmd_length=${#1}
  if [ "$cmd_length" -gt 8192 ]; then
    local cmd64
    cmd64=$(printf '# command too large (%d bytes)' "$cmd_length" | base64 2>/dev/null | tr -d '\n\r')
    printf '\033]16162;C;{"cmd64":"%s"}\007' "$cmd64"
  else
    local cmd64
    cmd64=$(printf '%s' "$1" | base64 2>/dev/null | tr -d '\n\r')
    if [ -n "$cmd64" ]; then
      printf '\033]16162;C;{"cmd64":"%s"}\007' "$cmd64"
    else
      printf '\033]16162;C\007'
    fi
  fi
}

typeset -g AGENTMUX_SI_INPUTEMPTY=1

_agentmux_si_inputempty() {
  _agentmux_si_blocked && return
  
  local current_empty=1
  if [[ -n "$BUFFER" ]]; then
    current_empty=0
  fi
  
  if (( current_empty != AGENTMUX_SI_INPUTEMPTY )); then
    AGENTMUX_SI_INPUTEMPTY=$current_empty
    if (( current_empty )); then
      printf '\033]16162;I;{"inputempty":true}\007'
    else
      printf '\033]16162;I;{"inputempty":false}\007'
    fi
  fi
}

autoload -Uz add-zle-hook-widget 2>/dev/null
if (( $+functions[add-zle-hook-widget] )); then
  add-zle-hook-widget zle-line-init _agentmux_si_inputempty
  add-zle-hook-widget zle-line-pre-redraw _agentmux_si_inputempty
fi

autoload -U add-zsh-hook
add-zsh-hook precmd  _agentmux_si_precmd
add-zsh-hook preexec _agentmux_si_preexec
add-zsh-hook chpwd   _agentmux_si_osc7