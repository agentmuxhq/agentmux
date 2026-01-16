// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package reactive

import (
	"regexp"
	"strings"
	"unicode"
)

const (
	// MaxMessageLength is the maximum allowed message length in bytes
	MaxMessageLength = 10000

	// TruncationSuffix is appended when messages are truncated
	TruncationSuffix = "\n[Message truncated]"
)

var (
	// ansiEscapeRegex matches ANSI escape sequences
	ansiEscapeRegex = regexp.MustCompile(`\x1b\[[0-9;]*[a-zA-Z]`)

	// oscSequenceRegex matches OSC escape sequences (like \x1b]...\x07)
	oscSequenceRegex = regexp.MustCompile(`\x1b\][^\x07]*\x07`)

	// csiSequenceRegex matches CSI sequences
	csiSequenceRegex = regexp.MustCompile(`\x1b\[[^\x40-\x7e]*[\x40-\x7e]`)
)

// SanitizeMessage removes potentially dangerous escape sequences and
// limits the message length to prevent abuse.
func SanitizeMessage(msg string) string {
	if msg == "" {
		return ""
	}

	// Remove ANSI escape sequences
	msg = ansiEscapeRegex.ReplaceAllString(msg, "")

	// Remove OSC sequences (terminal commands)
	msg = oscSequenceRegex.ReplaceAllString(msg, "")

	// Remove CSI sequences
	msg = csiSequenceRegex.ReplaceAllString(msg, "")

	// Remove other control characters (keep newline, tab, carriage return)
	var sanitized strings.Builder
	sanitized.Grow(len(msg))

	for _, r := range msg {
		if r == '\n' || r == '\t' || r == '\r' {
			// Always keep these whitespace characters
			sanitized.WriteRune(r)
		} else if unicode.IsPrint(r) {
			// Keep printable characters (handles ASCII 32-126 and valid Unicode)
			sanitized.WriteRune(r)
		}
		// Skip control characters (0x00-0x1F except \n, \t, \r) and
		// non-printable characters (0x7F DEL, 0x80-0x9F C1 controls)
	}

	result := sanitized.String()

	// Limit length
	if len(result) > MaxMessageLength {
		// Find a safe truncation point (don't break UTF-8)
		truncateAt := MaxMessageLength - len(TruncationSuffix)
		if truncateAt > 0 {
			// Ensure we don't break a UTF-8 sequence
			for truncateAt > 0 && !isValidUTF8Start(result[truncateAt]) {
				truncateAt--
			}
			result = result[:truncateAt] + TruncationSuffix
		}
	}

	return result
}

// isValidUTF8Start returns true if the byte is a valid start of a UTF-8 sequence
func isValidUTF8Start(b byte) bool {
	// ASCII or start of multi-byte sequence (not a continuation byte)
	return b < 0x80 || b >= 0xC0
}

// ValidateAgentID checks if an agent ID is valid.
// Agent IDs must be alphanumeric with optional underscores/hyphens.
func ValidateAgentID(agentID string) bool {
	if agentID == "" || len(agentID) > 64 {
		return false
	}

	for _, r := range agentID {
		if !unicode.IsLetter(r) && !unicode.IsDigit(r) && r != '_' && r != '-' {
			return false
		}
	}

	return true
}

// FormatInjectedMessage optionally wraps the message with source attribution.
// Set includeSource to true to prefix with the source agent.
func FormatInjectedMessage(msg string, sourceAgent string, includeSource bool) string {
	if !includeSource || sourceAgent == "" {
		return msg
	}

	// Simple prefix format: @AgentA: message
	return "@" + sourceAgent + ": " + msg
}
