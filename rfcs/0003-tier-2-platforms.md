---
title: "Tier-2 platforms: WASM, RISC-V, embedded ARM Cortex-M"
authors: ["mom-core"]
status: "draft"
phase: "7"
discussion: "TBD"
implementation: "TBD"
---

# Summary

Bring up `mom` on the three platform tiers listed in `docs/roadmap.md`
Phase 7 row "Tier-2: WASM, RISC-V, embedded ARM Cortex-M". Each tier
has different cross-toolchain requirements, different runtime
constraints, and different acceptance bars.

# Motivation

Tier-1 (Linux/macOS/Windows × x86_64/aarch64) is the table-stakes
matrix for a systems language and is gated by `.github/workflows/ci.yml`.
Tier-2 platforms are where mom's "small runtime, no GC, predictable
allocation" pitch actually competes — embedded targets and the
sandboxed web platform.

# Detailed design

Three independent tracks:

## WASM (wasm32-unknown-unknown + wasm32-wasi)

- Backend: `mom build --target wasm32-wasi` invokes `clang` with
  `--target=wasm32-wasi` and links against WASI's libc. Runtime stays
  the same — `mom_print_*` map to WASI `fd_write`.
- Acceptance: `examples/hello.mom` runs under `wasmtime` and prints
  `hello, mom`.

## RISC-V (riscv64gc-unknown-linux-gnu, riscv32imac-unknown-none-elf)

- Backend: same `cc` path with `--target=riscv64-linux-gnu`. Linux
  tests run under `qemu-system-riscv64`.
- Bare-metal `riscv32imac-unknown-none-elf` needs `--no-stdlib` build
  mode (Phase 7.1 RFC) — out of scope for this RFC.
- Acceptance: linux variant of `tests/native_build.rs` matrix passes
  under qemu.

## ARM Cortex-M (thumbv7m-none-eabi, thumbv7em-none-eabihf)

- Backend: needs a `--no-stdlib` build mode so user code can supply
  its own `mom_print_*` and `mom_main` shim. The default Phase 1
  runtime calls `printf`, which doesn't exist on bare-metal Cortex-M.
- Acceptance: a `bare-metal/` example builds with `cargo run --
  build --target thumbv7em-none-eabihf hello.mom --no-stdlib` and
  links cleanly against a user-supplied `mom_print_int` stub.

# Drawbacks

- Each platform adds a CI job, a cross toolchain to maintain, and a
  permutation in the test matrix.
- Bare-metal Cortex-M requires significant runtime refactoring
  (`--no-stdlib`, custom allocator hooks, no `<stdio.h>`). This is
  why the RFC explicitly defers Cortex-M to a later phase.

# Rationale and alternatives

- Skip WASM entirely: lose the "mom in the browser" pitch. WASM is the
  cheapest of the three to add, so we keep it.
- Skip RISC-V until vendors ship more silicon: defer-by-default; only
  bring up when there's a real user.

# Acceptance criteria

For each track:
1. A `.github/workflows/tier2-<platform>.yml` workflow exists and
   runs at least one `mom run examples/hello.mom` equivalent.
2. The cross toolchain install command is one line in
   `CONTRIBUTING.md`.
3. The platform's row in `docs/roadmap.md` Phase 7 table flips from
   ⏳ to ✅ once the CI workflow is green for 7 consecutive days.
