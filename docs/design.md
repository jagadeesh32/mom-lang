# mom — Surface Language Design

This document describes the **observable language**: lexical structure,
declarations, expressions, statements, and the design intent behind each.
For the executable grammar, see [grammar.ebnf](grammar.ebnf). For the
internal compiler architecture see [compiler.md](compiler.md).

The bootstrap interpreter (in this repo) implements a substantial
executable subset; constructs marked **(parsed only)** are accepted by the
front end and reserved by the type checker but their runtime semantics
land in the native backend.

---

## 1. Files and modules

- File extension: `.mom`
- Encoding: UTF-8
- Each file is a **module**. The file `foo/bar.mom` declares
  `module foo.bar` implicitly.
- An explicit nested module is written with `module Name { … }`.
- Cross-module references use `import` or `use`:
  ```mom
  import std.io
  import std.collections.{Map, Set}
  use net.http as http
  ```
- Visibility: declarations are module-private by default; `pub` exposes
  them outside the module.

## 2. Lexical structure

```
identifier   = letter (letter | digit | '_')*
integer      = digit (digit | '_')*
float        = digit+ '.' digit+
string       = '"' ( char | escape )* '"'
escape       = '\n' | '\r' | '\t' | '\0' | '\\' | '\"'
line-comment = '//' ... newline
block-comment = '/* ... */'   (nestable in the native compiler)
```

Keywords (reserved):

```
actor as async await break comptime const continue defer
else enum extern false fn for from if impl import in let
match module mut pub receive return self spawn struct
supervise trait true type unsafe use where while
```

## 3. Bindings

```mom
let x = 1                    // immutable binding
let mut counter = 0          // mutable binding
const PAGE_SIZE: Int = 4096  // compile-time constant
```

- `let` introduces a name, optionally annotated `let x: Int = …`.
- `let mut` allows rebinding to a new value of the same type.
- `const` requires a literal-ish expression and a type annotation.
- There is no shadowing inside the same scope; nested scopes may shadow.

## 4. Functions

```mom
fn add(a: Int, b: Int) -> Int { a + b }

pub fn map[T, U](xs: [T], f: fn(T) -> U) -> [U] { … }

async fn fetch(url: String) -> Result[Body, HttpError] { … }
```

- Generics are written with `[T, U, …]`.
- Implicit returns: the last expression in a block is its value.
- `return` may be used for early exits.
- `async fn` declares a function returning `Future[T]`; the bootstrap
  interpreter runs the body synchronously.

## 5. Lambdas

```mom
let square = fn(x: Int) => x * x
let area   = fn(w: Float, h: Float) -> Float { w * h }
```

Lambdas capture variables from the enclosing scope by reference if
mutable, by value otherwise.

## 6. Structs

```mom
pub struct Point { x: Float, y: Float }

let p = Point { x: 1.0, y: 2.0 }
print(p.x)

impl Point {
    fn distance(self, other: Point) -> Float {
        let dx = self.x - other.x
        let dy = self.y - other.y
        sqrt(dx * dx + dy * dy)
    }
}
```

- Fields default to module-private; mark with `pub` to expose.
- `self` in method position is the receiver.
- An `impl` block adds methods to a type.

## 7. Enums (sum types) and pattern matching

```mom
pub enum Shape {
    Circle(Float),
    Rect(Float, Float),
    Triangle { base: Float, height: Float },
}

fn area(s: Shape) -> Float {
    match s {
        Circle(r)             => 3.14159 * r * r,
        Rect(w, h)            => w * h,
        Triangle { base, height } => 0.5 * base * height,
    }
}
```

- `Option[T]` and `Result[T, E]` are built-in enums in the prelude:
  ```mom
  enum Option[T] { Some(T), None }
  enum Result[T, E] { Ok(T), Err(E) }
  ```
- `match` is **exhaustive**; the compiler errors on missing variants.

## 8. Traits and `impl`

```mom
pub trait Writer {
    fn write(self, bytes: [Byte]) -> Result[Int, IoError]
    fn flush(self) -> Result[(), IoError]
}

impl Writer for File {
    fn write(self, bytes: [Byte]) -> Result[Int, IoError] { … }
    fn flush(self) -> Result[(), IoError]                  { … }
}
```

Trait dispatch is **static (monomorphized) by default**. Dynamic dispatch
is opt-in via `dyn Writer`.

## 9. Error handling: `Result`, `Option`, `?`

```mom
fn open_config(path: String) -> Result[Config, IoError] {
    let bytes = fs.read(path)?
    parse_config(bytes)
}
```

- No exceptions, no panics in normal code paths.
- `?` propagates `Err` and `None` early.
- Programmer bugs (assertion failures, OOB without bounds checks
  elided) trap rather than unwind.

## 10. Control flow

```mom
if condition { … } else if other { … } else { … }

while running {
    step()
}

for x in xs        { … }   // iterate any IntoIter
for i in 0..n       { … }   // integer range
```

`break` and `continue` terminate the innermost loop. `if`/`match`/blocks
are expressions and have a value.

## 11. Pipelines

```mom
let result = input
    |> parse
    |> validate
    |> compile
```

`x |> f(args…)` is `f(x, args…)`. Pipelines have lower precedence than
arithmetic so they read naturally.

## 12. Concurrency  **(parsed only in interpreter; runtime in native build)**

```mom
async fn fetch(url: String) -> Result[Body, HttpError] { … }

let body = await fetch("https://mom-lang.org")

let task = spawn fetch("https://other")
let body = await task

actor Cache {
    state map: Map[String, Bytes]
    receive {
        Get(key, reply) => reply.send(map.get(key)),
        Put(key, val)   => map.insert(key, val),
    }
}

let cache = spawn Cache()
supervise cache with restart(limit: 3, window: 60s)
```

See [concurrency.md](concurrency.md) for the full model.

## 13. C/C++ interop  **(parsed only in interpreter; runtime in native build)**

```mom
extern c "m" {
    fn cos(x: Float) -> Float
    fn sin(x: Float) -> Float
}

extern cpp "rocksdb" {
    type DB
    fn DB.open(path: String) -> Result[DB, CppError]
    fn DB.put(self, key: [Byte], val: [Byte]) -> Result[(), CppError]
}
```

See [interop.md](interop.md).

## 14. Compile-time computation

```mom
comptime fn pow2(n: Int) -> Int {
    if n == 0 { 1 } else { 2 * pow2(n - 1) }
}

const PAGE: Int = pow2(12)   // 4096, computed at compile time
```

mom uses a **comptime evaluator** rather than macros: the same language
runs at compile time over compile-time values. No separate macro DSL is
needed.

## 15. Unsafe blocks

```mom
unsafe {
    let raw = ffi.malloc(1024)
    ffi.free(raw)
}
```

`unsafe` opts a small region out of safety guarantees (raw pointers,
manual lifetime management, FFI calls). Code outside `unsafe` cannot be
unsound; `unsafe` is the audit boundary.

## 16. Mutability and ownership

See [memory.md](memory.md) for the full model. Headline:

- Values have a single owner.
- Borrows (`&T`, `&mut T`) are explicit, scoped, non-null, and
  non-aliasing for `&mut`.
- Region allocators handle request-scoped arenas.
- Send between actors moves or copies; aliased shared state requires
  `Atomic`, `Mutex`, or an actor.
