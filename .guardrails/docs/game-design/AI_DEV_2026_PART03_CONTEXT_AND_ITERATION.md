# AI-Powered Development 2026: Part 3 — Context & Iterative Development (Chapters 4–5)

## The Token Economy

In 2026, context is the primary currency of AI-assisted development. Not money, not compute, not model size — context. The amount of information you can feed into the model, and how effectively you manage that information, determines the quality of every interaction. Understanding token windows, context allocation, and retrieval strategies separates average AI users from masters.

A "token" is the atomic unit of text processing for large language models. Roughly, one token equals 0.75 words in English, though this varies by language and content type. Code, with its dense punctuation and symbol density, often consumes more tokens per line than natural language. A 200K context window sounds massive, but in a TypeScript or C++ codebase, it might only cover 50,000-80,000 lines of code — substantial, but not infinite.

The models available in 2026 typically offer context windows ranging from 128K to 2 million tokens. Claude 3.7 Sonnet and Opus, GPT-4.5, Gemini 2.5 Pro, and their competitors all support at least 200K tokens of context. But raw capacity is only half the story. How you use that capacity matters more than how much you have.

## Context Window Management Strategies

**The Full-Context Approach:** For small to medium codebases (under 100K lines), you can often include the entire relevant module or service in the context window. This gives the AI complete visibility into relationships between files, shared utilities, and architectural patterns. The advantage is coherence: the AI never makes assumptions about unseen code because it has seen everything.

The limitation is latency and cost. Longer contexts increase processing time and API costs. For simple tasks, feeding the AI 50,000 tokens of context when it only needs 500 is wasteful. Reserve full-context mode for complex refactoring, cross-module integration, and architectural analysis.

**The Focused-Context Approach:** For most day-to-day tasks, you want to provide only the relevant subset of code. This requires identifying which files, functions, and data structures are pertinent to the task. Modern tools automate much of this: Cursor's indexing automatically includes related files, and Claude Code can search and retrieve specific symbols on demand.

The skill here is learning to curate context manually when automatic retrieval fails. When the AI misses a critical dependency or misunderstands a relationship, explicitly adding the missing file to the conversation often fixes the issue. Developing an intuition for what the AI "needs to see" is a core competency.

**The Rolling-Context Approach:** For very long tasks — implementing a feature that takes hours — you cannot maintain the entire conversation history indefinitely. At some point, earlier parts of the discussion fall out of the context window. The rolling-context approach involves periodically summarizing the conversation state, decisions made, and current task status, then starting a "fresh" conversation with that summary as the initial context.

This technique is essential for agentic workflows. An agent working on a multi-step task must maintain state across steps. When context limits approach, the agent should write a checkpoint — a summary of completed work, current file states, and remaining tasks — to a file, then reload from that checkpoint in a new conversation.

## Retrieval-Augmented Generation for Code

Retrieval-Augmented Generation (RAG) is the technique of augmenting the AI's context window with dynamically retrieved information rather than relying solely on the conversation history or a static dump. For codebases, RAG means searching the repository for relevant files, documentation, and examples and injecting them into the prompt.

**How Code RAG Works:** The system indexes the codebase — parsing files, extracting symbols, building embeddings, and constructing a searchable vector database. When you ask a question or request a change, the system performs a semantic search: it converts your query into a vector and finds the most similar vectors in the code index. These matching chunks are then added to the AI's context window as "retrieved context."

**The Advantage:** RAG scales beyond context limits. A repository with a million lines of code cannot fit in any model's context window, but RAG can retrieve the 20 most relevant snippets for any query. This makes AI assistance viable for truly massive projects — Linux kernel development, enterprise monorepos, and large-scale game engines.

**The Limitation:** RAG is only as good as the retrieval. If the search misses a critical file or retrieves irrelevant boilerplate, the AI operates with incomplete or noisy context. RAG struggles with implicit dependencies, runtime configurations, and cross-cutting concerns that are not captured by semantic similarity.

**Improving RAG Quality:**
- Maintain good file organization and naming. RAG systems use paths and filenames as retrieval signals.
- Write module-level documentation. Docstrings and README files are highly retrievable and provide crucial context.
- Use explicit imports and exports. Dynamic imports, reflection, and heavy metaprogramming defeat static analysis.
- Keep related code physically close. RAG retrieves by file chunking; scattering related logic across distant files hurts retrieval accuracy.

## Project-Specific Knowledge Injection

RAG retrieves existing code. But what about knowledge that is not in the code? Business rules, architectural decisions, team conventions, and domain expertise exist in documentation, meeting notes, and tribal knowledge. Project-specific knowledge injection techniques embed this information into the AI's context.

**The Rules Files:** As discussed in Chapter 2, `.cursorrules`, `CLAUDE.md`, and similar files inject static knowledge into every prompt. These files should contain timeless, project-wide conventions that the AI needs for every task.

**The Knowledge Base:** For larger projects, a structured knowledge base provides richer injection. This might be a directory of markdown files covering architecture, deployment procedures, API contracts, and business logic. Tools like Cursor and Claude Code can be configured to automatically include specific knowledge base files based on the task at hand.

**Dynamic Injection via Embeddings:** Advanced setups use vector databases to store project knowledge. When you ask a question about "our billing flow," the system retrieves the most relevant sections from the knowledge base and injects them into the prompt. This scales to thousands of pages of documentation without overwhelming the context window.

**Session-Specific Context:** For tasks that require knowledge not covered by the rules or knowledge base, explicitly provide it at the start of the conversation. "For this session, remember that our billing system uses Stripe with custom invoice logic. The relevant files are `src/billing/` and `src/integrations/stripe.ts`." This temporary context applies to the current task without polluting permanent rules files.

## Keeping Context Fresh Across Sessions

A persistent challenge in AI-assisted development is maintaining continuity. You work with the AI for an hour, make progress, stop for the day, and return tomorrow. How do you restore the context efficiently?

**Conversation History:** Simple chat interfaces maintain conversation threads, but these degrade over long sessions due to context window limits. After 50+ exchanges, early parts of the conversation are effectively invisible to the model. Relying on conversation history for project state is unreliable.

**Checkpoint Files:** The professional approach is to write explicit checkpoints. At natural breakpoints — after completing a subtask, before a significant decision, or when pausing work — write a summary file. This file contains: what was accomplished, what files were modified, what decisions were made, and what remains to do. When resuming, feed this checkpoint to the AI as the opening context.

**Branch-Based Context:** For agentic workflows, use git branches as context anchors. Each agent session works on a dedicated branch. The branch history, commit messages, and diffs provide a concrete record of what the AI did. When resuming, the AI can read the git log and diffs to reconstruct its previous state. This is more reliable than conversational summaries because it is ground-truth data.

**State Files:** Complex agents should write structured state files — JSON or YAML — documenting their plan, current step, tool outputs, and evaluation results. These files serve as both audit logs and resume points. They are the agent equivalent of a developer's notebook.

## Strategies for Monorepos and Massive Codebases

Monorepos present the ultimate context challenge. A repository with 500K lines of code across 50 packages simply does not fit in any context window. Working effectively in these environments requires architectural context management.

**Package-Level Isolation:** Treat each package or module as a self-contained unit. When working on the authentication service, provide context from the auth package and its direct dependencies, but exclude unrelated packages like the marketing site or the analytics pipeline. Modern monorepo tools (Nx, Turborepo, Rush) make these boundaries explicit.

**Interface-First Context:** For cross-package work, provide the interfaces (API contracts, type definitions, shared models) rather than the implementations. The AI needs to know what a function does and what types it accepts, not how it is implemented internally. This dramatically reduces context consumption.

**Generated Summaries:** For packages where you need implementation details, generate and maintain summary files. A "package digest" is a manually or AI-generated document describing the package's purpose, key exports, important internal patterns, and gotchas. These summaries are small (1-2K tokens) but convey the essence of a package that might be 50K tokens in raw code.

**Graph-Based Retrieval:** Advanced tools build dependency graphs of the codebase. When you modify a file, the graph identifies all upstream and downstream dependents. This transitive closure is often the minimal sufficient context for safe refactoring. If you change a shared utility, the graph tells you exactly which files might break.

## Context Compression and Summarization Techniques

When you have more relevant context than the window allows, compression becomes essential. Raw code is information-dense but often redundant. Summarization techniques distill the essential meaning without losing critical details.

**Structural Summarization:** Replace full file contents with structural descriptions. Instead of including a 500-line controller, describe it as: "The AuthController handles login, logout, token refresh, and password reset. It uses AuthService for business logic and JWTService for token operations. Key methods: login(credentials) returns tokens, logout(token) invalidates sessions, refresh(token) issues new tokens, reset(email) sends reset links." This summary conveys the structure in 100 tokens rather than 5,000.

**Hierarchical Context:** Provide context at multiple levels of abstraction. Level 1 is a one-sentence summary of every module. Level 2 is a detailed summary of the modules directly relevant to the task. Level 3 is full content of the most relevant files. The AI can request deeper levels if needed, but most tasks are resolved at Level 1 or 2. This tiered approach mimics how human developers navigate large codebases.

**Diff-Based Compression:** When modifying existing code, include only the diff, not the full file. "Here is the current function signature and the proposed change. Review the diff for correctness." This is especially effective for review tasks where the reviewer needs to focus on changes, not existing code. Diff compression can reduce context by 80-90% for modification tasks.

**Token Budgeting:** Allocate your context window deliberately. Reserve 20% for the system prompt and project rules. Reserve 30% for task-specific context. Reserve 40% for conversation history. Keep 10% as headroom for the AI's response. If a task requires more context than your budget allows, it should be broken into subtasks. Discipline in budgeting prevents context overflow and the degradation of output quality that accompanies it.

## The Future: Infinite Context vs. Intelligent Retrieval

Two competing paradigms are emerging for solving the context problem.

**Infinite Context:** Model providers are racing to expand context windows. 2 million tokens is already available in 2026, and 10 million token models are in research. The infinite-context vision is that you simply feed the AI your entire codebase, documentation, and conversation history, and it reasons about everything at once. This is appealingly simple and works well for moderately sized projects.

**Intelligent Retrieval:** The alternative vision accepts that even infinite context is wasteful. Most of a million-line codebase is irrelevant to any specific task. Intelligent retrieval uses query understanding, dependency analysis, and user intent modeling to retrieve exactly the context needed. This is harder to build but more scalable and cost-effective.

In practice, 2026 is a hybrid era. You use large context windows for tasks where coherence matters (refactoring across modules, architectural reviews), and intelligent retrieval for everyday tasks where precision matters (implementing a feature, debugging a specific issue). Mastering both approaches gives you the flexibility to work effectively at any scale.

## Actionable Takeaways

- Context is your primary resource. Manage it deliberately, not haphazardly.
- Use full-context mode for complex cross-module work, focused-context for daily tasks.
- Write checkpoint summaries when pausing long AI-assisted sessions.
- Maintain project rules files and knowledge bases for consistent knowledge injection.
- In monorepos, work at package boundaries and use interface-first context.
- Do not rely on conversation history for project state. Use git branches and state files.
- RAG scales beyond context limits but requires good code organization to be effective.
- Combine large context windows with intelligent retrieval based on task type.
- Apply structural summarization and hierarchical context to fit more information into limited windows.
- Use diff-based compression for review and modification tasks.
- Budget your context window across system, task, history, and response allocations.


---

# Chapter 5: The Iterative Development Loop with AI

## Planning Before Generating

The most dangerous trap in AI-assisted development is impatience. The AI generates code so quickly that it is tempting to skip planning and dive straight into prompting. This produces code that compiles but does not compose — functions that work in isolation but fail to integrate, features that satisfy the immediate request but break existing workflows, and architectures that scale poorly because no one thought about the next iteration.

Planning with AI is different from traditional planning. In traditional development, you plan because implementation is expensive. In AI-assisted development, implementation is cheap but correction is expensive. A bad plan executed instantly creates a mess that takes hours to untangle. A good plan executed by the AI produces coherent, maintainable code on the first attempt.

**The Planning Prompt:** Before asking the AI to implement anything significant, ask it to plan. "We need to add OAuth2 authentication to our REST API. Before writing code, outline the approach: which endpoints to add, what middleware changes are needed, which existing code to refactor, and what tests to write." This planning phase costs a few thousand tokens and a minute of your time. It saves you from discovering, after the AI has rewritten six files, that it chose an incompatible OAuth library.

**Plan Review Checklist:** When the AI returns a plan, review it for:
- Does it understand the existing architecture?
- Are the proposed changes minimal and focused?
- Does it account for edge cases (token expiry, refresh flows, error handling)?
- Are the testing and validation steps adequate?
- Does it introduce dependencies or changes beyond the scope?

Only after the plan passes review should you proceed to implementation. This two-phase approach — plan, then execute — is the hallmark of experienced AI-assisted developers.

## Breaking Work into Atomic, Verifiable Tasks

The AI works best on tasks that are concrete, bounded, and verifiable. "Implement the entire user dashboard" is too large. The AI will generate hundreds of lines across multiple files, and you will struggle to review it effectively. Instead, break the work down:

1. Create the route and controller skeleton
2. Implement the data fetching logic
3. Build the React component with mock data
4. Connect the component to the API
5. Add loading and error states
6. Write tests for the data layer
7. Write tests for the UI layer

Each task is small enough to review in five minutes. Each task produces a verifiable outcome: the route responds, the component renders, the tests pass. If the AI goes wrong on task 3, you catch it early rather than discovering the issue after 500 lines of dependent code have been written.

This atomic approach mirrors traditional agile methodology, but the velocity is different. In traditional development, breaking tasks down is overhead because each task requires manual implementation. In AI-assisted development, the AI implements each atom in seconds, so the overhead of decomposition is negligible compared to the benefit of early error detection.

## Test-Driven Development with AI

Test-driven development (TDD) and AI are a natural pairing. The AI can generate tests based on specifications, then generate implementation to satisfy those tests. This flips the traditional TDD workflow: instead of you writing tests, the AI writes both tests and implementation under your direction.

**The AI-TDD Loop:**
1. You specify the behavior: "We need a function that validates email addresses according to RFC 5322."
2. The AI generates a test suite covering valid emails, invalid emails, edge cases, and boundary conditions.
3. You review the tests. Do they cover the right cases? Are the assertions correct?
4. The AI implements the function.
5. The tests run. If they pass, great. If they fail, the AI debugs based on the test output.
6. You review the final implementation.

This workflow is powerful because tests provide an objective correctness criterion. The AI cannot hallucinate passing tests — the test runner is ground truth. This constrains the AI's output to verifiably correct code, at least with respect to the test coverage.

**The Limitation:** AI-generated tests reflect the AI's understanding of the requirements, which may be incomplete. If you do not specify that internationalized email addresses should be supported, the AI will not test for them. AI-TDD amplifies specification quality: great specs produce great tests and great code; vague specs produce shallow tests and incomplete code.

## Incremental Commits and Rollbacks

When working with AI agents that modify multiple files, git becomes your safety net and your audit log. The cardinal rule of AI-assisted development is: commit after every significant AI action. Not after a day of work. Not after a feature is complete. After every coherent unit of AI-generated change.

**Commit Granularity:** A good AI-assisted commit history looks like:
- `ai: add user authentication controller and routes`
- `ai: implement JWT token generation and validation`
- `ai: add auth middleware to protected endpoints`
- `human: review and fix token expiry handling`
- `ai: add integration tests for auth flow`
- `human: approve tests, fix edge case in refresh token`

This granularity serves multiple purposes. It makes rollbacks precise: if the AI's third action broke something, you revert only that commit. It makes review manageable: each commit is a reviewable unit. It creates an audit trail: you can see exactly which changes were AI-generated and which were human-corrected.

**Branching Strategy:** Use dedicated branches for AI experimentation. Never let an AI agent work directly on your main branch. A common pattern is `ai/feature-name-attempt-N` branches. If the AI goes down a bad path, you delete the branch and start fresh. If it succeeds, you squash or merge after human review.

**Rollback Discipline:** When the AI produces bad output, do not try to fix it inline. Revert to the last good commit and try again with a better prompt. Developers often waste hours patching bad AI output when a clean rollback and re-prompt would have solved the problem in minutes. Be ruthless about discarding bad generations.

## Working with AI-Generated Code at Scale

As you become proficient with AI tools, you will encounter a new problem: volume. An AI agent can generate more code in an hour than you can comfortably review. How do you maintain quality when the AI is producing faster than you can read?

**Heuristic Review:** You cannot read every line of a 2,000-line AI generation in detail. Instead, use a tiered review strategy:
- **Structure review:** Does the file organization make sense? Are the right files created in the right places?
- **Interface review:** Do the exported functions have correct signatures? Are the types right? Do the contracts match the requirements?
- **Sample review:** Pick a few representative functions and read them in detail. Do they handle errors? Are the algorithms correct?
- **Test review:** Are the tests comprehensive? Do they actually exercise the code? Do they pass?
- **Integration review:** Does the new code compile and run? Does it integrate with existing code without breaking things?

If all five heuristics pass, the generation is probably good. If any fail, dig deeper. This is not perfect — a subtle bug can slip through heuristic review — but it scales to large generations in a way that line-by-line review does not.

**Diff Review:** Modern tools present AI changes as diffs. Reading diffs is faster than reading full files because you focus only on what changed. Train yourself to read diffs critically. Ask: Why was this line removed? Why was this added? Does this change preserve the original intent?

**Automation Assistance:** Use static analysis tools to augment your review. Linters, type checkers, and security scanners catch issues that human review misses. In 2026, AI-assisted review tools (secondary AI systems that review primary AI output) are increasingly common. They catch bugs, style violations, and security issues before human review begins.

## Collaborative Iteration: The Pair Programming Model

The most productive AI-assisted developers in 2026 do not treat the AI as a code generator; they treat it as a pair programming partner. This shift in mindset changes everything about the interaction.

**Conversational Development:** Instead of one-shot prompts, engage in a conversation. "I am thinking about implementing this feature with a state machine. What are the tradeoffs compared to a rules engine?" The AI responds with analysis. You ask follow-ups. "Good point about complexity. Given our team size is three developers, which approach is more maintainable?" The conversation refines the approach before any code is written.

**The Socratic Loop:** Use the AI to question your assumptions. "I plan to use Redis for session storage. What could go wrong?" The AI lists: single point of failure, data loss on restart, network latency, operational complexity. You address each concern in your design. This loop prevents the blind spots that come from working in isolation.

**Rubber Duck Debugging with Intelligence:** Traditional rubber duck debugging involves explaining your problem to an inanimate object, which forces you to articulate assumptions. An AI rubber duck is animate — it responds with questions, suggestions, and alternative perspectives. "Explain why this race condition happens." Your explanation reveals the bug before the AI even comments.

**Shared Ownership:** In true pair programming, both partners own the code. When the AI generates code, you are not "accepting its output"; you are "co-authoring a solution." This means you should feel free to modify, reject, or redirect the AI at any point. The AI has no ego. It will not be offended if you discard its third attempt and ask for a completely different approach.

## When to Stop Iterating

A subtle skill in AI-assisted development is knowing when to stop. The AI will keep iterating as long as you keep prompting. It will refactor your refactor, optimize your optimizations, and add features you did not ask for if your prompts are loose. Knowing when a task is "good enough" prevents over-engineering.

**The Definition of Done:** Before starting a task, define what "done" means. "The endpoint returns correct data for happy path and common error cases. Tests cover 80% of branches. No new dependencies." When the AI's output meets these criteria, stop. Do not let it "improve" the code further unless you have a specific improvement in mind.

**The Diminishing Returns Curve:** The first AI generation gets you 70% of the way there. The second iteration (your review and follow-up prompts) gets you to 90%. The third iteration might get you to 95%. Chasing 100% perfection through endless AI iteration is usually less efficient than accepting 95% and manually polishing the remaining edge cases yourself.

**The Human Override:** There comes a point in every AI-assisted task where manual intervention is faster than another prompt. If you find yourself prompting the AI five times to fix a specific edge case, just write the fix yourself. The AI is a productivity multiplier, not a replacement for direct manipulation when the path is clear.

## Actionable Takeaways

- Always ask the AI to plan before implementing. Review the plan carefully.
- Break work into atomic, verifiable tasks. Do not ask for multi-file features in a single prompt.
- Use AI-TDD: have the AI write tests first, then implementation.
- Commit after every significant AI action. Use dedicated branches.
- Revert bad generations rather than patching them. Be ruthless.
- Use heuristic review for large generations: structure, interface, sample, test, integration.
- Treat the AI as a pair programming partner, not a code vending machine.
- Use conversational development and Socratic questioning to refine approaches before coding.
- Define "done" before starting. Stop iterating when criteria are met.
- Know when to override manually. Do not prompt five times for a fix you could write in two minutes.


---

