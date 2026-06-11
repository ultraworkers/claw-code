#!/usr/bin/env python3
"""
Mock Team Manager for testing without file system operations.
"""

import json
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Any
from copy import deepcopy

from .mock_file_system import MockFileSystem
from .mock_logger import MockLogger
from .mock_user_context import MockUserContext


@dataclass
class MockRole:
    """Mock role for in-memory storage."""
    name: str
    responsibility: str
    deliverables: List[str]
    assigned_to: Optional[str] = None


@dataclass
class MockTeam:
    """Mock team for in-memory storage."""
    id: int
    name: str
    phase: str
    description: str
    roles: List[MockRole]
    exit_criteria: List[str]
    status: str = "not_started"
    started_at: Optional[str] = None
    completed_at: Optional[str] = None


class MockTeamManager:
    """
    Mock TeamManager for unit testing without file I/O.

    Provides all TeamManager operations using in-memory storage.
    Tracks method calls for assertions.
    """

    # Standard team definitions (copied from team_manager.py)
    STANDARD_TEAMS = {
        1: MockTeam(
            id=1, name="Business & Product Strategy",
            phase="Phase 1: Strategy, Governance & Planning",
            description="The 'Why' - Business case and product strategy",
            roles=[
                MockRole("Business Relationship Manager", "Connects IT to C-suite",
                        ["Strategic alignment docs", "Executive briefings"]),
                MockRole("Lead Product Manager", "Owns long-term roadmap",
                        ["Product roadmap", "OKRs", "Feature prioritization"]),
                MockRole("Business Systems Analyst", "Translates business to technical",
                        ["Requirements specs", "User stories", "Acceptance criteria"]),
                MockRole("Financial Controller (FinOps)", "Approves budget and cloud spend",
                        ["Budget forecasts", "Cost projections", "Spend reports"]),
            ],
            exit_criteria=["Business case approved", "Budget allocated", "Roadmap defined", "Success metrics established"]
        ),
        2: MockTeam(
            id=2, name="Enterprise Architecture",
            phase="Phase 1: Strategy, Governance & Planning",
            description="The 'Standards' - Technology vision and standards",
            roles=[
                MockRole("Chief Architect", "Sets 5-year tech vision",
                        ["Architecture vision", "Tech radar", "Strategic plans"]),
                MockRole("Domain Architect", "Specialized stack expertise",
                        ["Domain-specific patterns", "Best practices guides"]),
                MockRole("Solution Architect", "Maps projects to standards",
                        ["Solution designs", "Architecture decision records"]),
                MockRole("Standards Lead", "Manages Approved Tech List",
                        ["Technology standards", "Evaluation criteria", "Approved list"]),
            ],
            exit_criteria=["Architecture approved", "Technology choices validated", "Standards compliance verified"]
        ),
        3: MockTeam(
            id=3, name="GRC (Governance, Risk, & Compliance)",
            phase="Phase 1: Strategy, Governance & Planning",
            description="Compliance and risk management",
            roles=[
                MockRole("Compliance Officer", "SOX/HIPAA/GDPR adherence",
                        ["Compliance checklists", "Audit reports"]),
                MockRole("Internal Auditor", "Pre-production mock audits",
                        ["Audit findings", "Remediation plans"]),
                MockRole("Privacy Engineer", "Data masking and PII",
                        ["Privacy impact assessments", "Data flow diagrams"]),
                MockRole("Policy Manager", "Maintains SOPs",
                        ["Standard operating procedures", "Policy updates"]),
            ],
            exit_criteria=["Compliance review passed", "Risk assessment complete", "Privacy requirements met", "Policies acknowledged"]
        ),
        4: MockTeam(
            id=4, name="Infrastructure & Cloud Ops",
            phase="Phase 2: Platform & Foundation",
            description="Cloud infrastructure and networking",
            roles=[
                MockRole("Cloud Architect", "VPC and network design",
                        ["Network diagrams", "Security groups", "Routing tables"]),
                MockRole("IaC Engineer", "Provisions the 'metal'",
                        ["Terraform modules", "Ansible playbooks", "Infrastructure code"]),
                MockRole("Network Security Engineer", "Firewalls, VPNs, Direct Connect",
                        ["Security rules", "Network policies", "Access controls"]),
                MockRole("Storage Engineer", "S3/SAN management",
                        ["Storage policies", "Backup strategies", "Archival rules"]),
            ],
            exit_criteria=["Infrastructure provisioned", "Network connectivity verified", "Security rules applied", "Monitoring enabled"]
        ),
        5: MockTeam(
            id=5, name="Platform Engineering",
            phase="Phase 2: Platform & Foundation",
            description="The 'Internal Tools' - Developer experience platform",
            roles=[
                MockRole("Platform Product Manager", "Developer experience as product",
                        ["Platform roadmap", "DX metrics", "Adoption reports"]),
                MockRole("CI/CD Architect", "Golden pipelines",
                        ["Pipeline templates", "Build configs", "Deployment strategies"]),
                MockRole("Kubernetes Administrator", "Cluster management",
                        ["Cluster configs", "Resource quotas", "Ingress rules"]),
                MockRole("Developer Advocate", "Dev squad adoption",
                        ["Onboarding guides", "Training materials", "Feedback loops"]),
            ],
            exit_criteria=["Platform services ready", "CI/CD pipelines functional", "Developer onboarding complete"]
        ),
        6: MockTeam(
            id=6, name="Data Governance & Analytics",
            phase="Phase 2: Platform & Foundation",
            description="Enterprise data management",
            roles=[
                MockRole("Data Architect", "Enterprise data model",
                        ["Data models", "Schema designs", "Lineage documentation"]),
                MockRole("DBA", "Production database performance",
                        ["Query optimization", "Index tuning", "Backup verification"]),
                MockRole("Data Privacy Officer", "Retention and deletion rules",
                        ["Data retention policies", "Deletion workflows"]),
                MockRole("ETL Developer", "Data flow management",
                        ["ETL pipelines", "Data quality checks", "Transformation logic"]),
            ],
            exit_criteria=["Data models defined", "Pipelines operational", "Privacy controls implemented"]
        ),
        7: MockTeam(
            id=7, name="Core Feature Squad",
            phase="Phase 3: The Build Squads",
            description="The 'Devs' - Feature implementation",
            roles=[
                MockRole("Technical Lead", "Final word on implementation",
                        ["Code reviews", "Architecture decisions", "Technical guidance"]),
                MockRole("Senior Backend Engineer", "Logic, APIs, microservices",
                        ["Backend services", "API endpoints", "Business logic"]),
                MockRole("Senior Frontend Engineer", "Design system, state management",
                        ["UI components", "Frontend architecture", "State logic"]),
                MockRole("Accessibility (A11y) Expert", "WCAG compliance",
                        ["A11y audits", "Remediation plans", "Testing reports"]),
                MockRole("Technical Writer", "Internal/external docs",
                        ["API docs", "User guides", "Runbooks"]),
            ],
            exit_criteria=["Features implemented", "Code reviewed and approved", "Documentation complete", "A11y requirements met"]
        ),
        8: MockTeam(
            id=8, name="Middleware & Integration",
            phase="Phase 3: The Build Squads",
            description="APIs and system integrations",
            roles=[
                MockRole("API Product Manager", "API lifecycle and versioning",
                        ["API specs", "Versioning strategy", "Deprecation plans"]),
                MockRole("Integration Engineer", "SAP/Oracle/Mainframe connections",
                        ["Integration specs", "Data mappings", "Error handling"]),
                MockRole("Messaging Engineer", "Kafka/RabbitMQ management",
                        ["Topic design", "Message schemas", "Consumer groups"]),
                MockRole("IAM Specialist", "Okta/AD integration",
                        ["Auth flows", "Permission models", "Access policies"]),
            ],
            exit_criteria=["APIs documented and tested", "Integrations verified", "Auth flows functional"]
        ),
        9: MockTeam(
            id=9, name="Cybersecurity (AppSec)",
            phase="Phase 4: Validation & Hardening",
            description="Application security",
            roles=[
                MockRole("Security Architect", "Threat model review",
                        ["Threat models", "Security architecture", "Risk assessments"]),
                MockRole("Vulnerability Researcher", "SAST/DAST/SCA scanners",
                        ["Scan reports", "Vulnerability triage", "Fix verification"]),
                MockRole("Penetration Tester", "Manual security testing",
                        ["Pen test reports", "Exploit verification", "Remediation"]),
                MockRole("DevSecOps Engineer", "Security in CI/CD",
                        ["Security gates", "Pipeline integration", "Compliance checks"]),
            ],
            exit_criteria=["Security review passed", "Vulnerabilities remediated or accepted", "Pen testing complete", "Security gates passing"]
        ),
        10: MockTeam(
            id=10, name="Quality Engineering (SDET)",
            phase="Phase 4: Validation & Hardening",
            description="Testing and quality assurance",
            roles=[
                MockRole("QA Architect", "Global testing strategy",
                        ["Test strategy", "Test plans", "Coverage reports"]),
                MockRole("SDET", "Automated test code",
                        ["Test automation", "Framework maintenance", "CI integration"]),
                MockRole("Performance/Load Engineer", "Scale testing",
                        ["Load test scripts", "Performance baselines", "Capacity reports"]),
                MockRole("Manual QA / UAT Coordinator", "User acceptance testing",
                        ["Test cases", "UAT coordination", "Sign-off reports"]),
            ],
            exit_criteria=["Test coverage requirements met", "Performance benchmarks achieved", "UAT sign-off obtained"]
        ),
        11: MockTeam(
            id=11, name="Site Reliability Engineering (SRE)",
            phase="Phase 5: Delivery & Sustainment",
            description="Reliability and observability",
            roles=[
                MockRole("SRE Lead", "Error budget and uptime SLA",
                        ["SLOs", "Error budgets", "Reliability reports"]),
                MockRole("Observability Engineer", "Monitoring and logging",
                        ["Dashboards", "Alerts", "Log aggregation", "Traces"]),
                MockRole("Chaos Engineer", "Resiliency testing",
                        ["Chaos experiments", "Failure scenarios", "Recovery tests"]),
                MockRole("Incident Manager", "War room leadership",
                        ["Incident response", "Post-mortems", "Runbook updates"]),
            ],
            exit_criteria=["Monitoring in place", "Alerts configured", "Runbooks complete", "Error budget healthy"]
        ),
        12: MockTeam(
            id=12, name="IT Operations & Support (NOC)",
            phase="Phase 5: Delivery & Sustainment",
            description="Production operations",
            roles=[
                MockRole("NOC Analyst", "24/7 monitoring",
                        ["Monitoring dashboards", "Alert triage", "Incident tickets"]),
                MockRole("Change Manager", "Deployment approval",
                        ["Change requests", "Deployment windows", "CAB approval"]),
                MockRole("Release Manager", "Go/No-Go coordination",
                        ["Release plans", "Rollback procedures", "Coordination"]),
                MockRole("L3 Support Engineer", "Production bug escalation",
                        ["Root cause analysis", "Hotfix coordination", "KB articles"]),
            ],
            exit_criteria=["Change approved", "Release deployed", "Support handoff complete"]
        ),
    }

    def __init__(self, project_name: str, user_context: Optional[MockUserContext] = None,
                 logger: Optional[MockLogger] = None, fs: Optional[MockFileSystem] = None):
        self.project_name = project_name
        self.teams: Dict[int, MockTeam] = {}
        self.user_context = user_context
        self.logger = logger or MockLogger("mock_team_manager")
        self.fs = fs or MockFileSystem()
        self._method_calls: List[Dict] = []

    def _track_call(self, method: str, args: tuple, kwargs: Dict) -> None:
        """Track method call for assertions."""
        self._method_calls.append({
            "method": method,
            "args": args,
            "kwargs": kwargs,
            "timestamp": datetime.now().isoformat()
        })

    def _require_auth(self, operation: str, team_id: Optional[int] = None) -> None:
        """Check if user is authorized for the operation."""
        from scripts.team_manager import PermissionDenied

        if self.user_context is None:
            raise PermissionDenied(f"Authentication required for {operation}")

        if not self.user_context.has_permission("team-lead"):
            raise PermissionDenied(
                f"User '{self.user_context.user_id}' with role '{self.user_context.role}' "
                f"does not have permission to {operation}"
            )

        if team_id is not None and not self.user_context.can_modify_team(team_id):
            raise PermissionDenied(
                f"User '{self.user_context.user_id}' cannot modify team {team_id}. "
                f"Requires admin role or team-lead for this specific team."
            )

    def get_calls(self) -> List[Dict]:
        """Get all tracked method calls."""
        return self._method_calls.copy()

    def was_called(self, method: str) -> bool:
        """Check if a method was called."""
        return any(c["method"] == method for c in self._method_calls)

    def get_call_count(self, method: str) -> int:
        """Get number of times a method was called."""
        return sum(1 for c in self._method_calls if c["method"] == method)

    def clear_calls(self) -> None:
        """Clear tracked calls."""
        self._method_calls.clear()

    def initialize_project(self) -> bool:
        """Initialize a new project with all teams."""
        self._track_call("initialize_project", (), {})
        self._require_auth("initialize project")
        self.teams = {team_id: deepcopy(team) for team_id, team in self.STANDARD_TEAMS.items()}
        self.save()
        self.logger.info("project_initialized", {"project": self.project_name, "teams": len(self.teams)})
        return True

    def load(self) -> bool:
        """Load team configuration from mock file system."""
        self._track_call("load", (), {})
        config_path = Path(f".teams/{self.project_name}.json")
        if not self.fs.exists(config_path):
            return False

        try:
            data = self.fs.read_json(config_path)
            self.teams = {}
            for team_data in data.get("teams", []):
                # Filter out roles and provide empty list initially
                team_kwargs = {k: v for k, v in team_data.items() if k != 'roles'}
                team_kwargs['roles'] = []  # Will be populated next
                team = MockTeam(**team_kwargs)
                team.roles = [MockRole(**r) for r in team_data.get("roles", [])]
                self.teams[team.id] = team
            return True
        except Exception as e:
            self.logger.error("load_failed", {"error": str(e)})
            return False

    def save(self) -> bool:
        """Save team configuration to mock file system."""
        self._track_call("save", (), {})
        config_path = Path(f".teams/{self.project_name}.json")

        data = {
            "project_name": self.project_name,
            "updated_at": datetime.now().isoformat(),
            "teams": [
                {
                    "id": t.id,
                    "name": t.name,
                    "phase": t.phase,
                    "description": t.description,
                    "status": t.status,
                    "started_at": t.started_at,
                    "completed_at": t.completed_at,
                    "roles": [
                        {"name": r.name, "responsibility": r.responsibility,
                         "deliverables": r.deliverables, "assigned_to": r.assigned_to}
                        for r in t.roles
                    ],
                    "exit_criteria": t.exit_criteria
                }
                for t in self.teams.values()
            ]
        }

        self.fs.write_json(config_path, data)
        self.logger.info("project_saved", {"project": self.project_name, "teams": len(self.teams)})
        return True

    def assign_role(self, team_id: int, role_name: str, assignee: str) -> bool:
        """Assign a person to a role."""
        self._track_call("assign_role", (team_id, role_name, assignee), {})
        self._require_auth("assign role", team_id)

        if team_id not in self.teams:
            self.logger.error("team_not_found", {"team_id": team_id})
            return False

        team = self.teams[team_id]
        for role in team.roles:
            if role.name == role_name:
                role.assigned_to = assignee
                self.save()
                self.logger.info("role_assigned", {
                    "team_id": team_id, "role_name": role_name, "assignee": assignee
                })
                return True

        self.logger.error("role_not_found", {"team_id": team_id, "role_name": role_name})
        return False

    def unassign_role(self, team_id: int, role_name: str) -> bool:
        """Remove assignment from a role."""
        self._track_call("unassign_role", (team_id, role_name), {})

        if team_id not in self.teams:
            return False

        team = self.teams[team_id]
        for role in team.roles:
            if role.name == role_name:
                if role.assigned_to is None:
                    return False
                role.assigned_to = None
                self.save()
                return True

        return False

    def start_team(self, team_id: int) -> bool:
        """Mark a team as active."""
        self._track_call("start_team", (team_id,), {})

        if team_id not in self.teams:
            return False

        team = self.teams[team_id]
        team.status = "active"
        team.started_at = datetime.now().isoformat()
        self.save()
        return True

    def complete_team(self, team_id: int) -> bool:
        """Mark a team as completed."""
        self._track_call("complete_team", (team_id,), {})

        if team_id not in self.teams:
            return False

        team = self.teams[team_id]
        team.status = "completed"
        team.completed_at = datetime.now().isoformat()
        self.save()
        return True

    def get_phase_status(self, phase: str) -> dict:
        """Get status summary for a phase."""
        self._track_call("get_phase_status", (phase,), {})

        phase_teams = [t for t in self.teams.values() if t.phase == phase]
        total = len(phase_teams)
        completed = len([t for t in phase_teams if t.status == "completed"])
        active = len([t for t in phase_teams if t.status == "active"])

        return {
            "phase": phase,
            "total_teams": total,
            "completed": completed,
            "active": active,
            "not_started": total - completed - active,
            "progress_pct": (completed / total * 100) if total > 0 else 0
        }

    def list_teams(self, phase: str = None) -> List[Dict]:
        """Return list of teams (not print)."""
        self._track_call("list_teams", (), {"phase": phase})

        teams = list(self.teams.values())
        if phase:
            teams = [t for t in teams if t.phase == phase]

        return [
            {
                "id": t.id,
                "name": t.name,
                "phase": t.phase,
                "status": t.status,
                "assigned_count": sum(1 for r in t.roles if r.assigned_to),
                "total_roles": len(t.roles)
            }
            for t in sorted(teams, key=lambda t: (t.phase, t.id))
        ]

    def get_agent_team(self, agent_type: str) -> Optional[MockTeam]:
        """Map agent type to appropriate team."""
        mapping = {
            "planner": 2,
            "coder": 7,
            "reviewer": 10,
            "security": 9,
            "tester": 10,
            "ops": 11,
        }
        team_id = mapping.get(agent_type.lower())
        return self.teams.get(team_id)

    def validate_team_size(self, team_id: Optional[int] = None) -> dict:
        """Validate team sizes meet 4-6 member requirement."""
        self._track_call("validate_team_size", (), {"team_id": team_id})

        MIN_TEAM_SIZE = 4
        MAX_TEAM_SIZE = 6

        results = {"valid": True, "violations": [], "teams_checked": 0}
        teams_to_check = [self.teams[team_id]] if team_id else list(self.teams.values())

        for team in teams_to_check:
            results["teams_checked"] += 1
            assigned_count = sum(1 for role in team.roles if role.assigned_to)

            if assigned_count < MIN_TEAM_SIZE:
                results["valid"] = False
                results["violations"].append({
                    "team_id": team.id,
                    "team_name": team.name,
                    "issue": "undersized",
                    "assigned": assigned_count,
                    "required": MIN_TEAM_SIZE
                })
            elif assigned_count > MAX_TEAM_SIZE:
                results["valid"] = False
                results["violations"].append({
                    "team_id": team.id,
                    "team_name": team.name,
                    "issue": "oversized",
                    "assigned": assigned_count,
                    "maximum": MAX_TEAM_SIZE
                })

        return results

    def delete_team(self, team_id: int, confirmed: bool = False) -> dict:
        """Delete a specific team from the project."""
        self._track_call("delete_team", (team_id,), {"confirmed": confirmed})

        result = {
            "success": False,
            "team_id": team_id,
            "message": "",
            "requires_confirmation": False
        }

        if team_id not in self.teams:
            result["message"] = f"Team {team_id} not found"
            return result

        team = self.teams[team_id]

        if not confirmed:
            result["requires_confirmation"] = True
            result["message"] = f"Deletion requires confirmation for team {team_id} ({team.name})"
            return result

        deleted_name = team.name
        del self.teams[team_id]
        self.save()

        result["success"] = True
        result["message"] = f"Team {team_id} ({deleted_name}) deleted"
        return result

    def delete_project(self, confirmed: bool = False) -> dict:
        """Delete the entire project."""
        self._track_call("delete_project", (), {"confirmed": confirmed})

        result = {
            "success": False,
            "project_name": self.project_name,
            "message": "",
            "requires_confirmation": False
        }

        config_path = Path(f".teams/{self.project_name}.json")
        if not self.fs.exists(config_path):
            result["message"] = f"Project '{self.project_name}' not found"
            return result

        if not confirmed:
            result["requires_confirmation"] = True
            result["message"] = f"Deletion requires confirmation for project '{self.project_name}'"
            return result

        self.fs.delete(config_path)
        team_count = len(self.teams)
        self.teams = {}

        result["success"] = True
        result["message"] = f"Project '{self.project_name}' ({team_count} teams) deleted"
        return result

    def reset(self) -> None:
        """Reset the manager to initial state."""
        self.teams.clear()
        self._method_calls.clear()
        self.logger.clear()
        self.fs.reset()

    def to_dict(self) -> dict:
        """Export current state as dictionary."""
        return {
            "project_name": self.project_name,
            "team_count": len(self.teams),
            "teams": {
                tid: {
                    "id": t.id,
                    "name": t.name,
                    "phase": t.phase,
                    "status": t.status,
                    "roles": [
                        {"name": r.name, "assigned_to": r.assigned_to}
                        for r in t.roles
                    ]
                }
                for tid, t in self.teams.items()
            }
        }

    # FUNC-005: Batch operation wrappers
    def import_csv_file(self, csv_path: Path, dry_run: bool = False) -> Dict[str, Any]:
        """Import role assignments from CSV file."""
        from ..batch_operations import import_csv
        return import_csv(self, csv_path, dry_run)

    def export_csv_file(self, csv_path: Path) -> Dict[str, Any]:
        """Export role assignments to CSV file."""
        from ..batch_operations import export_csv
        return export_csv(self, csv_path)

    def import_json_file(self, json_path: Path, dry_run: bool = False) -> Dict[str, Any]:
        """Import role assignments from JSON file."""
        from ..batch_operations import import_json
        return import_json(self, json_path, dry_run)

    def export_json_file(self, json_path: Path, pretty: bool = True) -> Dict[str, Any]:
        """Export project state to JSON file."""
        from ..batch_operations import export_json
        return export_json(self, json_path, pretty)
