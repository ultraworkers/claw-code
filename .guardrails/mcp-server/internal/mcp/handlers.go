package mcp

/*
CQRS Handlers — wired to domain command/query handlers

Each handler processes MCP tool calls through the CQRS layer, splitting
read operations (evaluation) from write operations (logging, rule management).
*/

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"strconv"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/domain"
)

// GuardrailHandlers contains CQRS-wired handlers for MCP tools
type GuardrailHandlers struct {
	// Query side (read-optimized, caching)
	evalCommandHandler   *domain.EvaluateCommandHandler
	evalGitHandler       *domain.EvaluateGitHandler
	evalFileEditHandler  *domain.EvaluateFileEditHandler
	listRulesHandler     *domain.ListRulesHandler

	// Command side (writes)
	createRuleHandler    *domain.CreateRuleHandler
	logViolationHandler *domain.LogViolationHandler

	// Domain services
	guardrailSvc domain.GuardrailService
	auditLogger  domain.AuditLogger
}

// NewGuardrailHandlers creates handlers wired to domain interfaces
func NewGuardrailHandlers(
	guardrailSvc domain.GuardrailService,
	ruleRepo domain.RuleRepository,
	auditLogger domain.AuditLogger,
	cache domain.CachePort,
	matcher domain.PatternMatcher,
	bus domain.EventBus,
) *GuardrailHandlers {
	h := &GuardrailHandlers{
		guardrailSvc: guardrailSvc,
		auditLogger:  auditLogger,
	}

	// Wire query handlers
	h.evalCommandHandler = domain.NewEvaluateCommandHandler(guardrailSvc, cache)
	h.evalGitHandler = domain.NewEvaluateGitHandler(guardrailSvc)
	h.evalFileEditHandler = domain.NewEvaluateFileEditHandler(guardrailSvc)
	h.listRulesHandler = domain.NewListRulesHandler(ruleRepo)

	// Wire command handlers
	h.createRuleHandler = domain.NewCreateRuleHandler(ruleRepo, bus, cache, matcher)
	h.logViolationHandler = domain.NewLogViolationHandler(auditLogger)

	return h
}

// ValidateBash handles bash command validation via CQRS query
func (h *GuardrailHandlers) ValidateBash(ctx context.Context, command string) (*mcp.CallToolResult, error) {
	if command == "" {
		return errorResult(fmt.Sprintf(`{"error":"command is required","meta":{"checked_at":"%s"}}`, time.Now().Format(time.RFC3339))), nil
	}

	result, err := h.evalCommandHandler.Handle(ctx, domain.EvaluateCommandQuery{
		Command:   command,
		Categories: []string{"bash", "command"},
	})
	if err != nil {
		slog.Error("Bash validation failed", "error", err, "command", command)
		return errorResult(fmt.Sprintf(`{"error":"validation failed: %s","meta":{"checked_at":"%s"}}`, err.Error(), time.Now().Format(time.RFC3339))), nil
	}

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{
			Type: "text",
			Text: formatValidationResult(result, command),
		}},
	}, nil
}

// ValidateGit handles git command validation via CQRS query
func (h *GuardrailHandlers) ValidateGit(ctx context.Context, command string, isForce bool) (*mcp.CallToolResult, error) {
	if command == "" {
		return errorResult(`{"error":"command is required"}`), nil
	}

	result, err := h.evalGitHandler.Handle(ctx, domain.EvaluateGitQuery{Command: command})
	if err != nil {
		return errorResult(fmt.Sprintf(`{"error":"validation failed: %s"}`, err.Error())), nil
	}

	// Add force push violation via domain model (not hardcoded in handler)
	if isForce {
		result.Violations = append(result.Violations, domain.Violation{
			RuleID:   "PREVENT-FORCE-001",
			RuleName: "No Force Operation",
			Severity: domain.SeverityCritical,
			Message:  "Force operations are not allowed. Use --force-with-lease or standard push instead.",
			Category: "git",
			Timestamp: time.Now(),
		})
	}

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{
			Type: "text",
			Text: formatValidationResult(result, command),
		}},
	}, nil
}

// ValidateFileEdit handles file edit validation via CQRS query
func (h *GuardrailHandlers) ValidateFileEdit(ctx context.Context, filePath, content string, sessionID string) (*mcp.CallToolResult, error) {
	if filePath == "" {
		return errorResult(`{"error":"file_path is required"}`), nil
	}

	result, err := h.evalFileEditHandler.Handle(ctx, domain.EvaluateFileEditQuery{
		FilePath:  filePath,
		Content:   content,
		SessionID: sessionID,
	})
	if err != nil {
		return errorResult(fmt.Sprintf(`{"error":"validation failed: %s"}`, err.Error())), nil
	}

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{
			Type: "text",
			Text: formatValidationResultWithFile(result, filePath, len(content)),
		}},
	}, nil
}

// LogViolation logs a violation via CQRS command
func (h *GuardrailHandlers) LogViolation(ctx context.Context, violation domain.Violation, sessionID string) error {
	return h.logViolationHandler.Handle(ctx, domain.LogViolationCommand{
		Violation: violation,
		SessionID: sessionID,
	})
}

// CreateRule creates a new rule via CQRS command
func (h *GuardrailHandlers) CreateRule(ctx context.Context, cmd domain.CreateRuleCommand) (*domain.PreventionRule, error) {
	return h.createRuleHandler.Handle(ctx, cmd)
}

// ListRules lists rules via CQRS query
func (h *GuardrailHandlers) ListRules(ctx context.Context, enabled *bool, category string, limit, offset int) ([]domain.PreventionRule, error) {
	return h.listRulesHandler.Handle(ctx, domain.ListRulesQuery{
		Enabled:  enabled,
		Category: category,
		Limit:    limit,
		Offset:   offset,
	})
}

// --- Response formatters ---

func formatValidationResult(result *domain.ValidationResult, command string) string {
	var sb strings.Builder
	sb.Grow(512)

	if result.Passed {
		sb.WriteString(`{"valid":true,"violations":[],"meta":{"checked_at":"`)
	} else {
		sb.WriteString(`{"valid":false,"violations":[`)
		for i, v := range result.Violations {
			if i > 0 {
				sb.WriteString(",")
			}
			sb.WriteString(formatViolation(v))
		}
		sb.WriteString(`],"meta":{"checked_at":"`)
	}

	sb.WriteString(result.CheckedAt.Format(time.RFC3339))
	sb.WriteString(`","command_analyzed":"`)
	jsonEscape(&sb, command)
	sb.WriteString(`"}}`)

	return sb.String()
}

func formatValidationResultWithFile(result *domain.ValidationResult, filePath string, contentSize int) string {
	var sb strings.Builder
	sb.Grow(512)

	if result.Passed {
		sb.WriteString(`{"valid":true,"violations":[],"meta":{"checked_at":"`)
	} else {
		sb.WriteString(`{"valid":false,"violations":[`)
		for i, v := range result.Violations {
			if i > 0 {
				sb.WriteString(",")
			}
			sb.WriteString(formatViolation(v))
		}
		sb.WriteString(`],"meta":{"checked_at":"`)
	}

	sb.WriteString(result.CheckedAt.Format(time.RFC3339))
	sb.WriteString(`","file":"`)
	jsonEscape(&sb, filePath)
	sb.WriteString(`","changes_size":`)
	sb.WriteString(strconv.Itoa(contentSize))
	sb.WriteString(`"}}`)

	return sb.String()
}

func formatViolation(v domain.Violation) string {
	var sb strings.Builder
	sb.Grow(256)
	sb.WriteString(`{"rule_id":"`)
	jsonEscape(&sb, v.RuleID)
	sb.WriteString(`","name":"`)
	jsonEscape(&sb, v.RuleName)
	sb.WriteString(`","severity":"`)
	jsonEscape(&sb, string(v.Severity))
	sb.WriteString(`","message":"`)
	jsonEscape(&sb, v.Message)
	sb.WriteString(`"}`)
	return sb.String()
}

func jsonEscape(sb *strings.Builder, s string) {
	for _, r := range s {
		switch r {
		case '"':
			sb.WriteString(`\"`)
		case '\\':
			sb.WriteString(`\\`)
		case '\b':
			sb.WriteString(`\b`)
		case '\f':
			sb.WriteString(`\f`)
		case '\n':
			sb.WriteString(`\n`)
		case '\r':
			sb.WriteString(`\r`)
		case '\t':
			sb.WriteString(`\t`)
		default:
			if r < 0x20 {
				sb.WriteString(`\u00`)
				sb.WriteByte(hexChar(byte(r) >> 4))
				sb.WriteByte(hexChar(byte(r) & 0x0F))
			} else {
				sb.WriteRune(r)
			}
		}
	}
}

func hexChar(n byte) byte {
	if n < 10 {
		return '0' + n
	}
	return 'a' + n - 10
}

func errorResult(msg string) *mcp.CallToolResult {
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: msg}},
		IsError: true,
	}
}

// SessionHandlers provides CQRS-backed session management
type SessionHandlers struct {
	ruleRepo   domain.RuleRepository
	projectSvc ProjectService
	audit      domain.AuditLogger
}

// ProjectService abstracts project lookup for session context
type ProjectService interface {
	GetBySlug(ctx context.Context, slug string) (*ProjectContext, error)
}

// ProjectContext holds project-specific guardrail context
type ProjectContext struct {
	Slug            string
	GuardrailContext string
}

// NewSessionHandlers creates session handlers
func NewSessionHandlers(ruleRepo domain.RuleRepository, projectSvc ProjectService, audit domain.AuditLogger) *SessionHandlers {
	return &SessionHandlers{
		ruleRepo:   ruleRepo,
		projectSvc: projectSvc,
		audit:      audit,
	}
}

// InitSession initializes a session and returns context
func (h *SessionHandlers) InitSession(ctx context.Context, projectSlug, agentType, clientVersion string) (map[string]interface{}, error) {
	// Get project context (via domain service, not infrastructure directly)
	contextStr := ""
	if h.projectSvc != nil {
		proj, err := h.projectSvc.GetBySlug(ctx, projectSlug)
		if err != nil {
			slog.Warn("Failed to get project context", "project_slug", projectSlug, "error", err)
		}
		if proj != nil {
			contextStr = proj.GuardrailContext
		}
	}

	// Get active rules
	rules, err := h.ruleRepo.GetActiveRules(ctx)
	if err != nil {
		slog.Error("Failed to get active rules", "error", err)
		rules = []domain.PreventionRule{}
	}

	return map[string]interface{}{
		"session_token":        "", // filled by caller
		"expires_at":           time.Now().Add(24 * time.Hour).Format(time.RFC3339),
		"project_context":      contextStr,
		"active_rules_count":    len(rules),
		"capabilities":         []string{"bash_validation", "git_validation", "edit_validation"},
	}, nil
}
