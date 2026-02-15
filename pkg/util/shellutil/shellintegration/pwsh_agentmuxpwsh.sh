# ============================================================================
# AgentMux Shell Integration for PowerShell
# ============================================================================
# Generated Version: {{.AGENTMUX_VERSION}}
# Template Version: 3
# Generated: {{.TIMESTAMP}}
# DO NOT EDIT - This file is auto-generated
# ============================================================================

# We source this file with: pwsh -NoExit -File <this-file>

# ----------------------------------------------------------------------------
# 1. VERSION GUARD
# ----------------------------------------------------------------------------
$AGENTMUX_SHELL_VERSION = "{{.AGENTMUX_VERSION}}"
$AGENTMUX_TEMPLATE_VERSION = 3

# Warn if file is stale (optional, non-breaking)
if ($env:AGENTMUX_VERSION -and $env:AGENTMUX_VERSION -ne $AGENTMUX_SHELL_VERSION) {
    Write-Host "[AgentMux] Shell integration outdated (file: $AGENTMUX_SHELL_VERSION, running: $env:AGENTMUX_VERSION)" -ForegroundColor Yellow
    Write-Host "[AgentMux] Restart AgentMux to regenerate" -ForegroundColor Yellow
}

# ----------------------------------------------------------------------------
# 2. BINARY DISCOVERY (Multi-Strategy)
# ----------------------------------------------------------------------------

# Strategy 1: Template-injected binary directory (most reliable)
$wshBinaryDir = {{.WSHBINDIR_PWSH}}

# Strategy 2: Portable mode (check for ./bin/ subdirectory)
$portableBinDir = $null
if ($env:AGENTMUX -and (Test-Path $env:AGENTMUX -PathType Leaf)) {
    $appDir = Split-Path -Parent $env:AGENTMUX
    $candidateDir = Join-Path $appDir "bin"
    if (Test-Path $candidateDir -PathType Container) {
        # Validate: ensure wsh binary exists
        $wshTest = Get-ChildItem -Path $candidateDir -Filter "wsh*.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($wshTest) {
            $portableBinDir = $candidateDir
        }
    }
}

# Strategy 3: Already in PATH (globally installed)
$wshInPath = $null
$wshCommand = Get-Command wsh -ErrorAction SilentlyContinue
if ($wshCommand) {
    $wshInPath = Split-Path -Parent $wshCommand.Source
}

# Select best strategy (priority: portable > installed > PATH)
$selectedWshDir = $null
if ($portableBinDir) {
    $selectedWshDir = $portableBinDir
} elseif (Test-Path $wshBinaryDir -PathType Container) {
    $selectedWshDir = $wshBinaryDir
} elseif ($wshInPath) {
    $selectedWshDir = $wshInPath
}

# ----------------------------------------------------------------------------
# 3. PATH SETUP
# ----------------------------------------------------------------------------

if ($selectedWshDir) {
    # Prepend to PATH (only if not already present)
    if ($env:PATH -notlike "*$selectedWshDir*") {
        $env:PATH = $selectedWshDir + "{{.PATHSEP}}" + $env:PATH
    }
} else {
    Write-Verbose "[AgentMux] wsh binary not found - some features will be unavailable"
}

# ----------------------------------------------------------------------------
# 4. HELPER FUNCTIONS
# ----------------------------------------------------------------------------

function Test-WshAvailable {
    $cmd = Get-Command wsh -ErrorAction SilentlyContinue
    return ($null -ne $cmd)
}

# ----------------------------------------------------------------------------
# 5. TOKEN SWAP (Dynamic Shell Configuration)
# ----------------------------------------------------------------------------

if (Test-WshAvailable) {
    if ($env:AGENTMUX_SWAPTOKEN) {
        try {
            $agentmux_swaptoken_output = wsh token $env:AGENTMUX_SWAPTOKEN pwsh 2>$null | Out-String
            if ($agentmux_swaptoken_output -and $agentmux_swaptoken_output.Trim() -ne "") {
                Invoke-Expression $agentmux_swaptoken_output
            }
        } catch {
            Write-Verbose "[AgentMux] Token swap failed: $_"
        }

        # Cleanup
        Remove-Variable -Name agentmux_swaptoken_output -ErrorAction SilentlyContinue
        if (Test-Path Env:AGENTMUX_SWAPTOKEN) {
            Remove-Item Env:AGENTMUX_SWAPTOKEN -ErrorAction SilentlyContinue
        }
    }
}

# ----------------------------------------------------------------------------
# 6. LOAD COMPLETIONS
# ----------------------------------------------------------------------------

if (Test-WshAvailable) {
    try {
        $completions = wsh completion powershell 2>$null | Out-String
        if ($completions -and $completions.Trim() -ne "") {
            Invoke-Expression $completions
        }
    } catch {
        Write-Verbose "[AgentMux] Completion loading failed: $_"
    }
}

# ----------------------------------------------------------------------------
# 7. SHELL INTEGRATION FEATURES
# ----------------------------------------------------------------------------

# tmux/screen detection
function Global:_agentmux_si_blocked {
    return ($env:TMUX -or $env:STY -or $env:TERM -like "tmux*" -or $env:TERM -like "screen*")
}

# OSC 7 directory tracking
function Global:_agentmux_si_osc7 {
    if (_agentmux_si_blocked) { return }

    # Get hostname (allow empty for file:/// format)
    $hostname = $env:COMPUTERNAME
    if (-not $hostname) {
        $hostname = $env:HOSTNAME
    }
    if (-not $hostname) {
        $hostname = ""
    }

    # Percent-encode the raw path as-is (handles UNC, drive letters, etc.)
    $encoded_pwd = [System.Uri]::EscapeDataString($PWD.Path)

    # OSC 7 - current directory
    Write-Host -NoNewline "`e]7;file://$hostname/$encoded_pwd`a"
}

$Global:_AGENTMUX_SI_LAST_AGENT = ""

# Escape string for JSON embedding (escape backslashes and quotes)
function Global:_agentmux_si_json_escape {
    param([string]$s)
    $s = $s.Replace('\', '\\')  # Escape backslashes first
    $s = $s.Replace('"', '\"')  # Escape quotes
    return $s
}

# Send agent environment for per-pane identification (on every prompt if changed)
function Global:_agentmux_si_agent_env {
    if (_agentmux_si_blocked) { return }

    $current_agent = ""
    if ($env:WAVEMUX_AGENT_ID) {
        $current_agent = "WAVEMUX_AGENT_ID:$env:WAVEMUX_AGENT_ID"
    }

    # Only send if changed
    if ($current_agent -ne $Global:_AGENTMUX_SI_LAST_AGENT) {
        $Global:_AGENTMUX_SI_LAST_AGENT = $current_agent
        if ($env:WAVEMUX_AGENT_ID) {
            $escaped = _agentmux_si_json_escape $env:WAVEMUX_AGENT_ID
            Write-Host -NoNewline "`e]16162;E;{`"WAVEMUX_AGENT_ID`":`"$escaped`"}`a"
        } else {
            # Agent was cleared - send empty object to clear metadata
            Write-Host -NoNewline "`e]16162;E;{}`a"
        }
    }
}

# Hook OSC 7 to prompt
function Global:_agentmux_si_prompt {
    _agentmux_si_osc7
    _agentmux_si_agent_env
}

# Add the OSC 7 call to the prompt function
if (Test-Path Function:\prompt) {
    $global:_agentmux_original_prompt = $function:prompt
    function Global:prompt {
        _agentmux_si_prompt
        & $global:_agentmux_original_prompt
    }
} else {
    function Global:prompt {
        _agentmux_si_prompt
        "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
    }
}

# ----------------------------------------------------------------------------
# END OF AGENTMUX SHELL INTEGRATION
# ----------------------------------------------------------------------------
