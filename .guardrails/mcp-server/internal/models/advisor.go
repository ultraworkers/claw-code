package models

// Advisor represents a cross-cutting advisory role
type Advisor struct {
	ID                string   `json:"id"`
	Name              string   `json:"name"`
	Alias             string   `json:"alias"`
	EnforcementLevel  string   `json:"enforcement_level"` // "block", "warn", "info"
	Scope             string   `json:"scope"`
	ConsultsWithTeams []int    `json:"consults_with_teams"`
	Responsibility    string   `json:"responsibility"`
	PersonaVoice      string   `json:"persona_voice"`
	Deliverables      []string `json:"deliverables"`
	TriggerPatterns   []string `json:"trigger_patterns"`
	AssignedTo        *string  `json:"assigned_to,omitempty"`
}

// AdvisorListResult from list_advisors tool
type AdvisorListResult struct {
	Advisors []Advisor `json:"advisors"`
	Count    int       `json:"count"`
}

// AdvisorTriggerResult from trigger_check tool
type AdvisorTriggerResult struct {
	Triggered []TriggeredAdvisor `json:"triggered"`
	Count     int                `json:"count"`
}

// TriggeredAdvisor represents an advisor that was triggered
type TriggeredAdvisor struct {
	ID               string   `json:"id"`
	Name             string   `json:"name"`
	EnforcementLevel string   `json:"enforcement_level"`
	MatchedPatterns  []string `json:"matched_patterns"`
	Reason           string   `json:"reason"`
}

// AdvisorConsultInput for consult tool
type AdvisorConsultInput struct {
	AdvisorID   string   `json:"advisor_id"`
	Context     string   `json:"context"`
	FilePaths   []string `json:"file_paths"`
	FileDiffs   map[string]string `json:"file_diffs,omitempty"`
	SessionToken string  `json:"session_token"`
}

// AdvisorConsultResult from consult tool
type AdvisorConsultResult struct {
	AdvisorID       string   `json:"advisor_id"`
	AdvisorName     string   `json:"advisor_name"`
	Alias           string   `json:"alias"`
	Enforcement     string   `json:"enforcement"` // "block", "warn", "info"
	Severity        string   `json:"severity"`
	Message         string   `json:"message"`
	Recommendations []string `json:"recommendations"`
	References      []string `json:"references,omitempty"`
	PersonaVoice    string   `json:"persona_voice"`
}

// AdvisorResolveInput for resolve tool
type AdvisorResolveInput struct {
	AdvisorID         string `json:"advisor_id"`
	ResolutionStatus  string `json:"resolution_status"` // "applied", "bypassed_with_risk", "false_positive"
	Justification     string `json:"justification"`
	SessionToken      string `json:"session_token"`
}

// AdvisorResolveResult from resolve tool
type AdvisorResolveResult struct {
	Success     bool   `json:"success"`
	AdvisorID   string `json:"advisor_id"`
	Status      string `json:"status"`
	Message     string `json:"message"`
	Unblocked   bool   `json:"unblocked"`
}

// StandardAdvisors returns the built-in advisor definitions
func StandardAdvisors() map[string]Advisor {
	return map[string]Advisor{
		"advisor-cost": {
			ID:                "advisor-cost",
			Name:              "Cost & Efficiency Advisor",
			Alias:             "The Accountant",
			EnforcementLevel:  "warn",
			Scope:             "all_phases",
			ConsultsWithTeams: []int{1, 4, 5, 11},
			Responsibility:    "Reviews architectural decisions and infrastructure choices through a cost lens. Flags over-provisioned resources.",
			PersonaVoice:      "Before we spin up another cluster — what's the actual load forecast? Show me the numbers.",
			Deliverables:      []string{"Cost estimation reviews", "Resource right-sizing recommendations", "Reserved capacity analysis"},
			TriggerPatterns:   []string{"*instance*", "*cluster*", "*provision*", "*capacity*", "*scale*"},
		},
		"advisor-dx": {
			ID:                "advisor-dx",
			Name:              "Developer Experience Advisor",
			Alias:             "The Advocate",
			EnforcementLevel:  "info",
			Scope:             "all_phases",
			ConsultsWithTeams: []int{5, 7, 8, 10},
			Responsibility:    "Evaluates tooling choices, CI/CD pipeline ergonomics, and documentation quality. Champions minimal cognitive load.",
			PersonaVoice:      "If a new engineer can't get this running in under 30 minutes, we have a DX problem.",
			Deliverables:      []string{"Onboarding friction reports", "Tooling ergonomics assessments", "Documentation gap analysis"},
			TriggerPatterns:   []string{"*onboard*", "*setup*", "*config*", "*tool*", "*ci/cd*"},
		},
		"advisor-resilience": {
			ID:                "advisor-resilience",
			Name:              "Resilience & Failure Advisor",
			Alias:             "The Pessimist",
			EnforcementLevel:  "block",
			Scope:             "phase_2_to_5",
			ConsultsWithTeams: []int{4, 7, 9, 11},
			Responsibility:    "Reviews designs for single points of failure, missing retries, absent circuit breakers, and untested failure paths.",
			PersonaVoice:      "Great, it works. Now what happens when the database is 200ms slower than expected? What about when it's gone entirely?",
			Deliverables:      []string{"FMEA", "Blast radius assessments", "Chaos experiment proposals"},
			TriggerPatterns:   []string{"*retry*", "*timeout*", "*circuit*", "*fallback*", "*health*", "*bulkhead*", "*rate-limit*"},
		},
		"advisor-privacy": {
			ID:                "advisor-privacy",
			Name:              "Data Privacy & Ethics Advisor",
			Alias:             "The Conscience",
			EnforcementLevel:  "block",
			Scope:             "all_phases",
			ConsultsWithTeams: []int{3, 6, 9},
			Responsibility:    "Ensures GDPR/CCPA compliance, data minimization, consent management, and ethical AI use.",
			PersonaVoice:      "We're collecting this data — but do we actually need it? What's the retention policy? Can the user delete it?",
			Deliverables:      []string{"Privacy impact assessments", "Data flow audits", "Consent management reviews"},
			TriggerPatterns:   []string{"*pii*", "*gdpr*", "*consent*", "*retention*", "*encrypt*", "*personal*"},
		},
		"advisor-api": {
			ID:                "advisor-api",
			Name:              "API & Integration Advisor",
			Alias:             "The Diplomat",
			EnforcementLevel:  "block",
			Scope:             "phase_2_to_4",
			ConsultsWithTeams: []int{2, 7, 8},
			Responsibility:    "Reviews API contracts for breaking changes, ensures versioning strategy is followed, and checks third-party reliability.",
			PersonaVoice:      "You're adding a required field to a v2 response — every downstream consumer will break. Let's talk migration.",
			Deliverables:      []string{"API contract reviews", "Breaking change assessments", "Version migration plans"},
			TriggerPatterns:   []string{"*api*", "*endpoint*", "*contract*", "*version*", "*schema*", "*openapi*"},
		},
		"advisor-perf": {
			ID:                "advisor-perf",
			Name:              "Performance & Scalability Advisor",
			Alias:             "The Profiler",
			EnforcementLevel:  "warn",
			Scope:             "phase_2_to_5",
			ConsultsWithTeams: []int{4, 7, 10, 11},
			Responsibility:    "Reviews code for N+1 queries, memory leaks, and cache misses. Ensures capacity planning is data-driven.",
			PersonaVoice:      "This endpoint does a full table scan. At current traffic it's fine — at 5x it'll take the service down.",
			Deliverables:      []string{"Performance benchmarks", "Scalability assessments", "Capacity planning recommendations"},
			TriggerPatterns:   []string{"*query*", "*cache*", "*memory*", "*cpu*", "*benchmark*", "*load*"},
		},
		"advisor-a11y": {
			ID:                "advisor-a11y",
			Name:              "Accessibility & UX Advisor",
			Alias:             "The Equalizer",
			EnforcementLevel:  "warn",
			Scope:             "phase_3_to_4",
			ConsultsWithTeams: []int{7, 10},
			Responsibility:    "Reviews UI components and DOM structures for WCAG compliance, screen reader compatibility, and keyboard navigation.",
			PersonaVoice:      "A beautiful button is useless if a keyboard user can't tab to it or a screen reader just says 'unlabeled graphic'.",
			Deliverables:      []string{"WCAG compliance audits", "Screen reader compatibility reports", "Keyboard navigation assessments"},
			TriggerPatterns:   []string{"*aria*", "*label*", "*focus*", "*tab*", "*screen*", "*wcag*"},
		},
		"advisor-supply-chain": {
			ID:                "advisor-supply-chain",
			Name:              "Supply Chain & OSS Advisor",
			Alias:             "The Librarian",
			EnforcementLevel:  "block",
			Scope:             "all_phases",
			ConsultsWithTeams: []int{5, 8, 9},
			Responsibility:    "Evaluates third-party dependencies for known CVEs, abandoned maintenance status, and restrictive open-source licenses.",
			PersonaVoice:      "You're pulling in a library maintained by one person who hasn't committed since 2019. We need an alternative.",
			Deliverables:      []string{"CVE impact assessments", "Dependency health reports", "License compliance audits"},
			TriggerPatterns:   []string{"*package*", "*dependency*", "*vendor*", "*cve*", "*license*", "*version*"},
		},
		"advisor-audit": {
			ID:                "advisor-audit",
			Name:              "Compliance & Audit Advisor",
			Alias:             "The Auditor",
			EnforcementLevel:  "block",
			Scope:             "all_phases",
			ConsultsWithTeams: []int{3, 4, 6},
			Responsibility:    "Focuses strictly on SOC2, HIPAA, PCI-DSS, or ISO27001 controls. Verifies audit logging and auditable access controls.",
			PersonaVoice:      "I see the database is encrypted, but where is the immutable audit log showing who accessed this? If we can't prove it, we fail.",
			Deliverables:      []string{"Control gap assessments", "Audit trail verifications", "Compliance readiness reports"},
			TriggerPatterns:   []string{"*audit*", "*log*", "*soc2*", "*hipaa*", "*pci*", "*iso*", "*encrypt*"},
		},
	}
}
