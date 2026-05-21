# mom — Bootstrap & Self-Hosting Plan

mom must eventually compile itself: the canonical compiler is written
**in mom**, built by the previous mom compiler, and the chain is rooted
in a small, auditable starting point. This document explains how we
get from "Rust prototype" to "self-hosted production toolchain"
without losing functionality or trust.

## 1. The plan in one sentence

> Stage-0 (Rust) → Stage-1 (mom written in mom, compiled by Stage-0)
> → Stage-2 (mom written in mom, compiled by Stage-1) → bit-for-bit
> reproducible.

## 2. Stage-0 — the bootstrap (this repository)

- **Implementation language:** Rust.
- **Surface coverage:** the full language *grammar* — everything the
  user can write is parsed and type-checked (leniently).
- **Execution:** a tree-walk interpreter for the executable subset of
  the language.
- **Outputs:** `tokens`, `ast`, `check`, `run`. **Not yet a native
  compiler.**
- **Purpose:** unblock developing the standard library, language
  tests, and the future Stage-1 compiler — all written in `.mom`.

Stage-0 lives in `src/` and is what `cargo build` produces today.

## 3. Stage-1 — first self-host

- **Implementation language:** mom (a subset of the language that
  Stage-0 can actually run end-to-end).
- **Output:** a real native compiler: lexer, parser, HIR, MIR, borrow
  check, LLVM backend, linker driver.
- **Built with:** Stage-0 first, by **interpreting** the Stage-1 source.
- **Lives in:** `compiler/` (created in this phase).

Cycle:

```
cd compiler
mom run src/main.mom -- build src/main.mom -o stage1_compiler
```

This is slow — interpreted compilation is meaningful for a
proof-of-concept, not for daily use. Stage-1 only has to compile
itself once.

## 4. Stage-2 — self-host on native code

- **Same source as Stage-1**, but compiled by the Stage-1 binary.
- The result is the **first native mom-in-mom binary**.
- We then check: does `stage1_compiler` build `stage2_compiler`, and
  does `stage2_compiler` build a bit-identical `stage2_compiler`?
  - Yes → we have **fixed-point self-host**. Reproducibility achieved.
  - No → re-iterate. Most discrepancies are debug info paths or
    Rust-stage-only assumptions.

## 5. Stage-3 — drop Rust

- Once Stage-2 is reproducible, the Rust Stage-0 sources become a
  reference implementation and a regression oracle.
- Stage-3 = Stage-2 + new features that Stage-0 can't even parse.
- The Rust source tree is archived in a `bootstrap-rs/` branch for
  posterity and supply-chain auditing.

## 6. Cross-checks

At every stage transition we run a **triple-check**:

1. **Self-host check**: `stage_n` compiles `stage_n` → `stage_{n+1}`.
2. **Bit-identical check**: `stage_n` compiles `stage_{n+1}` source;
   the output is byte-identical to `stage_{n+1}` produced earlier.
3. **Behavioral check**: the language test suite (thousands of `.mom`
   programs) passes when run through `stage_n` *and* `stage_{n+1}`.

Failing any of these blocks the release.

## 7. How other languages did it

| Language | Strategy | Stage-0 | Notes                                  |
|----------|----------|---------|----------------------------------------|
| Rust     | OCaml stage-0 → Rust stage-1 | OCaml | Rust 1.0 finally retired OCaml in 2015 |
| Go       | C stage-0 → Go stage-1       | C     | Removed C in Go 1.5                    |
| Zig      | C++/LLVM bootstrap, then stage-1 in Zig | C++ | Currently mid-migration to stage-2 native backend |
| Swift    | C++ compiler kept in tree    | C++   | Not self-hosted (yet)                  |
| TypeScript | bootstrapped from JS       | TS    | Single binary distribution unlike mom  |
| OCaml    | Caml light → OCaml           | Caml  | Multi-decade evolution                  |

mom follows the **Zig / Go pattern**: a stage-0 in a mature language,
then a deliberate migration. The Rust stage-0 buys us memory safety
in the bootstrap itself (we audit Rust safely, not raw C++).

## 8. Risks and how we de-risk

### Risk: divergence between stage-0 semantics and stage-1 semantics
*Mitigation*: the test suite is the canonical specification — every
language test passes through both stages.

### Risk: Stage-1 too large to interpret
*Mitigation*: Stage-1 is **explicitly minimal** — only the language
features it actually uses. Pretty printers, error explanations, LSP,
formatter are added in stage-2 or later.

### Risk: subtle UB in stage-0 corrupts stage-1
*Mitigation*: stage-0 is Rust, which forbids UB outside `unsafe`.
The only `unsafe` in stage-0 is in the bootstrap interpreter's value
representation, audited line-by-line.

### Risk: LLVM / linker drift breaks bit-identical builds
*Mitigation*: pinned LLVM version per release; CI runs the fixed-point
check on every commit.

### Risk: chicken-and-egg for the standard library
*Mitigation*: `std::core` is feasible to write in the stage-0 subset
(no allocator needed). `std::alloc` and above ship with stage-1.

## 9. Timeline (target)

| Stage | Target date | Concrete artifact                        |
|-------|-------------|------------------------------------------|
| 0     | shipped (this repo) | Rust bootstrap toolchain         |
| 0.5   | +3 months   | LLVM backend wired into stage-0          |
| 1     | +9 months   | mom-in-mom compiler builds with stage-0  |
| 2     | +12 months  | mom-in-mom compiler self-hosts           |
| 3     | +18 months  | Rust stage-0 retired                     |

These dates are stretch targets, not commitments. The non-negotiable
constraint is **stability before stage-3** — the language must already
be in production use before we retire the bootstrap.

## 10. What this means for users today

- Use stage-0 (`cargo run -- run …`) to learn the language, write
  programs, and contribute.
- Track stage-1 progress in `compiler/CHANGELOG.md`.
- When stage-2 lands, `mom` is a self-contained binary download with
  no Rust dependency.
- Existing `.mom` source does **not change** across the transition —
  the canonical spec is what the user writes, not the implementation
  language under the hood.

---

## 11. Stage-1.0 — what shipped in Phase 4

The first mom-in-mom compiler is now in `compiler/src/main.mom` and is
driven by `compiler/bootstrap.sh`. Honest scope:

- **Implementation:** a single `.mom` file, executed by the stage-0
  interpreter. It reads `MOM_INPUT`, lexes, parses, emits C99 source,
  and writes `MOM_OUTPUT`.
- **Supported source subset:**
  ```mom
  fn main() {
      let x = …          // Int literal or arithmetic expression
      let mut y = …
      y = y + 1
      print(x)
  }
  ```
  Integer literals, identifiers, and the left-associative operators
  `+`, `-`, `*`.
- **Bootstrap chain:** `bootstrap.sh source.mom -o bin` runs:
  1. `mom run compiler/src/main.mom` (stage-0 hosts stage-1)
  2. `cc -I runtime stage1.c runtime/runtime.c -o bin`
- **Tests:** `tests/selfhost.rs` runs five `.mom` snippets through the
  full chain and asserts the resulting binary's stdout.
- **Architecture:** the mom code is structured exactly like the
  eventual full compiler — `Token` enum → `Expr` / `Stmt` enums →
  parser state → C emitter. Sub-phases 4.1 → 4.3 port additional
  passes (control flow, functions, references, type checker, borrow
  checker, MIR, native backend) into `compiler/src/`.

### Path to stage-2 (full self-host)

| Sub-phase | Status  | Adds                                                                       |
|-----------|---------|----------------------------------------------------------------------------|
| 4.1       | shipped | multiple functions, `if/else`, `while`, `return`, Bool, comparisons, logical ops, function calls |
| 4.2       | planned | Structs, enums, pattern matching, lists                                    |
| 4.3       | planned | References (`&T`, `&mut T`), regions, smart pointers; borrow checker in mom |
| 4.4       | planned | Stage-1 compiles `compiler/src/main.mom` itself; bit-identical fixed point |
| 4.5       | planned | Retire `src/codegen.rs` Rust path; `mom build` defaults to stage-2 toolchain |

### Stage-1.1 supported subset (live today)

```mom
fn NAME(p1: T, p2: T, ...) -> RT { stmts }    // T ∈ { Int, Bool, () }
stmt = let [mut] NAME = EXPR
     | NAME = EXPR
     | print(EXPR)
     | if EXPR { stmts } [else { stmts }]
     | while EXPR { stmts }
     | return [EXPR]
     | EXPR

expr precedence (low → high):
  ||  &&  | == !=  | < <= > >=  | + -  | *  | unary !/-  | call / primary
```

Examples in `compiler/examples/`:
- `answer.mom` — `let` + arithmetic
- `fib.mom` — recursion + `if/else`
- `factorial.mom` — `while` + mutable state
- `bool.mom` — Bool, comparisons, logical operators
