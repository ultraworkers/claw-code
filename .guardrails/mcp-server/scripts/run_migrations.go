// Migration runner for AI01 database
// Usage: go run scripts/run_migrations.go <database_url>
// Example: go run scripts/run_migrations.go "postgresql://guardrail:guardrail123@localhost:5432/guardrail?sslmode=disable"

package main

import (
	"database/sql"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	_ "github.com/jackc/pgx/v5/stdlib"
)

func main() {
	if len(os.Args) < 2 {
		log.Fatal("Usage: go run scripts/run_migrations.go <database_url>")
	}

	dbURL := os.Args[1]

	db, err := sql.Open("pgx", dbURL)
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}
	defer db.Close()

	if err := db.Ping(); err != nil {
		log.Fatalf("Failed to ping database: %v", err)
	}

	log.Println("Connected to database")

	// Create migrations table if not exists
	if err := createMigrationsTable(db); err != nil {
		log.Fatalf("Failed to create migrations table: %v", err)
	}

	// Get list of migration files
	migrationsDir := "internal/database/migrations"
	files, err := os.ReadDir(migrationsDir)
	if err != nil {
		log.Fatalf("Failed to read migrations directory: %v", err)
	}

	// Apply pending migrations
	for _, file := range files {
		if strings.HasSuffix(file.Name(), ".up.sql") {
			if err := applyMigration(db, migrationsDir, file.Name()); err != nil {
				log.Fatalf("Failed to apply migration %s: %v", file.Name(), err)
			}
		}
	}

	// Verify tables
	if err := verifyTables(db); err != nil {
		log.Fatalf("Table verification failed: %v", err)
	}

	log.Println("\n✅ All migrations applied and verified successfully!")
}

func createMigrationsTable(db *sql.DB) error {
	_, err := db.Exec(`
		CREATE TABLE IF NOT EXISTS schema_migrations (
			version VARCHAR(255) PRIMARY KEY,
			applied_at TIMESTAMP NOT NULL DEFAULT NOW()
		)
	`)
	return err
}

func isMigrationApplied(db *sql.DB, version string) bool {
	var count int
	err := db.QueryRow("SELECT COUNT(*) FROM schema_migrations WHERE version = $1", version).Scan(&count)
	if err != nil {
		return false
	}
	return count > 0
}

func applyMigration(db *sql.DB, dir, filename string) error {
	version := strings.TrimSuffix(filename, ".up.sql")

	if isMigrationApplied(db, version) {
		log.Printf("Skipping %s (already applied)", filename)
		return nil
	}

	content, err := os.ReadFile(filepath.Join(dir, filename))
	if err != nil {
		return fmt.Errorf("failed to read migration file: %w", err)
	}

	log.Printf("Applying migration: %s", filename)

	// Execute migration in a transaction
	tx, err := db.Begin()
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	// Split and execute statements
	statements := splitStatements(string(content))
	for _, stmt := range statements {
		stmt = strings.TrimSpace(stmt)
		if stmt == "" || strings.HasPrefix(stmt, "--") || strings.HasPrefix(stmt, "\\echo") {
			continue
		}

		if _, err := tx.Exec(stmt); err != nil {
			return fmt.Errorf("failed to execute statement: %w\nStatement: %s", err, stmt[:min(100, len(stmt))])
		}
	}

	// Record migration
	_, err = tx.Exec("INSERT INTO schema_migrations (version) VALUES ($1)", version)
	if err != nil {
		return fmt.Errorf("failed to record migration: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	log.Printf("✅ Applied %s", filename)
	return nil
}

func splitStatements(content string) []string {
	var statements []string
	var current strings.Builder
	inString := false
	stringChar := byte('"')

	for i := 0; i < len(content); i++ {
		c := content[i]

		if !inString && (c == '"' || c == '\'') {
			inString = true
			stringChar = c
		} else if inString && c == stringChar {
			inString = false
		}

		if !inString && c == ';' {
			statements = append(statements, current.String())
			current.Reset()
		} else {
			current.WriteByte(c)
		}
	}

	// Add remaining content
	if current.Len() > 0 {
		statements = append(statements, current.String())
	}

	return statements
}

func verifyTables(db *sql.DB) error {
	tables := []string{
		"prevention_rules",
		"failure_registry",
		"file_reads",
		"task_attempts",
		"uncertainty_tracking",
		"production_code_tracking",
	}

	fmt.Println("\n=== Table Verification ===")
	allExist := true

	for _, table := range tables {
		var exists bool
		query := `
			SELECT EXISTS (
				SELECT 1 FROM information_schema.tables
				WHERE table_schema = 'public' AND table_name = $1
			)
		`
		err := db.QueryRow(query, table).Scan(&exists)
		if err != nil {
			return fmt.Errorf("failed to check table %s: %w", table, err)
		}

		if exists {
			// Count rows
			var count int
			err := db.QueryRow(fmt.Sprintf("SELECT COUNT(*) FROM %s", table)).Scan(&count)
			if err != nil {
				log.Printf("  ❌ %s: error counting rows: %v", table, err)
				allExist = false
			} else {
				log.Printf("  ✅ %s: exists (%d rows)", table, count)
			}
		} else {
			log.Printf("  ❌ %s: MISSING", table)
			allExist = false
		}
	}

	if !allExist {
		return fmt.Errorf("not all required tables exist")
	}

	return nil
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
