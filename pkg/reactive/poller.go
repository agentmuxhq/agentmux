// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package reactive

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/url"
	"os"
	"sync"
	"time"
)

// PendingInjection represents an injection waiting for delivery from AgentMux.
type PendingInjection struct {
	ID          string `json:"id"`
	Message     string `json:"message"`
	SourceAgent string `json:"source_agent"`
	Priority    string `json:"priority"`
	CreatedAt   string `json:"created_at"`
}

// PendingResponse is the response from /reactive/pending/{agent_id}.
type PendingResponse struct {
	Injections []PendingInjection `json:"injections"`
}

// AckRequest is the request body for /reactive/ack.
type AckRequest struct {
	InjectionIDs []string `json:"injection_ids"`
}

// Poller polls AgentMux for pending cross-host injections.
type Poller struct {
	mu sync.RWMutex

	// Configuration
	agentmuxURL   string
	agentmuxToken string
	pollInterval  time.Duration
	httpClient    *http.Client

	// Reference to local handler
	handler *Handler

	// Control
	ctx    context.Context
	cancel context.CancelFunc
	wg     sync.WaitGroup

	// Stats
	pollCount       int64
	injectionsCount int64
	lastPollTime    time.Time
	lastError       error
}

// PollerConfig holds configuration for the cross-host poller.
type PollerConfig struct {
	AgentMuxURL   string        // e.g., "https://agentmux.asaf.cc"
	AgentMuxToken string        // Bearer token for auth
	PollInterval  time.Duration // How often to poll (default: 5s)
}

// NewPoller creates a new cross-host injection poller.
func NewPoller(handler *Handler, config PollerConfig) *Poller {
	if config.PollInterval == 0 {
		config.PollInterval = 5 * time.Second
	}

	return &Poller{
		agentmuxURL:   config.AgentMuxURL,
		agentmuxToken: config.AgentMuxToken,
		pollInterval:  config.PollInterval,
		handler:       handler,
		httpClient: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

// Start begins the polling loop in a background goroutine.
func (p *Poller) Start() error {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.ctx != nil {
		return fmt.Errorf("poller already started")
	}

	if p.agentmuxURL == "" {
		return fmt.Errorf("agentmux URL not configured")
	}

	p.ctx, p.cancel = context.WithCancel(context.Background())

	p.wg.Add(1)
	go p.pollLoop()

	log.Printf("[reactive/poller] started polling %s every %v", p.agentmuxURL, p.pollInterval)
	return nil
}

// Stop gracefully stops the polling loop.
func (p *Poller) Stop() {
	p.mu.Lock()
	if p.cancel != nil {
		p.cancel()
	}
	p.mu.Unlock()

	p.wg.Wait()
	log.Printf("[reactive/poller] stopped")
}

// Stats returns current polling statistics.
func (p *Poller) Stats() map[string]interface{} {
	p.mu.RLock()
	defer p.mu.RUnlock()

	stats := map[string]interface{}{
		"poll_count":       p.pollCount,
		"injections_count": p.injectionsCount,
		"poll_interval_ms": p.pollInterval.Milliseconds(),
		"agentmux_url":     p.agentmuxURL,
	}

	if !p.lastPollTime.IsZero() {
		stats["last_poll_time"] = p.lastPollTime.Format(time.RFC3339)
	}
	if p.lastError != nil {
		stats["last_error"] = p.lastError.Error()
	}

	return stats
}

// pollLoop runs the main polling loop.
func (p *Poller) pollLoop() {
	defer p.wg.Done()

	ticker := time.NewTicker(p.pollInterval)
	defer ticker.Stop()

	// Poll immediately on start
	p.pollAndInject()

	for {
		select {
		case <-p.ctx.Done():
			return
		case <-ticker.C:
			p.pollAndInject()
		}
	}
}

// pollAndInject polls for pending injections and executes them locally.
func (p *Poller) pollAndInject() {
	p.mu.Lock()
	p.pollCount++
	p.lastPollTime = time.Now()
	p.mu.Unlock()

	// Get list of locally registered agents
	agents := p.handler.ListAgents()
	if len(agents) == 0 {
		return // No local agents, nothing to poll for
	}

	for _, agent := range agents {
		if err := p.pollForAgent(agent.AgentID); err != nil {
			p.mu.Lock()
			p.lastError = err
			p.mu.Unlock()
			log.Printf("[reactive/poller] error polling for agent %s: %v", agent.AgentID, err)
		}
	}
}

// pollForAgent polls for pending injections for a specific agent.
func (p *Poller) pollForAgent(agentID string) error {
	// Build request URL with proper escaping for defense in depth
	reqURL := fmt.Sprintf("%s/reactive/pending/%s", p.agentmuxURL, url.PathEscape(agentID))

	req, err := http.NewRequestWithContext(p.ctx, http.MethodGet, reqURL, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	// Add auth headers
	if p.agentmuxToken != "" {
		req.Header.Set("Authorization", "Bearer "+p.agentmuxToken)
	}
	req.Header.Set("X-Agent-ID", agentID)

	// Make request
	resp, err := p.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	// Check status
	if resp.StatusCode == http.StatusNotFound {
		// No pending injections
		return nil
	}
	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 1024))
		return fmt.Errorf("unexpected status %d: %s", resp.StatusCode, string(body))
	}

	// Parse response
	var pendingResp PendingResponse
	if err := json.NewDecoder(resp.Body).Decode(&pendingResp); err != nil {
		return fmt.Errorf("failed to parse response: %w", err)
	}

	if len(pendingResp.Injections) == 0 {
		return nil
	}

	log.Printf("[reactive/poller] found %d pending injection(s) for agent %s", len(pendingResp.Injections), agentID)

	// Process each injection
	deliveredIDs := make([]string, 0, len(pendingResp.Injections))

	for _, injection := range pendingResp.Injections {
		// Execute local injection
		result := p.handler.InjectMessage(InjectionRequest{
			TargetAgentID: agentID,
			Message:       injection.Message,
			SourceAgent:   injection.SourceAgent,
			Priority:      injection.Priority,
			RequestID:     injection.ID,
		})

		if result.Success {
			deliveredIDs = append(deliveredIDs, injection.ID)
			p.mu.Lock()
			p.injectionsCount++
			p.mu.Unlock()
			log.Printf("[reactive/poller] delivered injection %s to %s", injection.ID, agentID)
		} else {
			log.Printf("[reactive/poller] failed to deliver injection %s to %s: %s", injection.ID, agentID, result.Error)
		}
	}

	// Acknowledge delivered injections
	if len(deliveredIDs) > 0 {
		if err := p.acknowledgeDelivery(deliveredIDs, agentID); err != nil {
			log.Printf("[reactive/poller] failed to acknowledge delivery: %v", err)
			// Don't return error - injections were still delivered locally
		}
	}

	return nil
}

// acknowledgeDelivery marks injections as delivered in AgentMux.
func (p *Poller) acknowledgeDelivery(injectionIDs []string, agentID string) error {
	url := fmt.Sprintf("%s/reactive/ack", p.agentmuxURL)

	ackReq := AckRequest{
		InjectionIDs: injectionIDs,
	}

	body, err := json.Marshal(ackReq)
	if err != nil {
		return fmt.Errorf("failed to marshal ack request: %w", err)
	}

	req, err := http.NewRequestWithContext(p.ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")
	if p.agentmuxToken != "" {
		req.Header.Set("Authorization", "Bearer "+p.agentmuxToken)
	}
	req.Header.Set("X-Agent-ID", agentID)

	resp, err := p.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		respBody, _ := io.ReadAll(io.LimitReader(resp.Body, 1024))
		return fmt.Errorf("unexpected status %d: %s", resp.StatusCode, string(respBody))
	}

	log.Printf("[reactive/poller] acknowledged %d injection(s)", len(injectionIDs))
	return nil
}

// Global poller instance
var (
	globalPoller     *Poller
	globalPollerOnce sync.Once
	globalPollerMu   sync.Mutex
)

// GetGlobalPoller returns the global poller instance, creating it if needed.
func GetGlobalPoller() *Poller {
	globalPollerOnce.Do(func() {
		// Read config from environment
		config := PollerConfig{
			AgentMuxURL:   os.Getenv("AGENTMUX_URL"),
			AgentMuxToken: os.Getenv("AGENTMUX_TOKEN"),
		}

		// Parse poll interval from env
		if intervalStr := os.Getenv("WAVEMUX_REACTIVE_POLL_INTERVAL"); intervalStr != "" {
			if interval, err := time.ParseDuration(intervalStr); err == nil {
				config.PollInterval = interval
			}
		}

		// No default URL - must be explicitly configured to avoid
		// unintended outbound requests to third-party servers

		globalPoller = NewPoller(GetGlobalHandler(), config)
	})

	return globalPoller
}

// StartGlobalPoller starts the global poller if configured.
// Returns nil if cross-host polling is not configured.
// Requires both AGENTMUX_URL and AGENTMUX_TOKEN to be set.
func StartGlobalPoller() error {
	globalPollerMu.Lock()
	defer globalPollerMu.Unlock()

	poller := GetGlobalPoller()

	// Require both URL and token to be explicitly configured
	if poller.agentmuxURL == "" {
		log.Printf("[reactive/poller] cross-host polling disabled (no AGENTMUX_URL)")
		return nil
	}
	if poller.agentmuxToken == "" {
		log.Printf("[reactive/poller] cross-host polling disabled (no AGENTMUX_TOKEN)")
		return nil
	}

	return poller.Start()
}

// StopGlobalPoller stops the global poller.
func StopGlobalPoller() {
	globalPollerMu.Lock()
	defer globalPollerMu.Unlock()

	if globalPoller != nil {
		globalPoller.Stop()
	}
}
