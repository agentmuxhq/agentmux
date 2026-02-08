// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package authkey

import (
	"fmt"
	"net/http"
	"os"
)

var authkey string

const WaveAuthKeyEnv = "WAVETERM_AUTH_KEY"
const AuthKeyHeader = "X-AuthKey"

func ValidateIncomingRequest(r *http.Request) error {
	// Check for auth key in header first (Electron, Node.js WebSocket)
	reqAuthKey := r.Header.Get(AuthKeyHeader)

	// Fallback to query parameter (Tauri browser WebSocket)
	// Browser WebSocket API doesn't support custom headers, so we use query param
	if reqAuthKey == "" {
		reqAuthKey = r.URL.Query().Get("authkey")
	}

	if reqAuthKey == "" {
		return fmt.Errorf("no auth key provided (checked header and query param)")
	}
	if reqAuthKey != GetAuthKey() {
		return fmt.Errorf("auth key is invalid")
	}
	return nil
}

func SetAuthKeyFromEnv() error {
	authkey = os.Getenv(WaveAuthKeyEnv)
	if authkey == "" {
		return fmt.Errorf("no auth key found in environment variables")
	}
	os.Unsetenv(WaveAuthKeyEnv)
	return nil
}

func GetAuthKey() string {
	return authkey
}
