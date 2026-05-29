# Project Structure

Mom's build system is opinionated about layout and convention so that `mom build`, `mom test`, and `mom run` work without configuration in the common case. This page covers everything from scaffolding a new project to cross-compiling for a different target.

---

## Creating a New Project

### `mom new <name>`

Scaffolds a new binary project in a fresh directory:

```sh
mom new my-service
cd my-service
mom run
```

Generates:

```
my-service/
├── mom.toml
├── mom.lock
└── src/
    └── main.mom
```

`main.mom` contains a starter `fn main()`. `mom.lock` is empty until you add a dependency.

### `mom init`

Initializes a Mom project in the **current directory** — useful when adding Mom to an existing codebase:

```sh
cd existing-project
mom init
```

Creates `mom.toml` and `src/main.mom` if neither exists. Existing files are left untouched.

---

## Directory Layout

A typical Mom project after adding tests, benchmarks, and native sources:

```
my-service/
├── mom.toml                  # project manifest (hand-edited)
├── mom.lock                  # generated lockfile (commit this)
├── src/
│   ├── main.mom              # binary entry point
│   ├── handler.mom           # module: handler
│   └── handler/
│       └── auth.mom          # module: handler.auth
├── tests/
│   └── integration.mom       # integration tests
├── benches/
│   └── hot_path.mom          # microbenchmarks
├── examples/
│   └── demo.mom              # runnable examples
├── std/                      # vendored std override (rare)
└── target/
    ├── mom-cache/            # incremental build cache
    └── release/              # compiled binary and artifacts
```

Every directory may contain a `mod.mom` to expose nested modules. Importing `handler.auth` resolves to `src/handler/auth.mom`.

---

## The `mom.toml` Manifest

`mom.toml` is the single source of truth for the project. All fields have sensible defaults; only `[package]` is required.

### `[package]`

```toml
[package]
name    = "my-service"
version = "0.2.0"
edition = "2025"
```

| Field     | Type   | Default  | Description                                   |
|-----------|--------|----------|-----------------------------------------------|
| `name`    | string | required | Package name (lowercase, hyphens allowed)     |
| `version` | string | required | SemVer version string                         |
| `edition` | string | `"2025"` | Mom language edition (controls syntax/stdlib) |

### `[build]`

Controls native code integration:

```toml
[build]
c.sources = ["src/native/parser.c", "src/native/utf8.c"]
c.include  = ["src/native/include"]
c.flags    = ["-O2", "-DUSE_SIMD=1"]
```

| Field        | Type       | Description                                   |
|--------------|------------|-----------------------------------------------|
| `c.sources`  | `[string]` | C source files to compile and link            |
| `c.include`  | `[string]` | Extra include directories for C compilation   |
| `c.flags`    | `[string]` | Additional compiler flags for C files         |

### `[dependencies]`

Packages from the Mom registry:

```toml
[dependencies]
http    = "1.4"
json    = "0.9"
log     = { version = "0.4", features = ["timestamps"] }
```

Exact versions are pinned in `mom.lock` after the first `mom build`. Commit `mom.lock` so teammates and CI reproduce the same build.

### `[lint]`

Configure lint rules for the project:

```toml
[lint]
deny  = ["unused_imports", "dead_code"]
warn  = ["missing_docs"]
allow = ["clippy::needless_return"]
```

### `[features]`

Optional compilation flags for conditional code:

```toml
[features]
default  = ["logging"]
logging  = []
metrics  = ["dep:prometheus"]
tls      = ["dep:openssl"]
```

Enable from the command line: `mom build --features tls,metrics`

### `[lib]`

When the project produces a library rather than a binary:

```toml
[lib]
crate-type = "cdylib"          # or "staticlib"
exports    = ["mom_add", "mom_init"]
```

### Full Example

```toml
[package]
name    = "auth-service"
version = "1.0.0"
edition = "2025"

[build]
c.sources = ["vendor/argon2/src/argon2.c"]
c.include  = ["vendor/argon2/include"]
c.flags    = ["-O3", "-march=native"]

[dependencies]
http = "1.4"
json = "0.9"

[features]
default = ["http"]
http    = []
grpc    = ["dep:grpc"]

[lint]
deny = ["unused_imports"]
```

---

## Running and Building

| Command                           | What it does                                             |
|-----------------------------------|----------------------------------------------------------|
| `mom run`                         | Build (debug) and run `src/main.mom`                     |
| `mom run -- arg1 arg2`            | Run with arguments                                       |
| `mom build`                       | Compile a debug binary to `target/debug/<name>`          |
| `mom build --release`             | Compile an optimized binary to `target/release/<name>`   |
| `mom test`                        | Build and run all `tests/` and inline `#[test]` blocks   |
| `mom test --filter auth`          | Run only tests whose names contain `auth`                |
| `mom bench`                       | Run benchmarks in `benches/`                             |
| `mom clean`                       | Remove `target/`                                         |
| `mom fmt`                         | Format all `.mom` source files                           |
| `mom lint`                        | Run the linter                                           |

---

## Multi-File Projects

Each `.mom` file is its own module. The module name matches the file path relative to `src/`, with slashes replaced by dots:

| File path             | Module name       |
|-----------------------|-------------------|
| `src/main.mom`        | (entry point)     |
| `src/handler.mom`     | `handler`         |
| `src/handler/auth.mom`| `handler.auth`    |
| `src/db/pool.mom`     | `db.pool`         |

Import as:

```mom
import handler.{process_request}
import handler.auth.{verify_token}
import db.pool.{Pool, connect}
```

### Directory Modules

A directory can expose a unified public surface via `mod.mom`:

```
src/
└── db/
    ├── mod.mom        ← re-exports pool, query, migrate
    ├── pool.mom
    ├── query.mom
    └── migrate.mom
```

`src/db/mod.mom`:

```mom
import db.pool.{Pool, connect}
import db.query.{Query, run}
import db.migrate.{run_migrations}

pub use Pool
pub use connect
pub use Query
pub use run
pub use run_migrations
```

Consumers then just:

```mom
import db.{Pool, connect, run_migrations}
```

---

## The `target/` Directory

| Path                         | Contents                                      |
|------------------------------|-----------------------------------------------|
| `target/debug/<name>`        | Unoptimized binary with debug info            |
| `target/release/<name>`      | Optimized release binary                      |
| `target/mom-cache/`          | Incremental compilation cache                 |
| `target/mom-cache/~/.cache/mom/` | Shared cross-project cache (symlinked) |

The cache is content-addressed and keyed by source hash, compiler version, flags, and target triple. A no-op rebuild of a 1M-LOC project completes in well under a second. Add `target/` to `.gitignore`.

---

## Cross-Compilation

Specify a target triple with `--target`:

```sh
mom build --target aarch64-linux
mom build --target x86_64-windows
mom build --target wasm32-wasi
```

The toolchain resolves the appropriate linker and system libraries. For targets that require a sysroot:

```toml
[toolchain]
target  = "aarch64-linux"
sysroot = "/opt/sysroots/aarch64"
linker  = "aarch64-linux-gnu-gcc"
```

Pre-built standard library artifacts for common targets are downloaded automatically on first use.

---

## Project Conventions

| Convention | Rule |
|------------|------|
| Source encoding | UTF-8, no BOM |
| File names | `snake_case.mom` |
| Module names | `snake_case` |
| Type names | `PascalCase` |
| Function/variable names | `snake_case` |
| Constants | `SCREAMING_SNAKE_CASE` |
| Test files | In `tests/` or inline `#[test]` blocks at the bottom of each module |
| Example files | One file per concept in `examples/` |
| Benchmark files | One file per scenario in `benches/` |

### Test Organization

Inline unit tests live at the bottom of the file they test, gated by `#[test]`:

```mom
fn add(a: Int, b: Int) -> Int { a + b }

#[test]
fn test_add() {
    assert(add(2, 3) == 5)
    assert(add(-1, 1) == 0)
}
```

Integration tests in `tests/` can import any `pub` item from the project and run as standalone binaries:

```mom
// tests/integration.mom
import my_service.handler.{process_request}

#[test]
fn test_round_trip() {
    let req = make_test_request()
    let resp = process_request(req)
    assert(resp.status == 200)
}
```
