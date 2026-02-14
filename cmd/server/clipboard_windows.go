//go:build windows

// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"log"
	"os/exec"
	"strings"
)

// copyToClipboard copies text to clipboard using Windows clip.exe
func copyToClipboard(text string) {
	cmd := exec.Command("clip")
	cmd.Stdin = strings.NewReader(text)

	if err := cmd.Run(); err != nil {
		log.Printf("[tray] Failed to copy to clipboard: %v", err)
		return
	}

	log.Printf("[tray] Copied to clipboard: %s", text)
}
