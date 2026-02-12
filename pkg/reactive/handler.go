// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package reactive

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"log"
	"strings"
	"sync"
	"time"

	"github.com/a5af/agentmux/pkg/waveobj"
	"github.com/a5af/agentmux/pkg/wstore"
	"github.com/google/uuid"
)

// InputSender is a function type that sends input to a block's PTY.
// This decouples the handler from direct blockcontroller dependency.
type InputSender func(blockId string, inputData []byte) error

// Handler manages reactive message injection for agent-to-agent communication.
type Handler struct {
	mu sync.RWMutex

	// agentToBlock maps agent IDs to block IDs
	agentToBlock map[string]string

	// blockToAgent is the reverse mapping
	blockToAgent map[string]string

	// agentInfo stores detailed registration info
	agentInfo map[string]*AgentRegistration

	// inputSender is the function to send input to blocks
	inputSender InputSender

	// auditLog stores recent injection attempts (ring buffer)
	auditLog     []AuditLogEntry
	auditLogSize int
	auditLogIdx  int

	// config
	includeSourceInMessage bool

	// Rate limiting (protects against DoS while keeping synchronous injection)
	rateLimitMu      sync.Mutex
	rateLimitTokens  int
	rateLimitMax     int
	rateLimitRefresh time.Time
}

// NewHandler creates a new reactive message handler.
func NewHandler(inputSender InputSender) *Handler {
	return &Handler{
		agentToBlock:           make(map[string]string),
		blockToAgent:           make(map[string]string),
		agentInfo:              make(map[string]*AgentRegistration),
		inputSender:            inputSender,
		auditLog:               make([]AuditLogEntry, 100), // Keep last 100 entries
		auditLogSize:           100,
		auditLogIdx:            0,
		includeSourceInMessage: false, // Can be configured
		rateLimitMax:           10,    // 10 injections per second max
		rateLimitTokens:        10,
		rateLimitRefresh:       time.Now(),
	}
}

// checkRateLimit returns true if the request is allowed, false if rate limited.
// Uses a simple token bucket: 10 tokens per second, refilled each second.
func (h *Handler) checkRateLimit() bool {
	h.rateLimitMu.Lock()
	defer h.rateLimitMu.Unlock()

	now := time.Now()
	// Refill tokens every second
	if now.Sub(h.rateLimitRefresh) >= time.Second {
		h.rateLimitTokens = h.rateLimitMax
		h.rateLimitRefresh = now
	}

	if h.rateLimitTokens > 0 {
		h.rateLimitTokens--
		return true
	}
	return false
}

// RegisterAgent associates an agent ID with a block ID.
// Called when a pane's shell sets WAVEMUX_AGENT_ID.
func (h *Handler) RegisterAgent(agentID, blockID, tabID string) error {
	// Normalize agent ID to lowercase for case-insensitive matching
	agentID = strings.ToLower(agentID)

	if !ValidateAgentID(agentID) {
		return fmt.Errorf("invalid agent ID: %s", agentID)
	}

	h.mu.Lock()
	defer h.mu.Unlock()

	// Check if this block already has a different agent
	if existingAgent, ok := h.blockToAgent[blockID]; ok && existingAgent != agentID {
		// Unregister the old agent
		delete(h.agentToBlock, existingAgent)
		delete(h.agentInfo, existingAgent)
	}

	// Check if this agent is already registered to a different block
	if existingBlock, ok := h.agentToBlock[agentID]; ok && existingBlock != blockID {
		// Unregister from the old block
		delete(h.blockToAgent, existingBlock)
	}

	now := time.Now()
	h.agentToBlock[agentID] = blockID
	h.blockToAgent[blockID] = agentID
	h.agentInfo[agentID] = &AgentRegistration{
		AgentID:      agentID,
		BlockID:      blockID,
		TabID:        tabID,
		RegisteredAt: now,
		LastSeen:     now,
	}

	log.Printf("[reactive] registered agent %s -> block %s", agentID, blockID)
	return nil
}

// UnregisterAgent removes an agent's registration.
// Called when a pane closes or agent ID is cleared.
func (h *Handler) UnregisterAgent(agentID string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if blockID, ok := h.agentToBlock[agentID]; ok {
		delete(h.blockToAgent, blockID)
	}
	delete(h.agentToBlock, agentID)
	delete(h.agentInfo, agentID)

	log.Printf("[reactive] unregistered agent %s", agentID)
}

// UnregisterBlock removes registration by block ID.
// Called when a block/pane is closed.
func (h *Handler) UnregisterBlock(blockID string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if agentID, ok := h.blockToAgent[blockID]; ok {
		delete(h.agentToBlock, agentID)
		delete(h.agentInfo, agentID)
	}
	delete(h.blockToAgent, blockID)

	log.Printf("[reactive] unregistered block %s", blockID)
}

// UpdateLastSeen updates the last seen time for an agent.
func (h *Handler) UpdateLastSeen(agentID string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if info, ok := h.agentInfo[agentID]; ok {
		info.LastSeen = time.Now()
	}
}

// GetAgent returns the registration info for an agent.
func (h *Handler) GetAgent(agentID string) *AgentRegistration {
	h.mu.RLock()
	defer h.mu.RUnlock()

	if info, ok := h.agentInfo[agentID]; ok {
		// Return a copy
		copy := *info
		return &copy
	}
	return nil
}

// GetAgentByBlock returns the agent ID for a block.
func (h *Handler) GetAgentByBlock(blockID string) string {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return h.blockToAgent[blockID]
}

// ListAgents returns all registered agents.
func (h *Handler) ListAgents() []AgentRegistration {
	h.mu.RLock()
	defer h.mu.RUnlock()

	result := make([]AgentRegistration, 0, len(h.agentInfo))
	for _, info := range h.agentInfo {
		result = append(result, *info)
	}
	return result
}

// InjectMessage sends a message to a target agent's terminal.
func (h *Handler) InjectMessage(req InjectionRequest) InjectionResponse {
	// Generate request ID if not provided
	if req.RequestID == "" {
		req.RequestID = uuid.New().String()
	}

	// Rate limit check (prevents DoS while keeping synchronous injection)
	if !h.checkRateLimit() {
		return h.errorResponse(req, "rate limited: too many injection requests")
	}

	// Validate target agent ID
	if !ValidateAgentID(req.TargetAgentID) {
		return h.errorResponse(req, "invalid target agent ID")
	}

	// Sanitize the message
	sanitizedMsg := SanitizeMessage(req.Message)
	if sanitizedMsg == "" {
		return h.errorResponse(req, "message is empty after sanitization")
	}

	// Format message with optional source attribution
	finalMsg := FormatInjectedMessage(sanitizedMsg, req.SourceAgent, h.includeSourceInMessage)

	// Look up target block
	h.mu.RLock()
	blockID, exists := h.agentToBlock[req.TargetAgentID]
	h.mu.RUnlock()

	if !exists {
		return h.errorResponse(req, fmt.Sprintf("agent %s not found or not in a WaveMux pane", req.TargetAgentID))
	}

	// Send input to the block's PTY
	if h.inputSender == nil {
		return h.errorResponse(req, "input sender not configured")
	}

	// IMPORTANT: Message and Enter MUST be sent separately, not atomically.
	// Atomic writes (message + \r) do NOT reliably trigger input processing.
	// The PTY needs to see the message first, then Enter as a distinct event.
	//
	// ALSO IMPORTANT: This MUST be synchronous, not in a goroutine.
	// Async Enter sending was tried and failed - the Enter keys fire but
	// don't submit the message (they create blank lines instead).
	// The synchronous approach ensures message and Enter stay coordinated.

	// Step 1: Send the message content
	err := h.inputSender(blockID, []byte(finalMsg))
	if err != nil {
		h.logAudit(req, blockID, len(finalMsg), false, err.Error())
		return h.errorResponse(req, fmt.Sprintf("failed to send input: %v", err))
	}

	// Step 2: Small delay to let the terminal process the message
	time.Sleep(150 * time.Millisecond)

	// Step 3: Send Enter key (carriage return only, not CRLF)
	err = h.inputSender(blockID, []byte("\r"))
	if err != nil {
		// Log but don't fail - message was delivered, Enter just didn't work
		log.Printf("[reactive] Enter key send failed for block %s: %v", blockID, err)
	}

	// Log successful injection
	h.logAudit(req, blockID, len(finalMsg), true, "")

	return InjectionResponse{
		Success:   true,
		RequestID: req.RequestID,
		BlockID:   blockID,
		Timestamp: time.Now(),
	}
}

// errorResponse creates an error response and logs it.
func (h *Handler) errorResponse(req InjectionRequest, errMsg string) InjectionResponse {
	h.logAudit(req, "", len(req.Message), false, errMsg)
	return InjectionResponse{
		Success:   false,
		RequestID: req.RequestID,
		Error:     errMsg,
		Timestamp: time.Now(),
	}
}

// logAudit records an injection attempt to the audit log.
func (h *Handler) logAudit(req InjectionRequest, blockID string, msgLen int, success bool, errMsg string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	// Compute message hash (for audit without storing content)
	hash := sha256.Sum256([]byte(req.Message))

	entry := AuditLogEntry{
		Timestamp:     time.Now(),
		SourceAgent:   req.SourceAgent,
		TargetAgent:   req.TargetAgentID,
		BlockID:       blockID,
		MessageHash:   hex.EncodeToString(hash[:8]), // First 8 bytes only
		MessageLength: msgLen,
		Success:       success,
		ErrorMessage:  errMsg,
		RequestID:     req.RequestID,
	}

	h.auditLog[h.auditLogIdx] = entry
	h.auditLogIdx = (h.auditLogIdx + 1) % h.auditLogSize
}

// GetAuditLog returns recent audit entries.
func (h *Handler) GetAuditLog(limit int) []AuditLogEntry {
	h.mu.RLock()
	defer h.mu.RUnlock()

	if limit <= 0 || limit > h.auditLogSize {
		limit = h.auditLogSize
	}

	result := make([]AuditLogEntry, 0, limit)

	// Read entries in reverse order (most recent first)
	idx := (h.auditLogIdx - 1 + h.auditLogSize) % h.auditLogSize
	for i := 0; i < limit; i++ {
		entry := h.auditLog[idx]
		if entry.Timestamp.IsZero() {
			break // No more entries
		}
		result = append(result, entry)
		idx = (idx - 1 + h.auditLogSize) % h.auditLogSize
	}

	return result
}

// SyncAgentsFromBlocks scans all blocks and registers any with WAVEMUX_AGENT_ID set.
// This is useful on startup to recover agent registrations.
func (h *Handler) SyncAgentsFromBlocks(ctx context.Context) error {
	blocks, err := wstore.DBGetAllObjsByType[*waveobj.Block](ctx, "block")
	if err != nil {
		return fmt.Errorf("failed to get blocks: %w", err)
	}

	for _, block := range blocks {
		if block.Meta == nil {
			continue
		}

		// Check if block has agent env var set
		cmdEnv, ok := block.Meta["cmd:env"].(map[string]interface{})
		if !ok {
			continue
		}

		agentID, ok := cmdEnv["WAVEMUX_AGENT_ID"].(string)
		if !ok || agentID == "" {
			continue
		}

		// Get tab ID from parent
		tabID := ""
		if block.ParentORef != "" {
			// ParentORef format is "tab:<tabid>"
			if len(block.ParentORef) > 4 && block.ParentORef[:4] == "tab:" {
				tabID = block.ParentORef[4:]
			}
		}

		h.RegisterAgent(agentID, block.OID, tabID)
	}

	return nil
}

// Global handler instance
var globalHandler *Handler
var globalHandlerOnce sync.Once

// GetGlobalHandler returns the global reactive handler instance.
func GetGlobalHandler() *Handler {
	globalHandlerOnce.Do(func() {
		// Handler will be initialized with proper inputSender later
		globalHandler = NewHandler(nil)
	})
	return globalHandler
}

// InitGlobalHandler initializes the global handler with an input sender.
func InitGlobalHandler(inputSender InputSender) {
	handler := GetGlobalHandler()
	handler.mu.Lock()
	handler.inputSender = inputSender
	handler.mu.Unlock()
}
