package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// Base path for documentation files (relative to project root)
const docsBasePath = "/mnt/ollama/git/agent-guardrails-template"

// readAgentGuardrailsResource reads the AGENT_GUARDRAILS.md documentation
func (s *MCPServer) readAgentGuardrailsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	content, err := os.ReadFile(filepath.Join(docsBasePath, "docs", "AGENT_GUARDRAILS.md"))
	if err != nil {
		return nil, fmt.Errorf("failed to read agent guardrails: %w", err)
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     string(content),
			},
		},
	}, nil
}

// readWorkflowsResource lists all workflow documentation
func (s *MCPServer) readWorkflowsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	workflowsPath := filepath.Join(docsBasePath, "docs", "workflows")

	// Read INDEX.md if it exists
	indexContent := ""
	indexPath := filepath.Join(workflowsPath, "INDEX.md")
	if data, err := os.ReadFile(indexPath); err == nil {
		indexContent = string(data)
	}

	// List all files in the workflows directory
	files, err := os.ReadDir(workflowsPath)
	if err != nil {
		// Return just the index if we can't read directory
		return &mcp.ReadResourceResult{
			Contents: []interface{}{
				mcp.TextResourceContents{
					Uri:      uri,
					MimeType: "text/markdown",
					Text:     indexContent,
				},
			},
		}, nil
	}

	// Build content with index and file listing
	var sb strings.Builder
	if indexContent != "" {
		sb.WriteString(indexContent)
		sb.WriteString("\n\n---\n\n")
	}
	sb.WriteString("## Available Workflow Files\n\n")

	for _, file := range files {
		if file.IsDir() || !strings.HasSuffix(file.Name(), ".md") {
			continue
		}
		sb.WriteString(fmt.Sprintf("- `%s`\n", file.Name()))
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     sb.String(),
			},
		},
	}, nil
}

// readStandardsResource lists all standards documentation
func (s *MCPServer) readStandardsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	standardsPath := filepath.Join(docsBasePath, "docs", "standards")

	// Read INDEX.md if it exists
	indexContent := ""
	indexPath := filepath.Join(standardsPath, "INDEX.md")
	if data, err := os.ReadFile(indexPath); err == nil {
		indexContent = string(data)
	}

	// List all files in the standards directory
	files, err := os.ReadDir(standardsPath)
	if err != nil {
		// Return just the index if we can't read directory
		return &mcp.ReadResourceResult{
			Contents: []interface{}{
				mcp.TextResourceContents{
					Uri:      uri,
					MimeType: "text/markdown",
					Text:     indexContent,
				},
			},
		}, nil
	}

	// Build content with index and file listing
	var sb strings.Builder
	if indexContent != "" {
		sb.WriteString(indexContent)
		sb.WriteString("\n\n---\n\n")
	}
	sb.WriteString("## Available Standard Files\n\n")

	for _, file := range files {
		if file.IsDir() || !strings.HasSuffix(file.Name(), ".md") {
			continue
		}
		sb.WriteString(fmt.Sprintf("- `%s`\n", file.Name()))
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     sb.String(),
			},
		},
	}, nil
}

// readFourLawsResource reads the Four Laws of Agent Safety
func (s *MCPServer) readFourLawsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	content, err := os.ReadFile(filepath.Join(docsBasePath, "skills", "shared-prompts", "four-laws.md"))
	if err != nil {
		return nil, fmt.Errorf("failed to read four laws: %w", err)
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     string(content),
			},
		},
	}, nil
}

// readHaltConditionsResource reads the halt conditions documentation
func (s *MCPServer) readHaltConditionsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	content, err := os.ReadFile(filepath.Join(docsBasePath, "skills", "shared-prompts", "halt-conditions.md"))
	if err != nil {
		return nil, fmt.Errorf("failed to read halt conditions: %w", err)
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     string(content),
			},
		},
	}, nil
}

// readPreWorkChecklistResource reads the pre-work checklist
func (s *MCPServer) readPreWorkChecklistResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	content, err := os.ReadFile(filepath.Join(docsBasePath, ".guardrails", "pre-work-check.md"))
	if err != nil {
		return nil, fmt.Errorf("failed to read pre-work checklist: %w", err)
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     string(content),
			},
		},
	}, nil
}

// readGitSafetyPolicyResource returns git safety policy
func (s *MCPServer) readGitSafetyPolicyResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	content := `# Git Safety Policy

## Forbidden Operations

The following git operations are FORBIDDEN:

- **NO FORCE PUSH** - Never use ` + "`git push --force`" + ` or ` + "`git push -f`" + `
  - Consequence: Data loss, history corruption

- **NO AMEND** - Do not amend commits you didn't create this session
  - Consequence: Breaks collaborator history

- **NO REBASE** - Never rebase shared branches
  - Consequence: Destroys collaborator work

- **NO SKIP HOOKS** - Never use ` + "`--no-verify`" + `
  - Consequence: Bypasses safety checks

- **NO DESTRUCTIVE OPS** - No ` + "`git reset --hard`" + ` on shared branches
  - Consequence: Irreversible data loss

## Required Checks

Before any push operation:

1. **All tests must pass** - Run full test suite
2. **No secrets in commit** - Scan for API keys, passwords, tokens
3. **No merge conflicts** - Verify clean working directory
4. **No binaries without justification** - Document why binary is needed

## Push Permission

**CRITICAL**: Only push if user EXPLICITLY requests it.

If uncertain, ASK before pushing.
`

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     content,
			},
		},
	}, nil
}

// readTestProdSeparationPolicyResource returns test/production separation policy
func (s *MCPServer) readTestProdSeparationPolicyResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	content := `# Test/Production Separation Policy

## Core Principles

### 1. Production Code FIRST

Production code MUST be created before test or infrastructure code.

**Violation Action:** HALT and ask user

### 2. Separate Databases

- Production databases MUST NOT be used for testing
- Test databases MUST NOT contain production data
- Connection strings MUST be environment-specific

**Violation Action:** HALT and rollback

### 3. Separate Services

- Production services MUST run separately from test services
- Test fixtures MUST NOT call production APIs
- Production credentials MUST NOT be in test files

**Violation Action:** HALT and rollback

### 4. No Test Data in Production

- Test users MUST NOT exist in production
- Test data MUST be clearly marked and isolated
- Test transactions MUST NOT affect production state

**Violation Action:** HALT and rollback

## Validation Rules

When creating test code, verify:

- [ ] Production code exists for the feature being tested
- [ ] Test uses mock/test services, not production
- [ ] Test data is synthetic or clearly marked
- [ ] Database connections point to test instances
- [ ] No production credentials in test configuration

## Separation Checklist

Before creating test files:

1. Verify production implementation exists
2. Confirm test database is configured
3. Ensure test services are running
4. Mark test data appropriately
5. Document any production-like test needs

## When to Halt

HALT immediately if:
- Attempting to use production DB for tests
- Production credentials found in test code
- Test data detected in production environment
- Uncertain about environment boundaries
`

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "text/markdown",
				Text:     content,
			},
		},
	}, nil
}

// readAvailableAdvisorsResource returns list of all advisors
func (s *MCPServer) readAvailableAdvisorsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
	advisors := models.StandardAdvisors()

	content, err := json.MarshalIndent(advisors, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("failed to marshal advisors: %w", err)
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "application/json",
				Text:     string(content),
			},
		},
	}, nil
}

// readAdvisorDetailResource returns specific advisor details
func (s *MCPServer) readAdvisorDetailResource(ctx context.Context, uri string, advisorID string) (*mcp.ReadResourceResult, error) {
	advisors := models.StandardAdvisors()

	advisor, ok := advisors[advisorID]
	if !ok {
		return nil, fmt.Errorf("advisor not found: %s", advisorID)
	}

	content, err := json.MarshalIndent(advisor, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("failed to marshal advisor: %w", err)
	}

	return &mcp.ReadResourceResult{
		Contents: []interface{}{
			mcp.TextResourceContents{
				Uri:      uri,
				MimeType: "application/json",
				Text:     string(content),
			},
		},
	}, nil
}
