# Container and Deployment Security Audit

**Repository:** /mnt/ollama/git/agent-guardrails-template
**Focus:** mcp-server/deploy/
**Audit Date:** 2026-02-08
**Auditor:** Security Engineer

---

## Executive Summary

This audit reviews container and deployment security configurations for the MCP server deployment. Overall, the configurations demonstrate strong security practices with distroless base images, non-root execution, capability dropping, and network isolation. Several medium and low severity findings are identified with actionable remediation steps.

**Overall Security Rating:** GOOD (8.2/10)

---

## 1. Dockerfile Security Analysis

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/Dockerfile`

### 1.1 Non-Root User Usage (UID 65532)

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| Non-root user | USER 65532:65532 configured | PASS |
| UID consistency | Matches distroless nonroot user | PASS |
| File ownership | --chown=65532:65532 applied to binary | PASS |

**Details:**
- Line 38: `USER 65532:65532` correctly sets non-root user
- Line 41: Binary ownership set to non-root user during copy
- Uses distroless static nonroot image which provides no shell access

### 1.2 Minimal Base Image (Distroless)

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| Base image | gcr.io/distroless/static:nonroot | PASS |
| Image size | Minimal attack surface | PASS |
| Shell access | None available | PASS |

**Details:**
- Line 31: Uses `gcr.io/distroless/static:nonroot` - excellent choice
- No package manager, shell, or unnecessary utilities
- Reduces attack surface significantly compared to alpine/full images

### 1.3 Multi-Stage Build

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| Multi-stage | Build and final stages separated | PASS |
| Build tools excluded | Only binary and certs in final image | PASS |
| Dependency cleanup | go mod verify executed | PASS |

**Details:**
- Lines 2-28: Build stage with golang:1.23-alpine
- Lines 31-59: Final stage with distroless
- Build artifacts not present in production image

### 1.4 No Sensitive Files Copied

**Status:** LOW RISK

| Aspect | Finding | Severity |
|--------|---------|----------|
| Source copy | COPY . . copies all files (line 14) | LOW |
| Go modules | Only go.mod/go.sum copied first | PASS |
| .dockerignore | Not verified in repository | INFO |

**Details:**
- Line 14: `COPY . .` copies entire source directory
- Build context should be filtered via .dockerignore
- Recommendation: Ensure .dockerignore excludes sensitive files

**Remediation:**
```dockerfile
# Add .dockerignore to exclude:
# - .git/
# - *.md
# - .env*
# - tests/
# - deploy/
# - docs/
```

---

## 2. Podman/Docker Compose Security

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml`

### 2.1 Read-Only Filesystem Settings

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| read_only | Enabled for mcp-server (line 180) | PASS |
| tmpfs mounts | /tmp with noexec,nosuid (line 189-190) | PASS |
| Writable paths | Only tmpfs volumes for temporary data | PASS |

**Details:**
- Line 180: `read_only: true` prevents filesystem modifications
- Lines 189-190: tmpfs with security flags `noexec,nosuid,size=100m,mode=1777`
- Database volumes properly isolated for persistent storage

### 2.2 Capability Dropping

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| cap_drop ALL | Applied to all services | PASS |
| cap_add minimal | Only required capabilities added | PASS |
| no-new-privileges | Enabled for all services | PASS |

**Details:**
- Lines 24-25, 75-76, 184-185: `cap_drop: - ALL` across all services
- Lines 26-28: Redis adds only SETGID, SETUID
- Lines 78-80: Postgres adds CHOWN, SETGID, SETUID
- Lines 182-183: MCP server has no capability additions with `no-new-privileges:true`

### 2.3 Security Contexts

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| User mapping | user: "65532:65532" (line 181) | PASS |
| Group consistency | Matches Dockerfile USER directive | PASS |

### 2.4 Secret Mounting Practices

**Status:** MEDIUM RISK

| Aspect | Finding | Severity |
|--------|---------|----------|
| Environment variables | Secrets passed via env vars | MEDIUM |
| Redis password | Shell interpolation for config (lines 9-15) | MEDIUM |
| File-based secrets | Not implemented | INFO |

**Details:**
- Secrets (DB_PASSWORD, REDIS_PASSWORD, JWT_SECRET, API keys) passed as environment variables
- Lines 9-15: Redis password embedded in shell command
- No Docker secrets or mounted secret files used

**Remediation:**
```yaml
# Use Docker/Podman secrets for production:
secrets:
  db_password:
    file: ./secrets/db_password.txt

# Mount in container:
environment:
  DB_PASSWORD__FILE: /run/secrets/db_password
```

### 2.5 Network Isolation

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| Network segmentation | frontend and backend networks | PASS |
| Internal network | backend marked internal:true (line 233) | PASS |
| Port binding | 127.0.0.1 only for MCP ports (lines 114-115) | PASS |

**Details:**
- Lines 228-234: Separate frontend and backend networks
- Line 233: Backend network `internal: true` - no external access
- Lines 114-115: Services bind to localhost only (127.0.0.1)
- Database and Redis isolated to backend network

---

## 3. Kubernetes Manifests Security

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/k8s-deployment.yaml`

### 3.1 Pod Security Contexts

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| runAsNonRoot | true (line 30) | PASS |
| runAsUser | 65532 (line 31) | PASS |
| runAsGroup | 65532 (line 32) | PASS |
| fsGroup | 65532 (line 33) | PASS |

**Container-level securityContext:**
- Line 147: `allowPrivilegeEscalation: false` - prevents privilege escalation attacks
- Line 148: `readOnlyRootFilesystem: true` - immutable root filesystem
- Lines 149-151: `capabilities: drop: - ALL` - no capabilities granted

### 3.2 Resource Limits

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| Memory requests | 512Mi (line 141) | PASS |
| Memory limits | 2Gi (line 144) | PASS |
| CPU requests | 500m (line 142) | PASS |
| CPU limits | 2000m (line 145) | PASS |

**Additional Controls:**
- Lines 262-299: HorizontalPodAutoscaler configured
- Lines 301-310: PodDisruptionBudget ensures availability

### 3.3 Security Policies

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| PodDisruptionBudget | minAvailable: 1 (line 307) | PASS |
| Rolling update | maxUnavailable: 0 (line 15) | PASS |
| Image pull policy | Always (line 37) | INFO |

### 3.4 Secret Management

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| SecretKeyRef | All secrets via Kubernetes secrets | PASS |
| No hardcoded secrets | No plaintext credentials | PASS |
| Secret separation | Separate secrets for DB, Redis, JWT, API keys | PASS |

**Secrets referenced:**
- Lines 65-88: guardrail-db-credentials
- Lines 91-105: guardrail-redis-credentials
- Lines 108-117: guardrail-api-keys
- Lines 118-122: guardrail-jwt-secret

### 3.5 Network Policies

**Status:** MEDIUM RISK

| Aspect | Finding | Severity |
|--------|---------|----------|
| NetworkPolicy defined | Yes (lines 218-260) | PASS |
| Ingress rules | Restricted to ingress-nginx namespace | PASS |
| Egress rules | Defined but allow all DNS (lines 255-260) | MEDIUM |
| Missing CNI | Policy enforcement requires CNI plugin | INFO |

**Details:**
- Lines 230-239: Ingress only from ingress-nginx namespace
- Lines 240-254: Egress allowed to postgres (5432) and redis (6379)
- Lines 255-260: DNS egress to any destination (required but broad)

**Remediation:**
```yaml
# Consider restricting DNS to specific namespaces:
- to:
    - namespaceSelector:
        matchLabels:
          kubernetes.io/metadata.name: kube-system
  ports:
    - protocol: TCP
      port: 53
```

### 3.6 Health Checks and Probes

**Status:** COMPLIANT

| Aspect | Finding | Severity |
|--------|---------|----------|
| Liveness probe | exec probe using /server --health-check (lines 152-160) | PASS |
| Readiness probe | httpGet to /health/ready (lines 161-168) | PASS |
| Startup probe | httpGet to /health/live (lines 169-176) | PASS |
| Probe timeouts | Appropriate timeouts configured | PASS |

### 3.7 Missing Security Configurations

**Status:** LOW RISK

| Aspect | Finding | Severity |
|--------|---------|----------|
| seccomp profile | Not specified | LOW |
| AppArmor | Not specified | LOW |
| SELinux options | Not specified | LOW |
| PodSecurityContext supplementalGroups | Not specified | LOW |

**Remediation:**
```yaml
# Add to securityContext:
seccompProfile:
  type: RuntimeDefault

# Or for stricter control:
seccompProfile:
  type: Localhost
  localhostProfile: profiles/guardrail-mcp.json
```

---

## 4. Findings Summary

### Critical Findings (0)

None identified.

### High Severity Findings (0)

None identified.

### Medium Severity Findings (3)

| ID | Finding | File | Location | Remediation |
|----|---------|------|----------|-------------|
| MED-001 | Secrets passed via environment variables | podman-compose.yml | Lines 116-179 | Implement Docker/Podman secrets or mounted files |
| MED-002 | Redis password in shell command | podman-compose.yml | Lines 9-15 | Use Redis ACL file or Kubernetes secrets |
| MED-003 | DNS egress allows all destinations | k8s-deployment.yaml | Lines 255-260 | Restrict DNS to kube-system namespace |

### Low Severity Findings (3)

| ID | Finding | File | Location | Remediation |
|----|---------|------|----------|-------------|
| LOW-001 | COPY . . may include unnecessary files | Dockerfile | Line 14 | Implement .dockerignore |
| LOW-002 | seccomp profile not specified | k8s-deployment.yaml | Lines 146-151 | Add seccompProfile: RuntimeDefault |
| LOW-003 | Image pull policy Always may cause issues | k8s-deployment.yaml | Line 37 | Consider IfNotPresent with pinned tags |

### Informational Findings (2)

| ID | Finding | File | Location | Recommendation |
|----|---------|------|----------|----------------|
| INFO-001 | NetworkPolicy requires CNI plugin | k8s-deployment.yaml | Lines 218-260 | Ensure Calico/Cilium/Weave is installed |
| INFO-002 | Health check commented in Dockerfile | Dockerfile | Lines 49-57 | Consider adding HEALTHCHECK for standalone use |

---

## 5. Compliance Mapping

### CIS Docker Benchmark

| Control | Status | Notes |
|---------|--------|-------|
| 4.1 - Create user for container | PASS | UID 65532 used |
| 4.6 - Add HEALTHCHECK | N/A | Orchestrator handles health checks |
| 4.9 - Use COPY instead of ADD | PASS | COPY used exclusively |
| 4.10 - Content trust | INFO | Verify image signatures in CI |

### CIS Kubernetes Benchmark

| Control | Status | Notes |
|---------|--------|-------|
| 5.2.1 - Minimize admission of privileged containers | PASS | allowPrivilegeEscalation: false |
| 5.2.2 - Minimize sharing host PID | PASS | Not specified (default false) |
| 5.2.3 - Minimize sharing host IPC | PASS | Not specified (default false) |
| 5.2.4 - Minimize sharing host network | PASS | Not specified (default false) |
| 5.2.5 - Minimize admission of containers with allowPrivilegeEscalation | PASS | Explicitly false |
| 5.2.6 - Minimize admission of containers with added capabilities | PASS | ALL capabilities dropped |
| 5.3.2 - Network policies | PASS | Policy defined and restrictive |
| 5.4.1 - Resource quotas | PASS | Resource limits defined |

### NIST SP 800-190 (Container Security)

| Control | Status | Notes |
|---------|--------|-------|
| Image vulnerabilities | INFO | Implement image scanning in CI |
| Registry security | INFO | Use private registry with authentication |
| Orchestrator access | PASS | Network policies restrict traffic |
| Container runtime | INFO | Consider gVisor/Kata for additional isolation |

---

## 6. Recommendations

### Immediate Actions (High Priority)

1. **Implement .dockerignore** (LOW-001)
   - Exclude .git/, tests/, docs/, deploy/ from build context
   - Reduces image size and attack surface

2. **Add seccomp profiles** (LOW-002)
   - Use `RuntimeDefault` as minimum
   - Consider custom profile for additional hardening

### Short-term Improvements (Medium Priority)

3. **Migrate to file-based secrets** (MED-001)
   - Use Docker secrets or Kubernetes Secrets
   - Prevents secrets from appearing in process lists

4. **Restrict DNS egress** (MED-003)
   - Limit DNS queries to kube-system namespace
   - Reduces exfiltration vectors

### Long-term Hardening (Low Priority)

5. **Implement runtime security**
   - Add Falco or similar for runtime threat detection
   - Monitor for anomalous container behavior

6. **Enable image signing**
   - Use Sigstore/cosign for image attestation
   - Verify signatures in deployment pipeline

7. **Consider sandboxed runtimes**
   - Evaluate gVisor or Kata Containers for high-security environments
   - Provides additional kernel isolation

---

## 7. Verification Commands

### Dockerfile Security Verification

```bash
# Check for root user in image
docker run --rm guardrail-mcp:latest id
# Expected: uid=65532 gid=65532

# Verify no shell available
docker run --rm guardrail-mcp:latest /bin/sh
# Expected: executable file not found

# Scan image for vulnerabilities
trivy image guardrail-mcp:latest
grype guardrail-mcp:latest
```

### Kubernetes Security Verification

```bash
# Check pod security context
kubectl get pod -n guardrail -o yaml | grep -A 10 securityContext

# Verify network policies
kubectl get networkpolicy -n guardrail
kubectl describe networkpolicy guardrail-mcp-server -n guardrail

# Check resource limits
kubectl top pod -n guardrail
kubectl describe pod -n guardrail | grep -A 5 Limits

# Verify secrets are not in environment
kubectl exec -n guardrail deployment/guardrail-mcp-server -- env | grep -i password
# Expected: Should show file references, not actual secrets
```

### Podman Security Verification

```bash
# Inspect container capabilities
podman inspect guardrail-mcp-server | jq '.[0].EffectiveCaps'
# Expected: empty array []

# Check read-only filesystem
podman exec guardrail-mcp-server touch /test
# Expected: Read-only file system error

# Verify user
podman exec guardrail-mcp-server id
# Expected: uid=65532 gid=65532
```

---

## 8. References

- [CIS Docker Benchmark v1.6.0](https://www.cisecurity.org/benchmark/docker)
- [CIS Kubernetes Benchmark v1.8.0](https://www.cisecurity.org/benchmark/kubernetes)
- [NIST SP 800-190 - Application Container Security Guide](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-190.pdf)
- [OWASP Container Security Verification Standard](https://owasp.org/www-project-container-security-verification-standard/)
- [Kubernetes Security Best Practices](https://kubernetes.io/docs/concepts/security/)
- [Distroless Images - GoogleContainerTools](https://github.com/GoogleContainerTools/distroless)

---

**Document Owner:** Security Team
**Review Cycle:** Quarterly
**Next Review:** 2026-05-08
**Version:** 1.0
