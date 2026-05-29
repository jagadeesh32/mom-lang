# Arguments

This page covers every kind of argument and parameter in Mom: function parameters, default behavior, command-line arguments, and common patterns.

---

## Function Parameters

Parameters are declared as `name: Type` pairs, comma-separated, inside `()`:

```mom
fn greet(name: String, times: Int):
    for i in 0..times:
        print("Hello, " + name + "!")

greet("Alice", 3)
// Hello, Alice!
// Hello, Alice!
// Hello, Alice!
```

### Rules

- Every parameter **must** have a type annotation. There is no inference for function parameters.
- Parameters are passed **by value** by default.
- There is no implicit `null`; use `Option[T]` for optional parameters.

---

## Passing by Reference

To avoid copying a large value (or to mutate a binding), pass a reference:

```mom
fn print_all(items: &[Int]):
    for x in items:
        print(x)

fn double_all(items: &mut [Int]):
    for i in 0..len(items):
        items[i] = items[i] * 2
```

---

## Optional Parameters via `Option[T]`

Mom has no built-in optional/default parameters. Model them with `Option[T]`:

```mom
fn connect(host: String, port: Option[Int]) -> String:
    let p = match port:
        Some(n) => n
        None    => 8080    // default
    host + ":" + str(p)

print(connect("localhost", Some(9090)))   // localhost:9090
print(connect("localhost", None))         // localhost:8080
```

---

## Variadic-Style via Lists

There are no variadic (`...`) parameters. Pass a list instead:

```mom
fn sum_all(values: [Int]) -> Int:
    let mut total = 0
    for v in values:
        total = total + v
    total

print(sum_all([1, 2, 3, 4, 5]))   // 15
```

---

## Function Type as a Parameter (Callbacks)

Pass functions as values using `fn(T) -> U` types:

```mom
fn apply_to_each(f: fn(Int) -> Int, xs: [Int]) -> [Int]:
    map(f, xs)

let doubled = apply_to_each(fn(x: Int) => x * 2, [1, 2, 3])
print(doubled)   // [2, 4, 6]
```

---

## Generic Parameters

Type parameters let a function work with any type:

```mom
fn repeat[T](value: T, n: Int) -> [T]:
    let mut result = []
    for i in 0..n:
        result = push(result, value)
    result

print(repeat(0, 5))         // [0, 0, 0, 0, 0]
print(repeat("x", 3))      // ["x", "x", "x"]
```

With bounds:

```mom
fn clamp[T: Ord](value: T, lo: T, hi: T) -> T:
    if value < lo: lo
    elif value > hi: hi
    else: value
```

---

## `self` — The Implicit Method Receiver

Methods receive `self` as their first parameter. It is the value the method is called on:

```mom
struct Stack:
    items: [Int]
    size: Int

impl Stack:
    fn push(self, v: Int) -> Stack:
        Stack { items: push(self.items, v), size: self.size + 1 }

    fn top(self) -> Option[Int]:
        if self.size == 0: None
        else: Some(self.items[self.size - 1])
```

`self` is always the receiver of a method call:

```mom
let s = Stack { items: [], size: 0 }
let s = s.push(1).push(2).push(3)
print(s.top())   // Some(3)
```

---

## Command-Line Arguments

Command-line arguments are retrieved with the `args()` built-in:

```mom
fn main():
    let argv = args()
    // argv[0] is the program path
    // argv[1], argv[2], ... are the actual arguments

    if len(argv) < 2:
        print("usage: " + argv[0] + " <name>")
        exit(1)

    let name = argv[1]
    print("Hello, " + name + "!")
```

### Parsing multiple named arguments

A simple key=value parser:

```mom
fn parse_args(argv: [String]) -> [(String, String)]:
    let mut pairs = []
    for i in 1..len(argv):
        let arg = argv[i]
        let eq = find(arg, "=")
        if eq >= 0:
            let key = substr(arg, 0, eq)
            let val = substr(arg, eq + 1, len(arg) - eq - 1)
            pairs = push(pairs, (key, val))
    pairs

fn main():
    let args_map = parse_args(args())
    for pair in args_map:
        print(pair.0 + " -> " + pair.1)
```

Run as:

```bash
mom run myapp.mom host=localhost port=8080
```

Output:

```
host -> localhost
port -> 8080
```

---

## Argument Validation Pattern

Validate arguments at the boundary where they enter the program:

```mom
fn parse_port(s: String) -> Int:
    match parse_int(s):
        None    => panic("PORT must be an integer, got: " + s)
        Some(n) =>
            if n < 1 or n > 65535:
                panic("PORT out of range (1-65535): " + str(n))
            n

fn main():
    let argv = args()
    if len(argv) != 3:
        print("usage: server <host> <port>")
        exit(1)
    let host = argv[1]
    let port = parse_port(argv[2])
    print("Listening on " + host + ":" + str(port))
```

---

## Return Values

Functions communicate results back through their return value. Mom encourages wrapping potential failures in `Result[T, E]`:

```mom
fn divide(a: Int, b: Int) -> Result[Int, String]:
    if b == 0:
        Err("cannot divide by zero")
    else:
        Ok(a / b)

match divide(10, 2):
    Ok(v)  => print("result: " + str(v))
    Err(e) => print("error: " + e)
```

### Multiple return values via structs

Mom doesn't have tuple return syntax for multiple values — use a struct:

```mom
struct DivResult:
    quotient: Int
    remainder: Int

fn divmod(a: Int, b: Int) -> DivResult:
    DivResult { quotient: a / b, remainder: a % b }

let r = divmod(17, 5)
print(r.quotient)     // 3
print(r.remainder)    // 2
```

---

## Summary

| Kind | Syntax | Notes |
|---|---|---|
| Value parameter | `name: Type` | Copied on call |
| Reference parameter | `name: &Type` | Immutable borrow |
| Mutable reference | `name: &mut Type` | Mutable borrow |
| Optional parameter | `name: Option[T]` | Pass `Some(x)` or `None` |
| Generic parameter | `[T]` in function signature | Works for any type |
| Self (method) | `self` (first param in `impl`) | Implicit receiver |
| Callback | `f: fn(T) -> U` | First-class function |
| CLI args | `args()` | Returns `[String]` |
