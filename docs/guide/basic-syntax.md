# Basic Syntax

Mom uses **indentation** to define blocks, similar to Python. Understanding the three pillars of Mom syntax — indentation, colons, and expressions-as-values — lets you read and write any Mom program.

---

## Indentation and Blocks

A block is introduced by a colon `:` at the end of a header line, followed by an indented body. The body ends when indentation returns to the level of the header.

```mom
fn double(x: Int) -> Int:
    x * 2          // body of the function

fn main():
    let result = double(5)
    print(result)  // still inside main
                   // main ends here (end of file or next top-level item)
```

**Rules:**
- Use 4 spaces *or* 1 tab per level. Do not mix.
- An empty block needs at least one statement. Use `()` (unit value) as a no-op if needed.
- Brace style `{ }` is also accepted everywhere (useful for one-liners in larger expressions).

```mom
// Equivalent with braces
fn double(x: Int) -> Int { x * 2 }
```

---

## Statements vs Expressions

Mom distinguishes **statements** (which do something) from **expressions** (which produce a value). Most things in Mom are expressions — `if`, `match`, `block`, function calls all return values.

```mom
// expression: if returns a value
let sign = if x > 0 { 1 } else if x < 0 { -1 } else { 0 }

// expression: match returns a value
let label = match code:
    200 => "ok"
    404 => "not found"
    _   => "other"

// statement: assignment does not return a value
let mut y = 0
y = y + 1
```

The last expression in a function body (or block) is automatically the **return value** — no `return` keyword needed in most cases.

```mom
fn add(a: Int, b: Int) -> Int:
    a + b          // implicit return
```

---

## Top-Level Items

Programs are a sequence of **items** at the top level:

| Item | Keyword |
|---|---|
| Function | `fn`, `async fn` |
| Struct | `struct` |
| Enum | `enum` |
| Trait | `trait` |
| Implementation | `impl` |
| Module | `module` |
| Import | `import`, `use` |
| Constant | `const` |
| Extern block | `extern` |

```mom
const MAX_SIZE: Int = 1024

struct Config:
    host: String
    port: Int

enum Status:
    Active
    Inactive(String)

fn main():
    let cfg = Config { host: "localhost", port: 8080 }
    print(cfg.port)
```

---

## Semicolons

Semicolons are **optional**. They may be used to separate two statements on the same line (unusual) but are never required at the end of a line.

```mom
// Both are valid:
let x = 1
let y = 2

// Or on one line (rare):
let x = 1; let y = 2
```

---

## Whitespace

Whitespace is significant only for indentation. Horizontal whitespace between tokens is ignored.

```mom
let result = 1   +   2   *   3   // valid
```

---

## Case Conventions

| Kind | Convention | Example |
|---|---|---|
| Variables and functions | `snake_case` | `my_variable`, `parse_input` |
| Types (struct, enum, trait) | `PascalCase` | `HttpResponse`, `ErrorKind` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_CONNECTIONS` |
| Module names | `snake_case` | `std::io` |
| Enum variants | `PascalCase` | `Some`, `Ok`, `Err`, `None` |

---

## Inline (One-Liner) Functions

Short functions can be written on a single line:

```mom
fn square(x: Int) -> Int: x * x
fn is_even(n: Int) -> Bool: n % 2 == 0
```

---

## Program Entry Point

Every executable Mom program must define:

```mom
fn main():
    // program starts here
```

`main` takes no arguments and returns nothing (unit). Use `args()` to access command-line arguments.

---

## Full Minimal Example

```mom
// constants at the top level
const GREETING: String = "Hello"

// a pure function
fn greet(name: String) -> String:
    GREETING + ", " + name + "!"

// entry point
fn main():
    let msg = greet("world")
    print(msg)
```
