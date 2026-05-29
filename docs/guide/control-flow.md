# Control Flow

Mom's control flow constructs are mostly **expressions** — they produce values. This lets you write concise, value-oriented code without temporary mutable variables.

---

## `if` / `elif` / `else`

### Statement form

```mom
if condition:
    // executed when condition is true

if x > 0:
    print("positive")
elif x < 0:
    print("negative")
else:
    print("zero")
```

### Expression form

When all branches return the same type, `if` is an expression:

```mom
let label = if x > 0 { "positive" } elif x < 0 { "negative" } else { "zero" }
```

Or with indented style:

```mom
let abs_val =
    if x >= 0: x
    else: -x
```

**Rules:**
- All branches must have the same type when used as an expression.
- The `else` branch is required in expression form.
- Braces `{ }` and colon-indent style are both valid.

---

## `while` Loop

Executes the body as long as the condition is `true`.

```mom
let mut i = 0
while i < 5:
    print(i)
    i = i + 1
// prints: 0, 1, 2, 3, 4
```

### Infinite loop with `break`

```mom
let mut total = 0
let mut n = 1
while true:
    total = total + n
    n = n + 1
    if total > 100: break
print("stopped at n = " + str(n))
```

---

## `for` Loop

### Range loop

```mom
for i in 0..10:
    print(i)
// 0, 1, 2, 3, 4, 5, 6, 7, 8, 9  (exclusive upper bound)
```

### List loop

```mom
let fruits = ["apple", "banana", "cherry"]
for fruit in fruits:
    print(fruit)
```

### Enumerate

```mom
for i in 0..len(fruits):
    print(str(i) + ": " + fruits[i])
```

---

## `break` and `continue`

### `break` — exit the nearest enclosing loop

```mom
for i in 0..100:
    if i == 5: break
    print(i)
// prints: 0, 1, 2, 3, 4
```

### `continue` — skip to the next iteration

```mom
for i in 0..10:
    if i % 2 == 0: continue
    print(i)
// prints: 1, 3, 5, 7, 9
```

---

## `match` Expression

`match` tests a value against a sequence of **patterns**. The first matching arm executes. Every possible case must be covered (exhaustiveness is checked at compile time).

### Matching integers

```mom
let code = 200
let msg = match code:
    200 => "OK"
    301 => "Moved Permanently"
    404 => "Not Found"
    500 => "Internal Server Error"
    _   => "Unknown"

print(msg)    // OK
```

### Matching booleans

```mom
let flag = true
match flag:
    true  => print("on")
    false => print("off")
```

### Wildcard `_`

`_` matches anything and discards the value. It is the catch-all pattern:

```mom
match status:
    Active    => print("running")
    Inactive  => print("stopped")
    _         => print("unknown status")
```

### Named binding

An identifier in pattern position binds the matched value:

```mom
match find_user(id):
    Some(user) => print("Found: " + user.name)
    None       => print("Not found")
```

### Matching enum variants with data

```mom
enum Shape:
    Circle(Float)
    Rectangle(Int, Int)
    Point

let s = Rectangle(3, 4)
let area = match s:
    Circle(r)    => 3.14159 * r * r
    Rectangle(w, h) => w * h as Float
    Point        => 0.0

print(area)    // 12.0
```

### Nested patterns

Patterns can be nested — a variant whose payload is another variant:

```mom
enum Inner:
    A(Int)
    B

enum Outer:
    Wrap(Inner)
    Empty

let val = Wrap(A(7))
let result = match val:
    Wrap(A(n)) => n
    Wrap(B)    => -1
    Empty      => 0

print(result)   // 7
```

### Literal patterns inside variants

```mom
enum Cmd:
    Move(Int)
    Stop

let cmd = Move(0)
match cmd:
    Move(0) => print("no movement")
    Move(n) => print("move by " + str(n))
    Stop    => print("stop")
```

### Match as a statement (arm bodies are assignments)

Match arms can perform assignments, not just return values:

```mom
let mut count = 0
match cmd:
    Inc    => count = count + 1
    Add(n) => count = count + n
    Reset  => count = 0
```

---

## `return`

Exits the current function early and returns a value:

```mom
fn find(xs: [Int], target: Int) -> Option[Int]:
    for i in 0..len(xs):
        if xs[i] == target:
            return Some(i)
    None    // implicit return if loop completes without finding target
```

`return` with no value returns `()`:

```mom
fn greet_if_valid(name: String):
    if len(name) == 0:
        return    // early exit
    print("Hello, " + name + "!")
```

---

## `block` Expression

`block` creates an anonymous scope. The value of the block is its final expression. This is useful for:

1. Limiting the scope of temporary variables
2. Ending a borrow before continuing with a value

```mom
let result = block:
    let temp = heavy_computation()
    temp * temp + 1
// temp is not accessible here
print(result)
```

### Scoped borrows

```mom
let mut counter = 0
block:
    let m = &mut counter
    m = m + 5
// m's borrow ended; counter is freely usable again
counter = counter + 1
print(counter)    // 6
```

---

## `?` — Error Propagation (Try Operator)

Inside a function returning `Result[T, E]` or `Option[T]`, the `?` operator short-circuits on failure:

```mom
fn process(path: String) -> Result[Int, String]:
    let text = read_file(path)?     // returns Err early if file missing
    let n = parse_int(text)?        // returns None→Err early if not a number
    Ok(n * 2)
```

Without `?`, this would be:

```mom
fn process(path: String) -> Result[Int, String]:
    let text = match read_file(path):
        Ok(t)  => t
        Err(e) => return Err(e)
    let n = match parse_int(text):
        Some(v) => v
        None    => return Err("not an integer")
    Ok(n * 2)
```

---

## Combining Control Flow

Control flow expressions compose naturally:

```mom
fn classify_list(xs: [Int]) -> String:
    if len(xs) == 0:
        "empty"
    elif all(xs |> map(fn(x) => x > 0)):
        "all positive"
    elif any(xs |> map(fn(x) => x < 0)):
        "has negatives"
    else:
        "mixed"
```

---

## Summary

| Construct | Kind | Description |
|---|---|---|
| `if / elif / else` | expression + statement | Conditional branching |
| `while` | statement | Loop while condition holds |
| `for x in range/list` | statement | Iterate over a range or collection |
| `break` | statement | Exit the enclosing loop |
| `continue` | statement | Skip to the next loop iteration |
| `match value: pat => body` | expression + statement | Exhaustive pattern matching |
| `return expr` | statement | Exit function with a value |
| `block: ...` | expression | Scoped block yielding its tail value |
| `expr?` | expression | Propagate `Err`/`None` from Result/Option |
