# Generics

Generics let you write a single function, struct, or enum that works correctly for many different types, while still being type-checked at compile time. Without generics you would need a separate `identity_int`, `identity_string`, `identity_float` — with generics you write one `identity[T]` and the compiler (or interpreter) handles the rest.

---

## Why Generics?

| Without generics | With generics |
|-----------------|---------------|
| Duplicate code per type | One definition, many types |
| `Any` / runtime casts | Full static type safety |
| Bugs hidden until runtime | Errors caught at compile time |

mom's **native compiler** achieves this via **monomorphisation**: it stamps out a dedicated copy of the code for each concrete type used, so there is zero runtime overhead. The **bootstrap interpreter** accepts the same generic syntax and handles it via dynamic dispatch — programs run correctly, but specialisation happens later.

---

## Generic Functions

Declare type parameters in **square brackets** immediately after the function name, before the ordinary parameter list:

```mom
fn identity[T](value: T) -> T:
    value
```

`T` is a **type parameter** — a placeholder that is filled in when the function is called.

```mom
fn main():
    print(identity(42))        // T = Int   → 42
    print(identity("hello"))   // T = String → hello
    print(identity(3.14))      // T = Float  → 3.14
```

The compiler infers `T` from the argument; you never need to write it explicitly at the call site.

### Multiple Type Parameters

```mom
fn swap[A, B](a: A, b: B) -> B:
    b   // trivial demo

fn zip[A, B](xs: [A], ys: [B]) -> [[A]]:
    // returns pairs as a list-of-lists
    // ...
```

Separate multiple parameters with commas inside the brackets.

### Grammar Reference

```ebnf
function = "async"? "fn" IDENT generics? params return_type? where_clause? block ;
generics = "[" IDENT ( "," IDENT )* ","? "]" ;
```

---

## Type Bounds

Constrain a type parameter to only types that implement certain traits:

```mom
fn max_of[T: Ord](a: T, b: T) -> T:
    if a.cmp(b) >= 0: a else: b

fn first_sorted[T: Ord + Clone](xs: [T]) -> Option[T]:
    // needs Ord to sort, Clone to copy
    // ...
```

Bounds use the `IDENT : IDENT ( "+" IDENT )*` grammar — the same syntax as Rust trait bounds but in mom's square-bracket generics.

### `where` Clauses

Move bounds after the parameter list for readability when they grow long:

```mom
fn merge[T](a: [T], b: [T]) -> [T]
    where T: Ord + Clone:
    // combine and sort two lists
```

Multiple `where` entries are comma-separated:

```mom
fn zip_with[A, B, C](xs: [A], ys: [B], f: fn(A, B) -> C) -> [C]
    where A: Clone,
          B: Clone:
    // ...
```

---

## Generic Structs

Put the type parameters after the struct name:

```mom
struct Pair[A, B]:
    first: A
    second: B
```

Instantiate by supplying concrete types:

```mom
fn main():
    let p = Pair { first: 1, second: "one" }
    print(p.first)    // 1
    print(p.second)   // one
```

### Buffer Example

```mom
struct Buffer[T]:
    data: [T]
    len: Int
    cap: Int
```

### Grammar Reference

```ebnf
struct_decl   = "struct" IDENT generics? "{" struct_fields? "}" ;
struct_fields = struct_field ( "," struct_field )* ","? ;
struct_field  = visibility? IDENT ":" type ;
```

---

## Generic Enums

The canonical sum types are generic enums:

```mom
enum Option[T]:
    Some(T)
    None

enum Result[T, E]:
    Ok(T)
    Err(E)
```

Use them in match expressions:

```mom
fn safe_divide(a: Int, b: Int) -> Option[Int]:
    if b == 0:
        Option.None
    else:
        Option.Some(a / b)

fn main():
    let result = safe_divide(10, 2)
    match result:
        Option.Some(v) => print(v)    // 5
        Option.None    => print("division by zero")
```

### Grammar Reference

```ebnf
enum_decl = "enum" IDENT generics? "{" variants? "}" ;
variants  = variant ( "," variant )* ","? ;
variant   = IDENT ( "(" type_list? ")" )? ;
```

---

## Generic `impl` Blocks

When implementing methods on a generic struct, repeat the type parameters on both `impl` and the type name:

```mom
impl Pair[A, B]:
    fn swap(self) -> Pair[B, A]:
        Pair { first: self.second, second: self.first }
```

To implement a trait for a generic struct:

```mom
trait Container[T]:
    fn add(self, value: T)
    fn count(self) -> Int

struct Stack[T]:
    items: [T]

impl Container[T] for Stack[T]:
    fn add(self, value: T):
        self.items = self.items + [value]

    fn count(self) -> Int:
        self.items.len()
```

The `impl` header mirrors the grammar:

```ebnf
impl_block = "impl" generics? IDENT ( "[" type_list "]" )?
             ( "for" IDENT ( "[" type_list "]" )? )?
             "{" impl_method* "}" ;
```

---

## Type Inference for Generic Arguments

mom uses **bidirectional type inference** in the native compiler. For generic calls, the compiler infers type parameters from the arguments — you never write `identity[Int](42)`:

```mom
let x = identity(42)          // T inferred as Int
let y = identity("hello")     // T inferred as String
let z = max_of(3, 7)          // T inferred as Int
```

Rules:
- `let` bindings infer from the initialiser expression.
- Generic type parameters are inferred from the concrete argument types.
- Function parameters and return types are always written explicitly (no inference there).
- The bootstrap interpreter is more lenient: unknown generics are accepted without full checking.

---

## How Generics Work: Interpreter vs Native Backend

| Aspect | Bootstrap Interpreter | Native Compiler (planned) |
|--------|-----------------------|---------------------------|
| Execution model | Dynamic dispatch at runtime | Monomorphisation at compile time |
| Type checking | Lenient — unknown params accepted | Strict — all params resolved |
| Performance | Uniform (no specialisation) | Zero overhead per specialisation |
| `dyn Trait` | Implicit via runtime lookup | Explicit `dyn Trait` pointer |

The bootstrap interpreter lets you write and run generic code today. When you move to the native backend, the compiler creates a separate specialised version of every generic function and struct for each concrete type it is called with.

---

## Full Worked Examples

### `identity` and `first`

```mom
fn identity[T](value: T) -> T:
    value

fn first[T](xs: [T]) -> Option[T]:
    if xs.len() == 0:
        Option.None
    else:
        Option.Some(xs[0])

fn main():
    print(identity(42))          // 42
    print(identity("hello"))     // hello

    let nums = [10, 20, 30]
    match first(nums):
        Option.Some(v) => print(v)   // 10
        Option.None    => print("empty")
```

### Generic `Pair`

```mom
struct Pair[A, B]:
    first: A
    second: B

impl Pair[A, B]:
    fn map_first[C](self, f: fn(A) -> C) -> Pair[C, B]:
        Pair { first: f(self.first), second: self.second }

fn main():
    let p = Pair { first: 3, second: "three" }
    print(p.first)    // 3

    let p2 = p.map_first(fn(x: Int) => x * 10)
    print(p2.first)   // 30
```

### `map` and `filter`

```mom
fn map[A, B](xs: [A], f: fn(A) -> B) -> [B]:
    let mut result: [B] = []
    for x in xs:
        result = result + [f(x)]
    result

fn filter[T](xs: [T], pred: fn(T) -> Bool) -> [T]:
    let mut result: [T] = []
    for x in xs:
        if pred(x):
            result = result + [x]
    result

fn reduce[T, Acc](xs: [T], init: Acc, f: fn(Acc, T) -> Acc) -> Acc:
    let mut acc = init
    for x in xs:
        acc = f(acc, x)
    acc

fn main():
    let nums = [1, 2, 3, 4, 5, 6]

    let doubled = map(nums, fn(x: Int) => x * 2)
    print(doubled)   // [2, 4, 6, 8, 10, 12]

    let evens = filter(nums, fn(x: Int) => x % 2 == 0)
    print(evens)     // [2, 4, 6]

    let sum = reduce(nums, 0, fn(acc: Int, x: Int) => acc + x)
    print(sum)       // 21
```

### Bounded Generic Sort Helper

```mom
fn min_of[T: Ord](xs: [T]) -> Option[T]:
    if xs.len() == 0:
        Option.None
    else:
        let mut best = xs[0]
        for x in xs:
            if x.cmp(best) < 0:
                best = x
        Option.Some(best)

fn main():
    let scores = [42, 7, 99, 3, 55]
    match min_of(scores):
        Option.Some(v) => print(v)   // 3
        Option.None    => print("empty")
```

---

## Current Limitations

| Feature | Status |
|---------|--------|
| Type parameter inference | Supported in both interpreter and native backend |
| Multiple type parameters | Supported |
| Trait bounds (`T: Ord`) | Parsed; enforced by native compiler |
| `where` clauses | Parsed; enforced by native compiler |
| Generic `impl` blocks | Supported |
| Monomorphisation | Native backend only (planned) |
| `dyn Trait` / trait objects | Native backend only (planned) |
| Higher-kinded types | Not planned for v1 |
| Const generics | Not planned for v1 |

---

## Summary

| Concept | Syntax |
|---------|--------|
| Generic function | `fn foo[T](x: T) -> T` |
| Multiple type params | `fn foo[A, B](a: A, b: B)` |
| Trait bound | `fn foo[T: Ord](x: T)` |
| Multiple bounds | `fn foo[T: Ord + Clone](x: T)` |
| `where` clause | `fn foo[T](x: T) where T: Ord + Clone` |
| Generic struct | `struct Pair[A, B] { first: A, second: B }` |
| Generic enum | `enum Option[T] { Some(T), None }` |
| Generic impl | `impl Container[T] for Stack[T]` |
| Type inference | Automatic at call site — no explicit `[Int]` needed |
