---
title: "Recruiting production users at 3+ organisations"
authors: ["mom-core"]
status: "draft"
phase: "7"
discussion: "TBD"
implementation: "TBD"
---

# Summary

Process for converting `mom` from "interesting language with a green
test suite" to "language with at least three independent organisations
running it in production". This is the Phase 7 acceptance row that
fundamentally **cannot** be satisfied inside the repository — it's a
go-to-market and developer-relations RFC, not a code RFC.

# Motivation

`docs/roadmap.md` lists "Production users at 3+ organisations" as a
Phase 7 deliverable with acceptance "case studies, public reference".
Until this is real, the language's claim to be a "modern, self-hosted
systems language for enterprise-scale software" remains a marketing
sentence rather than an observable fact.

# Detailed design

Three phases of outreach, each tracked outside this repo:

1. **Lighthouse user (1 org)** — find a single team with
   high-stakes infrastructure (preferably an internal-tools team at a
   company that already loves Rust) willing to try mom for one
   genuine production workload. Required artifacts before pitching:
   - Phase 5 tooling complete (✅, this repo).
   - Phase 6 stdlib good enough for a non-trivial service (✅
     stage-0 + stretch; native parity per RFC #0001).
   - A 1-pager comparing mom to Rust on compile-time, binary size,
     onboarding curve, and runtime characteristics for one realistic
     micro-benchmark.
   - Per-incident SLA: 24h response from a named maintainer.

2. **Reference customers (2-3 orgs)** — once the lighthouse is
   stable, recruit two more. Use the lighthouse's case study as
   the primary marketing artifact. Channels:
   - Conference talks at Strange Loop / Handmade Seattle / GopherCon
     (cross-pollination from other systems-language audiences).
   - Cross-posts on lobste.rs, /r/programming, Hacker News with the
     lighthouse case study, not language announcements.
   - Direct outreach to teams that publicly use Rust + Go and have
     written about their pain points.

3. **Case studies** — for each adopter, publish a 1500-word
   write-up under `docs/case-studies/<org>/<year>-<topic>.md` covering:
   - The problem they were solving.
   - Why they chose mom over the alternatives.
   - One quantitative result (latency, throughput, headcount on the
     project, or onboarding time).
   - One thing that didn't work and how they worked around it.

# Drawbacks

- This is the highest-effort, lowest-leverage Phase 7 row. There is no
  way to short-cut it.
- It binds the project to a calendar (~12-18 months minimum from
  Phase 6 completion) regardless of how quickly the engineering work
  ships.

# Rationale and alternatives

- Skip "production users" as an acceptance criterion: ship `mom 1.0`
  based purely on technical readiness. Loses the credibility that
  comes from external validation; risks shipping a 1.0 that nobody
  actually uses.
- Use internal Anthropic-equivalent teams as the lighthouse: works
  only for a project hosted inside a single company. For a
  community-owned language, the social commitment to external
  adoption is the entire point.

# Acceptance criteria

The Phase 7 row flips from ⏳ external to ✅ shipped when:
- `docs/case-studies/` contains at least three `.md` files, one per
  organisation, each linked from `README.md`'s "Users" section.
- The lighthouse user is named in `MAINTAINERS.md` (or equivalent) as
  a non-maintainer reference.
- `mom 1.0` is tagged.
