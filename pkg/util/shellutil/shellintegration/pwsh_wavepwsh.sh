# We source this file with -NoExit -File
$env:PATH = {{.WSHBINDIR_PWSH}} + "{{.PATHSEP}}" + $env:PATH

# Source dynamic script from wsh token
$waveterm_swaptoken_output = wsh token $env:WAVETERM_SWAPTOKEN pwsh 2>$null | Out-String
if ($waveterm_swaptoken_output -and $waveterm_swaptoken_output -ne "") {
    Invoke-Expression $waveterm_swaptoken_output
}
Remove-Variable -Name waveterm_swaptoken_output
Remove-Item Env:WAVETERM_SWAPTOKEN

# Load Wave completions
wsh completion powershell | Out-String | Invoke-Expression

# shell integration
function Global:_waveterm_si_blocked {
    # Check if we're in tmux or screen
    return ($env:TMUX -or $env:STY -or $env:TERM -like "tmux*" -or $env:TERM -like "screen*")
}

function Global:_waveterm_si_osc7 {
    if (_waveterm_si_blocked) { return }
    
    # Get hostname (allow empty for file:/// format)
    $hostname = $env:COMPUTERNAME
    if (-not $hostname) {
        $hostname = $env:HOSTNAME
    }
    
    # Percent-encode the raw path as-is (handles UNC, drive letters, etc.)
    $encoded_pwd = [System.Uri]::EscapeDataString($PWD.Path)
    
    # OSC 7 - current directory
    Write-Host -NoNewline "`e]7;file://$hostname/$encoded_pwd`a"
}

$Global:_WAVETERM_SI_LAST_AGENT = ""

# Escape string for JSON embedding (escape backslashes and quotes)
function Global:_waveterm_si_json_escape {
    param([string]$s)
    $s = $s.Replace('\', '\\')  # Escape backslashes first
    $s = $s.Replace('"', '\"')  # Escape quotes
    return $s
}

# Send agent environment for per-pane identification (on every prompt if changed)
function Global:_waveterm_si_agent_env {
    if (_waveterm_si_blocked) { return }

    $current_agent = ""
    if ($env:WAVEMUX_AGENT_ID) {
        $current_agent = "WAVEMUX_AGENT_ID:$env:WAVEMUX_AGENT_ID"
    } elseif ($env:AGENTMUX_AGENT_ID) {
        $current_agent = "AGENTMUX_AGENT_ID:$env:AGENTMUX_AGENT_ID"
    }

    # Only send if changed
    if ($current_agent -ne $Global:_WAVETERM_SI_LAST_AGENT) {
        $Global:_WAVETERM_SI_LAST_AGENT = $current_agent
        if ($env:WAVEMUX_AGENT_ID) {
            $escaped = _waveterm_si_json_escape $env:WAVEMUX_AGENT_ID
            Write-Host -NoNewline "`e]16162;E;{`"WAVEMUX_AGENT_ID`":`"$escaped`"}`a"
        } elseif ($env:AGENTMUX_AGENT_ID) {
            $escaped = _waveterm_si_json_escape $env:AGENTMUX_AGENT_ID
            Write-Host -NoNewline "`e]16162;E;{`"AGENTMUX_AGENT_ID`":`"$escaped`"}`a"
        } else {
            # Agent was cleared - send empty object to clear metadata
            Write-Host -NoNewline "`e]16162;E;{}`a"
        }
    }
}

# Hook OSC 7 to prompt
function Global:_waveterm_si_prompt {
    _waveterm_si_osc7
    _waveterm_si_agent_env
}

# Add the OSC 7 call to the prompt function
if (Test-Path Function:\prompt) {
    $global:_waveterm_original_prompt = $function:prompt
    function Global:prompt {
        _waveterm_si_prompt
        & $global:_waveterm_original_prompt
    }
} else {
    function Global:prompt {
        _waveterm_si_prompt
        "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
    }
}