package ingest

import (
	"crypto/sha256"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
	"gopkg.in/yaml.v3"
)

// Frontmatter represents YAML frontmatter from markdown files
type Frontmatter struct {
	Title    string                 `yaml:"title"`
	Category string                 `yaml:"category"`
	Version  string                 `yaml:"version"`
	Metadata map[string]interface{} `yaml:"metadata,omitempty"`
}

// Parser handles parsing of markdown documents
type Parser struct {
	defaultCategory string
}

// NewParser creates a new markdown parser
func NewParser() *Parser {
	return &Parser{
		defaultCategory: "reference",
	}
}

// ParseFile parses a markdown file and returns a ParsedDocument
func (p *Parser) ParseFile(filePath string) (*models.ParsedDocument, error) {
	content, err := os.ReadFile(filePath)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}

	return p.ParseContent(string(content), filePath)
}

// ParseContent parses markdown content and returns a ParsedDocument
func (p *Parser) ParseContent(content, filePath string) (*models.ParsedDocument, error) {
	// Calculate content hash
	hash := sha256.Sum256([]byte(content))
	contentHash := fmt.Sprintf("%x", hash)

	// Parse frontmatter
	frontmatter, body, err := p.extractFrontmatter(content)
	if err != nil {
		// If frontmatter parsing fails, use defaults
		body = content
	}

	// Generate slug from filename
	slug := p.generateSlug(filePath)

	// Use frontmatter values or defaults
	title := frontmatter.Title
	if title == "" {
		title = p.extractTitleFromContent(body)
	}
	if title == "" {
		title = slug
	}

	category := frontmatter.Category
	if category == "" || !models.IsValidCategory(category) {
		category = p.inferCategory(filePath)
	}

	version := frontmatter.Version
	if version == "" {
		version = "1.0.0"
	}

	// Clean up body (remove frontmatter if present)
	body = strings.TrimSpace(body)

	return &models.ParsedDocument{
		Title:       title,
		Content:     body,
		Category:    category,
		Slug:        slug,
		Version:     version,
		Metadata:    frontmatter.Metadata,
		FilePath:    filePath,
		ContentHash: contentHash,
	}, nil
}

// extractFrontmatter extracts YAML frontmatter from markdown content
func (p *Parser) extractFrontmatter(content string) (*Frontmatter, string, error) {
	// Check if content starts with frontmatter delimiter
	if !strings.HasPrefix(content, "---") {
		return &Frontmatter{}, content, nil
	}

	// Find the end of frontmatter
	parts := strings.SplitN(content[3:], "---", 2)
	if len(parts) != 2 {
		return &Frontmatter{}, content, nil
	}

	frontmatterYAML := strings.TrimSpace(parts[0])
	body := strings.TrimSpace(parts[1])

	var frontmatter Frontmatter
	if err := yaml.Unmarshal([]byte(frontmatterYAML), &frontmatter); err != nil {
		return &Frontmatter{}, content, err
	}

	return &frontmatter, body, nil
}

// generateSlug generates a slug from a file path
func (p *Parser) generateSlug(filePath string) string {
	// Get filename without extension
	base := filepath.Base(filePath)
	ext := filepath.Ext(base)
	name := strings.TrimSuffix(base, ext)

	// Convert to lowercase and replace spaces/special chars with hyphens
	slug := strings.ToLower(name)
	slug = regexp.MustCompile(`[^a-z0-9]+`).ReplaceAllString(slug, "-")
	slug = strings.Trim(slug, "-")

	return slug
}

// extractTitleFromContent extracts the first H1 heading from markdown
func (p *Parser) extractTitleFromContent(content string) string {
	// Look for # Heading
	lines := strings.Split(content, "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "# ") {
			return strings.TrimSpace(line[2:])
		}
	}
	return ""
}

// inferCategory infers document category from file path
func (p *Parser) inferCategory(filePath string) string {
	path := strings.ToLower(filePath)

	// Check path patterns
	if strings.Contains(path, "workflow") {
		return "workflow"
	}
	if strings.Contains(path, "standard") {
		return "standard"
	}
	if strings.Contains(path, "guide") {
		return "guide"
	}
	if strings.Contains(path, "reference") {
		return "reference"
	}

	// Default
	return p.defaultCategory
}

// IsMarkdownFile checks if a file is a markdown file
func IsMarkdownFile(filePath string) bool {
	ext := strings.ToLower(filepath.Ext(filePath))
	return ext == ".md" || ext == ".markdown"
}
