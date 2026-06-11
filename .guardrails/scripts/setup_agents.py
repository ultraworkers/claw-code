#!/usr/bin/env python3
"""
Agent Guardrails Setup Script

Installs pre-committed guardrails configurations for various AI coding platforms.

Usage:
    # Full platform install
    python scripts/setup_agents.py --install --platform claude,cursor,opencode,windsurf,copilot

    # Clone a single skill file (no repo clone needed)
    python scripts/setup_agents.py --clone .claude/skills/guardrails-enforcer.json
    python scripts/setup_agents.py --clone .claude/skills/guardrails-enforcer.json --target ~/myproject

    # Install a single skill by name
    python scripts/setup_agents.py --install-skill guardrails-enforcer
    python scripts/setup_agents.py --install-skill commit-validator --target ~/myproject --platform claude

    # List available skills
    python scripts/setup_agents.py --list-skills

    # MCP tool also supports per-skill args:
    # guardrail_install_skills({ skill: "guardrails-enforcer", platform: "claude" })
"""

import argparse
import json
import os
import shutil
import sys
import urllib.request
import urllib.error
from pathlib import Path
from typing import Optional


SCRIPT_DIR = Path(__file__).parent.resolve()
REPO_ROOT = SCRIPT_DIR.parent
REPO_OWNER = "TheArchitectit"
REPO_NAME = "agent-guardrails-template"
RAW_BASE = f"https://raw.githubusercontent.com/{REPO_OWNER}/{REPO_NAME}/main"
GITHUB_API = f"https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}"

PLATFORM_CONFIGS = {
    "claude": {
        "source": REPO_ROOT / ".claude",
        "description": "Claude Code skills and hooks",
        "target_name": ".claude",
    },
    "cursor": {
        "source": REPO_ROOT / ".cursor" / "rules",
        "description": "Cursor rules",
        "target_name": ".cursor/rules",
    },
    "opencode": {
        "source": REPO_ROOT / ".opencode",
        "description": "OpenCode agents and skills",
        "target_name": ".opencode",
    },
    "windsurf": {
        "source": REPO_ROOT / ".windsurfrules",
        "description": "Windsurf rules",
        "target_name": ".windsurfrules",
    },
    "copilot": {
        "source": REPO_ROOT / ".github" / "copilot-instructions.md",
        "description": "GitHub Copilot instructions",
        "target_name": ".github/copilot-instructions.md",
    },
}

# Per-skill registry: skill_name -> (repo_path, target_subdir)
SKILL_REGISTRY = {
    # Claude Code skills
    "guardrails-enforcer": (".claude/skills/guardrails-enforcer.json", ".claude/skills/"),
    "commit-validator": (".claude/skills/commit-validator.json", ".claude/skills/"),
    "env-separator": (".claude/skills/env-separator.json", ".claude/skills/"),
    "scope-validator": (".claude/skills/scope-validator.json", ".claude/skills/"),
    "production-first": (".claude/skills/production-first.json", ".claude/skills/"),
    "three-strikes": (".claude/skills/three-strikes.json", ".claude/skills/"),
    "error-recovery": (".claude/skills/error-recovery.json", ".claude/skills/"),
    # Claude hooks
    "pre-commit-hook": (".claude/hooks/pre-commit.sh", ".claude/hooks/"),
    "pre-execution-hook": (".claude/hooks/pre-execution.sh", ".claude/hooks/"),
    "post-execution-hook": (".claude/hooks/post-execution.sh", ".claude/hooks/"),
    # Cursor rules
    "cursor-guardrails": (".cursor/rules/guardrails-enforcer.md", ".cursor/rules/"),
    "cursor-production-first": (".cursor/rules/production-first.md", ".cursor/rules/"),
    "cursor-three-strikes": (".cursor/rules/three-strikes.md", ".cursor/rules/"),
    # Windsurf
    "windsurf-rules": (".windsurfrules", ".windsurfrules"),
    # Copilot
    "copilot-instructions": (".github/copilot-instructions.md", ".github/"),
    # OpenCode
    "opencode-config": (".opencode/oh-my-opencode.jsonc", ".opencode/"),
    "opencode-guardrails": (".opencode/skills/guardrails-enforcer/SKILL.md", ".opencode/skills/guardrails-enforcer/"),
    "opencode-commit-validator": (".opencode/skills/commit-validator/SKILL.md", ".opencode/skills/commit-validator/"),
    # Shared prompts
    "four-laws": ("skills/shared-prompts/four-laws.md", "skills/shared-prompts/"),
    "halt-conditions": ("skills/shared-prompts/halt-conditions.md", "skills/shared-prompts/"),
    "vibe-coding": ("skills/shared-prompts/vibe-coding.md", "skills/shared-prompts/"),
    "error-recovery-md": ("skills/shared-prompts/error-recovery.md", "skills/shared-prompts/"),
    "three-strikes-md": ("skills/shared-prompts/three-strikes.md", "skills/shared-prompts/"),
    "production-first-md": ("skills/shared-prompts/production-first.md", "skills/shared-prompts/"),
    "scope-validation": ("skills/shared-prompts/scope-validation.md", "skills/shared-prompts/"),
}


def resolve_target(target_root: Optional[str], skill_name: str) -> tuple[Path, Path]:
    """Return (source_path, target_path) for a skill."""
    repo_path, target_subdir = SKILL_REGISTRY[skill_name]
    if target_root:
        base = Path(target_root).resolve()
    else:
        base = REPO_ROOT
    source = REPO_ROOT / repo_path
    target_dir = base / target_subdir
    target = target_dir / Path(repo_path).name
    return source, target


def ensure_parent_dirs(path: Path) -> None:
    """Ensure parent directories exist."""
    parent = path.parent
    if parent != path:
        parent.mkdir(parents=True, exist_ok=True)


def get_current_branch() -> str:
    """Get the current git branch for the repository."""
    try:
        import subprocess
        result = subprocess.run(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except Exception:
        return "main"


def get_default_branch() -> str:
    """Get the default branch name (main or master)."""
    try:
        import subprocess
        result = subprocess.run(
            ["git", "symbolic-ref", "refs/remotes/origin/HEAD"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        # refs/remotes/origin/main -> main
        return result.stdout.strip().split("/")[-1]
    except Exception:
        return "main"


def download_file(url: str, target: Path, dry_run: bool) -> bool:
    """Download a single file from a raw GitHub URL."""
    if dry_run:
        print(f"[DRY-RUN] Would download: {url}")
        print(f"[DRY-RUN]            -> {target}")
        return True

    try:
        ensure_parent_dirs(target)
        with urllib.request.urlopen(url) as response:
            content = response.read()
        with open(target, "wb") as f:
            f.write(content)
        print(f"[OK] Downloaded: {url}")
        print(f"     -> {target}")
        return True
    except urllib.error.HTTPError as e:
        return None  # Signal 404 for branch fallback
    except Exception as e:
        print(f"[ERROR] Download failed: {e}")
        return False


def clone_skill(path: str, target_root: Optional[str], dry_run: bool) -> bool:
    """Clone (download) a single skill file by repo path. Tries default branch, falls back to current branch."""
    if target_root:
        base = Path(target_root).resolve()
    else:
        base = Path(".")

    # Determine target: strip leading ./ or /
    safe_path = path.lstrip("./")
    target = base / safe_path

    if dry_run:
        print(f"[DRY-RUN] Would download: {RAW_BASE}/{path}")
        print(f"[DRY-RUN]            -> {target}")
        return True

    # Try default branch first
    default_branch = get_default_branch()
    url = f"https://raw.githubusercontent.com/{REPO_OWNER}/{REPO_NAME}/{default_branch}/{path}"
    result = download_file(url, target, dry_run)
    if result is True:
        return True
    if result is False:
        return False  # Real error, not 404

    # Fall back to current branch (for feature branches not yet on main)
    current_branch = get_current_branch()
    url = f"https://raw.githubusercontent.com/{REPO_OWNER}/{REPO_NAME}/{current_branch}/{path}"
    result = download_file(url, target, dry_run)
    if result is True:
        print(f"[INFO] Downloaded from feature branch: {current_branch}")
    return result if result is not None else False


def clone_skill_by_name(name: str, target_root: Optional[str], dry_run: bool) -> bool:
    """Clone a skill by its registry name."""
    if name not in SKILL_REGISTRY:
        print(f"[ERROR] Unknown skill: {name}")
        print(f"[INFO] Run --list-skills to see available skills.")
        return False

    repo_path, _ = SKILL_REGISTRY[name]
    return clone_skill(repo_path, target_root, dry_run)


def install_skill(name: str, target_root: Optional[str], mode: str, dry_run: bool) -> bool:
    """Install a single skill from repo to target directory."""
    if name not in SKILL_REGISTRY:
        print(f"[ERROR] Unknown skill: {name}")
        print(f"[INFO] Run --list-skills to see available skills.")
        return False

    source, target = resolve_target(target_root, name)

    if dry_run:
        action = "symlink" if mode == "symlink" else "copy"
        print(f"[DRY-RUN] Would {action}: {source} -> {target}")
        return True

    if not source.exists():
        print(f"[ERROR] Source not found: {source}")
        return False

    if target.exists():
        print(f"[WARN] Target exists, skipping: {target}")
        return True

    ensure_parent_dirs(target)

    try:
        if mode == "symlink":
            rel_source = os.path.relpath(source, target.parent)
            target.symlink_to(rel_source)
            print(f"[OK] Symlinked: {target} -> {rel_source}")
        else:
            shutil.copy2(source, target)
            print(f"[OK] Copied: {source} -> {target}")
        return True
    except Exception as e:
        print(f"[ERROR] Failed to install {name}: {e}")
        return False


def list_skills() -> None:
    """List all available skills."""
    print("Available skills:")
    current_platform = None
    for name, (repo_path, _) in SKILL_REGISTRY.items():
        platform = repo_path.split("/")[0].lstrip(".")
        if platform != current_platform:
            current_platform = platform
            print(f"\n  [{platform}]")
        print(f"    {name:30s}  {repo_path}")


def install_platform(target_root: Optional[str], platform: str, mode: str, dry_run: bool) -> bool:
    """Install all configs for a single platform."""
    config = PLATFORM_CONFIGS[platform]
    source = config["source"]
    if target_root:
        base = Path(target_root).resolve()
    else:
        base = REPO_ROOT
    target = base / config["target_name"]

    if dry_run:
        action = "symlink" if mode == "symlink" else "copy"
        print(f"[DRY-RUN] Would {action}: {source} -> {target}")
        return True

    if not source.exists():
        print(f"[ERROR] Source not found: {source}")
        return False

    if target.exists():
        print(f"[WARN] Target exists, skipping: {target}")
        return True

    ensure_parent_dirs(target)

    try:
        if mode == "symlink":
            rel_source = os.path.relpath(source, target.parent)
            target.symlink_to(rel_source)
            print(f"[OK] Symlinked: {target} -> {rel_source}")
        else:
            if source.is_dir():
                shutil.copytree(source, target, dirs_exist_ok=False)
            else:
                shutil.copy2(source, target)
            print(f"[OK] Copied: {source} -> {target}")
        return True
    except Exception as e:
        print(f"[ERROR] Failed to install {platform}: {e}")
        return False


def install_all(target_root: Optional[str], platforms: list[str], mode: str, dry_run: bool) -> bool:
    """Install configs for all specified platforms."""
    success = True
    for platform in platforms:
        if platform not in PLATFORM_CONFIGS:
            print(f"[ERROR] Unknown platform: {platform}")
            success = False
            continue
        if not install_platform(target_root, platform, mode, dry_run):
            success = False
    return success


def validate_sources(platforms: list[str]) -> bool:
    """Validate that all source files exist."""
    missing = []
    for platform in platforms:
        if platform not in PLATFORM_CONFIGS:
            missing.append(platform)
            continue
        source = PLATFORM_CONFIGS[platform]["source"]
        if not source.exists():
            missing.append(f"{platform} ({source})")
    if missing:
        print("[ERROR] Missing source files:")
        for m in missing:
            print(f"  {m}")
        return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Install agent guardrails for Claude Code, Cursor, OpenCode, Windsurf, and Copilot."
    )
    parser.add_argument(
        "--install",
        action="store_true",
        help="Install all configs for a platform (or all platforms)",
    )
    parser.add_argument(
        "--clone",
        type=str,
        metavar="PATH",
        help="Clone a single skill file by repo path (e.g. .claude/skills/guardrails-enforcer.json)",
    )
    parser.add_argument(
        "--install-skill",
        type=str,
        metavar="NAME",
        help="Install a single skill by name (e.g. guardrails-enforcer). Use --list-skills to see all.",
    )
    parser.add_argument(
        "--platform",
        type=str,
        default="all",
        help="Comma-separated list of platforms: claude, cursor, opencode, windsurf, copilot (default: all)",
    )
    parser.add_argument(
        "--target",
        type=str,
        default=None,
        help="Target project directory (default: current directory or repo root)",
    )
    parser.add_argument(
        "--mode",
        type=str,
        choices=["copy", "symlink"],
        default="copy",
        help="Installation mode: copy or symlink (default: copy)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what would be installed without making changes",
    )
    parser.add_argument(
        "--list-platforms",
        action="store_true",
        help="List available platforms and exit",
    )
    parser.add_argument(
        "--list-skills",
        action="store_true",
        help="List all available skills and exit",
    )

    args = parser.parse_args()

    if args.list_platforms:
        print("Available platforms:")
        for name, config in PLATFORM_CONFIGS.items():
            exists = " [exists]" if config["source"].exists() else " [missing]"
            print(f"  {name}: {config['description']}{exists}")
        return 0

    if args.list_skills:
        list_skills()
        return 0

    # Clone a single file by repo path (e.g. --clone .claude/skills/guardrails-enforcer.json)
    if args.clone:
        ok = clone_skill(args.clone, args.target, args.dry_run)
        return 0 if ok else 1

    # Install a single skill by name (e.g. --install-skill guardrails-enforcer)
    if args.install_skill:
        ok = install_skill(args.install_skill, args.target, args.mode, args.dry_run)
        return 0 if ok else 1

    # Install full platform(s)
    if args.install or args.dry_run:
        platforms = [p.strip() for p in args.platform.split(",")]
        if "all" in platforms:
            platforms = list(PLATFORM_CONFIGS.keys())

        if not validate_sources(platforms):
            return 1

        ok = install_all(args.target, platforms, args.mode, args.dry_run)
        if args.dry_run:
            print("\n[INFO] Dry-run complete.")
        else:
            print(f"\n[OK] Installed guardrails for: {', '.join(platforms)}")
        return 0 if ok else 1

    parser.print_help()
    print("\nExamples:")
    print("  python scripts/setup_agents.py --install --platform claude")
    print("  python scripts/setup_agents.py --install-skill guardrails-enforcer")
    print("  python scripts/setup_agents.py --clone .claude/skills/guardrails-enforcer.json")
    print("  python scripts/setup_agents.py --list-skills")
    return 1


if __name__ == "__main__":
    sys.exit(main())
