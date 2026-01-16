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
	"path/filepath"
	"sync"
	"time"

	"github.com/a5af/wavemux/pkg/wavebase"
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

// AgentMuxConfigFile represents the agentmux.json config file format.
type AgentMuxConfigFile struct {
	URL   string `json:"url"`
	Token string `json:"token"`
}

// AgentMuxConfigFileName is the name of the config file in the wave data directory.
const AgentMuxConfigFileName = "agentmux.json"

// LoadAgentMuxConfigFile loads the agentmux configuration from the config file.
// Returns empty config if file doesn't exist or is invalid.
func LoadAgentMuxConfigFile() (AgentMuxConfigFile, error) {
	var config AgentMuxConfigFile

	dataDir := wavebase.GetWaveDataDir()
	if dataDir == "" {
		return config, fmt.Errorf("wave data directory not set")
	}

	configPath := filepath.Join(dataDir, AgentMuxConfigFileName)
	data, err := os.ReadFile(configPath)
	if err != nil {
		if os.IsNotExist(err) {
			return config, nil // File doesn't exist, return empty config
		}
		return config, fmt.Errorf("failed to read config file: %w", err)
	}

	if err := json.Unmarshal(data, &config); err != nil {
		return config, fmt.Errorf("failed to parse config file: %w", err)
	}

	return config, nil
}

// SaveAgentMuxConfigFile saves the agentmux configuration to the config file.
func SaveAgentMuxConfigFile(config AgentMuxConfigFile) error {
	dataDir := wavebase.GetWaveDataDir()
	if dataDir == "" {
		return fmt.Errorf("wave data directory not set")
	}

	configPath := filepath.Join(dataDir, AgentMuxConfigFileName)
	data, err := json.MarshalIndent(config, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	if err := os.WriteFile(configPath, data, 0600); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	log.Printf("[reactive/poller] saved config to %s", configPath)
	return nil
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
		config := PollerConfig{}

		// Priority 1: Load from config file (auto-configuration)
		if fileConfig, err := LoadAgentMuxConfigFile(); err != nil {
			log.Printf("[reactive/poller] error loading config file: %v", err)
		} else if fileConfig.URL != "" {
			config.AgentMuxURL = fileConfig.URL
			config.AgentMuxToken = fileConfig.Token
			log.Printf("[reactive/poller] loaded config from file: URL=%s", fileConfig.URL)
		}

		// Priority 2: Environment variables override file config
		if envURL := os.Getenv("AGENTMUX_URL"); envURL != "" {
			config.AgentMuxURL = envURL
		}
		if envToken := os.Getenv("AGENTMUX_TOKEN"); envToken != "" {
			config.AgentMuxToken = envToken
		}

		// Parse poll interval from env
		if intervalStr := os.Getenv("WAVEMUX_REACTIVE_POLL_INTERVAL"); intervalStr != "" {
			if interval, err := time.ParseDuration(intervalStr); err == nil {
				config.PollInterval = interval
			}
		}

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

// ReconfigureGlobalPoller updates the global poller configuration at runtime.
// If agentmuxURL is empty, polling is stopped. If both URL and token are set,
// polling is started/restarted with the new configuration.
// This enables runtime configuration without restarting WaveMux.
// The configuration is also persisted to the config file for future startups.
func ReconfigureGlobalPoller(agentmuxURL, agentmuxToken string) error {
	globalPollerMu.Lock()
	defer globalPollerMu.Unlock()

	poller := GetGlobalPoller()

	// Stop existing poller if running
	// Note: Stop() only briefly locks poller.mu and then waits on wg.
	// The poll loop doesn't acquire globalPollerMu, so no deadlock risk.
	poller.mu.Lock()
	wasRunning := poller.ctx != nil
	poller.mu.Unlock()

	if wasRunning {
		poller.Stop()

		// Reset context after stop
		poller.mu.Lock()
		poller.ctx = nil
		poller.cancel = nil
		poller.mu.Unlock()
	}

	// Update configuration
	poller.mu.Lock()
	poller.agentmuxURL = agentmuxURL
	poller.agentmuxToken = agentmuxToken
	poller.mu.Unlock()

	// Persist configuration to file for future startups
	if err := SaveAgentMuxConfigFile(AgentMuxConfigFile{
		URL:   agentmuxURL,
		Token: agentmuxToken,
	}); err != nil {
		log.Printf("[reactive/poller] warning: failed to save config file: %v", err)
		// Don't fail - runtime config still works
	}

	// If URL is empty, leave poller stopped
	if agentmuxURL == "" {
		log.Printf("[reactive/poller] cross-host polling disabled (URL cleared)")
		return nil
	}

	// If token is empty, don't start (require both)
	if agentmuxToken == "" {
		log.Printf("[reactive/poller] cross-host polling disabled (no token)")
		return nil
	}

	// Start the poller with new config
	log.Printf("[reactive/poller] reconfigured: URL=%s", agentmuxURL)
	return poller.Start()
}

// GetPollerStatus returns the current poller configuration status.
func GetPollerStatus() map[string]interface{} {
	globalPollerMu.Lock()
	defer globalPollerMu.Unlock()

	poller := GetGlobalPoller()
	poller.mu.RLock()
	defer poller.mu.RUnlock()

	status := map[string]interface{}{
		"configured": poller.agentmuxURL != "" && poller.agentmuxToken != "",
		"running":    poller.ctx != nil,
		"url":        poller.agentmuxURL,
		"has_token":  poller.agentmuxToken != "",
	}

	if poller.ctx != nil {
		status["poll_count"] = poller.pollCount
		status["injections_count"] = poller.injectionsCount
		if !poller.lastPollTime.IsZero() {
			status["last_poll"] = poller.lastPollTime.Format(time.RFC3339)
		}
	}

	return status
}
