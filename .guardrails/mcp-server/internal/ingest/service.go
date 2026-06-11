package ingest

import (
	"context"
	"fmt"
	"log/slog"
	"io"
	"os"
	"path/filepath"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// Service handles document ingestion operations
type Service struct {
	docStore      *database.DocumentStore
	ruleStore     *database.RuleStore
	parser        *Parser
	ruleParser    *RuleParser
	ruleSyncSvc   *RuleSyncService
	watchedDirs   []string
	rulesDir      string
}

// NewService creates a new ingest service
func NewService(docStore *database.DocumentStore, ruleStore *database.RuleStore, watchedDirs []string, rulesDir string) *Service {
	ruleParser := NewRuleParser()
	ruleSyncSvc := NewRuleSyncService(ruleStore)

	return &Service{
		docStore:    docStore,
		ruleStore:   ruleStore,
		parser:      NewParser(),
		ruleParser:  ruleParser,
		ruleSyncSvc: ruleSyncSvc,
		watchedDirs: watchedDirs,
		rulesDir:    rulesDir,
	}
}

// SyncFromRepo syncs documents from the git repo
func (s *Service) SyncFromRepo(ctx context.Context, jobID uuid.UUID) error {
	return s.syncDirectory(ctx, jobID, "repo", s.watchedDirs)
}

// SyncFromUpload handles uploaded files
func (s *Service) SyncFromUpload(ctx context.Context, jobID uuid.UUID, files map[string][]byte) error {
	stats := &syncStats{}
	var errors []models.IngestError

	for filename, content := range files {
		if !IsMarkdownFile(filename) {
			continue
		}

		stats.processed++

		doc, err := s.parser.ParseContent(string(content), filename)
		if err != nil {
			errors = append(errors, models.IngestError{
				File:    filename,
				Message: err.Error(),
			})
			continue
		}

		// Check if document already exists by file path
		existing, err := s.docStore.GetBySlug(ctx, doc.Slug)
		if err == nil && existing != nil {
			// Update existing document
			existing.Title = doc.Title
			existing.Content = doc.Content
			existing.Category = doc.Category
			existing.Metadata = doc.Metadata
			existing.ContentHash = doc.ContentHash
			existing.FilePath = doc.FilePath
			existing.Source = string(models.SourceUpload)
			existing.Orphaned = false

			if err := s.docStore.Update(ctx, existing); err != nil {
				errors = append(errors, models.IngestError{
					File:    filename,
					Message: fmt.Sprintf("update failed: %v", err),
				})
				continue
			}
			stats.updated++
		} else {
			// Create new document
			newDoc := &models.Document{
				ID:          uuid.New(),
				Slug:        doc.Slug,
				Title:       doc.Title,
				Content:     doc.Content,
				Category:    doc.Category,
				Path:        fmt.Sprintf("/docs/%s", doc.Slug),
				Version:     1,
				Metadata:    doc.Metadata,
				Source:      string(models.SourceUpload),
				ContentHash: doc.ContentHash,
				FilePath:    doc.FilePath,
				Orphaned:    false,
			}

			if err := s.docStore.Create(ctx, newDoc); err != nil {
				errors = append(errors, models.IngestError{
					File:    filename,
					Message: fmt.Sprintf("create failed: %v", err),
				})
				continue
			}
			stats.added++
		}
	}

	return s.completeJob(ctx, jobID, stats, errors)
}

// syncStats tracks sync statistics
type syncStats struct {
	processed int
	added     int
	updated   int
	orphaned  int
}

// syncDirectory syncs documents from watched directories
func (s *Service) syncDirectory(ctx context.Context, jobID uuid.UUID, source string, dirs []string) error {
	stats := &syncStats{}
	var errors []models.IngestError
	processedPaths := make(map[string]bool)

	for _, dir := range dirs {
		if err := s.walkDirectory(ctx, dir, source, stats, &errors, processedPaths); err != nil {
			// Log error but continue with other directories
			errors = append(errors, models.IngestError{
				File:    dir,
				Message: fmt.Sprintf("walk failed: %v", err),
			})
		}
	}

	// Mark orphaned documents
	orphanCount, err := s.markOrphans(ctx, processedPaths)
	if err != nil {
		errors = append(errors, models.IngestError{
			File:    "orphan-check",
			Message: fmt.Sprintf("orphan check failed: %v", err),
		})
	}
	stats.orphaned = orphanCount

	return s.completeJob(ctx, jobID, stats, errors)
}

// walkDirectory walks a directory and processes markdown files
func (s *Service) walkDirectory(ctx context.Context, dir, source string, stats *syncStats, errors *[]models.IngestError, processedPaths map[string]bool) error {
	return filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		// Skip directories
		if info.IsDir() {
			return nil
		}

		// Only process markdown files
		if !IsMarkdownFile(path) {
			return nil
		}

		stats.processed++
		processedPaths[path] = true

		doc, err := s.parser.ParseFile(path)
		if err != nil {
			*errors = append(*errors, models.IngestError{
				File:    path,
				Message: fmt.Sprintf("parse failed: %v", err),
			})
			return nil
		}

		// Check if document already exists
		existing, err := s.docStore.GetBySlug(ctx, doc.Slug)
		if err == nil && existing != nil {
			// Only update if content changed
			if existing.ContentHash != doc.ContentHash {
				existing.Title = doc.Title
				existing.Content = doc.Content
				existing.Category = doc.Category
				existing.Metadata = doc.Metadata
				existing.ContentHash = doc.ContentHash
				existing.FilePath = doc.FilePath
				existing.Source = source
				existing.Orphaned = false

				if err := s.docStore.Update(ctx, existing); err != nil {
					*errors = append(*errors, models.IngestError{
						File:    path,
						Message: fmt.Sprintf("update failed: %v", err),
					})
					return nil
				}
				stats.updated++
			}
		} else {
			// Create new document
			newDoc := &models.Document{
				ID:          uuid.New(),
				Slug:        doc.Slug,
				Title:       doc.Title,
				Content:     doc.Content,
				Category:    doc.Category,
				Path:        fmt.Sprintf("/docs/%s", doc.Slug),
				Version:     1,
				Metadata:    doc.Metadata,
				Source:      source,
				ContentHash: doc.ContentHash,
				FilePath:    doc.FilePath,
				Orphaned:    false,
			}

			if err := s.docStore.Create(ctx, newDoc); err != nil {
				*errors = append(*errors, models.IngestError{
					File:    path,
					Message: fmt.Sprintf("create failed: %v", err),
				})
				return nil
			}
			stats.added++
		}

		return nil
	})
}

// markOrphans marks documents as orphaned if their file no longer exists
func (s *Service) markOrphans(ctx context.Context, processedPaths map[string]bool) (int, error) {
	// This is a simplified version - in production you'd query the DB
	// for all documents with source='repo' and file_path NOT IN processedPaths
	// For now, return 0 and implement proper orphan detection later
	return 0, nil
}

// completeJob finalizes an ingest job
func (s *Service) completeJob(ctx context.Context, jobID uuid.UUID, stats *syncStats, errors []models.IngestError) error {
	// In a real implementation, this would update the ingest_jobs table
	// For now, just log the results
	now := time.Now()
	_ = models.IngestJob{
		ID:             jobID,
		Status:         models.IngestStatusCompleted,
		CompletedAt:    &now,
		FilesProcessed: stats.processed,
		FilesAdded:     stats.added,
		FilesUpdated:   stats.updated,
		FilesOrphaned:  stats.orphaned,
		Errors:         errors,
	}

	return nil
}

// SaveUploadedFile saves an uploaded file to a temporary location
func SaveUploadedFile(file io.Reader, filename string) (string, error) {
	tempDir := os.TempDir()
	tempPath := filepath.Join(tempDir, fmt.Sprintf("upload_%d_%s", time.Now().Unix(), filepath.Base(filename)))

	out, err := os.Create(tempPath)
	if err != nil {
		return "", err
	}
	defer out.Close()

	if _, err := io.Copy(out, file); err != nil {
		return "", err
	}

	return tempPath, nil
}

// CleanOrphanedDocuments removes all orphaned documents
func (s *Service) CleanOrphanedDocuments(ctx context.Context) (int, error) {
	// In a real implementation, this would delete documents where orphaned=true
	// For now, return 0
	return 0, nil
}

// SyncRulesFromRepo syncs prevention rules from markdown files in watched directories
func (s *Service) SyncRulesFromRepo(ctx context.Context) (*RuleSyncResult, error) {
	slog.Info("Starting rule sync from repository", "rules_dir", s.rulesDir)
	slog.Debug("Checking rules directory configuration")
	if s.rulesDir == "" {
		slog.Error("Rules directory not configured")
		return nil, fmt.Errorf("rules directory not configured")
	}

	// Check if rules directory exists
	slog.Debug("Checking if rules directory exists", "dir", s.rulesDir)
	if _, err := os.Stat(s.rulesDir); os.IsNotExist(err) {
		slog.Info("Rules directory does not exist, skipping sync", "dir", s.rulesDir)
		return &RuleSyncResult{}, nil // No rules directory, nothing to sync
	}

	result, err := s.ruleSyncSvc.SyncRulesFromDirectory(ctx, s.rulesDir)
	if err != nil {
		slog.Error("Rule sync failed", "dir", s.rulesDir, "error", err)
		return result, err
	}
	slog.Info("Rule sync completed", "dir", s.rulesDir, "added", result.Added, "updated", result.Updated, "disabled", result.Disabled, "errors", len(result.Errors))
	return result, nil
}

// SyncRulesFromUpload syncs prevention rules from uploaded markdown content
func (s *Service) SyncRulesFromUpload(ctx context.Context, content []byte, filename string) (*RuleSyncResult, error) {
	return s.ruleSyncSvc.SyncRulesFromContent(ctx, string(content), filename)
}
