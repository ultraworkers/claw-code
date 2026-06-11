package validation

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

func TestGameEngineDetector_DetectGodot(t *testing.T) {
	// Create a temp directory with project.godot
	tmpDir, err := os.MkdirTemp("", "godot-project-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	// Write a minimal project.godot
	projFile := filepath.Join(tmpDir, "project.godot")
	if err := os.WriteFile(projFile, []byte("; Engine config\n[application]\nname=\"Test\"\n"), 0644); err != nil {
		t.Fatal(err)
	}

	detector := &GameEngineDetector{}
	result, err := detector.DetectGameEngine(tmpDir)
	if err != nil {
		t.Fatalf("DetectGameEngine failed: %v", err)
	}
	if result == nil {
		t.Fatal("Expected detection result, got nil")
	}
	if result.Engine != models.EngineGodot {
		t.Errorf("Expected engine %s, got %s", models.EngineGodot, result.Engine)
	}
	if result.ConfigFile != "project.godot" {
		t.Errorf("Expected config file project.godot, got %s", result.ConfigFile)
	}
}

func TestGameEngineDetector_NoEngine(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "no-engine-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	detector := &GameEngineDetector{}
	result, err := detector.DetectGameEngine(tmpDir)
	if err != nil {
		t.Fatalf("DetectGameEngine failed: %v", err)
	}
	if result != nil {
		t.Errorf("Expected nil result for non-game directory, got %+v", result)
	}
}

func TestGodotValidator_ValidateProjectFile(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "godot-validate-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	validator := NewGodotValidator("godot")
	result := &models.GameBuildResult{
		Engine:      models.EngineGodot,
		ProjectPath: tmpDir,
		Status:      models.BuildPassed,
	}

	// Without project.godot
	err = validator.validateProjectFile(tmpDir, result)
	if err == nil {
		t.Error("Expected error when project.godot missing")
	}
	if result.Status != models.BuildErrored {
		t.Errorf("Expected status GameBuildError, got %s", result.Status)
	}
}

func TestGodotValidator_ValidateSceneFiles(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scene-validate-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	// Write a valid scene file
	validScene := filepath.Join(tmpDir, "main.tscn")
	if err := os.WriteFile(validScene, []byte("[gd_scene load_steps=2 format=3]\n"), 0644); err != nil {
		t.Fatal(err)
	}

	// Write an invalid scene file
	invalidScene := filepath.Join(tmpDir, "broken.tscn")
	if err := os.WriteFile(invalidScene, []byte("this is not a scene file\n"), 0644); err != nil {
		t.Fatal(err)
	}

	validator := NewGodotValidator("godot")
	result := &models.GameBuildResult{
		Engine:      models.EngineGodot,
		ProjectPath: tmpDir,
		Status:      models.BuildPassed,
	}

	err = validator.ValidateSceneFiles(tmpDir, result)
	if err != nil {
		t.Fatalf("ValidateSceneFiles failed: %v", err)
	}

	// Should have one error for the broken scene
	errCount := 0
	for _, e := range result.Errors {
		if e.File == invalidScene {
			errCount++
		}
	}
	if errCount == 0 {
		t.Error("Expected error for invalid scene file")
	}
}

func TestGodotValidator_ValidateGDScripts(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "script-validate-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	// Write a valid script
	validScript := filepath.Join(tmpDir, "player.gd")
	if err := os.WriteFile(validScript, []byte("extends CharacterBody2D\n\nfunc _ready():\n\tpass\n"), 0644); err != nil {
		t.Fatal(err)
	}

	// Write a script missing extends
	noExtends := filepath.Join(tmpDir, "broken.gd")
	if err := os.WriteFile(noExtends, []byte("func something():\n\tpass\n"), 0644); err != nil {
		t.Fatal(err)
	}

	// Write an empty script
	emptyScript := filepath.Join(tmpDir, "empty.gd")
	if err := os.WriteFile(emptyScript, []byte(""), 0644); err != nil {
		t.Fatal(err)
	}

	validator := NewGodotValidator("godot")
	result := &models.GameBuildResult{
		Engine:      models.EngineGodot,
		ProjectPath: tmpDir,
		Status:      models.BuildPassed,
	}

	err = validator.ValidateGDScripts(tmpDir, result)
	if err != nil {
		t.Fatalf("ValidateGDScripts failed: %v", err)
	}

	// Should have warnings for broken.gd (no extends) and empty.gd
	warningCount := 0
	for _, e := range result.Errors {
		if e.Severity == "warning" {
			warningCount++
		}
	}
	if warningCount < 2 {
		t.Errorf("Expected at least 2 warnings, got %d", warningCount)
	}
}

func TestGodotValidator_NewGodotValidator(t *testing.T) {
	v := NewGodotValidator("")
	if v.GodotPath != "godot" {
		t.Errorf("Expected default godot path 'godot', got %s", v.GodotPath)
	}

	v = NewGodotValidator("/usr/local/bin/godot")
	if v.GodotPath != "/usr/local/bin/godot" {
		t.Errorf("Expected custom godot path, got %s", v.GodotPath)
	}
}
