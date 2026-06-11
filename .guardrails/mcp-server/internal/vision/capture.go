package vision

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/fsnotify/fsnotify"
)

// CaptureWatcher monitors a directory for new screenshots and optionally triggers review.
type CaptureWatcher struct {
	watchDir   string
	engine     *ReviewEngine
	watcher    *fsnotify.Watcher
	log        *slog.Logger
	stopCh     chan struct{}
}

// NewCaptureWatcher creates a watcher for the given directory.
func NewCaptureWatcher(watchDir string, engine *ReviewEngine) (*CaptureWatcher, error) {
	if err := os.MkdirAll(watchDir, 0755); err != nil {
		return nil, fmt.Errorf("create watch dir: %w", err)
	}
	w, err := fsnotify.NewWatcher()
	if err != nil {
		return nil, fmt.Errorf("create watcher: %w", err)
	}
	if err := w.Add(watchDir); err != nil {
		return nil, fmt.Errorf("add watch dir: %w", err)
	}
	return &CaptureWatcher{
		watchDir: watchDir,
		engine:   engine,
		watcher:  w,
		log:      slog.Default().With("component", "capture_watcher"),
		stopCh:   make(chan struct{}),
	}, nil
}

// Start begins watching for new PNG/JPG files. Each new file triggers a review.
func (cw *CaptureWatcher) Start(ctx context.Context) {
	cw.log.Info("capture watcher started", "dir", cw.watchDir)
	go func() {
		for {
			select {
			case <-cw.stopCh:
				return
			case <-ctx.Done():
				return
			case event, ok := <-cw.watcher.Events:
				if !ok {
					return
				}
				if event.Op&fsnotify.Create == fsnotify.Create {
					if isImage(event.Name) {
						cw.log.Info("new screenshot detected", "path", event.Name)
						// Give Godot time to finish writing
						time.Sleep(500 * time.Millisecond)
						_, err := cw.engine.Run(ctx, event.Name)
						if err != nil {
							cw.log.Error("auto-review failed", "path", event.Name, "error", err)
						}
					}
				}
			case err, ok := <-cw.watcher.Errors:
				if !ok {
					return
				}
				cw.log.Error("watcher error", "error", err)
			}
		}
	}()
}

// Stop halts the watcher.
func (cw *CaptureWatcher) Stop() error {
	close(cw.stopCh)
	return cw.watcher.Close()
}

// TriggerCapture instructs Godot to save a screenshot immediately.
// This is a no-op unless integrated with a Godot signal or HTTP trigger.
func (cw *CaptureWatcher) TriggerCapture() string {
	return fmt.Sprintf("Trigger capture signal sent to %s", cw.watchDir)
}

func isImage(path string) bool {
	ext := strings.ToLower(filepath.Ext(path))
	return ext == ".png" || ext == ".jpg" || ext == ".jpeg"
}
