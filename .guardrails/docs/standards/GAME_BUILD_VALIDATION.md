# Game Build Validation - Guardrail Integration

> Automated game engine project validation as an MCP guardrail tool

---

## Overview

The `guardrail_validate_game_build` MCP tool validates game engine projects (Godot, Unity, Unreal) as part of the agent guardrails safety framework. It runs headless build checks, script validation, and test execution before allowing commits or pushes to game repositories.

## Why This Matters

Game projects have unique validation needs that standard code linters miss:

- **Scene files (.tscn)** can break silently — no compiler catches a missing node reference
- **GDScript** errors only surface when Godot parses the file, not during git diff review
- **Core game loop** bugs (towers don't fire, selling crashes) need runtime testing
- **Mobile export** issues (wrong texture format, missing icons) only appear at export time

Without game-specific guardrails, agents can commit broken scenes, push scripts with syntax errors, and break the game loop without realizing it.

## MCP Tool: `guardrail_validate_game_build`

### Inputs

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `session_token` | string | No | Session token from `guardrail_init_session` |
| `project_path` | string | No* | Path to game project root (auto-detected if omitted) |
| `godot_path` | string | No | Path to Godot binary (default: `godot` from PATH) |
| `check_scenes` | boolean | No | Validate .tscn scene files (default: true) |
| `check_scripts` | boolean | No | Validate .gd script files (default: true) |
| `run_tests` | boolean | No | Run headless test scripts (default: true) |

*If `project_path` is omitted, the tool scans the current directory for `project.godot` and common subdirectories (`game/`, `godot/`, `src/`).

### Output

```json
{
  "session_token": "",
  "engine": "godot",
  "project_path": "/path/to/project",
  "status": "passed",
  "tests_run": 19,
  "tests_passed": 18,
  "tests_failed": 1,
  "duration_ms": 2847,
  "errors": [],
  "validated_at": "2026-04-16T10:30:00Z"
}
```

### Status Values

| Status | Meaning |
|--------|---------|
| `passed` | All checks passed |
| `failed` | Tests failed or errors found |
| `error` | Validation could not run (missing binary, bad path) |
| `skipped` | No game project detected |

### Error Types

| Type | Severity | Description |
|------|----------|-------------|
| `scene_parse` | error | .tscn file has invalid format |
| `script_error` | error | .gd file has parse/runtime errors |
| `export_config` | error | export_presets.cfg missing or invalid |
| `test_failure` | error | Headless test assertion failed |
| `generic` | error | Other validation errors |

Warnings (severity: `warning`) don't fail the build but are reported. Example: GDScript missing `extends` declaration.

## Validation Pipeline

```
1. Detect Engine → Look for project.godot, etc.
2. Validate Config → project.godot exists and parses
3. Headless Check → godot --headless --quit-after 100
4. Scene Validation → Scan .tscn files for valid headers
5. Script Validation → Scan .gd files for extends, non-empty
6. Run Tests → godot --headless --script test_runner.gd
```

## Supported Engines

### Godot (Full Support)

- **Detection:** `project.godot` in project root
- **Headless mode:** `godot --headless --path <project>`
- **Test scripts:** `test_runner.gd`, `tests/test_runner.gd`
- **Custom Godot:** Use `godot_path` to point to automation fork binary
- **Profile:** See `Sentinel Profile - GDScript - Godot.txt` for full language guardrails
- **Polyglot Engine:** Auto-detected and added to the ToolchainManager

### Unity (Planned)

- **Detection:** `ProjectSettings/ProjectSettings.asset`
- **Headless mode:** Unity batch mode
- **Test runner:** Unity Test Framework

### Unreal (Planned)

- **Detection:** `.uproject` file
- **Headless mode:** Unreal Automation Tool
- **Test runner:** Unreal Automation System

## Usage with AI Agents

### Pre-Commit Hook

```json
{
  "tool": "guardrail_validate_game_build",
  "arguments": {
    "project_path": "/mnt/data/git/playgodot-pong/godot",
    "check_scenes": true,
    "check_scripts": true,
    "run_tests": true
  }
}
```

### With Custom Godot Binary (Automation Fork)

```json
{
  "tool": "guardrail_validate_game_build",
  "arguments": {
    "project_path": "/mnt/data/git/MergeKingdom",
    "godot_path": "/mnt/data/git/godot-automation/bin/godot.linuxbsd.editor.x86_64",
    "run_tests": true
  }
}
```

### Quick Check (No Tests)

```json
{
  "tool": "guardrail_validate_game_build",
  "arguments": {
    "project_path": "/path/to/project",
    "run_tests": false
  }
}
```

## Test Script Format

Godot headless test scripts should output `✅ PASS:` and `❌ FAIL:` lines for the validator to parse:

```gdscript
extends SceneTree

func _init() -> void:
    print("========== TESTS ==========")
    
    # Test something
    if some_condition:
        print("  ✅ PASS: Description of what passed")
    else:
        print("  ❌ FAIL: Description of what failed")
    
    print("========== RESULTS ==========")
    quit()
```

## Integration with Pre-Work Check

The game build validation integrates with the existing `guardrail_pre_work_check` tool:

1. When `project.godot` is detected in the repo, `pre_work_check` automatically recommends running `validate_game_build`
2. Files with `.tscn` or `.gd` extensions trigger game-specific failure registry checks
3. Known game bugs (towers not firing, crash on sell) are added to the failure registry

## Proven Results

Tested against the playgodot-pong project:

- **18/19 headless tests pass** (scene loading, node structure, game state, ball physics, score tracking)
- **Scene validation** catches malformed .tscn files
- **Script validation** catches missing extends declarations, empty scripts
- **Auto-detection** finds project.godot in root and subdirectories

Custom Godot build: `4.6.stable.custom_build.a8cacb6f2` (Randroids-Dojo/godot automation branch)

## Future Enhancements

- [ ] Unity batch mode support
- [ ] Unreal AUT support
- [ ] Export validation (Android/iOS export presets)
- [ ] Asset compliance (texture sizes, audio formats)
- [ ] Performance profiling (frame time analysis)
- [ ] PlayGodot Python API integration (screenshot-based UI testing)
