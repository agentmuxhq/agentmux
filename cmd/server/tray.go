// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package main

import (
	_ "embed"
	"fmt"
	"log"

	"github.com/getlantern/systray"
)

//go:embed assets/icon.ico
var iconData []byte

// InitTray initializes the system tray icon with context menu
func InitTray() {
	go systray.Run(onTrayReady, onTrayExit)
}

func onTrayReady() {
	systray.SetIcon(iconData)
	systray.SetTitle("AgentMux")
	systray.SetTooltip("AgentMux - AI Terminal")

	buildTrayMenu()

	log.Println("[tray] System tray initialized with menu")
}

func buildTrayMenu() {
	version := WaveVersion

	// Version menu item (click to copy to clipboard)
	mVersion := systray.AddMenuItem(
		fmt.Sprintf("AgentMux v%s", version),
		"Click to copy version to clipboard",
	)

	// Handle version click
	go func() {
		for {
			select {
			case <-mVersion.ClickedCh:
				log.Printf("[tray] Version clicked: %s", version)
				copyToClipboard(version)
			}
		}
	}()
}

func onTrayExit() {
	log.Println("[tray] System tray exiting")
}
