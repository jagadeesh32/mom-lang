# The Pipeline Operator

The pipeline operator `|>` threads a value from left to right through a sequence of functions. It is the idiomatic way to write data transformation chains in Mom — replacing deep nesting with a flat, readable sequence of steps.

---

## Basic Pipeline

`x |> f` is exactly `f(x)`.

```mom
fn inc(x: Int) -> Int { x + 1 }

fn main() {
    let result = 5 |> inc     // inc(5) = 6
    print(result)             // 6
}
```

There is no magic: `|>` is syntactic sugar. The compiler rewrites `x |> f` to `f(x)` before type-checking.

---

## Chained Pipelines

`x |> f |> g |> h` rewrites to `h(g(f(x)))`. Reading left to right tells you the order of operations.

```mom
fn inc(x: Int) -> Int    { x + 1 }
fn double(x: Int) -> Int { x * 2 }

fn main() {
    // From examples/pipeline.mom:
    let value = 20 |> inc |> double   // double(inc(20)) = (20+1)*2 = 42
    print(value)                       // 42
}
```

Compare to the nested form:

```mom
// Nested — read inside-out
let value = double(inc(20))

// Pipeline — read left to right
let value = 20 |> inc |> double
```

Both are equivalent; the pipeline form scales better as the chain grows.

---

## Pipelines with Multi-Argument Functions

When the next stage takes more than one argument, supply the extra arguments in a partial call. The piped value becomes the **first argument**:

```mom
fn add(a: Int, b: Int) -> Int { a + b }
fn clamp(lo: Int, hi: Int, x: Int) -> Int {
    if x < lo { lo } else if x > hi { hi } else { x }
}

fn main() {
    let result = 10
        |> add(5)          // add(10, 5)  = 15
        |> clamp(0, 20)    // clamp(0, 20, 15) = 15
    print(result)          // 15
}
```

> **Note:** Partial application with `|>` works by passing the remaining arguments in the parentheses. The piped value is always inserted as the first positional argument.

---

## Pipelines with Method Calls

Method calls chain naturally with `|>`. Use the dot call directly:

```mom
fn main() {
    let words = "hello world foo bar"
        |> String.split(" ")     // ["hello", "world", "foo", "bar"]
        |> List.sort             // alphabetical
        |> List.reverse          // reverse alphabetical
    print(words)
}
```

---

## Pipelines with Inline Lambdas

Insert an anonymous function anywhere in the chain:

```mom
fn main() {
    let result = [1, 2, 3, 4, 5]
        |> filter(fn(x) => x % 2 == 0)      // [2, 4]
        |> map(fn(x) => x * x)               // [4, 16]
        |> reduce(0, fn(acc, x) => acc + x)  // 20
    print(result)   // 20
}
```

The lambda `fn(x) => x % 2 == 0` is an expression — no separate declaration needed.

---

## Data Transformation Chains

The pipeline operator shines for ETL-style transformations:

```mom
struct Record { name: String, score: Int, active: Bool }

fn process(records: [Record]) -> [String] {
    records
        |> filter(fn(r) => r.active)
        |> sort(fn(a, b) => b.score - a.score)   // descending by score
        |> map(fn(r) => r.name + ": " + to_string(r.score))
}

fn main() {
    let data = [
        Record { name: "Alice", score: 92, active: true },
        Record { name: "Bob",   score: 85, active: false },
        Record { name: "Carol", score: 97, active: true },
    ]
    let summary = process(data)
    print(summary)   // ["Carol: 97", "Alice: 92"]
}
```

---

## Pipeline vs Nested Calls — Readability

Same computation, two styles:

```mom
// Nested — must be read inside-out
let result = reduce(
    0,
    fn(acc, x) => acc + x,
    map(
        fn(x) => x * x,
        filter(fn(x) => x > 0, raw_data)
    )
)

// Pipeline — read top to bottom
let result = raw_data
    |> filter(fn(x) => x > 0)
    |> map(fn(x) => x * x)
    |> reduce(0, fn(acc, x) => acc + x)
```

The pipeline form:
- Matches the execution order (filter first, then map, then reduce).
- Is straightforward to extend (add another `|>` stage).
- Avoids deeply nested parentheses.

---

## Building a Processing Pipeline

A realistic text-processing example:

```mom
import std.str.{trim, to_lower}
import std.collections.{HashMap}

fn word_count(text: String) -> HashMap[String, Int] {
    text
        |> to_lower
        |> String.split(" ")
        |> map(fn(w) => trim(w))
        |> filter(fn(w) => w != "")
        |> reduce(HashMap.new(), fn(acc, word) {
            let count = acc.get(word).unwrap_or(0)
            acc.insert(word, count + 1)
            acc
        })
}

fn main() {
    let counts = word_count("The quick brown fox jumps over the lazy dog")
    print(counts.get("the"))   // Some(2)
}
```

---

## Operator Precedence

`|>` sits at **level 3** in the precedence table — lower than arithmetic operators, higher than `&&` and `||`:

| Level | Operators             | Associativity |
|-------|-----------------------|---------------|
| 1     | `\|\|`                | left          |
| 2     | `&&`                  | left          |
| **3** | **`\|>`**, `..`       | **left**      |
| 4     | `==`, `!=`            | left          |
| 5     | `<`, `<=`, `>`, `>=`  | left          |
| 6     | `+`, `-`              | left          |
| 7     | `*`, `/`, `%`         | left          |
| 8     | unary `!`, `-`        | right         |
| 9     | `()`, `.`, `[]`, `?`  | left          |

Practical consequence: arithmetic in the piped value is evaluated before `|>` is applied, and `|>` chains are evaluated before `&&`/`||`:

```mom
// (2 + 3) is evaluated first, then piped into inc
let a = 2 + 3 |> inc       // inc(5) = 6

// The pipeline result feeds &&
let ok = data |> validate && data |> has_items
//       ^^^^^^^^^^^^^^^^^       ^^^^^^^^^^^^^
//          left side              right side of &&
```

Use parentheses to make intent explicit when mixing operators:

```mom
let ok = (data |> validate) && (data |> has_items)
```

---

## Use with `map`, `filter`, `reduce`, `sort`, `reverse`

Standard collection functions are designed for pipelines — they all take the collection as the first argument:

| Function | Signature | Common pipeline use |
|----------|-----------|---------------------|
| `map` | `fn[A, B](xs: [A], f: fn(A) -> B) -> [B]` | Transform each element |
| `filter` | `fn[A](xs: [A], pred: fn(A) -> Bool) -> [A]` | Keep matching elements |
| `reduce` | `fn[A, B](init: B, f: fn(B, A) -> B, xs: [A]) -> B` | Fold to single value |
| `sort` | `fn[A](xs: [A], cmp: fn(A, A) -> Int) -> [A]` | Sort by comparator |
| `sort_by` | `fn[A, K](xs: [A], key: fn(A) -> K) -> [A]` | Sort by key function |
| `reverse` | `fn[A](xs: [A]) -> [A]` | Reverse the list |
| `take` | `fn[A](xs: [A], n: Int) -> [A]` | First N elements |
| `drop` | `fn[A](xs: [A], n: Int) -> [A]` | Skip first N elements |
| `flat_map` | `fn[A, B](xs: [A], f: fn(A) -> [B]) -> [B]` | Map then flatten |
| `zip` | `fn[A, B](xs: [A], ys: [B]) -> [(A, B)]` | Pair elements |

```mom
fn main() {
    let scores = [88, 42, 95, 73, 60, 91]

    let top3 = scores
        |> filter(fn(s) => s >= 70)
        |> sort(fn(a, b) => b - a)     // descending
        |> take(3)

    print(top3)    // [95, 91, 88]
}
```

---

## Full Worked Examples

From `examples/pipeline.mom`:

```mom
// pipeline.mom — left-to-right data flow with `|>`.

fn inc(x: Int) -> Int:
    x + 1

fn double(x: Int) -> Int:
    x * 2

fn main():
    let square = fn(x: Int) => x * x
    let value = 20 |> inc |> double      // (20 + 1) * 2 = 42

    print(value)     // 42
    print(square(7)) // 49
```

Extended example — building an HTTP request preprocessing pipeline:

```mom
import std.str.{trim, to_lower}
import std.net.http.{Request}

fn normalize_path(req: Request) -> Request {
    Request { path: to_lower(trim(req.path)), ..req }
}

fn strip_query(req: Request) -> Request {
    let path = req.path |> String.split("?") |> List.first |> Option.unwrap
    Request { path, ..req }
}

fn add_trace_header(req: Request) -> Request {
    req.with_header("X-Trace", generate_trace_id())
}

fn preprocess(req: Request) -> Request {
    req
        |> normalize_path
        |> strip_query
        |> add_trace_header
}
```

Each stage is a pure function from `Request` to `Request` — easy to test individually, easy to reorder.
