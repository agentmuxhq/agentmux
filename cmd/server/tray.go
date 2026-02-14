// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package main

import (
	_ "embed"
	"log"

	"github.com/getlantern/systray"
)

//go:embed assets/icon.ico
var iconData []byte

// InitTray initializes the system tray icon (stub - no functionality yet)
func InitTray() {
	go systray.Run(onTrayReady, onTrayExit)
}

func onTrayReady() {
	systray.SetIcon(iconData)
	systray.SetTitle("AgentMux")
	systray.SetTooltip("AgentMux - AI Terminal")

	log.Println("[tray] System tray initialized (stub - no menu yet)")
}

func onTrayExit() {
	log.Println("[tray] System tray exiting")
}
