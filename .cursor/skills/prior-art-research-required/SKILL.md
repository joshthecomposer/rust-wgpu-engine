---
name: prior-art-research-required
description: Require brief prior-art research before proposing meaningful non-debug design or implementation changes. Use for architecture, engine systems, gameplay systems, ECS/state-machine/animation/physics/rendering design, or implementation guidance where multiple reasonable approaches exist. Do not use for debugging, mechanical edits, or direct code patches that follow an already-chosen design.
---

# Prior-Art Research Required

Use this skill before proposing meaningful non-debug code or architecture changes where there are real design choices.

The goal is not to delay work. The goal is to avoid inventing architecture in a vacuum when existing engines, Rust game projects, or established game-dev patterns already provide useful guidance.

## Apply when the user asks for

- Design or architecture guidance
- Engine/system-level implementation plans
- Gameplay, AI, animation, physics, rendering, ECS, resource, or asset-pipeline patterns
- Refactors where multiple structures or ownership models are plausible
- Trade-off analysis between approaches
- “How should I structure this?” / “What is the right pattern?” / “Is this a good design?”

## Do not apply when the user asks for

- Debugging, bug diagnosis, or repro-driven investigation
- Explaining an error message
- Small Rust syntax fixes
- Mechanical edits, renames, formatting, or simple cleanup
- Filling in code that directly follows a design the user already chose
- Emergency “just give me the patch” requests
- Questions where prior art would add noise instead of helping

## Required workflow

Before proposing a design or meaningful implementation change:

1. Do a quick prior-art scan.
2. Prefer concrete sources over opinion-only posts.
3. Extract only the parts relevant to the user’s problem.
4. Then give a practical recommendation for this codebase.

Do not turn the response into a literature review. Keep the research brief and actionable.

## Source priority

Prefer sources in this order:

1. Fyrox docs/source/blog, especially architecture-oriented material
2. Veloren blog/source/codebase notes relevant to the topic
3. Bevy ecosystem docs, examples, RFCs, or source
4. Other Rust game-engine or ECS projects
5. Game-dev postmortems, GDC talks, engine architecture articles
6. CS papers/textbooks, only when the problem is algorithmic or data-structure-heavy

Prefer:
- Real source code
- Official docs
- Architecture writeups
- Postmortems describing trade-offs
- Mature project examples

Avoid relying mainly on:
- Reddit comments
- Low-context blog posts
- Generic “best practice” articles
- AI-generated summaries
- Sources that do not map to the user’s actual problem

## Response requirements

Include a short `Prior art` section before the recommendation.

The `Prior art` section should contain:

- 2-5 bullets maximum
- A citation/link for each source
- One sentence explaining what the source supports
- One sentence connecting it to the user’s situation, when useful

Then include:

- `Recommendation`
- `Trade-offs`
- `Concrete shape for your codebase`

## If evidence is weak

If the scan does not find strong prior art, say so explicitly.

Use wording like:

> Prior art is thin here, so I would default to the simplest conservative design and avoid adding framework-like machinery until the pressure is obvious.

Then propose the smallest reasonable design.

## If sources disagree

If sources suggest different approaches, summarize the disagreement briefly and choose based on the user’s constraints.

Prioritize:
- Simplicity
- Debuggability
- Small surface area
- Compatibility with the user’s existing ECS/command-buffer/animation/physics architecture
- Avoiding premature abstraction

## Important constraints

Do not recommend large architecture changes unless the prior art clearly supports the complexity.

Do not cite sources performatively. Every citation must directly support a decision or trade-off in the answer.

Do not block the user with research if the task is obviously a debugging task.

Do not replace the user’s existing architecture with a framework pattern unless the benefit is clear.

When in doubt, propose the smallest change that preserves future options.

Prefer recommendations that fit the existing Rust/OpenGL/Rapier engine architecture: ECS storage, command buffer, animator state, behavior tree decision layer, and explicit physics commands.