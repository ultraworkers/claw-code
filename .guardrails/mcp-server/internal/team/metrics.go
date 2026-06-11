// Package team provides team management functionality with performance metrics.
package team

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"
)

// OperationStats tracks statistics for a single operation type.
type OperationStats struct {
	Count           int   `json:"count"`
	TotalDurationMs int64 `json:"total_duration_ms"`
	Success         int   `json:"success"`
	Failures        int   `json:"failures"`
}

// MetricsData represents the persisted metrics format.
type MetricsData struct {
	Project     string                     `json:"project"`
	Operations  map[string]*OperationStats `json:"operations"`
	LastUpdated time.Time                  `json:"last_updated"`
}

// PerformanceMetrics tracks operation timing and success rates.
// Thread-safe using sync.Mutex for concurrent access.
type PerformanceMetrics struct {
	mu          sync.Mutex
	projectName string
	metricsDir  string
	metricsPath string
	data        *MetricsData
}

// NewPerformanceMetrics creates a new metrics collector for the project.
// Initializes the metrics directory and loads any existing metrics.
func NewPerformanceMetrics(projectName string) *PerformanceMetrics {
	metricsDir := ".teams"
	metricsPath := filepath.Join(metricsDir, "metrics.json")

	pm := &PerformanceMetrics{
		projectName: projectName,
		metricsDir:  metricsDir,
		metricsPath: metricsPath,
		data: &MetricsData{
			Project:     projectName,
			Operations:  make(map[string]*OperationStats),
			LastUpdated: time.Now().UTC(),
		},
	}

	// Ensure metrics directory exists
	_ = os.MkdirAll(metricsDir, 0755)

	// Load existing metrics
	_ = pm.LoadMetrics()

	return pm
}

// RecordOperation records timing and success status for an operation.
// Duration is converted to milliseconds for storage.
func (pm *PerformanceMetrics) RecordOperation(operation string, duration time.Duration, success bool) {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	stats, exists := pm.data.Operations[operation]
	if !exists {
		stats = &OperationStats{}
		pm.data.Operations[operation] = stats
	}

	stats.Count++
	stats.TotalDurationMs += duration.Milliseconds()
	if success {
		stats.Success++
	} else {
		stats.Failures++
	}
	pm.data.LastUpdated = time.Now().UTC()
}

// GetMetrics returns a copy of all metrics data.
// Returns a map suitable for serialization or inspection.
func (pm *PerformanceMetrics) GetMetrics() map[string]interface{} {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	// Copy operations map
	opsCopy := make(map[string]OperationStats, len(pm.data.Operations))
	for op, stats := range pm.data.Operations {
		opsCopy[op] = *stats
	}

	return map[string]interface{}{
		"project":      pm.data.Project,
		"operations":   opsCopy,
		"last_updated": pm.data.LastUpdated.Format(time.RFC3339),
	}
}

// GetAverageDuration returns the average duration for a specific operation.
// Returns 0 if the operation has not been recorded.
func (pm *PerformanceMetrics) GetAverageDuration(operation string) time.Duration {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	stats, exists := pm.data.Operations[operation]
	if !exists || stats.Count == 0 {
		return 0
	}

	avgMs := stats.TotalDurationMs / int64(stats.Count)
	return time.Duration(avgMs) * time.Millisecond
}

// GetSuccessRate returns the success rate (0.0 to 1.0) for a specific operation.
// Returns 1.0 if the operation has not been recorded.
func (pm *PerformanceMetrics) GetSuccessRate(operation string) float64 {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	stats, exists := pm.data.Operations[operation]
	if !exists || stats.Count == 0 {
		return 1.0
	}

	return float64(stats.Success) / float64(stats.Count)
}

// SaveMetrics persists metrics to disk in JSON format.
// Creates the metrics directory if it doesn't exist.
func (pm *PerformanceMetrics) SaveMetrics() error {
	pm.mu.Lock()
	data := *pm.data
	// Deep copy operations map
	data.Operations = make(map[string]*OperationStats, len(pm.data.Operations))
	for op, stats := range pm.data.Operations {
		statCopy := *stats
		data.Operations[op] = &statCopy
	}
	pm.mu.Unlock()

	// Update timestamp
	data.LastUpdated = time.Now().UTC()

	// Ensure directory exists
	if err := os.MkdirAll(pm.metricsDir, 0755); err != nil {
		return fmt.Errorf("failed to create metrics directory: %w", err)
	}

	// Write to temporary file first for atomicity
	tempPath := pm.metricsPath + ".tmp"
	file, err := os.Create(tempPath)
	if err != nil {
		return fmt.Errorf("failed to create temp file: %w", err)
	}

	encoder := json.NewEncoder(file)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(data); err != nil {
		file.Close()
		os.Remove(tempPath)
		return fmt.Errorf("failed to encode metrics: %w", err)
	}

	if err := file.Close(); err != nil {
		os.Remove(tempPath)
		return fmt.Errorf("failed to close temp file: %w", err)
	}

	// Atomic rename
	if err := os.Rename(tempPath, pm.metricsPath); err != nil {
		os.Remove(tempPath)
		return fmt.Errorf("failed to save metrics: %w", err)
	}

	return nil
}

// LoadMetrics loads metrics from disk if the file exists.
// No error is returned if the file doesn't exist.
func (pm *PerformanceMetrics) LoadMetrics() error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	data, err := os.ReadFile(pm.metricsPath)
	if err != nil {
		if os.IsNotExist(err) {
			// No existing metrics, start fresh
			return nil
		}
		return fmt.Errorf("failed to read metrics file: %w", err)
	}

	var loaded MetricsData
	if err := json.Unmarshal(data, &loaded); err != nil {
		return fmt.Errorf("failed to parse metrics file: %w", err)
	}

	// Ensure project name is preserved if loading mismatched file
	if loaded.Project != "" && loaded.Project != pm.projectName {
		// Keep current project name, but load operations
		pm.data.Project = pm.projectName
	} else {
		pm.data.Project = loaded.Project
	}

	// Initialize operations map if nil
	if loaded.Operations == nil {
		loaded.Operations = make(map[string]*OperationStats)
	}
	pm.data.Operations = loaded.Operations
	pm.data.LastUpdated = loaded.LastUpdated

	return nil
}

// GetOperationStats returns detailed stats for a specific operation.
func (pm *PerformanceMetrics) GetOperationStats(operation string) (count int, avgDuration time.Duration, successRate float64) {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	stats, exists := pm.data.Operations[operation]
	if !exists || stats.Count == 0 {
		return 0, 0, 1.0
	}

	avgMs := stats.TotalDurationMs / int64(stats.Count)
	return stats.Count, time.Duration(avgMs) * time.Millisecond, float64(stats.Success) / float64(stats.Count)
}

// Reset clears all metrics data for the current project.
func (pm *PerformanceMetrics) Reset() {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	pm.data.Operations = make(map[string]*OperationStats)
	pm.data.LastUpdated = time.Now().UTC()
}
