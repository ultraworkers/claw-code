package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
)

// handleDetectLanguage auto-detects the project language from the repo root
func (s *MCPServer) handleDetectLanguage(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	projectPath, _ := args["project_path"].(string)
	if projectPath == "" {
		projectPath, _ = os.Getwd()
	}

	type langInfo struct {
		Name           string   `json:"name"`
		DetectionFile  string   `json:"detection_file"`
		TestCmd        []string `json:"test_cmd"`
		BuildCmd       []string `json:"build_cmd"`
		LintCmd        []string `json:"lint_cmd"`
		Found          bool     `json:"found"`
	}

	// Detection order: most specific first
	detections := []struct {
		file string
		lang langInfo
	}{
		{"project.godot", langInfo{Name: "godot", DetectionFile: "project.godot", TestCmd: []string{"godot", "--headless", "--script", "test_runner.gd"}, BuildCmd: []string{"godot", "--headless", "--export-debug"}, LintCmd: []string{"gdformat", "--check"}}},
		{"go.mod", langInfo{Name: "go", DetectionFile: "go.mod", TestCmd: []string{"go", "test", "-v", "-race", "./..."}, BuildCmd: []string{"go", "build", "./..."}, LintCmd: []string{"golangci-lint", "run"}}},
		{"Cargo.toml", langInfo{Name: "rust", DetectionFile: "Cargo.toml", TestCmd: []string{"cargo", "test"}, BuildCmd: []string{"cargo", "build", "--release"}, LintCmd: []string{"cargo", "clippy"}}},
		{"package.json", langInfo{Name: "typescript", DetectionFile: "package.json", TestCmd: []string{"npm", "test"}, BuildCmd: []string{"npm", "run", "build"}, LintCmd: []string{"npm", "run", "lint"}}},
		{"pom.xml", langInfo{Name: "java", DetectionFile: "pom.xml", TestCmd: []string{"mvn", "test"}, BuildCmd: []string{"mvn", "package"}, LintCmd: []string{"mvn", "checkstyle:check"}}},
		{"build.gradle", langInfo{Name: "java", DetectionFile: "build.gradle", TestCmd: []string{"gradle", "test"}, BuildCmd: []string{"gradle", "build"}, LintCmd: []string{"gradle", "check"}}},
		{"build.gradle.kts", langInfo{Name: "kotlin", DetectionFile: "build.gradle.kts", TestCmd: []string{"gradle", "test"}, BuildCmd: []string{"gradle", "build"}, LintCmd: []string{"detekt", "check"}}},
		{"Gemfile", langInfo{Name: "ruby", DetectionFile: "Gemfile", TestCmd: []string{"bundle", "exec", "rspec"}, BuildCmd: []string{"gem", "build"}, LintCmd: []string{"rubocop"}}},
		{"pyproject.toml", langInfo{Name: "python", DetectionFile: "pyproject.toml", TestCmd: []string{"pytest"}, BuildCmd: []string{"python", "-m", "build"}, LintCmd: []string{"ruff", "check"}}},
		{"setup.py", langInfo{Name: "python", DetectionFile: "setup.py", TestCmd: []string{"pytest"}, BuildCmd: []string{"python", "setup.py", "build"}, LintCmd: []string{"ruff", "check"}}},
		{"requirements.txt", langInfo{Name: "python", DetectionFile: "requirements.txt", TestCmd: []string{"pytest"}, BuildCmd: []string{}, LintCmd: []string{"ruff", "check"}}},
		{"*.sln", langInfo{Name: "csharp", DetectionFile: "*.sln", TestCmd: []string{"dotnet", "test"}, BuildCmd: []string{"dotnet", "build"}, LintCmd: []string{"dotnet", "format", "--verify-no-changes"}}},
		{"CMakeLists.txt", langInfo{Name: "cpp", DetectionFile: "CMakeLists.txt", TestCmd: []string{"ctest"}, BuildCmd: []string{"cmake", "--build"}, LintCmd: []string{"clang-tidy"}}},
		{"Package.swift", langInfo{Name: "swift", DetectionFile: "Package.swift", TestCmd: []string{"swift", "test"}, BuildCmd: []string{"swift", "build"}, LintCmd: []string{"swiftlint", "lint"}}},
		{"composer.json", langInfo{Name: "php", DetectionFile: "composer.json", TestCmd: []string{"phpunit"}, BuildCmd: []string{}, LintCmd: []string{"phpcs"}}},
		{"build.sbt", langInfo{Name: "scala", DetectionFile: "build.sbt", TestCmd: []string{"sbt", "test"}, BuildCmd: []string{"sbt", "compile"}, LintCmd: []string{"scalafmt", "--check"}}},
		{"flake.nix", langInfo{Name: "nix", DetectionFile: "flake.nix", TestCmd: []string{"nix", "flake", "check"}, BuildCmd: []string{"nix", "build"}, LintCmd: []string{"statix", "check"}}},
		{"Dockerfile", langInfo{Name: "docker", DetectionFile: "Dockerfile", TestCmd: []string{}, BuildCmd: []string{"docker", "build", "."}, LintCmd: []string{"hadolint"}}},
	}

	for _, d := range detections {
		checkPath := filepath.Join(projectPath, d.file)
		if strings.Contains(d.file, "*") {
			// Glob match
			matches, _ := filepath.Glob(checkPath)
			if len(matches) > 0 {
				info := d.lang
				info.Found = true
				resultJSON, _ := json.Marshal(info)
				return &mcp.CallToolResult{
					Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
				}, nil
			}
		} else if _, err := os.Stat(checkPath); err == nil {
			info := d.lang
			info.Found = true
			resultJSON, _ := json.Marshal(info)
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
			}, nil
		}
	}

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"No language detected","found":false}`}},
		IsError: true,
	}, nil
}

// handleGetLanguageProfile returns the guardrail profile for a specific language
func (s *MCPServer) handleGetLanguageProfile(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	language, _ := args["language"].(string)
	if language == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"language parameter required"}`}},
			IsError: true,
		}, nil
	}

	// Map language names to profile file names
	profileMap := map[string]string{
		"go":         "Sentinel Profile - Go.txt",
		"python":     "Sentinel Profile - Python.txt",
		"typescript": "Sentinel Profile - TypeScript.txt",
		"rust":       "Sentinel Profile - Rust.txt",
		"java":       "Sentinel Profile - Java.txt",
		"kotlin":     "Sentinel Profile - Kotlin.txt",
		"csharp":     "Sentinel Profile - C# - .NET",
		"cpp":        "Sentinel Profile - C++.txt",
		"php":        "Sentinel Profile - PHP.txt",
		"ruby":       "Sentinel Profile - Ruby.txt",
		"swift":      "Sentinel Profile - Swift.txt",
		"scala":      "Sentinel Profile - Scala.txt",
		"r":          "Sentinel Profile - R.txt",
		"sql":        "Sentinel Profile - SQL.txt",
		"godot":      "Sentinel Profile - GDScript - Godot.txt",
		"gdscript":   "Sentinel Profile - GDScript - Godot.txt",
		"docker":     "Sentinel Profile - Docker - DevOps.txt",
		"ai":         "Sentinel Profile - AI - ML.txt",
		"ml":         "Sentinel Profile - AI - ML.txt",
	}

	profileFile, ok := profileMap[strings.ToLower(language)]
	if !ok {
		available := make([]string, 0, len(profileMap))
		for k := range profileMap {
			available = append(available, k)
		}
		result := map[string]interface{}{
			"error":     fmt.Sprintf("No profile found for language: %s", language),
			"available": available,
		}
		resultJSON, _ := json.Marshal(result)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
			IsError: true,
		}, nil
	}

	// Read the profile file
	repoPath := s.getRepoPath()
	profilePath := filepath.Join(repoPath, "docs", "agentmcp", profileFile)
	content, err := os.ReadFile(profilePath)
	if err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"error":"Profile file not found: %s"}`, profileFile)}},
			IsError: true,
		}, nil
	}

	result := map[string]interface{}{
		"language": language,
		"profile":  string(content),
	}
	resultJSON, _ := json.Marshal(result)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// handleListLanguages returns all supported language profiles
func (s *MCPServer) handleListLanguages(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	type langEntry struct {
		Name           string `json:"name"`
		DetectionFile  string `json:"detection_file"`
		Module         string `json:"module"`
	}

	languages := []langEntry{
		{Name: "go", DetectionFile: "go.mod", Module: "25"},
		{Name: "typescript", DetectionFile: "package.json", Module: "26"},
		{Name: "python", DetectionFile: "pyproject.toml/requirements.txt", Module: "27"},
		{Name: "java", DetectionFile: "pom.xml/build.gradle", Module: "28"},
		{Name: "ruby", DetectionFile: "Gemfile", Module: "29"},
		{Name: "rust", DetectionFile: "Cargo.toml", Module: "30"},
		{Name: "csharp", DetectionFile: "*.sln/*.csproj", Module: "31"},
		{Name: "php", DetectionFile: "composer.json", Module: "32"},
		{Name: "cpp", DetectionFile: "CMakeLists.txt", Module: "33"},
		{Name: "swift", DetectionFile: "Package.swift", Module: "34"},
		{Name: "kotlin", DetectionFile: "build.gradle.kts", Module: "35"},
		{Name: "scala", DetectionFile: "build.sbt", Module: "36"},
		{Name: "r", DetectionFile: "renv.lock", Module: "37"},
		{Name: "sql", DetectionFile: "dbt_project.yml", Module: "39"},
		{Name: "gdscript", DetectionFile: "project.godot", Module: "40"},
		{Name: "docker", DetectionFile: "Dockerfile", Module: "41"},
		{Name: "ai/ml", DetectionFile: "train*.py", Module: "42"},
	}

	resultJSON, _ := json.Marshal(map[string]interface{}{
		"languages": languages,
		"total":     len(languages),
	})
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// handleValidateLanguageRules runs language-specific checks on file content
func (s *MCPServer) handleValidateLanguageRules(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	language, _ := args["language"].(string)
	filePath, _ := args["file_path"].(string)
	content, _ := args["content"].(string)

	if language == "" || content == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"language and content parameters required"}`}},
			IsError: true,
		}, nil
	}

	type violation struct {
		RuleID    string `json:"rule_id"`
		RuleName  string `json:"rule_name"`
		Severity  string `json:"severity"`
		Message   string `json:"message"`
		File      string `json:"file"`
		Suggestion string `json:"suggestion,omitempty"`
	}

	violations := []violation{}

	switch strings.ToLower(language) {
	case "gdscript", "godot":
		// Check: extends mandate
		if !strings.Contains(content, "extends ") && !strings.Contains(content, "@tool") {
			violations = append(violations, violation{
				RuleID: "GD-001", RuleName: "Extends Mandate", Severity: "warning",
				Message: "Script missing 'extends' declaration", File: filePath,
				Suggestion: "Every GDScript should extend a node type",
			})
		}
		// Check: direct .free()
		if strings.Contains(content, ".free()") && !strings.Contains(content, "queue_free") {
			violations = append(violations, violation{
				RuleID: "GD-002", RuleName: "Queue Free", Severity: "warning",
				Message: "Direct .free() detected — use queue_free()", File: filePath,
				Suggestion: "Use node.queue_free() for safe deferred deletion",
			})
		}
		// Check: string get_node()
		if strings.Contains(content, `get_node("`) || strings.Contains(content, "get_node('") {
			violations = append(violations, violation{
				RuleID: "GD-003", RuleName: "Node Path Safety", Severity: "warning",
				Message: "String-based get_node() detected", File: filePath,
				Suggestion: "Prefer @onready var node = $Path or %UniqueName",
			})
		}
		// Check: heavy ops in _process
		for _, fn := range []string{"_process", "_physics_process"} {
			if idx := strings.Index(content, "func "+fn); idx >= 0 {
				funcBody := content[idx:]
				if nextFunc := strings.Index(funcBody[10:], "func "); nextFunc > 0 {
					funcBody = funcBody[:nextFunc+10]
				}
				for _, op := range []string{"FileAccess", "HTTPRequest", "load("} {
					if strings.Contains(funcBody, op) {
						violations = append(violations, violation{
							RuleID: "GD-004", RuleName: "Process Performance", Severity: "warning",
							Message: fmt.Sprintf("'%s' in %s — move to _ready()", op, fn), File: filePath,
						})
					}
				}
			}
		}
	case "python":
		// Check: bare except
		if strings.Contains(content, "except:") && !strings.Contains(content, "except Exception") && !strings.Contains(content, "except ValueError") {
			violations = append(violations, violation{
				RuleID: "PY-001", RuleName: "Bare Except", Severity: "error",
				Message: "Bare except clause catches SystemExit/KeyboardInterrupt", File: filePath,
				Suggestion: "Use 'except Exception as e:' or catch specific exceptions",
			})
		}
		// Check: mutable default args
		if strings.Contains(content, "= []") || strings.Contains(content, "= {}") {
			if strings.Contains(content, "def ") {
				violations = append(violations, violation{
					RuleID: "PY-002", RuleName: "Mutable Defaults", Severity: "error",
					Message: "Mutable default argument detected", File: filePath,
					Suggestion: "Use None as default, initialize inside function",
				})
			}
		}
	case "go":
		// Check: ignored errors
		if strings.Contains(content, "_, err :=") || strings.Contains(content, "_ = ") {
			// Rough heuristic, real check needs AST
			violations = append(violations, violation{
				RuleID: "GO-001", RuleName: "Error Checking", Severity: "warning",
				Message: "Potential ignored error return", File: filePath,
				Suggestion: "Always check errors: if err != nil { return err }",
			})
		}
	case "typescript", "javascript":
		// Check: any type
		if strings.Contains(content, ": any") || strings.Contains(content, "as any") {
			violations = append(violations, violation{
				RuleID: "TS-001", RuleName: "Any Type", Severity: "error",
				Message: "Usage of 'any' type detected", File: filePath,
				Suggestion: "Define a proper interface or use 'unknown'",
			})
		}
		// Check: console.log
		if strings.Contains(content, "console.log") {
			violations = append(violations, violation{
				RuleID: "TS-002", RuleName: "Console Log", Severity: "warning",
				Message: "console.log in production code", File: filePath,
				Suggestion: "Remove or replace with proper logging library",
			})
		}
	case "rust":
		// Check: unwrap in non-test
		if strings.Contains(content, ".unwrap()") && !strings.Contains(filePath, "test") {
			violations = append(violations, violation{
				RuleID: "RS-001", RuleName: "Unwrap Usage", Severity: "warning",
				Message: "unwrap() can panic in production", File: filePath,
				Suggestion: "Use match, ?, or expect() with descriptive message",
			})
		}
	}

	result := map[string]interface{}{
		"language":     language,
		"file":         filePath,
		"violations":   violations,
		"valid":        len(violations) == 0,
		"checked_at":   fmt.Sprintf("%s", args["session_token"]),
	}
	resultJSON, _ := json.Marshal(result)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}
