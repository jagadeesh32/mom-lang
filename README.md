# mom

**Mom** is a modern, safe, fast, self-hosted systems programming language.  
It compiles to native binaries via C, features a borrow checker, and is written in itself.

[![CI](https://github.com/jagadeesh32/mom/actions/workflows/ci.yml/badge.svg)](https://github.com/jagadeesh32/mom/actions/workflows/ci.yml)
[![Release](https://github.com/jagadeesh32/mom/actions/workflows/release.yml/badge.svg)](https://github.com/jagadeesh32/mom/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Version](https://img.shields.io/github/v/release/jagadeesh32/mom)](https://github.com/jagadeesh32/mom/releases/latest)

```mom
fn fib(n: Int) -> Int:
    if n <= 1: n
    else: fib(n - 1) + fib(n - 2)

fn main():
    for i in 0..10:
        print(fib(i))
```

---

## Table of Contents

- [Installation](#installation)
  - [Linux](#linux)
  - [macOS](#macos)
  - [Windows](#windows)
  - [One-line installer](#one-line-installer)
  - [Build from source](#build-from-source)
- [Quick Start](#quick-start)
- [Language Features](#language-features)
- [CLI Reference](#cli-reference)
- [Standard Library](#standard-library)
- [Repository Layout](#repository-layout)
- [Contributing](#contributing)
- [License](#license)

---

## Installation

### Linux

#### Debian / Ubuntu (x86\_64)

```bash
# Download and install .deb package
curl -fsSL https://github.com/jagadeesh32/mom/releases/latest/download/mom-x86_64.deb -o mom.deb
sudo dpkg -i mom.deb
mom version
```

#### Debian / Ubuntu (ARM64 — Raspberry Pi, AWS Graviton, etc.)

```bash
curl -fsSL https://github.com/jagadeesh32/mom/releases/latest/download/mom-aarch64.deb -o mom.deb
sudo dpkg -i mom.deb
mom version
```

#### Red Hat / Fedora / CentOS / RHEL (x86\_64)

```bash
# DNF (Fedora / RHEL 8+)
sudo dnf install https://github.com/jagadeesh32/mom/releases/latest/download/mom-x86_64.rpm

# YUM (CentOS 7 / older RHEL)
sudo yum install https://github.com/jagadeesh32/mom/releases/latest/download/mom-x86_64.rpm
```

#### Red Hat / Fedora / CentOS (ARM64)

```bash
sudo dnf install https://github.com/jagadeesh32/mom/releases/latest/download/mom-aarch64.rpm
```

#### Universal tarball (any Linux distro)

```bash
# x86_64
curl -fsSL https://github.com/jagadeesh32/mom/releases/latest/download/mom-linux-x86_64.tar.gz | \
  tar -xzf - --strip-components=1 -C ~/.local/

# ARM64
curl -fsSL https://github.com/jagadeesh32/mom/releases/latest/download/mom-linux-aarch64.tar.gz | \
  tar -xzf - --strip-components=1 -C ~/.local/

# Add to PATH (add this to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
```

---

### macOS

#### Apple Silicon (M1 / M2 / M3 / M4)

```bash
curl -fsSL https://github.com/jagadeesh32/mom/releases/latest/download/mom-macos-aarch64.tar.gz | \
  tar -xzf - --strip-components=1 -C ~/.local/
export PATH="$HOME/.local/bin:$PATH"
mom version
```

> **Note:** Intel Mac (x86\_64) is not a packaged target.  
> Use Rosetta 2 (`arch -x86_64 ...`) or build from source.

---

### Windows

#### x86\_64 (Intel / AMD — most PCs)

```powershell
# PowerShell — run as regular user (no admin needed)
Invoke-WebRequest -Uri "https://github.com/jagadeesh32/mom/releases/latest/download/mom-windows-x86_64.zip" `
  -OutFile mom.zip
Expand-Archive mom.zip -DestinationPath "$env:LOCALAPPDATA\mom" -Force

# Add to PATH (run once)
$binDir = "$env:LOCALAPPDATA\mom\mom-windows-x86_64"
[Environment]::SetEnvironmentVariable("PATH", "$binDir;$([Environment]::GetEnvironmentVariable('PATH','User'))", "User")
```

Open a new terminal and test:
```cmd
mom version
```

#### ARM64 (Snapdragon X / Copilot+ PCs)

```powershell
Invoke-WebRequest -Uri "https://github.com/jagadeesh32/mom/releases/latest/download/mom-windows-aarch64.zip" `
  -OutFile mom.zip
Expand-Archive mom.zip -DestinationPath "$env:LOCALAPPDATA\mom" -Force

$binDir = "$env:LOCALAPPDATA\mom\mom-windows-aarch64"
[Environment]::SetEnvironmentVariable("PATH", "$binDir;$([Environment]::GetEnvironmentVariable('PATH','User'))", "User")
```

---

### One-line installer

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.sh | bash
```

Install a specific version:
```bash
curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.sh | bash -s -- --version v0.2.0
```

Install to a custom prefix:
```bash
curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.sh | bash -s -- --prefix /usr/local
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.ps1 | iex
```

---

### Build from source

Requires **Rust 1.78+** and `cargo`.

```bash
git clone https://github.com/jagadeesh32/mom.git
cd mom
cargo build --release
./target/release/mom version
```

The compiled binary is at `target/release/mom` (or `target\release\mom.exe` on Windows).

**Verify self-hosting:**
```bash
# Compile the mom-in-mom compiler with itself
./target/release/mom selfhost compiler/src/main.mom -o mom-stage1

# Run an example with the self-hosted binary
MOM_INPUT=examples/hello.mom MOM_OUTPUT=/tmp/hello.c ./mom-stage1
gcc -std=c99 -I compiler compiler/runtime.c /tmp/hello.c -o hello
./hello
```

---

## Quick Start

### Hello World

```mom
fn main():
    print("Hello, world!")
```

```bash
mom run hello.mom
```

### Fibonacci

```mom
fn fib(n: Int) -> Int:
    if n <= 1: n
    else: fib(n - 1) + fib(n - 2)

fn main():
    print(fib(10))   // 55
```

### Structs and methods

```mom
struct Point:
    x: Int
    y: Int

impl Point:
    fn distance(self, other: Point) -> Int:
        let dx = self.x - other.x
        let dy = self.y - other.y
        dx * dx + dy * dy

fn main():
    let a = Point { x: 0, y: 0 }
    let b = Point { x: 3, y: 4 }
    print(a.distance(b))   // 25
```

### Error handling

```mom
fn divide(a: Int, b: Int) -> Result[Int, String]:
    if b == 0: Err("division by zero")
    else:      Ok(a / b)

fn main():
    match divide(10, 2):
        Ok(v)  => print(v)      // 5
        Err(e) => print(e)

    match divide(10, 0):
        Ok(v)  => print(v)
        Err(e) => print(e)      // division by zero
```

### Pattern matching

```mom
fn classify(n: Int) -> String:
    match n:
        0       => "zero"
        1       => "one"
        _       => "many"

fn main():
    print(classify(0))    // zero
    print(classify(1))    // one
    print(classify(42))   // many
```

### Compile to a native binary

```bash
mom build fib.mom -o fib
./fib
```

---

## Language Features

| Feature | Status |
|---|---|
| Functions, recursion | ✅ |
| `let` / `let mut` bindings | ✅ |
| `Int`, `Bool`, `String`, `Float` | ✅ |
| Lists `[T]`, indexing, `for x in list` | ✅ |
| `if/else` expressions | ✅ |
| `while`, `for i in lo..hi` | ✅ |
| Structs + `impl` blocks | ✅ |
| Enums + pattern matching | ✅ |
| `Option[T]` / `Result[T,E]` + `?` | ✅ |
| Traits + `impl Trait for Type` | ✅ |
| Channels + actors | ✅ |
| `async fn` / `await` (synchronous) | ✅ |
| Generics (dynamic dispatch) | ✅ |
| Modules + imports | ✅ |
| Pipeline operator `\|>` | ✅ |
| Borrow checker (stage-0) | ✅ |
| Native codegen via C | ✅ |
| Self-hosted compiler | ✅ |
| LSP server | ✅ |
| Formatter | ✅ |
| Linter | ✅ |
| Package manager (`mom pkg`) | ✅ |
| Multi-threaded async runtime | 🔜 |
| LLVM backend | 🔜 |
| Full generics monomorphization | 🔜 |

---

## CLI Reference

```bash
# Run and compile
mom run       <file.mom>              # interpret and run
mom build     <file.mom> [-o OUT]     # compile to native binary
mom build-run <file.mom>              # compile then execute
mom emit-c    <file.mom>              # show generated C source

# Inspection
mom tokens    <file.mom>              # show token stream
mom ast       <file.mom>              # print parsed AST
mom check     <file.mom>              # type-check + borrow-check

# Tooling
mom fmt       <file.mom> [--check]    # format in place
mom lint      <file.mom>              # lint with mom.toml rules
mom doc       <file.mom>              # emit Markdown API docs
mom test      [dir]                   # discover and run *.mom tests
mom bench     [dir]                   # run benchmarks
mom prof      <file.mom>              # profile with call-trace

# Project management
mom new       <dir>                   # scaffold new project
mom init      [dir]                   # scaffold in existing directory
mom pkg       list|add|remove|audit   # manage dependencies

# Language server
mom lsp                               # LSP server on stdio
mom dbg                               # DAP debugger on stdio

# Self-hosted compiler
mom selfhost  <file.mom> [-o OUT]     # compile via stage-1 (mom-in-mom)

mom version                           # print version
mom help                              # full help
```

---

## Standard Library

The standard library is in the `std/` directory:

| Module | Description |
|---|---|
| `std/core.mom` | Core types and operations |
| `std/io.mom` | File and stream I/O |
| `std/fmt.mom` | Formatting utilities |
| `std/math.mom` | Math functions |
| `std/net.mom` | Network primitives |
| `std/os.mom` | OS interface |
| `std/sync.mom` | Synchronization primitives |
| `std/async.mom` | Async utilities |
| `std/actor.mom` | Actor pattern |
| `std/alloc.mom` | Memory allocation |
| `std/crypto.mom` | Cryptographic primitives |
| `std/log.mom` | Structured logging |
| `std/serde.mom` | Serialization/deserialization |
| `std/test.mom` | Test harness |

---

## Repository Layout

```
.
├── Cargo.toml              # Rust workspace (stage-0 compiler)
├── src/                    # Stage-0: lexer, parser, AST, types,
│                           #   borrow checker, interpreter, C-codegen, CLI
├── compiler/               # Stage-1: mom-in-mom self-hosted compiler
│   ├── src/main.mom        #   The compiler written in mom
│   ├── runtime.c           #   C runtime library
│   ├── runtime.h
│   └── examples/           #   Stage-1 test programs
├── std/                    # Standard library (.mom files)
├── tests/                  # Rust integration test suites
├── examples/               # Sample .mom programs
├── docs/                   # Design docs, grammar, roadmap
├── rfcs/                   # Accepted proposals
├── scripts/                # install.sh, install.ps1
└── .github/workflows/      # CI + release pipeline
```

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

```bash
git clone https://github.com/jagadeesh32/mom.git
cd mom
cargo build
cargo test
```

To run the self-hosting fixed-point test:
```bash
bash compiler/self_host_test.sh
```

---

## License

Licensed under the [Apache License, Version 2.0](LICENSE-APACHE).

---

*Mom is an independent open-source project.*
