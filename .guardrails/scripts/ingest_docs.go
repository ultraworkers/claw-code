package main

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

func main() {
	ctx := context.Background()

	// Connect to database
	db, err := database.Connect()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to connect to database: %v\n", err)
		os.Exit(1)
	}
	defer db.Close()

	docStore := database.NewDocumentStore(db)

	// Track statistics
	var ingested, updated, failed int

	// Walk docs directory
	err = filepath.Walk("docs", func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() || !strings.HasSuffix(path, ".md") {
			return err
		}

		content, err := os.ReadFile(path)
		if err != nil {
			fmt.Printf("Failed to read %s: %v\n", path, err)
			failed++
			return nil
		}

		// Extract title from first h1
		title := extractTitle(string(content))
		slug := slugify(filepath.Base(path, ".md"))
		category := categorize(path)

		// Check if document already exists
		existing, err := docStore.GetBySlug(ctx, slug)
		if err == nil && existing != nil {
			// Update existing document
			existing.Title = title
			existing.Content = string(content)
			existing.Category = category
			existing.Path = path
			existing.Version++

			err = docStore.Update(ctx, existing)
			if err != nil {
				fmt.Printf("Failed to update %s: %v\n", path, err)
				failed++
			} else {
				fmt.Printf("Updated: %s (v%d)\n", path, existing.Version)
				updated++
			}
		} else {
			// Create new document
			doc := &models.Document{
				ID:       uuid.New(),
				Slug:     slug,
				Title:    title,
				Content:  string(content),
				Category: category,
				Path:     path,
				Version:  1,
			}

			err = docStore.Create(ctx, doc)
			if err != nil {
				fmt.Printf("Failed to ingest %s: %v\n", path, err)
				failed++
			} else {
				fmt.Printf("Ingested: %s\n", path)
				ingested++
			}
		}

		return nil
	})

	if err != nil {
		fmt.Fprintf(os.Stderr, "Walk failed: %v\n", err)
		os.Exit(1)
	}

	// Also ingest canonical Four Laws from skills
	skillsDocs := []string{
		"skills/shared-prompts/four-laws.md",
		"skills/shared-prompts/halt-conditions.md",
	}

	for _, path := range skillsDocs {
		content, err := os.ReadFile(path)
		if err != nil {
			fmt.Printf("Failed to read %s: %v\n", path, err)
			failed++
			continue
		}

		title := extractTitle(string(content))
		slug := "canonical-" + slugify(filepath.Base(path, ".md"))
		category := "canonical"

		existing, err := docStore.GetBySlug(ctx, slug)
		if err == nil && existing != nil {
			existing.Title = title
			existing.Content = string(content)
			existing.Category = category
			existing.Path = path
			existing.Version++

			err = docStore.Update(ctx, existing)
			if err != nil {
				fmt.Printf("Failed to update %s: %v\n", path, err)
				failed++
			} else {
				fmt.Printf("Updated: %s (v%d)\n", path, existing.Version)
				updated++
			}
		} else {
			doc := &models.Document{
				ID:       uuid.New(),
				Slug:     slug,
				Title:    title,
				Content:  string(content),
				Category: category,
				Path:     path,
				Version:  1,
			}

			err = docStore.Create(ctx, doc)
			if err != nil {
				fmt.Printf("Failed to ingest %s: %v\n", path, err)
				failed++
			} else {
				fmt.Printf("Ingested: %s\n", path)
				ingested++
			}
		}
	}

	fmt.Println("\n--- Ingestion Complete ---")
	fmt.Printf("Ingested: %d\nUpdated: %d\nFailed: %d\n", ingested, updated, failed)
}

// extractTitle extracts the title from the first h1 heading
func extractTitle(content string) string {
	// Look for # Title pattern
	re := regexp.MustCompile(`(?m)^#\s+(.+)$`)
	matches := re.FindStringSubmatch(content)
	if len(matches) > 1 {
		return strings.TrimSpace(matches[1])
	}

	// Fallback: look for first line
	lines := strings.Split(content, "\n")
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" && !strings.HasPrefix(trimmed, "<!--") {
			return trimmed
		}
	}

	return "Untitled"
}

// slugify converts a filename to a slug
func slugify(name string) string {
	// Convert to lowercase
	slug := strings.ToLower(name)
	// Replace spaces and underscores with hyphens
	slug = strings.ReplaceAll(slug, " ", "-")
	slug = strings.ReplaceAll(slug, "_", "-")
	// Remove any non-alphanumeric characters except hyphens
	re := regexp.MustCompile(`[^a-z0-9-]`)
	slug = re.ReplaceAllString(slug, "")
	// Remove multiple hyphens
	slug = regexp.MustCompile(`-+`).ReplaceAllString(slug, "-")
	// Trim hyphens from ends
	slug = strings.Trim(slug, "-")
	return slug
}

// categorize determines the category based on the file path
func categorize(path string) string {
	path = strings.ToLower(path)

	switch {
	case strings.Contains(path, "sprint"):
		return "sprints"
	case strings.Contains(path, "workflow"):
		return "workflows"
	case strings.Contains(path, "standard"):
		return "standards"
	case strings.Contains(path, "security"):
		return "security"
	case strings.Contains(path, "plan"):
		return "plans"
	case strings.Contains(path, "release"):
		return "releases"
	case strings.Contains(path, "skills"):
		return "canonical"
	default:
		return "general"
	}
}
