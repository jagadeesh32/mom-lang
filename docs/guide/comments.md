# Comments

Mom supports two comment styles. Comments are ignored by the compiler and have no runtime effect.

---

## Line Comments

A line comment begins with `//` and extends to the end of the line.

```mom
// This is a full-line comment.
let x = 42  // This is an inline comment.
```

---

## Block Comments

Block comments begin with `/*` and end with `*/`. They can span multiple lines or appear inline.

```mom
/* This is a
   multi-line block comment. */

let y = /* inline block comment */ 10
```

Block comments do **not** nest:

```mom
/* outer /* inner */ still in comment */
//                   ^-- this closes the block, "still in comment */" is code (error)
```

---

## Documentation Comments

The `mom doc` tool generates Markdown API documentation from comments placed immediately before a declaration. By convention, use `//` lines starting with a capital letter and a period.

```mom
// Computes the nth Fibonacci number.
// Returns 0 for n <= 0.
fn fib(n: Int) -> Int:
    if n <= 1: n
    else: fib(n - 1) + fib(n - 2)
```

Run `mom doc myfile.mom` to produce `myfile.md` with the documented API.

---

## When to Comment

Mom code is generally self-documenting through clear names. Add a comment when:

- The **why** is not obvious from the code (a hidden constraint, a workaround, a subtle invariant)
- The algorithm is non-trivial
- A public API needs usage guidance

Do not comment what the code does — well-named identifiers already say that.

```mom
// BAD: says what, not why
// increment count by 1
count = count + 1

// GOOD: explains a non-obvious constraint
// Skip index 0 — it is reserved for the sentinel node.
for i in 1..len(items):
    process(items[i])
```

---

## Disabling Code

Use `//` to temporarily disable a line during debugging:

```mom
fn main():
    print("step 1")
    // print("step 2")  // disabled for now
    print("step 3")
```

---

## TODO / FIXME Convention

Common markers that `mom lint` can flag:

```mom
// TODO: implement proper error handling here
// FIXME: this crashes on empty input
// HACK: workaround for compiler bug #42
// NOTE: this relies on the order of enum variants
```
