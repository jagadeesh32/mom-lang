# Error Handling

Mom has no exceptions, no `null`, and no implicit failure. Every function that can fail says so in its return type, and the compiler refuses to let you ignore the failure path. This makes error handling explicit, composable, and zero-surprise.

## Philosophy

- **No exceptions.** Exceptions create invisible control-flow paths. Mom's errors travel through the normal return channel.
- **No null.** A value of type `User` is always a valid `User`. Absence is expressed with `Option[User]`.
- **Explicit propagation.** If a caller wants to forward an error up the stack, it must say so — with `?` or with a `match`.

---

## `Option[T]`: Modeling Optional Values

`Option[T]` represents a value that may or may not be present. It has two variants:

| Variant | Meaning |
|---------|---------|
| `Some(x)` | A value is present; `x` has type `T` |
| `None` | No value |

### Constructing an `Option`

```mom
let a: Option[Int] = Some(42)
let b: Option[Int] = None
```

Functions return `Option` when the result legitimately might not exist:

```mom
fn first(xs: [Int]) -> Option[Int]:
    if len(xs) == 0 { None } else { Some(xs[0]) }
```

### Unwrapping with `match`

The safe way to get the value out of an `Option` is `match`:

```mom
match first([7, 8, 9]):
    Some(x) => print(x)    // x = 7
    None    => print(0)     // default
```

The compiler requires both arms to be covered. You cannot accidentally ignore `None`.

### The `or_else` / Fallback Pattern

When you just need a default value, `match` doubles as an inline fallback:

```mom
let value = match lookup(key):
    Some(v) => v
    None    => "default"
```

This is the idiomatic replacement for `opt.unwrap_or(default)` or the ternary null-coalescing patterns found in other languages.

---

## `Result[T, E]`: Modeling Fallible Operations

`Result[T, E]` represents a computation that either succeeds with a value of type `T` or fails with an error of type `E`. It has two variants:

| Variant | Meaning |
|---------|---------|
| `Ok(x)` | Success; `x` has type `T` |
| `Err(e)` | Failure; `e` has type `E` |

### Constructing a `Result`

```mom
fn parse(value: Int) -> Result[Int, String]:
    if value < 0:
        Err("negative")
    else:
        Ok(value * 2)
```

### Unwrapping with `match`

```mom
match parse(5):
    Ok(v)  => print(v)      // 10
    Err(e) => print(e)

match parse(-1):
    Ok(v)  => print(v)
    Err(e) => print(e)      // "negative"
```

---

## The `?` Try Operator: Propagating Errors

The `?` operator is syntactic sugar for "if this is an error, return it immediately; otherwise give me the inner value." It turns verbose early-return boilerplate into a single character.

```mom
fn doubled_plus_one(value: Int) -> Result[Int, String]:
    let inner = parse(value)?   // returns Err early if parse fails
    Ok(inner + 1)
```

Without `?`, this would be:

```mom
fn doubled_plus_one(value: Int) -> Result[Int, String]:
    match parse(value):
        Ok(inner) => Ok(inner + 1)
        Err(e)    => Err(e)
```

### Rules for `?`

- The enclosing function **must** return `Result[_, E]` (or `Option[_]`).
- On `Ok(v)`, `?` unwraps to `v` and execution continues.
- On `Err(e)`, `?` immediately returns `Err(e)` from the current function.
- On `Option`: `Some(v)` unwraps to `v`; `None` immediately returns `None`.

### Chaining Fallible Operations

`?` makes chains of fallible steps read like straight-line code:

```mom
fn process(raw: String) -> Result[Summary, String]:
    let parsed  = parse_json(raw)?
    let valid   = validate(parsed)?
    let summary = summarize(valid)?
    Ok(summary)
```

Each step either succeeds and passes its value forward, or short-circuits with the error.

---

## `?` on `Option` vs `Result`

| Applied to | Returns on failure |
|------------|-------------------|
| `Option[T]` | `None` |
| `Result[T, E]` | `Err(e)` |

Both require the enclosing function's return type to match:

```mom
fn safe_head(xs: [Int]) -> Option[Int]:
    let first = find_first(xs)?   // returns None if find_first returns None
    Some(first * 2)
```

---

## Converting Between `Option` and `Result`

Sometimes you have an `Option` but need a `Result` (e.g., to use `?` in a function returning `Result`):

```mom
// Option -> Result: supply an error value for the None case
let result: Result[Int, String] = match maybe_int:
    Some(v) => Ok(v)
    None    => Err("value was absent")

// Result -> Option: discard the error
let opt: Option[Int] = match result:
    Ok(v)  => Some(v)
    Err(_) => None
```

---

## `panic(msg)`: Unrecoverable Errors

`panic` is for **programmer errors** — violated invariants, impossible states, bugs. It is not for user-facing errors or anything you expect to happen at runtime in production.

```mom
fn get(xs: [Int], i: Int) -> Int:
    if i >= len(xs):
        panic("index out of bounds")
    xs[i]
```

When a panic is triggered, the program prints the message and terminates. There is no way to catch a panic. If a case can legitimately fail at runtime, use `Result` instead.

---

## `assert(condition)`: Invariant Checking

`assert` is a lightweight panic for boolean conditions. It is the idiomatic way to document and enforce invariants:

```mom
fn sqrt(n: Float) -> Float:
    assert(n >= 0.0)
    // ... compute sqrt
```

If the condition is false, `assert` panics with a message indicating which assertion failed. Use `assert` freely during development and in functions where a contract violation is a bug.

---

## Error Type Design

### String Errors

The simplest approach — good for prototyping and internal utilities:

```mom
fn divide(a: Int, b: Int) -> Result[Int, String]:
    if b == 0:
        Err("division by zero")
    else:
        Ok(a / b)
```

Readable, but callers cannot programmatically distinguish error kinds.

### Enum Errors

For library code or complex failure modes, define an enum:

```mom
enum ParseError:
    UnexpectedChar(Char)
    UnexpectedEnd
    TooLong(Int)

fn parse_hex(s: String) -> Result[Int, ParseError]:
    if len(s) == 0:
        Err(ParseError.UnexpectedEnd)
    else if len(s) > 16:
        Err(ParseError.TooLong(len(s)))
    else:
        Ok(hex_to_int(s))
```

Callers can then match on specific error variants:

```mom
match parse_hex(input):
    Ok(n)                        => use(n)
    Err(ParseError.UnexpectedEnd) => print("empty input")
    Err(ParseError.TooLong(n))   => print("too many digits: " + n)
    Err(ParseError.UnexpectedChar(c)) => print("bad char: " + c)
```

---

## Worked Examples

### Example 1: Safe Division

```mom
fn divide(a: Int, b: Int) -> Result[Int, String]:
    if b == 0:
        Err("division by zero")
    else:
        Ok(a / b)

fn main():
    match divide(10, 2):
        Ok(v)  => print(v)      // 5
        Err(e) => print(e)

    match divide(10, 0):
        Ok(v)  => print(v)
        Err(e) => print(e)      // division by zero
```

### Example 2: Parsing a Number

```mom
fn parse_positive(value: Int) -> Result[Int, String]:
    if value < 0:
        Err("negative")
    else:
        Ok(value * 2)

fn doubled_plus_one(value: Int) -> Result[Int, String]:
    let inner = parse_positive(value)?
    Ok(inner + 1)

fn main():
    match doubled_plus_one(5):
        Ok(v)  => print(v)      // 11
        Err(e) => print(e)

    match doubled_plus_one(-1):
        Ok(v)  => print(v)
        Err(e) => print(e)      // negative
```

### Example 3: Optional List Head

```mom
fn first(xs: [Int]) -> Option[Int]:
    if len(xs) == 0 { None } else { Some(xs[0]) }

fn main():
    match first([7, 8, 9]):
        Some(x) => print(x)    // 7
        None    => print(0)

    match first([]):
        Some(x) => print(x)
        None    => print(0)    // 0
```

### Example 4: Chained API Call Simulation

```mom
enum ApiError:
    NotFound
    Unauthorized
    ParseFailed(String)

fn fetch_user(id: Int) -> Result[String, ApiError]:
    if id <= 0:
        Err(ApiError.NotFound)
    else:
        Ok("user:" + id)

fn fetch_profile(user: String) -> Result[String, ApiError]:
    if len(user) == 0:
        Err(ApiError.ParseFailed("empty user"))
    else:
        Ok("profile:" + user)

fn get_profile(id: Int) -> Result[String, ApiError]:
    let user    = fetch_user(id)?
    let profile = fetch_profile(user)?
    Ok(profile)

fn main():
    match get_profile(42):
        Ok(p)                         => print(p)
        Err(ApiError.NotFound)         => print("not found")
        Err(ApiError.Unauthorized)     => print("unauthorized")
        Err(ApiError.ParseFailed(msg)) => print("parse error: " + msg)
```

### Example 5: Fallback Default

```mom
fn lookup(key: String) -> Option[Int]:
    // ... returns Some or None
    None

fn main():
    let port = match lookup("PORT"):
        Some(p) => p
        None    => 8080

    print(port)     // 8080
```

---

## Summary

| Tool | Use when |
|------|---------|
| `Option[T]` | A value may legitimately be absent |
| `Result[T, E]` | An operation may legitimately fail |
| `?` | Propagating errors up the call stack |
| `match` | Handling all outcomes safely |
| `panic` | A bug in the program (not a runtime error) |
| `assert` | Enforcing an invariant that must always hold |

Mom's error model is explicit by design. You always know which functions can fail and exactly what they fail with. The compiler ensures you handle every case.
