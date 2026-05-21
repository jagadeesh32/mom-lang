# mom — Risks & Mitigations

This is the honest list of things that could blow the project up, and
how we propose to handle each. Each item carries a **likelihood** (L)
and **impact** (I) on a 1–5 scale, and a **mitigation owner**.

| # | Risk                                                                 | L | I | Mitigation                                                                                              | Owner            |
|---|----------------------------------------------------------------------|---|---|---------------------------------------------------------------------------------------------------------|------------------|
| 1 | Simpler ownership model proves either too weak or too restrictive     | 3 | 5 | Implement borrow + region + actor isolation incrementally; ship Phase 2 behind a feature flag; benchmark against Rust on real workloads | Compiler frontend |
| 2 | C++ interop consumes endless engineering time                         | 4 | 4 | Constrain the supported subset (curated classes, vtables, `std::string/vector/optional`); reject features instead of perpetually adding | Backend           |
| 3 | Async + actors + low-level systems push runtime size beyond budget    | 3 | 4 | Strict featurization: `std-core` builds without any runtime; profile binary size in CI; refuse PRs that bloat the no-feature build | Runtime           |
| 4 | Self-hosting too early slows semantic iteration                       | 3 | 4 | Keep stage-0 alive until language is in production at 3+ orgs; do not retire Rust bootstrap before 1.0  | Tech lead         |
| 5 | Build-speed promise (Zig-tier) misses target                          | 3 | 5 | Query-based incremental compiler from day 1; content-addressed disk cache; benchmark every PR against a 1 MLOC fixture | Backend           |
| 6 | LLVM update breaks reproducibility / bit-identical self-host          | 3 | 3 | Pin LLVM version per release; reproducibility job in CI flags drift; vendor LLVM build scripts          | Backend           |
| 7 | Standard library grows past maintainability                           | 3 | 3 | Strict triage: anything that can live in the registry does; `std` covers OS interface + concurrency only | Stdlib lead       |
| 8 | LSP performance regressions block editor adoption                     | 2 | 4 | Reuse compiler query engine; budget < 100 ms for keystroke-to-diagnostic; LSP perf tests in CI          | Tooling           |
| 9 | Concurrency story confuses users (three layers)                       | 3 | 3 | Excellent docs and recipes; "default ladder": async for IO, threads for CPU, actors for state; lints suggest the right layer | DX                |
| 10 | Memory model wars: community split on linear vs borrow vs GC        | 2 | 4 | Make decision early, document the why; no toggle, no compromise that adds complexity                    | Tech lead         |
| 11 | Hiring competent compiler engineers is hard                          | 4 | 4 | Open source from day one; clear good-first-issues; explicit mentorship for new contributors             | Project lead      |
| 12 | Funding runs out before Phase 4                                      | 3 | 5 | Aim for production users in Phase 2; pursue grants and corporate sponsorship; keep core team < 10        | Project lead      |
| 13 | Security vulnerability in stage-0 compromises stage-1 build           | 1 | 5 | Stage-0 written in Rust; minimal `unsafe`; SLSA-style provenance; reproducible builds across all stages | Security          |
| 14 | Cross-platform support drains effort                                 | 4 | 3 | Tier system: Tier-1 (always green), Tier-2 (best-effort), Tier-3 (community); CI matrix enforces tiers   | Backend           |
| 15 | C-style FFI security incidents from unsafe boundary                   | 3 | 4 | Audit-on-PR for `unsafe`/`extern`; integration tests with sanitizers (ASan, UBSan, TSan)                | Security          |
| 16 | Generics complexity (monomorphization explosion)                      | 3 | 3 | Cap recursion + cache; emit warning when a function instantiates > 50 times; LTO + dead-code elimination | Backend           |
| 17 | Edition transitions break the ecosystem                              | 2 | 4 | Edition deltas are syntactic only; semantic changes require a major version bump                         | Tech lead         |
| 18 | "Yet another language" perception locks out adoption                  | 4 | 3 | Lead with concrete benchmarks against incumbents; case studies; not "better Rust" but "complementary"   | DX / advocacy     |

## Top-3 watch list

1. **Build-speed promise (#5)** — if mom is not visibly faster than
   Rust at compile time we lose the largest single positioning
   advantage. We must ship the query engine and disk cache **in
   Phase 1**, not bolt them on later.
2. **C++ interop scope (#2)** — every other C++ FFI story (cxx,
   bindgen, Swift, …) underestimated this. We commit early to the
   curated-subset stance and publish it.
3. **Self-host timing (#4)** — the temptation to declare victory at
   stage-1 must be resisted until the test suite is mature enough that
   regressions are guaranteed-caught.

## Cancellation criteria

This project should be deprioritized if **any** of the following are
true after Phase 2:

- No production deployment has been onboarded at a partner organisation.
- Compile speed is not within 2× of Zig on equivalent code.
- Memory-safety story produces > 1 false-positive per 1 kLOC of
  idiomatic code.
- A community RFC indicates the borrow + region + actor stack feels
  worse than the alternatives users came from.

Honest stop conditions exist so the team does not invest 5 years in a
direction that isn't working.
