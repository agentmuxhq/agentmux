// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package reactive

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
)

// HandleInject handles POST requests to inject messages into agent terminals.
// Endpoint: POST /wave/reactive/inject
func HandleInject(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Read request body
	body, err := io.ReadAll(io.LimitReader(r.Body, 1024*1024)) // 1MB limit
	if err != nil {
		writeJSONError(w, "failed to read request body", http.StatusBadRequest)
		return
	}
	defer r.Body.Close()

	// Parse request
	var req InjectionRequest
	if err := json.Unmarshal(body, &req); err != nil {
		writeJSONError(w, "invalid JSON: "+err.Error(), http.StatusBadRequest)
		return
	}

	// Validate required fields
	if req.TargetAgentID == "" {
		writeJSONError(w, "target_agent is required", http.StatusBadRequest)
		return
	}
	if req.Message == "" {
		writeJSONError(w, "message is required", http.StatusBadRequest)
		return
	}

	// Perform injection
	handler := GetGlobalHandler()
	resp := handler.InjectMessage(req)

	// Write response
	w.Header().Set("Content-Type", "application/json")
	if resp.Success {
		w.WriteHeader(http.StatusOK)
	} else {
		w.WriteHeader(http.StatusBadRequest)
	}
	json.NewEncoder(w).Encode(resp)
}

// HandleListAgents handles GET requests to list registered agents.
// Endpoint: GET /wave/reactive/agents
func HandleListAgents(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	handler := GetGlobalHandler()
	agents := handler.ListAgents()

	resp := AgentListResponse{
		Agents: agents,
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// HandleGetAgent handles GET requests to get a specific agent's info.
// Endpoint: GET /wave/reactive/agent?id=AgentX
func HandleGetAgent(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	agentID := r.URL.Query().Get("id")
	if agentID == "" {
		writeJSONError(w, "id query parameter is required", http.StatusBadRequest)
		return
	}

	// Validate agent ID to prevent malicious input
	if !ValidateAgentID(agentID) {
		writeJSONError(w, "invalid agent ID format", http.StatusBadRequest)
		return
	}

	handler := GetGlobalHandler()
	agent := handler.GetAgent(agentID)

	if agent == nil {
		writeJSONError(w, "agent not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(agent)
}

// AgentRegistrationRequest represents a request to register an agent.
type AgentRegistrationRequest struct {
	AgentID string `json:"agent_id"`
	BlockID string `json:"block_id"`
	TabID   string `json:"tab_id,omitempty"`
}

// HandleRegisterAgent handles POST requests to register an agent.
// Endpoint: POST /wave/reactive/register
func HandleRegisterAgent(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Read request body
	body, err := io.ReadAll(io.LimitReader(r.Body, 4096)) // Small payload expected
	if err != nil {
		writeJSONError(w, "failed to read request body", http.StatusBadRequest)
		return
	}
	defer r.Body.Close()

	// Parse request
	var req AgentRegistrationRequest
	if err := json.Unmarshal(body, &req); err != nil {
		writeJSONError(w, "invalid JSON: "+err.Error(), http.StatusBadRequest)
		return
	}

	// Validate required fields
	if req.AgentID == "" {
		writeJSONError(w, "agent_id is required", http.StatusBadRequest)
		return
	}
	if req.BlockID == "" {
		writeJSONError(w, "block_id is required", http.StatusBadRequest)
		return
	}

	// Validate agent ID format
	if !ValidateAgentID(req.AgentID) {
		writeJSONError(w, "invalid agent_id format", http.StatusBadRequest)
		return
	}

	// Register the agent
	handler := GetGlobalHandler()
	if err := handler.RegisterAgent(req.AgentID, req.BlockID, req.TabID); err != nil {
		writeJSONError(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"success":  true,
		"agent_id": req.AgentID,
		"block_id": req.BlockID,
	})
}

// HandleUnregisterAgent handles DELETE requests to unregister an agent.
// Endpoint: DELETE /wave/reactive/register?agent_id=AgentX
// Also supports: POST /wave/reactive/unregister with JSON body
func HandleUnregisterAgent(w http.ResponseWriter, r *http.Request) {
	var agentID string

	if r.Method == http.MethodDelete {
		agentID = r.URL.Query().Get("agent_id")
	} else if r.Method == http.MethodPost {
		// Support POST with JSON body for easier frontend integration
		body, err := io.ReadAll(io.LimitReader(r.Body, 4096))
		if err != nil {
			writeJSONError(w, "failed to read request body", http.StatusBadRequest)
			return
		}
		defer r.Body.Close()

		var req struct {
			AgentID string `json:"agent_id"`
		}
		if err := json.Unmarshal(body, &req); err != nil {
			writeJSONError(w, "invalid JSON: "+err.Error(), http.StatusBadRequest)
			return
		}
		agentID = req.AgentID
	} else {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if agentID == "" {
		writeJSONError(w, "agent_id is required", http.StatusBadRequest)
		return
	}

	// Validate agent ID format
	if !ValidateAgentID(agentID) {
		writeJSONError(w, "invalid agent_id format", http.StatusBadRequest)
		return
	}

	// Unregister the agent
	handler := GetGlobalHandler()
	handler.UnregisterAgent(agentID)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"success":  true,
		"agent_id": agentID,
	})
}

// HandleAuditLog handles GET requests to retrieve the audit log.
// Endpoint: GET /wave/reactive/audit?limit=50
func HandleAuditLog(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	limit := 50 // default
	if limitStr := r.URL.Query().Get("limit"); limitStr != "" {
		if l, err := parseInt(limitStr); err == nil && l > 0 {
			limit = l
		}
	}

	handler := GetGlobalHandler()
	entries := handler.GetAuditLog(limit)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"entries": entries,
	})
}

// HandlePollerStats handles GET requests to retrieve cross-host poller statistics.
// Endpoint: GET /wave/reactive/poller/stats
func HandlePollerStats(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	poller := GetGlobalPoller()
	stats := poller.Stats()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(stats)
}

// PollerConfigRequest represents a request to configure the poller at runtime.
type PollerConfigRequest struct {
	AgentMuxURL   string `json:"agentmux_url"`
	AgentMuxToken string `json:"agentmux_token"`
}

// validateAgentMuxURL validates the agentmux URL to prevent SSRF attacks.
// Only allows https:// URLs (or http://localhost for development).
func validateAgentMuxURL(urlStr string) error {
	parsed, err := url.Parse(urlStr)
	if err != nil {
		return fmt.Errorf("malformed URL: %w", err)
	}

	// Must have a scheme
	if parsed.Scheme == "" {
		return fmt.Errorf("URL must include scheme (https://)")
	}

	// Must have a host
	if parsed.Host == "" {
		return fmt.Errorf("URL must include host")
	}

	// Only allow https, or http for localhost (dev mode)
	switch parsed.Scheme {
	case "https":
		// Always allowed
	case "http":
		// Only allow for localhost/127.0.0.1/[::1] (development)
		host := parsed.Hostname() // Properly extracts host, handling IPv6 brackets
		if host != "localhost" && host != "127.0.0.1" && host != "::1" {
			return fmt.Errorf("http:// only allowed for localhost; use https:// for remote servers")
		}
	default:
		return fmt.Errorf("URL scheme must be https (got %s)", parsed.Scheme)
	}

	// Block file://, ftp://, and other potentially dangerous schemes
	// (already handled by scheme check above)

	return nil
}

// HandlePollerConfig handles POST requests to configure the cross-host poller at runtime.
// Endpoint: POST /wave/reactive/poller/config
// This allows updating AGENTMUX_URL and AGENTMUX_TOKEN without restarting AgentMux.
func HandlePollerConfig(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Read request body
	body, err := io.ReadAll(io.LimitReader(r.Body, 4096)) // Small payload expected
	if err != nil {
		writeJSONError(w, "failed to read request body", http.StatusBadRequest)
		return
	}
	defer r.Body.Close()

	// Parse request
	var req PollerConfigRequest
	if err := json.Unmarshal(body, &req); err != nil {
		writeJSONError(w, "invalid JSON: "+err.Error(), http.StatusBadRequest)
		return
	}

	// Validate URL if provided (empty URL is allowed to disable polling)
	if req.AgentMuxURL != "" {
		if err := validateAgentMuxURL(req.AgentMuxURL); err != nil {
			writeJSONError(w, "invalid agentmux_url: "+err.Error(), http.StatusBadRequest)
			return
		}
	}

	// Reconfigure the poller
	if err := ReconfigureGlobalPoller(req.AgentMuxURL, req.AgentMuxToken); err != nil {
		writeJSONError(w, "failed to reconfigure poller: "+err.Error(), http.StatusInternalServerError)
		return
	}

	// Return current status
	status := GetPollerStatus()
	status["success"] = true

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(status)
}

// HandlePollerStatus handles GET requests to retrieve the current poller configuration status.
// Endpoint: GET /wave/reactive/poller/status
func HandlePollerStatus(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	status := GetPollerStatus()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(status)
}

// Helper functions

func writeJSONError(w http.ResponseWriter, message string, status int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"success": false,
		"error":   message,
	})
}

func parseInt(s string) (int, error) {
	if len(s) == 0 {
		return 0, fmt.Errorf("empty string")
	}
	if len(s) > 10 {
		// Prevent overflow - int max is ~2 billion (10 digits)
		return 0, fmt.Errorf("integer too large: %s", s)
	}
	var result int
	for _, c := range s {
		if c < '0' || c > '9' {
			return 0, fmt.Errorf("invalid integer: %s", s)
		}
		digit := int(c - '0')
		// Check for overflow before multiplication
		if result > (1<<31-1-digit)/10 {
			return 0, fmt.Errorf("integer overflow: %s", s)
		}
		result = result*10 + digit
	}
	return result, nil
}
