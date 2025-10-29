// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Package webhookdelivery - Integration with WaveTerm block controllers
package webhookdelivery

import (
	"context"
	"fmt"
	"log"
	"sync"

	"github.com/a5af/wavemux/pkg/wconfig"
)

// WebhookService manages webhook integration with WaveTerm
type WebhookService struct {
	client        *WebhookClient
	config        wconfig.WebhookConfigType
	terminalMap   map[string]string // terminalId => blockId
	terminalMapMu sync.RWMutex
	ctx           context.Context
	cancel        context.CancelFunc
}

var (
	globalService     *WebhookService
	globalServiceOnce sync.Once
	globalServiceMu   sync.RWMutex
)

// InitializeWebhookService initializes the global webhook service
func InitializeWebhookService() error {
	var initErr error

	globalServiceOnce.Do(func() {
		log.Printf("[WebhookService] Initializing webhook service\n")

		// Load webhook configuration
		config, err := wconfig.ReadWebhookConfig()
		if err != nil {
			initErr = fmt.Errorf("failed to read webhook config: %w", err)
			return
		}

		if !config.Enabled {
			log.Printf("[WebhookService] Webhook integration disabled\n")
			return
		}

		// Create service
		ctx, cancel := context.WithCancel(context.Background())
		service := &WebhookService{
			config:      config,
			terminalMap: make(map[string]string),
			ctx:         ctx,
			cancel:      cancel,
		}

		// Create webhook client
		client, err := NewWebhookClient(config, service.handleWebhookEvent)
		if err != nil {
			cancel()
			initErr = fmt.Errorf("failed to create webhook client: %w", err)
			return
		}

		service.client = client

		// Start client
		if err := client.Start(); err != nil {
			cancel()
			initErr = fmt.Errorf("failed to start webhook client: %w", err)
			return
		}

		globalServiceMu.Lock()
		globalService = service
		globalServiceMu.Unlock()

		log.Printf("[WebhookService] Webhook service initialized successfully\n")
	})

	return initErr
}

// GetWebhookService returns the global webhook service instance
func GetWebhookService() *WebhookService {
	globalServiceMu.RLock()
	defer globalServiceMu.RUnlock()
	return globalService
}

// ShutdownWebhookService gracefully shuts down the webhook service
func ShutdownWebhookService() error {
	globalServiceMu.Lock()
	service := globalService
	globalService = nil
	globalServiceMu.Unlock()

	if service == nil {
		return nil
	}

	log.Printf("[WebhookService] Shutting down webhook service\n")

	// Stop client
	if service.client != nil {
		if err := service.client.Stop(); err != nil {
			log.Printf("[WebhookService] Error stopping client: %v\n", err)
		}
	}

	service.cancel()

	log.Printf("[WebhookService] Webhook service shut down\n")

	return nil
}

// handleWebhookEvent processes incoming webhook events
func (s *WebhookService) handleWebhookEvent(event WebhookEvent) error {
	log.Printf("[WebhookService] Received webhook event: provider=%s, type=%s, terminalId=%s, command=%s\n",
		event.Provider, event.EventType, event.TerminalId, event.Command)

	// Check if terminal is subscribed
	if !s.config.IsTerminalSubscribed(event.TerminalId) {
		log.Printf("[WebhookService] Terminal %s not subscribed to webhooks, ignoring\n", event.TerminalId)
		return nil
	}

	// Get block ID for terminal
	s.terminalMapMu.RLock()
	blockId, exists := s.terminalMap[event.TerminalId]
	s.terminalMapMu.RUnlock()

	if !exists {
		// Use terminalId as blockId if mapping not found
		blockId = event.TerminalId
	}

	log.Printf("[WebhookService] Routing command to block %s\n", blockId)

	// TODO: Inject command into block
	// This requires integration with blockcontroller package
	// For now, just log the command that would be injected
	log.Printf("[WebhookService] Command to inject: %s\n", event.Command)

	return nil
}

// RegisterTerminal manually registers a terminal ID to block ID mapping
func (s *WebhookService) RegisterTerminal(terminalId string, blockId string) {
	s.terminalMapMu.Lock()
	s.terminalMap[terminalId] = blockId
	s.terminalMapMu.Unlock()

	log.Printf("[WebhookService] Registered terminal %s -> block %s\n", terminalId, blockId)
}

// UnregisterTerminal removes a terminal from the mapping
func (s *WebhookService) UnregisterTerminal(terminalId string) {
	s.terminalMapMu.Lock()
	delete(s.terminalMap, terminalId)
	s.terminalMapMu.Unlock()

	log.Printf("[WebhookService] Unregistered terminal %s\n", terminalId)
}

// GetStatus returns the current status of the webhook service
func (s *WebhookService) GetStatus() map[string]interface{} {
	if s == nil {
		return map[string]interface{}{
			"enabled":   false,
			"connected": false,
		}
	}

	s.terminalMapMu.RLock()
	terminalCount := len(s.terminalMap)
	s.terminalMapMu.RUnlock()

	return map[string]interface{}{
		"enabled":       s.config.Enabled,
		"connected":     s.client != nil && s.client.IsConnected(),
		"workspaceId":   s.config.WorkspaceId,
		"endpoint":      s.config.CloudEndpoint,
		"terminalCount": terminalCount,
	}
}
