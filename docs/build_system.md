# mom — Build System & Package Manager

mom ships a single tool, `mom`, that drives:

- compilation (debug + release)
- the package manager (lockfile, registry, vendoring)
- cross-compilation
- caching (content-addressed, network-shareable)
- tests, benchmarks, fuzzing
- docs, formatter, linter, LSP

The build system is **first-class, not an afterthought**. Compile speed
is a feature, not a side effect.

---

## 1. Project layout

```
my-service/
├── mom.toml                # project manifest
├── mom.lock                # generated lockfile (commit this)
├── src/
│   ├── main.mom            # binary entrypoint
│   ├── handler.mom
│   └── handler/
│       └── auth.mom
├── tests/
│   └── integration.mom
├── benches/
│   └── hot_path.mom
└── examples/
    └── demo.mom
```

Every directory may contain a `mod.mom` to expose nested modules.
Importing `handler.auth` resolves to `src/handler/auth.mom`.

## 2. Manifest

```toml
[package]
name        = "my-service"
version     = "0.3.0"
edition     = "2026"
authors     = ["…"]
license     = "Apache-2.0"

[dependencies]
http        = "1.4"
postgres    = { version = "0.7", features = ["tls"] }
metrics     = { path  = "../metrics" }
fastlog     = { git   = "https://git.example/fastlog", tag = "v2.0" }

[dev-dependencies]
test-utils  = "0.2"

[features]
default     = ["std", "tls"]
embedded    = []

[build]
target      = "x86_64-linux"          # cross compile from any host
optimize    = "release"
lto         = "thin"
debug-info  = "lines"
panic       = "abort"

[build.c]
sources     = ["native/decode.c"]
include     = ["native/include"]
flags       = ["-O2"]
```

## 3. Commands

```sh
mom new       my-service        # scaffold
mom init                        # in-place scaffold
mom build                       # debug build
mom build --release             # optimized
mom run -- --port 8080          # build & run
mom test                        # run all tests
mom test handler::auth          # filter
mom bench                       # benchmarks
mom fmt                         # format the tree
mom lint                        # lints
mom check                       # type-check only (CI gate)
mom doc --open                  # generate & open API docs
mom add http@1.4                # update mom.toml and lock
mom upgrade                     # bump within semver
mom audit                       # security advisory check
mom publish                     # registry push
mom version-bump patch          # semver bump
mom vendor                      # mirror deps into ./vendor
```

## 4. Build pipeline (release)

1. **Manifest resolution** — read `mom.toml`, compute the full
   dependency graph from `mom.lock`.
2. **Module discovery** — walk `src/`, build the module tree.
3. **Parallel parse** — each `.mom` file is parsed independently.
4. **Cross-module name resolution** — single sequential pass over the
   already-parsed ASTs.
5. **Parallel type check + lowering** — modules are checked in
   topological order; independent strongly-connected components run
   in parallel.
6. **Generic monomorphization** — performed once for the whole
   program, cached per `(generic_name, type_args)` tuple.
7. **Parallel codegen** — LLVM IR per module → object files in
   parallel.
8. **Linker driver** — link the object files and any C/C++ archives.

## 5. Caching strategy

The cache is content-addressed and keyed by:

- the textual hash of every `.mom` source plus its transitive imports
- compiler version
- relevant compiler flags
- target triple
- feature flags

Cache levels:

- **In-memory** — for a single `mom` invocation.
- **On-disk** — `~/.cache/mom/` shared across projects on the same
  machine.
- **Distributed** — optional remote cache (`mom.toml [cache] remote =
  "https://cache.example/"`). One CI build serves the whole team.

Cache hits avoid even reopening source files; a no-op rebuild of a
1M-LOC project completes in well under a second.

## 6. Cross-compilation

```sh
mom build --target aarch64-linux
mom build --target wasm32-wasi
mom build --target x86_64-windows
mom build --target armv7-none-eabihf        # embedded
```

mom ships a static linker driver per supported target. The native
toolchain (clang, lld) is downloaded on demand and cached.

## 7. Reproducible builds

- The build is deterministic: same inputs → same bytes out.
- Source filenames in debug info are rewritten relative to the project
  root, not absolute.
- Linker timestamps are zeroed (or set to `SOURCE_DATE_EPOCH`).
- The build records every input hash in a sidecar `*.mombuild.json`
  for SLSA-style attestation.

## 8. Package registry

The default registry is community-run and federated:

- A user-managed mirror is a single HTTP server serving signed indexes.
- A private corporate registry is the same shape.
- Each crate version is **content-addressed** by hash.
- `mom audit` cross-checks the local lock against a publicly hosted
  advisory feed.

## 9. Workspaces

```toml
# top-level mom.toml
[workspace]
members = [
    "crates/core",
    "crates/http",
    "crates/runtime",
    "services/api",
]

[workspace.dependencies]
log = "0.4"
```

Workspaces share a lockfile, target directory, and cache. A single
`mom build` builds everything in topological order.

## 10. Performance targets

The build system is optimized around three numbers:

| Scenario                        | Goal                |
|---------------------------------|---------------------|
| No-op rebuild                   | < 50 ms             |
| Edit one file, hot rebuild      | < 500 ms / 100 kLOC |
| Clean release build, large repo | < 60 s / 1 MLOC     |

Compare: comparable Rust workloads are 5–20× slower; comparable C++
workloads are 3–10× slower at link time.
