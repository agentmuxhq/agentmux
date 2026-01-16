// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package reactive

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
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
