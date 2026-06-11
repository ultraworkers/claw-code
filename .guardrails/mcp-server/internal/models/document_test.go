package models

import (
	"strings"
	"testing"
)

func TestDocument_Validate(t *testing.T) {
	tests := []struct {
		name    string
		doc     Document
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid document",
			doc: Document{
				Slug:     "test-doc",
				Title:    "Test Document",
				Content:  "This is test content",
				Category: "workflow",
				Path:     "/docs/test.md",
				Version:  1,
			},
			wantErr: false,
		},
		{
			name: "valid document with different category",
			doc: Document{
				Slug:     "reference-doc",
				Title:    "Reference Document",
				Content:  "Reference content",
				Category: "reference",
				Path:     "/docs/ref.md",
				Version:  2,
			},
			wantErr: false,
		},
		{
			name: "missing slug",
			doc: Document{
				Slug:     "",
				Title:    "Test Document",
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: true,
			errMsg:  "slug is required",
		},
		{
			name: "missing title",
			doc: Document{
				Slug:     "test-doc",
				Title:    "",
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: true,
			errMsg:  "title is required",
		},
		{
			name: "missing content",
			doc: Document{
				Slug:     "test-doc",
				Title:    "Test Document",
				Content:  "",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: true,
			errMsg:  "content is required",
		},
		{
			name: "invalid category",
			doc: Document{
				Slug:     "test-doc",
				Title:    "Test Document",
				Content:  "Content",
				Category: "invalid-category",
				Path:     "/docs/test.md",
			},
			wantErr: true,
			errMsg:  "invalid category",
		},
		{
			name: "missing path",
			doc: Document{
				Slug:     "test-doc",
				Title:    "Test Document",
				Content:  "Content",
				Category: "guide",
				Path:     "",
			},
			wantErr: true,
			errMsg:  "path is required",
		},
		{
			name: "slug too long",
			doc: Document{
				Slug:     strings.Repeat("a", 256),
				Title:    "Test Document",
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: true,
			errMsg:  "slug must be at most 255 characters",
		},
		{
			name: "title too long",
			doc: Document{
				Slug:     "test-doc",
				Title:    strings.Repeat("a", 501),
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: true,
			errMsg:  "title must be at most 500 characters",
		},
		{
			name: "path too long",
			doc: Document{
				Slug:     "test-doc",
				Title:    "Test Document",
				Content:  "Content",
				Category: "guide",
				Path:     strings.Repeat("a", 501),
			},
			wantErr: true,
			errMsg:  "path must be at most 500 characters",
		},
		{
			name: "version defaults to 1",
			doc: Document{
				Slug:     "test-doc",
				Title:    "Test Document",
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
				Version:  0,
			},
			wantErr: false,
		},
		{
			name: "boundary slug length",
			doc: Document{
				Slug:     strings.Repeat("a", 255),
				Title:    "Test Document",
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: false,
		},
		{
			name: "boundary title length",
			doc: Document{
				Slug:     "test-doc",
				Title:    strings.Repeat("a", 500),
				Content:  "Content",
				Category: "guide",
				Path:     "/docs/test.md",
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.doc.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("Validate() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("Validate() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestIsValidCategory(t *testing.T) {
	tests := []struct {
		name string
		cat  string
		want bool
	}{
		{"workflow category", "workflow", true},
		{"standard category", "standard", true},
		{"guide category", "guide", true},
		{"reference category", "reference", true},
		{"empty string", "", false},
		{"invalid category", "invalid", false},
		{"uppercase workflow", "WORKFLOW", false},
		{"mixed case", "Workflow", false},
		{"similar but invalid", "workflows", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsValidCategory(tt.cat)
			if got != tt.want {
				t.Errorf("IsValidCategory(%q) = %v, want %v", tt.cat, got, tt.want)
			}
		})
	}
}

func TestDocumentCategory_Constants(t *testing.T) {
	// Test that all expected constants are defined
	categories := []DocumentCategory{
		CategoryWorkflow,
		CategoryStandard,
		CategoryGuide,
		CategoryReference,
	}

	expected := []string{"workflow", "standard", "guide", "reference"}

	for i, cat := range categories {
		if string(cat) != expected[i] {
			t.Errorf("Category constant %d = %q, want %q", i, cat, expected[i])
		}
	}
}

func BenchmarkDocument_Validate(b *testing.B) {
	doc := Document{
		Slug:     "benchmark-doc",
		Title:    "Benchmark Document",
		Content:  "This is benchmark content for testing",
		Category: "workflow",
		Path:     "/docs/benchmark.md",
		Version:  1,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = doc.Validate()
	}
}

func BenchmarkIsValidCategory(b *testing.B) {
	categories := []string{"workflow", "standard", "guide", "reference", "invalid"}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, cat := range categories {
			_ = IsValidCategory(cat)
		}
	}
}
