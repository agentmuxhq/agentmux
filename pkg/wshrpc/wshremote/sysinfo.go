// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package wshremote

import (
	"log"
	"strconv"
	"sync"
	"time"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/mem"
	"github.com/shirou/gopsutil/v4/net"
	"github.com/a5af/wavemux/pkg/wps"
	"github.com/a5af/wavemux/pkg/wshrpc"
	"github.com/a5af/wavemux/pkg/wshrpc/wshclient"
	"github.com/a5af/wavemux/pkg/wshutil"
)

const BYTES_PER_GB = 1073741824
const BYTES_PER_MB = 1048576

// Network I/O tracking state
var (
	prevNetStats     net.IOCountersStat
	prevNetTimestamp time.Time
	netStatsMutex    sync.Mutex
)

func getCpuData(values map[string]float64) {
	percentArr, err := cpu.Percent(0, false)
	if err != nil {
		return
	}
	if len(percentArr) > 0 {
		values[wshrpc.TimeSeries_Cpu] = percentArr[0]
	}
	percentArr, err = cpu.Percent(0, true)
	if err != nil {
		return
	}
	for idx, percent := range percentArr {
		values[wshrpc.TimeSeries_Cpu+":"+strconv.Itoa(idx)] = percent
	}
}

func getMemData(values map[string]float64) {
	memData, err := mem.VirtualMemory()
	if err != nil {
		return
	}
	values["mem:total"] = float64(memData.Total) / BYTES_PER_GB
	values["mem:available"] = float64(memData.Available) / BYTES_PER_GB
	values["mem:used"] = float64(memData.Used) / BYTES_PER_GB
	values["mem:free"] = float64(memData.Free) / BYTES_PER_GB
}

func getNetData(values map[string]float64) {
	netStatsMutex.Lock()
	defer netStatsMutex.Unlock()

	// Get current network I/O counters (aggregated across all interfaces)
	netStats, err := net.IOCounters(false)
	if err != nil || len(netStats) == 0 {
		return
	}
	currentStats := netStats[0] // Aggregated stats for all interfaces
	currentTime := time.Now()

	// Calculate rates if we have previous data
	if !prevNetTimestamp.IsZero() {
		timeDelta := currentTime.Sub(prevNetTimestamp).Seconds()
		if timeDelta > 0 {
			// Calculate bytes per second, then convert to MB/s
			bytesSentPerSec := float64(currentStats.BytesSent-prevNetStats.BytesSent) / timeDelta
			bytesRecvPerSec := float64(currentStats.BytesRecv-prevNetStats.BytesRecv) / timeDelta

			values["net:bytessent"] = bytesSentPerSec / BYTES_PER_MB  // MB/s
			values["net:bytesrecv"] = bytesRecvPerSec / BYTES_PER_MB  // MB/s
			values["net:bytestotal"] = (bytesSentPerSec + bytesRecvPerSec) / BYTES_PER_MB // MB/s
		}
	}

	// Update previous stats for next iteration
	prevNetStats = currentStats
	prevNetTimestamp = currentTime
}

func generateSingleServerData(client *wshutil.WshRpc, connName string) {
	now := time.Now()
	values := make(map[string]float64)
	getCpuData(values)
	getMemData(values)
	getNetData(values)
	tsData := wshrpc.TimeSeriesData{Ts: now.UnixMilli(), Values: values}
	event := wps.WaveEvent{
		Event:   wps.Event_SysInfo,
		Scopes:  []string{connName},
		Data:    tsData,
		Persist: 1024,
	}
	wshclient.EventPublishCommand(client, event, &wshrpc.RpcOpts{NoResponse: true})
}

func RunSysInfoLoop(client *wshutil.WshRpc, connName string) {
	defer func() {
		log.Printf("sysinfo loop ended conn:%s\n", connName)
	}()
	for {
		generateSingleServerData(client, connName)
		time.Sleep(1 * time.Second)
	}
}
