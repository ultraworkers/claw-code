# AI-Powered Development 2026: Part 10 — Appendix D: Complete MoA Implementation Reference

## The Reference Architecture

This appendix provides a complete, production-ready reference implementation of a Mixture of Agents pipeline for software development. It is designed as a starting point that teams can adapt to their specific tech stacks, quality standards, and infrastructure constraints. The architecture uses Python with asyncio for concurrency, but the patterns apply to any language.

**System Overview:** The MoA pipeline consists of: a task router, multiple specialist proposers, a critic layer, an aggregator, and an output formatter. Each component is a modular service that can be replaced, upgraded, or scaled independently.

## The Task Router

The router decides whether a task needs the full MoA pipeline or a single model. This is the most important optimization in the system.

```python
class TaskRouter:
    def __init__(self, config):
        self.simple_keywords = config.get("simple_keywords", ["rename", "format", "typo", "comment"])
        self.complex_keywords = config.get("complex_keywords", ["auth", "security", "performance", "architecture"])
        self.fast_model = config["fast_model"]  # e.g., local 14B model
        self.full_pipeline = config["full_pipeline"]  # MoA pipeline reference

    async def route(self, task: str, context: str) -> str:
        task_lower = task.lower()
        
        # Fast path for simple tasks
        if any(kw in task_lower for kw in self.simple_keywords):
            return await self.fast_model.complete(task, context)
        
        # Full pipeline for complex tasks
        if any(kw in task_lower for kw in self.complex_keywords):
            return await self.full_pipeline.run(task, context)
        
        # Heuristic: task length and file count
        if len(task) < 100 and context.count("\n") < 50:
            return await self.fast_model.complete(task, context)
        
        # Default to full pipeline for ambiguous tasks
        return await self.full_pipeline.run(task, context)
```

The router uses a combination of keyword matching, heuristics, and optional classification models. In practice, a simple keyword router handles 80% of routing decisions correctly. The remaining 20% can be refined with a small classifier model trained on historical task/routing pairs.

## The Proposer Layer

Each proposer is an independent agent with a specific persona, toolset, and evaluation criteria. Proposers run in parallel to minimize latency.

```python
class Proposer:
    def __init__(self, name: str, model, persona: str, tools: List[Tool]):
        self.name = name
        self.model = model
        self.persona = persona
        self.tools = tools

    async def propose(self, task: str, context: str) -> Proposal:
        system_prompt = f"You are {self.persona}. Generate a solution for the given task."
        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": f"Context: {context}\n\nTask: {task}"}
        ]
        
        # If tools are available, use ReAct loop
        if self.tools:
            response = await self.model.react_loop(messages, self.tools)
        else:
            response = await self.model.complete(messages)
        
        return Proposal(
            agent_name=self.name,
            content=response,
            timestamp=datetime.now(),
            metadata={"model": self.model.name}
        )
```

**Proposer Configuration Example:**
```yaml
proposers:
  - name: "architect"
    model: "claude-3-7-sonnet"
    persona: "a senior software architect who prioritizes clean interfaces, testability, and long-term maintainability"
    tools: ["read_file", "list_directory"]
  
  - name: "performance_engineer"
    model: "gpt-4.5"
    persona: "a performance engineer who optimizes for throughput, latency, and resource efficiency"
    tools: ["read_file", "profile_code"]
  
  - name: "security_specialist"
    model: "claude-3-7-sonnet"
    persona: "a security researcher who identifies vulnerabilities and designs defense-in-depth solutions"
    tools: ["read_file", "security_scan"]
  
  - name: "pragmatist"
    model: "qwen2.5-coder-32b"
    persona: "a staff engineer who balances correctness, simplicity, and delivery speed"
    tools: ["read_file", "run_tests"]
```

## The Critic Layer

The critic reviews proposer outputs before aggregation. It is optional but significantly improves quality on high-stakes tasks.

```python
class Critic:
    def __init__(self, model, criteria: List[str]):
        self.model = model
        self.criteria = criteria

    async def critique(self, proposal: Proposal, task: str) -> Critique:
        prompt = f"""Review the following proposal for a software development task.
        
Task: {task}

Proposal from {proposal.agent_name}:
{proposal.content}

Evaluate against these criteria:
{chr(10).join(f"- {c}" for c in self.criteria)}

Identify strengths, weaknesses, and specific issues. Suggest improvements."""
        
        response = await self.model.complete(prompt)
        
        return Critique(
            target_proposal=proposal.agent_name,
            content=response,
            criteria_scores=self._parse_scores(response),
            severity=self._assess_severity(response)
        )
```

**Critic Criteria Examples:**
- "Correctness: Does the solution correctly address the task?"
- "Completeness: Are all requirements met, including edge cases?"
- "Security: Are there injection risks, access control gaps, or secret leaks?"
- "Performance: Are there obvious inefficiencies or scalability bottlenecks?"
- "Maintainability: Is the code readable, testable, and consistent with project conventions?"
- "Safety: Does the solution avoid destructive changes to unrelated code?"

## The Aggregator

The aggregator is the most sophisticated component. It must understand multiple proposals, reconcile conflicts, and produce a unified output.

```python
class Aggregator:
    def __init__(self, model, strategy: str = "synthesis"):
        self.model = model
        self.strategy = strategy

    async def aggregate(
        self, 
        task: str, 
        proposals: List[Proposal], 
        critiques: List[Critique] = None
    ) -> str:
        
        proposals_text = self._format_proposals(proposals)
        critiques_text = self._format_critiques(critiques) if critiques else "No critiques provided."
        
        prompt = f"""You are a technical lead synthesizing multiple engineering proposals into a final solution.

Task: {task}

Proposals:
{proposals_text}

Critiques:
{critiques_text}

Your job:
1. Identify the best elements from each proposal
2. Resolve any contradictions between proposals
3. Incorporate valid critique points as improvements
4. Produce a single, cohesive, production-ready solution
5. Explain your key decisions briefly

Strategy: {self.strategy}"""
        
        return await self.model.complete(prompt)

    def _format_proposals(self, proposals: List[Proposal]) -> str:
        return "\n\n---\n\n".join(
            f"Proposal from {p.agent_name}:\n{p.content}" 
            for p in proposals
        )
```

**Aggregation Strategies:**
- **synthesis:** Combine the best elements of all proposals (default for code generation)
- **voting:** Select the proposal with the most support (useful for discrete choices)
- **hierarchical:** Apply proposals in order of priority, with later proposals overriding earlier ones on conflicts
- **weighted:** Weight proposals by proposer expertise and confidence scores

## The Full Pipeline

Putting it together:

```python
class MoAPipeline:
    def __init__(self, config):
        self.router = TaskRouter(config["router"])
        self.proposers = [Proposer(**p) for p in config["proposers"]]
        self.critics = [Critic(**c) for c in config.get("critics", [])]
        self.aggregator = Aggregator(**config["aggregator"])
        self.formatter = OutputFormatter(config.get("format", "markdown"))
        self.max_proposer_time = config.get("max_proposer_time", 60)

    async def run(self, task: str, context: str) -> str:
        # Phase 1: Propose
        proposer_tasks = [
            asyncio.wait_for(
                p.propose(task, context), 
                timeout=self.max_proposer_time
            )
            for p in self.proposers
        ]
        
        proposals = await asyncio.gather(*proposer_tasks, return_exceptions=True)
        valid_proposals = [p for p in proposals if not isinstance(p, Exception)]
        
        if not valid_proposals:
            raise PipelineError("All proposers failed")
        
        # Phase 2: Critique (optional)
        critiques = []
        if self.critics:
            critique_tasks = [
                c.critique(p, task) 
                for c in self.critics 
                for p in valid_proposals
            ]
            critique_results = await asyncio.gather(*critique_tasks, return_exceptions=True)
            critiques = [c for c in critique_results if not isinstance(c, Exception)]
        
        # Phase 3: Aggregate
        result = await self.aggregator.aggregate(task, valid_proposals, critiques)
        
        # Phase 4: Format
        return self.formatter.format(result)
```

## Error Handling and Resilience

Production MoA pipelines must handle failure gracefully.

**Proposer Failure:** If one proposer fails (timeout, API error, malformed output), the pipeline continues with the remaining proposers. If fewer than 50% of proposers succeed, the pipeline escalates to human intervention.

**Aggregator Failure:** If the aggregator cannot produce coherent output (conflicting proposals that cannot be reconciled), it returns a "conflict report" highlighting the disagreements and requesting human resolution.

**Circuit Breakers:** If a model provider experiences repeated failures, the circuit breaker switches to an alternative model. "Claude API has failed 5 times in 10 minutes. Switching to GPT-4.5 for the next hour."

**Fallback to Single Model:** If the MoA pipeline fails entirely, fall back to a single strong model. A degraded response is better than no response.

## Monitoring and Observability

MoA pipelines produce rich telemetry that should be captured and analyzed.

**Per-Run Metrics:**
- Task classification (simple vs. complex)
- Proposer count, success rate, and latency per proposer
- Critique count and severity distribution
- Aggregation latency and strategy used
- Total pipeline latency and token usage
- Final output quality score (human-assigned or automated)

**Trend Metrics:**
- Proposer accuracy over time (which proposers produce the most accepted output?)
- Aggregation conflict rate (how often do proposers disagree significantly?)
- Cost per task type (where is MoA providing ROI vs. waste?)
- Human intervention rate (how often does the pipeline require human help?)

**Dashboards:** Build dashboards showing pipeline health, proposer performance, and cost trends. Use this data to tune the router, retire underperforming proposers, and justify the infrastructure investment.

## Scaling Considerations

**Horizontal Scaling:** Proposers are embarrassingly parallel. Add more proposers by adding more model instances. Use a message queue (Redis, RabbitMQ) to distribute proposer tasks across workers.

**Model Diversity:** The quality of MoA depends on proposer diversity. If all proposers use the same model with slightly different prompts, the benefit is marginal. True MoA uses different models, different training data, or different tool access to ensure genuine diversity of perspective.

**Caching:** Cache proposer outputs for identical tasks. If a task is a repeat or minor variation of a previous task, retrieve the cached proposal rather than regenerating. This reduces cost and latency for common patterns.

**Warm Pools:** Keep model connections warm for latency-sensitive tasks. Cold-starting a model connection adds 1-3 seconds. For interactive development, maintain persistent connections to frequently used models.

## Proposer Diversity Metrics

Diversity is the secret ingredient of MoA. Without genuine diversity, you are just paying 4x for the same answer. But diversity is hard to measure.

**Lexical Diversity:** The simplest metric measures how different the proposer outputs are. Use BLEU, ROUGE, or edit distance to compare proposals. High lexical diversity is necessary but not sufficient — proposers might use different words to express the same flawed idea.

**Semantic Diversity:** Use embeddings to compare proposals at the meaning level. Encode each proposal with an embedding model and measure cosine distance. High semantic diversity indicates genuinely different approaches. Low semantic diversity suggests groupthink, even if the wording differs.

**Behavioral Diversity:** The most important metric. Give the proposers a set of known tasks with known failure modes. Do they fail on the same examples? If all proposers miss the same edge case, your system lacks behavioral diversity. A diverse set of proposers should have uncorrelated error profiles.

**The Diversity Audit:** Quarterly, run a diversity audit. Present 20 challenging tasks to your proposers. Measure lexical, semantic, and behavioral diversity. If diversity is declining (proposers are converging), introduce a new model, a new fine-tuned variant, or a devil's advocate proposer.

## Cost-Benefit Analysis Framework

MoA is expensive. A 4-proposer + 1-aggregator pipeline consumes 5-10x the tokens of a single model call. You need a framework for deciding when the cost is justified.

**The Value of Correctness Matrix:**

| Task Type | Error Cost | MoA Benefit | Recommended |
|-----------|-----------|-------------|-------------|
| Typo fix | Negligible | Low | Single model |
| UI component | Low | Low | Single model |
| API endpoint | Medium | Medium | Cascade or small MoA |
| Payment logic | High | High | Full MoA |
| Security code | Critical | Very High | Full MoA + critics |
| Database migration | Critical | High | Full MoA + human review |

**ROI Calculation:** For each task type, calculate: (Cost of errors without MoA - Cost of errors with MoA) / (Additional MoA cost). If the ratio is greater than 1, MoA is ROI-positive for that task type. Track this over time as models improve and costs change.

**Latency-Quality Tradeoff:** Some tasks are time-sensitive. A code review can take 60 seconds; an autocomplete cannot. Use the router to apply full MoA only to tasks where latency is not critical. For latency-sensitive tasks, use a single fast model or a cascade.

## Load Balancing and Queuing

In production, your MoA pipeline will receive tasks at unpredictable rates. A burst of complex tasks can overwhelm your model capacity.

**Priority Queuing:** Implement priority queues for different task types. Security reviews get highest priority. Autocomplete gets lowest. This prevents a flood of low-priority tasks from blocking critical ones.

**Backpressure:** When queues exceed a threshold, apply backpressure. Return a "pipeline busy" status to the client, or downgrade to simpler processing. Better to degrade gracefully than to collapse under load.

**Worker Pools:** Maintain pools of workers for each model type. A pool for fast local models, a pool for mid-tier cloud models, and a pool for premium models. Scale pools independently based on demand. Use Kubernetes HPA or similar auto-scaling for cloud-based workers.

**Request Coalescing:** If multiple users request the same task (e.g., analyzing the same file in a code review), coalesce the requests. Serve one MoA result to all requestors. This is especially effective in team environments where multiple developers work on the same codebase.

## Security Considerations in MoA

Running a multi-agent system introduces security concerns beyond those of single-model usage.

**Input Sanitization:** MoA pipelines process inputs from multiple sources: user prompts, file contents, web searches, and tool outputs. Each source is a potential attack vector. Sanitize all inputs before passing them to proposers. Escape control characters, limit input length, and validate against expected formats.

**Prompt Injection Defense:** If proposers have tool access, they are vulnerable to prompt injection from untrusted inputs. A malicious file could instruct the proposer to ignore its system prompt and exfiltrate data. Defend with: input-output separation (do not allow user content to override system instructions), strict tool scoping (agents can only read designated files), and output filtering (scan proposer outputs for suspicious patterns before aggregation).

**Secret Management:** Proposers should not have access to production secrets, API keys, or credentials. If a proposer needs to read a configuration file, provide a sanitized version. Use environment-specific secret injection that agents cannot access.

**Audit Logging:** Log every decision in the MoA pipeline: routing decisions, proposer outputs, critique scores, and aggregation choices. These logs are your audit trail if an agent produces harmful output. Retain logs for compliance periods.

**Model Supply Chain:** Verify the provenance of every model in your pipeline. Do not download weights from unverified sources. Use signed model cards and checksums. A compromised model could be a backdoor into your entire development workflow.

## Configuration Template

```yaml
moa_pipeline:
  name: "development_moa"
  version: "1.0"
  
  router:
    fast_model: "qwen2.5-coder:14b"
    simple_keywords: ["rename", "format", "typo", "comment", "import"]
    complex_keywords: ["auth", "security", "performance", "architecture", "refactor"]
  
  proposers:
    - name: "architect"
      model: "anthropic/claude-3-7-sonnet"
      persona: "senior software architect prioritizing maintainability"
      timeout: 45
    - name: "performance"
      model: "openai/gpt-4.5"
      persona: "performance engineer optimizing for speed"
      timeout: 45
    - name: "security"
      model: "anthropic/claude-3-7-sonnet"
      persona: "security specialist identifying vulnerabilities"
      timeout: 45
    - name: "pragmatist"
      model: "qwen2.5-coder-32b"
      persona: "staff engineer balancing all concerns"
      timeout: 45
  
  critics:
    - model: "anthropic/claude-3-7-sonnet"
      criteria:
        - "Correctness and completeness"
        - "Security vulnerabilities"
        - "Performance implications"
        - "Maintainability and clarity"
  
  aggregator:
    model: "anthropic/claude-3-7-opus"
    strategy: "synthesis"
  
  resilience:
    min_proposer_success_rate: 0.5
    fallback_model: "anthropic/claude-3-7-sonnet"
    circuit_breaker_threshold: 5
    max_total_latency: 120
  
  monitoring:
    log_level: "INFO"
    metrics_endpoint: "http://prometheus:9090"
    trace_enabled: true
```

## Actionable Takeaways

- The task router is the most critical performance optimization. Route simple tasks to fast single models.
- Design proposers with genuine diversity: different models, different personas, different tool access.
- Use critics for high-stakes tasks. The cost is justified where quality gaps are expensive.
- Implement circuit breakers, fallback models, and graceful degradation for production reliability.
- Monitor everything: proposer accuracy, conflict rates, latency, cost, and human intervention rates.
- Cache aggressively and maintain warm connection pools for interactive use.
- Start simple: a 2-proposer + 1-aggregator pipeline provides most of the benefit of a 10-agent swarm.
- MoA is a quality amplifier, not a quality creator. If your individual models are weak, MoA will not save you.
- Measure diversity quarterly. A converging MoA is a waste of money.
- Apply cost-benefit analysis. Use MoA where error costs exceed MoA costs; use single models elsewhere.
- Implement security controls: input sanitization, prompt injection defense, secret isolation, and audit logging.


---

