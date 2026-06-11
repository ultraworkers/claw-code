package models

import (
	"strings"
	"testing"
)

func TestProject_Validate(t *testing.T) {
	tests := []struct {
		name    string
		project Project
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid project",
			project: Project{
				Name: "Test Project",
				Slug: "test-project",
			},
			wantErr: false,
		},
		{
			name: "valid project with underscore",
			project: Project{
				Name: "Test_Project",
				Slug: "test_project",
			},
			wantErr: false,
		},
		{
			name: "valid project with mixed case",
			project: Project{
				Name: "TestProject",
				Slug: "Test-Project-123",
			},
			wantErr: false,
		},
		{
			name: "missing name",
			project: Project{
				Name: "",
				Slug: "test-project",
			},
			wantErr: true,
			errMsg:  "name is required",
		},
		{
			name: "missing slug",
			project: Project{
				Name: "Test Project",
				Slug: "",
			},
			wantErr: true,
			errMsg:  "slug is required",
		},
		{
			name: "name too long",
			project: Project{
				Name: strings.Repeat("a", 256),
				Slug: "test-project",
			},
			wantErr: true,
			errMsg:  "name must be at most 255 characters",
		},
		{
			name: "slug too long",
			project: Project{
				Name: "Test Project",
				Slug: strings.Repeat("a", 101),
			},
			wantErr: true,
			errMsg:  "slug must be at most 100 characters",
		},
		{
			name: "slug with spaces",
			project: Project{
				Name: "Test Project",
				Slug: "test project",
			},
			wantErr: true,
			errMsg:  "slug contains invalid characters",
		},
		{
			name: "slug with special characters",
			project: Project{
				Name: "Test Project",
				Slug: "test@project!",
			},
			wantErr: true,
			errMsg:  "slug contains invalid characters",
		},
		{
			name: "slug with dots",
			project: Project{
				Name: "Test Project",
				Slug: "test.project",
			},
			wantErr: true,
			errMsg:  "slug contains invalid characters",
		},
		{
			name: "slug with slashes",
			project: Project{
				Name: "Test Project",
				Slug: "test/project",
			},
			wantErr: true,
			errMsg:  "slug contains invalid characters",
		},
		{
			name: "boundary name length",
			project: Project{
				Name: strings.Repeat("a", 255),
				Slug: "test-project",
			},
			wantErr: false,
		},
		{
			name: "boundary slug length",
			project: Project{
				Name: "Test Project",
				Slug: strings.Repeat("a", 100),
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.project.Validate()
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

func Test_isValidSlugChar(t *testing.T) {
	tests := []struct {
		name string
		r    rune
		want bool
	}{
		// Valid characters
		{"lowercase a", 'a', true},
		{"lowercase z", 'z', true},
		{"uppercase A", 'A', true},
		{"uppercase Z", 'Z', true},
		{"digit 0", '0', true},
		{"digit 9", '9', true},
		{"hyphen", '-', true},
		{"underscore", '_', true},

		// Invalid characters
		{"space", ' ', false},
		{"dot", '.', false},
		{"slash", '/', false},
		{"backslash", '\\', false},
		{"at symbol", '@', false},
		{"exclamation", '!', false},
		{"hash", '#', false},
		{"percent", '%', false},
		{"ampersand", '&', false},
		{"asterisk", '*', false},
		{"plus", '+', false},
		{"equals", '=', false},
		{"question mark", '?', false},
		{"newline", '\n', false},
		{"tab", '\t', false},
		{"unicode", 'ñ', false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := isValidSlugChar(tt.r)
			if got != tt.want {
				t.Errorf("isValidSlugChar(%q) = %v, want %v", tt.r, got, tt.want)
			}
		})
	}
}

func BenchmarkProject_Validate(b *testing.B) {
	project := Project{
		Name: "Benchmark Test Project",
		Slug: "benchmark-test-project",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = project.Validate()
	}
}

func Benchmark_isValidSlugChar(b *testing.B) {
	testRunes := []rune{'a', 'Z', '5', '-', '_', ' ', '.', '/'}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, r := range testRunes {
			_ = isValidSlugChar(r)
		}
	}
}
