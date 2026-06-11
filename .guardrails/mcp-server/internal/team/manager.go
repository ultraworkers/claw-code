// Package team provides team management functionality for MCP server.
// This is a Go port of the Python team_manager.py core functionality.
package team

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// TeamMember represents a person assigned to a team role
type TeamMember struct {
	Person     string `json:"person"`
	Role       string `json:"role"`
	AssignedAt string `json:"assigned_at"`
}

// TeamAssignment represents a role assignment
type TeamAssignment struct {
	TeamID     int    `json:"team_id"`
	RoleName   string `json:"role_name"`
	Person     string `json:"person"`
	AssignedAt string `json:"assigned_at"`
}

// PhaseStatus represents the status of a phase
type PhaseStatus struct {
	Phase       string  `json:"phase"`
	TotalTeams  int     `json:"total_teams"`
	Completed   int     `json:"completed"`
	Active      int     `json:"active"`
	NotStarted  int     `json:"not_started"`
	ProgressPct float64 `json:"progress_pct"`
}

// TeamListItem represents a team in list output
type TeamListItem struct {
	ID             int    `json:"id"`
	Name           string `json:"name"`
	Phase          string `json:"phase"`
	Status         string `json:"status"`
	AssignedCount  int    `json:"assigned_count"`
	TotalRoles     int    `json:"total_roles"`
}

// Manager handles team operations with thread-safe access
type Manager struct {
	projectName string
	baseDir     string
	configPath  string
	lockPath    string
	teams       map[int]Team
	mu          sync.RWMutex
}

// ManagerOption configures the Manager
type ManagerOption func(*Manager)

// WithBaseDir sets the base directory for team files
func WithBaseDir(dir string) ManagerOption {
	return func(m *Manager) {
		m.baseDir = dir
	}
}

// WithTestMode enables test mode (no auth checks)
func WithTestMode(enabled bool) ManagerOption {
	return func(m *Manager) {
		// Test mode is handled by the caller
		_ = enabled
	}
}

// NewManager creates a new team manager
func NewManager(projectName string, opts ...ManagerOption) (*Manager, error) {
	if err := ValidateProjectName(projectName); err != nil {
		return nil, fmt.Errorf("invalid project name: %w", err)
	}

	m := &Manager{
		projectName: projectName,
		baseDir:     ".teams",
		teams:       make(map[int]Team),
	}

	for _, opt := range opts {
		opt(m)
	}

	// Validate and set paths
	configPath, err := ValidateProjectPath(projectName, m.baseDir)
	if err != nil {
		return nil, err
	}
	m.configPath = configPath
	m.lockPath = strings.TrimSuffix(configPath, ".json") + ".lock"

	return m, nil
}

// InitProject initializes a new project with all standard teams (deprecated, use InitializeProject)
func (m *Manager) InitProject() error {
	return m.InitializeProject()
}

// InitializeProject initializes a new project with all standard teams
func (m *Manager) InitializeProject() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Initialize with standard teams (deep copy)
	for id, team := range StandardTeams {
		m.teams[id] = copyTeam(team)
	}

	// Ensure directory exists
	if err := os.MkdirAll(filepath.Dir(m.configPath), 0755); err != nil {
		return fmt.Errorf("failed to create directory: %w", err)
	}

	if err := m.save(); err != nil {
		return fmt.Errorf("failed to save project: %w", err)
	}

	return nil
}

// Load loads team configuration from disk
func (m *Manager) Load() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if _, err := os.Stat(m.configPath); os.IsNotExist(err) {
		return fmt.Errorf("project not found: %s", m.projectName)
	}

	data, err := os.ReadFile(m.configPath)
	if err != nil {
		return fmt.Errorf("failed to read config: %w", err)
	}

	var projectData ProjectData
	if err := json.Unmarshal(data, &projectData); err != nil {
		return fmt.Errorf("failed to parse config: %w", err)
	}

	m.teams = make(map[int]Team)
	for _, team := range projectData.Teams {
		m.teams[team.ID] = team
	}

	return nil
}

// save persists team configuration to disk (must hold write lock)
func (m *Manager) save() error {
	projectData := ProjectData{
		ProjectName: m.projectName,
		Version:     "1.0.0",
		UpdatedAt:   time.Now(),
		Teams:       make([]Team, 0, len(m.teams)),
	}

	for _, team := range m.teams {
		projectData.Teams = append(projectData.Teams, team)
	}

	data, err := json.MarshalIndent(projectData, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	// Atomic write: write to temp file, then rename
	tempPath := m.configPath + ".tmp"
	if err := os.WriteFile(tempPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write temp file: %w", err)
	}

	if err := os.Rename(tempPath, m.configPath); err != nil {
		os.Remove(tempPath)
		return fmt.Errorf("failed to rename file: %w", err)
	}

	return nil
}

// AssignRole assigns a person to a role
func (m *Manager) AssignRole(teamID int, roleName, person string) error {
	if err := ValidateRoleName(roleName); err != nil {
		return err
	}
	if err := ValidatePersonName(person); err != nil {
		return err
	}

	m.mu.Lock()
	defer m.mu.Unlock()

	team, exists := m.teams[teamID]
	if !exists {
		return fmt.Errorf("team %d not found", teamID)
	}

	for i := range team.Roles {
		if team.Roles[i].Name == roleName {
			previous := team.Roles[i].AssignedTo
			team.Roles[i].AssignedTo = &person
			m.teams[teamID] = team

			if err := m.save(); err != nil {
				// Rollback on error
				team.Roles[i].AssignedTo = previous
				m.teams[teamID] = team
				return err
			}

			return nil
		}
	}

	return fmt.Errorf("role '%s' not found in team %d", roleName, teamID)
}

// UnassignRole removes assignment from a role
func (m *Manager) UnassignRole(teamID int, roleName string) error {
	if err := ValidateRoleName(roleName); err != nil {
		return err
	}

	m.mu.Lock()
	defer m.mu.Unlock()

	team, exists := m.teams[teamID]
	if !exists {
		return fmt.Errorf("team %d not found", teamID)
	}

	for i := range team.Roles {
		if team.Roles[i].Name == roleName {
			if team.Roles[i].AssignedTo == nil {
				return fmt.Errorf("role '%s' in %s is already unassigned", roleName, team.Name)
			}

			previous := team.Roles[i].AssignedTo
			team.Roles[i].AssignedTo = nil
			m.teams[teamID] = team

			if err := m.save(); err != nil {
				// Rollback on error
				team.Roles[i].AssignedTo = previous
				m.teams[teamID] = team
				return err
			}

			return nil
		}
	}

	return fmt.Errorf("role '%s' not found in team %d", roleName, teamID)
}

// StartTeam marks a team as active
func (m *Manager) StartTeam(teamID int, override bool, reason string) error {
	_ = reason // Reserved for future audit logging
	_ = override

	m.mu.Lock()
	defer m.mu.Unlock()

	team, exists := m.teams[teamID]
	if !exists {
		return fmt.Errorf("team %d not found", teamID)
	}

	if team.Status == TeamStatusActive {
		return fmt.Errorf("team %d is already active", teamID)
	}

	if team.Status == TeamStatusCompleted {
		return fmt.Errorf("team %d is already completed", teamID)
	}

	now := time.Now()
	previousStatus := team.Status
	team.Status = TeamStatusActive
	team.StartedAt = &now
	m.teams[teamID] = team

	if err := m.save(); err != nil {
		// Rollback
		team.Status = previousStatus
		team.StartedAt = nil
		m.teams[teamID] = team
		return err
	}

	return nil
}

// CompleteTeam marks a team as completed
func (m *Manager) CompleteTeam(teamID int) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	team, exists := m.teams[teamID]
	if !exists {
		return fmt.Errorf("team %d not found", teamID)
	}

	if team.Status == TeamStatusCompleted {
		return fmt.Errorf("team %d is already completed", teamID)
	}

	if team.Status != TeamStatusActive {
		return fmt.Errorf("team %d must be active before completing", teamID)
	}

	now := time.Now()
	previousStatus := team.Status
	team.Status = TeamStatusCompleted
	team.CompletedAt = &now
	m.teams[teamID] = team

	if err := m.save(); err != nil {
		// Rollback
		team.Status = previousStatus
		team.CompletedAt = nil
		m.teams[teamID] = team
		return err
	}

	return nil
}

// GetTeamStatus returns the current status of a team
func (m *Manager) GetTeamStatus(teamID int) (map[string]any, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	team, exists := m.teams[teamID]
	if !exists {
		return nil, fmt.Errorf("team %d not found", teamID)
	}

	assignedCount := 0
	assignedRoles := make([]map[string]string, 0)
	for _, role := range team.Roles {
		if role.AssignedTo != nil {
			assignedCount++
			assignedRoles = append(assignedRoles, map[string]string{
				"name":        role.Name,
				"assigned_to": *role.AssignedTo,
			})
		}
	}

	return map[string]any{
		"id":             team.ID,
		"name":           team.Name,
		"phase":          team.Phase,
		"description":    team.Description,
		"status":         team.Status,
		"started_at":     team.StartedAt,
		"completed_at":   team.CompletedAt,
		"assigned_count": assignedCount,
		"total_roles":    len(team.Roles),
		"assigned_roles": assignedRoles,
		"exit_criteria":  team.ExitCriteria,
	}, nil
}

// ListTeams returns all teams, optionally filtered by phase
func (m *Manager) ListTeams(phase string) ([]TeamListItem, error) {
	if phase != "" {
		if err := ValidatePhase(phase); err != nil {
			return nil, err
		}
	}

	m.mu.RLock()
	defer m.mu.RUnlock()

	items := make([]TeamListItem, 0, len(m.teams))
	for _, team := range m.teams {
		if phase != "" && team.Phase != phase {
			continue
		}

		assignedCount := 0
		for _, role := range team.Roles {
			if role.AssignedTo != nil {
				assignedCount++
			}
		}

		items = append(items, TeamListItem{
			ID:            team.ID,
			Name:          team.Name,
			Phase:         team.Phase,
			Status:        string(team.Status),
			AssignedCount: assignedCount,
			TotalRoles:    len(team.Roles),
		})
	}

	return items, nil
}

// GetPhaseStatus returns status summary for a phase
func (m *Manager) GetPhaseStatus(phase string) (PhaseStatus, error) {
	if err := ValidatePhase(phase); err != nil {
		return PhaseStatus{}, err
	}

	m.mu.RLock()
	defer m.mu.RUnlock()

	var total, completed, active int
	for _, team := range m.teams {
		if team.Phase != phase {
			continue
		}
		total++
		switch team.Status {
		case TeamStatusCompleted:
			completed++
		case TeamStatusActive:
			active++
		}
	}

	progressPct := 0.0
	if total > 0 {
		progressPct = float64(completed) / float64(total) * 100
	}

	return PhaseStatus{
		Phase:       phase,
		TotalTeams:  total,
		Completed:   completed,
		Active:      active,
		NotStarted:  total - completed - active,
		ProgressPct: progressPct,
	}, nil
}

// GetAllPhaseStatuses returns status for all phases
func (m *Manager) GetAllPhaseStatuses() ([]PhaseStatus, error) {
	phases := make(map[string]bool)
	for _, team := range m.teams {
		phases[team.Phase] = true
	}

	results := make([]PhaseStatus, 0, len(phases))
	for phase := range phases {
		status, err := m.GetPhaseStatus(phase)
		if err != nil {
			return nil, err
		}
		results = append(results, status)
	}

	return results, nil
}

// GetTeamAssignments returns all assignments for a team
func (m *Manager) GetTeamAssignments(teamID int) ([]TeamAssignment, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	team, exists := m.teams[teamID]
	if !exists {
		return nil, fmt.Errorf("team %d not found", teamID)
	}

	assignments := make([]TeamAssignment, 0)
	var assignedAt string
	if team.StartedAt != nil {
		assignedAt = team.StartedAt.Format(time.RFC3339)
	}

	for _, role := range team.Roles {
		if role.AssignedTo != nil {
			assignments = append(assignments, TeamAssignment{
				TeamID:     teamID,
				RoleName:   role.Name,
				Person:     *role.AssignedTo,
				AssignedAt: assignedAt,
			})
		}
	}

	return assignments, nil
}

// GetPersonAssignments returns all assignments for a person
func (m *Manager) GetPersonAssignments(person string) ([]TeamAssignment, error) {
	if err := ValidatePersonName(person); err != nil {
		return nil, err
	}

	m.mu.RLock()
	defer m.mu.RUnlock()

	assignments := make([]TeamAssignment, 0)
	for _, team := range m.teams {
		for _, role := range team.Roles {
			if role.AssignedTo != nil && *role.AssignedTo == person {
				var assignedAt string
				if team.StartedAt != nil {
					assignedAt = team.StartedAt.Format(time.RFC3339)
				}
				assignments = append(assignments, TeamAssignment{
					TeamID:     team.ID,
					RoleName:   role.Name,
					Person:     person,
					AssignedAt: assignedAt,
				})
			}
		}
	}

	return assignments, nil
}

// GetTeamByID returns a team by ID
func (m *Manager) GetTeamByID(teamID int) (Team, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	team, exists := m.teams[teamID]
	if !exists {
		return Team{}, fmt.Errorf("team %d not found", teamID)
	}

	return copyTeam(team), nil
}

// GetAllTeams returns all teams
func (m *Manager) GetAllTeams() []Team {
	m.mu.RLock()
	defer m.mu.RUnlock()

	teams := make([]Team, 0, len(m.teams))
	for _, team := range m.teams {
		teams = append(teams, copyTeam(team))
	}
	return teams
}

// GetTeamsByPhase returns teams filtered by phase
func (m *Manager) GetTeamsByPhase(phase string) []Team {
	m.mu.RLock()
	defer m.mu.RUnlock()

	teams := make([]Team, 0)
	for _, team := range m.teams {
		if team.Phase == phase {
			teams = append(teams, copyTeam(team))
		}
	}
	return teams
}

// GetProjectStatus returns overall project status
func (m *Manager) GetProjectStatus() map[string]interface{} {
	m.mu.RLock()
	defer m.mu.RUnlock()

	total := len(m.teams)
	completed := 0
	active := 0
	notStarted := 0

	for _, team := range m.teams {
		switch team.Status {
		case TeamStatusCompleted:
			completed++
		case TeamStatusActive:
			active++
		case TeamStatusNotStarted:
			notStarted++
		}
	}

	progressPct := 0.0
	if total > 0 {
		progressPct = float64(completed) / float64(total) * 100
	}

	return map[string]interface{}{
		"project":       m.projectName,
		"total_teams":   total,
		"completed":     completed,
		"active":        active,
		"not_started":   notStarted,
		"progress_pct":  progressPct,
	}
}

// QueryTeams queries teams with filters
func (m *Manager) QueryTeams(status, phase, assignee, roleName string) ([]Team, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	results := make([]Team, 0)
	for _, team := range m.teams {
		// Check status filter
		if status != "" && string(team.Status) != status {
			continue
		}

		// Check phase filter
		if phase != "" && team.Phase != phase {
			continue
		}

		// Check assignee filter
		if assignee != "" {
			found := false
			for _, role := range team.Roles {
				if role.AssignedTo != nil && *role.AssignedTo == assignee {
					found = true
					break
				}
			}
			if !found {
				continue
			}
		}

		// Check role name filter
		if roleName != "" {
			found := false
			for _, role := range team.Roles {
				if role.Name == roleName {
					found = true
					break
				}
			}
			if !found {
				continue
			}
		}

		results = append(results, copyTeam(team))
	}

	return results, nil
}

// GetConfigPath returns the configuration file path
func (m *Manager) GetConfigPath() string {
	return m.configPath
}

// GetProjectName returns the project name
func (m *Manager) GetProjectName() string {
	return m.projectName
}

// copyTeam creates a deep copy of a Team
func copyTeam(team Team) Team {
	roles := make([]Role, len(team.Roles))
	for i, r := range team.Roles {
		deliverables := make([]string, len(r.Deliverables))
		copy(deliverables, r.Deliverables)
		roles[i] = Role{
			Name:           r.Name,
			Responsibility: r.Responsibility,
			Deliverables:   deliverables,
			AssignedTo:     r.AssignedTo,
		}
	}

	exitCriteria := make([]string, len(team.ExitCriteria))
	copy(exitCriteria, team.ExitCriteria)

	return Team{
		ID:           team.ID,
		Name:         team.Name,
		Phase:        team.Phase,
		Description:  team.Description,
		Roles:        roles,
		ExitCriteria: exitCriteria,
		Status:       team.Status,
		StartedAt:    team.StartedAt,
		CompletedAt:  team.CompletedAt,
	}
}

// DeleteTeam removes a team from the project (marks as deleted)
func (m *Manager) DeleteTeam(teamID int, confirmed bool) error {
	if !confirmed {
		return fmt.Errorf("deletion requires confirmation")
	}

	m.mu.Lock()
	defer m.mu.Unlock()

	if _, exists := m.teams[teamID]; !exists {
		return fmt.Errorf("team %d not found", teamID)
	}

	// Remove the team from the map
	delete(m.teams, teamID)

	if err := m.save(); err != nil {
		return fmt.Errorf("failed to save after delete: %w", err)
	}

	return nil
}

// DeleteProject removes the entire project configuration file
func (m *Manager) DeleteProject(confirmed bool) error {
	if !confirmed {
		return fmt.Errorf("project deletion requires confirmation")
	}

	m.mu.Lock()
	defer m.mu.Unlock()

	// Remove the config file
	if err := os.Remove(m.configPath); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("failed to remove project file: %w", err)
	}

	// Clear the teams map
	m.teams = make(map[int]Team)

	return nil
}

// Health returns the health status of the team manager
func (m *Manager) Health() map[string]interface{} {
	m.mu.RLock()
	defer m.mu.RUnlock()

	totalTeams := len(m.teams)
	active := 0
	completed := 0
	notStarted := 0
	assignedRoles := 0

	for _, team := range m.teams {
		switch team.Status {
		case TeamStatusActive:
			active++
		case TeamStatusCompleted:
			completed++
		case TeamStatusNotStarted:
			notStarted++
		}
		for _, role := range team.Roles {
			if role.AssignedTo != nil && *role.AssignedTo != "" {
				assignedRoles++
			}
		}
	}

	return map[string]interface{}{
		"status":         "healthy",
		"project":        m.projectName,
		"total_teams":    totalTeams,
		"active":         active,
		"completed":      completed,
		"not_started":    notStarted,
		"assigned_roles": assignedRoles,
		"config_path":    m.configPath,
	}
}
