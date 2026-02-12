// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package cmd

import (
	"encoding/json"
	"fmt"

	"github.com/spf13/cobra"
)

var agentmuxCmd = &cobra.Command{
	Use:   "agentmux",
	Short: "Manage AgentMux cross-host reactive messaging",
	Long:  `Configure AgentMux for cross-host reactive messaging between agents.`,
}

var agentmuxConfigCmd = &cobra.Command{
	Use:   "config <url> <token>",
	Short: "Configure AgentMux connection at runtime",
	Long: `Sets the AgentMux URL and token for cross-host reactive messaging.
This allows updating the AgentMux configuration without restarting AgentMux.

The configuration is sent via OSC escape sequence to the terminal, which
forwards it to the AgentMux backend to start/restart the cross-host poller.

Examples:
  wsh agentmux config https://agentmux.example.com mytoken123
  wsh agentmux config clear  # Disable cross-host polling`,
	Args: cobra.RangeArgs(1, 2),
	RunE: runAgentmuxConfigCmd,
}

var agentmuxStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show how to check AgentMux poller status",
	Long: `Display instructions for checking the AgentMux poller status.

The poller status can be checked via the local HTTP endpoint:
  curl http://localhost:<port>/wave/reactive/poller/status

Or check the AgentMux logs for poller activity.`,
	Args: cobra.NoArgs,
	RunE: runAgentmuxStatusCmd,
}

func init() {
	agentmuxCmd.AddCommand(agentmuxConfigCmd)
	agentmuxCmd.AddCommand(agentmuxStatusCmd)
	rootCmd.AddCommand(agentmuxCmd)
}

func runAgentmuxConfigCmd(cmd *cobra.Command, args []string) error {
	if args[0] == "clear" {
		// Send OSC 16162 X command with empty values to disable polling
		// Format: \033]16162;X;{JSON}\007
		jsonData, _ := json.Marshal(map[string]string{
			"agentmux_url":   "",
			"agentmux_token": "",
		})
		WriteStdout("\033]16162;X;%s\007", string(jsonData))
		WriteStderr("AgentMux cross-host polling disabled\n")
		return nil
	}

	if len(args) != 2 {
		return fmt.Errorf("usage: wsh agentmux config <url> <token>\n       wsh agentmux config clear")
	}

	url := args[0]
	token := args[1]

	// Send OSC 16162 X command with the config
	// Format: \033]16162;X;{JSON}\007
	jsonData, err := json.Marshal(map[string]string{
		"agentmux_url":   url,
		"agentmux_token": token,
	})
	if err != nil {
		return fmt.Errorf("failed to encode config: %w", err)
	}

	WriteStdout("\033]16162;X;%s\007", string(jsonData))
	WriteStderr("AgentMux configured: %s\n", url)
	return nil
}

func runAgentmuxStatusCmd(cmd *cobra.Command, args []string) error {
	WriteStdout("To check AgentMux poller status, use one of these methods:\n\n")
	WriteStdout("1. Check AgentMux logs for '[reactive/poller]' messages\n")
	WriteStdout("   Log location: ~/.waveterm/waveapp.log (or ~/.waveterm-dev/ in dev mode)\n\n")
	WriteStdout("2. Use curl to query the local endpoint:\n")
	WriteStdout("   curl http://localhost:$WAVETERM_DEV_PORT/wave/reactive/poller/status\n\n")
	WriteStdout("3. Configuration can also be checked via environment variables:\n")
	WriteStdout("   echo $AGENTMUX_URL\n")
	WriteStdout("   echo $AGENTMUX_TOKEN\n")
	return nil
}
