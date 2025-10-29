// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Package webhookdelivery - WebSocket client for webhook command delivery
package webhookdelivery

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/url"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/a5af/wavemux/pkg/wconfig"
)

// WebhookEvent represents an event received from the webhook cloud
type WebhookEvent struct {
	EventType    string                 `json:"eventType"`    // Type of event (e.g., "pull_request")
	Provider     string                 `json:"provider"`     // Provider (e.g., "github")
	TerminalId   string                 `json:"terminalId"`   // Target terminal ID
	Command      string                 `json:"command"`      // Rendered command to inject
	Timestamp    int64                  `json:"timestamp"`    // Event timestamp
	WorkspaceId  string                 `json:"workspaceId"`  // Workspace ID
	RawData      map[string]interface{} `json:"rawData"`      // Original webhook data
	ConnectionId string                 `json:"connectionId"` // WebSocket connection ID
}

// CommandHandler is called when a webhook event is received
type CommandHandler func(event WebhookEvent) error

// WebhookClient manages the WebSocket connection to the cloud endpoint
type WebhookClient struct {
	config         wconfig.WebhookConfigType
	conn           *websocket.Conn
	connMu         sync.RWMutex
	commandHandler CommandHandler
	ctx            context.Context
	cancel         context.CancelFunc
	reconnectCh    chan struct{}
	doneCh         chan struct{}
	connected      bool
	connectedMu    sync.RWMutex
}

const (
	// WebSocket connection timeouts
	writeWait      = 10 * time.Second
	pongWait       = 60 * time.Second
	pingPeriod     = 50 * time.Second
	maxMessageSize = 10 * 1024 * 1024 // 10MB

	// Reconnection backoff
	initialReconnectDelay = 1 * time.Second
	maxReconnectDelay     = 2 * time.Minute
	reconnectBackoffRate  = 2.0
)

// NewWebhookClient creates a new webhook client
func NewWebhookClient(config wconfig.WebhookConfigType, handler CommandHandler) (*WebhookClient, error) {
	if err := config.Validate(); err != nil {
		return nil, fmt.Errorf("invalid webhook config: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	client := &WebhookClient{
		config:         config,
		commandHandler: handler,
		ctx:            ctx,
		cancel:         cancel,
		reconnectCh:    make(chan struct{}, 1),
		doneCh:         make(chan struct{}),
		connected:      false,
	}

	return client, nil
}

// Start begins the WebSocket connection and message processing
func (c *WebhookClient) Start() error {
	if !c.config.Enabled {
		log.Printf("[WebhookClient] Webhook integration disabled in config\n")
		return nil
	}

	log.Printf("[WebhookClient] Starting webhook client for workspace: %s\n", c.config.WorkspaceId)

	// Start connection loop in background
	go c.connectionLoop()

	return nil
}

// Stop gracefully shuts down the WebSocket connection
func (c *WebhookClient) Stop() error {
	log.Printf("[WebhookClient] Stopping webhook client\n")

	c.cancel()

	c.connMu.Lock()
	if c.conn != nil {
		// Send close message
		c.conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
		c.conn.Close()
		c.conn = nil
	}
	c.connMu.Unlock()

	// Wait for goroutines to finish
	select {
	case <-c.doneCh:
	case <-time.After(5 * time.Second):
		log.Printf("[WebhookClient] Timeout waiting for shutdown\n")
	}

	return nil
}

// IsConnected returns true if the WebSocket connection is active
func (c *WebhookClient) IsConnected() bool {
	c.connectedMu.RLock()
	defer c.connectedMu.RUnlock()
	return c.connected
}

// connectionLoop manages connection lifecycle with reconnection
func (c *WebhookClient) connectionLoop() {
	defer close(c.doneCh)

	reconnectDelay := initialReconnectDelay

	for {
		select {
		case <-c.ctx.Done():
			return
		default:
		}

		// Attempt connection
		if err := c.connect(); err != nil {
			log.Printf("[WebhookClient] Connection failed: %v, retrying in %v\n", err, reconnectDelay)

			// Wait before reconnecting
			select {
			case <-c.ctx.Done():
				return
			case <-time.After(reconnectDelay):
			}

			// Exponential backoff
			reconnectDelay = time.Duration(float64(reconnectDelay) * reconnectBackoffRate)
			if reconnectDelay > maxReconnectDelay {
				reconnectDelay = maxReconnectDelay
			}

			continue
		}

		// Connection established, reset backoff
		reconnectDelay = initialReconnectDelay

		// Run read/write loops
		c.runConnection()

		// Connection closed, wait before reconnecting
		select {
		case <-c.ctx.Done():
			return
		case <-time.After(reconnectDelay):
		}
	}
}

// connect establishes WebSocket connection to cloud endpoint
func (c *WebhookClient) connect() error {
	// Build WebSocket URL with query parameters
	u, err := url.Parse(c.config.CloudEndpoint)
	if err != nil {
		return fmt.Errorf("invalid cloud endpoint URL: %w", err)
	}

	q := u.Query()
	q.Set("workspaceId", c.config.WorkspaceId)
	q.Set("token", c.config.AuthToken)
	u.RawQuery = q.Encode()

	log.Printf("[WebhookClient] Connecting to: %s\n", u.String())

	// Create WebSocket connection
	conn, _, err := websocket.DefaultDialer.Dial(u.String(), nil)
	if err != nil {
		return fmt.Errorf("dial failed: %w", err)
	}

	// Configure connection
	conn.SetReadLimit(maxMessageSize)
	conn.SetReadDeadline(time.Now().Add(pongWait))
	conn.SetPongHandler(func(string) error {
		conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	c.connMu.Lock()
	c.conn = conn
	c.connMu.Unlock()

	c.setConnected(true)

	log.Printf("[WebhookClient] Connected successfully\n")

	return nil
}

// runConnection runs read/write loops for active connection
func (c *WebhookClient) runConnection() {
	var wg sync.WaitGroup

	// Read loop
	wg.Add(1)
	go func() {
		defer wg.Done()
		c.readLoop()
	}()

	// Ping loop
	wg.Add(1)
	go func() {
		defer wg.Done()
		c.pingLoop()
	}()

	wg.Wait()

	// Clean up connection
	c.connMu.Lock()
	if c.conn != nil {
		c.conn.Close()
		c.conn = nil
	}
	c.connMu.Unlock()

	c.setConnected(false)

	log.Printf("[WebhookClient] Connection closed\n")
}

// readLoop reads messages from WebSocket
func (c *WebhookClient) readLoop() {
	for {
		select {
		case <-c.ctx.Done():
			return
		default:
		}

		c.connMu.RLock()
		conn := c.conn
		c.connMu.RUnlock()

		if conn == nil {
			return
		}

		_, message, err := conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseNormalClosure) {
				log.Printf("[WebhookClient] Read error: %v\n", err)
			}
			return
		}

		// Parse webhook event
		var event WebhookEvent
		if err := json.Unmarshal(message, &event); err != nil {
			log.Printf("[WebhookClient] Failed to parse event: %v\n", err)
			continue
		}

		// Handle event
		if c.commandHandler != nil {
			go func() {
				if err := c.commandHandler(event); err != nil {
					log.Printf("[WebhookClient] Command handler error: %v\n", err)
				}
			}()
		}
	}
}

// pingLoop sends periodic ping messages
func (c *WebhookClient) pingLoop() {
	ticker := time.NewTicker(pingPeriod)
	defer ticker.Stop()

	for {
		select {
		case <-c.ctx.Done():
			return
		case <-ticker.C:
			c.connMu.RLock()
			conn := c.conn
			c.connMu.RUnlock()

			if conn == nil {
				return
			}

			conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				log.Printf("[WebhookClient] Ping failed: %v\n", err)
				return
			}
		}
	}
}

// setConnected updates connection status thread-safely
func (c *WebhookClient) setConnected(connected bool) {
	c.connectedMu.Lock()
	c.connected = connected
	c.connectedMu.Unlock()
}
