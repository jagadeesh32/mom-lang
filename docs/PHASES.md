# Mom Language — Phase Plan

This document is the **engineering master plan** for Mom. Every phase has:
- A concrete goal stated in one sentence
- Measurable deliverables with acceptance criteria
- A performance target where applicable
- Dependencies on previous phases

Phases 0–3 are shipped. Phases 4–8 are the active roadmap.

---

## Status Legend

| Symbol | Meaning |
|---|---|
| ✅ | Shipped and locked by regression tests |
| 🔄 | In progress |
| 🔜 | Planned, not started |
| 💡 | Research / RFC stage |

---

## Phase 0 — Stage-0 Toolchain ✅

**Goal:** A working interpreter and CLI written in Rust that can run Mom programs.

| Deliverable | Acceptance |
|---|---|
| Lexer, parser, AST | All examples parse without error |
| Lenient type checker | Type mismatches on known types caught |
| Tree-walking interpreter | `mom run examples/fib.mom` → correct output |
| CLI: `tokens`, `ast`, `check`, `run`, `version` | Each command exits 0 on valid input |
| Example programs (`examples/*.mom`) | All 22 examples check and run |
| Documentation set | README covers install, quick-start, CLI |

---

## Phase 1 — Native C Backend ✅

**Goal:** Compile Mom to a native binary via C; match handwritten C performance on scalar programs.

| Deliverable | Acceptance |
|---|---|
| C codegen for `Int`, `Bool`, `Float` | `fib(30)` produces same output as interpreter |
| `String` → `const char*` with C re-escaping | String literals, print, concat work natively |
| `struct` → C `typedef struct` | Struct literals, field access, field assignment |
| `enum` → C tagged union + `match` | Variant construct, value-form + statement-form match |
| Nested and literal sub-patterns | `Wrap(A(n))`, `Val(0)` lower to guard chains |
| Linker driver (`cc` / `$CC`) | Binary runs on Linux x86_64, ARM64, macOS, Windows |
| Build cache keyed on compiler identity | Stale cache impossible after compiler rebuild |
| `mom build`, `mom build-run`, `mom emit-c` | 13 native-build tests pass |
| Release CI matrix | Packages for `.deb`, `.rpm`, `.tar.gz`, `.zip` |

**Performance target:** `fib(40)` native ≤ 2× GCC `-O2` time.

---

## Phase 2 — Memory Safety ✅

**Goal:** Compile-time borrow checker catches use-after-free, aliasing violations, and mutation-while-borrowed.

| Deliverable | Acceptance |
|---|---|
| `&T` / `&mut T` reference types | Parsed and type-checked |
| Borrow checker (lexical, phase-2) | Catches: use-after-move, double-`&mut`, shared+mut mix, assign-while-borrowed, `&mut` on immutable |
| `block:` scoped expression | Borrow ends at block boundary |
| List elements / field assignment as reads | No false move errors |
| Match-arm assignment bodies | `Some(n) => count = count + n` works |
| `Option[T]`, `Result[T,E]`, `?` operator | 14 type-checker tests pass |
| Python-style indent/dedent syntax | All examples work in both brace and indent style |
| 142 total tests green | `cargo test --release` |

---

## Phase 3 — Concurrency (Interpreter) ✅

**Goal:** Channels, actors, async/await, and cancellation work in the interpreter.

| Deliverable | Acceptance |
|---|---|
| `Channel(cap?)` with `.send`, `.recv`, `.try_recv`, `.len`, `.close` | `examples/channels.mom` runs |
| `Cancel()` with `.signal()`, `.is_cancelled()` | `examples/cancel.mom` runs |
| `spawn expr` → `Task[T]` | Task handle returned |
| `await expr` | Unwraps task result |
| Actor pattern via channels | `examples/actor_via_channels.mom` runs |
| `std/actor.mom` mailbox loop | `run_counter` demo works |

---

## Phase 3.1 — Native Lists + Async Runtime 🔜

**Goal:** Lists compile to native code; a real work-stealing async executor ships.

### 3.1a — Native List Codegen

| Deliverable | Acceptance |
|---|---|
| `[T]` lowered to a C tagged struct `{T* ptr, int64_t len, int64_t cap}` | `examples/lists.mom` builds and runs |
| Indexing with runtime bounds check | Out-of-bounds panics with file+line |
| `push`, `pop`, `len`, `reverse`, `sort` in runtime.c | All list built-ins work natively |
| `for x in list` loop in native codegen | Iterator desugars to C `for` |
| `map`, `filter`, `reduce` as native calls | Pipeline example compiles |

### 3.1b — Async Runtime

| Deliverable | Acceptance |
|---|---|
| Work-stealing executor (N threads, M tasks) | `spawn` + `await` run in parallel on 4 cores |
| Real `sleep(ms)` via OS timer | `sleep(100)` pauses 100±10ms |
| `timeout(ms, future)` combinator | Returns `Err(TimedOut)` if exceeded |
| `join(task1, task2)` | Both tasks run concurrently |
| `select(task1, task2)` | Returns whichever finishes first |
| Channel send/recv integrated with executor | No busy-wait |

**Performance target:** 1M channel messages/sec on 4-core machine.

---

## Phase 3.2 — Supervision Trees + Advanced Channels 🔜

**Goal:** Production-grade fault tolerance: actors restart automatically, channels have broadcast/oneshot modes.

| Deliverable | Acceptance |
|---|---|
| `supervise actor with restart(limit, window, strategy)` | Actor restarts on panic, up to limit |
| `OneForOne`, `OneForAll`, `RestForOne` strategies | Each triggers correct restart scope |
| Supervisor escalation (fail up to parent) | Budget exhaustion propagates |
| `permanent` / `transient` / `temporary` lifetimes | Correct restart behavior per policy |
| `channel.broadcast[T](cap)` | Multi-consumer fan-out |
| `channel.oneshot[T]()` | Request-reply without shared state |
| `actor.ask(msg)` → `Future[T]` | Type-safe request-reply |
| `supervise group { spawn A; spawn B }` | Group restart works |

**Acceptance test:** A crashing actor restarts 3 times then escalates; the supervisor demo terminates cleanly.

---

## Phase 4 — Full Self-Host + Generics Monomorphization 🔜

**Goal:** The Mom compiler compiles itself end-to-end; generic code monomorphizes at compile time; no Rust required to build Mom.

### 4.1 — Stage-1 Widening (Mom-in-Mom Compiler Completeness)

The `compiler/src/main.mom` self-hosted compiler currently handles a strict subset. This sub-phase closes the gap:

| Feature to add to Stage-1 | Notes |
|---|---|
| Structs + field access/assignment | Required for compiler's own AST nodes |
| Enums + pattern matching | Required for token/AST dispatch |
| Lists with push/pop/map/filter | Required for token stream, AST node lists |
| Generics (monomorphized templates) | Required for `Option[T]`, `Result[T,E]` usage |
| Modules + multi-file compilation | Required for compiler source split across files |
| Traits + impl dispatch | Required for `Display`, `Debug` on AST nodes |
| String operations | Required for identifier/literal handling |
| For-in-list loops | Required for iteration over token/AST lists |

**Acceptance:** Stage-1 compiles `compiler/src/main.mom` into itself (triple fixed point: stage-1 built by stage-1 built by stage-0 produces identical C).

### 4.2 — Generics Monomorphization

| Deliverable | Acceptance |
|---|---|
| Type parameter collection pass | All `[T]`, `[T, E]` sites recorded |
| Monomorphization expander | One C function per concrete instantiation |
| Inline `Option[Int]` → no boxing | Zero overhead vs hand-coded enum |
| `fn identity[T]` → `identity_Int`, `identity_String`, etc. | Correct output for each type |
| Recursive generics (e.g. `Option[Option[Int]]`) | Terminates; correct behavior |

**Performance target:** Generic `fib` ≤ 5% slower than monotyped `fib`.

### 4.3 — Bootstrap Independence

| Deliverable | Acceptance |
|---|---|
| `mom bootstrap` command | Builds Mom using only a C compiler |
| No Rust required after bootstrap | Fresh machine: C compiler → Mom binary |
| `make bootstrap` in CI | Green on Linux x86_64 and ARM64 |
| Stage-2 binary passes all existing tests | Same 142+ tests green |

---

## Phase 5 — LLVM Backend 🔜

**Goal:** Mom generates LLVM IR directly; performance matches Clang `-O3`; supports SIMD, inline assembly, and all LLVM targets.

| Deliverable | Acceptance |
|---|---|
| LLVM IR emitter in `src/codegen_llvm.rs` | Hello world emits valid `.ll` |
| All Phase-1–4 features lowered to LLVM | Full test suite green via LLVM path |
| `--backend llvm` CLI flag | User can choose C or LLVM |
| Auto-vectorization of `map`/`filter` loops | 4× speedup on AVX2 machine |
| Inline assembly blocks | `asm { ... }` accepted and emitted |
| Cross-compile via LLVM targets | `--target wasm32-unknown-unknown` works |
| WebAssembly output | Mom wasm binary runs in browser |
| Debug info (DWARF) | `mom dbg` sets real breakpoints in gdb/lldb |
| Profile-guided optimization | `--pgo-generate`, `--pgo-use` flags |

**Performance target:** Mom LLVM ≥ 95% of Clang `-O3` on the Mom benchmark suite (fib, matrix-mul, json-parse, actor-throughput).

---

## Phase 6 — Full Standard Library 🔜

**Goal:** The `std/` library is complete, tested, documented, and implemented natively (not interpreter stubs).

### Priority 1 — Core and Collections (ships with Phase 4)

| Module | Key additions over current stub |
|---|---|
| `std::core` | `Iterator` trait, `collect`, `chain`, `flat_map`, `take`, `skip` |
| `std::alloc` | Real `Vec[T]`, `HashMap[K,V]`, `BTreeMap`, `Set`, `Deque`, `String` (owned) |
| `std::fmt` | `write!`, `format!`, `Display`, `Debug` derive |

### Priority 2 — I/O and OS (ships with Phase 5)

| Module | Key additions |
|---|---|
| `std::io` | Real file I/O, `BufReader`, `BufWriter`, memory-mapped files |
| `std::os` | Real process, filesystem, env, signal, time APIs |
| `std::path` | `Path`, `PathBuf`, join, resolve |

### Priority 3 — Networking and Crypto (ships with Phase 5.1)

| Module | Key additions |
|---|---|
| `std::net` | Real TCP, UDP, TLS, HTTP/1.1, HTTP/2, gRPC client+server |
| `std::crypto` | SHA-2/3, BLAKE3, AES-GCM, ChaCha20-Poly1305, Ed25519, X25519 |
| `std::log` | OTLP-compatible structured logging, tracing spans |
| `std::serde` | JSON, CBOR, MsgPack, TOML with zero-copy deserialization |

### Priority 4 — Concurrency Primitives (ships with Phase 6)

| Module | Key additions |
|---|---|
| `std::sync` | Real `Mutex`, `RwLock`, `Atomic`, `Condvar`, `Barrier` |
| `std::async` | Full executor, `join!`, `select!`, `race!`, `try_join!` |

**Acceptance:** Every module has a documented API, an oracle test in `tests/stdlib.rs`, and an implementation that passes the oracle without the interpreter shim.

---

## Phase 7 — Package Ecosystem 🔜

**Goal:** A public package registry, `mom pkg` fully functional, and a healthy ecosystem of community packages.

| Deliverable | Acceptance |
|---|---|
| Package registry at `pkg.mom-lang.org` | `mom pkg add json` fetches and caches |
| `mom.toml` `[dependencies]` section | Version resolution, lock file |
| `mom pkg audit` | Checks against CVE database |
| `mom pkg publish` | Maintainers can publish packages |
| Namespace isolation | Packages cannot access private items across crate boundaries |
| `mom pkg update` | Upgrades to latest compatible versions |
| 50+ community packages at launch | json, http-server, postgres, redis, uuid, regex, ... |

---

## Phase 8 — 1.0 Stable Release 💡

**Goal:** Stable language spec, API stability guarantee, edition system, and enterprise-ready tooling.

| Deliverable | Notes |
|---|---|
| Language specification 1.0 | Formal grammar + type rules document |
| API stability promise | No breaking changes without edition bump |
| Edition `2027` | Opt-in breaking improvements every 2 years |
| `#[stable(since = "1.0")]` / `#[unstable]` markers | Public API tracked |
| Security response team | CVE process, 7-day patch SLA |
| Long-term support (LTS) releases | 18-month support window per LTS |
| Full LLVM coverage | 100% of language features on LLVM path |
| Windows ARM64 native binary | All 4 tier-1 targets passing CI |
| `mom fmt` deterministic | Same source always produces same output |
| Incremental compilation | Only changed files recompile |
| IDE plugin v1.0 | VS Code + JetBrains extensions in marketplaces |
| Embedded/RTOS target | Runs without OS (bare-metal `no_std` equivalent) |

---

## Phase Timeline (Estimates)

```
2025  Q1–Q2   Phase 0–2 (shipped)
2025  Q3–Q4   Phase 3 (concurrency interpreter) (shipped)
               Phase 3.1 (native lists + async runtime)   ← current
2026  Q1      Phase 3.2 (supervision trees)
2026  Q2–Q3   Phase 4 (full self-host + monomorphization)
2026  Q4      Phase 5 (LLVM backend)
2027  Q1–Q2   Phase 6 (full standard library)
2027  Q3      Phase 7 (package ecosystem)
2027  Q4      Phase 8 — Mom 1.0
```

---

## Performance Targets by Phase

| Phase | Benchmark | Target |
|---|---|---|
| Phase 1 (now) | `fib(40)` native | ≤ 2× GCC -O2 |
| Phase 3.1 | 1M channel msgs/sec | 4-core machine |
| Phase 4 | Generic overhead | ≤ 5% vs monotyped |
| Phase 5 | Full suite vs Clang | ≥ 95% Clang -O3 |
| Phase 5 | Startup time | < 1ms cold start |
| Phase 5 | Wasm binary size | < 100KB for hello-world |
| Phase 6 | HTTP server throughput | ≥ Go's net/http on same HW |

---

## RFC Process

Major language changes follow the RFC (Request for Comments) process in `rfcs/`:

1. **Draft** — open a PR with `rfcs/NNNN-feature-name.md`
2. **Comment period** — 14 days minimum, maintainer review
3. **Accepted / Postponed / Rejected** — merged, closed, or marked deferred
4. **Implementation** — PR linked to accepted RFC
5. **Stabilization** — feature flag removed, added to stable spec

Accepted RFCs so far:
- RFC-0001: Stage-2 native codegen widening (structs, enums, strings)
- RFC-0002: DWARF debug info and real breakpoints
- RFC-0003: Tier-2 platform support (RISC-V, MIPS, PowerPC)
- RFC-0004: Production user outreach program

---

## How to Contribute

The highest-leverage areas right now (Phase 3.1):

1. **Native list codegen** — `src/codegen.rs`: add `CType::List`, emit `mom_vec_t` struct, wire `push`/`pop`/iteration
2. **Work-stealing executor** — `src/interpreter.rs`: replace the synchronous `await` stub with a real Tokio-style scheduler
3. **`for x in list` native** — parser already handles it; codegen needs the list iterator
4. **`Option[T]` native** — use the existing tagged-union pattern from enums

See `CONTRIBUTING.md` for the PR workflow and test requirements.
