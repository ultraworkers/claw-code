package validation

import (
	"bufio"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// GameEngineDetector detects game engine projects
type GameEngineDetector struct{}

// DetectResult contains the detected engine and project path
type DetectResult struct {
	Engine      models.GameEngine
	ProjectPath string
	ConfigFile  string
}

// DetectGameEngine scans a directory for game engine project files
func (d *GameEngineDetector) DetectGameEngine(repoPath string) (*DetectResult, error) {
	entries, err := os.ReadDir(repoPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read directory: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		switch entry.Name() {
		case "project.godot":
			return &DetectResult{
				Engine:      models.EngineGodot,
				ProjectPath: repoPath,
				ConfigFile:  entry.Name(),
			}, nil
		}
	}

	// Check subdirectories (common in monorepos: game/ or godot/ subdirs)
	subdirs := []string{"game", "godot", "src", "project"}
	for _, subdir := range subdirs {
		subPath := filepath.Join(repoPath, subdir)
		if info, err := os.Stat(subPath); err == nil && info.IsDir() {
			if result, _ := d.DetectGameEngine(subPath); result != nil {
				return result, nil
			}
		}
	}

	return nil, nil
}

// GodotValidator validates Godot projects using headless mode
type GodotValidator struct {
	GodotPath string // Path to Godot binary
	Timeout   time.Duration
}

// NewGodotValidator creates a new Godot validator
func NewGodotValidator(godotPath string) *GodotValidator {
	if godotPath == "" {
		godotPath = "godot" // Rely on PATH
	}
	return &GodotValidator{
		GodotPath: godotPath,
		Timeout:   60 * time.Second,
	}
}

var (
	// Regex patterns for parsing Godot headless output
	reTestPass = regexp.MustCompile(`✅\s*PASS:\s*(.+)`)
	reTestFail = regexp.MustCompile(`❌\s*FAIL:\s*(.+)`)
	reScriptError = regexp.MustCompile(`SCRIPT ERROR:\s*(.+?)(?:\n|$)`)
	reParseError = regexp.MustCompile(`Parse Error:\s*(.+?)(?:\n|$)`)
	reSceneError = regexp.MustCompile(`Error loading resource:\s*(.+?)(?:\n|$)`)
)

// ValidateProject runs headless Godot validation on a project
func (v *GodotValidator) ValidateProject(ctx context.Context, projectPath string) (*models.GameBuildResult, error) {
	start := time.Now()
	result := &models.GameBuildResult{
		Engine:      models.EngineGodot,
		ProjectPath: projectPath,
		Status:      models.BuildPassed,
		ValidatedAt: time.Now(),
	}

	// Step 1: Validate project.godot exists and parses
	if err := v.validateProjectFile(projectPath, result); err != nil {
		result.DurationMs = time.Since(start).Milliseconds()
		result.Status = models.BuildErrored
	}

	// Step 2: Run headless script check (validates .gd files parse)
	if err := v.runHeadlessCheck(ctx, projectPath, result); err != nil {
		result.DurationMs = time.Since(start).Milliseconds()
		return result, err
	}

	// Step 3: Run test script if available
	v.runHeadlessTests(ctx, projectPath, result)

	result.DurationMs = time.Since(start).Milliseconds()
	if result.TestsFailed > 0 {
		result.Status = models.BuildFailed
	}

	return result, nil
}

// validateProjectFile checks that project.godot exists
func (v *GodotValidator) validateProjectFile(projectPath string, result *models.GameBuildResult) error {
	projFile := filepath.Join(projectPath, "project.godot")
	if _, err := os.Stat(projFile); os.IsNotExist(err) {
		result.Status = models.BuildErrored
		result.Errors = append(result.Errors, models.BuildError{
			Type:     models.GameErrorExport,
			File:     "project.godot",
			Message:  "project.godot not found",
			Severity: "error",
		})
		return fmt.Errorf("project.godot not found in %s", projectPath)
	}
	return nil
}

// runHeadlessCheck runs Godot in headless mode to validate scripts and scenes
func (v *GodotValidator) runHeadlessCheck(ctx context.Context, projectPath string, result *models.GameBuildResult) error {
	checkCtx, cancel := context.WithTimeout(ctx, v.Timeout)
	defer cancel()

	cmd := exec.CommandContext(checkCtx, v.GodotPath,
		"--headless",
		"--path", projectPath,
		"--quit-after", "100", // Quit after 100 frames
	)

	output, err := cmd.CombinedOutput()
	outputStr := string(output)

	// Parse script errors
	for _, match := range reScriptError.FindAllStringSubmatch(outputStr, -1) {
		result.Errors = append(result.Errors, models.BuildError{
			Type:     models.GameErrorScript,
			Message:  match[1],
			Severity: "error",
		})
	}

	// Parse parse errors
	for _, match := range reParseError.FindAllStringSubmatch(outputStr, -1) {
		result.Errors = append(result.Errors, models.BuildError{
			Type:     models.GameErrorScript,
			Message:  match[1],
			Severity: "error",
		})
	}

	// Parse scene errors
	for _, match := range reSceneError.FindAllStringSubmatch(outputStr, -1) {
		result.Errors = append(result.Errors, models.BuildError{
			Type:     models.GameErrorScene,
			Message:  match[1],
			Severity: "error",
		})
	}

	if err != nil && len(result.Errors) == 0 {
		// Command failed but no parseable errors — might be timeout or missing binary
		return fmt.Errorf("godot headless check failed: %w (output: %s)", err, truncate(outputStr, 500))
	}

	return nil
}

// runHeadlessTests runs a Godot headless test script if one exists
func (v *GodotValidator) runHeadlessTests(ctx context.Context, projectPath string, result *models.GameBuildResult) {
	// Look for common test script names
	testScripts := []string{
		"test_runner.gd",
		"tests/test_runner.gd",
		"test_pong_headless.gd", // Legacy from playgodot-pong
	}

	var testScript string
	for _, script := range testScripts {
		if _, err := os.Stat(filepath.Join(projectPath, script)); err == nil {
			testScript = script
			break
		}
	}

	if testScript == "" {
		// No test script found — skip tests
		result.TestsRun = 0
		return
	}

	testCtx, cancel := context.WithTimeout(ctx, v.Timeout)
	defer cancel()

	cmd := exec.CommandContext(testCtx, v.GodotPath,
		"--headless",
		"--path", projectPath,
		"--script", testScript,
	)

	output, _ := cmd.CombinedOutput()
	outputStr := string(output)

	// Parse test results
	scanner := bufio.NewScanner(strings.NewReader(outputStr))
	for scanner.Scan() {
		line := scanner.Text()
		if reTestPass.MatchString(line) {
			result.TestsRun++
			result.TestsPassed++
		} else if reTestFail.MatchString(line) {
			result.TestsRun++
			result.TestsFailed++
			matches := reTestFail.FindStringSubmatch(line)
			if len(matches) > 1 {
				result.Errors = append(result.Errors, models.BuildError{
					Type:     models.GameErrorTest,
					Message:  matches[1],
					Severity: "error",
				})
			}
		}
	}
}

// ValidateSceneFiles scans for .tscn files and validates their basic structure
func (v *GodotValidator) ValidateSceneFiles(projectPath string, result *models.GameBuildResult) error {
	return filepath.Walk(projectPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if info.IsDir() || !strings.HasSuffix(path, ".tscn") {
			return nil
		}

		// Basic validation: check file starts with [gd_scene or [gd_resource
		data, err := os.ReadFile(path)
		if err != nil {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScene,
				File:     path,
				Message:  fmt.Sprintf("Cannot read scene file: %v", err),
				Severity: "error",
			})
			return nil
		}

		content := strings.TrimSpace(string(data))
		if !strings.HasPrefix(content, "[gd_scene") && !strings.HasPrefix(content, "[gd_resource") {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScene,
				File:     path,
				Message:  "Invalid scene file: missing [gd_scene or [gd_resource header",
				Severity: "error",
			})
		}

		return nil
	})
}

// ValidateGDScripts scans for .gd files and checks for syntax issues
// Implements guardrails from Sentinel Profile - GDScript - Godot
func (v *GodotValidator) ValidateGDScripts(projectPath string, result *models.GameBuildResult) error {
	return filepath.Walk(projectPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if info.IsDir() || !strings.HasSuffix(path, ".gd") {
			return nil
		}

		data, err := os.ReadFile(path)
		if err != nil {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScript,
				File:     path,
				Message:  fmt.Sprintf("Cannot read script file: %v", err),
				Severity: "error",
			})
			return nil
		}

		content := string(data)

		// Check 1: Empty script file
		if len(content) == 0 {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScript,
				File:     path,
				Message:  "Empty script file",
				Severity: "warning",
			})
			return nil
		}

		// Check 2: Missing 'extends' declaration (GDScript Mandate rule)
		if !strings.Contains(content, "extends ") && !strings.Contains(content, "@tool") {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScript,
				File:     path,
				Message:  "Script missing 'extends' declaration",
				Severity: "warning",
			})
		}

		// Check 3: Direct .free() on nodes (use queue_free instead)
		if strings.Contains(content, ".free()") && !strings.Contains(content, "queue_free") {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScript,
				File:     path,
				Message:  "Direct .free() detected — use queue_free() for safe deferred deletion",
				Severity: "warning",
			})
		}

		// Check 4: String-based get_node() (fragile, use @onready)
		if strings.Contains(content, "get_node(\"") || strings.Contains(content, "get_node('") {
			result.Errors = append(result.Errors, models.BuildError{
				Type:     models.GameErrorScript,
				File:     path,
				Message:  "String-based get_node() detected — prefer @onready with $ or % syntax",
				Severity: "warning",
			})
		}

		// Check 5: Heavy operations in _process (file I/O, HTTP)
		processFuncs := []string{"_process", "_physics_process"}
		for _, fn := range processFuncs {
			if idx := strings.Index(content, "func "+fn); idx >= 0 {
				// Find end of function (rough heuristic: next 'func' or end of file)
				funcBody := content[idx:]
				if nextFunc := strings.Index(funcBody[10:], "func "); nextFunc > 0 {
					funcBody = funcBody[:nextFunc+10]
				}
				heavyOps := []string{"FileAccess", "HTTPRequest.request", "load(", "ResourceLoader.load"}
				for _, op := range heavyOps {
					if strings.Contains(funcBody, op) {
						result.Errors = append(result.Errors, models.BuildError{
							Type:     models.GameErrorScript,
							File:     path,
							Message:  fmt.Sprintf("Heavy operation '%s' in %s — move to _ready() or background thread", op, fn),
							Severity: "warning",
						})
					}
				}
			}
		}

		return nil
	})
}

// truncate truncates a string to maxLen characters
func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}
