# Collections

Mom's primary collection types are **lists** — dynamically-sized, homogeneous sequences written `[T]`. Dictionaries are not yet part of the stage-0 runtime; the pattern-level workarounds are covered at the end of this page. Strings are handled as a special case: they are opaque byte sequences with character-level access via `chars()`.

---

## Lists `[T]`

A list holds zero or more values of the same type. Lists are the building block of almost all data transformations in Mom.

### Creating a list

```mom
let xs = [1, 2, 3, 4, 5]   // list literal — inferred as [Int]
let empty: [Int] = []       // empty list with explicit type annotation
let words = ["hello", "world"]
```

### Indexing

Indexing is **0-based**. Accessing an index that does not exist causes a runtime panic.

```mom
let xs = [10, 20, 30]
print(xs[0])            // 10
print(xs[2])            // 30
print(xs[len(xs) - 1])  // last element: 30
```

Do not index past the end:

```mom
// BAD — panics at runtime:
print(xs[3])
```

### Length

```mom
let n = len(xs)   // Int — number of elements
```

### Appending with `push`

`push` does **not** mutate the list in-place. It returns a new list with the element appended. Because the list is immutable by default, you must bind `mut` and reassign:

```mom
let mut out = []
for i in 0..5:
    push(out, i * i)
print(out)   // [0, 1, 4, 9, 16]
```

The idiomatic pattern is to declare `let mut` before the loop and `push` inside it.

### Removing the last element with `pop`

`pop(xs)` returns a tuple of `(last_element, rest_of_list)`. Like `push`, it does not mutate:

```mom
let xs = [1, 2, 3]
let (last, rest) = pop(xs)
print(last)   // 3
print(rest)   // [1, 2]
```

### Iterating

**Value loop** — most common, iterates each element directly:

```mom
for x in xs:
    print(x)
```

**Index loop** — use a range when you need the position:

```mom
for i in 0..len(xs):
    print(to_string(i) + ": " + to_string(xs[i]))
```

The range `lo..hi` is **exclusive** at the upper bound (like Rust), so `0..5` yields `0, 1, 2, 3, 4`.

### Reversing and sorting

Both return a new copy; the original is unmodified:

```mom
let xs = [3, 1, 4, 1, 5]
let rev = reverse(xs)   // [5, 1, 4, 1, 3]
let srt = sort(xs)      // [1, 1, 3, 4, 5]
```

### Numeric reductions

```mom
let xs = [1, 2, 3, 4, 5]
print(sum(xs))   // 15
```

For boolean lists:

```mom
let flags = [true, false, true]
print(any(flags))   // true  — at least one true
print(all(flags))   // false — not all true
```

### Higher-order operations

`map`, `filter`, and `reduce` accept a function (or lambda) as the first argument.

```mom
let xs = [1, 2, 3, 4, 5]

// map — transform every element
let doubled = map(fn(x: Int) => x * 2, xs)
print(doubled)   // [2, 4, 6, 8, 10]

// filter — keep elements where the predicate is true
let evens = filter(fn(x: Int) => x % 2 == 0, xs)
print(evens)   // [2, 4]

// reduce — fold left with an initial accumulator
let product = reduce(fn(acc: Int, x: Int) => acc * x, xs)
print(product)   // 120
```

You can also pass a named function:

```mom
fn is_positive(x: Int) -> Bool:
    x > 0

let positives = filter(is_positive, [-1, 2, -3, 4])
print(positives)   // [2, 4]
```

### `enumerate` — index-value pairs

`enumerate(xs)` returns a list of `(Int, T)` tuples so you can iterate with both index and value:

```mom
let words = ["alpha", "beta", "gamma"]
for (i, w) in enumerate(words):
    print(to_string(i) + ": " + w)
// 0: alpha
// 1: beta
// 2: gamma
```

### `zip` — pairing two lists

`zip(xs, ys)` produces a list of `(T, U)` tuples, stopping at the shorter list:

```mom
let names = ["Alice", "Bob"]
let scores = [95, 87, 72]
for (name, score) in zip(names, scores):
    print(name + " -> " + to_string(score))
// Alice -> 95
// Bob -> 87
```

### `range` — numeric sequences

`range(lo, hi)` returns a `[Int]` from `lo` up to (but not including) `hi`. Prefer the `lo..hi` syntax inside `for`; use `range` when you need the list as a value:

```mom
let r = range(1, 6)   // [1, 2, 3, 4, 5]
print(sum(r))         // 15
```

---

## Nested lists

A list can contain lists. Each inner list must be the same type `[[T]]`:

```mom
let matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
print(matrix[1][2])   // 6  (row 1, column 2)

for row in matrix:
    for cell in row:
        print(cell)
```

---

## List of structs

Lists hold struct values just like any other type:

```mom
struct Point:
    x: Int
    y: Int

let points = [Point { x: 0, y: 0 }, Point { x: 3, y: 4 }]

for p in points:
    print(to_string(p.x) + ", " + to_string(p.y))
```

Sorting a list of structs requires a custom comparison via `reduce` or a hand-written sort, since `sort` applies to `[Int]` and `[String]` by default.

---

## List of enums

```mom
enum Color:
    Red
    Green
    Blue

let palette = [Red, Green, Blue, Red]

for c in palette:
    match c:
        Red   => print("red")
        Green => print("green")
        Blue  => print("blue")
```

---

## Dictionaries

Mom does not have a built-in dictionary literal in the stage-0 interpreter. The idiomatic workarounds are:

- **Parallel lists**: maintain a `keys: [String]` and `values: [Int]` pair and search with `for`.
- **Struct-as-record**: when the key set is fixed and known at compile time, a struct with named fields is clearer and faster.
- **`std::alloc`**: the native stage-2 standard library will expose `HashMap` under `std::alloc`; this is not yet reachable from `.mom` code.

---

## Strings as sequences

Strings are opaque at the byte level. Use `chars(s)` to obtain a `[String]` where each element is one UTF-8 character:

```mom
let letters = chars("hello")
print(len(letters))   // 5
print(letters[0])     // "h"

for ch in chars("abc"):
    print(ch)
// a
// b
// c
```

Direct indexing into a `String` (`s[i]`) operates at the **byte** level and returns an `Int` (the raw byte value), not a character string. Prefer `chars()` unless you are intentionally working with raw bytes.

---

## Common patterns

### Accumulate into a new list

```mom
fn evens_up_to(n: Int) -> [Int]:
    let mut out = []
    for i in range(0, n):
        if i % 2 == 0:
            push(out, i)
    out
```

### Transform with `map`

```mom
fn square_all(xs: [Int]) -> [Int]:
    map(fn(x: Int) => x * x, xs)
```

### Filter then count

```mom
fn count_positives(xs: [Int]) -> Int:
    let positives = filter(fn(x: Int) => x > 0, xs)
    len(positives)
```

### Build a string from a list

Use `std::fmt.join` or `std::fmt.join_ints` (see [Standard Library](standard-library.md)):

```mom
use std::fmt

let words = ["one", "two", "three"]
print(join(words, ", "))       // one, two, three

let nums = [1, 2, 3]
print(join_ints(nums, " | "))  // 1 | 2 | 3
```
