# mom

**mom** is a modern, self-hosted systems programming language designed for
enterprise-scale software: operating systems, distributed systems, AI
infrastructure, networking stacks, and high-performance backend services.

Source files use the `.mom` extension. The compiler binary is `mom`.

## Language Features

**Syntax**
- Python-style indentation-based blocks (`if`, `fn`, `struct`, etc.) — brace `{ }` style also accepted
- `let` / `let mut` bindings, immutable by default
- `if` / `elif` / `else`, `while`, `for x in`, `break`, `continue`
- `match` — exhaustive pattern matching on literals, enums, and payloads
- `fn` functions with explicit return types; `async fn` for async functions
- `pub`, `const`, `defer`, `unsafe`, `comptime`, `region`
- Attributes: `#[name]` on functions and items

**Type System**
- Primitive types: `Int`, `Float`, `Bool`, `String`
- `struct` with named fields and generic parameters
- `enum` with payload variants (algebraic data types / sum types)
- `trait` declarations and `impl Trait for Type` dispatch
- Built-in `Option[T]` and `Result[T, E]`; `?` propagation operator
- Generic functions `fn foo[T](...)` and generic types
- List type `[T]` with indexing and iteration

**Memory Safety**
- Borrow checker enforced at compile time: move semantics, `&` shared borrows, `&mut` exclusive borrows
- No null pointers — use `Option[T]`
- `unsafe` escape hatch for low-level code
- `region` allocations for arena-style memory management

**Concurrency & Actors**
- `actor` keyword — state machine desugared to struct + step method
- `spawn` lightweight tasks, `async` / `await`
- `supervise` for fault-isolated supervision trees
- Channel-based message passing with `receive`

**C Interoperability**
- `extern "C" { ... }` blocks for direct C FFI
- Links against native `.so` / `.a` libraries; no runtime needed

**Operators & Expressions**
- Arithmetic, comparison, logical (`and` / `or` / `not`)
- Pipe-forward `|>`, range `..`, borrow `&` / `&mut`

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

The stage-0 compiler is written in Rust. You need **Rust 1.78+** and `cargo`.

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

To exercise the stage-1 self-hosted compiler after building:

```sh
./target/release/mom selfhost compiler/src/main.mom -o mom-stage1 --run
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
// Structs and methods
struct Point:
    x: Int
    y: Int

impl Point:
    fn shift(self, dx: Int, dy: Int) -> Point:
        Point { x: self.x + dx, y: self.y + dy }

fn main():
    let p = Point { x: 3, y: 4 }
    let q = p.shift(1, 1)
    print(q.x)   // 4
```

```mom
// Enums and exhaustive pattern matching
enum Shape:
    Circle(Float)
    Rect(Float, Float)

fn area(s: Shape) -> Float:
    match s:
        Circle(r)    => 3.14159 * r * r
        Rect(w, h)   => w * h

fn main():
    print(area(Circle(2.0)))   // 12.566...
    print(area(Rect(3.0, 4.0)))  // 12.0
```

```mom
// Option and Result with ? propagation
fn parse(v: Int) -> Result[Int, String]:
    if v < 0 { Err("negative") } else { Ok(v * 2) }

fn doubled_plus_one(v: Int) -> Result[Int, String]:
    let inner = parse(v)?
    Ok(inner + 1)

fn main():
    match doubled_plus_one(5):
        Ok(n)  => print(n)     // 11
        Err(e) => print(e)
```

```mom
// Traits
trait Shape:
    fn area(self) -> Float

struct Circle:
    radius: Float

impl Shape for Circle:
    fn area(self) -> Float:
        3.14159 * self.radius * self.radius

fn main():
    let c = Circle { radius: 2.0 }
    print(c.area())
```

```mom
// Actor — state machine with message passing
enum Msg:
    Inc
    Add(Int)

actor Counter:
    state count: Int
    receive:
        Inc    => Counter { count: self.count + 1 }
        Add(n) => Counter { count: self.count + n }

fn main():
    let mut c = Counter { count: 0 }
    c = c.step(Inc)
    c = c.step(Add(10))
    print(c.count)   // 11
```

Both indentation and `{ }` brace style are accepted.

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

# Self-host bootstrap
mom selfhost  <file.mom> [-o OUT] [--run] # drive stage-1 (mom-in-mom)
                                          # compiler end-to-end and link

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

| Stage | Description | Status |
|-------|-------------|--------|
| Stage 0 | Rust-based bootstrap compiler — lexer, parser, type checker, borrow checker, interpreter, C codegen | **done** |
| Stage 1 | mom-in-mom compiler (`compiler/src/main.mom`) — compiles a subset of mom to C | **active** |
| Stage 2 | Native LLVM / custom codegen backend; full self-hosting | planned |

The compiler currently targets C as an intermediate language and links via `cc`.
Full self-hosting (the mom compiler written and compiled entirely in mom) is the
end goal. See [`docs/plan.md`](docs/plan.md) and
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
