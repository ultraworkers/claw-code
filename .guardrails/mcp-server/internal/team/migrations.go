package team

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// MigrationFunc is a function that migrates data from one version to another.
type MigrationFunc func(data map[string]interface{}) (map[string]interface{}, error)

// MigrationManager manages data migrations between versions.
// Provides automatic migration detection and execution for
// project data files when schema versions change.
type MigrationManager struct {
	projectName    string
	configPath     string
	currentVersion string
	migrations     map[string]MigrationFunc
}

// CurrentVersion is the target version for migrations.
const CurrentVersion = "1.0.0"

// NewMigrationManager creates a new migration manager for a project.
func NewMigrationManager(projectName string) *MigrationManager {
	configPath := filepath.Join(".teams", projectName+".json")

	mm := &MigrationManager{
		projectName:    projectName,
		configPath:     configPath,
		currentVersion: CurrentVersion,
		migrations:     make(map[string]MigrationFunc),
	}

	mm.registerMigrations()
	return mm
}

// registerMigrations registers available migration scripts.
// Each migration should be a callable that takes data map and returns migrated data.
func (mm *MigrationManager) registerMigrations() {
	// Register migrations from version -> version
	// Example: mm.migrations["0.9.0"] = mm.migrateV090ToV100
}

// GetDataVersion extracts version from data map.
func (mm *MigrationManager) GetDataVersion(data map[string]interface{}) string {
	if v, ok := data["version"].(string); ok {
		return v
	}
	return CurrentVersion
}

// NeedsMigration checks if data needs migration.
func (mm *MigrationManager) NeedsMigration(data map[string]interface{}) bool {
	dataVersion := mm.GetDataVersion(data)
	return mm.versionCompare(dataVersion, mm.currentVersion) < 0
}

// versionCompare compares two version strings. Returns -1, 0, or 1.
func (mm *MigrationManager) versionCompare(v1, v2 string) int {
	parseVersion := func(v string) []int {
		parts := strings.Split(v, ".")
		result := make([]int, 0, len(parts))
		for _, p := range parts {
			if n, err := strconv.Atoi(p); err == nil {
				result = append(result, n)
			}
		}
		return result
	}

	p1 := parseVersion(v1)
	p2 := parseVersion(v2)

	maxLen := len(p1)
	if len(p2) > maxLen {
		maxLen = len(p2)
	}

	for i := 0; i < maxLen; i++ {
		n1 := 0
		if i < len(p1) {
			n1 = p1[i]
		}
		n2 := 0
		if i < len(p2) {
			n2 = p2[i]
		}
		if n1 < n2 {
			return -1
		} else if n1 > n2 {
			return 1
		}
	}
	return 0
}

// Migrate migrates data to current version.
// Returns the migrated data map with updated version.
func (mm *MigrationManager) Migrate(data map[string]interface{}) (map[string]interface{}, error) {
	originalVersion := mm.GetDataVersion(data)
	currentVersion := originalVersion

	if !mm.NeedsMigration(data) {
		return data, nil
	}

	fmt.Printf("🔄 Migrating project '%s' from v%s to v%s\n", mm.projectName, originalVersion, mm.currentVersion)

	// Build sorted list of migration versions
	type migrationItem struct {
		version string
		fn      MigrationFunc
	}
	var items []migrationItem
	for v, fn := range mm.migrations {
		items = append(items, migrationItem{version: v, fn: fn})
	}

	// Sort by version comparison (ascending toward target)
	for i := 0; i < len(items); i++ {
		for j := i + 1; j < len(items); j++ {
			if mm.versionCompare(items[i].version, items[j].version) > 0 {
				items[i], items[j] = items[j], items[i]
			}
		}
	}

	// Apply migrations in order
	for _, item := range items {
		if mm.versionCompare(currentVersion, item.version) < 0 {
			fmt.Printf("   Applying migration to v%s...\n", item.version)
			var err error
			data, err = item.fn(data)
			if err != nil {
				fmt.Printf("   ❌ Migration failed: %v\n", err)
				return nil, err
			}
			data["version"] = item.version
			currentVersion = item.version
		}
	}

	// Update to final version
	data["version"] = mm.currentVersion
	data["migrated_from"] = originalVersion
	data["migrated_at"] = time.Now().Format(time.RFC3339)

	fmt.Printf("✅ Migration complete: v%s -> v%s\n", originalVersion, mm.currentVersion)
	return data, nil
}

// GetMigrationStatus returns migration status for project.
func (mm *MigrationManager) GetMigrationStatus() map[string]interface{} {
	_, err := os.Stat(mm.configPath)
	if os.IsNotExist(err) {
		return map[string]interface{}{
			"status":          "not_found",
			"current_version": nil,
		}
	}

	file, err := os.Open(mm.configPath)
	if err != nil {
		return map[string]interface{}{
			"status": "error",
			"error":  err.Error(),
		}
	}
	defer file.Close()

	var data map[string]interface{}
	if err := json.NewDecoder(file).Decode(&data); err != nil {
		return map[string]interface{}{
			"status": "error",
			"error":  err.Error(),
		}
	}

	dataVersion := mm.GetDataVersion(data)
	needsMig := mm.versionCompare(dataVersion, mm.currentVersion) < 0

	status := "current"
	if needsMig {
		status = "needs_migration"
	}

	return map[string]interface{}{
		"status":           status,
		"current_version":  dataVersion,
		"target_version":   mm.currentVersion,
		"project":          mm.projectName,
	}
}

// RegisterMigration adds a migration function for a target version.
func (mm *MigrationManager) RegisterMigration(version string, fn MigrationFunc) {
	mm.migrations[version] = fn
}
