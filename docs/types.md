# mom — Type System

mom is a **statically typed**, **strongly typed**, **type-inferring**
language with structural enums (sum types), nominal structs, and trait-based
polymorphism. The native compiler performs whole-program monomorphization
for generics.

## 1. Built-in scalar types

| Type   | Description                                  |
|--------|----------------------------------------------|
| `Int`  | 64-bit signed integer (target-aligned for low-level targets) |
| `Int32`, `Int16`, `Int8` | sized signed integers           |
| `UInt`, `UInt32`, `UInt16`, `UInt8` (`Byte`) | unsigned       |
| `Float`, `Float32` | IEEE-754 64-bit and 32-bit float        |
| `Bool` | `true` / `false`                              |
| `Char` | Unicode scalar value (U+0000..U+10FFFF)       |
| `String` | UTF-8 encoded immutable text                |
| `()`   | the unit type, the only value is `()`         |

The bootstrap interpreter currently implements `Int`, `Float`, `Bool`,
`String`, and `()`. Sized integers are reserved by the parser and will be
added in the native backend.

## 2. Composite types

```mom
[T]                 // list/slice of T  (sequence with O(1) index)
(T, U, V)           // tuple            (heterogeneous, fixed-arity)
fn(T, U) -> V       // function type
&T                  // immutable borrow of T          (planned)
&mut T              // mutable borrow of T            (planned)
*T                  // raw pointer to T               (unsafe-only)
```

## 3. Generic types

Generics use square brackets to avoid `>>` parsing pain and to give
type-application a visually distinct, JSON-like feel.

```mom
fn first[T](xs: [T]) -> Option[T] { … }

struct Buffer[T] { data: [T], len: Int, cap: Int }

trait Container[T] {
    fn add(self, value: T)
    fn count(self) -> Int
}
```

Generics are **monomorphized** by the native compiler. The bootstrap
interpreter treats unresolved type parameters as `Unknown` and skips
strict checking — programs run, but specialization happens at native
codegen time.

## 4. Sum types and `Option` / `Result`

```mom
pub enum Option[T] { Some(T), None }
pub enum Result[T, E] { Ok(T), Err(E) }
```

These two are part of the prelude. `?` propagation is defined on both:

- `Some(x)?  → x`,   `None?  → return None`
- `Ok(x)?    → x`,   `Err(e)? → return Err(e)`

## 5. Traits and bounds

```mom
trait Hash {
    fn hash(self) -> UInt64
}

trait Eq {
    fn eq(self, other: Self) -> Bool
}

fn count[T: Hash + Eq](xs: [T]) -> Map[T, Int] { … }
```

`where` clauses are the alternative syntax for many bounds:

```mom
fn merge[T](a: [T], b: [T]) -> [T] where T: Ord + Clone { … }
```

## 6. Type inference

mom uses **bidirectional inference** in the native compiler:
- Local `let` bindings infer the type from the initialiser.
- Function parameters and return types are always written explicitly.
- Generic type parameters are inferred from arguments.
- Top-level `const`s must annotate their type when the value isn't
  obviously primitive.

The bootstrap interpreter implements a simpler, lenient version:
mismatches on known types (e.g. `Int + String`) error; unknown
generics are accepted.

## 7. Subtyping

mom does **not** have nominal subtyping. The only structural compat:

- A value of type `Option[Int]` is compatible with a binding annotated
  `Option` (nominal-with-erased-args) when interfacing with code that
  hasn't yet been monomorphized. The native compiler removes this
  loophole once full inference lands.
- Trait objects implement structural dispatch via `dyn Trait`.

## 8. Type aliases

```mom
type Bytes  = [Byte]
type UserId = UInt64
type Result[T] = Result[T, AppError]   // partial application
```

## 9. Sized vs unsized

All values have a statically known size. `[T]` is a *slice header*
(pointer + length); the owned-array type is `Array[T, N]` or `Vec[T]`.
Trait objects (`dyn Trait`) carry a vtable pointer; they are
unsized-without-pointer and must be referenced via `Box[dyn Trait]`,
`&dyn Trait`, etc.

## 10. Lifetimes and references — short version

```mom
fn longest(a: &String, b: &String) -> &String { … }
```

mom borrows are non-null and scoped. The compiler infers lifetime
parameters when they are unambiguous; explicit `'a` annotations are
needed only in API boundaries where multiple references could live
different durations. See [memory.md](memory.md).

## 11. Exhaustiveness and totality

- `match` must cover every variant; the compiler will refuse incomplete
  matches.
- Function bodies must produce a value of the declared return type on
  every path. `panic!` and divergent functions return `Never` (a.k.a.
  the empty type) which is compatible with any expected type.

## 12. `Never` and unreachable code

```mom
fn fail(reason: String) -> Never { panic(reason) }

let value = match maybe {
    Some(x) => x,
    None    => fail("unexpected None"),
}
```

## 13. Reflection and `comptime`

There is no runtime reflection. Compile-time introspection is done via
`comptime` (see [design.md](design.md)). The reflection API surface is
intentionally narrow to keep binary sizes predictable.
