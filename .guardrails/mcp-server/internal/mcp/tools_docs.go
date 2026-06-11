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

// handleGetStandard fetches a standards document by name
func (s *MCPServer) handleGetStandard(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	name, _ := args["name"].(string)
	if name == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"name parameter required"}`}},
			IsError: true,
		}, nil
	}

	repoPath := s.getRepoPath()
	// Try with and without .md extension
	fileName := name
	if !strings.HasSuffix(fileName, ".md") {
		fileName += ".md"
	}

	filePath := filepath.Join(repoPath, "docs", "standards", fileName)
	content, err := os.ReadFile(filePath)
	if err != nil {
		// Try searching for partial matches
		entries, readErr := os.ReadDir(filepath.Join(repoPath, "docs", "standards"))
		if readErr != nil {
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"error":"Standard not found: %s"}`, name)}},
				IsError: true,
			}, nil
		}
		// Find partial match
		for _, entry := range entries {
			if strings.Contains(strings.ToUpper(entry.Name()), strings.ToUpper(name)) {
				content, err = os.ReadFile(filepath.Join(repoPath, "docs", "standards", entry.Name()))
				if err == nil {
					name = entry.Name()
					break
				}
			}
		}
		if content == nil {
			available := make([]string, 0)
			for _, entry := range entries {
				if strings.HasSuffix(entry.Name(), ".md") && entry.Name() != "INDEX.md" {
					available = append(available, strings.TrimSuffix(entry.Name(), ".md"))
				}
			}
			result := map[string]interface{}{
				"error":     fmt.Sprintf("Standard not found: %s", name),
				"available": available,
			}
			resultJSON, _ := json.Marshal(result)
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
				IsError: true,
			}, nil
		}
	}

	result := map[string]interface{}{
		"name":    name,
		"content": string(content),
		"type":    "standard",
	}
	resultJSON, _ := json.Marshal(result)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// handleGetWorkflow fetches a workflow document by name
func (s *MCPServer) handleGetWorkflow(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	name, _ := args["name"].(string)
	if name == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"name parameter required"}`}},
			IsError: true,
		}, nil
	}

	repoPath := s.getRepoPath()
	fileName := name
	if !strings.HasSuffix(fileName, ".md") {
		fileName += ".md"
	}

	filePath := filepath.Join(repoPath, "docs", "workflows", fileName)
	content, err := os.ReadFile(filePath)
	if err != nil {
		// Try partial match
		entries, _ := os.ReadDir(filepath.Join(repoPath, "docs", "workflows"))
		for _, entry := range entries {
			if strings.Contains(strings.ToUpper(entry.Name()), strings.ToUpper(name)) {
				content, _ = os.ReadFile(filepath.Join(repoPath, "docs", "workflows", entry.Name()))
				if content != nil {
					name = entry.Name()
					break
				}
			}
		}
		if content == nil {
			available := make([]string, 0)
			for _, entry := range entries {
				if strings.HasSuffix(entry.Name(), ".md") && entry.Name() != "INDEX.md" {
					available = append(available, strings.TrimSuffix(entry.Name(), ".md"))
				}
			}
			result := map[string]interface{}{
				"error":     fmt.Sprintf("Workflow not found: %s", name),
				"available": available,
			}
			resultJSON, _ := json.Marshal(result)
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
				IsError: true,
			}, nil
		}
	}

	result := map[string]interface{}{
		"name":    name,
		"content": string(content),
		"type":    "workflow",
	}
	resultJSON, _ := json.Marshal(result)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// handleSearchDocs searches all documentation for a query
func (s *MCPServer) handleSearchDocs(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	query, _ := args["query"].(string)
	if query == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"query parameter required"}`}},
			IsError: true,
		}, nil
	}

	repoPath := s.getRepoPath()
	queryLower := strings.ToLower(query)

	type searchResult struct {
		Path    string `json:"path"`
		Name    string `json:"name"`
		Type    string `json:"type"`
		Snippet string `json:"snippet"`
		Score   int    `json:"score"`
	}

	results := []searchResult{}

	// Search directories
	searchDirs := []struct {
		path string
		docType string
	}{
		{filepath.Join(repoPath, "docs", "standards"), "standard"},
		{filepath.Join(repoPath, "docs", "workflows"), "workflow"},
		{filepath.Join(repoPath, "docs", "agentmcp"), "profile"},
		{filepath.Join(repoPath, "docs"), "root"},
	}

	for _, dir := range searchDirs {
		entries, err := os.ReadDir(dir.path)
		if err != nil {
			continue
		}
		for _, entry := range entries {
			if entry.IsDir() {
				continue
			}
			ext := strings.ToLower(filepath.Ext(entry.Name()))
			if ext != ".md" && ext != ".txt" {
				continue
			}

			filePath := filepath.Join(dir.path, entry.Name())
			content, err := os.ReadFile(filePath)
			if err != nil {
				continue
			}
			contentStr := string(content)
			contentLower := strings.ToLower(contentStr)

			// Score based on matches
			score := 0
			// Title match (first line or filename)
			firstLine := strings.Split(contentStr, "\n")[0]
			if strings.Contains(strings.ToLower(firstLine), queryLower) {
				score += 10
			}
			if strings.Contains(strings.ToLower(entry.Name()), queryLower) {
				score += 5
			}
			// Content frequency
			score += strings.Count(contentLower, queryLower)

			if score > 0 {
				// Extract snippet around first match
				snippet := ""
				if idx := strings.Index(contentLower, queryLower); idx >= 0 {
					start := idx - 50
					if start < 0 {
						start = 0
					}
					end := idx + len(query) + 100
					if end > len(contentStr) {
						end = len(contentStr)
					}
					snippet = contentStr[start:end]
					if start > 0 {
						snippet = "..." + snippet
					}
					if end < len(contentStr) {
						snippet = snippet + "..."
					}
				}

				relPath := strings.TrimPrefix(filePath, repoPath+"/")
				results = append(results, searchResult{
					Path:    relPath,
					Name:    strings.TrimSuffix(entry.Name(), filepath.Ext(entry.Name())),
					Type:    dir.docType,
					Snippet: snippet,
					Score:   score,
				})
			}
		}
	}

	// Sort by score (simple: just return in order, client can sort)
	resultJSON, _ := json.Marshal(map[string]interface{}{
		"query":   query,
		"results": results,
		"total":   len(results),
	})
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}
