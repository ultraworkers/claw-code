# Standardized Team Layout

> Enterprise-grade team structure for software development lifecycle

**Version:** 1.0
**Applies To:** All projects, squads, and agent teams

---

## Overview

This document defines the standardized team structure across 5 phases of the software development lifecycle. Each team has specific responsibilities, required roles, and integration points.

### Team Size Requirements

All teams **MUST** have between **4 and 6 members** (inclusive).

- **Minimum:** 4 members (ensures adequate coverage of all critical roles)
- **Maximum:** 6 members (prevents team bloat and communication overhead)

Teams with fewer than 4 members are understaffed and cannot effectively cover all required responsibilities. Teams with more than 6 members suffer from coordination overhead and should be split into sub-teams.

This rule applies to:
- Human teams
- AI agent teams
- Mixed human-agent teams

---

## Phase 1: Strategy, Governance & Planning

### Team 1: Business & Product Strategy (The "Why")

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Business Relationship Manager (BRM)** | Connects IT to C-suite | Strategic alignment docs, executive briefings |
| **Lead Product Manager** | Owns long-term roadmap | Product roadmap, OKRs, feature prioritization |
| **Business Systems Analyst (BSA)** | Translates business to technical | Requirements specs, user stories, acceptance criteria |
| **Financial Controller (FinOps)** | Approves budget and cloud spend | Budget forecasts, cost projections, spend reports |

**Exit Criteria:**
- [ ] Business case approved
- [ ] Budget allocated
- [ ] Roadmap defined
- [ ] Success metrics established

---

### Team 2: Enterprise Architecture (The "Standards")

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Chief Architect** | Sets 5-year tech vision | Architecture vision, tech radar, strategic plans |
| **Domain Architect** | Specialized stack expertise | Domain-specific patterns, best practices guides |
| **Solution Architect** | Maps projects to standards | Solution designs, architecture decision records (ADRs) |
| **Standards Lead** | Manages Approved Tech List | Technology standards, evaluation criteria, approved list |

**Exit Criteria:**
- [ ] Architecture approved
- [ ] Technology choices validated
- [ ] Standards compliance verified

---

### Team 3: GRC (Governance, Risk, & Compliance)

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Compliance Officer** | SOX/HIPAA/GDPR adherence | Compliance checklists, audit reports |
| **Internal Auditor** | Pre-production mock audits | Audit findings, remediation plans |
| **Privacy Engineer** | Data masking and PII | Privacy impact assessments, data flow diagrams |
| **Policy Manager** | Maintains SOPs | Standard operating procedures, policy updates |

**Exit Criteria:**
- [ ] Compliance review passed
- [ ] Risk assessment complete
- [ ] Privacy requirements met
- [ ] Policies acknowledged

---

## Phase 2: Platform & Foundation

### Team 4: Infrastructure & Cloud Ops

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Cloud Architect** | VPC and network design | Network diagrams, security groups, routing tables |
| **IaC Engineer** | Provisions the "metal" | Terraform modules, Ansible playbooks, infrastructure code |
| **Network Security Engineer** | Firewalls, VPNs, Direct Connect | Security rules, network policies, access controls |
| **Storage Engineer** | S3/SAN management | Storage policies, backup strategies, archival rules |

**Exit Criteria:**
- [ ] Infrastructure provisioned
- [ ] Network connectivity verified
- [ ] Security rules applied
- [ ] Monitoring enabled

---

### Team 5: Platform Engineering (The "Internal Tools")

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Platform Product Manager** | Developer experience as product | Platform roadmap, DX metrics, adoption reports |
| **CI/CD Architect** | Golden pipelines | Pipeline templates, build configs, deployment strategies |
| **Kubernetes Administrator** | Cluster management | Cluster configs, resource quotas, ingress rules |
| **Developer Advocate** | Dev squad adoption | Onboarding guides, training materials, feedback loops |

**Exit Criteria:**
- [ ] Platform services ready
- [ ] CI/CD pipelines functional
- [ ] Developer onboarding complete

---

### Team 6: Data Governance & Analytics

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Data Architect** | Enterprise data model | Data models, schema designs, lineage documentation |
| **DBA** | Production database performance | Query optimization, index tuning, backup verification |
| **Data Privacy Officer (DPO)** | Retention and deletion rules | Data retention policies, deletion workflows |
| **ETL Developer** | Data flow management | ETL pipelines, data quality checks, transformation logic |

**Exit Criteria:**
- [ ] Data models defined
- [ ] Pipelines operational
- [ ] Privacy controls implemented

---

## Phase 3: The Build Squads

### Team 7: Core Feature Squad (The "Devs")

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Technical Lead** | Final word on implementation | Code reviews, architecture decisions, technical guidance |
| **Senior Backend Engineer** | Logic, APIs, microservices | Backend services, API endpoints, business logic |
| **Senior Frontend Engineer** | Design system, state management | UI components, frontend architecture, state logic |
| **Accessibility (A11y) Expert** | WCAG compliance | A11y audits, remediation plans, testing reports |
| **Technical Writer** | Internal/external docs | API docs, user guides, runbooks |

**Exit Criteria:**
- [ ] Features implemented
- [ ] Code reviewed and approved
- [ ] Documentation complete
- [ ] A11y requirements met

---

### Team 8: Middleware & Integration

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **API Product Manager** | API lifecycle and versioning | API specs, versioning strategy, deprecation plans |
| **Integration Engineer** | SAP/Oracle/Mainframe connections | Integration specs, data mappings, error handling |
| **Messaging Engineer** | Kafka/RabbitMQ management | Topic design, message schemas, consumer groups |
| **IAM Specialist** | Okta/AD integration | Auth flows, permission models, access policies |

**Exit Criteria:**
- [ ] APIs documented and tested
- [ ] Integrations verified
- [ ] Auth flows functional

---

## Phase 4: Validation & Hardening

### Team 9: Cybersecurity (AppSec)

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **Security Architect** | Threat model review | Threat models, security architecture, risk assessments |
| **Vulnerability Researcher** | SAST/DAST/SCA scanners | Scan reports, vulnerability triage, fix verification |
| **Penetration Tester** | Manual security testing | Pen test reports, exploit verification, remediation |
| **DevSecOps Engineer** | Security in CI/CD | Security gates, pipeline integration, compliance checks |

**Exit Criteria:**
- [ ] Security review passed
- [ ] Vulnerabilities remediated or accepted
- [ ] Pen testing complete
- [ ] Security gates passing

---

### Team 10: Quality Engineering (SDET)

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **QA Architect** | Global testing strategy | Test strategy, test plans, coverage reports |
| **SDET** | Automated test code | Test automation, framework maintenance, CI integration |
| **Performance/Load Engineer** | Scale testing | Load test scripts, performance baselines, capacity reports |
| **Manual QA / UAT Coordinator** | User acceptance testing | Test cases, UAT coordination, sign-off reports |

**Exit Criteria:**
- [ ] Test coverage requirements met
- [ ] Performance benchmarks achieved
- [ ] UAT sign-off obtained

---

## Phase 5: Delivery & Sustainment

### Team 11: Site Reliability Engineering (SRE)

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **SRE Lead** | Error budget and uptime SLA | SLOs, error budgets, reliability reports |
| **Observability Engineer** | Monitoring and logging | Dashboards, alerts, log aggregation, traces |
| **Chaos Engineer** | Resiliency testing | Chaos experiments, failure scenarios, recovery tests |
| **Incident Manager** | War room leadership | Incident response, post-mortems, runbook updates |

**Exit Criteria:**
- [ ] Monitoring in place
- [ ] Alerts configured
- [ ] Runbooks complete
- [ ] Error budget healthy

---

### Team 12: IT Operations & Support (NOC)

| Role | Responsibility | Key Deliverables |
|------|---------------|------------------|
| **NOC Analyst** | 24/7 monitoring | Monitoring dashboards, alert triage, incident tickets |
| **Change Manager** | Deployment approval | Change requests, deployment windows, CAB approval |
| **Release Manager** | Go/No-Go coordination | Release plans, rollback procedures, coordination |
| **L3 Support Engineer** | Production bug escalation | Root cause analysis, hotfix coordination, KB articles |

**Exit Criteria:**
- [ ] Change approved
- [ ] Release deployed
- [ ] Support handoff complete

---

## Team Interaction Model

```
┌─────────────────────────────────────────────────────────────────┐
│                     PHASE GATES                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1 → Phase 2: Architecture Review                         │
│  Phase 2 → Phase 3: Environment Readiness                      │
│  Phase 3 → Phase 4: Feature Complete + Code Review            │
│  Phase 4 → Phase 5: Security + QA Sign-off                     │
│  Phase 5 → Release: SRE + Change Approval                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Communication Channels

| Channel | Purpose | Participants |
|---------|---------|--------------|
| **Architecture Review Board** | Tech decisions | Team 2, Team 4, Team 5 |
| **Security Review** | Security gates | Team 3, Team 9 |
| **Release Coordination** | Deployment planning | Team 11, Team 12, Team 7 Lead |
| **Incident Response** | Production issues | Team 11, Team 12 |

---

## Agent Team Mapping

For AI agent teams, map to these standardized roles:

| Agent Role | Maps To | Responsibility |
|------------|---------|----------------|
| **Planner** | Solution Architect + BSA | Requirements analysis, solution design |
| **Coder** | Senior Backend/Frontend Engineer | Implementation |
| **Reviewer** | Technical Lead + QA Architect | Code review, quality gates |
| **Security** | Security Architect + AppSec | Security review, vulnerability checks |
| **Tester** | SDET + QA Architect | Automated testing, validation |
| **Ops** | SRE Lead + Platform Engineer | Deployment, monitoring |

---

## Quick Reference: Team Responsibilities by Phase

| Phase | Teams | Key Output |
|-------|-------|------------|
| 1 | 1, 2, 3 | Approved architecture, budget, compliance |
| 2 | 4, 5, 6 | Platform ready, infrastructure provisioned |
| 3 | 7, 8 | Working code, integrations complete |
| 4 | 9, 10 | Security clearance, quality sign-off |
| 5 | 11, 12 | Production deployment, operational handoff |

---

**Last Updated:** 2026-02-15
**Owner:** Enterprise Architecture Team
