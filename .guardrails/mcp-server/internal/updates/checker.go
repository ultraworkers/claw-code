package updates

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

const (
	// Default timeout for external checks
	checkTimeout = 10 * time.Second

	// Docker Hub API endpoint for guardrail-mcp
	dockerHubTagsURL = "https://hub.docker.com/v2/repositories/thearchitectit/guardrail-mcp/tags/?page_size=1&ordering=last_updated"

	// GitHub API endpoint for latest commit
	githubCommitsURL = "https://api.github.com/repos/thearchitectit/guardrail-mcp/commits/main"
)

// Checker handles update checking operations
type Checker struct {
	db         *database.DB
	httpClient *http.Client
	version    string
	gitCommit  string
}

// NewChecker creates a new update checker
func NewChecker(db *database.DB, version, gitCommit string) *Checker {
	return &Checker{
		db: db,
		httpClient: &http.Client{
			Timeout: checkTimeout,
		},
		version:   version,
		gitCommit: gitCommit,
	}
}

// CheckResult contains the results of an update check
type CheckResult struct {
	DockerCurrentVersion     string
	DockerLatestVersion      string
	DockerReleaseNotes       string
	DockerUpdateAvailable    bool
	GuardrailCurrentCommit   string
	GuardrailLatestCommit    string
	GuardrailNewFiles        int
	GuardrailModifiedFiles   int
	GuardrailDeletedFiles    int
	GuardrailUpdateAvailable bool
	Metadata                 map[string]any
}

// Check performs a full update check and saves results to database
func (c *Checker) Check(ctx context.Context) (*models.UpdateCheck, error) {
	result, err := c.performCheck(ctx)
	if err != nil {
		slog.Error("Update check failed", "error", err)
		// Continue to save partial results
	}

	// Create update check record
	check := &models.UpdateCheck{
		ID:                       uuid.New(),
		CheckedAt:                time.Now().UTC(),
		DockerCurrentVersion:     result.DockerCurrentVersion,
		DockerLatestVersion:      result.DockerLatestVersion,
		DockerReleaseNotes:       result.DockerReleaseNotes,
		DockerUpdateAvailable:    result.DockerUpdateAvailable,
		GuardrailCurrentCommit:   result.GuardrailCurrentCommit,
		GuardrailLatestCommit:    result.GuardrailLatestCommit,
		GuardrailNewFiles:        result.GuardrailNewFiles,
		GuardrailModifiedFiles:   result.GuardrailModifiedFiles,
		GuardrailDeletedFiles:    result.GuardrailDeletedFiles,
		GuardrailUpdateAvailable: result.GuardrailUpdateAvailable,
		Metadata:                 result.Metadata,
	}

	// Save to database
	if err := c.saveCheckResult(ctx, check); err != nil {
		slog.Error("Failed to save update check result", "error", err)
		return nil, fmt.Errorf("failed to save check result: %w", err)
	}

	slog.Info("Update check completed",
		"docker_update_available", check.DockerUpdateAvailable,
		"guardrail_update_available", check.GuardrailUpdateAvailable,
	)

	return check, nil
}

// performCheck executes all update checks
func (c *Checker) performCheck(ctx context.Context) (*CheckResult, error) {
	result := &CheckResult{
		Metadata: make(map[string]any),
	}

	// Check Docker version
	dockerErr := c.checkDockerVersion(ctx, result)
	if dockerErr != nil {
		result.Metadata["docker_check_error"] = dockerErr.Error()
		slog.Warn("Docker version check failed", "error", dockerErr)
	}

	// Check Git repository for updates
	gitErr := c.checkGitUpdates(ctx, result)
	if gitErr != nil {
		result.Metadata["git_check_error"] = gitErr.Error()
		slog.Warn("Git update check failed", "error", gitErr)
	}

	return result, nil
}

// checkDockerVersion checks for available Docker image updates
func (c *Checker) checkDockerVersion(ctx context.Context, result *CheckResult) error {
	// Get current version
	result.DockerCurrentVersion = c.getCurrentDockerVersion()

	// Fetch latest version from Docker Hub
	latestVersion, releaseNotes, err := c.fetchLatestDockerVersion(ctx)
	if err != nil {
		return fmt.Errorf("failed to fetch latest Docker version: %w", err)
	}

	result.DockerLatestVersion = latestVersion
	result.DockerReleaseNotes = releaseNotes
	result.DockerUpdateAvailable = c.isNewerVersion(result.DockerCurrentVersion, result.DockerLatestVersion)

	return nil
}

// getCurrentDockerVersion returns the current Docker version
func (c *Checker) getCurrentDockerVersion() string {
	// First check environment variable
	if version := os.Getenv("DOCKER_IMAGE_VERSION"); version != "" {
		return version
	}

	// Check version file
	if data, err := os.ReadFile("/app/version"); err == nil {
		return strings.TrimSpace(string(data))
	}

	// Fallback to build version
	if c.version != "" && c.version != "dev" {
		return c.version
	}

	return "unknown"
}

// fetchLatestDockerVersion fetches the latest version from Docker Hub
func (c *Checker) fetchLatestDockerVersion(ctx context.Context) (string, string, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, dockerHubTagsURL, nil)
	if err != nil {
		return "", "", err
	}

	// Set headers to avoid rate limiting
	req.Header.Set("Accept", "application/json")
	req.Header.Set("User-Agent", "guardrail-mcp-updater/1.0")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return "", "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", "", fmt.Errorf("Docker Hub API returned status %d", resp.StatusCode)
	}

	var dockerResponse struct {
		Results []struct {
			Name        string `json:"name"`
			LastUpdated string `json:"last_updated"`
		}	`json:"results"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&dockerResponse); err != nil {
		return "", "", fmt.Errorf("failed to decode Docker Hub response: %w", err)
	}

	if len(dockerResponse.Results) == 0 {
		return "", "", fmt.Errorf("no tags found in Docker Hub response")
	}

	latestTag := dockerResponse.Results[0]
	releaseNotes := fmt.Sprintf("https://hub.docker.com/r/thearchitectit/guardrail-mcp/tags?name=%s",
		latestTag.Name)

	return latestTag.Name, releaseNotes, nil
}

// checkGitUpdates checks for git repository updates
func (c *Checker) checkGitUpdates(ctx context.Context, result *CheckResult) error {
	// Get current commit
	result.GuardrailCurrentCommit = c.getCurrentGitCommit()

	// Try to get latest commit from local git if available
	if latest, err := c.getLatestLocalCommit(ctx); err == nil && latest != "" {
		result.GuardrailLatestCommit = latest
	} else {
		// Fall back to GitHub API
		latest, err := c.fetchLatestGitHubCommit(ctx)
		if err != nil {
			return fmt.Errorf("failed to fetch latest commit: %w", err)
		}
		result.GuardrailLatestCommit = latest
	}

	// Check if commits differ
	if result.GuardrailCurrentCommit != "" && result.GuardrailLatestCommit != "" {
		result.GuardrailUpdateAvailable = result.GuardrailCurrentCommit != result.GuardrailLatestCommit
	}

	// Count file changes if update is available
	if result.GuardrailUpdateAvailable {
		newFiles, modifiedFiles, deletedFiles, err := c.countGitChanges(ctx,
			result.GuardrailCurrentCommit, result.GuardrailLatestCommit)
		if err == nil {
			result.GuardrailNewFiles = newFiles
			result.GuardrailModifiedFiles = modifiedFiles
			result.GuardrailDeletedFiles = deletedFiles
		}
	}

	return nil
}

// getCurrentGitCommit returns the current git commit hash
func (c *Checker) getCurrentGitCommit() string {
	// First check environment variable
	if commit := os.Getenv("GIT_COMMIT"); commit != "" && commit != "unknown" {
		return commit
	}

	// Check commit file
	if data, err := os.ReadFile("/app/git-commit"); err == nil {
		return strings.TrimSpace(string(data))
	}

	// Fall back to build commit
	if c.gitCommit != "" && c.gitCommit != "unknown" {
		return c.gitCommit
	}

	// Try to get from git command
	if commit, err := exec.Command("git", "rev-parse", "HEAD").Output(); err == nil {
		return strings.TrimSpace(string(commit))
	}

	return ""
}

// getLatestLocalCommit tries to get the latest commit from local git
func (c *Checker) getLatestLocalCommit(ctx context.Context) (string, error) {
	cmd := exec.CommandContext(ctx, "git", "rev-parse", "origin/main")
	output, err := cmd.Output()
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(output)), nil
}

// fetchLatestGitHubCommit fetches the latest commit from GitHub API
func (c *Checker) fetchLatestGitHubCommit(ctx context.Context) (string, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, githubCommitsURL, nil)
	if err != nil {
		return "", err
	}

	// Set headers
	req.Header.Set("Accept", "application/vnd.github.v3+json")
	req.Header.Set("User-Agent", "guardrail-mcp-updater/1.0")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("GitHub API returned status %d: %s", resp.StatusCode, string(body))
	}

	var commitResponse struct {
		SHA string `json:"sha"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&commitResponse); err != nil {
		return "", fmt.Errorf("failed to decode GitHub response: %w", err)
	}

	return commitResponse.SHA, nil
}

// countGitChanges counts file changes between two commits
func (c *Checker) countGitChanges(ctx context.Context, currentCommit, latestCommit string) (int, int, int, error) {
	if currentCommit == "" || latestCommit == "" {
		return 0, 0, 0, fmt.Errorf("invalid commit hashes")
	}

	cmd := exec.CommandContext(ctx, "git", "diff", "--stat", currentCommit, latestCommit)
	output, err := cmd.Output()
	if err != nil {
		return 0, 0, 0, err
	}

	// Parse git diff --stat output
	// Example: " file.go | 10 +++++-----"
	lines := strings.Split(string(output), "\n")
	newFiles, modifiedFiles, deletedFiles := 0, 0, 0

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" || strings.Contains(line, "|") == false {
			continue
		}

		parts := strings.Split(line, "|")
		if len(parts) < 2 {
			continue
		}

		filename := strings.TrimSpace(parts[0])
		// Check for special indicators
		if strings.HasPrefix(filename, "{") && strings.Contains(filename, "=>") {
			// Renamed file
			modifiedFiles++
		} else if strings.Contains(line, "Bin") {
			// Binary file
			modifiedFiles++
		} else {
			// Regular file change
			modifiedFiles++
		}
	}

	// Try to get more detailed stats using git diff --numstat
	cmd = exec.CommandContext(ctx, "git", "diff", "--numstat", currentCommit, latestCommit)
	output, err = cmd.Output()
	if err == nil {
		newFiles, modifiedFiles, deletedFiles = c.parseNumstat(string(output))
	}

	return newFiles, modifiedFiles, deletedFiles, nil
}

// parseNumstat parses git diff --numstat output
func (c *Checker) parseNumstat(output string) (int, int, int) {
	lines := strings.Split(output, "\n")
	newFiles, modifiedFiles, deletedFiles := 0, 0, 0

	for _, line := range lines {
		fields := strings.Fields(line)
		if len(fields) < 3 {
			continue
		}

		added := fields[0]
		deleted := fields[1]

		if added == "-" && deleted == "-" {
			// Binary file
			modifiedFiles++
		} else if added == "0" {
			// Only deletions - file was deleted
			deletedFiles++
		} else if deleted == "0" {
			// Only additions - new file
			newFiles++
		} else {
			// Both additions and deletions - modified
			modifiedFiles++
		}
	}

	return newFiles, modifiedFiles, deletedFiles
}

// isNewerVersion compares two version strings
func (c *Checker) isNewerVersion(current, latest string) bool {
	// Handle "latest" tag specially
	if current == "latest" {
		return false
	}

	// Normalize versions
	current = strings.TrimPrefix(current, "v")
	latest = strings.TrimPrefix(latest, "v")

	// Simple string comparison for now
	// In production, use semantic versioning library
	return current != latest && latest != "" && latest != "unknown"
}

// saveCheckResult saves the update check result to the database
func (c *Checker) saveCheckResult(ctx context.Context, check *models.UpdateCheck) error {
	metadataJSON, err := json.Marshal(check.Metadata)
	if err != nil {
		metadataJSON = []byte("{}")
	}

	query := `
		INSERT INTO update_checks (
			id, checked_at,
			docker_current_version, docker_latest_version, docker_release_notes, docker_update_available,
			guardrail_current_commit, guardrail_latest_commit,
			guardrail_new_files, guardrail_modified_files, guardrail_deleted_files,
			guardrail_update_available, metadata
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
	`

	_, err = c.db.ExecContext(ctx, query,
		check.ID, check.CheckedAt,
		check.DockerCurrentVersion, check.DockerLatestVersion, check.DockerReleaseNotes, check.DockerUpdateAvailable,
		check.GuardrailCurrentCommit, check.GuardrailLatestCommit,
		check.GuardrailNewFiles, check.GuardrailModifiedFiles, check.GuardrailDeletedFiles,
		check.GuardrailUpdateAvailable, metadataJSON,
	)

	return err
}

// GetLatestCheck retrieves the most recent update check from the database
func (c *Checker) GetLatestCheck(ctx context.Context) (*models.UpdateCheck, error) {
	query := `
		SELECT id, checked_at,
			docker_current_version, docker_latest_version, docker_release_notes, docker_update_available,
			guardrail_current_commit, guardrail_latest_commit,
			guardrail_new_files, guardrail_modified_files, guardrail_deleted_files,
			guardrail_update_available, metadata
		FROM update_checks
		ORDER BY checked_at DESC
		LIMIT 1
	`

	check := &models.UpdateCheck{}
	var metadataJSON []byte

	err := c.db.QueryRowContext(ctx, query).Scan(
		&check.ID, &check.CheckedAt,
		&check.DockerCurrentVersion, &check.DockerLatestVersion, &check.DockerReleaseNotes, &check.DockerUpdateAvailable,
		&check.GuardrailCurrentCommit, &check.GuardrailLatestCommit,
		&check.GuardrailNewFiles, &check.GuardrailModifiedFiles, &check.GuardrailDeletedFiles,
		&check.GuardrailUpdateAvailable, &metadataJSON,
	)

	if err != nil {
		return nil, err
	}

	if len(metadataJSON) > 0 {
		json.Unmarshal(metadataJSON, &check.Metadata)
	}

	return check, nil
}

// ToStatusResponse converts an UpdateCheck to an UpdateStatusResponse
func ToStatusResponse(check *models.UpdateCheck) *models.UpdateStatusResponse {
	if check == nil {
		return &models.UpdateStatusResponse{
			LastChecked: time.Time{},
		}
	}

	response := &models.UpdateStatusResponse{
		LastChecked: check.CheckedAt,
	}

	if check.DockerUpdateAvailable {
		response.DockerUpdate = &models.DockerUpdateInfo{
			CurrentVersion: check.DockerCurrentVersion,
			LatestVersion:  check.DockerLatestVersion,
			ReleaseNotes:   check.DockerReleaseNotes,
		}
	}

	if check.GuardrailUpdateAvailable {
		response.GuardrailUpdate = &models.GuardrailUpdateInfo{
			CurrentCommit: check.GuardrailCurrentCommit,
			LatestCommit:  check.GuardrailLatestCommit,
			NewFiles:      check.GuardrailNewFiles,
			ModifiedFiles: check.GuardrailModifiedFiles,
			DeletedFiles:  check.GuardrailDeletedFiles,
		}
	}

	return response
}
