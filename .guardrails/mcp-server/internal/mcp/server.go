package mcp

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
	"github.com/thearchitectit/guardrail-mcp/internal/audit"
	"github.com/thearchitectit/guardrail-mcp/internal/cache"
	"github.com/thearchitectit/guardrail-mcp/internal/config"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/metrics"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
	"github.com/thearchitectit/guardrail-mcp/internal/validation"
)

// MCPServer handles MCP protocol requests
type MCPServer struct {
	mcpServer   *server.MCPServer
	db          *database.DB
	cache       *cache.Cache
	metrics     *metrics.Metrics
	audit       *audit.AuditLogger
	validator   *validation.Engine
	config      *config.Config
	visionTools *VisionTools
}

// NewServer creates a new MCP server instance
func NewServer(db *database.DB, cache *cache.Cache, metrics *metrics.Metrics, audit *audit.AuditLogger, validator *validation.Engine, cfg *config.Config) *MCPServer {
	s := &MCPServer{
		mcpServer: server.NewMCPServer(
			"Guardrail Enforcement Server",
			cfg.Version,
			server.WithResourceCapabilities(true, true),
			server.WithLogging(),
		),
		db:        db,
		cache:     cache,
		metrics:   metrics,
		audit:     audit,
		validator: validator,
		config:    cfg,
	}

	// Initialize vision tools if configured
	if cfg.Vision.Enabled {
		s.visionTools = NewVisionTools(cfg)
	}

	s.setupHandlers()
	return s
}

func (s *MCPServer) setupHandlers() {
	// Register tools
	s.mcpServer.HandleListTools(func(ctx context.Context, cursor *string) (*mcp.ListToolsResult, error) {
		tools := []mcp.Tool{
			{
				Name:        "guardrail_init_session",
				Description: "Initialize a new session with security parameters and session ID",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"user_id": map[string]interface{}{
							"type":        "string",
							"description": "Unique identifier for the user",
						},
						"environment": map[string]interface{}{
							"type":        "string",
							"description": "Target environment (development, staging, production)",
						},
					},
					Required: []string{"user_id"},
				},
			},
			{
				Name:        "guardrail_validate_bash",
				Description: "Validate a bash command against security policies and prevention rules",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"command": map[string]interface{}{
							"type":        "string",
							"description": "The bash command to validate",
						},
						"working_dir": map[string]interface{}{
							"type":        "string",
							"description": "Current working directory",
						},
					},
					Required: []string{"command"},
				},
			},
			{
				Name:        "guardrail_validate_file_edit",
				Description: "Validate a file edit operation (search and replace) against safety rules",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "Path to the file being edited",
						},
						"old_string": map[string]interface{}{
							"type":        "string",
							"description": "Text to be replaced",
						},
						"new_string": map[string]interface{}{
							"type":        "string",
							"description": "Replacement text",
						},
					},
					Required: []string{"file_path", "old_string", "new_string"},
				},
			},
			{
				Name:        "guardrail_validate_git_operation",
				Description: "Validate a git operation (commit, push, branch) against policy",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"operation": map[string]interface{}{
							"type":        "string",
							"description": "Git command to validate (e.g., commit, push)",
						},
						"args": map[string]interface{}{
							"type":        "array",
							"items":       map[string]interface{}{"type": "string"},
							"description": "Arguments to the git command",
						},
					},
					Required: []string{"operation"},
				},
			},
			{
				Name:        "guardrail_pre_work_check",
				Description: "Perform a mandatory pre-work safety check before starting a new task",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"task_description": map[string]interface{}{
							"type":        "string",
							"description": "Brief description of the planned task",
						},
					},
					Required: []string{"task_description"},
				},
			},
			{
				Name:        "guardrail_get_context",
				Description: "Get the current active guardrail context and applicable rules",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"path": map[string]interface{}{
							"type":        "string",
							"description": "Current working directory or file path",
						},
					},
				},
			},
			{
				Name:        "guardrail_validate_scope",
				Description: "Verify if a file path is within authorized project scope",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "Path to the file to validate",
						},
						"authorized_scope": map[string]interface{}{
							"type":        "string",
							"description": "Root directory of the authorized scope",
						},
					},
					Required: []string{"file_path"},
				},
			},
			{
				Name:        "guardrail_validate_commit",
				Description: "Validate proposed commit message and changed files",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"message": map[string]interface{}{
							"type":        "string",
							"description": "Commit message to validate",
						},
						"files": map[string]interface{}{
							"type":        "array",
							"items":       map[string]interface{}{"type": "string"},
							"description": "List of files to be committed",
						},
					},
					Required: []string{"message", "files"},
				},
			},
			{
				Name:        "guardrail_prevent_regression",
				Description: "Check if changes might reintroduce known bugs or violate strict patterns",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "File being modified",
						},
						"changes": map[string]interface{}{
							"type":        "string",
							"description": "Description or diff of planned changes",
						},
					},
					Required: []string{"file_path", "changes"},
				},
			},
			{
				Name:        "guardrail_check_test_prod_separation",
				Description: "Enforce strict separation between test code and production code",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "Path to the file being checked",
						},
					},
					Required: []string{"file_path"},
				},
			},
			{
				Name:        "guardrail_validate_push",
				Description: "Pre-push validation of current branch status and health",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"branch": map[string]interface{}{
							"type":        "string",
							"description": "Branch to be pushed",
						},
						"remote": map[string]interface{}{
							"type":        "string",
							"description": "Remote name (e.g., origin)",
						},
					},
					Required: []string{"branch"},
				},
			},
			{
				Name:        "guardrail_record_file_read",
				Description: "Record that a file has been read by the agent (Four Laws enforcement)",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "Path to the file that was read",
						},
					},
					Required: []string{"file_path"},
				},
			},
			{
				Name:        "guardrail_record_attempt",
				Description: "Record a tool use attempt for tracking progress/failure rates",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"tool_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the tool being attempted",
						},
						"success": map[string]interface{}{
							"type":        "boolean",
							"description": "Whether the attempt was successful",
						},
						"error_msg": map[string]interface{}{
							"type":        "string",
							"description": "Error message if failed",
						},
					},
					Required: []string{"tool_name", "success"},
				},
			},
			{
				Name:        "guardrail_verify_file_read",
				Description: "Verify a file has been read in current context before editing",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "Path to the file to verify",
						},
					},
					Required: []string{"file_path"},
				},
			},
			{
				Name:        "guardrail_validate_three_strikes",
				Description: "Check if current task has hit consecutive failure threshold",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"task_id": map[string]interface{}{
							"type":        "string",
							"description": "Unique identifier for the current task",
						},
					},
				},
			},
			{
				Name:        "guardrail_validate_exact_replacement",
				Description: "Verify that strings for replacement exactly match target file content",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "File path to check",
						},
						"target_string": map[string]interface{}{
							"type":        "string",
							"description": "The string to find for replacement",
						},
					},
					Required: []string{"file_path", "target_string"},
				},
			},
			{
				Name:        "guardrail_reset_attempts",
				Description: "Reset failure counters for a given task or tool",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"task_id": map[string]interface{}{
							"type":        "string",
							"description": "ID of task to reset",
						},
					},
				},
			},
			{
				Name:        "guardrail_check_uncertainty",
				Description: "Force self-reflection when confidence in next step is low",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"current_plan": map[string]interface{}{
							"type":        "string",
							"description": "Description of the current plan",
						},
						"uncertainty_reason": map[string]interface{}{
							"type":        "string",
							"description": "Reason for uncertainty",
						},
					},
					Required: []string{"current_plan", "uncertainty_reason"},
				},
			},
			{
				Name:        "guardrail_check_halt_conditions",
				Description: "Evaluate if current state requires manual human escalation",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"status": map[string]interface{}{
							"type":        "string",
							"description": "Current system/task status",
						},
					},
				},
			},
			{
				Name:        "guardrail_record_halt",
				Description: "Record a system-forced halt event",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"reason": map[string]interface{}{
							"type":        "string",
							"description": "Reason for the halt",
						},
					},
					Required: []string{"reason"},
				},
			},
			{
				Name:        "guardrail_acknowledge_halt",
				Description: "Acknowledged a previously recorded halt to resume operation",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"halt_id": map[string]interface{}{
							"type":        "string",
							"description": "ID of the halt being acknowledged",
						},
					},
					Required: []string{"halt_id"},
				},
			},
			{
				Name:        "guardrail_validate_production_first",
				Description: "Ensure production changes are prioritized or isolated correctly",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"path": map[string]interface{}{
							"type":        "string",
							"description": "Path being modified",
						},
					},
				},
			},
			{
				Name:        "guardrail_detect_feature_creep",
				Description: "Analyze if changes exceed original task scope",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"task_id": map[string]interface{}{
							"type":        "string",
							"description": "Original task identifier",
						},
						"current_changes": map[string]interface{}{
							"type":        "string",
							"description": "Diff or summary of changes so far",
						},
					},
					Required: []string{"task_id", "current_changes"},
				},
			},
			{
				Name:        "guardrail_verify_fixes_intact",
				Description: "Ensure recent bugfixes haven't been regressed by new edits",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"bug_id": map[string]interface{}{
							"type":        "string",
							"description": "Known bug ID or description",
						},
						"file_path": map[string]interface{}{
							"type":        "string",
							"description": "File to check",
						},
					},
					Required: []string{"bug_id", "file_path"},
				},
			},
			{
				Name:        "guardrail_team_init",
				Description: "Initialize a new project team with roles and rules",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the project",
						},
						"teams": map[string]interface{}{
							"type":        "array",
							"items":       map[string]interface{}{"type": "string"},
							"description": "List of team names to initialize",
						},
					},
					Required: []string{"project_name", "teams"},
				},
			},
			{
				Name:        "guardrail_team_list",
				Description: "List all active teams and their configurations",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Filter by project name",
						},
					},
				},
			},
			{
				Name:        "guardrail_team_config_get",
				Description: "Get detailed configuration for a specific team",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the project",
						},
						"team_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the team",
						},
					},
					Required: []string{"project_name", "team_name"},
				},
			},
			{
				Name:        "guardrail_team_config_update",
				Description: "Update rules or roles for an existing team",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the project",
						},
						"team_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the team",
						},
						"config": map[string]interface{}{
							"type":        "object",
							"description": "New configuration data",
						},
					},
					Required: []string{"project_name", "team_name", "config"},
				},
			},
			{
				Name:        "guardrail_advisor_list",
				Description: "List all available AI advisors and their specialties",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
				},
			},
			{
				Name:        "guardrail_advisor_query",
				Description: "Ask a specialist AI advisor for guidance on a specific topic",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"advisor_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the specialist advisor",
						},
						"query": map[string]interface{}{
							"type":        "string",
							"description": "Your question or request",
						},
					},
					Required: []string{"advisor_name", "query"},
				},
			},
			{
				Name:        "guardrail_team_assign",
				Description: "Assign a specific team member (AI advisor) to a project",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Project name",
						},
						"advisor_name": map[string]interface{}{
							"type":        "string",
							"description": "Advisor to assign",
						},
						"role": map[string]interface{}{
							"type":        "string",
							"description": "Specific project role",
						},
					},
					Required: []string{"project_name", "advisor_name"},
				},
			},
			{
				Name:        "guardrail_team_remove",
				Description: "Remove a team or advisor assignment from a project",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Project name",
						},
						"team_id": map[string]interface{}{
							"type":        "number",
							"description": "Team ID to delete (1-12)",
						},
						"confirmed": map[string]interface{}{
							"type":        "boolean",
							"description": "Set to true to confirm deletion. First call without this to see confirmation prompt.",
						},
					},
				},
			},
			{
				Name:        "guardrail_project_delete",
				Description: "Delete an entire project and all its teams. Requires confirmation.",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Name of the project to delete",
						},
						"confirmed": map[string]interface{}{
							"type":        "boolean",
							"description": "Set to true to confirm deletion. First call without this to see confirmation prompt.",
						},
					},
				},
			},
			{
				Name:        "guardrail_team_health",
				Description: "Check team_manager.py health status - validates Python backend and file system access",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"project_name": map[string]interface{}{
							"type":        "string",
							"description": "Optional: Project name for config directory check",
						},
					},
				},
			},
			{
				Name:        "guardrail_install_skills",
				Description: "Install or clone guardrails skill configs. Use 'skill' for per-skill install/clone, 'platforms' for full platform install, or 'path' for single-file clone.",
				InputSchema: mcp.ToolInputSchema{
					Type: "object",
					Properties: mcp.ToolInputSchemaProperties{
						"target_path": map[string]interface{}{
							"type":        "string",
							"description": "Target project directory path (default: current directory)",
						},
						"platforms": map[string]interface{}{
							"type":        "string",
							"description": "Comma-separated list of platforms: claude, cursor, opencode, windsurf, copilot (default: all). Use with action=install.",
						},
						"skill": map[string]interface{}{
							"type":        "string",
							"description": "Install a single skill by name (e.g. 'guardrails-enforcer', 'commit-validator', 'four-laws'). Use action=install. Run list_skills=true to see all.",
						},
						"path": map[string]interface{}{
							"type":        "string",
							"description": "Clone a single file by repo path (e.g. '.claude/skills/guardrails-enforcer.json'). Downloads from GitHub raw. Use with action=clone.",
						},
						"action": map[string]interface{}{
							"type":        "string",
							"description": "Action to perform: 'install' (default), 'clone' (download from GitHub), 'list' (list skills/platforms)",
							"enum":        []string{"install", "clone", "list"},
						},
						"list_skills": map[string]interface{}{
							"type":        "boolean",
							"description": "List all available skills and exit",
						},
						"list_platforms": map[string]interface{}{
							"type":        "boolean",
							"description": "List all available platforms and exit",
						},
						"mode": map[string]interface{}{
							"type":        "string",
							"description": "Installation mode: 'copy' or 'symlink' (default: copy). Applies to action=install.",
							"enum":        []string{"copy", "symlink"},
						},
						"dry_run": map[string]interface{}{
							"type":        "boolean",
							"description": "Preview what would be done without making changes (default: false)",
						},
					},
				},
			},
		}

		if s.visionTools != nil {
			tools = append(tools, s.visionTools.visionToolList()...)
		}

		return &mcp.ListToolsResult{
			Tools: tools,
		}, nil
	})

	// Handle tool calls
	s.mcpServer.HandleCallTool(func(ctx context.Context, name string, arguments map[string]interface{}) (*mcp.CallToolResult, error) {
		// Try vision tools first if enabled
		if s.visionTools != nil {
			result, err := s.visionTools.dispatch(ctx, name, arguments)
			if err == nil {
				return result, nil
			}
		}
		return s.handleToolCall(ctx, name, arguments)
	})

	// Handle resource list requests
	s.mcpServer.HandleListResources(func(ctx context.Context, cursor *string) (*mcp.ListResourcesResult, error) {
		return &mcp.ListResourcesResult{
			Resources: []mcp.Resource{
				{
					URI:  "guardrail://config",
					Name: "Guardrail Configuration",
				},
				{
					URI:  "guardrail://stats",
					Name: "Guardrail Usage Stats",
				},
			},
		}, nil
	})

	// Handle resource read requests
	s.mcpServer.HandleReadResource(func(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
		if uri == "guardrail://config" {
			configJSON, _ := json.MarshalIndent(s.config, "", "  ")
			return &mcp.ReadResourceResult{
				Contents: []mcp.ResourceContent{
					{
						URI:      uri,
						MimeType: "application/json",
						Text:     string(configJSON),
					},
				},
			}, nil
		}
		return nil, fmt.Errorf("resource not found: %s", uri)
	})
}

func (s *MCPServer) handleToolCall(ctx context.Context, name string, args map[string]interface{}) (*mcp.CallToolResult, error) {
	slog.Info("Tool call received", "name", name, "args", args)

	switch name {
	case "guardrail_init_session":
		return s.handleInitSession(ctx, args)
	case "guardrail_validate_bash":
		return s.handleValidateBash(ctx, args)
	case "guardrail_validate_file_edit":
		return s.handleValidateFileEdit(ctx, args)
	case "guardrail_validate_git_operation":
		return s.handleValidateGitOperation(ctx, args)
	case "guardrail_pre_work_check":
		return s.handlePreWorkCheck(ctx, args)
	case "guardrail_get_context":
		return s.handleGetContext(ctx, args)
	case "guardrail_validate_scope":
		return s.handleValidateScope(ctx, args)
	case "guardrail_validate_commit":
		return s.handleValidateCommit(ctx, args)
	case "guardrail_prevent_regression":
		return s.handlePreventRegression(ctx, args)
	case "guardrail_check_test_prod_separation":
		return s.handleCheckTestProdSeparation(ctx, args)
	case "guardrail_validate_push":
		return s.handleValidatePush(ctx, args)
	case "guardrail_record_file_read":
		return s.handleRecordFileRead(ctx, args)
	case "guardrail_record_attempt":
		return s.handleRecordAttempt(ctx, args)
	case "guardrail_verify_file_read":
		return s.handleVerifyFileRead(ctx, args)
	case "guardrail_validate_three_strikes":
		return s.handleValidateThreeStrikes(ctx, args)
	case "guardrail_validate_exact_replacement":
		return s.handleValidateExactReplacement(ctx, args)
	case "guardrail_reset_attempts":
		return s.handleResetAttempts(ctx, args)
	case "guardrail_check_uncertainty":
		return s.handleCheckUncertainty(ctx, args)
	case "guardrail_check_halt_conditions":
		return s.handleCheckHaltConditions(ctx, args)
	case "guardrail_record_halt":
		return s.handleRecordHalt(ctx, args)
	case "guardrail_acknowledge_halt":
		return s.handleAcknowledgeHalt(ctx, args)
	case "guardrail_validate_production_first":
		return s.handleValidateProductionFirst(ctx, args)
	case "guardrail_detect_feature_creep":
		return s.handleDetectFeatureCreep(ctx, args)
	case "guardrail_verify_fixes_intact":
		return s.handleVerifyFixesIntact(ctx, args)
	case "guardrail_team_init":
		return s.handleTeamInit(ctx, args)
	case "guardrail_team_list":
		return s.handleTeamList(ctx, args)
	case "guardrail_team_config_get":
		return s.handleTeamConfigGet(ctx, args)
	case "guardrail_team_config_update":
		return s.handleTeamConfigUpdate(ctx, args)
	case "guardrail_advisor_list":
		return s.handleAdvisorList(ctx, args)
	case "guardrail_advisor_query":
		return s.handleAdvisorQuery(ctx, args)
	case "guardrail_team_assign":
		return s.handleTeamAssign(ctx, args)
	case "guardrail_team_remove":
		return s.handleTeamRemove(ctx, args)
	case "guardrail_project_delete":
		return s.handleProjectDelete(ctx, args)
	case "guardrail_team_health":
		return s.handleTeamHealth(ctx, args)
	case "guardrail_install_skills":
		return s.handleInstallSkills(ctx, args)
	default:
		return nil, fmt.Errorf("unknown tool: %s", name)
	}
}

// buildToolResult is a helper to centralize formatting of MCP tool returns
func buildToolResult(data interface{}, isJson bool) (*mcp.CallToolResult, error) {
	var text string
	if isJson {
		j, _ := json.MarshalIndent(data, "", "  ")
		text = string(j)
	} else {
		text = fmt.Sprintf("%v", data)
	}

	return &mcp.CallToolResult{
		Content: []mcp.CallToolContent{
			{
				Type: "text",
				Text: text,
			},
		},
	}, nil
}

func (s *MCPServer) handleInitSession(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	userID, _ := args["user_id"].(string)
	env, _ := args["environment"].(string)

	token := make([]byte, 8)
	rand.Read(token)
	sessionID := hex.EncodeToString(token)

	result := models.SessionInfo{
		SessionID:   sessionID,
		UserID:      userID,
		Environment: env,
		StartTime:   time.Now(),
	}

	return buildToolResult(result, true)
}

// Serve HTTP requests (SSE for MCP)
func (s *MCPServer) Serve(addr string) error {
	e := echo.New()
	e.Use(middleware.Logger())
	e.Use(middleware.Recover())

	e.GET("/mcp", func(c echo.Context) error {
		s.mcpServer.HandleSSE(c.Response().Writer, c.Request())
		return nil
	})

	e.POST("/mcp", func(c echo.Context) error {
		s.mcpServer.HandleSSE(c.Response().Writer, c.Request())
		return nil
	})

	return e.Start(addr)
}

func (s *MCPServer) handleGetContext(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	path, _ := args["path"].(string)
	if path == "" {
		path, _ = os.Getwd()
	}

	rules := s.validator.GetRulesForPath(ctx, path)
	result := map[string]interface{}{
		"path":            path,
		"applicable_rules": rules,
		"timestamp":       time.Now().Format(time.RFC3339),
	}

	return buildToolResult(result, true)
}
