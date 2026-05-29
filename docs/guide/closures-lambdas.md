# Closures and Lambdas

mom uses the `fn` keyword for both named functions and anonymous lambdas. The two forms share identical syntax — a lambda is simply an `fn` expression that appears inline rather than at the top level.

---

## Lambda Syntax

The simplest lambda takes parameters and returns an expression:

```mom
fn(x: Int) => x * 2
```

Break it down:

| Part | Meaning |
|------|---------|
| `fn` | keyword — same as a named function |
| `(x: Int)` | parameter list with explicit types |
| `=>` | "maps to" — separates params from body |
| `x * 2` | body expression (implicitly returned) |

### Lambda with Explicit Return Type

Annotate the return type between the parameter list and `=>`:

```mom
fn(x: Int) -> Int => x * 2
```

The return type annotation is optional when the compiler can infer it from the body.

### Multi-Statement Lambda (Block Body)

Replace `=> expression` with a brace block when you need multiple statements:

```mom
fn(x: Int, y: Int) -> Int {
    let sum = x + y
    sum * sum
}
```

The last expression in the block is the return value (no explicit `return` needed, though `return` is also accepted).

### Grammar Reference

```ebnf
lambda = "fn" params ( "->" type )? ( "=>" expression | block ) ;

params = "(" ( param ( "," param )* ","? )? ")" ;
param  = "self" | IDENT ":" type ;
```

---

## Lambda Types

The **type** of a lambda is written as `fn(ParamTypes) -> ReturnType`:

```mom
fn(Int) -> Int           // takes one Int, returns Int
fn(Int, Int) -> Bool     // takes two Ints, returns Bool
fn(String) -> ()         // takes String, returns unit (no value)
fn() -> Float            // takes nothing, returns Float
```

This is the type you use when declaring a variable or function parameter that holds a lambda.

---

## Storing Lambdas in Variables

Assign a lambda to a `let` binding and call it later:

```mom
let double: fn(Int) -> Int = fn(x: Int) => x * 2
let greet: fn(String) -> String = fn(name: String) => "Hello, " + name

fn main():
    print(double(5))       // 10
    print(greet("World"))  // Hello, World
```

The type annotation on the binding is optional when the compiler can infer it:

```mom
let square = fn(x: Int) => x * x
print(square(9))   // 81
```

---

## Passing Lambdas as Arguments (Callbacks)

Functions that accept a lambda declare the parameter with a `fn(...)` type:

```mom
fn apply(value: Int, transform: fn(Int) -> Int) -> Int:
    transform(value)

fn apply_twice(value: Int, transform: fn(Int) -> Int) -> Int:
    transform(transform(value))

fn main():
    print(apply(3, fn(x: Int) => x + 10))         // 13
    print(apply_twice(2, fn(x: Int) => x * 3))    // 18
```

Lambdas defined elsewhere can be passed by name:

```mom
fn negate(x: Int) -> Int:
    -x

fn main():
    print(apply(5, negate))   // -5
```

---

## Higher-Order Functions: `map`, `filter`, `reduce`

These three primitives are the backbone of functional programming in mom. Write them as generic functions that accept lambda arguments:

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
    let nums = [1, 2, 3, 4, 5]

    let doubled  = map(nums, fn(x: Int) => x * 2)
    print(doubled)   // [2, 4, 6, 8, 10]

    let evens = filter(nums, fn(x: Int) => x % 2 == 0)
    print(evens)     // [2, 4]

    let total = reduce(nums, 0, fn(acc: Int, x: Int) => acc + x)
    print(total)     // 15
```

---

## The Pipeline Operator `|>`

The `|>` operator passes the left-hand value as the first argument to the right-hand function. It is left-associative and sits at precedence level 3 (above equality, below arithmetic):

```ebnf
pipeline = logic_or ( ( "|>" | ".." ) logic_or )* ;
```

Use it to chain transformations without nesting:

```mom
fn main():
    let result =
        [1, 2, 3, 4, 5, 6, 7, 8]
        |> filter(fn(x: Int) => x % 2 == 0)
        |> map(fn(x: Int) => x * x)
        |> reduce(0, fn(acc: Int, x: Int) => acc + x)

    print(result)   // 4 + 16 + 36 + 64 = 120
```

`|>` works with any function or lambda — not only higher-order ones:

```mom
fn add_one(x: Int) -> Int: x + 1
fn to_string(x: Int) -> String: x.display()

fn main():
    let s = 41 |> add_one |> to_string
    print(s)   // "42"
```

---

## Returning Lambdas from Functions

A function can return a lambda. The return type is the lambda's `fn(...)` type:

```mom
fn make_adder(n: Int) -> fn(Int) -> Int:
    fn(x: Int) => x + n

fn main():
    let add5 = make_adder(5)
    let add10 = make_adder(10)
    print(add5(3))    // 8
    print(add10(3))   // 13
```

### Closure Capture — Current Status

> **Important:** The bootstrap interpreter does not yet implement full lexical closure capture. The lambda `fn(x: Int) => x + n` above **will work** when `n` is a literal or a compile-time constant, but capture of mutable local variables from the enclosing scope is **not yet guaranteed** to behave correctly in all cases.

Full closure capture (capturing mutable bindings from the environment) is planned for the native backend. For now, prefer passing all needed values as explicit parameters.

---

## `fn` as a Unified Keyword

mom deliberately uses one keyword — `fn` — for all function forms:

| Form | Example |
|------|---------|
| Named top-level function | `fn add(a: Int, b: Int) -> Int: a + b` |
| Named method in `impl` | `fn area(self) -> Float: ...` |
| Inline lambda (expression body) | `fn(x: Int) => x * 2` |
| Inline lambda (block body) | `fn(x: Int) { let y = x * 2; y + 1 }` |
| Lambda stored in variable | `let f = fn(x: Int) => x + 1` |
| Lambda as argument | `map(xs, fn(x: Int) => x * 2)` |

This uniformity means any named function can be passed as a lambda argument without wrapping:

```mom
fn double(x: Int) -> Int: x * 2

fn main():
    let xs = [1, 2, 3]
    let ys = map(xs, double)   // no wrapper needed
    print(ys)   // [2, 4, 6]
```

---

## Full Worked Examples

### Sorting Comparator

```mom
fn sort_by[T](xs: [T], less_than: fn(T, T) -> Bool) -> [T]:
    // insertion sort for clarity
    let mut sorted = xs
    let n = sorted.len()
    let mut i = 1
    while i < n:
        let key = sorted[i]
        let mut j = i - 1
        while j >= 0 && less_than(key, sorted[j]):
            sorted[j + 1] = sorted[j]
            j = j - 1
        sorted[j + 1] = key
        i = i + 1
    sorted

fn main():
    let nums = [5, 3, 8, 1, 9, 2]

    let asc  = sort_by(nums, fn(a: Int, b: Int) => a < b)
    let desc = sort_by(nums, fn(a: Int, b: Int) => a > b)

    print(asc)   // [1, 2, 3, 5, 8, 9]
    print(desc)  // [9, 8, 5, 3, 2, 1]
```

### Pipeline Chain

```mom
fn words(s: String) -> [String]:
    // split on spaces — stub; real impl in stdlib
    [s]

fn upper(s: String) -> String:
    s   // stub

fn main():
    let sentence = "the quick brown fox"
    let result =
        sentence
        |> words
        |> filter(fn(w: String) => w.len() > 3)
        |> map(fn(w: String) => upper(w))

    print(result)
```

### Function Factory

```mom
fn make_multiplier(factor: Int) -> fn(Int) -> Int:
    fn(x: Int) => x * factor

fn main():
    let triple = make_multiplier(3)
    let nums   = [1, 2, 3, 4, 5]
    let result = map(nums, triple)
    print(result)   // [3, 6, 9, 12, 15]
```

### Composing Transformations

```mom
fn compose[A, B, C](f: fn(B) -> C, g: fn(A) -> B) -> fn(A) -> C:
    fn(x: A) => f(g(x))

fn main():
    let add1    = fn(x: Int) => x + 1
    let double  = fn(x: Int) => x * 2
    let double_then_add1 = compose(add1, double)

    print(double_then_add1(4))   // 9  (4*2 + 1)
    print(double_then_add1(10))  // 21 (10*2 + 1)
```

---

## Current Limitations

| Feature | Status |
|---------|--------|
| Lambda expression body (`=> expr`) | Supported |
| Lambda block body (`{ ... }`) | Supported |
| Explicit return type annotation | Supported |
| Lambdas as function arguments | Supported |
| Lambdas stored in variables | Supported |
| Returning lambdas from functions | Supported |
| Named function as lambda value | Supported |
| Pipeline operator `|>` | Supported |
| Full lexical closure capture (mutable variables) | Planned (native backend) |
| Recursive lambdas (self-reference) | Not yet supported |
| Zero-argument lambdas `fn() => expr` | Supported |
| Generic lambdas | Not yet supported (use generic named functions) |

---

## Summary

| Concept | Syntax |
|---------|--------|
| Simple lambda | `fn(x: Int) => x * 2` |
| With return type | `fn(x: Int) -> Int => x * 2` |
| Block body | `fn(x: Int) -> Int { let y = x; y * 2 }` |
| Lambda type | `fn(Int, String) -> Bool` |
| Store in variable | `let f = fn(x: Int) => x + 1` |
| Pass as argument | `map(xs, fn(x: Int) => x * 2)` |
| Return lambda | `fn make_adder(n: Int) -> fn(Int) -> Int: fn(x: Int) => x + n` |
| Pipeline | `value \|> transform \|> format` |
| Named fn as lambda | `map(xs, double)` — no wrapper needed |
