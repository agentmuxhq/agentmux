// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package authkey

import (
	"fmt"
	"log"
	"net/http"
	"os"
)

var authkey string

const WaveAuthKeyEnv = "WAVETERM_AUTH_KEY"
const AuthKeyHeader = "X-AuthKey"

func ValidateIncomingRequest(r *http.Request) error {
	// Check for auth key in header first (Electron, Node.js WebSocket)
	reqAuthKey := r.Header.Get(AuthKeyHeader)
	authSource := "header"

	// Fallback to query parameter (Tauri browser WebSocket)
	// Browser WebSocket API doesn't support custom headers, so we use query param
	if reqAuthKey == "" {
		reqAuthKey = r.URL.Query().Get("authkey")
		authSource = "query"
	}

	expectedKey := GetAuthKey()

	if reqAuthKey == "" {
		log.Printf("[authkey] REJECT: no auth key in %s (URL: %s)\n", authSource, r.URL.String())
		return fmt.Errorf("no auth key provided (checked header and query param)")
	}
	if reqAuthKey != expectedKey {
		log.Printf("[authkey] REJECT: key mismatch via %s - got %.8s... expected %.8s...\n",
			authSource, reqAuthKey, expectedKey)
		return fmt.Errorf("auth key is invalid")
	}
	log.Printf("[authkey] ACCEPT: valid key via %s\n", authSource)
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
