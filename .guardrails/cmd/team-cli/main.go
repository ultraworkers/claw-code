package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

var (
	// Version info - set by ldflags during build
	version   = "dev"
	buildTime = "unknown"
	gitCommit = "unknown"

	// CLI flags
	projectName string
	phase       string
	teamID      int
	roleName    string
	person      string
	output      string

	// Styles
	titleStyle   = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#7C3AED"))
	textStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("#E5E7EB"))
	successStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("#10B981"))
	errorStyle   = lipgloss.NewStyle().Foreground(lipgloss.Color("#EF4444"))
	warnStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("#F59E0B"))
	infoStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("#3B82F6"))
)

func main() {
	rootCmd := &cobra.Command{
		Use:   "team",
		Short: "Team Manager CLI - Manage standardized team layouts",
		Long: `Team Manager CLI

A command-line tool for managing standardized team structures across projects.
Integrates with the team_manager.py backend to provide team initialization,
role assignments, and status tracking.`,
		Version: fmt.Sprintf("%s (built: %s, commit: %s)", version, buildTime, gitCommit),
	}

	// Global flags
	rootCmd.PersistentFlags().StringVarP(&projectName, "project", "p", "", "Project name (required for most commands)")
	rootCmd.PersistentFlags().StringVarP(&output, "output", "o", "text", "Output format: text, json")

	// Add subcommands
	rootCmd.AddCommand(initCmd())
	rootCmd.AddCommand(listCmd())
	rootCmd.AddCommand(assignCmd())
	rootCmd.AddCommand(unassignCmd())
	rootCmd.AddCommand(startCmd())
	rootCmd.AddCommand(completeCmd())
	rootCmd.AddCommand(statusCmd())
	rootCmd.AddCommand(validateCmd())
	rootCmd.AddCommand(phaseGateCmd())
	rootCmd.AddCommand(queryCmd())
	rootCmd.AddCommand(reassignCmd())
	rootCmd.AddCommand(exportCmd())
	rootCmd.AddCommand(importCmd())
	rootCmd.AddCommand(backupCmd())
	rootCmd.AddCommand(restoreCmd())
	rootCmd.AddCommand(deleteCmd())
	rootCmd.AddCommand(auditCmd())
	rootCmd.AddCommand(historyCmd())
	rootCmd.AddCommand(templateCmd())
	rootCmd.AddCommand(healthCmd())

	if err := rootCmd.Execute(); err != nil {
		log.Error(err)
		os.Exit(1)
	}
}

// getTeamManagerPath returns the path to the team_manager.py script
func getTeamManagerPath() string {
	// Check if TEAM_MANAGER_PATH env var is set
	if path := os.Getenv("TEAM_MANAGER_PATH"); path != "" {
		return path
	}

	// Try to find relative to executable
	exe, err := os.Executable()
	if err == nil {
		dir := filepath.Dir(exe)
		// Try multiple relative paths
		candidates := []string{
			filepath.Join(dir, "..", "..", "..", "scripts", "team_manager.py"),
			filepath.Join(dir, "..", "..", "scripts", "team_manager.py"),
			filepath.Join(dir, "..", "scripts", "team_manager.py"),
			filepath.Join(dir, "scripts", "team_manager.py"),
		}
		for _, candidate := range candidates {
			if _, err := os.Stat(candidate); err == nil {
				return candidate
			}
		}
	}

	// Default path
	return "scripts/team_manager.py"
}

// runTeamManager executes the team_manager.py script with the given arguments
// The Python script expects: --project PROJECT COMMAND [args...]
func runTeamManager(project string, command string, args ...string) ([]byte, error) {
	scriptPath := getTeamManagerPath()

	// Check if Python is available
	pythonCmd := "python3"
	if _, err := exec.LookPath("python3"); err != nil {
		pythonCmd = "python"
		if _, err := exec.LookPath("python"); err != nil {
			return nil, fmt.Errorf("Python not found. Please install Python 3")
		}
	}

	// Build command: script --project PROJECT command [args...]
	cmdArgs := []string{scriptPath}
	// Always include --project since Python script requires it
	if project == "" {
		project = "_cli_health_check_"
	}
	cmdArgs = append(cmdArgs, "--project", project)
	cmdArgs = append(cmdArgs, command)
	cmdArgs = append(cmdArgs, args...)

	cmd := exec.Command(pythonCmd, cmdArgs...)
	cmd.Stderr = os.Stderr

	output, err := cmd.Output()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return nil, fmt.Errorf("team_manager.py failed: %s", string(exitErr.Stderr))
		}
		return nil, fmt.Errorf("failed to run team_manager.py: %w", err)
	}

	return output, nil
}

// initCmd creates the init command
func initCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "init [project-name]",
		Short: "Initialize a new project with team structure",
		Long:  `Initialize a new project with the standardized 12-team structure.`,
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			project := args[0]

			if output == "json" {
				result, err := runTeamManager(project, "init", "--format", "json")
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Initializing Team Structure"))
			fmt.Printf("Project: %s\n\n", textStyle.Render(project))

			result, err := runTeamManager(project, "init")
			if err != nil {
				return fmt.Errorf("%s %v", errorStyle.Render("Failed to initialize project:"), err)
			}

			fmt.Println(string(result))
			fmt.Println()
			fmt.Println(successStyle.Render("✓ Project initialized successfully"))
			fmt.Println(infoStyle.Render(fmt.Sprintf("Config: .teams/%s.json", project)))

			return nil
		},
	}
}

// listCmd creates the list command
func listCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "list",
		Short: "List all teams for a project",
		Long:  `List all teams and their role assignments for a project.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			listExtraArgs := []string{}
			if phase != "" {
				listExtraArgs = append(listExtraArgs, "--phase", phase)
			}

			if output == "json" {
				listExtraArgs = append(listExtraArgs, "--format", "json")
				result, err := runTeamManager(projectName, "list", listExtraArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Team List"))
			fmt.Printf("Project: %s\n\n", textStyle.Render(projectName))

			result, err := runTeamManager(projectName, "list", listExtraArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVar(&phase, "phase", "", "Filter by phase (e.g., 'Phase 1')")
	return cmd
}

// assignCmd creates the assign command
func assignCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "assign",
		Short: "Assign a person to a role",
		Long:  `Assign a person to a specific role within a team.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if teamID == 0 {
				return fmt.Errorf("--team flag is required")
			}
			if roleName == "" {
				return fmt.Errorf("--role flag is required")
			}
			if person == "" {
				return fmt.Errorf("--person flag is required")
			}

			assignArgs := []string{
				"--team", fmt.Sprintf("%d", teamID),
				"--role", roleName,
				"--person", person,
			}

			if output == "json" {
				assignArgs = append(assignArgs, "--format", "json")
				result, err := runTeamManager(projectName, "assign", assignArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Assigning Role"))
			fmt.Printf("Project: %s\n", textStyle.Render(projectName))
			fmt.Printf("Team: %s\n", textStyle.Render(fmt.Sprintf("Team %d", teamID)))
			fmt.Printf("Role: %s\n", textStyle.Render(roleName))
			fmt.Printf("Person: %s\n\n", textStyle.Render(person))

			result, err := runTeamManager(projectName, "assign", assignArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVarP(&teamID, "team", "t", 0, "Team ID (1-12)")
	cmd.Flags().StringVarP(&roleName, "role", "r", "", "Role name to assign")
	cmd.Flags().StringVar(&person, "person", "", "Person to assign")

	cmd.MarkFlagRequired("team")
	cmd.MarkFlagRequired("role")
	cmd.MarkFlagRequired("person")

	return cmd
}

// unassignCmd creates the unassign command
func unassignCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "unassign",
		Short: "Remove a person from a role",
		Long:  `Remove a person from a specific role within a team.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if teamID == 0 {
				return fmt.Errorf("--team flag is required")
			}
			if roleName == "" {
				return fmt.Errorf("--role flag is required")
			}

			unassignArgs := []string{
				"--team", fmt.Sprintf("%d", teamID),
				"--role", roleName,
			}

			if output == "json" {
				unassignArgs = append(unassignArgs, "--format", "json")
				result, err := runTeamManager(projectName, "unassign", unassignArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Unassigning Role"))
			fmt.Printf("Project: %s\n", textStyle.Render(projectName))
			fmt.Printf("Team: %s\n", textStyle.Render(fmt.Sprintf("Team %d", teamID)))
			fmt.Printf("Role: %s\n\n", textStyle.Render(roleName))

			result, err := runTeamManager(projectName, "unassign", unassignArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVarP(&teamID, "team", "t", 0, "Team ID (1-12)")
	cmd.Flags().StringVarP(&roleName, "role", "r", "", "Role name to unassign")

	cmd.MarkFlagRequired("team")
	cmd.MarkFlagRequired("role")

	return cmd
}

// startCmd creates the start command
func startCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "start",
		Short: "Start a team",
		Long:  `Mark a team as started/in-progress.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if teamID == 0 {
				return fmt.Errorf("--team flag is required")
			}

			startArgs := []string{"--team", fmt.Sprintf("%d", teamID)}

			if output == "json" {
				startArgs = append(startArgs, "--format", "json")
				result, err := runTeamManager(projectName, "start", startArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Starting Team"))
			fmt.Printf("Project: %s\n", textStyle.Render(projectName))
			fmt.Printf("Team: %s\n\n", textStyle.Render(fmt.Sprintf("Team %d", teamID)))

			result, err := runTeamManager(projectName, "start", startArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVarP(&teamID, "team", "t", 0, "Team ID (1-12)")
	cmd.MarkFlagRequired("team")

	return cmd
}

// completeCmd creates the complete command
func completeCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "complete",
		Short: "Complete a team",
		Long:  `Mark a team as completed.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if teamID == 0 {
				return fmt.Errorf("--team flag is required")
			}

			completeArgs := []string{"--team", fmt.Sprintf("%d", teamID)}

			if output == "json" {
				completeArgs = append(completeArgs, "--format", "json")
				result, err := runTeamManager(projectName, "complete", completeArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Completing Team"))
			fmt.Printf("Project: %s\n", textStyle.Render(projectName))
			fmt.Printf("Team: %s\n\n", textStyle.Render(fmt.Sprintf("Team %d", teamID)))

			result, err := runTeamManager(projectName, "complete", completeArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVarP(&teamID, "team", "t", 0, "Team ID (1-12)")
	cmd.MarkFlagRequired("team")

	return cmd
}

// statusCmd creates the status command
func statusCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "status",
		Short: "Show project or phase status",
		Long:  `Display the current status of teams in a project or phase.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			statusArgs := []string{}
			if phase != "" {
				statusArgs = append(statusArgs, "--phase", phase)
			}

			if output == "json" {
				statusArgs = append(statusArgs, "--format", "json")
				result, err := runTeamManager(projectName, "status", statusArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Project Status"))
			fmt.Printf("Project: %s\n\n", textStyle.Render(projectName))

			result, err := runTeamManager(projectName, "status", statusArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVar(&phase, "phase", "", "Show status for specific phase")
	return cmd
}

// validateCmd creates the validate command
func validateCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "validate",
		Short: "Validate team sizes meet requirements",
		Long:  `Validate that all teams have 4-6 members as required by TEAM-007.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			if output == "json" {
				result, err := runTeamManager(projectName, "validate-size", "--format", "json")
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Team Size Validation"))
			fmt.Printf("Project: %s\n\n", textStyle.Render(projectName))

			result, err := runTeamManager(projectName, "validate-size")
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}
}

// phaseGateCmd creates the phase-gate command
func phaseGateCmd() *cobra.Command {
	var fromPhase, toPhase int

	cmd := &cobra.Command{
		Use:   "phase-gate",
		Short: "Check phase gate requirements",
		Long:  `Check if requirements are met for transitioning between phases.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if fromPhase == 0 || toPhase == 0 {
				return fmt.Errorf("--from and --to flags are required")
			}

			phaseGateArgs := []string{
				"--from", fmt.Sprintf("%d", fromPhase),
				"--to", fmt.Sprintf("%d", toPhase),
			}

			if output == "json" {
				phaseGateArgs = append(phaseGateArgs, "--format", "json")
				result, err := runTeamManager(projectName, "phase-gate-check", phaseGateArgs...)
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Phase Gate Check"))
			fmt.Printf("Project: %s\n", textStyle.Render(projectName))
			fmt.Printf("From: Phase %d → To: Phase %d\n\n", fromPhase, toPhase)

			result, err := runTeamManager(projectName, "phase-gate-check", phaseGateArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVar(&fromPhase, "from", 0, "Source phase number (1-4)")
	cmd.Flags().IntVar(&toPhase, "to", 0, "Target phase number (2-5)")

	cmd.MarkFlagRequired("from")
	cmd.MarkFlagRequired("to")

	return cmd
}

// queryCmd creates the query command
func queryCmd() *cobra.Command {
	var statusFilter, assigneeFilter, roleFilter string

	cmd := &cobra.Command{
		Use:   "query",
		Short: "Query teams with filters",
		Long:  `Query teams with filters for status, assignee, or role.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			queryArgs := []string{}
			if statusFilter != "" {
				queryArgs = append(queryArgs, "--status", statusFilter)
			}
			if assigneeFilter != "" {
				queryArgs = append(queryArgs, "--assignee", assigneeFilter)
			}
			if roleFilter != "" {
				queryArgs = append(queryArgs, "--role", roleFilter)
			}

			if output == "json" {
				queryArgs = append(queryArgs, "--format", "json")
			}

			result, err := runTeamManager(projectName, "query", queryArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVar(&statusFilter, "status", "", "Filter by status (not_started, active, completed, blocked)")
	cmd.Flags().StringVar(&assigneeFilter, "assignee", "", "Filter by assignee")
	cmd.Flags().StringVar(&roleFilter, "role", "", "Filter by role")

	return cmd
}

// reassignCmd creates the reassign command
func reassignCmd() *cobra.Command {
	var fromTeam, toTeam int
	var fromRole, toRole, personName string

	cmd := &cobra.Command{
		Use:   "reassign",
		Short: "Reassign person from one role to another",
		Long:  `Move a person from one role/team to another role/team.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			reassignArgs := []string{
				"--from-team", fmt.Sprintf("%d", fromTeam),
				"--from-role", fromRole,
				"--to-team", fmt.Sprintf("%d", toTeam),
				"--to-role", toRole,
				"--person", personName,
			}

			if output == "json" {
				reassignArgs = append(reassignArgs, "--format", "json")
			}

			result, err := runTeamManager(projectName, "reassign", reassignArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVar(&fromTeam, "from-team", 0, "Source team ID")
	cmd.Flags().StringVar(&fromRole, "from-role", "", "Source role")
	cmd.Flags().IntVar(&toTeam, "to-team", 0, "Target team ID")
	cmd.Flags().StringVar(&toRole, "to-role", "", "Target role")
	cmd.Flags().StringVar(&personName, "person", "", "Person to reassign")

	cmd.MarkFlagRequired("from-team")
	cmd.MarkFlagRequired("from-role")
	cmd.MarkFlagRequired("to-team")
	cmd.MarkFlagRequired("to-role")
	cmd.MarkFlagRequired("person")

	return cmd
}

// auditCmd creates the audit command
func auditCmd() *cobra.Command {
	var limit int

	cmd := &cobra.Command{
		Use:   "audit",
		Short: "Query audit log",
		Long:  `Query the audit log for project changes.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			auditArgs := []string{}
			if limit > 0 {
				auditArgs = append(auditArgs, "--limit", fmt.Sprintf("%d", limit))
			}

			if output == "json" {
				auditArgs = append(auditArgs, "--format", "json")
			}

			result, err := runTeamManager(projectName, "audit", auditArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVar(&limit, "limit", 50, "Number of entries to show")

	return cmd
}

// historyCmd creates the history command
func historyCmd() *cobra.Command {
	var startDate, endDate string

	cmd := &cobra.Command{
		Use:   "history",
		Short: "Show team history",
		Long:  `Show history for a specific team.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if teamID == 0 {
				return fmt.Errorf("--team flag is required")
			}

			historyArgs := []string{
				"--team", fmt.Sprintf("%d", teamID),
			}
			if startDate != "" {
				historyArgs = append(historyArgs, "--start-date", startDate)
			}
			if endDate != "" {
				historyArgs = append(historyArgs, "--end-date", endDate)
			}

			if output == "json" {
				historyArgs = append(historyArgs, "--format", "json")
			}

			result, err := runTeamManager(projectName, "team-history", historyArgs...)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVarP(&teamID, "team", "t", 0, "Team ID")
	cmd.Flags().StringVar(&startDate, "start-date", "", "Start date (ISO format)")
	cmd.Flags().StringVar(&endDate, "end-date", "", "End date (ISO format)")

	cmd.MarkFlagRequired("team")

	return cmd
}

// templateCmd creates the template command
func templateCmd() *cobra.Command {
	var format, outputFile string

	cmd := &cobra.Command{
		Use:   "template",
		Short: "Create template for bulk assignments",
		Long:  `Create a CSV or JSON template for bulk role assignments.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			var cmdName string
			var fileFlag string
			switch format {
			case "csv":
				cmdName = "template-csv"
				fileFlag = "--file"
			case "json":
				cmdName = "template-json"
				fileFlag = "--file"
			default:
				return fmt.Errorf("unsupported format: %s (use csv or json)", format)
			}

			result, err := runTeamManager(projectName, cmdName, fileFlag, outputFile)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVarP(&format, "format", "f", "csv", "Template format (csv or json)")
	cmd.Flags().StringVarP(&outputFile, "output", "o", "", "Output file path")

	return cmd
}

// healthCmd creates the health command
func healthCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "health",
		Short: "Check team manager health status",
		Long:  `Check the health status of the team manager.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			// Health check doesn't require a project
			result, err := runTeamManager("", "health")
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}
}

// exportCmd creates the export command
func exportCmd() *cobra.Command {
	var format string

	cmd := &cobra.Command{
		Use:   "export",
		Short: "Export project data",
		Long:  `Export team assignments and project data to a file.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			var result []byte
			var err error

			switch format {
			case "json":
				result, err = runTeamManager(projectName, "export-json")
			case "csv":
				result, err = runTeamManager(projectName, "export-csv")
			default:
				return fmt.Errorf("unsupported export format: %s", format)
			}

			if err != nil {
				return err
			}

			// Write to stdout or file
			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVarP(&format, "format", "f", "json", "Export format (json, csv)")
	return cmd
}

// importCmd creates the import command
func importCmd() *cobra.Command {
	var filePath string
	var format string

	cmd := &cobra.Command{
		Use:   "import",
		Short: "Import project data",
		Long:  `Import team assignments from a file.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if filePath == "" {
				return fmt.Errorf("--file flag is required")
			}

			var result []byte
			var err error

			switch format {
			case "json":
				result, err = runTeamManager(projectName, "import-json", "--file", filePath)
			case "csv":
				result, err = runTeamManager(projectName, "import-csv", "--file", filePath)
			default:
				return fmt.Errorf("unsupported import format: %s", format)
			}

			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVarP(&filePath, "file", "f", "", "Path to import file")
	cmd.Flags().StringVar(&format, "format", "json", "Import format (json, csv)")

	cmd.MarkFlagRequired("file")

	return cmd
}

// backupCmd creates the backup command
func backupCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "backup",
		Short: "List available backups",
		Long:  `List all available backups for the project.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			if output == "json" {
				result, err := runTeamManager(projectName, "list-backups", "--format", "json")
				if err != nil {
					return err
				}
				fmt.Println(string(result))
				return nil
			}

			fmt.Println(titleStyle.Render("Available Backups"))
			fmt.Printf("Project: %s\n\n", textStyle.Render(projectName))

			result, err := runTeamManager(projectName, "list-backups")
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}
}

// restoreCmd creates the restore command
func restoreCmd() *cobra.Command {
	var backupFile string

	cmd := &cobra.Command{
		Use:   "restore",
		Short: "Restore from backup",
		Long:  `Restore project data from a backup file.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}
			if backupFile == "" {
				return fmt.Errorf("--backup flag is required")
			}

			fmt.Println(titleStyle.Render("Restoring from Backup"))
			fmt.Printf("Project: %s\n", textStyle.Render(projectName))
			fmt.Printf("Backup: %s\n\n", textStyle.Render(backupFile))

			result, err := runTeamManager(projectName, "restore", "--backup", backupFile)
			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().StringVarP(&backupFile, "backup", "b", "", "Path to backup file")
	cmd.MarkFlagRequired("backup")

	return cmd
}

// deleteCmd creates the delete command
func deleteCmd() *cobra.Command {
	var force bool

	cmd := &cobra.Command{
		Use:   "delete",
		Short: "Delete project or team",
		Long:  `Delete a project or a specific team from the project.`,
		RunE: func(cmd *cobra.Command, args []string) error {
			if projectName == "" {
				return fmt.Errorf("--project flag is required")
			}

			var result []byte
			var err error

			if teamID > 0 {
				// Delete specific team
				if !force {
					fmt.Printf("Delete team %d from project '%s'? [y/N]: ", teamID, projectName)
					var response string
					fmt.Scanln(&response)
					if !strings.EqualFold(response, "y") {
						fmt.Println("Aborted.")
						return nil
					}
				}

				fmt.Println(titleStyle.Render("Deleting Team"))
				fmt.Printf("Project: %s\n", textStyle.Render(projectName))
				fmt.Printf("Team: %d\n\n", teamID)

				result, err = runTeamManager(projectName, "delete-team", "--team", fmt.Sprintf("%d", teamID))
			} else {
				// Delete entire project
				if !force {
					fmt.Printf("Delete entire project '%s'? This cannot be undone! [y/N]: ", projectName)
					var response string
					fmt.Scanln(&response)
					if !strings.EqualFold(response, "y") {
						fmt.Println("Aborted.")
						return nil
					}
				}

				fmt.Println(titleStyle.Render("Deleting Project"))
				fmt.Printf("Project: %s\n\n", textStyle.Render(projectName))

				result, err = runTeamManager(projectName, "delete-project")
			}

			if err != nil {
				return err
			}

			fmt.Println(string(result))
			return nil
		},
	}

	cmd.Flags().IntVarP(&teamID, "team", "t", 0, "Team ID to delete (optional - if not specified, deletes entire project)")
	cmd.Flags().BoolVar(&force, "force", false, "Skip confirmation prompt")

	return cmd
}

// Helper function to pretty print JSON
func prettyPrintJSON(data []byte) error {
	var v interface{}
	if err := json.Unmarshal(data, &v); err != nil {
		return err
	}
	pretty, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return err
	}
	fmt.Println(string(pretty))
	return nil
}

// CheckPython returns an error if Python is not available
func CheckPython() error {
	pythonCmd := "python3"
	if runtime.GOOS == "windows" {
		pythonCmd = "python"
	}
	_, err := exec.LookPath(pythonCmd)
	if err != nil {
		_, err = exec.LookPath("python")
		if err != nil {
			return fmt.Errorf("Python is required but not found. Please install Python 3")
		}
	}
	return nil
}
