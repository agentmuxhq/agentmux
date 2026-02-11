package wavefileutil

import (
	"fmt"

	"github.com/a5af/wavemux/pkg/filestore"
	"github.com/a5af/wavemux/pkg/remote/fileshare/fsutil"
	"github.com/a5af/wavemux/pkg/util/fileutil"
	"github.com/a5af/wavemux/pkg/wshrpc"
)

const (
	MuxFilePathPattern = "muxfile://%s/%s"
)

func WaveFileToFileInfo(wf *filestore.WaveFile) *wshrpc.FileInfo {
	path := fmt.Sprintf(MuxFilePathPattern, wf.ZoneId, wf.Name)
	rtn := &wshrpc.FileInfo{
		Path:          path,
		Dir:           fsutil.GetParentPathString(path),
		Name:          wf.Name,
		Opts:          &wf.Opts,
		Size:          wf.Size,
		Meta:          &wf.Meta,
		SupportsMkdir: false,
	}
	fileutil.AddMimeTypeToFileInfo(path, rtn)
	return rtn
}

func WaveFileListToFileInfoList(wfList []*filestore.WaveFile) []*wshrpc.FileInfo {
	var fileInfoList []*wshrpc.FileInfo
	for _, wf := range wfList {
		fileInfoList = append(fileInfoList, WaveFileToFileInfo(wf))
	}
	return fileInfoList
}
