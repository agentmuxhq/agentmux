// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package cmd

import (
	"encoding/json"
	"fmt"

	"github.com/spf13/cobra"
)

var agentbusCmd = &cobra.Command{
	Use:   "agentbus",
	Short: "Manage AgentBus cross-host reactive messaging",
	Long:  `Configure AgentBus for cross-host reactive messaging between agents.`,
}

var agentbusConfigCmd = &cobra.Command{
	Use:   "config <url> <token>",
	Short: "Configure AgentBus connection at runtime",
	Long: `Sets the AgentBus URL and token for cross-host reactive messaging.
This allows updating the AgentBus configuration without restarting WaveMux.

The configuration is sent via OSC escape sequence to the terminal, which
forwards it to the WaveMux backend to start/restart the cross-host poller.

Examples:
  wsh agentbus config https://agentmux.example.com mytoken123
  wsh agentbus config clear  # Disable cross-host polling`,
	Args: cobra.RangeArgs(1, 2),
	RunE: runAgentbusConfigCmd,
}

var agentbusStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show how to check AgentBus poller status",
	Long: `Display instructions for checking the AgentBus poller status.

The poller status can be checked via the local HTTP endpoint:
  curl http://localhost:<port>/wave/reactive/poller/status

Or check the WaveMux logs for poller activity.`,
	Args: cobra.NoArgs,
	RunE: runAgentbusStatusCmd,
}

func init() {
	agentbusCmd.AddCommand(agentbusConfigCmd)
	agentbusCmd.AddCommand(agentbusStatusCmd)
	rootCmd.AddCommand(agentbusCmd)
}

func runAgentbusConfigCmd(cmd *cobra.Command, args []string) error {
	if args[0] == "clear" {
		// Send OSC 16162 X command with empty values to disable polling
		// Format: \033]16162;X;{JSON}\007
		jsonData, _ := json.Marshal(map[string]string{
			"agentbus_url":   "",
			"agentbus_token": "",
		})
		WriteStdout("\033]16162;X;%s\007", string(jsonData))
		WriteStderr("AgentBus cross-host polling disabled\n")
		return nil
	}

	if len(args) != 2 {
		return fmt.Errorf("usage: wsh agentbus config <url> <token>\n       wsh agentbus config clear")
	}

	url := args[0]
	token := args[1]

	// Send OSC 16162 X command with the config
	// Format: \033]16162;X;{JSON}\007
	jsonData, err := json.Marshal(map[string]string{
		"agentbus_url":   url,
		"agentbus_token": token,
	})
	if err != nil {
		return fmt.Errorf("failed to encode config: %w", err)
	}

	WriteStdout("\033]16162;X;%s\007", string(jsonData))
	WriteStderr("AgentBus configured: %s\n", url)
	return nil
}

func runAgentbusStatusCmd(cmd *cobra.Command, args []string) error {
	WriteStdout("To check AgentBus poller status, use one of these methods:\n\n")
	WriteStdout("1. Check WaveMux logs for '[reactive/poller]' messages\n")
	WriteStdout("   Log location: ~/.waveterm/waveapp.log (or ~/.waveterm-dev/ in dev mode)\n\n")
	WriteStdout("2. Use curl to query the local endpoint:\n")
	WriteStdout("   curl http://localhost:$WAVETERM_DEV_PORT/wave/reactive/poller/status\n\n")
	WriteStdout("3. Configuration can also be checked via environment variables:\n")
	WriteStdout("   echo $AGENTBUS_URL\n")
	WriteStdout("   echo $AGENTBUS_TOKEN\n")
	return nil
}
