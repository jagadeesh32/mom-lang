# mom — C and C++ Interoperability

mom treats interoperability with C and C++ as a **first-class feature**,
not a bolted-on extension. The goals:

1. Calling C code is **a single declaration** away, with no glue
   generators and no IDL.
2. Calling C++ code is a *first-class* operation on a curated subset of
   classes, virtual methods, templates, and exceptions.
3. The boundary is **type-safe by default** and **performance-zero in
   the common case** (no marshalling on primitives).
4. Existing C / C++ codebases can be incrementally migrated to mom file
   by file, function by function.

---

## 1. Calling C

### Declaring C symbols

```mom
extern c "m" {
    fn cos(x: Float) -> Float
    fn sin(x: Float) -> Float
    fn pow(base: Float, exp: Float) -> Float
}

fn main() {
    print(cos(0.0))         // 1
    print(pow(2.0, 10.0))   // 1024
}
```

- `extern c` declares C ABI functions.
- The optional string literal is the library tag passed to the linker
  (`-lm` here). Omit it to assume the symbol resolves in the main
  executable.
- The function bodies live in the C source; mom only sees signatures.

### Type mapping

| mom               | C                    |
|-------------------|----------------------|
| `Int`, `Int64`    | `int64_t`            |
| `Int32`           | `int32_t`            |
| `UInt8` / `Byte`  | `uint8_t`            |
| `Float`           | `double`             |
| `Float32`         | `float`              |
| `Bool`            | `_Bool`              |
| `Char`            | `uint32_t` (Unicode) |
| `&T`              | `const T*`           |
| `&mut T`          | `T*`                 |
| `*T`              | `T*` (unchecked)     |
| `String`          | UTF-8 borrow `(ptr, len)` |
| `[T]` slice       | `(T* ptr, size_t len)` struct |

mom strings are length-prefixed UTF-8 slices — they are **not** zero
terminated. To pass to a C `char*` API, use `.as_cstr()`:

```mom
extern c { fn puts(s: *Char) -> Int }
fn main() { unsafe { puts("hello".as_cstr()) } }
```

### Calling mom from C

```mom
pub extern "C" fn mom_add(a: Int, b: Int) -> Int { a + b }
```

`extern "C"` on a `pub` mom function exports it under the C ABI. From C:

```c
extern int64_t mom_add(int64_t, int64_t);
```

### Memory ownership across the boundary

The default rule is **producer owns**:

- A C function returning a `T*` that is documented as "caller frees"
  is declared as returning `*T` (unsafe raw pointer). The mom code
  must wrap it in a `Box[T]` or release it explicitly.
- A C function returning `const T*` is read-only; declare it as
  `&T` and the borrow checker takes over.
- A mom value passed by reference (`&T`) is alive at least until the
  C call returns. Storing the pointer past the call requires `Box[T]`.

### Build integration

```toml
# mom.toml
[build]
c.sources = ["src/native/parser.c", "src/native/utf8.c"]
c.include = ["src/native/include"]
c.flags   = ["-O2", "-DUSE_SIMD=1"]
```

The mom build system invokes the project's C compiler (`cc` or
configurable) and links the result into the final binary.

---

## 2. Calling C++

C++ is wider than C, so mom supports a **curated, well-defined subset**:

- Free functions (`extern cpp { fn … }`).
- Concrete classes with public methods.
- Virtual methods through generated vtables.
- `std::string`, `std::vector<T>`, `std::span<T>`, `std::optional<T>`
  bridge automatically.
- Exceptions are **converted to `Result[T, CppError]`** at the
  boundary; they never unwind into mom code.
- Templates are instantiated **by the C++ compiler**; mom imports the
  resulting concrete types.

```mom
extern cpp "rocksdb" {
    type DB

    fn DB.open(path: String) -> Result[DB, CppError]
    fn DB.put(self, key: [Byte], val: [Byte]) -> Result[(), CppError]
    fn DB.get(self, key: [Byte]) -> Result[[Byte], CppError]
    fn DB.close(self)
}

fn main() -> Result[(), CppError] {
    let db = DB.open("/tmp/db")?
    db.put(b"hello", b"world")?
    let v = db.get(b"hello")?
    print(v)
    db.close()
    Ok(())
}
```

### What is *not* supported (yet)

- Multiple inheritance with virtual bases.
- C++ template parameter packs at the mom signature level (use
  concrete instantiations).
- Non-trivial destructors that throw.
- ABI-fragile types from libraries with no stable ABI guarantee
  (these need a thin C wrapper).

### How it works under the hood

The mom build driver invokes `clang++ -emit-llvm` on the header pack
referenced by `extern cpp` and the chosen wrapper file. The compiler
extracts mangled symbols, classes, and vtables, then emits mom-side
stubs and the same LLVM IR is linked into the final binary. There is
**no shadow file** the user has to maintain by hand — the toolchain is
the source of truth.

---

## 3. Building libraries

mom can produce:

- `cdylib` — a shared library exporting `extern "C"` symbols.
- `staticlib` — a `.a` archive embeddable in other languages.
- `bin` — a native executable (default).

```toml
[lib]
crate-type = "cdylib"
exports    = ["mom_add", "mom_init"]
```

The result is a **plain `.so` / `.dylib` / `.dll`** that other languages
(Python, Node, Go, Rust, C, C++, Java via JNI, Swift, …) can call
without any mom runtime dependency in the consumer's process — beyond
the optional concurrency runtime if the exported symbols themselves
spawn tasks.

---

## 4. Migration recipe — adding mom to a C/C++ project

1. Build the existing C/C++ as today.
2. Add a `mom.toml` with `[build] c.sources = …`.
3. Add a thin `extern "C"` shim per migrated function: declare it in
   mom, point a C header at it.
4. Replace one C function at a time with a mom implementation.
5. Repeat. Keep tests green at each step.

This recipe avoids the all-or-nothing rewrite that kills most
migrations. The same project can have a `core/` written in C++, a
`net/` written in C, and a `services/` written in mom, all sharing the
same build and binary.
