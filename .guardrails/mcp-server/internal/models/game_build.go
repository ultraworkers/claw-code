package models

import (
	"fmt"
	"time"
)

// GameBuildStatus represents the status of a game build validation
type GameBuildStatus string

const (
	BuildPassed  GameBuildStatus = "passed"
	BuildFailed  GameBuildStatus = "failed"
	BuildErrored GameBuildStatus = "error"
	BuildSkipped GameBuildStatus = "skipped"
)

// GameEngine represents supported game engines
type GameEngine string

const (
	EngineGodot  GameEngine = "godot"
	EngineUnity  GameEngine = "unity"
	EngineUnreal GameEngine = "unreal"
)

// GameBuildResult represents the result of a game build validation
type GameBuildResult struct {
	SessionToken string          `json:"session_token"`
	Engine       GameEngine      `json:"engine"`
	ProjectPath  string          `json:"project_path"`
	Status       GameBuildStatus `json:"status"`
	TestsRun     int             `json:"tests_run"`
	TestsPassed  int             `json:"tests_passed"`
	TestsFailed  int             `json:"tests_failed"`
	DurationMs   int64           `json:"duration_ms"`
	Errors       []BuildError      `json:"errors,omitempty"`
	ValidatedAt  time.Time       `json:"validated_at"`
}

// BuildError represents a single error from game build validation
type BuildError struct {
	Type      BuildErrorType `json:"type"`
	File      string             `json:"file,omitempty"`
	Line      int                `json:"line,omitempty"`
	Message   string             `json:"message"`
	Severity  string             `json:"severity"`
}

// BuildErrorType represents the type of game build error
type BuildErrorType string

const (
	GameErrorScene      BuildErrorType = "scene_parse"
	GameErrorScript     BuildErrorType = "script_error"
	GameErrorExport     BuildErrorType = "export_config"
	GameErrorTest       BuildErrorType = "test_failure"
	GameErrorGeneric    BuildErrorType = "generic"
)

// Summary returns a human-readable summary of the build result
func (r *GameBuildResult) Summary() string {
	return fmt.Sprintf("Game Build [%s]: %s (%d/%d tests passed, %dms)",
		r.Engine, r.Status, r.TestsPassed, r.TestsRun, r.DurationMs)
}
