// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Package wconfig - webhook configuration types
package wconfig

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/a5af/wavemux/pkg/wavebase"
)

// WebhookConfigType defines the webhook integration configuration
type WebhookConfigType struct {
	Version       string   `json:"version"`       // Config version
	WorkspaceId   string   `json:"workspaceId"`   // Unique workspace identifier
	AuthToken     string   `json:"authToken"`     // Authentication token for WebSocket
	CloudEndpoint string   `json:"cloudEndpoint"` // WebSocket endpoint URL
	Enabled       bool     `json:"enabled"`       // Enable/disable webhook integration
	Terminals     []string `json:"terminals"`     // List of terminal IDs subscribed to webhooks
}

// DefaultWebhookConfig returns the default webhook configuration
func DefaultWebhookConfig() WebhookConfigType {
	return WebhookConfigType{
		Version:       "1.0",
		WorkspaceId:   "",
		AuthToken:     "",
		CloudEndpoint: "",
		Enabled:       false,
		Terminals:     []string{},
	}
}

// ReadWebhookConfig reads the webhook configuration file
func ReadWebhookConfig() (WebhookConfigType, error) {
	configPath := filepath.Join(wavebase.GetWaveDataDir(), "webhook-config.json")

	// Return default config if file doesn't exist
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return DefaultWebhookConfig(), nil
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		return DefaultWebhookConfig(), err
	}

	var config WebhookConfigType
	if err := json.Unmarshal(data, &config); err != nil {
		return DefaultWebhookConfig(), err
	}

	// Apply defaults for missing fields
	if config.Version == "" {
		config.Version = "1.0"
	}
	if config.Terminals == nil {
		config.Terminals = []string{}
	}

	return config, nil
}

// WriteWebhookConfig writes the webhook configuration file
func WriteWebhookConfig(config WebhookConfigType) error {
	configPath := filepath.Join(wavebase.GetWaveDataDir(), "webhook-config.json")

	data, err := json.MarshalIndent(config, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(configPath, data, 0600)
}

// IsTerminalSubscribed checks if a terminal ID is subscribed to webhooks
func (c *WebhookConfigType) IsTerminalSubscribed(terminalId string) bool {
	for _, id := range c.Terminals {
		if id == terminalId {
			return true
		}
	}
	return false
}

// AddTerminal adds a terminal ID to the subscription list
func (c *WebhookConfigType) AddTerminal(terminalId string) {
	if !c.IsTerminalSubscribed(terminalId) {
		c.Terminals = append(c.Terminals, terminalId)
	}
}

// RemoveTerminal removes a terminal ID from the subscription list
func (c *WebhookConfigType) RemoveTerminal(terminalId string) {
	filtered := make([]string, 0, len(c.Terminals))
	for _, id := range c.Terminals {
		if id != terminalId {
			filtered = append(filtered, id)
		}
	}
	c.Terminals = filtered
}

// Validate checks if the webhook configuration is valid
func (c *WebhookConfigType) Validate() error {
	if c.Enabled {
		if c.WorkspaceId == "" {
			return fmt.Errorf("workspaceId is required when webhook is enabled")
		}
		if c.AuthToken == "" {
			return fmt.Errorf("authToken is required when webhook is enabled")
		}
		if c.CloudEndpoint == "" {
			return fmt.Errorf("cloudEndpoint is required when webhook is enabled")
		}
	}
	return nil
}
