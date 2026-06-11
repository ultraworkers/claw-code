package models

import (
	"time"

	"github.com/google/uuid"
)

// DocumentSource represents the source of a document
type DocumentSource string

const (
	SourceSystem DocumentSource = "system"
	SourceRepo   DocumentSource = "repo"
	SourceUpload DocumentSource = "upload"
)

// IsValidSource checks if a document source is valid
func IsValidSource(source string) bool {
	switch DocumentSource(source) {
	case SourceSystem, SourceRepo, SourceUpload:
		return true
	}
	return false
}

// IngestJob represents a document ingestion job
type IngestJob struct {
	ID              uuid.UUID       `json:"id" db:"id"`
	Source          string          `json:"source" db:"source"`
	Status          IngestJobStatus `json:"status" db:"status"`
	StartedAt       time.Time       `json:"started_at" db:"started_at"`
	CompletedAt     *time.Time      `json:"completed_at,omitempty" db:"completed_at"`
	FilesProcessed  int             `json:"files_processed" db:"files_processed"`
	FilesAdded      int             `json:"files_added" db:"files_added"`
	FilesUpdated    int             `json:"files_updated" db:"files_updated"`
	FilesOrphaned   int             `json:"files_orphaned" db:"files_orphaned"`
	Errors          []IngestError   `json:"errors" db:"errors"`
	Metadata        map[string]any  `json:"metadata" db:"metadata"`
	CreatedBy       string          `json:"created_by" db:"created_by"`
}

// IngestJobStatus represents the status of an ingest job
type IngestJobStatus string

const (
	IngestStatusPending    IngestJobStatus = "pending"
	IngestStatusRunning    IngestJobStatus = "running"
	IngestStatusCompleted  IngestJobStatus = "completed"
	IngestStatusFailed     IngestJobStatus = "failed"
)

// IngestError represents an error during ingestion
type IngestError struct {
	File    string `json:"file"`
	Message string `json:"message"`
}

// ParsedDocument represents a document parsed from a file
type ParsedDocument struct {
	Title     string
	Content   string
	Category  string
	Slug      string
	Version   string
	Metadata  map[string]any
	FilePath  string
	ContentHash string
}

// UpdateCheck represents the result of an update check
type UpdateCheck struct {
	ID                         uuid.UUID      `json:"id" db:"id"`
	CheckedAt                  time.Time      `json:"checked_at" db:"checked_at"`
	DockerCurrentVersion       string         `json:"docker_current_version" db:"docker_current_version"`
	DockerLatestVersion        string         `json:"docker_latest_version" db:"docker_latest_version"`
	DockerReleaseNotes         string         `json:"docker_release_notes" db:"docker_release_notes"`
	DockerUpdateAvailable      bool           `json:"docker_update_available" db:"docker_update_available"`
	GuardrailCurrentCommit     string         `json:"guardrail_current_commit" db:"guardrail_current_commit"`
	GuardrailLatestCommit      string         `json:"guardrail_latest_commit" db:"guardrail_latest_commit"`
	GuardrailNewFiles          int            `json:"guardrail_new_files" db:"guardrail_new_files"`
	GuardrailModifiedFiles     int            `json:"guardrail_modified_files" db:"guardrail_modified_files"`
	GuardrailDeletedFiles      int            `json:"guardrail_deleted_files" db:"guardrail_deleted_files"`
	GuardrailUpdateAvailable   bool           `json:"guardrail_update_available" db:"guardrail_update_available"`
	Metadata                   map[string]any `json:"metadata" db:"metadata"`
}

// UpdateStatusResponse is the API response for update status
type UpdateStatusResponse struct {
	LastChecked         time.Time           `json:"last_checked"`
	DockerUpdate        *DockerUpdateInfo   `json:"docker_update,omitempty"`
	GuardrailUpdate     *GuardrailUpdateInfo `json:"guardrail_update,omitempty"`
}

// DockerUpdateInfo represents available Docker update
type DockerUpdateInfo struct {
	CurrentVersion string `json:"current_version"`
	LatestVersion  string `json:"latest_version"`
	ReleaseNotes   string `json:"release_notes"`
}

// GuardrailUpdateInfo represents available guardrail update
type GuardrailUpdateInfo struct {
	CurrentCommit  string `json:"current_commit"`
	LatestCommit   string `json:"latest_commit"`
	NewFiles       int    `json:"new_files"`
	ModifiedFiles  int    `json:"modified_files"`
	DeletedFiles   int    `json:"deleted_files"`
}
