# AI-Powered Development 2026: Part 5 — Legacy Refactoring & Agent Paradigm (Chapters 8–9)

## The Archaeology Problem

Legacy code is the greatest challenge in software engineering. Not because it is old, but because it is opaque. Code without documentation, without tests, without living authors, and without clear intent is a puzzle where the pieces have been partially melted. In 2026, AI tools have transformed legacy archaeology from a manual excavation into an assisted investigation.

**Understanding Undocumented Code:** The first step in working with legacy code is understanding what it does. AI excels at this if you provide the right inputs. Feed the AI a legacy module and ask: "Explain what this code does in business terms. What is the intended behavior? What are the inputs and outputs? What external systems does it interact with?" The AI reverse-engineers intent from implementation, often identifying patterns and logic that are not immediately obvious.

**Dependency Mapping:** Legacy systems often have hidden dependencies — database tables that are accessed through string interpolation, APIs called through reflection, configuration loaded from environment variables with undocumented names. The AI can trace these if you provide broad context. "Here is the entire `src/legacy/` directory. Identify all external dependencies: databases, APIs, files, environment variables, and shared memory. Map each dependency to the code that uses it."

**Code Summarization:** For very large legacy files (thousands of lines), AI summarization is essential. "Summarize this 3,000-line file. Break it into logical sections, describe the purpose of each section, identify the key functions and their roles, and flag any code that appears dead, redundant, or especially complex." This produces a navigable map of a file that would take hours to read manually.

**Identifying Hotspots:** Legacy codebases usually have a small number of files that cause most of the problems. The AI can help identify these hotspots by analyzing complexity metrics, change frequency (if you have git history), and error logs. "Analyze these logs and identify which legacy modules are most frequently associated with production errors. Correlate with code complexity to identify the highest-priority refactoring targets."

## Safe Refactoring Patterns with AI

Refactoring legacy code is dangerous because the safety nets — tests, documentation, and original authors — are often missing. AI can assist, but the approach must be conservative and verifiable.

**The Characterization Test Strategy:** Before refactoring legacy code, use AI to generate characterization tests. These are not tests that verify correct behavior (you may not know what correct behavior is), but tests that capture the current behavior. "This function takes inputs and produces outputs. Write tests that document the current behavior for a range of inputs: typical cases, edge cases, and any inputs you can find in the production logs." Once you have characterization tests, you can refactor and verify that behavior has not changed.

**Incremental Refactoring:** Never let the AI refactor a large legacy module in a single pass. The risk of subtle breakage is too high. Instead, break the refactoring into steps:
1. Extract functions and rename variables (no behavior change)
2. Add type annotations or interfaces (no behavior change)
3. Replace magic numbers with constants (no behavior change)
4. Extract classes and modules (verified by tests)
5. Replace algorithms (only after thorough testing)

At each step, run characterization tests and integration tests to verify preservation of behavior.

**The AI as Refactoring Assistant:** Use the AI for mechanical refactoring that humans find tedious but that is low-risk: renaming variables consistently, extracting repeated logic into functions, converting callbacks to async/await, and applying linting rules. "Rename all variables in this file to match our naming convention: camelCase for variables, PascalCase for classes, SCREAMING_SNAKE_CASE for constants. Ensure all references are updated." The AI handles the mechanical work; you verify the semantic preservation.

**Type System Rescue:** For untyped legacy code (JavaScript, Python, Ruby), adding types is one of the safest and highest-value refactorings. The AI can add TypeScript types to JavaScript, type hints to Python, or Sorbet signatures to Ruby. "Add TypeScript types to this JavaScript module. Preserve all runtime behavior. Add interfaces for the data structures. Mark any ambiguous types as `unknown` rather than `any`." The type system then becomes a safety net for future changes.

## Incremental Modernization Strategies

Sometimes refactoring is not enough. The legacy system needs to be modernized: new language version, new framework, new architecture. AI makes incremental modernization feasible by handling the translation and boilerplate.

**Strangler Fig Pattern:** The strangler fig pattern involves incrementally replacing a legacy system by routing traffic through a new system while the old system is still running. The AI can help implement the routing layer. "Write an API gateway layer that routes requests to either the legacy monolith or the new microservice based on the endpoint. For endpoints implemented in the new service, route there. For legacy endpoints, proxy to the monolith. Include circuit breakers and fallback logic."

**Module-by-Module Migration:** For framework upgrades (e.g., Angular 12 to Angular 18, Django 3 to Django 5), migrate one module at a time. The AI translates patterns from the old framework to the new. "Here is a Django 3 view using function-based views. Rewrite it for Django 5 using class-based views and the modern request/response handling. Maintain the same URL routing, authentication checks, and query logic." This module-by-module approach limits risk and allows parallel operation.

**Data Migration:** Data migration is often the hardest part of modernization. The AI can generate migration scripts, validation logic, and rollback procedures. "We are migrating user data from a legacy SQL schema to a new normalized schema. Write a migration script that: reads from the old tables, transforms the data, writes to the new tables, and validates row counts and checksums. Include a rollback script and a verification query."

## Generating Documentation for Legacy Systems

Documentation is the gift legacy code never received. AI can generate it retroactively, transforming opaque code into maintainable systems.

**API Documentation:** For legacy APIs without documentation, the AI can generate OpenAPI specs from code analysis. "Here is the routing code and request handlers for our legacy REST API. Generate an OpenAPI 3.0 specification documenting all endpoints, parameters, request bodies, and response codes. Infer types from the code where possible."

**Inline Documentation:** The AI can add docstrings, JSDoc, or XML documentation to legacy functions. "Add comprehensive docstrings to all public functions in this module. Document parameters, return values, exceptions, and side effects. Include a brief description of the function's purpose and any important caveats." This documentation helps future maintainers without changing behavior.

**Architecture Documentation:** For legacy systems where no architecture documentation exists, the AI can reconstruct it from code analysis. "Analyze this codebase and produce an architecture document describing: the overall structure, major components, data flow, external integrations, and deployment topology. Include diagrams in Mermaid format." This is never perfect — the AI may miss runtime configuration or deployment scripts — but it provides a starting point that would take weeks to produce manually.

## Dealing with Technical Debt at Scale

Technical debt in legacy systems is not a single problem but a spectrum. AI helps across the spectrum, from superficial debt to structural debt.

**Surface Debt:** Code smells, naming inconsistencies, formatting violations, and outdated comments. This is the easiest to address. The AI can clean it systematically: "Standardize all naming in this module to our style guide. Fix formatting. Remove dead code. Update comments that refer to removed features." Surface debt is low-risk and high-visibility. Cleaning it improves morale and makes deeper debt more visible.

**Structural Debt:** Monolithic modules, tight coupling, circular dependencies, and mixed concerns. This requires careful refactoring. The AI can assist by: identifying coupling points through static analysis, proposing extraction boundaries, and generating the new module structures. "This 5,000-line module handles authentication, authorization, and user management. Propose a decomposition into three separate modules with clear interfaces." The AI generates the proposal; the team decides whether to implement it.

**Architectural Debt:** Wrong technology choices, outdated frameworks, or mismatched paradigms. This is the hardest debt to address because it often requires rewriting rather than refactoring. The AI can help plan the rewrite: "We need to migrate from our custom ORM to Prisma. Analyze all database access points, map them to Prisma equivalents, and generate a migration plan with effort estimates." The AI cannot make the strategic decision to migrate, but it can dramatically reduce the planning and execution cost.

**Debt Quantification:** Use the AI to quantify technical debt. "Analyze this codebase and estimate the effort to resolve each category of debt: surface (days), structural (weeks), architectural (months). Identify which debt is actively causing bugs or slowing development." This quantification helps prioritize debt repayment against feature work.

## The Psychological Dimension of Legacy Work

Legacy work is not just technically challenging; it is psychologically draining. Developers often resist working on legacy code because it feels like cleaning someone else's mess with no recognition. AI changes the psychology by making legacy work faster and more rewarding.

**The Satisfaction of Understanding:** When the AI helps a developer understand a legacy module in minutes rather than days, the frustration turns to satisfaction. The developer feels competent rather than lost. This psychological shift makes legacy assignments less dreaded.

**The Reward of Transformation:** AI-assisted modernization produces visible, dramatic improvements. A module goes from untyped and untested to typed, tested, and documented in a week. The developer sees tangible progress, which is deeply motivating.

**The Confidence of Safety:** Characterization tests and type systems provide safety nets that reduce the anxiety of touching legacy code. Developers are more willing to refactor when they trust the safety nets. AI-generated safety nets build that trust quickly.

## Case Study: Modernizing a 100K Line Codebase

To make these concepts concrete, let us walk through a realistic case study: modernizing a 100,000-line JavaScript e-commerce monolith built in 2018.

**Phase 1: Discovery (Week 1):** The AI analyzes the entire codebase, producing: a dependency map, a complexity report, a list of external services, and a summary of each major module. The team reviews these artifacts and identifies priorities: the checkout flow is the most critical and most fragile; the admin dashboard is the least critical; the product catalog is stable but slow.

**Phase 2: Safety Net (Weeks 2-3):** The AI generates characterization tests for the checkout flow. These tests are not pretty — they mock external services aggressively and test at a high level — but they capture current behavior. The team runs these tests in CI and verifies they pass against production data snapshots.

**Phase 3: Type Safety (Weeks 4-6):** The AI adds TypeScript types to the checkout module. This is done incrementally: first the data models, then the utility functions, then the service layer, then the controllers. At each step, the characterization tests verify no behavior change. By week 6, the checkout module is fully typed, catching dozens of potential null reference and type mismatch bugs.

**Phase 4: Refactoring (Weeks 7-10):** With types and tests in place, the AI assists with structural refactoring. The monolithic checkout controller is split into: a cart service, a pricing service, a payment orchestrator, and an order creator. Each extraction is small, tested, and verified. The AI generates the new service skeletons; the team reviews and adjusts.

**Phase 5: Optimization (Weeks 11-12):** The AI analyzes the product catalog queries and proposes indexing and query restructuring. The team applies the safest optimizations first (adding database indexes) and measures performance. Query time drops 60%. The AI then proposes caching strategies, which the team implements with careful cache invalidation logic.

**Phase 6: Documentation (Week 13):** Throughout the process, the AI generates documentation: ADRs for each major decision, API docs for the refactored services, and runbooks for the deployment process. By the end, the codebase has more documentation than it has had in five years.

Total time: 13 weeks for a 100K-line modernization with a team of three developers and AI assistance. Without AI, this project would have been estimated at 9-12 months and likely abandoned due to risk. With AI, the mechanical work is accelerated, the documentation is generated, and the team focuses on verification and judgment rather than typing.

## Actionable Takeaways

- Use AI for legacy code archaeology: summarization, dependency mapping, and hotspot identification.
- Always establish a safety net before refactoring: characterization tests, types, or feature flags.
- Refactor incrementally. Never let AI modernize a large legacy module in a single pass.
- Use AI for mechanical refactoring (renaming, typing, extraction) while you verify semantic preservation.
- Generate missing documentation retroactively: API specs, inline docs, and architecture overviews.
- Apply modernization patterns like strangler fig to limit risk during large transitions.
- Focus team effort on verification and judgment; let AI handle mechanical translation and boilerplate.
- Quantify technical debt to prioritize repayment against feature work.
- Recognize the psychological benefits of AI-assisted legacy work: faster understanding, visible transformation, and confidence from safety nets.


---

# Chapter 9: From Copilot to Agent — The Paradigm Shift

## What Makes an Agent

The transition from copilot to agent is the most significant conceptual leap in AI-assisted development. A copilot suggests; an agent acts. A copilot waits for your prompt; an agent pursues a goal. A copilot generates text; an agent manipulates the world. Understanding this distinction is essential because the design patterns, failure modes, and human responsibilities are fundamentally different.

An agent in 2026 is defined by four capabilities: **autonomy, tool access, memory, and planning.**

**Autonomy** means the agent can operate without continuous human prompting. You give it a goal — "implement user authentication" — and it breaks that goal into subtasks, executes them, and reports progress. It does not stop after each subtask and ask "what next?" unless it encounters an ambiguity or blocker that requires human judgment.

**Tool access** means the agent can interact with external systems: read and write files, execute shell commands, query databases, call APIs, and control browsers. Without tools, the AI is a conversational engine trapped in a text box. With tools, it becomes an operative that can actually do work.

**Memory** means the agent maintains state across actions. It remembers what it has done, what it has learned, and what remains to do. Memory can be short-term (within a single conversation), medium-term (across conversations via files or databases), or long-term (embedded knowledge about the project, team, and domain).

**Planning** means the agent can decompose goals into actionable steps, sequence those steps, and revise the plan based on feedback. Planning is the difference between a scripted bot and an intelligent agent. A scripted bot follows a fixed sequence. An agent adapts when circumstances change.

## The ReAct Pattern: Reasoning and Acting

The ReAct pattern (Reasoning + Acting) is the foundational architecture for modern agents. It was introduced in 2023 and has become the standard approach for building systems that interleave thinking and doing.

**How ReAct Works:** The agent operates in a loop. At each step, it receives an observation (the current state of the world, which might include the contents of a file, the output of a command, or a human message). It then reasons about what to do next — considering the goal, the progress so far, and the available tools. Finally, it acts by invoking a tool or generating a response. The result of that action becomes the next observation, and the loop continues.

**Example ReAct Trace:**

*Observation:* "You need to add a password reset feature. The project uses Express.js with MongoDB."

*Reasoning:* "To implement password reset, I need to: 1) add a route for requesting a reset, 2) generate a secure token, 3) store the token with an expiry, 4) add a route for validating the token and updating the password, 5) send an email with the reset link. I should start by examining the existing auth routes to understand the pattern."

*Action:* Read file `src/routes/auth.js`

*Observation:* [contents of auth.js showing login and register routes]

*Reasoning:* "The auth routes use a controller pattern with `authController.login` and `authController.register`. I should add `authController.requestPasswordReset` and `authController.resetPassword`. Let me examine the controller to understand the structure."

*Action:* Read file `src/controllers/authController.js`

This loop continues until the task is complete. The reasoning steps are often visible to the human operator, providing transparency into the agent's decision-making.

**The Power of ReAct:** The ReAct pattern handles unexpected situations gracefully. If a file does not exist where the agent expects it, the reasoning step notices the discrepancy and adjusts. If a test fails, the reasoning step analyzes the failure and plans a fix. This interleaving of reasoning and action is what makes agents robust in dynamic environments.

**The Limitations:** ReAct is not perfect. Agents can get stuck in loops, reasoning about the same problem without making progress. They can pursue dead ends, exploring approaches that cannot work. And they can hallucinate tool outputs — imagining what a command would return rather than actually executing it. The loop must include escape mechanisms: maximum iteration limits, human intervention triggers, and progress verification.

## Plan-and-Execute Frameworks

While ReAct interleaves planning and execution, plan-and-execute frameworks separate them. The agent first generates a complete plan, then executes it step by step. This approach is useful for tasks where the overall strategy matters more than local adaptivity.

**The Planner:** Given a goal, the planner generates a structured plan: a sequence of steps with dependencies, expected outcomes, and verification criteria. "Plan: implement password reset. Step 1: add token model. Step 2: add request route. Step 3: add reset route. Step 4: add email service integration. Step 5: write tests. Verification: all tests pass, manual test of the flow succeeds."

**The Executor:** The executor takes each step and implements it, often using a ReAct loop internally. The executor reports success, failure, or partial completion for each step.

**The Monitor:** A separate component (or the planner itself) reviews execution results. If a step fails, the monitor decides whether to retry, replan, or escalate to human intervention. "Step 3 failed because the token validation logic has a timing bug. Options: A) retry with a fix, B) modify the plan to use a different token strategy, C) ask the human for guidance."

**Frameworks:** LangGraph, CrewAI, and AutoGen all provide plan-and-execute abstractions. LangGraph represents plans as state machines with nodes (steps) and edges (transitions). CrewAI uses role-based agents where a "planner" agent delegates to "executor" agents. AutoGen supports multi-agent planning through group chats where agents propose and critique plans.

## The Spectrum: Copilot to Agent to Swarm to Autonomous System

AI-assisted development exists on a spectrum of autonomy. Understanding where you are on this spectrum helps you choose the right tools and set appropriate expectations.

**Level 1: Suggestion (Copilot):** The AI suggests completions and answers questions. The human decides, implements, and verifies. This is the most common mode in 2026. It is safe, predictable, and requires minimal setup.

**Level 2: Directed Action (Task Agent):** The AI performs a specific, bounded task under human direction. "Add a new API endpoint for user search with these parameters." The human reviews the result. This is the sweet spot for most development work in 2026: high productivity with manageable risk.

**Level 3: Delegated Goal (Goal Agent):** The AI pursues a higher-level goal with limited supervision. "Implement the password reset feature." The agent plans, executes, tests, and iterates. The human intervenes only for approvals, blockers, or final review. Claude Code and Aider operate at this level.

**Level 4: Coordinated Swarm (Multi-Agent):** Multiple agents collaborate, each with a specialty. A planner agent designs the approach, a coder agent implements, a reviewer agent critiques, and a tester agent verifies. The human orchestrates the swarm and resolves conflicts. This is the MoA (Mixture of Agents) paradigm that we will explore in depth in Part IV.

**Level 5: Autonomous System (Self-Directed):** The system operates with minimal human intervention over extended periods. It monitors the codebase, identifies issues, proposes improvements, implements fixes, and deploys them subject to automated verification. Humans set policy and handle exceptions. This is the frontier of 2026, available only to the most sophisticated teams.

## Evaluating Agent Effectiveness

As you adopt agentic tools, you need metrics to evaluate their performance. Without measurement, you cannot improve.

**Task Completion Rate:** The percentage of assigned tasks that the agent completes without human intervention. A completion rate of 80% means the agent handles most tasks autonomously but needs help for one in five. Track this by task type: boilerplate tasks might have 95% completion, while complex refactoring might have 40%.

**Correctness Rate:** Of the tasks the agent completes, what percentage are correct on first submission? This measures the quality of the agent's output. An agent with high completion but low correctness is generating technical debt faster than manual coding.

**Iteration Count:** How many back-and-forth cycles does a task require? Fewer iterations mean the agent understands your intent better. If simple tasks require five rounds of corrections, your prompts, rules files, or agent configuration need improvement.

**Time to Completion:** Wall-clock time from task assignment to human approval. Compare this to manual implementation time. An agent that takes 30 minutes to complete a task that would take 2 hours manually is a 4x speedup — unless the review takes 90 minutes, in which case it is a wash.

**Human Satisfaction:** The subjective measure: do developers trust and enjoy working with the agent? High satisfaction correlates with adoption and proper use. Low satisfaction leads to workarounds, avoidance, and "I could have done it faster myself" syndrome.

## When Agents Fail and Why

Agents fail for predictable reasons. Recognizing these failure modes helps you design around them.

**Ambiguous Goals:** If the task description is vague, the agent will interpret it in ways you did not intend. "Improve the codebase" is a recipe for disaster. The agent might reformat everything, change naming conventions, or refactor working code without improving anything meaningful.

**Insufficient Context:** Agents operate with limited visibility. If critical information is not in their context window or retrievable via tools, they make incorrect assumptions. An agent working on a frontend task might not know about backend constraints that limit the API design.

**Tool Misuse:** Agents can invoke tools incorrectly: deleting the wrong file, running a destructive command, or querying the production database instead of staging. Tool design must include safeguards: confirmation prompts for destructive actions, environment restrictions, and dry-run modes.

**Infinite Loops:** An agent can get stuck retrying a failing approach indefinitely. "The test failed because of a null pointer. I will add a null check. The test still fails because the null check is in the wrong place. I will move the null check. The test still fails..." Loop detection and escalation mechanisms are essential.

**Hallucinated Success:** An agent might claim a task is complete when it is not. It might run tests on the wrong files, check for the wrong criteria, or misinterpret output. Verification must be objective and external, not based on the agent's self-assessment.

**Scope Creep:** An agent given a narrow task might expand its scope, "fixing" unrelated issues it notices along the way. This is often well-intentioned but creates unpredictable changes that require extensive review.

## Human-Agent Collaboration Patterns

The most effective teams in 2026 have developed explicit collaboration patterns that define how humans and agents share work. These patterns are not accidental; they are designed, documented, and refined.

**The Delegation Pattern:** The human defines the goal and acceptance criteria. The agent plans, executes, and reports. The human reviews and approves. This is the standard pattern for feature implementation. It works best when the goal is clear, the context is complete, and the stakes are moderate.

**The Partnership Pattern:** The human and agent work simultaneously on different aspects of the same task. The human designs the API contract while the agent implements the endpoint. The human writes the business logic while the agent writes the tests. This parallel execution requires careful coordination to avoid conflicts but can halve implementation time.

**The Escalation Pattern:** The agent handles routine cases and escalates exceptions to the human. "I can fix 47 of these 50 linting violations automatically. The remaining 3 require semantic understanding of the business logic. Please review them." This pattern maximizes agent utility while reserving human judgment for the cases that need it.

**The Review Pattern:** The agent generates; the human reviews. This is the simplest pattern and the most common. It works for code generation, documentation, test creation, and configuration. The key discipline is that the human must actually review — not skim, not trust, not rubber-stamp. A review pattern with negligent review is worse than no agent at all.

**The Teaching Pattern:** The human corrects the agent's output and explains the reasoning. The agent updates its memory (rules files, knowledge base, or project context) to incorporate the feedback. Over time, the agent requires less correction. This pattern requires investment but produces agents that understand your specific codebase better than any off-the-shelf tool.

## Actionable Takeaways

- An agent requires autonomy, tools, memory, and planning. Missing any of these creates a fragile system.
- The ReAct pattern is the foundation: interleave reasoning and action in a loop.
- Plan-and-execute frameworks work best for complex tasks requiring upfront strategy.
- Know your autonomy level. Most teams in 2026 should operate at Level 2 or 3, not 4 or 5.
- Measure agent performance: completion rate, correctness, iterations, time, satisfaction.
- Guard against failure modes: ambiguous goals, missing context, tool misuse, loops, hallucinated success, and scope creep.
- Design explicit collaboration patterns: delegation, partnership, escalation, review, and teaching.
- Invest in the teaching pattern for long-term gains. The best agents are those that have learned from your corrections.
- Design tool safeguards and escalation paths before deploying agents to real codebases.


---

