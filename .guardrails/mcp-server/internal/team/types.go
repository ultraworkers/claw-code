package team

import (
	"time"
)

// Role represents a standard team role
type Role struct {
	Name           string   `json:"name"`
	Responsibility string   `json:"responsibility"`
	Deliverables   []string `json:"deliverables"`
	AssignedTo     *string  `json:"assigned_to,omitempty"`
}

// TeamStatus represents the status of a team
type TeamStatus string

const (
	TeamStatusNotStarted TeamStatus = "not_started"
	TeamStatusActive     TeamStatus = "active"
	TeamStatusCompleted  TeamStatus = "completed"
	TeamStatusBlocked    TeamStatus = "blocked"
)

// Team represents a standard team definition
type Team struct {
	ID           int          `json:"id"`
	Name         string       `json:"name"`
	Phase        string       `json:"phase"`
	Description  string       `json:"description"`
	Roles        []Role       `json:"roles"`
	ExitCriteria []string     `json:"exit_criteria"`
	Status       TeamStatus   `json:"status"`
	StartedAt    *time.Time   `json:"started_at,omitempty"`
	CompletedAt  *time.Time   `json:"completed_at,omitempty"`
}

// ProjectData represents the stored project configuration
type ProjectData struct {
	ProjectName string    `json:"project_name"`
	Version     string    `json:"version"`
	UpdatedAt   time.Time `json:"updated_at"`
	Teams       []Team    `json:"teams"`
}

// PhaseGate represents a phase transition gate
type PhaseGate struct {
	Name         string `json:"name"`
	FromPhase    int    `json:"from_phase"`
	ToPhase      int    `json:"to_phase"`
	RequiredTeams []int `json:"required_teams"`
	ApprovalTeam int   `json:"approval_team"`
	Deliverables []string `json:"deliverables"`
}

// AgentTypeMapping maps agent types to teams and roles
type AgentTypeMapping struct {
	AgentType    string   `json:"agent_type"`
	TeamID       int      `json:"team_id"`
	Phase        string   `json:"phase"`
	Roles        []string `json:"roles"`
}

// Standard team definitions
var StandardTeams = map[int]Team{
	// Phase 1: Strategy, Governance & Planning
	1: {
		ID:          1,
		Name:        "Business & Product Strategy",
		Phase:       "Phase 1: Strategy, Governance & Planning",
		Description: "The 'Why' - Business case and product strategy",
		Roles: []Role{
			{Name: "Business Relationship Manager", Responsibility: "Connects IT to C-suite", Deliverables: []string{"Strategic alignment docs", "Executive briefings"}},
			{Name: "Lead Product Manager", Responsibility: "Owns long-term roadmap", Deliverables: []string{"Product roadmap", "OKRs", "Feature prioritization"}},
			{Name: "Business Systems Analyst", Responsibility: "Translates business to technical", Deliverables: []string{"Requirements specs", "User stories", "Acceptance criteria"}},
			{Name: "Financial Controller (FinOps)", Responsibility: "Approves budget and cloud spend", Deliverables: []string{"Budget forecasts", "Cost projections", "Spend reports"}},
		},
		ExitCriteria: []string{"Business case approved", "Budget allocated", "Roadmap defined", "Success metrics established"},
		Status:       TeamStatusNotStarted,
	},
	2: {
		ID:          2,
		Name:        "Enterprise Architecture",
		Phase:       "Phase 1: Strategy, Governance & Planning",
		Description: "The 'Standards' - Technology vision and standards",
		Roles: []Role{
			{Name: "Chief Architect", Responsibility: "Sets 5-year tech vision", Deliverables: []string{"Architecture vision", "Tech radar", "Strategic plans"}},
			{Name: "Domain Architect", Responsibility: "Specialized stack expertise", Deliverables: []string{"Domain-specific patterns", "Best practices guides"}},
			{Name: "Solution Architect", Responsibility: "Maps projects to standards", Deliverables: []string{"Solution designs", "Architecture decision records"}},
			{Name: "Standards Lead", Responsibility: "Manages Approved Tech List", Deliverables: []string{"Technology standards", "Evaluation criteria", "Approved list"}},
		},
		ExitCriteria: []string{"Architecture approved", "Technology choices validated", "Standards compliance verified"},
		Status:       TeamStatusNotStarted,
	},
	3: {
		ID:          3,
		Name:        "GRC (Governance, Risk, & Compliance)",
		Phase:       "Phase 1: Strategy, Governance & Planning",
		Description: "Compliance and risk management",
		Roles: []Role{
			{Name: "Compliance Officer", Responsibility: "SOX/HIPAA/GDPR adherence", Deliverables: []string{"Compliance checklists", "Audit reports"}},
			{Name: "Internal Auditor", Responsibility: "Pre-production mock audits", Deliverables: []string{"Audit findings", "Remediation plans"}},
			{Name: "Privacy Engineer", Responsibility: "Data masking and PII", Deliverables: []string{"Privacy impact assessments", "Data flow diagrams"}},
			{Name: "Policy Manager", Responsibility: "Maintains SOPs", Deliverables: []string{"Standard operating procedures", "Policy updates"}},
		},
		ExitCriteria: []string{"Compliance review passed", "Risk assessment complete", "Privacy requirements met", "Policies acknowledged"},
		Status:       TeamStatusNotStarted,
	},
	// Phase 2: Platform & Foundation
	4: {
		ID:          4,
		Name:        "Infrastructure & Cloud Ops",
		Phase:       "Phase 2: Platform & Foundation",
		Description: "Cloud infrastructure and networking",
		Roles: []Role{
			{Name: "Cloud Architect", Responsibility: "VPC and network design", Deliverables: []string{"Network diagrams", "Security groups", "Routing tables"}},
			{Name: "IaC Engineer", Responsibility: "Provisions the 'metal'", Deliverables: []string{"Terraform modules", "Ansible playbooks", "Infrastructure code"}},
			{Name: "Network Security Engineer", Responsibility: "Firewalls, VPNs, Direct Connect", Deliverables: []string{"Security rules", "Network policies", "Access controls"}},
			{Name: "Storage Engineer", Responsibility: "S3/SAN management", Deliverables: []string{"Storage policies", "Backup strategies", "Archival rules"}},
		},
		ExitCriteria: []string{"Infrastructure provisioned", "Network connectivity verified", "Security rules applied", "Monitoring enabled"},
		Status:       TeamStatusNotStarted,
	},
	5: {
		ID:          5,
		Name:        "Platform Engineering",
		Phase:       "Phase 2: Platform & Foundation",
		Description: "The 'Internal Tools' - Developer experience platform",
		Roles: []Role{
			{Name: "Platform Product Manager", Responsibility: "Developer experience as product", Deliverables: []string{"Platform roadmap", "DX metrics", "Adoption reports"}},
			{Name: "CI/CD Architect", Responsibility: "Golden pipelines", Deliverables: []string{"Pipeline templates", "Build configs", "Deployment strategies"}},
			{Name: "Kubernetes Administrator", Responsibility: "Cluster management", Deliverables: []string{"Cluster configs", "Resource quotas", "Ingress rules"}},
			{Name: "Developer Advocate", Responsibility: "Dev squad adoption", Deliverables: []string{"Onboarding guides", "Training materials", "Feedback loops"}},
		},
		ExitCriteria: []string{"Platform services ready", "CI/CD pipelines functional", "Developer onboarding complete"},
		Status:       TeamStatusNotStarted,
	},
	6: {
		ID:          6,
		Name:        "Data Governance & Analytics",
		Phase:       "Phase 2: Platform & Foundation",
		Description: "Enterprise data management",
		Roles: []Role{
			{Name: "Data Architect", Responsibility: "Enterprise data model", Deliverables: []string{"Data models", "Schema designs", "Lineage documentation"}},
			{Name: "DBA", Responsibility: "Production database performance", Deliverables: []string{"Query optimization", "Index tuning", "Backup verification"}},
			{Name: "Data Privacy Officer", Responsibility: "Retention and deletion rules", Deliverables: []string{"Data retention policies", "Deletion workflows"}},
			{Name: "ETL Developer", Responsibility: "Data flow management", Deliverables: []string{"ETL pipelines", "Data quality checks", "Transformation logic"}},
		},
		ExitCriteria: []string{"Data models defined", "Pipelines operational", "Privacy controls implemented"},
		Status:       TeamStatusNotStarted,
	},
	// Phase 3: The Build Squads
	7: {
		ID:          7,
		Name:        "Core Feature Squad",
		Phase:       "Phase 3: The Build Squads",
		Description: "The 'Devs' - Feature implementation",
		Roles: []Role{
			{Name: "Technical Lead", Responsibility: "Final word on implementation", Deliverables: []string{"Code reviews", "Architecture decisions", "Technical guidance"}},
			{Name: "Senior Backend Engineer", Responsibility: "Logic, APIs, microservices", Deliverables: []string{"Backend services", "API endpoints", "Business logic"}},
			{Name: "Senior Frontend Engineer", Responsibility: "Design system, state management", Deliverables: []string{"UI components", "Frontend architecture", "State logic"}},
			{Name: "Accessibility (A11y) Expert", Responsibility: "WCAG compliance", Deliverables: []string{"A11y audits", "Remediation plans", "Testing reports"}},
			{Name: "Technical Writer", Responsibility: "Internal/external docs", Deliverables: []string{"API docs", "User guides", "Runbooks"}},
		},
		ExitCriteria: []string{"Features implemented", "Code reviewed and approved", "Documentation complete", "A11y requirements met"},
		Status:       TeamStatusNotStarted,
	},
	8: {
		ID:          8,
		Name:        "Middleware & Integration",
		Phase:       "Phase 3: The Build Squads",
		Description: "APIs and system integrations",
		Roles: []Role{
			{Name: "API Product Manager", Responsibility: "API lifecycle and versioning", Deliverables: []string{"API specs", "Versioning strategy", "Deprecation plans"}},
			{Name: "Integration Engineer", Responsibility: "SAP/Oracle/Mainframe connections", Deliverables: []string{"Integration specs", "Data mappings", "Error handling"}},
			{Name: "Messaging Engineer", Responsibility: "Kafka/RabbitMQ management", Deliverables: []string{"Topic design", "Message schemas", "Consumer groups"}},
			{Name: "IAM Specialist", Responsibility: "Okta/AD integration", Deliverables: []string{"Auth flows", "Permission models", "Access policies"}},
		},
		ExitCriteria: []string{"APIs documented and tested", "Integrations verified", "Auth flows functional"},
		Status:       TeamStatusNotStarted,
	},
	// Phase 4: Validation & Hardening
	9: {
		ID:          9,
		Name:        "Cybersecurity (AppSec)",
		Phase:       "Phase 4: Validation & Hardening",
		Description: "Application security",
		Roles: []Role{
			{Name: "Security Architect", Responsibility: "Threat model review", Deliverables: []string{"Threat models", "Security architecture", "Risk assessments"}},
			{Name: "Vulnerability Researcher", Responsibility: "SAST/DAST/SCA scanners", Deliverables: []string{"Scan reports", "Vulnerability triage", "Fix verification"}},
			{Name: "Penetration Tester", Responsibility: "Manual security testing", Deliverables: []string{"Pen test reports", "Exploit verification", "Remediation"}},
			{Name: "DevSecOps Engineer", Responsibility: "Security in CI/CD", Deliverables: []string{"Security gates", "Pipeline integration", "Compliance checks"}},
		},
		ExitCriteria: []string{"Security review passed", "Vulnerabilities remediated or accepted", "Pen testing complete", "Security gates passing"},
		Status:       TeamStatusNotStarted,
	},
	10: {
		ID:          10,
		Name:        "Quality Engineering (SDET)",
		Phase:       "Phase 4: Validation & Hardening",
		Description: "Testing and quality assurance",
		Roles: []Role{
			{Name: "QA Architect", Responsibility: "Global testing strategy", Deliverables: []string{"Test strategy", "Test plans", "Coverage reports"}},
			{Name: "SDET", Responsibility: "Automated test code", Deliverables: []string{"Test automation", "Framework maintenance", "CI integration"}},
			{Name: "Performance/Load Engineer", Responsibility: "Scale testing", Deliverables: []string{"Load test scripts", "Performance baselines", "Capacity reports"}},
			{Name: "Manual QA / UAT Coordinator", Responsibility: "User acceptance testing", Deliverables: []string{"Test cases", "UAT coordination", "Sign-off reports"}},
		},
		ExitCriteria: []string{"Test coverage requirements met", "Performance benchmarks achieved", "UAT sign-off obtained"},
		Status:       TeamStatusNotStarted,
	},
	// Phase 5: Delivery & Sustainment
	11: {
		ID:          11,
		Name:        "Site Reliability Engineering (SRE)",
		Phase:       "Phase 5: Delivery & Sustainment",
		Description: "Reliability and observability",
		Roles: []Role{
			{Name: "SRE Lead", Responsibility: "Error budget and uptime SLA", Deliverables: []string{"SLOs", "Error budgets", "Reliability reports"}},
			{Name: "Observability Engineer", Responsibility: "Monitoring and logging", Deliverables: []string{"Dashboards", "Alerts", "Log aggregation", "Traces"}},
			{Name: "Chaos Engineer", Responsibility: "Resiliency testing", Deliverables: []string{"Chaos experiments", "Failure scenarios", "Recovery tests"}},
			{Name: "Incident Manager", Responsibility: "War room leadership", Deliverables: []string{"Incident response", "Post-mortems", "Runbook updates"}},
		},
		ExitCriteria: []string{"Monitoring in place", "Alerts configured", "Runbooks complete", "Error budget healthy"},
		Status:       TeamStatusNotStarted,
	},
	12: {
		ID:          12,
		Name:        "IT Operations & Support (NOC)",
		Phase:       "Phase 5: Delivery & Sustainment",
		Description: "Production operations",
		Roles: []Role{
			{Name: "NOC Analyst", Responsibility: "24/7 monitoring", Deliverables: []string{"Monitoring dashboards", "Alert triage", "Incident tickets"}},
			{Name: "Change Manager", Responsibility: "Deployment approval", Deliverables: []string{"Change requests", "Deployment windows", "CAB approval"}},
			{Name: "Release Manager", Responsibility: "Go/No-Go coordination", Deliverables: []string{"Release plans", "Rollback procedures", "Coordination"}},
			{Name: "L3 Support Engineer", Responsibility: "Production bug escalation", Deliverables: []string{"Root cause analysis", "Hotfix coordination", "KB articles"}},
		},
		ExitCriteria: []string{"Change approved", "Release deployed", "Support handoff complete"},
		Status:       TeamStatusNotStarted,
	},
}

// PhaseGates defines the phase transition gates
var PhaseGates = map[string]PhaseGate{
	"1_to_2": {
		Name:         "Architecture Review Board",
		FromPhase:    1,
		ToPhase:      2,
		RequiredTeams: []int{1, 2, 3},
		ApprovalTeam: 2,
		Deliverables: []string{"Architecture Decision Records", "Approved Tech List", "Compliance Checklist"},
	},
	"2_to_3": {
		Name:         "Environment Readiness",
		FromPhase:    2,
		ToPhase:      3,
		RequiredTeams: []int{4, 5, 6},
		ApprovalTeam: 4,
		Deliverables: []string{"Infrastructure Provisioned", "CI/CD Pipelines", "Data Models"},
	},
	"3_to_4": {
		Name:         "Feature Complete + Code Review",
		FromPhase:    3,
		ToPhase:      4,
		RequiredTeams: []int{7, 8},
		ApprovalTeam: 7,
		Deliverables: []string{"Features Implemented", "Code Reviewed", "Documentation Complete"},
	},
	"4_to_5": {
		Name:         "Security + QA Sign-off",
		FromPhase:    4,
		ToPhase:      5,
		RequiredTeams: []int{9, 10},
		ApprovalTeam: 9,
		Deliverables: []string{"Security Review Passed", "Test Coverage Met", "UAT Sign-off"},
	},
}

// AgentTypeMappings maps agent types to teams
var AgentTypeMappings = map[string]AgentTypeMapping{
	"planner":       {AgentType: "planner", TeamID: 2, Phase: "Phase 1", Roles: []string{"Solution Architect", "Business Systems Analyst"}},
	"architect":     {AgentType: "architect", TeamID: 2, Phase: "Phase 1", Roles: []string{"Chief Architect", "Domain Architect"}},
	"infrastructure": {AgentType: "infrastructure", TeamID: 4, Phase: "Phase 2", Roles: []string{"Cloud Architect", "IaC Engineer"}},
	"platform":      {AgentType: "platform", TeamID: 5, Phase: "Phase 2", Roles: []string{"CI/CD Architect", "Kubernetes Administrator"}},
	"backend":       {AgentType: "backend", TeamID: 7, Phase: "Phase 3", Roles: []string{"Senior Backend Engineer", "Technical Lead"}},
	"frontend":      {AgentType: "frontend", TeamID: 7, Phase: "Phase 3", Roles: []string{"Senior Frontend Engineer", "Accessibility Expert"}},
	"security":      {AgentType: "security", TeamID: 9, Phase: "Phase 4", Roles: []string{"Security Architect", "Vulnerability Researcher"}},
	"qa":            {AgentType: "qa", TeamID: 10, Phase: "Phase 4", Roles: []string{"QA Architect", "SDET"}},
	"sre":           {AgentType: "sre", TeamID: 11, Phase: "Phase 5", Roles: []string{"SRE Lead", "Observability Engineer"}},
	"ops":           {AgentType: "ops", TeamID: 12, Phase: "Phase 5", Roles: []string{"Release Manager", "NOC Analyst"}},
}
