package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

// FileRead represents a record of a file being read within a session
type FileRead struct {
	ID          uuid.UUID `json:"id" db:"id"`
	SessionID   string    `json:"session_id" db:"session_id"`
	FilePath    string    `json:"file_path" db:"file_path"`
	ReadAt      time.Time `json:"read_at" db:"read_at"`
	ContentHash *string   `json:"content_hash,omitempty" db:"content_hash"`
	CreatedAt   time.Time `json:"created_at" db:"created_at"`
}

// Validate checks if the file read record is valid for creation
func (f *FileRead) Validate() error {
	if f.SessionID == "" {
		return fmt.Errorf("session_id is required")
	}
	if len(f.SessionID) > 100 {
		return fmt.Errorf("session_id must be at most 100 characters")
	}
	if f.FilePath == "" {
		return fmt.Errorf("file_path is required")
	}
	if len(f.FilePath) > 500 {
		return fmt.Errorf("file_path must be at most 500 characters")
	}
	return nil
}
