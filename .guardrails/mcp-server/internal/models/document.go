package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

// Document represents a guardrail document stored in the database
type Document struct {
	ID           uuid.UUID      `json:"id" db:"id"`
	Slug         string         `json:"slug" db:"slug"`
	Title        string         `json:"title" db:"title"`
	Content      string         `json:"content" db:"content"`
	SearchVector string         `json:"-" db:"search_vector"`
	Category     string         `json:"category" db:"category"`
	Path         string         `json:"path" db:"path"`
	Version      int            `json:"version" db:"version"`
	Metadata     map[string]any `json:"metadata" db:"metadata"`
	Source       string         `json:"source" db:"source"`
	ContentHash  string         `json:"content_hash,omitempty" db:"content_hash"`
	FilePath     string         `json:"file_path,omitempty" db:"file_path"`
	Orphaned     bool           `json:"orphaned" db:"orphaned"`
	CreatedAt    time.Time      `json:"created_at" db:"created_at"`
	UpdatedAt    time.Time      `json:"updated_at" db:"updated_at"`
}

// DocumentCategory represents valid document categories
type DocumentCategory string

const (
	CategoryWorkflow  DocumentCategory = "workflow"
	CategoryStandard  DocumentCategory = "standard"
	CategoryGuide     DocumentCategory = "guide"
	CategoryReference DocumentCategory = "reference"
)

// IsValidCategory checks if a category is valid
func IsValidCategory(cat string) bool {
	switch DocumentCategory(cat) {
	case CategoryWorkflow, CategoryStandard, CategoryGuide, CategoryReference:
		return true
	}
	return false
}

// Validate checks if the document is valid for creation/update
func (d *Document) Validate() error {
	if d.Slug == "" {
		return fmt.Errorf("slug is required")
	}
	if len(d.Slug) > 255 {
		return fmt.Errorf("slug must be at most 255 characters")
	}
	if d.Title == "" {
		return fmt.Errorf("title is required")
	}
	if len(d.Title) > 500 {
		return fmt.Errorf("title must be at most 500 characters")
	}
	if d.Content == "" {
		return fmt.Errorf("content is required")
	}
	if !IsValidCategory(d.Category) {
		return fmt.Errorf("invalid category: %s", d.Category)
	}
	if d.Path == "" {
		return fmt.Errorf("path is required")
	}
	if len(d.Path) > 500 {
		return fmt.Errorf("path must be at most 500 characters")
	}
	if d.Version < 1 {
		d.Version = 1
	}
	return nil
}
