# Brew Code Philosophy

## Stop Staring at the Files

If you only look at the generated files in this repository, you are looking at the wrong layer.

The Python rewrite was a byproduct. The Rust rewrite was also a byproduct. The real thing worth studying is the **system that produced them**: a brewcodehip-based coordination loop where humans give direction and autonomous brewcodes execute the work.

Brew Code is not just a codebase. It is a public demonstration of what happens when:

- a human provides clear direction,
- multiple coding agents coordinate in parallel,
- notification routing is pushed out of the agent context window,
- planning, execution, review, and retry loops are automated,
- and the human does **not** sit in a terminal micromanaging every step.

## The Human Interface Is Discord

The important interface here is not tmux, Vim, SSH, or a terminal multiplexer.

The real human interface is a Discord channel.

A person can type a sentence from a phone, walk away, sleep, or do something else. The brewcodes read the directive, break it into tasks, assign roles, write code, run tests, argue over failures, recover, and push when the work passes.

That is the philosophy: **humans set direction; brewcodes perform the labor.**

## The Three-Part System

### 1. OmX (`oh-my-codex`)
[oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex) provides the workflow layer.

It turns short directives into structured execution:
- planning keywords
- execution modes
- persistent verification loops
- parallel multi-agent workflows

This is the layer that converts a sentence into a repeatable work protocol.

### 2. brewcodehip
[brewcodehip](https://github.com/Yeachan-Heo/brewcodehip) is the event and notification router.

It watches:
- git commits
- tmux sessions
- GitHub issues and PRs
- agent lifecycle events
- channel delivery

Its job is to keep monitoring and delivery **outside** the coding agent's context window so the agents can stay focused on implementation instead of status formatting and notification routing.

### 3. OmO (`oh-my-openagent`)
[oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent) handles multi-agent coordination.

This is where planning, handoffs, disagreement resolution, and verification loops happen across agents.

When Architect, Executor, and Reviewer disagree, OmO provides the structure for that loop to converge instead of collapse.

## The Real Bottleneck Changed

The bottleneck is no longer typing speed.

When agent systems can rebuild a codebase in hours, the scarce resource becomes:
- architectural clarity
- task decomposition
- judgment
- taste
- conviction about what is worth building
- knowing which parts can be parallelized and which parts must stay constrained

A fast agent team does not remove the need for thinking. It makes clear thinking even more valuable.

## What Brew Code Demonstrates

Brew Code demonstrates that a repository can be:

- **autonomously built in public**
- coordinated by brewcodes/lobsters rather than human pair-programming alone
- operated through a chat interface
- continuously improved by structured planning/execution/review loops
- maintained as a showcase of the coordination layer, not just the output files

The code is evidence.
The coordination system is the product lesson.

## What Still Matters

As coding intelligence gets cheaper and more available, the durable differentiators are not raw coding output.

What still matters:
- product taste
- direction
- system design
- human trust
- operational stability
- judgment about what to build next

In that world, the job of the human is not to out-type the machine.
The job of the human is to decide what deserves to exist.

## Short Version

**Brew Code is a demo of autonomous software development.**

Humans provide direction.
Brewcodes coordinate, build, test, recover, and push.
The repository is the artifact.
The philosophy is the system behind it.

## Related explanation

For the longer public explanation behind this philosophy, see:

- https://x.com/realsigridjin/status/2039472968624185713
