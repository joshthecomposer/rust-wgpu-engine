---
name: prior-art-research-required
description: Require brief prior-art research before suggesting non-debug code changes. Use when the user asks for design, architecture, or implementation guidance (not debugging), especially for engine/system design decisions.
---
# Prior-Art Research Required

Use this skill when proposing code changes with meaningful design choices and the task is not a debugging/repro investigation.

## When to apply

Apply this before suggesting changes for:
- Design/architecture proposals
- Implementation guidance with trade-offs
- System-level or engine-level patterns

Do not apply for:
- Pure debugging/repro-driven investigations
- Mechanical edits (typos, renames, formatting only)
- Very small, self-evident fixes with no meaningful design choice

## Required workflow

1. Do a quick internet scan for relevant prior art before proposing the change.
2. Prefer evidence-backed sources (real codebases, docs, postmortems) over opinion-only takes.
3. Summarize findings and cite links in the response.

## Source priority

1. Fyrox docs/source/blog (especially architecture-oriented posts)
2. Veloren blog/codebase sections relevant to the topic
3. Other reputable Rust/game-engine sources (Bevy ecosystem, Amethyst archives, etc.)
4. Game dev postmortems/writeups (GDC talks, engine architecture articles)
5. CS papers/textbooks when algorithms/data structures are the focus

## Response format requirement

Include a short `Prior art` section with:
- 2-5 citation bullets (what each source supports and why it is relevant)
- Key constraints/trade-offs implied by that evidence

If evidence is weak, sparse, or contradictory, say so explicitly and propose a conservative default.
