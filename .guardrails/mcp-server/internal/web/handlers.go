package web

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/labstack/echo/v4"
	"github.com/thearchitectit/guardrail-mcp/internal/ingest"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
	"github.com/thearchitectit/guardrail-mcp/internal/security"
	"github.com/thearchitectit/guardrail-mcp/internal/updates"
)

// Pagination and validation constants
const (
	defaultPageLimit   = 20
	maxPageLimit       = 100
	maxSearchResults   = 50
	defaultSearchLimit = 20
)

// RuleSyncStatus tracks the status of the last rule sync operation
type RuleSyncStatus struct {
	Status       string    `json:"status"`
	LastSync     time.Time `json:"last_sync"`
	RulesAdded   int       `json:"rules_added"`
	RulesUpdated int       `json:"rules_updated"`
	RulesDeleted int       `json:"rules_deleted"`
	Errors       []string  `json:"errors"`
}

var (
	lastRuleSyncStatus     RuleSyncStatus
	lastRuleSyncStatusLock sync.RWMutex
)

// Document handlers

func (s *Server) listDocuments(c echo.Context) error {
	ctx := c.Request().Context()
	category := c.QueryParam("category")
	limit, err := strconv.Atoi(c.QueryParam("limit"))
	if err != nil || limit <= 0 || limit > maxPageLimit {
		limit = defaultPageLimit
	}
	offset, err := strconv.Atoi(c.QueryParam("offset"))
	if err != nil || offset < 0 {
		offset = 0
	}

	docs, err := s.docStore.List(ctx, category, limit, offset)
	if err != nil {
		slog.Error("Failed to list documents", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve documents"})
	}

	total, err := s.docStore.Count(ctx, category)
	if err != nil {
		slog.Warn("Failed to count documents", "error", err)
		total = len(docs) // Fallback to current page size
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"data": docs,
		"pagination": map[string]interface{}{
			"total":  total,
			"limit":  limit,
			"offset": offset,
		},
	})
}

func (s *Server) getDocument(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	doc, err := s.docStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		return c.JSON(http.StatusNotFound, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusOK, doc)
}

func (s *Server) updateDocument(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	var doc models.Document
	if err := c.Bind(&doc); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	// Scan for secrets before saving
	if findings := security.ScanContent(doc.Content); len(findings) > 0 {
		return c.JSON(http.StatusBadRequest, map[string]interface{}{
			"error":    "Potential secrets detected in content",
			"findings": findings,
		})
	}

	doc.ID = parsedUUID
	if err := s.docStore.Update(c.Request().Context(), &doc); err != nil {
		slog.Error("Failed to update document", "doc_id", id, "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to update document"})
	}

	// Invalidate cache - log error but don't fail the request
	if err := s.cache.InvalidateOnDocumentChange(c.Request().Context(), doc.Slug); err != nil {
		slog.Warn("Failed to invalidate document cache", "slug", doc.Slug, "error", err)
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(c.Request().Context(), keyHash, doc.Slug, "update")

	return c.JSON(http.StatusOK, doc)
}

func (s *Server) searchDocuments(c echo.Context) error {
	query := c.QueryParam("q")
	if query == "" {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "query parameter required"})
	}

	limit, err := strconv.Atoi(c.QueryParam("limit"))
	if err != nil || limit <= 0 || limit > maxSearchResults {
		limit = defaultSearchLimit
	}

	docs, err := s.docStore.Search(c.Request().Context(), query, limit)
	if err != nil {
		slog.Error("Failed to search documents", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to search documents"})
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"data":  docs,
		"query": query,
		"pagination": map[string]interface{}{
			"limit": limit,
		},
	})
}

// Rule handlers

func (s *Server) listRules(c echo.Context) error {
	ctx := c.Request().Context()
	var enabled *bool
	if enabledParam := c.QueryParam("enabled"); enabledParam != "" {
		e := enabledParam == "true"
		enabled = &e
	}
	category := c.QueryParam("category")
	limit, err := strconv.Atoi(c.QueryParam("limit"))
	if err != nil || limit <= 0 || limit > maxPageLimit {
		limit = defaultPageLimit
	}
	offset, err := strconv.Atoi(c.QueryParam("offset"))
	if err != nil || offset < 0 {
		offset = 0
	}

	rules, err := s.ruleStore.List(ctx, enabled, category, limit, offset)
	if err != nil {
		slog.Error("Failed to list rules", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve rules"})
	}

	total, err := s.ruleStore.Count(ctx, enabled, category)
	if err != nil {
		slog.Warn("Failed to count rules", "error", err)
		total = len(rules) // Fallback to current page size
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"data": rules,
		"pagination": map[string]interface{}{
			"total":  total,
			"limit":  limit,
			"offset": offset,
		},
	})
}

func (s *Server) getRule(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	rule, err := s.ruleStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		if err.Error() == fmt.Sprintf("rule not found: %s", parsedUUID) {
			return c.JSON(http.StatusNotFound, map[string]string{"error": "rule not found"})
		}
		slog.Error("Failed to get rule", "rule_id", id, "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve rule"})
	}

	return c.JSON(http.StatusOK, rule)
}

func (s *Server) createRule(c echo.Context) error {
	var rule models.PreventionRule
	if err := c.Bind(&rule); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	if err := s.ruleStore.Create(c.Request().Context(), &rule); err != nil {
		slog.Error("Failed to create rule", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to create rule"})
	}

	// Invalidate cache - log error but don't fail the request
	if err := s.cache.InvalidateOnRuleChange(c.Request().Context(), rule.RuleID); err != nil {
		slog.Warn("Failed to invalidate rule cache", "rule_id", rule.RuleID, "error", err)
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogRuleChange(c.Request().Context(), keyHash, rule.RuleID, "create")

	return c.JSON(http.StatusCreated, rule)
}

func (s *Server) updateRule(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	var rule models.PreventionRule
	if err := c.Bind(&rule); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	rule.ID = parsedUUID
	if err := s.ruleStore.Update(c.Request().Context(), &rule); err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	// Invalidate cache
	s.cache.InvalidateOnRuleChange(c.Request().Context(), rule.RuleID)

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogRuleChange(c.Request().Context(), keyHash, rule.RuleID, "update")

	return c.JSON(http.StatusOK, rule)
}

func (s *Server) deleteRule(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	// Get rule for cache invalidation before deleting
	rule, err := s.ruleStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		// Rule doesn't exist - return 404
		return c.JSON(http.StatusNotFound, map[string]string{"error": "rule not found"})
	}

	if err := s.ruleStore.Delete(c.Request().Context(), parsedUUID); err != nil {
		slog.Error("Failed to delete rule", "rule_id", id, "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to delete rule"})
	}

	// Invalidate cache - log error but don't fail the request
	if err := s.cache.InvalidateOnRuleChange(c.Request().Context(), rule.RuleID); err != nil {
		slog.Warn("Failed to invalidate cache after rule deletion", "rule_id", rule.RuleID, "error", err)
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogRuleChange(c.Request().Context(), keyHash, rule.RuleID, "delete")

	return c.NoContent(http.StatusNoContent)
}

func (s *Server) patchRule(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	var req struct {
		Enabled  *bool   `json:"enabled,omitempty"`
		Name     *string `json:"name,omitempty"`
		Message  *string `json:"message,omitempty"`
		Pattern  *string `json:"pattern,omitempty"`
		Severity *string `json:"severity,omitempty"`
	}
	if err := c.Bind(&req); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	// Get existing rule
	rule, err := s.ruleStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		if err.Error() == fmt.Sprintf("rule not found: %s", parsedUUID) {
			return c.JSON(http.StatusNotFound, map[string]string{"error": "rule not found"})
		}
		slog.Error("Failed to get rule for patch", "rule_id", id, "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve rule"})
	}

	// Apply patches
	if req.Enabled != nil {
		rule.Enabled = *req.Enabled
	}
	if req.Name != nil {
		rule.Name = *req.Name
	}
	if req.Message != nil {
		rule.Message = *req.Message
	}
	if req.Pattern != nil {
		rule.Pattern = *req.Pattern
	}
	if req.Severity != nil {
		rule.Severity = models.Severity(*req.Severity)
	}

	if err := s.ruleStore.Update(c.Request().Context(), rule); err != nil {
		slog.Error("Failed to patch rule", "rule_id", id, "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to update rule"})
	}

	// Invalidate cache - log error but don't fail the request
	if err := s.cache.InvalidateOnRuleChange(c.Request().Context(), rule.RuleID); err != nil {
		slog.Warn("Failed to invalidate rule cache after patch", "rule_id", rule.RuleID, "error", err)
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogRuleChange(c.Request().Context(), keyHash, rule.RuleID, "patch")

	return c.JSON(http.StatusOK, rule)
}

// syncRules triggers a rule sync from repository directories
func (s *Server) syncRules(c echo.Context) error {
	slog.Info("Received rule sync request")
	ctx := c.Request().Context()

	// Parse optional request body for sync options
	var req struct {
		Force bool `json:"force,omitempty"`
	}
	_ = c.Bind(&req) // Optional, ignore error

	// Create a new sync job
	jobID := uuid.New()
	slog.Info("Starting rule sync job", "job_id", jobID)

	// Trigger sync in background to not block the response
	go func() {
		bgCtx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
		defer cancel()

		// Update status to running
		lastRuleSyncStatusLock.Lock()
		lastRuleSyncStatus = RuleSyncStatus{
			Status:   "running",
			LastSync: time.Now().UTC(),
			Errors:   []string{},
		}
		lastRuleSyncStatusLock.Unlock()

		// Perform the sync
		result, err := s.ingestSvc.SyncRulesFromRepo(bgCtx)

		// Update final status
		lastRuleSyncStatusLock.Lock()
		if err != nil {
			lastRuleSyncStatus.Status = "failed"
			lastRuleSyncStatus.Errors = append(lastRuleSyncStatus.Errors, err.Error())
		} else {
			lastRuleSyncStatus.Status = "completed"
			lastRuleSyncStatus.RulesAdded = result.Added
			lastRuleSyncStatus.RulesUpdated = result.Updated
			lastRuleSyncStatus.RulesDeleted = result.Disabled
			if len(result.Errors) > 0 {
				lastRuleSyncStatus.Errors = result.Errors
			}
		}
		lastRuleSyncStatusLock.Unlock()

		if err != nil {
			slog.Error("Rule sync background job failed", "job_id", jobID, "error", err)
		} else {
			slog.Info("Rule sync background job completed", "job_id", jobID, "added", result.Added, "updated", result.Updated, "disabled", result.Disabled)
		}
	}()

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogRuleChange(ctx, keyHash, fmt.Sprintf("sync:%s", jobID), "sync")

	return c.JSON(http.StatusAccepted, map[string]interface{}{
		"job_id":  jobID,
		"status":  "running",
		"message": "Rule sync started",
	})
}

// getRuleSyncStatus returns the status of the last rule sync operation
func (s *Server) getRuleSyncStatus(c echo.Context) error {
	lastRuleSyncStatusLock.RLock()
	status := lastRuleSyncStatus
	lastRuleSyncStatusLock.RUnlock()

	// If never synced, return appropriate message
	if status.Status == "" {
		return c.JSON(http.StatusOK, map[string]interface{}{
			"status":        "never_run",
			"last_sync":     nil,
			"message":       "No rule sync has been performed yet. Use POST /api/rules/sync to trigger a sync.",
			"rules_added":   0,
			"rules_updated": 0,
			"rules_deleted": 0,
			"errors":        []string{},
		})
	}

	return c.JSON(http.StatusOK, status)
}

// triggerRuleSyncFromUpload handles uploaded markdown files to create/update rules
func (s *Server) triggerRuleSyncFromUpload(c echo.Context) error {
	ctx := c.Request().Context()

	// Parse multipart form with 50MB max memory
	form, err := c.MultipartForm()
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid multipart form"})
	}
	defer form.RemoveAll()

	files := form.File["files"]
	if len(files) == 0 {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "no files provided"})
	}

	// Create a new ingest job
	jobID := uuid.New()
	slog.Info("Starting rule sync job", "job_id", jobID)
	fileContents := make(map[string][]byte)

	// Process each uploaded file
	var processedFiles []string
	var skippedFiles []string

	for _, fileHeader := range files {
		// Validate file type
		if !ingest.IsMarkdownFile(fileHeader.Filename) {
			skippedFiles = append(skippedFiles, fileHeader.Filename)
			continue
		}

		// Open the uploaded file
		file, err := fileHeader.Open()
		if err != nil {
			slog.Error("Failed to open uploaded file", "filename", fileHeader.Filename, "error", err)
			continue
		}

		// Read file content
		content, err := io.ReadAll(file)
		file.Close()
		if err != nil {
			slog.Error("Failed to read uploaded file", "filename", fileHeader.Filename, "error", err)
			continue
		}

		fileContents[fileHeader.Filename] = content
		processedFiles = append(processedFiles, fileHeader.Filename)
	}

	// Process the files through the ingest service for rules
	totalResult := &ingest.RuleSyncResult{}
	for filename, content := range fileContents {
		result, err := s.ingestSvc.SyncRulesFromUpload(ctx, content, filename)
		if err != nil {
			slog.Error("Failed to process uploaded rule file", "filename", filename, "error", err)
			continue
		}
		totalResult.Added += result.Added
		totalResult.Updated += result.Updated
		totalResult.Disabled += result.Disabled
		totalResult.Errors = append(totalResult.Errors, result.Errors...)
	}

	// Update sync status
	lastRuleSyncStatusLock.Lock()
	lastRuleSyncStatus = RuleSyncStatus{
		Status:       "completed",
		LastSync:     time.Now().UTC(),
		RulesAdded:   totalResult.Added,
		RulesUpdated: totalResult.Updated,
		RulesDeleted: totalResult.Disabled,
		Errors:       totalResult.Errors,
	}
	lastRuleSyncStatusLock.Unlock()

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogRuleChange(ctx, keyHash, fmt.Sprintf("upload:%s", jobID), "upload")

	return c.JSON(http.StatusOK, map[string]interface{}{
		"job_id":    jobID,
		"processed": len(processedFiles),
		"skipped":   skippedFiles,
		"files":     processedFiles,
		"status":    "completed",
	})
}

// Project handlers

func (s *Server) listProjects(c echo.Context) error {
	ctx := c.Request().Context()
	limit, err := strconv.Atoi(c.QueryParam("limit"))
	if err != nil || limit <= 0 || limit > maxPageLimit {
		limit = defaultPageLimit
	}
	offset, err := strconv.Atoi(c.QueryParam("offset"))
	if err != nil || offset < 0 {
		offset = 0
	}

	projects, err := s.projStore.List(ctx, limit, offset)
	if err != nil {
		slog.Error("Failed to list projects", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve projects"})
	}

	total, err := s.projStore.Count(ctx)
	if err != nil {
		slog.Warn("Failed to count projects", "error", err)
		total = len(projects) // Fallback to current page size
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"data": projects,
		"pagination": map[string]interface{}{
			"total":  total,
			"limit":  limit,
			"offset": offset,
		},
	})
}

func (s *Server) getProject(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	proj, err := s.projStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		return c.JSON(http.StatusNotFound, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusOK, proj)
}

func (s *Server) createProject(c echo.Context) error {
	var proj models.Project
	if err := c.Bind(&proj); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	if err := s.projStore.Create(c.Request().Context(), &proj); err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusCreated, proj)
}

func (s *Server) updateProject(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	var proj models.Project
	if err := c.Bind(&proj); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	proj.ID = parsedUUID
	if err := s.projStore.Update(c.Request().Context(), &proj); err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	// Invalidate cache
	s.cache.InvalidateOnProjectChange(c.Request().Context(), proj.Slug)

	return c.JSON(http.StatusOK, proj)
}

func (s *Server) deleteProject(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	// Get project to find the slug
	proj, err := s.projStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		return c.JSON(http.StatusNotFound, map[string]string{"error": err.Error()})
	}

	if err := s.projStore.Delete(c.Request().Context(), proj.Slug); err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	// Invalidate cache
	s.cache.InvalidateOnProjectChange(c.Request().Context(), proj.Slug)

	return c.NoContent(http.StatusNoContent)
}

// Failure handlers

func (s *Server) listFailures(c echo.Context) error {
	status := c.QueryParam("status")
	category := c.QueryParam("category")
	projectSlug := c.QueryParam("project")

	limit, _ := strconv.Atoi(c.QueryParam("limit"))
	if limit <= 0 || limit > maxPageLimit {
		limit = defaultPageLimit
	}
	offset, _ := strconv.Atoi(c.QueryParam("offset"))
	if offset < 0 {
		offset = 0
	}

	failures, err := s.failStore.List(c.Request().Context(), status, category, projectSlug, limit, offset)
	if err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"data": failures,
		"pagination": map[string]interface{}{
			"limit":  limit,
			"offset": offset,
			"count":  len(failures),
		},
	})
}

func (s *Server) getFailure(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	failure, err := s.failStore.GetByID(c.Request().Context(), parsedUUID)
	if err != nil {
		return c.JSON(http.StatusNotFound, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusOK, failure)
}

func (s *Server) createFailure(c echo.Context) error {
	var failure models.FailureEntry
	if err := c.Bind(&failure); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	if err := s.failStore.Create(c.Request().Context(), &failure); err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusCreated, failure)
}

func (s *Server) updateFailure(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	var failure models.FailureEntry
	if err := c.Bind(&failure); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	failure.ID = parsedUUID
	if err := s.failStore.Update(c.Request().Context(), &failure); err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}

	return c.JSON(http.StatusOK, failure)
}

// System handlers

func (s *Server) getStats(c echo.Context) error {
	ctx := c.Request().Context()

	// Fetch all counts concurrently for better performance
	type countResult struct {
		name  string
		count int64
		err   error
	}

	counts := make(chan countResult, 4)

	// Document count
	go func() {
		count, err := s.docStore.Count(ctx, "")
		counts <- countResult{"documents", int64(count), err}
	}()

	// Rule count
	go func() {
		count, err := s.ruleStore.Count(ctx, nil, "")
		counts <- countResult{"rules", int64(count), err}
	}()

	// Project count
	go func() {
		count, err := s.projStore.Count(ctx)
		counts <- countResult{"projects", int64(count), err}
	}()

	// Failure count
	go func() {
		count, err := s.failStore.Count(ctx)
		counts <- countResult{"failures", count, err}
	}()

	// Collect results
	stats := make(map[string]int64)
	for i := 0; i < 4; i++ {
		result := <-counts
		if result.err != nil {
			slog.Error("Failed to get count", "entity", result.name, "error", result.err)
			stats[result.name] = 0
		} else {
			stats[result.name] = result.count
		}
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"documents_count": stats["documents"],
		"rules_count":     stats["rules"],
		"projects_count":  stats["projects"],
		"failures_count":  stats["failures"],
	})
}

func (s *Server) triggerIngest(c echo.Context) error {
	ctx := c.Request().Context()

	// Parse optional request body for ingest options
	var req struct {
		RepoPath    string   `json:"repo_path,omitempty"`
		Force       bool     `json:"force,omitempty"`
		ProjectSlug string   `json:"project_slug,omitempty"`
		Categories  []string `json:"categories,omitempty"`
	}
	_ = c.Bind(&req) // Optional, ignore error

	// Create a new ingest job
	jobID := uuid.New()
	slog.Info("Starting document ingest job", "job_id", jobID, "repo_path", req.RepoPath, "project_slug", req.ProjectSlug)

	// Trigger ingest in background to not block the response
	go func() {
		bgCtx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
		defer cancel()

		if req.RepoPath != "" {
			// Custom repo path provided, would need custom implementation
			slog.Info("Custom repo path ingest requested", "job_id", jobID, "path", req.RepoPath)
			// For now, fall through to default sync
		}

		// Trigger the ingest from watched directories
		if err := s.ingestSvc.SyncFromRepo(bgCtx, jobID); err != nil {
			slog.Error("Failed to ingest documents", "job_id", jobID, "error", err)
		} else {
			slog.Info("Document ingest completed", "job_id", jobID)
		}
	}()

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(ctx, keyHash, fmt.Sprintf("ingest:%s", jobID), "ingest")

	return c.JSON(http.StatusAccepted, map[string]interface{}{
		"job_id":       jobID,
		"status":       "running",
		"message":      "Document ingestion started",
		"repo_path":    req.RepoPath,
		"project_slug": req.ProjectSlug,
	})
}

// Ingest handlers

// uploadFiles handles multipart file uploads for document ingestion
func (s *Server) uploadFiles(c echo.Context) error {
	ctx := c.Request().Context()

	// Parse multipart form with 50MB max memory
	form, err := c.MultipartForm()
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid multipart form"})
	}
	defer form.RemoveAll()

	files := form.File["files"]
	if len(files) == 0 {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "no files provided"})
	}

	// Create a new ingest job
	jobID := uuid.New()
	slog.Info("Starting rule sync job", "job_id", jobID)
	fileContents := make(map[string][]byte)

	// Process each uploaded file
	var processedFiles []string
	var skippedFiles []string

	for _, fileHeader := range files {
		// Validate file type
		if !ingest.IsMarkdownFile(fileHeader.Filename) {
			skippedFiles = append(skippedFiles, fileHeader.Filename)
			continue
		}

		// Open the uploaded file
		file, err := fileHeader.Open()
		if err != nil {
			slog.Error("Failed to open uploaded file", "filename", fileHeader.Filename, "error", err)
			continue
		}

		// Read file content
		content, err := io.ReadAll(file)
		file.Close()
		if err != nil {
			slog.Error("Failed to read uploaded file", "filename", fileHeader.Filename, "error", err)
			continue
		}

		fileContents[fileHeader.Filename] = content
		processedFiles = append(processedFiles, fileHeader.Filename)
	}

	// Process the files through the ingest service
	if err := s.ingestSvc.SyncFromUpload(ctx, jobID, fileContents); err != nil {
		slog.Error("Failed to process uploaded files", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to process files"})
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(ctx, keyHash, fmt.Sprintf("upload:%s", jobID), "ingest")

	return c.JSON(http.StatusOK, map[string]interface{}{
		"job_id":    jobID,
		"processed": len(processedFiles),
		"skipped":   skippedFiles,
		"files":     processedFiles,
		"status":    "completed",
	})
}

// syncRepo triggers a repository sync
func (s *Server) syncRepo(c echo.Context) error {
	ctx := c.Request().Context()

	// Parse optional request body for sync options
	var req struct {
		Force bool `json:"force,omitempty"`
	}
	_ = c.Bind(&req) // Optional, ignore error

	// Create a new ingest job
	jobID := uuid.New()
	slog.Info("Starting rule sync job", "job_id", jobID)

	// Trigger sync in background to not block the response
	go func() {
		bgCtx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
		defer cancel()

		if err := s.ingestSvc.SyncFromRepo(bgCtx, jobID); err != nil {
			slog.Error("Failed to sync from repo", "job_id", jobID, "error", err)
		}
	}()

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(ctx, keyHash, fmt.Sprintf("sync:%s", jobID), "ingest")

	return c.JSON(http.StatusAccepted, map[string]interface{}{
		"job_id":  jobID,
		"status":  "running",
		"message": "Repository sync started",
	})
}

// getIngestStatus returns the status of the last sync operation
func (s *Server) getIngestStatus(c echo.Context) error {
	// For now, return a placeholder status
	// In a full implementation, this would query the ingest_jobs table
	return c.JSON(http.StatusOK, map[string]interface{}{
		"status":          "completed",
		"last_sync":       time.Now().UTC().Add(-time.Hour).Format(time.RFC3339),
		"files_processed": 0,
		"files_added":     0,
		"files_updated":   0,
		"files_orphaned":  0,
		"errors":          []interface{}{},
	})
}

// listOrphans returns a list of orphaned documents
func (s *Server) listOrphans(c echo.Context) error {
	_ = c.Request().Context() // Reserved for future use

	limit, err := strconv.Atoi(c.QueryParam("limit"))
	if err != nil || limit <= 0 || limit > maxPageLimit {
		limit = defaultPageLimit
	}
	offset, err := strconv.Atoi(c.QueryParam("offset"))
	if err != nil || offset < 0 {
		offset = 0
	}

	// Query orphaned documents using the existing List method with a filter
	// In a full implementation, this would query with orphaned=true filter
	// For now, return an empty list
	docs := []models.Document{}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"data": docs,
		"pagination": map[string]interface{}{
			"total":  0,
			"limit":  limit,
			"offset": offset,
		},
	})
}

// deleteOrphan deletes a single orphaned document
func (s *Server) deleteOrphan(c echo.Context) error {
	id := c.Param("id")
	parsedUUID, err := uuid.Parse(id)
	if err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid id format"})
	}

	ctx := c.Request().Context()

	// Get document to verify it exists and is orphaned
	doc, err := s.docStore.GetByID(ctx, parsedUUID)
	if err != nil {
		return c.JSON(http.StatusNotFound, map[string]string{"error": "document not found"})
	}

	// Verify the document is orphaned
	if !doc.Orphaned {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "document is not orphaned"})
	}

	// Delete the document
	if err := s.docStore.Delete(ctx, parsedUUID); err != nil {
		slog.Error("Failed to delete orphaned document", "doc_id", id, "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to delete document"})
	}

	// Invalidate cache
	if err := s.cache.InvalidateOnDocumentChange(ctx, doc.Slug); err != nil {
		slog.Warn("Failed to invalidate document cache", "slug", doc.Slug, "error", err)
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(ctx, keyHash, doc.Slug, "delete_orphan")

	return c.NoContent(http.StatusNoContent)
}

// Update handlers

// getUpdateStatus returns the current update status
func (s *Server) getUpdateStatus(c echo.Context) error {
	ctx := c.Request().Context()

	// Get the latest update check from database
	latestCheck, err := s.updateChecker.GetLatestCheck(ctx)
	if err != nil {
		// No check has been performed yet
		return c.JSON(http.StatusOK, map[string]interface{}{
			"last_checked":      nil,
			"updates_available": false,
			"message":           "No update check has been performed yet. Use POST /api/updates/check to trigger a check.",
		})
	}

	// Convert to response format
	response := updates.ToStatusResponse(latestCheck)

	return c.JSON(http.StatusOK, response)
}

// checkForUpdates triggers a manual update check
func (s *Server) checkForUpdates(c echo.Context) error {
	ctx := c.Request().Context()

	// Perform the update check
	check, err := s.updateChecker.Check(ctx)
	if err != nil {
		slog.Error("Update check failed", "error", err)
		return c.JSON(http.StatusInternalServerError, map[string]string{
			"error": "Failed to check for updates",
		})
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(ctx, keyHash, "update-check", "check")

	// Return the result
	response := updates.ToStatusResponse(check)

	return c.JSON(http.StatusOK, response)
}
func (s *Server) ideHealth(c echo.Context) error {
	return c.JSON(http.StatusOK, map[string]string{"status": "ok"})
}

func (s *Server) validateFile(c echo.Context) error {
	ctx := c.Request().Context()

	// Parse request body
	var req struct {
		FilePath    string `json:"file_path"`
		Content     string `json:"content"`
		ProjectSlug string `json:"project_slug,omitempty"`
		Language    string `json:"language,omitempty"`
	}

	if err := c.Bind(&req); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	if req.FilePath == "" {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "file_path is required"})
	}

	// Get active rules for the project
	var rules []models.PreventionRule
	var err error

	if req.ProjectSlug != "" {
		proj, err := s.projStore.GetBySlug(ctx, req.ProjectSlug)
		if err == nil && len(proj.ActiveRules) > 0 {
			rules, err = s.ruleStore.GetByRuleIDs(ctx, proj.ActiveRules)
			if err != nil {
				slog.Warn("Failed to get project rules, falling back to all active", "error", err)
			}
		}
	}

	// If no project-specific rules, get all active rules
	if len(rules) == 0 {
		rules, err = s.ruleStore.GetActiveRules(ctx)
		if err != nil {
			slog.Error("Failed to get active rules", "error", err)
			return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve rules"})
		}
	}

	// Validate content against rules
	violations := validateContentAgainstRules(req.FilePath, req.Content, req.Language, rules)

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogValidation(ctx, keyHash, "validate_file", len(violations) == 0, len(violations))

	return c.JSON(http.StatusOK, map[string]interface{}{
		"valid":         len(violations) == 0,
		"violations":    violations,
		"file_path":     req.FilePath,
		"rules_checked": len(rules),
	})
}

func (s *Server) validateSelection(c echo.Context) error {
	ctx := c.Request().Context()

	var req struct {
		Selection   string `json:"selection"`
		FilePath    string `json:"file_path"`
		ProjectSlug string `json:"project_slug,omitempty"`
		Language    string `json:"language,omitempty"`
		StartLine   int    `json:"start_line"`
		EndLine     int    `json:"end_line"`
	}

	if err := c.Bind(&req); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
	}

	if req.Selection == "" {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "selection is required"})
	}

	// Get active rules for the project
	var rules []models.PreventionRule
	var err error

	if req.ProjectSlug != "" {
		proj, err := s.projStore.GetBySlug(ctx, req.ProjectSlug)
		if err == nil && len(proj.ActiveRules) > 0 {
			rules, err = s.ruleStore.GetByRuleIDs(ctx, proj.ActiveRules)
			if err != nil {
				slog.Warn("Failed to get project rules, falling back to all active", "error", err)
			}
		}
	}

	// If no project-specific rules, get all active rules
	if len(rules) == 0 {
		rules, err = s.ruleStore.GetActiveRules(ctx)
		if err != nil {
			slog.Error("Failed to get active rules", "error", err)
			return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to retrieve rules"})
		}
	}

	// Validate selection against rules
	violations := validateContentAgainstRules(req.FilePath, req.Selection, req.Language, rules)

	// Adjust line numbers to be relative to the file, not the selection
	for i := range violations {
		violations[i].Line = req.StartLine + violations[i].Line - 1
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogValidation(ctx, keyHash, "validate_selection", len(violations) == 0, len(violations))

	return c.JSON(http.StatusOK, map[string]interface{}{
		"valid":         len(violations) == 0,
		"violations":    violations,
		"file_path":     req.FilePath,
		"start_line":    req.StartLine,
		"end_line":      req.EndLine,
		"rules_checked": len(rules),
	})
}

func (s *Server) getIDERules(c echo.Context) error {
	ctx := c.Request().Context()
	projectSlug := c.QueryParam("project")

	// Try cache first for better performance
	cacheKey := projectSlug
	if cacheKey == "" {
		cacheKey = "default"
	}

	if cached, err := s.cache.GetIDERules(ctx, cacheKey); err == nil && len(cached) > 0 {
		// Return cached JSON directly to avoid re-marshaling
		return c.JSONBlob(http.StatusOK, cached)
	}

	var rules []models.PreventionRule
	var err error

	if projectSlug != "" {
		// Get project to find active rules
		proj, err := s.projStore.GetBySlug(ctx, projectSlug)
		if err == nil && len(proj.ActiveRules) > 0 {
			// Batch fetch all project rules in a single query (prevents N+1)
			rules, err = s.ruleStore.GetByRuleIDs(ctx, proj.ActiveRules)
			if err != nil {
				slog.Warn("Failed to get project rules, falling back to all active", "error", err)
			}
		}
	}

	// If no project-specific rules, get all active rules
	if len(rules) == 0 {
		rules, err = s.ruleStore.GetActiveRules(ctx)
		if err != nil {
			return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
		}
	}

	// Marshal once for both caching and response
	rulesJSON, err := json.Marshal(rules)
	if err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": "failed to marshal rules"})
	}

	// Cache the result asynchronously to not block the response
	go func(ctx context.Context, key string, data []byte) {
		// Use a new context with timeout for cache operation
		cacheCtx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		if cacheErr := s.cache.SetIDERules(cacheCtx, key, data); cacheErr != nil {
			slog.Warn("Failed to cache IDE rules", "error", cacheErr)
		}
	}(ctx, cacheKey, rulesJSON)

	return c.JSONBlob(http.StatusOK, rulesJSON)
}

func (s *Server) getQuickReference(c echo.Context) error {
	ctx := c.Request().Context()

	// Try to find quick-reference document
	doc, err := s.docStore.GetBySlug(ctx, "quick-reference")
	if err != nil {
		// Try alternative slugs
		doc, err = s.docStore.GetBySlug(ctx, "quick-reference-card")
		if err != nil {
			// Search for any document with "quick reference" in title
			docs, searchErr := s.docStore.Search(ctx, "quick reference", 5)
			if searchErr != nil || len(docs) == 0 {
				return c.JSON(http.StatusOK, map[string]string{
					"reference": "Quick reference documentation not found. Please ensure documents are ingested.",
				})
			}
			doc = &docs[0]
		}
	}

	// Audit log
	keyHash := getAPIKeyHash(c)
	s.auditLogger.LogDocChange(ctx, keyHash, doc.Slug, "quick-reference-access")

	return c.JSON(http.StatusOK, map[string]interface{}{
		"reference": doc.Content,
		"title":     doc.Title,
		"slug":      doc.Slug,
		"category":  doc.Category,
	})
}

// validateContentAgainstRules checks content against prevention rules and returns violations
type ValidationViolation struct {
	RuleID   string `json:"rule_id"`
	RuleName string `json:"rule_name"`
	Severity string `json:"severity"`
	Message  string `json:"message"`
	Line     int    `json:"line"`
	Column   int    `json:"column"`
	Match    string `json:"match"`
}

func validateContentAgainstRules(filePath, content, language string, rules []models.PreventionRule) []ValidationViolation {
	var violations []ValidationViolation
	lines := strings.Split(content, "\n")

	for _, rule := range rules {
		if !rule.Enabled || rule.Pattern == "" {
			continue
		}

		// Skip language-specific rules if language doesn't match
		if rule.Category != "" && language != "" && rule.Category != language {
			continue
		}

		// Compile regex pattern
		re, err := regexp.Compile(rule.Pattern)
		if err != nil {
			slog.Warn("Invalid rule pattern", "rule_id", rule.RuleID, "error", err)
			continue
		}

		// Check each line
		for lineNum, line := range lines {
			matches := re.FindAllStringIndex(line, -1)
			for _, match := range matches {
				violations = append(violations, ValidationViolation{
					RuleID:   rule.RuleID,
					RuleName: rule.Name,
					Severity: string(rule.Severity),
					Message:  rule.Message,
					Line:     lineNum + 1,
					Column:   match[0] + 1,
					Match:    truncateMatch(line[match[0]:match[1]]),
				})
			}
		}
	}

	return violations
}

// truncateMatch limits the match length for display
func truncateMatch(match string) string {
	if len(match) > 50 {
		return match[:50] + "..."
	}
	return match
}

// getAPIKeyHash safely extracts the API key hash from the context
func getAPIKeyHash(c echo.Context) string {
	keyHash, ok := c.Get("api_key_hash").(string)
	if !ok || keyHash == "" {
		return "unknown"
	}
	return keyHash
}

// isValidSlug validates a project slug to prevent path traversal attacks
// Valid slugs contain only alphanumeric characters, hyphens, and underscores
func isValidSlug(slug string) bool {
	if slug == "" {
		return false
	}
	if len(slug) > 100 {
		return false
	}
	// Check for path traversal attempts
	if strings.Contains(slug, "..") || strings.Contains(slug, "/") || strings.Contains(slug, "\\") {
		return false
	}
	// Only allow alphanumeric, hyphens, and underscores
	for _, r := range slug {
		if !((r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') || r == '-' || r == '_') {
			return false
		}
	}
	return true
}
