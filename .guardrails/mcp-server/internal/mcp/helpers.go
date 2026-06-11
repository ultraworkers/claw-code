package mcp

import (
	"os"
)

// getRepoPath returns the base path to the guardrails repository
// where docs/ and .guardrails/ directories are located.
// Uses GUARDRAILS_REPO_PATH env var, or current working directory.
func (s *MCPServer) getRepoPath() string {
	if path := os.Getenv("GUARDRAILS_REPO_PATH"); path != "" {
		return path
	}
	// Default: one level up from mcp-server directory
	// (repo root contains docs/ and .guardrails/)
	path, err := os.Getwd()
	if err != nil {
		return "."
	}
	return path
}
