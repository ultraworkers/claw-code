# Operational Patterns

> **Self-healing systems.** Build resilience into your applications.

**Related:** [INFRASTRUCTURE_STANDARDS.md](./INFRASTRUCTURE_STANDARDS.md) | [LOGGING_PATTERNS.md](./LOGGING_PATTERNS.md)

---

## Overview

This document establishes operational patterns for building resilient, self-healing applications. These patterns ensure your systems can recover from failures automatically, provide meaningful health information, and degrade gracefully under stress.

**Core Principle:** Assume everything will fail. Plan for it.

---

## HEALTH CHECK PATTERNS

### The /health Endpoint

Every service MUST expose a health check endpoint.

```
HEALTH ENDPOINT REQUIREMENTS:

URL: GET /health or GET /healthz
Response Time: < 200ms
Response Format: JSON

HEALTHY RESPONSE (200 OK):
{
  "status": "healthy",
  "timestamp": "2026-01-21T15:30:00Z",
  "version": "1.2.3",
  "checks": {
    "database": "healthy",
    "cache": "healthy",
    "external_api": "healthy"
  }
}

UNHEALTHY RESPONSE (503 Service Unavailable):
{
  "status": "unhealthy",
  "timestamp": "2026-01-21T15:30:00Z",
  "version": "1.2.3",
  "checks": {
    "database": "unhealthy",
    "cache": "healthy",
    "external_api": "healthy"
  },
  "errors": [
    "Database connection timeout after 5000ms"
  ]
}
```

### Health Check Implementation

```typescript
// health.ts - Health check endpoint

interface HealthCheck {
  name: string;
  check: () => Promise<boolean>;
  timeout: number;
}

const healthChecks: HealthCheck[] = [
  {
    name: 'database',
    check: async () => {
      const result = await db.query('SELECT 1');
      return result !== null;
    },
    timeout: 5000,
  },
  {
    name: 'cache',
    check: async () => {
      await redis.ping();
      return true;
    },
    timeout: 1000,
  },
  {
    name: 'external_api',
    check: async () => {
      const response = await fetch('https://api.stripe.com/health');
      return response.ok;
    },
    timeout: 3000,
  },
];

export async function healthCheck(): Promise<HealthResponse> {
  const results: Record<string, string> = {};
  const errors: string[] = [];
  
  for (const check of healthChecks) {
    try {
      const result = await Promise.race([
        check.check(),
        new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Timeout')), check.timeout)
        ),
      ]);
      results[check.name] = result ? 'healthy' : 'unhealthy';
    } catch (error) {
      results[check.name] = 'unhealthy';
      errors.push(`${check.name}: ${error.message}`);
    }
  }
  
  const isHealthy = Object.values(results).every(r => r === 'healthy');
  
  return {
    status: isHealthy ? 'healthy' : 'unhealthy',
    timestamp: new Date().toISOString(),
    version: process.env.APP_VERSION || 'unknown',
    checks: results,
    ...(errors.length > 0 && { errors }),
  };
}
```

### Liveness vs Readiness

```
TWO TYPES OF HEALTH CHECKS:

LIVENESS (/health/live):
- "Is the process alive?"
- Simple check, always fast
- If fails → Restart the container
- Example: Can the app respond at all?

READINESS (/health/ready):
- "Is the service ready to receive traffic?"
- Checks dependencies (DB, cache, etc.)
- If fails → Remove from load balancer (don't restart)
- Example: Is the database connection established?

Kubernetes uses both:
  livenessProbe:
    httpGet:
      path: /health/live
      port: 3000
    initialDelaySeconds: 10
    periodSeconds: 5
    
  readinessProbe:
    httpGet:
      path: /health/ready
      port: 3000
    initialDelaySeconds: 5
    periodSeconds: 10
```

---

## CIRCUIT BREAKER PATTERN

### Why Circuit Breakers?

```
PROBLEM: Cascading Failures

When an external service (Stripe, OpenAI, etc.) goes down:
1. Your app keeps trying to call it
2. Each call times out (30 seconds)
3. Requests pile up
4. Your app runs out of resources
5. Your app crashes
6. Users blame you, not Stripe

SOLUTION: Circuit Breaker

When external service fails:
1. First few failures → Try normally
2. Too many failures → "Trip" the circuit
3. Circuit is OPEN → Fail immediately (don't wait)
4. After cooldown → Try again
5. If success → "Close" the circuit
6. Back to normal
```

### Circuit Breaker States

```
┌─────────────────────────────────────────────────────────────┐
│                   CIRCUIT BREAKER STATES                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  CLOSED (Normal Operation)                                   │
│    │                                                         │
│    │ Failure threshold exceeded                              │
│    ▼                                                         │
│  OPEN (Fail Fast)                                            │
│    │ - All calls immediately fail                           │
│    │ - No actual API calls made                             │
│    │ - Return fallback or error                             │
│    │                                                         │
│    │ Cooldown period elapsed                                │
│    ▼                                                         │
│  HALF-OPEN (Testing)                                         │
│    │ - Allow ONE request through                            │
│    │                                                         │
│    ├── Success → Back to CLOSED                             │
│    └── Failure → Back to OPEN                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Circuit Breaker Implementation

```typescript
// circuit-breaker.ts

interface CircuitBreakerConfig {
  failureThreshold: number;    // Failures before opening
  successThreshold: number;    // Successes to close from half-open
  timeout: number;             // Cooldown in ms
}

class CircuitBreaker {
  private state: 'CLOSED' | 'OPEN' | 'HALF_OPEN' = 'CLOSED';
  private failures = 0;
  private successes = 0;
  private lastFailureTime: number | null = null;
  
  constructor(private config: CircuitBreakerConfig) {}
  
  async execute<T>(fn: () => Promise<T>, fallback?: T): Promise<T> {
    // Check if circuit should transition from OPEN to HALF_OPEN
    if (this.state === 'OPEN') {
      const timeSinceFailure = Date.now() - (this.lastFailureTime || 0);
      if (timeSinceFailure >= this.config.timeout) {
        this.state = 'HALF_OPEN';
        this.successes = 0;
      } else {
        // Circuit is OPEN - fail fast
        if (fallback !== undefined) return fallback;
        throw new Error('Circuit breaker is OPEN');
      }
    }
    
    try {
      const result = await fn();
      this.onSuccess();
      return result;
    } catch (error) {
      this.onFailure();
      if (fallback !== undefined) return fallback;
      throw error;
    }
  }
  
  private onSuccess() {
    this.failures = 0;
    if (this.state === 'HALF_OPEN') {
      this.successes++;
      if (this.successes >= this.config.successThreshold) {
        this.state = 'CLOSED';
      }
    }
  }
  
  private onFailure() {
    this.failures++;
    this.lastFailureTime = Date.now();
    if (this.failures >= this.config.failureThreshold) {
      this.state = 'OPEN';
    }
  }
  
  getState() {
    return this.state;
  }
}

// Usage
const stripeCircuit = new CircuitBreaker({
  failureThreshold: 5,
  successThreshold: 2,
  timeout: 30000, // 30 seconds
});

async function processPayment(amount: number) {
  return stripeCircuit.execute(
    () => stripe.charges.create({ amount }),
    { success: false, error: 'Payment service temporarily unavailable' }
  );
}
```

---

## RETRY PATTERNS

### Exponential Backoff

```typescript
// retry.ts - Exponential backoff with jitter

interface RetryConfig {
  maxAttempts: number;
  baseDelay: number;
  maxDelay: number;
  jitter: boolean;
}

async function withRetry<T>(
  fn: () => Promise<T>,
  config: RetryConfig
): Promise<T> {
  let lastError: Error | null = null;
  
  for (let attempt = 1; attempt <= config.maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error as Error;
      
      if (attempt === config.maxAttempts) break;
      
      // Calculate delay with exponential backoff
      let delay = Math.min(
        config.baseDelay * Math.pow(2, attempt - 1),
        config.maxDelay
      );
      
      // Add jitter to prevent thundering herd
      if (config.jitter) {
        delay = delay * (0.5 + Math.random());
      }
      
      console.log(`Attempt ${attempt} failed, retrying in ${delay}ms`);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
  
  throw lastError;
}

// Usage
const data = await withRetry(
  () => fetchFromUnstableAPI(),
  {
    maxAttempts: 3,
    baseDelay: 1000,
    maxDelay: 10000,
    jitter: true,
  }
);
```

### Retry vs Circuit Breaker

```
WHEN TO USE EACH:

RETRY:
- Transient failures (network blip)
- Usually succeeds on second try
- Few failures, quick recovery
- Example: Database connection dropped

CIRCUIT BREAKER:
- Persistent failures (service down)
- Won't succeed even with retries
- Many failures, slow recovery
- Example: Third-party API outage

BEST PRACTICE: Use both together
1. Retry handles transient failures
2. If retries keep failing → Circuit breaker trips
3. Circuit breaker prevents retry storms
```

---

## GRACEFUL DEGRADATION

### Fallback Strategies

```typescript
// degradation.ts - Graceful degradation patterns

// PATTERN 1: Default Value Fallback
async function getRecommendations(userId: string): Promise<Product[]> {
  try {
    return await mlService.getPersonalizedRecommendations(userId);
  } catch (error) {
    // Fallback: Return popular products instead
    return await getPopularProducts();
  }
}

// PATTERN 2: Cached Value Fallback
async function getExchangeRate(currency: string): Promise<number> {
  try {
    const rate = await exchangeRateAPI.getRate(currency);
    await cache.set(`rate:${currency}`, rate, 3600); // Cache for 1 hour
    return rate;
  } catch (error) {
    // Fallback: Return cached rate (may be stale)
    const cached = await cache.get(`rate:${currency}`);
    if (cached) {
      console.warn(`Using cached exchange rate for ${currency}`);
      return cached;
    }
    throw error; // No fallback available
  }
}

// PATTERN 3: Feature Toggle Fallback
async function searchProducts(query: string): Promise<SearchResult> {
  if (featureFlags.get('advanced_search')) {
    try {
      return await elasticSearch.search(query);
    } catch (error) {
      console.warn('Elasticsearch failed, falling back to basic search');
    }
  }
  // Fallback: Basic database search
  return await db.products.find({ name: { $regex: query } });
}

// PATTERN 4: Partial Result Fallback
async function getDashboard(userId: string): Promise<Dashboard> {
  const [userResult, ordersResult, recommendationsResult] = await Promise.allSettled([
    getUser(userId),
    getOrders(userId),
    getRecommendations(userId),
  ]);
  
  return {
    user: userResult.status === 'fulfilled' ? userResult.value : null,
    orders: ordersResult.status === 'fulfilled' ? ordersResult.value : [],
    recommendations: recommendationsResult.status === 'fulfilled' 
      ? recommendationsResult.value 
      : [], // Empty array if recommendations fail
    warnings: [
      userResult.status === 'rejected' && 'User data unavailable',
      ordersResult.status === 'rejected' && 'Order history unavailable',
      recommendationsResult.status === 'rejected' && 'Recommendations unavailable',
    ].filter(Boolean),
  };
}
```

---

## RATE LIMITING

### Token Bucket Implementation

```typescript
// rate-limiter.ts

class TokenBucket {
  private tokens: number;
  private lastRefill: number;
  
  constructor(
    private capacity: number,
    private refillRate: number, // tokens per second
  ) {
    this.tokens = capacity;
    this.lastRefill = Date.now();
  }
  
  tryConsume(tokens: number = 1): boolean {
    this.refill();
    
    if (this.tokens >= tokens) {
      this.tokens -= tokens;
      return true;
    }
    
    return false;
  }
  
  private refill() {
    const now = Date.now();
    const elapsed = (now - this.lastRefill) / 1000;
    const tokensToAdd = elapsed * this.refillRate;
    
    this.tokens = Math.min(this.capacity, this.tokens + tokensToAdd);
    this.lastRefill = now;
  }
}

// Usage in middleware
const rateLimiters = new Map<string, TokenBucket>();

function rateLimitMiddleware(req, res, next) {
  const key = req.ip; // or req.user.id for authenticated users
  
  if (!rateLimiters.has(key)) {
    rateLimiters.set(key, new TokenBucket(100, 10)); // 100 burst, 10/sec steady
  }
  
  const bucket = rateLimiters.get(key)!;
  
  if (bucket.tryConsume()) {
    next();
  } else {
    res.status(429).json({
      error: 'Too Many Requests',
      retryAfter: 10,
    });
  }
}
```

### Rate Limit Headers

```typescript
// Always return rate limit information in headers

function setRateLimitHeaders(res, bucket) {
  res.setHeader('X-RateLimit-Limit', bucket.capacity);
  res.setHeader('X-RateLimit-Remaining', Math.floor(bucket.tokens));
  res.setHeader('X-RateLimit-Reset', Date.now() + 60000);
}
```

---

## TIMEOUT PATTERNS

### Request Timeouts

```typescript
// timeout.ts - Timeout wrapper

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  errorMessage: string = 'Operation timed out'
): Promise<T> {
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(() => reject(new Error(errorMessage)), timeoutMs);
  });
  
  return Promise.race([promise, timeoutPromise]);
}

// Usage
const data = await withTimeout(
  fetchExternalAPI(),
  5000,
  'External API did not respond in 5 seconds'
);
```

### Timeout Hierarchy

```
TIMEOUT HIERARCHY (inside → outside):

Database Query:    5 seconds
External API:     10 seconds
Request Handler:  30 seconds
Load Balancer:    60 seconds

RULE: Inner timeouts must be shorter than outer timeouts.
If database times out at 5s, handler should catch and respond
before the 30s handler timeout.
```

---

## OBSERVABILITY

### Metrics to Track

```
MANDATORY METRICS:

1. RED Metrics (per endpoint):
   - Rate: Requests per second
   - Errors: Error rate percentage
   - Duration: Response time (p50, p95, p99)

2. USE Metrics (per resource):
   - Utilization: % of resource used
   - Saturation: Queue depth / backlog
   - Errors: Error count

3. Circuit Breaker Metrics:
   - State (closed/open/half-open)
   - Failure count
   - Success count after half-open

4. Health Check Metrics:
   - Check duration
   - Check result (pass/fail)
   - Last check timestamp
```

### Structured Error Logging

```typescript
// error-logger.ts

function logError(error: Error, context: Record<string, any>) {
  const logEntry = {
    timestamp: new Date().toISOString(),
    level: 'ERROR',
    message: error.message,
    stack: error.stack,
    context: {
      ...context,
      requestId: context.requestId,
      userId: context.userId,
      endpoint: context.endpoint,
    },
    // DO NOT log sensitive data
    // PII must be redacted
  };
  
  console.error(JSON.stringify(logEntry));
}
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              OPERATIONAL PATTERNS QUICK REFERENCE                 |
+------------------------------------------------------------------+
| HEALTH CHECKS:                                                    |
|   /health/live  - Is process alive? (simple)                     |
|   /health/ready - Is service ready for traffic? (full check)     |
+------------------------------------------------------------------+
| CIRCUIT BREAKER:                                                  |
|   CLOSED → Normal operation                                       |
|   OPEN → Fail fast (don't call failing service)                  |
|   HALF-OPEN → Test if service recovered                          |
+------------------------------------------------------------------+
| RETRY:                                                            |
|   Use for: Transient failures                                    |
|   Pattern: Exponential backoff with jitter                       |
|   Limit: Usually 3 attempts max                                  |
+------------------------------------------------------------------+
| GRACEFUL DEGRADATION:                                             |
|   1. Try primary service                                         |
|   2. If fail → Use fallback (cache, default, partial data)       |
|   3. Always serve something useful                               |
+------------------------------------------------------------------+
| TIMEOUTS:                                                         |
|   Database: 5s | External API: 10s | Handler: 30s | LB: 60s      |
|   Inner < Outer (always)                                         |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-21
**Line Count:** ~450
