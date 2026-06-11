# Infrastructure Standards (IaC)

> **Infrastructure as Code.** No ClickOps, only declarative definitions.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [OPERATIONAL_PATTERNS.md](./OPERATIONAL_PATTERNS.md)

---

## Overview

This document establishes Infrastructure as Code (IaC) standards for projects that manage cloud resources. All infrastructure must be defined in version-controlled configuration files - never through manual console operations ("ClickOps").

**Core Principle:** If it's not in code, it doesn't exist.

---

## THE NO-CLICKOPS MANDATE

### Why ClickOps is Forbidden

```
CLICKOPS RISKS:

1. NO AUDIT TRAIL
   - Who created that server?
   - When was the firewall rule added?
   - Why is that bucket public?

2. NO REPRODUCIBILITY
   - Can't recreate environment after disaster
   - Can't spin up identical staging environment
   - "It works on my cloud" problems

3. DRIFT ACCUMULATION
   - Manual changes diverge from expected state
   - Security patches applied inconsistently
   - Configuration rot over time

4. AI AGENT INCOMPATIBILITY
   - Agents can't click buttons
   - Agents can generate IaC code
   - Declarative > Imperative for AI
```

### The IaC Mandate

```
MANDATORY RULE:

All infrastructure changes MUST be:
1. Defined in code (Terraform, Pulumi, CloudFormation)
2. Reviewed via pull request
3. Applied through CI/CD pipeline
4. Tracked in version control

FORBIDDEN:
- Creating resources via cloud console
- SSH'ing into servers to configure them
- Manual DNS changes
- Direct database modifications
- Any change not captured in code
```

---

## TERRAFORM STANDARDS

### Directory Structure

```
infrastructure/
├── environments/
│   ├── production/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   └── terraform.tfvars
│   ├── staging/
│   │   └── ...
│   └── development/
│       └── ...
├── modules/
│   ├── database/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   └── outputs.tf
│   ├── networking/
│   │   └── ...
│   └── compute/
│       └── ...
├── shared/
│   ├── providers.tf
│   └── backend.tf
└── README.md
```

### Required File Structure

```hcl
# main.tf - Resource definitions
# Keep resources grouped by type/purpose

# Database resources
resource "aws_db_instance" "main" {
  # ...
}

# Compute resources
resource "aws_ecs_service" "app" {
  # ...
}
```

```hcl
# variables.tf - Input variables
# All variables must have descriptions

variable "environment" {
  description = "Deployment environment (production, staging, development)"
  type        = string
  validation {
    condition     = contains(["production", "staging", "development"], var.environment)
    error_message = "Environment must be production, staging, or development."
  }
}

variable "database_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.micro"
}
```

```hcl
# outputs.tf - Exported values
# Document what other modules might need

output "database_endpoint" {
  description = "Database connection endpoint"
  value       = aws_db_instance.main.endpoint
  sensitive   = false
}

output "database_password" {
  description = "Database password (sensitive)"
  value       = aws_db_instance.main.password
  sensitive   = true
}
```

---

## THE PLAN-BEFORE-APPLY PROTOCOL

### Never Apply Without Plan Review

```
MANDATORY WORKFLOW:

1. MAKE CHANGES
   - Edit .tf files
   - Commit to branch
   - Create pull request

2. RUN PLAN
   $ terraform plan -out=tfplan
   
   Review output for:
   + Resources to CREATE (safe)
   ~ Resources to UPDATE (review carefully)
   - Resources to DESTROY (DANGER - requires approval)

3. REVIEW PLAN
   - Check for unintended destroys
   - Verify resource names
   - Confirm configuration values
   - Look for drift detection

4. GET APPROVAL
   - PR must be approved
   - Plan output must be reviewed
   - Destroy operations require explicit signoff

5. APPLY
   $ terraform apply tfplan
   
   Only after all reviews complete.
```

### Agent IaC Directive

```
AGENT PROTOCOL FOR INFRASTRUCTURE:

1. GENERATE CODE ONLY
   - Create/modify .tf files
   - DO NOT run terraform apply
   - DO NOT access cloud console

2. SHOW PLAN OUTPUT
   - Run: terraform plan
   - Present plan to user
   - Highlight any DESTROY operations

3. WAIT FOR APPROVAL
   - User must explicitly approve
   - Never auto-apply infrastructure changes

4. DOCUMENT CHANGES
   - Update README with what changed
   - Note any manual steps required
```

---

## DRIFT DETECTION

### What is Drift?

```
DRIFT: When actual infrastructure differs from code.

CAUSES:
- Manual console changes
- Another team member's direct modification
- Cloud provider auto-updates
- Failed partial applies

DETECTION:
$ terraform plan

If plan shows changes you didn't make → DRIFT DETECTED
```

### Drift Response Protocol

```
WHEN DRIFT DETECTED:

1. INVESTIGATE
   - What changed?
   - Who changed it?
   - When did it happen?
   - Was it intentional?

2. DECIDE
   Option A: Accept drift → Update code to match reality
   Option B: Reject drift → Apply code to fix reality

3. PREVENT
   - Add drift detection to CI/CD
   - Alert on unexpected changes
   - Review IAM permissions
```

### Automated Drift Detection

```yaml
# .github/workflows/drift-detection.yml
name: Infrastructure Drift Detection

on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours

jobs:
  detect-drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Terraform
        uses: hashicorp/setup-terraform@v3
      
      - name: Terraform Init
        run: terraform init
        working-directory: infrastructure/environments/production
      
      - name: Detect Drift
        id: plan
        run: |
          terraform plan -detailed-exitcode -out=tfplan 2>&1 | tee plan_output.txt
          echo "exitcode=$?" >> $GITHUB_OUTPUT
        working-directory: infrastructure/environments/production
        continue-on-error: true
      
      - name: Alert on Drift
        if: steps.plan.outputs.exitcode == '2'
        run: |
          echo "DRIFT DETECTED!"
          cat plan_output.txt
          # Send alert to Slack/PagerDuty
```

---

## STATE FILE MANAGEMENT

### State File Security

```
STATE FILES CONTAIN SENSITIVE DATA:
- Database passwords
- API keys
- Resource IDs
- Connection strings

MANDATORY RULES:
1. NEVER commit state files to git
2. Use remote state backend (S3, GCS, Terraform Cloud)
3. Enable state file encryption
4. Enable state file locking
5. Restrict state file access
```

### Backend Configuration

```hcl
# backend.tf
terraform {
  backend "s3" {
    bucket         = "company-terraform-state"
    key            = "production/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-state-lock"
  }
}
```

### State File Agent Rules

```
AGENT RULES FOR STATE:

NEVER:
- Output state file contents
- Commit state files
- Share state file paths publicly
- Modify state directly (terraform state commands without approval)

ALWAYS:
- Use remote backend
- Verify state is locked before operations
- Report state-related errors immediately
```

---

## RESOURCE NAMING CONVENTIONS

### Standard Naming Pattern

```
PATTERN: {project}-{environment}-{resource}-{identifier}

EXAMPLES:
- myapp-prod-db-primary
- myapp-staging-ecs-api
- myapp-dev-s3-uploads
- myapp-prod-vpc-main
```

### Tagging Standards

```hcl
# Required tags for all resources
locals {
  common_tags = {
    Project     = var.project_name
    Environment = var.environment
    ManagedBy   = "terraform"
    Owner       = var.team_name
    CostCenter  = var.cost_center
    CreatedAt   = timestamp()
  }
}

resource "aws_instance" "example" {
  # ...
  tags = merge(local.common_tags, {
    Name = "${var.project_name}-${var.environment}-ec2-web"
    Role = "web-server"
  })
}
```

---

## SECURITY CONSTRAINTS

### Forbidden Configurations

```hcl
# FORBIDDEN: Public S3 buckets
resource "aws_s3_bucket_public_access_block" "example" {
  bucket = aws_s3_bucket.example.id

  block_public_acls       = true   # MUST be true
  block_public_policy     = true   # MUST be true
  ignore_public_acls      = true   # MUST be true
  restrict_public_buckets = true   # MUST be true
}

# FORBIDDEN: Open security groups
resource "aws_security_group_rule" "bad" {
  cidr_blocks = ["0.0.0.0/0"]  # NEVER for SSH/RDP
  from_port   = 22
  to_port     = 22
  # This will be REJECTED
}

# FORBIDDEN: Unencrypted databases
resource "aws_db_instance" "bad" {
  storage_encrypted = false  # MUST be true
  # This will be REJECTED
}
```

### Required Security Controls

```
MANDATORY FOR ALL DEPLOYMENTS:

[ ] Encryption at rest enabled
[ ] Encryption in transit enabled
[ ] No public IP addresses (unless explicitly approved)
[ ] Security groups follow least privilege
[ ] IAM roles follow least privilege
[ ] Logging enabled
[ ] Backup enabled for stateful resources
[ ] No hardcoded secrets in .tf files
```

---

## CI/CD INTEGRATION

### Terraform CI Pipeline

```yaml
# .github/workflows/terraform.yml
name: Terraform

on:
  pull_request:
    paths:
      - 'infrastructure/**'
  push:
    branches: [main]
    paths:
      - 'infrastructure/**'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: hashicorp/setup-terraform@v3
      
      - name: Terraform Format Check
        run: terraform fmt -check -recursive
        working-directory: infrastructure
      
      - name: Terraform Init
        run: terraform init -backend=false
        working-directory: infrastructure/environments/production
      
      - name: Terraform Validate
        run: terraform validate
        working-directory: infrastructure/environments/production

  plan:
    needs: validate
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      - uses: hashicorp/setup-terraform@v3
      
      - name: Terraform Plan
        run: terraform plan -no-color
        working-directory: infrastructure/environments/production
      
      - name: Comment Plan on PR
        uses: actions/github-script@v7
        with:
          script: |
            // Post plan output as PR comment

  apply:
    needs: validate
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    environment: production
    steps:
      - uses: actions/checkout@v4
      - uses: hashicorp/setup-terraform@v3
      
      - name: Terraform Apply
        run: terraform apply -auto-approve
        working-directory: infrastructure/environments/production
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              INFRASTRUCTURE STANDARDS QUICK REFERENCE             |
+------------------------------------------------------------------+
| RULE: All infrastructure defined in code (Terraform/Pulumi)      |
| RULE: No manual console changes (ClickOps forbidden)             |
+------------------------------------------------------------------+
| WORKFLOW:                                                         |
|   1. Edit .tf files                                              |
|   2. Create PR                                                   |
|   3. Run terraform plan                                          |
|   4. Review plan (especially DESTROY operations)                 |
|   5. Get approval                                                |
|   6. Apply via CI/CD                                             |
+------------------------------------------------------------------+
| DRIFT DETECTION:                                                  |
|   - Run terraform plan periodically                              |
|   - Unexpected changes = drift                                   |
|   - Investigate and remediate                                    |
+------------------------------------------------------------------+
| STATE FILES:                                                      |
|   - Never commit to git                                          |
|   - Use remote backend with encryption                           |
|   - Enable state locking                                         |
+------------------------------------------------------------------+
| AGENT RULES:                                                      |
|   - Generate .tf code only                                       |
|   - Never run apply without approval                             |
|   - Never access cloud console                                   |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-21
**Line Count:** ~400
