---
title: "<one-line title of the proposal>"
authors: ["<your name or handle>"]
status: "draft"          # draft | accepted | rejected | superseded
phase: "<N or N.N>"      # which phase the RFC targets
discussion: "<link to the discussion thread>"
implementation: "<link to the tracking issue once accepted>"
---

# Summary

One paragraph: what changes, who is affected, why it matters.

# Motivation

What is the problem? Why is the status quo insufficient? Link to
specific user reports, regressions, or design docs where possible.
Cite numbers if you have them (binary size, compile time, runtime
performance, lines of code touched).

# Detailed design

The bulk of the document. Be concrete: write the syntax, the type
signatures, the wire format, the CLI flags, the error messages. Show
the *generated* code or output where it changes. If the proposal
touches multiple phases, list the order of operations.

# Drawbacks

Why might we *not* do this? Cost in compile time, runtime, learning
curve, ecosystem churn, security surface, documentation maintenance.

# Rationale and alternatives

- What other designs did you consider?
- Why this one over each alternative?
- What is the cost of doing nothing?

# Prior art

Cite languages, papers, or projects that have shipped something
similar. Note where mom should deviate and why.

# Unresolved questions

Bullet list of things the RFC explicitly *does not* settle. These
become tracking issues at accept time.

# Migration / compatibility

If this is a breaking change, describe the deprecation path and the
tooling support (`mom fmt --fix`, `mom lint`, etc.). If it is purely
additive, say so.

# Acceptance criteria

What set of tests, benchmarks, or example programs, when green,
proves the RFC is implemented? Reference test files by path.
