---
title: "DWARF v5 / CodeView + real breakpoints in mom dbg"
authors: ["mom-core"]
status: "draft"
phase: "5.2"
discussion: "TBD"
implementation: "TBD"
---

# Summary

Emit DWARF v5 (Linux/macOS) and CodeView (Windows) debug info from
the native codegen, plus extend `mom dbg` from its current "launch +
output + exit" surface to real breakpoint, step, and variable
inspection support.

# Motivation

Phase 5.1's `mom dbg` ships a DAP server that runs programs through
the interpreter and emits output events. Editors connect, but they
can't set a breakpoint or step a line. That's the gap between
"compatible with the protocol" and "useful as a debugger."

# Detailed design

Three workstreams:

1. **Debug info emission** in `src/codegen.rs`:
   - DWARF v5 `.debug_info`, `.debug_line`, `.debug_str` sections via
     the host `cc`'s `-g` flag is *not enough* — we need synthesized
     line tables that map each emitted C statement back to a `.mom`
     source span. Approach: emit `#line N "src.mom"` directives in the
     generated C so the platform compiler reuses them.
   - CodeView: `cc /Z7` on the MSVC toolchain produces the equivalent
     sections from the same `#line` directives.
   - Acceptance: `objdump --dwarf=info` shows source paths and line
     numbers matching `.mom` files; `gdb -batch -ex 'list main'`
     prints mom source, not the generated C.

2. **Breakpoint trapping** in `mom dbg`:
   - DAP `setBreakpoints` request maps `<path, line>` to a stable
     "breakpoint id". `mom dbg` spawns the native binary under
     `ptrace(PTRACE_TRACEME)` (Linux), records the load address, and
     writes `INT3` (x86_64) / `BRK #0` (aarch64) at the resolved
     instruction address from the DWARF line table.
   - `continue` resumes the tracee. On SIGTRAP, parse `siginfo`,
     resolve back to the breakpoint id, fire DAP `stopped` event.
   - macOS: `mach_exception_ports` API. Windows: `WaitForDebugEvent`.

3. **Variables + stack trace**:
   - Walk the DWARF `.debug_info` for the current PC's enclosing
     function, enumerate locals + parameters, read their values out of
     the tracee's memory via `process_vm_readv` (Linux) or
     `mach_vm_read_overwrite` (macOS).
   - Map mom types to DAP's `Variable` shape: Int/Float/Bool/String
     scalar, Struct/Enum recursive, References followed.

# Drawbacks

- Big OS-specific surface: three different breakpoint-trapping APIs,
  three different debug-info parsers. Each needs its own platform
  test matrix on hosted CI runners.
- DWARF v5 is large (~10kLOC of structured parsing/writing code we'd
  bring in-tree).

# Rationale and alternatives

- Use `gdb` as a backend: `mom dbg` shells out to `gdb --interpreter=mi`
  and translates between MI and DAP. Much smaller code surface; loses
  the "single binary, no system gdb" property.
- Ship interpreter-side breakpoints only: source-line breakpoints work
  via interpreter instrumentation; no native-binary debugging. Useful
  for fast-feedback but doesn't match the DAP promise.

# Acceptance criteria

A user in VS Code:
1. Opens a `.mom` file.
2. Clicks a gutter to set a breakpoint.
3. Hits `Run → Start Debugging`.
4. Sees the program stop at the breakpoint, with locals visible in
   the Variables pane and a stack trace whose frames are `.mom` lines
   (not generated C lines).

All three platforms (Linux x86_64, macOS aarch64, Windows x86_64)
must pass.
