# AI-Powered Development 2026: Part 6 — Building Agents & Tool Use (Chapters 10–11)

## Core Architecture: Planner, Executor, Memory, Evaluator

Building a development agent is not magic. It is software engineering applied to a new domain. The architecture of any effective development agent follows a consistent pattern, whether you are using a framework like LangGraph or building from scratch. Understanding these four components — planner, executor, memory, evaluator — lets you design agents that are maintainable, debuggable, and extensible.

**The Planner:** The planner is the brain of the agent. It takes a high-level goal and decomposes it into actionable steps. The planner must understand the domain (software development), the available tools, and the current state of the project. In simple agents, the planner is a prompt to a language model: "Given the goal and the current state, what is the next step?" In advanced agents, the planner is a dedicated model or a rule-based system that generates structured plans.

A good planner produces plans that are: concrete (specific files and actions), verifiable (each step has a success criterion), recoverable (if a step fails, the plan can be revised), and bounded (the plan does not expand beyond the original goal).

**The Executor:** The executor carries out the steps. It translates plan steps into tool invocations: reading a file, writing a file, running a test, or querying a database. The executor handles the mechanics of tool use: formatting arguments, parsing responses, and handling errors. A robust executor includes retry logic, timeout handling, and graceful degradation when tools are unavailable.

**The Memory:** Memory maintains state across the agent's operation. Short-term memory holds the conversation history and recent observations. Medium-term memory stores working data: files read, code written, test results. Long-term memory contains project knowledge: conventions, patterns, and previous decisions. In 2026, memory is typically implemented as a combination of in-context conversation, local files, and vector databases.

**The Evaluator:** The evaluator verifies that steps completed correctly and that the overall goal is achieved. It runs tests, checks syntax, verifies types, and compares outputs against expectations. The evaluator provides the feedback loop that makes agents self-correcting. Without evaluation, an agent has no way to know it made a mistake.

## Tool Integration: Filesystem, Shell, Web, Browser

The power of an agent comes from its tools. A development agent without tools is just a chatbot. Here is how to integrate the essential tool categories.

**Filesystem Tools:** The most basic and most important. The agent needs to read files, write files, list directories, and search for patterns. Design your filesystem tools with safety in mind: allow writes only within the project directory, require confirmation for deletions, and maintain a change log. "Write file `src/auth.js` with the following content" is a tool call that the agent can make autonomously.

**Shell Tools:** Running commands is essential for testing, building, and deployment. The agent needs to execute shell commands and capture stdout, stderr, and exit codes. Security is paramount here. Never give an agent unrestricted shell access. Restrict commands to a whitelist, sanitize arguments, and run in sandboxed environments. "Run command `npm test -- src/auth.test.js` and report the results."

**Web Search Tools:** Agents need to look up documentation, check package versions, and research APIs. A web search tool lets the agent query search engines and read result summaries. "Search for the latest Express.js authentication middleware best practices" enables the agent to stay current without requiring all knowledge to be embedded.

**Browser Tools:** For web application development, the agent needs to see what the application renders. Browser automation tools let the agent navigate pages, inspect elements, and capture screenshots. "Open `http://localhost:3000/login`, fill in the test credentials, click submit, and verify the redirect to `/dashboard`." This closes the loop between code changes and visual outcomes.

**Designing Tool Schemas:** Each tool needs a clear schema: name, description, parameters, and return type. The schema is what the AI model uses to decide when and how to invoke the tool. Good schemas are specific: "read_file" takes a path and returns contents. "write_file" takes a path and content and returns success/failure. Vague schemas confuse the model and produce unreliable tool use.

## Building with Frameworks vs. Custom Solutions

In 2026, you have a choice: use an existing agent framework or build your own. Both approaches have merits.

**LangChain and LangGraph:** The LangChain ecosystem is the most mature framework for building agents. LangChain provides tool abstractions, prompt templates, and chain compositions. LangGraph adds stateful, cyclic workflows — essential for agents that need to maintain complex state across many steps. LangGraph is the recommended choice for agents with non-trivial planning logic, human-in-the-loop checkpoints, and conditional branching.

**AutoGen:** Microsoft's AutoGen specializes in multi-agent conversations. It is excellent for building teams of agents that debate, critique, and collaborate. If your use case involves multiple specialist agents (a coder, a reviewer, a tester), AutoGen provides the conversation orchestration out of the box.

**CrewAI:** CrewAI emphasizes role-based agent teams with clear workflows. It is more accessible than AutoGen and provides good defaults for common patterns. It is particularly strong for business process automation but increasingly used for development tasks.

**Custom Frameworks:** For teams with specific requirements, building a custom agent framework is viable. A custom agent might be 500-1000 lines of Python using direct API calls to Claude, GPT, or Gemini. The advantage is full control: you define the exact prompt format, tool set, memory structure, and evaluation logic. The disadvantage is maintenance: you own every bug and limitation.

**The Decision Framework:** Use an existing framework if your needs are standard (ReAct loop, file tools, shell tools, web search). Build custom if you need unusual tools (proprietary APIs, custom hardware), specialized evaluation logic, or tight integration with existing infrastructure.

## State Management and Memory

State management is where many agent projects fail. Without careful state design, agents lose track of what they are doing, repeat actions, or make decisions based on stale information.

**Conversation State:** The simplest form of state is the conversation history. Each message (human, AI, tool result) is appended to a list and fed back to the model. This works for short tasks but degrades as the conversation grows. Early messages are compressed or dropped, and the model's attention scatters across too much information.

**Working Memory:** For medium-term state, agents maintain a "scratchpad" — a file or data structure where they record important information. "I have read the auth controller and identified three functions to modify. I have written the token generation logic. The remaining tasks are: write the reset route, add tests, verify manually." The agent reads and updates this scratchpad at each step, keeping its working state organized.

**Vector Memory:** For large projects, agents use vector databases to store and retrieve memories. Each memory (a file content, a decision, a test result) is converted to an embedding and stored. When the agent needs to recall something, it queries the vector DB for semantically similar memories. This scales to thousands of memories without overwhelming the context window.

**Structured State:** In LangGraph and similar frameworks, state is explicitly defined as a data structure. A development agent might have state fields like: `current_plan`, `completed_steps`, `modified_files`, `test_results`, `errors`, and `human_messages`. The framework manages state transitions, ensuring that each step has access to the current state and can update it for the next step.

## Error Recovery and Resilience

Agents fail. They execute wrong commands, write incorrect code, and misunderstand requirements. A production-quality agent must handle failure gracefully.

**Retry Logic:** When a tool call fails (network error, timeout, syntax error), the agent should retry with a modified approach. If a test fails, the agent should examine the error, hypothesize a cause, and attempt a fix. Limit retries to prevent infinite loops: three attempts is a common default.

**Fallback Strategies:** If repeated retries fail, the agent should escalate. Options include: switching to a different tool, asking the human for clarification, marking the task as blocked, or attempting an alternative approach. "The `npm install` failed due to a network timeout. I will retry once. If it fails again, I will ask whether to use a different registry or proceed offline."

**Checkpointing:** Long-running agents should save checkpoints — snapshots of their state — at regular intervals. If the agent crashes or needs to restart, it resumes from the last checkpoint rather than starting over. In LangGraph, checkpointing is built in. In custom agents, implement it by serializing the state to disk after each major step.

**Human Escalation:** Define clear conditions for human escalation. These might include: destructive operations (deleting files, dropping databases), security-sensitive changes (modifying auth, changing secrets), repeated failures (three retries exhausted), ambiguous requirements (the agent cannot determine what the human wants), or policy violations (the proposed change violates a defined rule).

## Agent Extensibility and Plugin Architecture

A well-designed agent is not a monolith. It is a platform that can be extended with new tools, capabilities, and integrations without rewriting core logic.

**Tool Registry:** Maintain a registry of available tools. Each tool is self-describing: it exposes its name, description, parameter schema, and handler function. The planner discovers tools through the registry, and new tools are added by registering them. This plugin architecture lets teams add proprietary tools (internal APIs, custom hardware, enterprise systems) without modifying the agent framework.

**Capability Modules:** Group related tools into capability modules. A "database" module includes tools for querying, migrating, and backing up databases. A "deployment" module includes tools for building containers, pushing to registries, and updating Kubernetes. Modules can be enabled or disabled per environment. The development agent runs with all modules; the CI agent runs with only build and test modules.

**Custom Evaluators:** Extend the evaluator with domain-specific checks. A fintech team might add a "compliance evaluator" that verifies all changes against regulatory requirements. A game studio might add a "performance evaluator" that checks frame rate impact. Evaluators are plugins that register themselves with the agent's evaluation pipeline.

## A Complete Example: Feature Implementation Agent

To make these concepts concrete, let us design a complete feature implementation agent. This agent takes a GitHub issue description and produces a pull request.

**Goal:** "Add password reset functionality to the web application."

**Architecture:**
- **Planner:** A Claude 3.7 Sonnet instance that reads the issue, examines the codebase, and produces a structured plan.
- **Executor:** A Python script with tools for file I/O, shell commands, and git operations.
- **Memory:** A JSON state file tracking plan, progress, and file modifications.
- **Evaluator:** A test runner and linter that verify correctness after each step.

**Execution Trace:**

1. **Planning Phase:** The agent reads the issue, then examines existing auth code (`src/routes/auth.js`, `src/controllers/authController.js`, `src/models/User.js`). It identifies the patterns used and generates a plan:
   - Add `PasswordResetToken` model with expiry
   - Add `POST /auth/reset-request` endpoint
   - Add `POST /auth/reset-confirm` endpoint
   - Integrate with existing email service
   - Add unit tests for token validation
   - Add integration tests for the full flow

2. **Step 1 — Model:** The agent writes `src/models/PasswordResetToken.js` following the existing model patterns (Mongoose if MongoDB, SQLAlchemy if Postgres). It then runs the model tests to verify the schema is valid.

3. **Step 2 — Routes:** The agent adds routes to `src/routes/auth.js`, following the existing controller pattern. It reads the route file, identifies where new routes should be added, and writes the changes.

4. **Step 3 — Controller:** The agent implements the controller methods. It checks the email service interface by reading `src/services/email.js`, then writes `authController.requestPasswordReset` and `authController.resetPassword`. It includes error handling consistent with existing controllers.

5. **Step 4 — Tests:** The agent writes tests in `tests/auth.reset.test.js`. It runs the tests. If they fail, it examines the output, identifies the issue, and fixes it. This loop continues until tests pass.

6. **Step 5 — Integration:** The agent runs the full test suite to ensure the new code does not break existing functionality. It also runs the linter to verify style compliance.

7. **Step 6 — Commit:** The agent commits the changes with a descriptive message: "feat: add password reset functionality. Add token model, request/confirm endpoints, email integration, and tests."

8. **Completion:** The agent reports success, summarizes the changes, and provides a link to the diff for human review.

**Total runtime:** 5-10 minutes for a task that might take a human developer 2-4 hours. The human spends 10-15 minutes reviewing the PR rather than 2-4 hours implementing.

## Actionable Takeaways

- Design agents with four components: planner, executor, memory, evaluator.
- Integrate filesystem, shell, web, and browser tools with clear schemas and safety limits.
- Use frameworks (LangGraph, AutoGen, CrewAI) for standard needs; build custom for special requirements.
- Implement working memory (scratchpads) and structured state for long-running tasks.
- Build in retry logic, fallback strategies, checkpointing, and human escalation paths.
- Design agents as extensible platforms with tool registries and capability modules.
- Start with a concrete example. Build an agent for one specific task before generalizing.
- Measure everything. Track completion rate, correctness, and iteration count from day one.


---

# Chapter 11: Tool Use and Function Calling Deep Dive

## How Function Calling Works Under the Hood

Function calling is the mechanism that transforms a language model from a text generator into an actor. Understanding how it works technically allows you to design better tools, debug failures, and build more reliable agents. In 2026, function calling is available in all major models — Claude, GPT, Gemini, and open-weight alternatives — and has matured from an experimental feature into a production-grade capability.

**The Token Prediction Mechanism:** At its core, function calling is still token prediction. The model is trained to recognize when a user's request implies an action that should be delegated to an external tool. When the model decides to use a tool, it does not "execute" anything internally. Instead, it generates a structured text object — a JSON blob containing the tool name and arguments — which the client application parses and acts upon.

**Constrained Decoding:** Modern models use constrained decoding for function calling. The model is restricted to generating tokens that form valid JSON matching the tool's schema. This dramatically reduces hallucination of non-existent parameters or malformed arguments. The constraint is enforced at the sampling level: the model's probability distribution is masked so that only valid next tokens are allowed.

**The Tool Description Interface:** Models receive tool definitions as part of the system prompt or API call. Each tool is described by a JSON Schema specifying its name, description, and parameters. The description is crucial — it is the only information the model has about what the tool does. A vague description produces unreliable tool selection. A precise description produces accurate selection.

**The Execution Loop:** The typical execution loop is:
1. User sends a request.
2. Client sends the request plus available tool definitions to the model.
3. Model either generates a direct response or selects a tool and generates arguments.
4. If a tool is selected, the client parses the arguments, executes the tool, and receives a result.
5. The client sends the tool result back to the model as a "tool message."
6. The model processes the result and either generates a final response or selects another tool.
7. The loop continues until the model generates a final response or a maximum iteration limit is reached.

## Designing Effective Tool Schemas

The quality of your agent is determined by the quality of its tools. Poorly designed tools confuse the model, produce errors, and limit what the agent can accomplish.

**Name and Description Precision:** Tool names should be verbs that clearly indicate the action. `read_file` is better than `file_reader`. `execute_shell_command` is better than `shell`. The description should explain what the tool does, when to use it, and what it returns. "Read the contents of a file at the given path. Use this when you need to examine existing code or configuration. Returns the file contents as a string."

**Parameter Design:** Parameters should be explicit and constrained. Use enums for parameters with a fixed set of values. Use clear types: string, number, boolean, array, object. Avoid optional parameters when possible — they increase the decision space for the model and can lead to omissions. If a parameter is optional, provide a sensible default and document it.

**Granularity:** Tools should be small and composable rather than large and monolithic. A `read_file` tool and a `write_file` tool are more flexible than a single `modify_file` tool that handles both. Small tools reduce the chance of unintended side effects and make the agent's reasoning more transparent.

**Idempotency:** Design tools to be idempotent where possible. Reading a file twice produces the same result. Writing a file with the same content twice leaves the file in the same state. Idempotent tools are safer because repeated invocations (including accidental retries) do not compound effects.

**Examples:** Include example tool calls in the description if the tool has complex parameters. "Example: `{'path': 'src/auth.js', 'line': 42, 'content': 'const token = generateToken();'}`"

## Compositional Tool Design

The power of an agent comes not from individual tools but from how they compose into workflows. A toolset of five simple tools can produce more complex behavior than a single monolithic tool.

**The Unix Philosophy for Agents:** Each tool should do one thing well. `list_files` lists files. `read_file` reads a file. `grep_code` searches for patterns. `write_file` writes a file. `run_tests` runs tests. Individually, these are trivial. Composed, they enable the agent to navigate, understand, modify, and verify a codebase.

**Workflow Composition:** An agent implementing a feature might compose tools like this:
1. `list_files` on `src/routes/` to understand the routing structure.
2. `read_file` on the auth route to see the existing pattern.
3. `grep_code` for "email" to find the email service integration.
4. `write_file` to create the new route file.
5. `run_tests` to verify the implementation.

Each tool call is a discrete step that the agent can reason about, retry, and verify.

**Tool Chains:** Some operations require chains of tool calls. Creating a database migration requires: reading the current schema, generating the migration script, writing it to the migrations directory, and running it against the test database. Design your agent to recognize these chains and execute them as atomic workflows, with rollback if any step fails.

**Conditional Tool Use:** Advanced agents select tools conditionally based on context. If a file exists, the agent reads it. If it does not exist, the agent creates it. If a test passes, the agent proceeds. If it fails, the agent debugs. This conditional logic requires the agent to reason about state and choose appropriate tools, which is the essence of intelligence.

## Error Handling and Retry Strategies

Tools fail. Networks timeout, files are locked, commands return non-zero exit codes, and APIs rate-limit. An agent that cannot handle tool failures is fragile.

**Error Classification:** Categorize tool errors to determine the appropriate response. Transient errors (network timeout, rate limit) should trigger retries. Permanent errors (file not found, permission denied, invalid arguments) should trigger replanning or human escalation. Logic errors (test failure, lint error) should trigger self-correction.

**Retry with Backoff:** For transient errors, implement exponential backoff with jitter. If a web search fails, wait 2 seconds and retry. If it fails again, wait 4 seconds. Limit total retries to 3-5. "The web search failed with a 429 rate limit. I will retry in 3 seconds."

**Argument Correction:** If a tool fails because of invalid arguments (the model generated a path that does not exist, or a type mismatch), the agent should analyze the error, correct the arguments, and retry. This requires the agent to understand the error message and map it back to the tool schema.

**Graceful Degradation:** If a tool is unavailable, the agent should have fallback options. If the web search tool is down, the agent might use its internal knowledge. If the test runner is unavailable, the agent might perform static analysis instead. Designing for degradation prevents total failure when one component misbehaves.

**Logging and Observability:** Log every tool invocation: the tool name, arguments, result, duration, and any errors. This logging is essential for debugging agent behavior, auditing actions, and identifying systemic issues. If the agent consistently generates invalid arguments for a particular tool, the tool schema or description needs improvement.

## Building Custom Tools for Your Stack

Generic tools (read file, run shell) get you 80% of the way. The remaining 20% requires tools specific to your tech stack, your conventions, and your infrastructure.

**Domain-Specific Tools:** If you use a custom framework or internal library, build tools that abstract its common operations. "Add a GraphQL resolver following our pattern" is a high-level tool that might internally generate boilerplate, add the resolver to the schema, and write a test. This is easier for the agent to use correctly than asking it to manually perform five steps.

**Integration Tools:** Tools that connect to your specific services: your deployment platform, your monitoring system, your ticketing system. "Deploy the current branch to the staging environment" or "Create a Jira ticket for the bug we just found" are tools that make the agent a true participant in your workflow.

**Analysis Tools:** Tools that perform code analysis using your specific standards. "Run the custom linter that enforces our architectural rules" or "Check that all new API endpoints have corresponding documentation in the wiki" are validation tools that go beyond generic tests.

**Tool Building Principles:**
- Start with a manual script that does what you want. Ensure it works reliably.
- Wrap the script in a tool schema with clear parameters and return values.
- Test the tool with the AI: give the agent a task that requires it and verify correct invocation.
- Iterate on the description and parameters based on how the agent uses it.
- Document the tool for other team members.

## Security Boundaries

The most dangerous aspect of agents is their tool access. An agent with unrestricted file write access can delete your codebase. An agent with shell access can run `rm -rf /`. An agent with API access can exfiltrate data. Security boundaries are not optional; they are foundational.

**The Principle of Least Privilege:** Give the agent only the tools it needs for its current task. If the agent is implementing a frontend feature, it does not need database write access. If the agent is writing documentation, it does not need shell access. Restrict tool availability based on the task context.

**Confirmation Gates:** For destructive operations (deleting files, dropping tables, deploying to production), require human confirmation. The agent proposes the action and waits for approval. "I need to delete the old migration file `migrations/001_old.js`. Confirm? (yes/no)"

**Sandboxing:** Run agents in sandboxed environments: containers, virtual machines, or restricted user accounts. Even if the agent goes rogue, the damage is contained. In 2026, most professional agent setups use Docker containers with read-only mounts for the source code and restricted network access.

**Audit Logging:** Log every tool invocation, especially destructive ones. Maintain an immutable audit trail of what the agent did, when, and with what result. This is essential for security reviews and incident response.

**No Secrets:** Agents should never have access to production secrets, API keys, or credentials. If an agent needs to interact with a service, use scoped tokens with minimal permissions, short expiration, and no access to sensitive data.

## Tool Evaluation: Measuring Tool Selection Accuracy

How do you know if your agent is using tools correctly? You measure.

**Selection Accuracy:** For a given task, does the agent choose the right tool? If the agent needs to find where a function is defined, does it use `grep_code` or does it waste time reading unrelated files? Track which tools the agent selects for common tasks and verify correctness.

**Argument Accuracy:** When the agent selects a tool, are the arguments correct? Does `read_file` receive a valid path? Does `write_file` receive well-formed content? Argument errors indicate either poor tool schema design or insufficient agent reasoning.

**Sequence Efficiency:** Does the agent use the minimum necessary tools, or does it make redundant calls? An agent that reads the same file three times is wasting tokens and time. Optimize agent prompts and memory to reduce redundancy.

**Success Rate:** What percentage of tool invocations succeed on the first attempt? A low success rate indicates that either the tools are unreliable (infrastructure problem) or the agent is using them incorrectly (schema or reasoning problem).

## Actionable Takeaways

- Function calling is token prediction with constrained decoding, not true execution. The client application handles execution.
- Design tool schemas with precise names, clear descriptions, and explicit parameters. Use small, composable, idempotent tools.
- Handle errors by classifying them (transient vs. permanent vs. logic) and responding appropriately.
- Build custom tools for your domain, stack, and infrastructure. Generic tools get you started; custom tools get you to production.
- Enforce strict security boundaries: least privilege, confirmation gates, sandboxing, audit logging, and no secrets.
- Measure tool selection accuracy, argument accuracy, sequence efficiency, and success rate. Iterate based on data.
- Log everything. Agent observability is as important as agent capability.


---

