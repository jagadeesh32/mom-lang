# Interpreter, Compiler, and Toolchain

Mom ships a single `mom` binary that acts as interpreter, compiler, formatter, linter, language server, and package manager. This page documents every command.

---

## How It Works

Mom has two execution paths:

```
Source (.mom)
     │
     ├──► Interpreter (mom run)
     │     Lexer → Parser → Type Checker → Borrow Checker → Tree-Walker
     │     Fast startup. No native code generated.
     │
     └──► Native Compiler (mom build)
           Lexer → Parser → Type Checker → Borrow Checker → C Codegen
           Produces a native binary via: C source → cc → linker
```

Both paths share the same front end (lexer, parser, type checker, borrow checker). The difference is only in the back end.

---

## Bootstrap Stages

Mom is self-hosted. The compiler exists in three stages:

| Stage | Name | Written in | Produces |
|---|---|---|---|
| Stage 0 | `mom` binary | Rust | Interprets + compiles Mom programs |
| Stage 1 | `compiler/src/main.mom` | Mom | C source from Mom programs |
| Stage 2 | *(planned)* | Mom | Full native binary |

**The self-hosting fixed point:** Stage-0 can compile Stage-1, which produces the same C output as Stage-1 compiled by itself. This is verified by `compiler/self_host_test.sh`.

---

## CLI Commands

### Running Programs

#### `mom run <file.mom>`

Interpret and run a program immediately. No compilation step. Best for development and exploration.

```bash
mom run hello.mom
mom run examples/fib.mom
```

#### `mom build <file.mom> [-o OUTPUT]`

Compile to a native binary via the C backend. Requires a C compiler (`cc` or `$CC`).

```bash
mom build hello.mom              # produces ./hello
mom build hello.mom -o /tmp/hi   # custom output path
```

The output binary is a self-contained native executable with no Mom runtime dependency.

#### `mom build-run <file.mom>`

Compile and immediately execute. Equivalent to `mom build` + running the result.

```bash
mom build-run examples/fib.mom
```

#### `mom emit-c <file.mom>`

Print the generated C source to stdout without compiling. Useful for debugging codegen.

```bash
mom emit-c hello.mom
```

---

### Inspection

#### `mom tokens <file.mom>`

Print the token stream produced by the lexer.

```bash
mom tokens hello.mom
# Keyword(Fn)   "fn"
# Ident         "main"
# ...
```

#### `mom ast <file.mom>`

Print the parsed AST in a human-readable format.

```bash
mom ast hello.mom
```

#### `mom check <file.mom>`

Run the type checker and borrow checker without executing. Exits 0 on success.

```bash
mom check myfile.mom
# ok: 3 function(s), 12 known type(s)
```

Use in CI to verify code correctness without building or running.

---

### Code Quality

#### `mom fmt <file.mom> [--check]`

Format the source file in place.

```bash
mom fmt myfile.mom         # format in place
mom fmt --check myfile.mom # check only, exit 1 if reformatting needed
```

#### `mom lint <file.mom>`

Lint the file with rules from `mom.toml` (or defaults). Prints warnings and errors.

```bash
mom lint myfile.mom
```

Common lint rules:
- Unused variables
- Unreachable code
- Missing type annotations on public items
- TODO/FIXME markers

#### `mom doc <file.mom>`

Extract doc comments and emit a Markdown API reference.

```bash
mom doc mylib.mom > docs/mylib.md
```

---

### Testing and Benchmarks

#### `mom test [dir]`

Discover and run all `#[test]` functions in `.mom` files. Defaults to the current directory.

```bash
mom test           # run all tests
mom test tests/    # run tests in a specific directory
```

Output:

```
running 5 tests
test lexer::tokenizes_keywords ... ok
test parser::parses_struct ... ok
test typechecker::infers_int ... ok
test borrow::rejects_double_mut ... ok
test integration::fib_10 ... ok

test result: ok. 5 passed; 0 failed
```

Writing a test:

```mom
#[test]
fn test_add():
    assert(2 + 2 == 4)
    assert(add(3, 7) == 10)
```

#### `mom bench [dir]`

Run all `#[bench]` benchmarks.

```bash
mom bench
```

Writing a benchmark:

```mom
#[bench]
fn bench_fib_30():
    fib(30)
```

#### `mom prof <file.mom>`

Profile with a call trace. Prints a function call frequency table.

```bash
mom prof examples/fib.mom
```

---

### Project Management

#### `mom new <directory>`

Scaffold a new project:

```bash
mom new my_project
```

Creates:

```
my_project/
├── mom.toml
├── src/
│   └── main.mom
└── tests/
```

#### `mom init [directory]`

Initialize a project in the current (or specified) directory:

```bash
cd existing_dir
mom init
```

#### `mom pkg list`

List all dependencies in `mom.toml`.

#### `mom pkg add <name>`

Add a dependency from the registry.

```bash
mom pkg add json
mom pkg add http-server
```

#### `mom pkg remove <name>`

Remove a dependency.

#### `mom pkg audit`

Check dependencies for known security vulnerabilities.

---

### Language Server and Debugger

#### `mom lsp`

Start the Language Server Protocol server on stdio. Used by IDE extensions.

```bash
mom lsp
```

Supports:
- Hover (type information)
- Go-to-definition
- Completions
- Inline diagnostics
- Format on save

#### `mom dbg`

Start the Debug Adapter Protocol server on stdio. Used by IDE debuggers.

```bash
mom dbg
```

---

### Self-Hosting

#### `mom selfhost <file.mom> [-o OUTPUT]`

Compile a Mom program using the Stage-1 self-hosted compiler (Mom-in-Mom). Requires Stage-0 to work first.

```bash
mom selfhost compiler/src/main.mom -o mom-stage1
```

#### `mom version`

Print the current version and build info.

```bash
mom version
# mom 0.3.0
```

#### `mom help`

Print the full help text.

```bash
mom help
```

---

## Environment Variables

| Variable | Description | Default |
|---|---|---|
| `CC` | C compiler to use for `mom build` | `cc` |
| `MOM_INPUT` | Input file path for self-hosted compiler | — |
| `MOM_OUTPUT` | Output C file path for self-hosted compiler | — |

---

## mom.toml — Project Manifest

```toml
[package]
name    = "my_project"
version = "0.1.0"
edition = "2026"

[build]
# Optional: extra C sources to link
c.sources = ["src/native/helper.c"]
c.include = ["src/native/include"]
c.flags   = ["-O2"]

[dependencies]
# Add packages from the registry (planned)
# json = "1.0"
# http = { version = "2.0", features = ["async"] }

[lint]
# Lint rules (mom lint)
unused_vars    = "warn"
missing_docs   = "warn"
unreachable    = "error"
```

---

## Build Cache

Native builds are cached under `target/mom-cache/`. The cache key includes:

- The source file content
- The C compiler name and optimization level
- The **compiler binary identity** (length + modification time)

This means:
- Rebuilding the same source with the same compiler hits the cache.
- After upgrading Mom, the cache is automatically invalidated and a fresh build runs.

To clear the cache:

```bash
rm -rf target/mom-cache/
```
