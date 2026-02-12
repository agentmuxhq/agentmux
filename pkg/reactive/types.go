// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Package reactive provides real-time message injection into terminal panes.
// This enables agent-to-agent communication by writing directly to PTY stdin.
package reactive

import "time"

// InjectionRequest represents a request to inject text into a terminal pane.
type InjectionRequest struct {
	// TargetAgentID is the agent to send the message to (e.g., "AgentX", "AgentA")
	TargetAgentID string `json:"target_agent"`

	// Message is the text to inject into the terminal
	Message string `json:"message"`

	// SourceAgent is the agent sending the message (for audit logging)
	SourceAgent string `json:"source_agent,omitempty"`

	// RequestID is a unique identifier for this request (for tracking)
	RequestID string `json:"request_id,omitempty"`

	// Priority indicates message urgency ("normal" or "urgent")
	Priority string `json:"priority,omitempty"`

	// WaitForIdle indicates whether to wait for the target to be idle before injecting
	WaitForIdle bool `json:"wait_for_idle,omitempty"`
}

// InjectionResponse represents the result of an injection attempt.
type InjectionResponse struct {
	// Success indicates whether the injection was successful
	Success bool `json:"success"`

	// RequestID echoes back the request ID for correlation
	RequestID string `json:"request_id,omitempty"`

	// BlockID is the block/pane that received the message
	BlockID string `json:"block_id,omitempty"`

	// Error contains the error message if Success is false
	Error string `json:"error,omitempty"`

	// Timestamp is when the injection occurred
	Timestamp time.Time `json:"timestamp,omitempty"`
}

// AgentRegistration represents an agent's presence in a AgentMux pane.
type AgentRegistration struct {
	// AgentID is the unique identifier for the agent (e.g., "AgentA", "AgentX")
	AgentID string `json:"agent_id"`

	// BlockID is the AgentMux block/pane ID
	BlockID string `json:"block_id"`

	// TabID is the tab containing the block
	TabID string `json:"tab_id,omitempty"`

	// RegisteredAt is when the agent was registered
	RegisteredAt time.Time `json:"registered_at"`

	// LastSeen is the last time the agent was active
	LastSeen time.Time `json:"last_seen"`
}

// AgentListResponse contains the list of registered agents.
type AgentListResponse struct {
	Agents []AgentRegistration `json:"agents"`
}

// AuditLogEntry records an injection attempt for security auditing.
type AuditLogEntry struct {
	Timestamp     time.Time `json:"timestamp"`
	SourceAgent   string    `json:"source_agent"`
	TargetAgent   string    `json:"target_agent"`
	BlockID       string    `json:"block_id"`
	MessageHash   string    `json:"message_hash"` // SHA256 of message (not full content)
	MessageLength int       `json:"message_length"`
	Success       bool      `json:"success"`
	ErrorMessage  string    `json:"error_message,omitempty"`
	RequestID     string    `json:"request_id,omitempty"`
}
