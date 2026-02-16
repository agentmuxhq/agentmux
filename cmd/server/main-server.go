// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"time"

	"github.com/joho/godotenv"
	"github.com/a5af/agentmux/pkg/authkey"
	"github.com/a5af/agentmux/pkg/blockcontroller"
	"github.com/a5af/agentmux/pkg/blocklogger"
	"github.com/a5af/agentmux/pkg/filestore"
	"github.com/a5af/agentmux/pkg/panichandler"
	"github.com/a5af/agentmux/pkg/reactive"
	"github.com/a5af/agentmux/pkg/remote/conncontroller"
	"github.com/a5af/agentmux/pkg/remote/fileshare/wshfs"
	"github.com/a5af/agentmux/pkg/service"
	"github.com/a5af/agentmux/pkg/telemetry"
	"github.com/a5af/agentmux/pkg/telemetry/telemetrydata"
	"github.com/a5af/agentmux/pkg/util/shellutil"
	"github.com/a5af/agentmux/pkg/util/sigutil"
	"github.com/a5af/agentmux/pkg/util/utilfn"
	"github.com/a5af/agentmux/pkg/wavebase"
	"github.com/a5af/agentmux/pkg/waveobj"
	"github.com/a5af/agentmux/pkg/wcloud"
	"github.com/a5af/agentmux/pkg/wconfig"
	"github.com/a5af/agentmux/pkg/wcore"
	"github.com/a5af/agentmux/pkg/web"
	"github.com/a5af/agentmux/pkg/wps"
	"github.com/a5af/agentmux/pkg/wshrpc"
	"github.com/a5af/agentmux/pkg/wshrpc/wshremote"
	"github.com/a5af/agentmux/pkg/wshrpc/wshserver"
	"github.com/a5af/agentmux/pkg/wshutil"
	"github.com/a5af/agentmux/pkg/wslconn"
	"github.com/a5af/agentmux/pkg/wstore"
	"github.com/a5af/agentmux/pkg/webhookdelivery"
)

// these are set at build time
var WaveVersion = "0.0.0"
var BuildTime = "0"

// CurrentInstanceID stores the instance ID for this running server
// Empty string for default instance, "instance-N" for auto-generated instances
var CurrentInstanceID = ""

// ExpectedVersion is the version this binary should be running
// This is auto-updated by bump-version.sh to match package.json
// If WaveVersion != ExpectedVersion, it indicates a stale cached binary
const ExpectedVersion = "0.28.8"

const InitialTelemetryWait = 10 * time.Second
const TelemetryTick = 2 * time.Minute
const TelemetryInterval = 4 * time.Hour
const TelemetryInitialCountsWait = 5 * time.Second
const TelemetryCountsInterval = 1 * time.Hour

var shutdownOnce sync.Once

func init() {
	envFilePath := os.Getenv("WAVETERM_ENVFILE")
	if envFilePath != "" {
		log.Printf("applying env file: %s\n", envFilePath)
		_ = godotenv.Load(envFilePath)
	}
}

func doShutdown(reason string) {
	shutdownOnce.Do(func() {
		log.Printf("shutting down: %s\n", reason)
		ctx, cancelFn := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancelFn()
		go blockcontroller.StopAllBlockControllers()
		shutdownActivityUpdate()
		sendTelemetryWrapper()
		// Shutdown webhook service
		webhookdelivery.ShutdownWebhookService()
		// TODO deal with flush in progress
		clearTempFiles()
		filestore.WFS.FlushCache(ctx)
		watcher := wconfig.GetWatcher()
		if watcher != nil {
			watcher.Close()
		}
		time.Sleep(500 * time.Millisecond)
		log.Printf("shutdown complete\n")
		os.Exit(0)
	})
}

// initReactiveHandler initializes the reactive messaging system for agent-to-agent communication.
// This wires up the input sender to the block controller.
func initReactiveHandler() {
	// Create input sender that uses blockcontroller.SendInput
	inputSender := func(blockId string, inputData []byte) error {
		return blockcontroller.SendInput(blockId, &blockcontroller.BlockInputUnion{
			InputData: inputData,
		})
	}

	// Initialize the global handler with the input sender
	reactive.InitGlobalHandler(inputSender)

	// Sync existing agent registrations from blocks (in background)
	go func() {
		defer func() {
			panichandler.PanicHandler("SyncAgentsFromBlocks", recover())
		}()
		// Wait a bit for the system to stabilize
		time.Sleep(2 * time.Second)
		ctx := context.Background()
		if err := reactive.GetGlobalHandler().SyncAgentsFromBlocks(ctx); err != nil {
			log.Printf("warning: failed to sync agent registrations: %v\n", err)
		} else {
			log.Printf("[reactive] agent sync complete\n")
		}
	}()

	// Start cross-host polling service (if configured)
	go func() {
		defer func() {
			panichandler.PanicHandler("StartGlobalPoller", recover())
		}()
		// Wait for agent sync to complete first
		time.Sleep(3 * time.Second)
		if err := reactive.StartGlobalPoller(); err != nil {
			log.Printf("warning: failed to start cross-host poller: %v\n", err)
		}
	}()

	log.Printf("[reactive] handler initialized\n")
}

// watch stdin, kill server if stdin is closed
func stdinReadWatch() {
	buf := make([]byte, 1024)
	for {
		_, err := os.Stdin.Read(buf)
		if err != nil {
			doShutdown(fmt.Sprintf("stdin closed/error (%v)", err))
			break
		}
	}
}

func startConfigWatcher() {
	watcher := wconfig.GetWatcher()
	if watcher != nil {
		watcher.Start()
	}
}

func telemetryLoop() {
	var nextSend int64
	time.Sleep(InitialTelemetryWait)
	for {
		if time.Now().Unix() > nextSend {
			nextSend = time.Now().Add(TelemetryInterval).Unix()
			sendTelemetryWrapper()
		}
		time.Sleep(TelemetryTick)
	}
}

func panicTelemetryHandler(panicName string) {
	activity := wshrpc.ActivityUpdate{NumPanics: 1}
	err := telemetry.UpdateActivity(context.Background(), activity)
	if err != nil {
		log.Printf("error updating activity (panicTelemetryHandler): %v\n", err)
	}
	telemetry.RecordTEvent(context.Background(), telemetrydata.MakeTEvent("debug:panic", telemetrydata.TEventProps{
		PanicType: panicName,
	}))
}

func sendTelemetryWrapper() {
	defer func() {
		panichandler.PanicHandler("sendTelemetryWrapper", recover())
	}()
	ctx, cancelFn := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancelFn()
	beforeSendActivityUpdate(ctx)
	client, err := wstore.DBGetSingleton[*waveobj.Client](ctx)
	if err != nil {
		log.Printf("[error] getting client data for telemetry: %v\n", err)
		return
	}
	err = wcloud.SendAllTelemetry(client.OID)
	if err != nil {
		log.Printf("[error] sending telemetry: %v\n", err)
	}
}

func updateTelemetryCounts(lastCounts telemetrydata.TEventProps) telemetrydata.TEventProps {
	ctx, cancelFn := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancelFn()
	var props telemetrydata.TEventProps
	props.CountBlocks, _ = wstore.DBGetCount[*waveobj.Block](ctx)
	props.CountTabs, _ = wstore.DBGetCount[*waveobj.Tab](ctx)
	props.CountWindows, _ = wstore.DBGetCount[*waveobj.Window](ctx)
	props.CountWorkspaces, _, _ = wstore.DBGetWSCounts(ctx)
	props.CountSSHConn = conncontroller.GetNumSSHHasConnected()
	props.CountWSLConn = wslconn.GetNumWSLHasConnected()
	props.CountViews, _ = wstore.DBGetBlockViewCounts(ctx)

	fullConfig := wconfig.GetWatcher().GetFullConfig()
	customWidgets := fullConfig.CountCustomWidgets()
	customAIPresets := fullConfig.CountCustomAIPresets()
	customSettings := wconfig.CountCustomSettings()

	props.UserSet = &telemetrydata.TEventUserProps{
		SettingsCustomWidgets:   customWidgets,
		SettingsCustomAIPresets: customAIPresets,
		SettingsCustomSettings:  customSettings,
	}

	if utilfn.CompareAsMarshaledJson(props, lastCounts) {
		return lastCounts
	}
	tevent := telemetrydata.MakeTEvent("app:counts", props)
	err := telemetry.RecordTEvent(ctx, tevent)
	if err != nil {
		log.Printf("error recording counts tevent: %v\n", err)
	}
	return props
}

func updateTelemetryCountsLoop() {
	defer func() {
		panichandler.PanicHandler("updateTelemetryCountsLoop", recover())
	}()
	var nextSend int64
	var lastCounts telemetrydata.TEventProps
	time.Sleep(TelemetryInitialCountsWait)
	for {
		if time.Now().Unix() > nextSend {
			nextSend = time.Now().Add(TelemetryCountsInterval).Unix()
			lastCounts = updateTelemetryCounts(lastCounts)
		}
		time.Sleep(TelemetryTick)
	}
}

func beforeSendActivityUpdate(ctx context.Context) {
	activity := wshrpc.ActivityUpdate{}
	activity.NumTabs, _ = wstore.DBGetCount[*waveobj.Tab](ctx)
	activity.NumBlocks, _ = wstore.DBGetCount[*waveobj.Block](ctx)
	activity.Blocks, _ = wstore.DBGetBlockViewCounts(ctx)
	activity.NumWindows, _ = wstore.DBGetCount[*waveobj.Window](ctx)
	activity.NumSSHConn = conncontroller.GetNumSSHHasConnected()
	activity.NumWSLConn = wslconn.GetNumWSLHasConnected()
	activity.NumWSNamed, activity.NumWS, _ = wstore.DBGetWSCounts(ctx)
	err := telemetry.UpdateActivity(ctx, activity)
	if err != nil {
		log.Printf("error updating before activity: %v\n", err)
	}
}

func startupActivityUpdate(firstLaunch bool) {
	ctx, cancelFn := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancelFn()
	activity := wshrpc.ActivityUpdate{Startup: 1}
	err := telemetry.UpdateActivity(ctx, activity) // set at least one record into activity (don't use go routine wrap here)
	if err != nil {
		log.Printf("error updating startup activity: %v\n", err)
	}
	autoUpdateChannel := telemetry.AutoUpdateChannel()
	autoUpdateEnabled := telemetry.IsAutoUpdateEnabled()
	shellType, shellVersion, shellErr := shellutil.DetectShellTypeAndVersion()
	if shellErr != nil {
		shellType = "error"
		shellVersion = ""
	}
	props := telemetrydata.TEventProps{
		UserSet: &telemetrydata.TEventUserProps{
			ClientVersion:     "v" + WaveVersion,
			ClientBuildTime:   BuildTime,
			ClientArch:        wavebase.ClientArch(),
			ClientOSRelease:   wavebase.UnameKernelRelease(),
			ClientIsDev:       wavebase.IsDevMode(),
			AutoUpdateChannel: autoUpdateChannel,
			AutoUpdateEnabled: autoUpdateEnabled,
			LocalShellType:    shellType,
			LocalShellVersion: shellVersion,
		},
		UserSetOnce: &telemetrydata.TEventUserProps{
			ClientInitialVersion: "v" + WaveVersion,
		},
	}
	if firstLaunch {
		props.AppFirstLaunch = true
	}
	tevent := telemetrydata.MakeTEvent("app:startup", props)
	err = telemetry.RecordTEvent(ctx, tevent)
	if err != nil {
		log.Printf("error recording startup event: %v\n", err)
	}
}

func shutdownActivityUpdate() {
	ctx, cancelFn := context.WithTimeout(context.Background(), 1*time.Second)
	defer cancelFn()
	activity := wshrpc.ActivityUpdate{Shutdown: 1}
	err := telemetry.UpdateActivity(ctx, activity) // do NOT use the go routine wrap here (this needs to be synchronous)
	if err != nil {
		log.Printf("error updating shutdown activity: %v\n", err)
	}
	err = telemetry.TruncateActivityTEventForShutdown(ctx)
	if err != nil {
		log.Printf("error truncating activity t-event for shutdown: %v\n", err)
	}
	tevent := telemetrydata.MakeTEvent("app:shutdown", telemetrydata.TEventProps{})
	err = telemetry.RecordTEvent(ctx, tevent)
	if err != nil {
		log.Printf("error recording shutdown event: %v\n", err)
	}
}

func createMainWshClient() {
	rpc := wshserver.GetMainRpcClient()
	wshfs.RpcClient = rpc
	wshutil.DefaultRouter.RegisterRoute(wshutil.DefaultRoute, rpc, true)
	wps.Broker.SetClient(wshutil.DefaultRouter)
	localConnWsh := wshutil.MakeWshRpc(nil, nil, wshrpc.RpcContext{Conn: wshrpc.LocalConnName}, &wshremote.ServerImpl{}, "conn:local")
	go wshremote.RunSysInfoLoop(localConnWsh, wshrpc.LocalConnName)
	wshutil.DefaultRouter.RegisterRoute(wshutil.MakeConnectionRouteId(wshrpc.LocalConnName), localConnWsh, true)
}

func grabAndRemoveEnvVars() error {
	err := authkey.SetAuthKeyFromEnv()
	if err != nil {
		return fmt.Errorf("setting auth key: %v", err)
	}
	err = wavebase.CacheAndRemoveEnvVars()
	if err != nil {
		return err
	}
	err = wcloud.CacheAndRemoveEnvVars()
	if err != nil {
		return err
	}

	// Remove WAVETERM env vars that leak from prod => dev
	os.Unsetenv("WAVETERM_CLIENTID")
	os.Unsetenv("WAVETERM_WORKSPACEID")
	os.Unsetenv("WAVETERM_TABID")
	os.Unsetenv("WAVETERM_BLOCKID")
	os.Unsetenv("WAVETERM_CONN")
	os.Unsetenv("WAVETERM_JWT")
	os.Unsetenv("WAVETERM_VERSION")

	return nil
}

func clearTempFiles() error {
	ctx, cancelFn := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancelFn()
	client, err := wstore.DBGetSingleton[*waveobj.Client](ctx)
	if err != nil {
		return fmt.Errorf("error getting client: %v", err)
	}
	filestore.WFS.DeleteZone(ctx, client.TempOID)
	return nil
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lmicroseconds)
	log.SetPrefix("[agentmuxsrv] ")
	wavebase.WaveVersion = WaveVersion
	wavebase.BuildTime = BuildTime

	// Verify version consistency to detect stale cached binaries
	if WaveVersion != ExpectedVersion {
		log.Printf("========================================")
		log.Printf("⚠️  VERSION MISMATCH DETECTED")
		log.Printf("========================================")
		log.Printf("Expected: %s", ExpectedVersion)
		log.Printf("Actual:   %s", WaveVersion)
		log.Printf("BuildTime: %s", BuildTime)
		log.Printf("")
		log.Printf("This likely means:")
		log.Printf("  1. Stale binary in src-tauri/target/")
		log.Printf("  2. Binary not rebuilt after version bump")
		log.Printf("")
		log.Printf("To fix:")
		log.Printf("  rm -f dist/bin/agentmuxsrv.* src-tauri/target/*/agentmuxsrv*")
		log.Printf("  task build:backend")
		log.Printf("  task sync:dev:binaries")
		log.Printf("========================================")
		// Continue anyway in dev, but log prominently
	}

	err := grabAndRemoveEnvVars()
	if err != nil {
		log.Printf("[error] %v\n", err)
		return
	}
	err = service.ValidateServiceMap()
	if err != nil {
		log.Printf("error validating service map: %v\n", err)
		return
	}
	err = wavebase.EnsureWaveDataDir()
	if err != nil {
		log.Printf("error ensuring wave home dir: %v\n", err)
		return
	}
	err = wavebase.EnsureWaveDBDir()
	if err != nil {
		log.Printf("error ensuring wave db dir: %v\n", err)
		return
	}
	err = wavebase.EnsureWaveConfigDir()
	if err != nil {
		log.Printf("error ensuring wave config dir: %v\n", err)
		return
	}

	// TODO: rather than ensure this dir exists, we should let the editor recursively create parent dirs on save
	err = wavebase.EnsureWavePresetsDir()
	if err != nil {
		log.Printf("error ensuring wave presets dir: %v\n", err)
		return
	}
	waveLock, instanceID, instanceDataDir, err := wavebase.AcquireWaveLockWithAutoInstance()
	if err != nil {
		log.Printf("error acquiring wave lock: %v\n", err)
		log.Printf("\n")
		log.Printf("========================================\n")
		log.Printf("ERROR: Maximum number of instances (10) reached\n")
		log.Printf("========================================\n")
		log.Printf("\n")
		log.Printf("Please close an existing AgentMux window and try again.\n")
		log.Printf("\n")
		log.Printf("Currently running instances use these data directories:\n")
		log.Printf("  Default:     %s\n", wavebase.GetWaveDataDirForInstance(""))
		for i := 1; i <= 10; i++ {
			log.Printf("  instance-%d: %s\n", i, wavebase.GetWaveDataDirForInstance(fmt.Sprintf("instance-%d", i)))
		}
		log.Printf("========================================\n")

		// Write startup error file for frontend to detect
		errorMessage := fmt.Sprintf(`========================================
ERROR: Maximum number of instances (10) reached
========================================

Please close an existing AgentMux window and try again.

Currently running instances use these data directories:
  Default:     %s
`, wavebase.GetWaveDataDirForInstance(""))
		for i := 1; i <= 10; i++ {
			errorMessage += fmt.Sprintf("  instance-%d: %s\n", i, wavebase.GetWaveDataDirForInstance(fmt.Sprintf("instance-%d", i)))
		}
		errorMessage += "========================================"

		errorFilePath := filepath.Join(wavebase.GetWaveDataDir(), "startup-error.txt")
		_ = os.WriteFile(errorFilePath, []byte(errorMessage), 0644)

		// Exit after delay so frontend can read the error file
		time.Sleep(2 * time.Second)
		return
	}

	// Store instance ID globally for later access
	CurrentInstanceID = instanceID

	// CRITICAL: Update the global data directory cache before any other operations
	// This must be done immediately after lock acquisition to ensure all subsequent
	// operations use the correct instance-specific data directory
	// instanceID is now always set: "default", "instance-1", "instance-2", etc.
	if instanceDataDir != "" {
		wavebase.DataHome_VarCache = instanceDataDir
		log.Printf("[multi-instance] Running as instance: %s (data: %s)\n", instanceID, instanceDataDir)
	}

	// Ensure db subdirectory exists for this instance
	// Database initialization expects this directory to exist
	dbDir := filepath.Join(wavebase.GetWaveDataDir(), wavebase.WaveDBDir)
	err = wavebase.TryMkdirs(dbDir, 0700, "database directory")
	if err != nil {
		log.Printf("error creating db directory: %v\n", err)
		return
	}

	// Clean up any old startup error file from previous failed attempts
	errorFilePath := filepath.Join(wavebase.GetWaveDataDir(), "startup-error.txt")
	_ = os.Remove(errorFilePath)

	// Write instance ID to file for frontend to display in window title
	instanceIDFile := filepath.Join(wavebase.GetWaveDataDir(), "instance-id.txt")
	_ = os.WriteFile(instanceIDFile, []byte(instanceID), 0644)
	defer func() {
		err = waveLock.Close()
		if err != nil {
			log.Printf("error releasing wave lock: %v\n", err)
		}
	}()
	log.Printf("wave version: %s (%s)\n", WaveVersion, BuildTime)
	log.Printf("wave data dir: %s\n", wavebase.GetWaveDataDir())
	log.Printf("wave config dir: %s\n", wavebase.GetWaveConfigDir())
	err = filestore.InitFilestore()
	if err != nil {
		log.Printf("error initializing filestore: %v\n", err)
		return
	}
	err = wstore.InitWStore()
	if err != nil {
		log.Printf("error initializing wstore: %v\n", err)
		return
	}
	panichandler.PanicTelemetryHandler = panicTelemetryHandler
	go func() {
		defer func() {
			panichandler.PanicHandler("InitCustomShellStartupFiles", recover())
		}()
		err := shellutil.InitCustomShellStartupFiles()
		if err != nil {
			log.Printf("error initializing wsh and shell-integration files: %v\n", err)
		}
	}()
	// Clean up old version lock files (best effort, runs in background)
	go func() {
		defer func() {
			panichandler.PanicHandler("CleanupOldLockFiles", recover())
		}()
		time.Sleep(2 * time.Second) // Wait for system to stabilize
		if err := wavebase.CleanupOldLockFiles(); err != nil {
			log.Printf("warning: failed to cleanup old lock files: %v\n", err)
		}
	}()
	firstLaunch, err := wcore.EnsureInitialData()
	if err != nil {
		log.Printf("error ensuring initial data: %v\n", err)
		return
	}
	if firstLaunch {
		log.Printf("first launch detected")
	}

	// Migrate orphaned layout references (cleanup existing orphans)
	go func() {
		defer func() {
			panichandler.PanicHandler("MigrateOrphanedLayouts", recover())
		}()
		ctx, cancelFn := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancelFn()
		err := wcore.MigrateOrphanedLayouts(ctx)
		if err != nil {
			log.Printf("error migrating orphaned layouts: %v\n", err)
		}
	}()

	err = clearTempFiles()
	if err != nil {
		log.Printf("error clearing temp files: %v\n", err)
		return
	}

	createMainWshClient()
	sigutil.InstallShutdownSignalHandlers(doShutdown)
	sigutil.InstallSIGUSR1Handler()
	startConfigWatcher()

	// Initialize webhook service for reactive agent communication
	go func() {
		defer func() {
			panichandler.PanicHandler("InitWebhookService", recover())
		}()
		if err := webhookdelivery.InitializeWebhookService(); err != nil {
			log.Printf("warning: failed to initialize webhook service: %v\n", err)
		}
	}()

	go stdinReadWatch()
	go telemetryLoop()
	go updateTelemetryCountsLoop()
	go startupActivityUpdate(firstLaunch) // must be after startConfigWatcher()
	blocklogger.InitBlockLogger()
	initReactiveHandler() // Initialize reactive messaging for agent-to-agent communication
	go wavebase.GetSystemSummary() // get this cached (used in AI)

	// Start dedicated reactive server for Docker container access (if configured)
	if reactivePort := os.Getenv("WAVEMUX_REACTIVE_PORT"); reactivePort != "" {
		go web.RunReactiveServer(reactivePort)
	}

	webListener, err := web.MakeTCPListener("web")
	if err != nil {
		log.Printf("error creating web listener: %v\n", err)
		return
	}
	wsListener, err := web.MakeTCPListener("websocket")
	if err != nil {
		log.Printf("error creating websocket listener: %v\n", err)
		return
	}
	go web.RunWebSocketServer(wsListener)
	unixListener, err := web.MakeUnixListener()
	if err != nil {
		log.Printf("error creating unix listener: %v\n", err)
		return
	}
	go func() {
		if BuildTime == "" {
			BuildTime = "0"
		}
		// use fmt instead of log here to make sure it goes directly to stderr
		fmt.Fprintf(os.Stderr, "WAVESRV-ESTART ws:%s web:%s version:%s buildtime:%s instance:%s\n", wsListener.Addr(), webListener.Addr(), WaveVersion, BuildTime, CurrentInstanceID)
	}()
	go wshutil.RunWshRpcOverListener(unixListener)

	// Initialize system tray icon (runs in separate goroutine)
	// Skip on macOS: getlantern/systray CGO crashes on darwin/arm64
	if runtime.GOOS != "darwin" {
		InitTray()
	}

	web.RunWebServer(webListener) // blocking
	runtime.KeepAlive(waveLock)
}
