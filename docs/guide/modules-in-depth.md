# Modules In Depth

A Mom module is both a **namespace** and a **visibility boundary**. It groups related types, functions, and constants under a path-addressable name, and controls which items are reachable from outside via `pub`.

---

## Inline Module Declaration

Define a module inline using `module <name> { ... }` (or the colon/indent style):

```mom
module geometry {
    pub struct Point {
        x: Float
        y: Float
    }

    pub fn distance(a: Point, b: Point) -> Float {
        let dx = a.x - b.x
        let dy = a.y - b.y
        // sqrt not yet in scope here — import or qualify
        pow(dx * dx + dy * dy, 0.5)
    }

    fn _internal_helper() -> Int { 42 }   // private — not visible outside
}
```

The colon/indent form (used in the `examples/modules.mom` sketch) is syntactically identical:

```mom
module net:
    pub struct Address:
        host: String
        port: Int

    pub fn pretty(addr: Address) -> String:
        addr.host + ":" + to_string(addr.port)
```

Grammar:

```ebnf
module_decl = "module" IDENT "{" item* "}"
```

Items without `pub` are private to the module. Items marked `pub` are accessible to importers.

---

## File-Based Modules

In a real project, each `.mom` file under `src/` **is** its own module. The module name derives from the file path relative to `src/`:

| File path              | Module path       |
|------------------------|-------------------|
| `src/handler.mom`      | `handler`         |
| `src/handler/auth.mom` | `handler.auth`    |
| `src/db/pool.mom`      | `db.pool`         |

No explicit `module` declaration is needed in the file — the path determines the name. A `pub` item in `src/db/pool.mom` is importable as `db.pool.Pool`.

---

## Nested Modules

Modules can be nested arbitrarily, either inline or via directory hierarchy:

```mom
module net {
    pub module http {
        pub struct Request {
            method: String
            path:   String
        }

        pub fn get(url: String) -> Request {
            Request { method: "GET", path: url }
        }
    }

    pub module ws {
        pub struct Frame { data: [Byte] }
    }
}
```

Access the nested item without import:

```mom
let req = net.http.get("/api/users")
```

Or import selectively:

```mom
import net.http.{Request, get}
let req = get("/api/users")
```

---

## `pub` Visibility

`pub` controls external visibility. Without it, an item is module-private:

```mom
module crypto {
    pub fn hash(data: [Byte]) -> [Byte] { ... }   // visible outside
    fn _expand_key(key: [Byte]) -> [Byte] { ... }  // private
}
```

| Placement | Visible to |
|-----------|-----------|
| (no `pub`) | Only items within the same module |
| `pub` | All importers (the whole crate) |

`pub` applies to functions, structs, struct fields, enums, constants, and nested modules:

```mom
module config {
    pub struct Settings {
        pub host: String    // field is also public
        port: Int           // field is private — only readable inside config
    }
}
```

---

## Selective Import: `import path.{Name1, Name2}`

Bring specific names into scope:

```mom
import net.http.{Request, get}
import std.collections.{HashMap, BTreeSet}
import std.crypto.{adler32, sha256}

fn main() {
    let req = get("/ping")
    let map: HashMap[String, Int] = HashMap.new()
}
```

Grammar:

```ebnf
import_decl = ( "import" | "use" ) import_path ( "{" import_list "}" )? ";"?
import_path = IDENT ( ( "." | "::" ) IDENT )*
import_list = IDENT ( "," IDENT )* ","?
```

Trailing commas in the import list are allowed.

---

## Wildcard Import: `import path.*`

Import all `pub` items from a module:

```mom
import std.math.*

fn main() {
    print(sqrt(2.0))    // sqrt came from std.math
    print(PI)           // constant from std.math
}
```

Use sparingly — wildcard imports can make it hard to trace where a name came from. Prefer selective imports in library code.

---

## Qualified Access Without Import

Any `pub` item can be used without importing it by writing the full path:

```mom
fn main() {
    let p = geometry.Point { x: 1.0, y: 2.0 }
    let d = geometry.distance(p, geometry.Point { x: 4.0, y: 6.0 })
    print(d)
}
```

This is useful for one-off uses or when avoiding name collisions matters more than brevity.

---

## `use` — Identical to `import`

`use` and `import` are synonyms. Both accept the same syntax:

```mom
use std.io.{File, read_to_string}
import std.io.{File, read_to_string}   // same thing
```

The two keywords exist for ergonomic parity with developers coming from different language backgrounds.

---

## Renaming on Import

Use `as` to give an imported name a local alias:

```mom
import std.crypto.{adler32 as checksum}
import std.collections.{HashMap as Map}
import vendor.legacy.{OldApiClient as Client}

fn main() {
    let cs = checksum(b"hello")
    let m: Map[String, Int] = Map.new()
    let cli = Client.connect("localhost:9000")
}
```

This is especially useful when two modules export the same name:

```mom
import net.http.{Response as HttpResponse}
import net.grpc.{Response as GrpcResponse}
```

---

## Circular Imports

Mom does **not** allow circular imports. If module `A` imports `B` and `B` imports `A`, the compiler reports an error at the point where the cycle closes.

To resolve a cycle, extract the shared types into a third module that neither `A` nor `B` imports from each other:

```
Before:   A ←→ B  (cycle)
After:    A → common, B → common  (no cycle)
```

---

## Prelude — Always in Scope

These items are available in every Mom file without any import:

| Name | What it is |
|------|-----------|
| `Option[T]` | Optional value type |
| `Result[T, E]` | Success-or-error type |
| `Some(v)` | `Option` constructor |
| `None` | Empty `Option` |
| `Ok(v)` | `Result` success constructor |
| `Err(e)` | `Result` error constructor |
| `print(v)` | Write to stdout |
| `panic(msg)` | Unrecoverable error |

Everything else — `HashMap`, `Vec`, `File`, etc. — must be imported explicitly.

---

## The `std` Standard Library Hierarchy

| Module path | Contents |
|-------------|----------|
| `std.io` | File I/O, streams, stdin/stdout |
| `std.fs` | Filesystem operations |
| `std.net` | TCP/UDP sockets, DNS |
| `std.net.http` | HTTP client and server |
| `std.collections` | `HashMap`, `BTreeMap`, `Vec`, `Set` |
| `std.math` | Math functions and constants |
| `std.crypto` | Hashing, HMAC, AES, ChaCha |
| `std.sync` | Channels, mutexes, semaphores |
| `std.time` | Clocks, durations, timestamps |
| `std.env` | Environment variables, args |
| `std.str` | String utilities |
| `std.fmt` | Formatting and serialization |

---

## Full Worked Example

The following reproduces and extends the pattern from `examples/modules.mom`:

```mom
// modules.mom — inline module declaration and import.
//
// In production, modules live in their own files (`net/http.mom`).
// This example shows the same syntax in a single file for clarity.

module net:
    pub struct Address:
        host: String
        port: Int

    pub fn pretty(addr: Address) -> String:
        addr.host + ":" + to_string(addr.port)

import net.{Address, pretty}

fn main():
    let a = Address { host: "127.0.0.1", port: 8080 }
    print(pretty(a))    // 127.0.0.1:8080
```

Extended version with nested modules and renaming:

```mom
module app {
    pub module config {
        pub struct Cfg {
            pub host: String
            pub port: Int
        }

        pub fn default() -> Cfg {
            Cfg { host: "0.0.0.0", port: 8080 }
        }
    }

    pub module handler {
        import app.config.{Cfg}

        pub fn handle(cfg: &Cfg, path: String) -> String {
            "routing " + path + " on " + cfg.host
        }
    }
}

import app.config.{default as default_cfg}
import app.handler.{handle}

fn main() {
    let cfg = default_cfg()
    print(handle(&cfg, "/api/ping"))
    // routing /api/ping on 0.0.0.0
}
```

---

## Quick Reference

| Syntax | Meaning |
|--------|---------|
| `module foo { ... }` | Declare inline module `foo` |
| `pub fn bar()` | Export `bar` from current module |
| `import a.b.{X, Y}` | Bring `X` and `Y` into scope |
| `import a.b.*` | Wildcard: all `pub` items from `a.b` |
| `use a.b.{X}` | Same as `import` |
| `import a.b.{X as Z}` | Import `X` and call it `Z` |
| `a.b.X { ... }` | Qualified access without import |
