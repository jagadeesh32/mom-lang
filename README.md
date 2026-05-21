# mom

**mom** is a modern, self-hosted systems programming language designed for
enterprise-scale software: operating systems, distributed systems, AI
infrastructure, networking stacks, and high-performance backend services.

Source files use the `.mom` extension. The compiler binary is `mom`.

mom combines:

| Influence  | Property                                                      |
|------------|---------------------------------------------------------------|
| Python     | readable, low-ceremony syntax with indentation-based blocks   |
| Rust       | memory safety, sum types, traits, exhaustive pattern matching |
| Erlang/OTP | actors, message passing, supervised fault isolation           |
| Zig        | extremely fast incremental compilation, no hidden allocations |
| Go         | lightweight tasks, easy concurrency, simple build flow        |
| C / C++    | first-class FFI, predictable performance, no managed runtime  |

The language goal is **simple to learn, safe by default, fast at runtime,
fast to compile, and able to compile itself**.

---

## Installation

### From release binaries (recommended)

Pre-built binaries for all tier-1 platforms are attached to every
[GitHub Release](https://github.com/your-org/mom/releases).

#### Linux x86_64

```sh
curl -L https://github.com/your-org/mom/releases/latest/download/mom-x86_64-linux.tar.gz \
  | tar -xz -C /usr/local/bin
chmod +x /usr/local/bin/mom
mom version
```

#### Linux ARM64 (aarch64)

```sh
curl -L https://github.com/your-org/mom/releases/latest/download/mom-aarch64-linux.tar.gz \
  | tar -xz -C /usr/local/bin
chmod +x /usr/local/bin/mom
mom version
```

#### macOS ARM (Apple Silicon — M1/M2/M3)

```sh
curl -L https://github.com/your-org/mom/releases/latest/download/mom-aarch64-darwin.tar.gz \
  | tar -xz -C /usr/local/bin
chmod +x /usr/local/bin/mom
mom version
```

#### macOS x86_64 (Intel)

```sh
curl -L https://github.com/your-org/mom/releases/latest/download/mom-x86_64-darwin.tar.gz \
  | tar -xz -C /usr/local/bin
chmod +x /usr/local/bin/mom
mom version
```

#### Windows x86_64

Download `mom-x86_64-windows.zip` from the
[latest release](https://github.com/your-org/mom/releases/latest),
extract, and place `mom.exe` somewhere on your `PATH`.

```powershell
# Using PowerShell
Expand-Archive mom-x86_64-windows.zip -DestinationPath "$env:LOCALAPPDATA\mom"
$env:PATH += ";$env:LOCALAPPDATA\mom"
mom version
```

#### Windows ARM64 (aarch64)

Download `mom-aarch64-windows.zip` from the
[latest release](https://github.com/your-org/mom/releases/latest),
extract, and place `mom.exe` somewhere on your `PATH`.

```powershell
Expand-Archive mom-aarch64-windows.zip -DestinationPath "$env:LOCALAPPDATA\mom"
$env:PATH += ";$env:LOCALAPPDATA\mom"
mom version
```

### One-line installer (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/your-org/mom/main/scripts/install.sh | bash
```

The installer detects your platform triple, downloads the matching binary,
and places it in `~/.local/bin`.

---

## Building from source

You need **Rust 1.78+** and `cargo`.

```sh
git clone https://github.com/your-org/mom.git
cd mom
cargo build --release
# Binary is at ./target/release/mom
./target/release/mom version
```

To cross-compile for a different target (requires
[cross](https://github.com/cross-rs/cross)):

```sh
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target aarch64-pc-windows-msvc
```

---

## Quick Start

Create `hello.mom`:

```mom
fn main():
    println("Hello, mom!")
    println("Version 0.2.0")
```

Run it:

```sh
mom run hello.mom
```

### More examples

```mom
// Fibonacci — recursion + if/elif/else
fn fib(n: Int) -> Int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

fn main():
    print(fib(10))   // 55
```

```mom
// Factorial — while loop + mutable bindings
fn factorial(n: Int) -> Int:
    let mut result = 1
    let mut i = 1
    while i <= n:
        result = result * i
        i = i + 1
    return result

fn main():
    print(factorial(7))   // 5040
```

```mom
// Boolean logic — Python-style keywords
fn between(lo: Int, x: Int, hi: Int) -> Bool:
    return lo <= x and x <= hi

fn main():
    print(between(1, 5, 10))    // True
    print(between(1, 99, 10))   // False
```

The old `{ }` brace syntax is still accepted for backward compatibility.

---

## CLI Commands

```sh
# Run and compile
mom run       <file.mom>                  # interpret and run
mom build     <file.mom> [-o OUT]         # compile to native binary
mom build-run <file.mom>                  # compile then execute
mom emit-c    <file.mom>                  # show generated C source

# Inspection
mom tokens    <file.mom>                  # show token stream
mom ast       <file.mom>                  # print parsed AST
mom check     <file.mom>                  # type-check + borrow-check

# Developer tooling
mom fmt       <file.mom> [--check]        # format in place
mom lint      <file.mom>                  # apply mom.toml [lints] config
mom doc       <file.mom>                  # emit Markdown API reference
mom test      [dir]                       # discover and run *.mom tests
mom new       <dir>                       # scaffold a new project
mom init      [dir]                       # scaffold in existing directory
mom pkg       list|add|remove|audit       # manage dependencies
mom lsp                                   # LSP server on stdio
mom bench     [file.mom]                  # benchmark #[bench] functions
mom prof      [file.mom]                  # profile with call-trace
mom dbg       [file.mom]                  # DAP-over-stdio debugger

mom version                               # print compiler version
mom help                                  # full help
```

---

## Standard Library

The `std/` directory ships 60+ built-in functions across 14 modules:

| Module         | Highlights                                              |
|----------------|---------------------------------------------------------|
| `std/core`     | identity, min, max, clamp, abs, sign, Option/Result     |
| `std/fmt`      | repeat, pad_left, pad_right, join, bracket              |
| `std/io`       | LineBuffer, write, writeln, flush                       |
| `std/math`     | gcd, lcm, pow_int, factorial, fib, LCG RNG              |
| `std/test`     | assert_eq_int, assert_true, assert_false, TestStats      |
| `std/async`    | compute, join_all_int, yield_now                        |
| `std/actor`    | channel-driven mailbox + run loop                       |
| `std/net`      | Address, Request, Response, route dispatch              |
| `std/serde`    | JSON-ish encoders (bool/int/string/list/key-value)      |
| `std/crypto`   | Adler-32, polynomial rolling hash, hex encoders         |
| `std/log`      | Level enum, Logger with rank-gated emit                 |
| `std/sync`     | Mutex/Atomic/Once surface                               |
| `std/os`       | env, sleep, process-info wrappers                       |
| `std/alloc`    | Box, Rc, Arc, region demo                               |

---

## Status

This repository hosts the **Rust-based bootstrap toolchain** (stage-0).
The plan ends with mom compiling itself through a native LLVM + custom-codegen
backend. See [`docs/plan.md`](docs/plan.md) and
[`docs/roadmap.md`](docs/roadmap.md) for the full roadmap.

---

## Repository Layout

```
.
├── Cargo.toml              # Rust workspace
├── src/                    # Rust sources: lexer, parser, AST, types,
│                           # borrow checker, interpreter, codegen, subcommands
├── runtime/                # native runtime support (runtime.c)
├── compiler/               # stage-1 mom-in-mom compiler + examples
├── std/                    # Phase 6 standard library (.mom files)
├── tests/                  # Rust acceptance suites
├── examples/               # sample .mom programs
├── docs/                   # design docs, grammar, plan, roadmap
├── rfcs/                   # RFC template + accepted proposals
├── scripts/                # install.sh and ops scripts
└── .github/workflows/      # CI matrix + release pipeline
```

---

## Documentation Index

| Topic                              | File                                          |
|------------------------------------|-----------------------------------------------|
| Language philosophy and goals      | [docs/philosophy.md](docs/philosophy.md)      |
| Surface language design            | [docs/design.md](docs/design.md)              |
| Formal grammar (EBNF)              | [docs/grammar.ebnf](docs/grammar.ebnf)        |
| Type system                        | [docs/types.md](docs/types.md)                |
| Memory model & safety              | [docs/memory.md](docs/memory.md)              |
| Concurrency, actors, supervision   | [docs/concurrency.md](docs/concurrency.md)    |
| C and C++ interoperability         | [docs/interop.md](docs/interop.md)            |
| Build system & package manager     | [docs/build_system.md](docs/build_system.md)  |
| Standard library plan              | [docs/stdlib.md](docs/stdlib.md)              |
| Tooling: fmt, lint, LSP, debugger  | [docs/tooling.md](docs/tooling.md)            |
| Compiler architecture              | [docs/compiler.md](docs/compiler.md)          |
| Bootstrapping & self-hosting plan  | [docs/bootstrap.md](docs/bootstrap.md)        |
| Engineering roadmap                | [docs/roadmap.md](docs/roadmap.md)            |
| Risks & mitigations                | [docs/risks.md](docs/risks.md)                |

---

## License

Apache-2.0.
