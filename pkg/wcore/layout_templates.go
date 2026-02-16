// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package wcore

import "github.com/a5af/agentmux/pkg/waveobj"

func GetStarterLayout() PortableLayout {
	// Simple layout: 1 terminal + 1 sysinfo panel
	// Reverted from 4-terminal layout to fix gamerlove startup issues.
	// The 4-terminal layout caused resource exhaustion on Windows sandbox.
	// Users can manually create additional terminals as needed.
	// Layout:
	//   +-----------------+
	//   | terminal        |
	//   | (focused)       |
	//   +-----------------+
	//   | sysinfo         |
	//   +-----------------+
	return PortableLayout{
		{IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{
			Meta: waveobj.MetaMapType{
				waveobj.MetaKey_View:       "term",
				waveobj.MetaKey_Controller: "shell",
			},
		}, Focused: true},
		{IndexArr: []int{1}, BlockDef: &waveobj.BlockDef{
			Meta: waveobj.MetaMapType{
				waveobj.MetaKey_View: "sysinfo",
			},
		}},
	}
}

func GetNewTabLayout() PortableLayout {
	return PortableLayout{
		{IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{
			Meta: waveobj.MetaMapType{
				waveobj.MetaKey_View:       "term",
				waveobj.MetaKey_Controller: "shell",
			},
		}, Focused: true},
	}
}
