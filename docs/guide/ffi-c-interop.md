# C and C++ Interoperability (FFI)

Mom treats interoperability with C and C++ as a **first-class feature**, not a bolted-on extension. The design goals:

1. Calling C code is **a single declaration** away — no glue generators, no IDL, no code-gen step.
2. Calling C++ code is a first-class operation over a curated, well-defined subset of classes, virtual methods, templates, and exceptions.
3. The boundary is **type-safe by default** and **zero-cost on primitives** — no marshalling overhead.
4. Existing C/C++ codebases can be incrementally migrated to Mom file by file, function by function.

---

## Declaring C Functions

Use an `extern c` block to make C symbols visible to Mom. The optional string literal names the library tag, which the build system passes to the linker as `-l<name>`.

```mom
extern c "m" {
    fn cos(x: Float) -> Float
    fn sin(x: Float) -> Float
    fn pow(base: Float, exp: Float) -> Float
}

fn main() {
    print(cos(0.0))         // 1
    print(sin(0.0))         // 0
    print(pow(2.0, 10.0))   // 1024
}
```

**Key points:**

- `extern c` registers the symbol for the linker. Mom only sees the signature; the body lives in the C source.
- Omit the library string to assume the symbol resolves in the main executable or a previously linked library.
- The EBNF for the block is: `extern_block = "extern" IDENT STRING? "{" extern_item* "}"` — identical syntax regardless of platform.

> **Status note:** The bootstrap interpreter parses `extern c` blocks but raises an error if you call one, because there is no native linker yet. Calls execute once the LLVM backend lands (roadmap Phase 1). The example in `examples/ffi_c_sketch.mom` demonstrates the syntax today.

### Sketch from `examples/ffi_c_sketch.mom`

```mom
// ffi_c_sketch.mom — PLANNED SYNTAX (C FFI)
extern c "m":
    fn cos(x: Float) -> Float
    fn sin(x: Float) -> Float
    fn pow(base: Float, exponent: Float) -> Float

fn main():
    print("FFI declarations parsed; native backend executes the calls.")
```

Both colon-style (indented body) and brace-style (`{ }`) are accepted by the parser.

---

## Type Mapping: Mom ↔ C

| Mom type          | C type                       | Notes                              |
|-------------------|------------------------------|------------------------------------|
| `Int`, `Int64`    | `int64_t`                    | Default integer                    |
| `Int32`           | `int32_t`                    |                                    |
| `UInt8`, `Byte`   | `uint8_t`                    |                                    |
| `Float`           | `double`                     | Default float                      |
| `Float32`         | `float`                      |                                    |
| `Bool`            | `_Bool`                      |                                    |
| `Char`            | `uint32_t`                   | Unicode scalar value               |
| `&T`              | `const T*`                   | Shared borrow — read-only          |
| `&mut T`          | `T*`                         | Exclusive borrow — mutable         |
| `*T`              | `T*` (unchecked)             | Raw pointer — requires `unsafe`    |
| `String`          | `(const char* ptr, size_t len)` | UTF-8 borrow pair, **not** NUL-terminated |
| `[T]` slice       | `(T* ptr, size_t len)`       | Fat pointer — two-word struct      |
| `()`              | `void`                       |                                    |

### Passing Strings to C

Mom `String` values are length-prefixed UTF-8 slices — they are **not** NUL-terminated. To pass to a C API expecting `char*`, call `.as_cstr()`:

```mom
extern c {
    fn puts(s: *Char) -> Int
    fn strlen(s: *Char) -> Int
}

fn main() {
    unsafe {
        puts("hello, world".as_cstr())       // prints and flushes
        print(strlen("hello".as_cstr()))     // 5
    }
}
```

`.as_cstr()` allocates a NUL-terminated copy on the stack (short strings) or heap (long strings). The pointer is valid until the end of the enclosing `unsafe` block.

---

## Exporting Mom Functions to C

Mark a `pub fn` with `extern "C"` to export it under the C ABI:

```mom
pub extern "C" fn mom_add(a: Int, b: Int) -> Int {
    a + b
}

pub extern "C" fn mom_greet(name: *Char) -> () {
    unsafe {
        // construct a Mom String from a C pointer
        let s = String.from_raw(name)
        print("hello, " + s)
    }
}
```

From C, the callers see:

```c
#include <stdint.h>

extern int64_t mom_add(int64_t a, int64_t b);
extern void    mom_greet(const char* name);
```

Mom mangles none of the names when `extern "C"` is present — symbol names are exactly as written.

---

## Memory Ownership Across the Boundary

The default rule is **producer owns**:

| Pattern | Declaration | Who frees |
|---------|-------------|-----------|
| C returns `const T*` (read-only, C-owned) | `-> &T` | C — borrow checker tracks lifetime |
| C returns `T*` (caller must free) | `-> *T` | Mom — wrap in `Box[T]` or call `free` explicitly |
| Mom passes `&T` | borrow | C may read until call returns; must not store |
| Mom passes `Box[T]` | ownership transfer | C owns after the call |

```mom
extern c "mylib" {
    fn mylib_create() -> *MyObj       // caller-frees
    fn mylib_name(obj: &MyObj) -> &Char  // read-only, C-owned lifetime
    fn mylib_free(obj: *MyObj)
}

fn use_obj() {
    unsafe {
        let obj: *MyObj = mylib_create()
        let name = mylib_name(obj)   // &Char — valid until mylib_free
        print(name)
        mylib_free(obj)
    }
}
```

A `*T` always requires an `unsafe` block — it is the signal that you are accepting manual lifetime responsibility.

---

## `unsafe` Blocks

Any operation involving raw pointers (`*T`) or dereferencing a C-returned address must appear inside an `unsafe` block:

```mom
unsafe {
    let raw: *Byte = alloc(1024)
    *raw = 0xFF              // raw dereference
    dealloc(raw)
}
```

`unsafe` is a local contract: the programmer asserts that the enclosed code upholds memory safety. The compiler relaxes its checks inside the block but continues type-checking everything else.

---

## Building C Sources Alongside Mom

Add C files to the build via `mom.toml`:

```toml
[build]
c.sources = ["src/native/parser.c", "src/native/utf8.c"]
c.include  = ["src/native/include"]
c.flags    = ["-O2", "-DUSE_SIMD=1"]
```

The Mom build driver invokes the system C compiler (`cc`, or the compiler specified in `[toolchain]`) and links the result into the final binary. No separate `Makefile` step is needed.

Multiple C files, subdirectories, and glob patterns are all accepted:

```toml
[build]
c.sources = ["vendor/sqlite/sqlite3.c"]
c.include  = ["vendor/sqlite"]
c.flags    = ["-DSQLITE_THREADSAFE=0", "-DSQLITE_DEFAULT_MEMSTATUS=0"]
```

---

## C++ Interoperability

C++ is a wider target than C, so Mom supports a **curated, well-defined subset**:

- Free functions via `extern cpp { fn … }`.
- Concrete classes with public methods.
- Virtual methods through generated vtables.
- `std::string`, `std::vector<T>`, `std::span<T>`, `std::optional<T>` bridge automatically.
- Exceptions are **converted to `Result[T, CppError]`** at the boundary — they never unwind into Mom code.
- Templates are instantiated **by the C++ compiler**; Mom imports the resulting concrete types.

```mom
extern cpp "rocksdb" {
    type DB

    fn DB.open(path: String) -> Result[DB, CppError]
    fn DB.put(self, key: [Byte], val: [Byte]) -> Result[(), CppError]
    fn DB.get(self, key: [Byte]) -> Result[[Byte], CppError]
    fn DB.close(self)
}

fn main() -> Result[(), CppError] {
    let db = DB.open("/tmp/mydb")?
    db.put(b"hello", b"world")?
    let v = db.get(b"hello")?
    print(v)
    db.close()
    Ok(())
}
```

### What Is Not Supported (Yet)

| Feature | Status |
|---------|--------|
| Multiple inheritance | Not supported |
| Operator overloading (beyond `[]`, `()`) | Not supported |
| Template metaprogramming (TMP) | Not supported |
| RTTI / `dynamic_cast` | Not supported |
| Stack-unwinding through Mom frames | Not supported — exceptions convert at boundary |

### How It Works Under the Hood

The Mom build driver invokes `clang++ -emit-llvm` on the header pack referenced by `extern cpp` and a generated wrapper file. The compiler extracts mangled symbols, class layouts, and vtables, then emits Mom-side stubs. The same LLVM IR is linked into the final binary. There is **no shadow file** you maintain by hand — the toolchain is the source of truth.

---

## Producing Libraries for C Consumers

Mom can emit:

- `cdylib` — a shared library (`.so` / `.dylib` / `.dll`) exporting `extern "C"` symbols.
- `staticlib` — a `.a` archive embeddable in other language toolchains.

```toml
[lib]
crate-type = "cdylib"
exports    = ["mom_add", "mom_init", "mom_shutdown"]
```

The result is a plain shared library that Python, Node, Go, Rust, C, C++, Java (via JNI), and Swift can all call without any Mom runtime dependency in the consumer's process — beyond the optional concurrency runtime if exported symbols spawn tasks.

---

## Incremental Migration Recipe

Adding Mom to an existing C/C++ project does not require a rewrite:

1. **Keep the existing build.** Build all C/C++ sources as today.
2. **Add `mom.toml`.** Set `[build] c.sources = [...]` to include the existing C files.
3. **Write a thin shim per function.** Declare each migrated function in Mom with `pub extern "C" fn`, and update the C header to `extern` it.
4. **Replace one function at a time.** Implement it in Mom; delete the C version.
5. **Run tests at each step.** Keep the test suite green throughout.

The same project can have a `core/` in C++, a `net/` in C, and a `services/` in Mom — all sharing the same build graph and final binary. There is no all-or-nothing cutover.

```toml
# mom.toml for a hybrid project
[package]
name    = "myapp"
version = "0.1.0"

[build]
c.sources = [
    "core/engine.c",
    "core/parser.c",
    "net/tcp.c",
]
c.include = ["core/include", "net/include"]
c.flags   = ["-O2"]
```
