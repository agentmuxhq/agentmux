//go:build !windows

// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"log"
	"os/exec"
	"runtime"
	"strings"
)

// copyToClipboard copies text to clipboard on macOS/Linux
func copyToClipboard(text string) {
	var cmd *exec.Cmd

	switch runtime.GOOS {
	case "darwin":
		// macOS: use pbcopy
		cmd = exec.Command("pbcopy")
	case "linux":
		// Linux: try xclip first, fallback to xsel
		if _, err := exec.LookPath("xclip"); err == nil {
			cmd = exec.Command("xclip", "-selection", "clipboard")
		} else if _, err := exec.LookPath("xsel"); err == nil {
			cmd = exec.Command("xsel", "--clipboard", "--input")
		} else {
			log.Printf("[tray] No clipboard utility found (xclip/xsel not installed)")
			return
		}
	default:
		log.Printf("[tray] Clipboard not supported on %s", runtime.GOOS)
		return
	}

	cmd.Stdin = strings.NewReader(text)

	if err := cmd.Run(); err != nil {
		log.Printf("[tray] Failed to copy to clipboard: %v", err)
		return
	}

	log.Printf("[tray] Copied to clipboard: %s", text)
}
