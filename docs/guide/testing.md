# Testing

Mom has first-class support for tests: a built-in test runner, an assertion vocabulary in `std::test`, and a `#[test]` attribute that marks functions for automatic discovery.

---

## Marking a test function

Prefix any zero-argument function with the `#[test]` attribute:

```mom
#[test]
fn addition_works():
    assert(1 + 1 == 2)
```

Test functions must:

- Take no parameters.
- Return `Unit` (implicitly, by not returning a value).
- Not be `pub` or `async` (plain `fn` only).

---

## Running tests

```sh
# Run all tests under the current directory
mom test

# Run all tests under a specific directory
mom test tests/

# Run a single test file
mom test tests/math_test.mom
```

The test runner discovers every `.mom` file under the given directory (default: `.`), collects all `#[test]` functions inside them, runs each one, and prints a summary.

---

## Test output format

Each test prints one line:

```
ok    addition_works
FAIL  subtraction_works: assertion failed
```

At the end:

```
passed=3 failed=1
```

The runner exits with **code 0** if every test passes, or **code 1** if any test fails. This integrates cleanly with CI pipelines:

```sh
mom test tests/ || echo "Tests failed"
```

---

## Basic assertions

### `assert(condition)`

The built-in `assert` terminates the program with a panic if `condition` is `false`:

```mom
#[test]
fn sum_of_list():
    let xs = [1, 2, 3, 4, 5]
    assert(sum(xs) == 15)
```

A failing `assert` prints a message and halts execution of that test immediately. The runner catches the panic, marks the test as failed, and continues with the next test.

### `assert_eq_int` — better error messages

When asserting integer equality, prefer `assert_eq_int` from `std::test`. It prints both the actual and expected values on failure:

```mom
use std::test

#[test]
fn fibonacci_tenth():
    let mut stats = new_stats()
    stats = assert_eq_int(stats, fib(10), 55, "fib(10)")
    print(stats.summary())
```

Output on failure:

```
FAIL  fib(10): got 54, want 55
passed=0 failed=1
```

---

## Using `std::test`

### `TestStats` accumulator

`TestStats` lets you run multiple assertions in one test function and get a full summary at the end, rather than stopping at the first failure:

```mom
use std::test

#[test]
fn math_properties():
    let mut stats = new_stats()
    stats = assert_eq_int(stats, 2 + 2, 4,   "2+2=4")
    stats = assert_eq_int(stats, 3 * 3, 9,   "3*3=9")
    stats = assert_eq_int(stats, 10 - 7, 3,  "10-7=3")
    stats = assert_true(stats,  2 < 3,       "2<3")
    stats = assert_false(stats, 5 > 100,     "5>100")
    print(stats.summary())
    assert(stats.failed == 0)
```

The final `assert(stats.failed == 0)` makes the test fail if any assertion in the block failed.

### `assert_true` and `assert_false`

```mom
stats = assert_true(stats,  len([1, 2, 3]) == 3, "list length")
stats = assert_false(stats, len([]) > 0,          "empty list")
```

Passing output:

```
ok    list length
ok    empty list
```

---

## Testing functions that return `Result`

Use `result_or` from `std::core` to extract the value, or pattern-match directly:

```mom
use std::test

fn safe_div(a: Int, b: Int) -> Result[Int, String]:
    if b == 0:
        Err("division by zero")
    else:
        Ok(a / b)

#[test]
fn division_ok():
    let mut stats = new_stats()
    let r = safe_div(10, 2)
    stats = assert_eq_int(stats, result_or(r, -1), 5, "10/2=5")
    print(stats.summary())
    assert(stats.failed == 0)

#[test]
fn division_by_zero_returns_err():
    let r = safe_div(10, 0)
    match r:
        Ok(_)  => assert(false)    // should not reach here
        Err(_) => assert(true)
```

---

## Testing functions that return `Option`

```mom
use std::test

fn find_first(xs: [Int], target: Int) -> Option[Int]:
    for i in 0..len(xs):
        if xs[i] == target:
            return Some(i)
    None

#[test]
fn find_present():
    let idx = find_first([10, 20, 30], 20)
    let mut stats = new_stats()
    stats = assert_eq_int(stats, option_or(idx, -1), 1, "found at index 1")
    print(stats.summary())
    assert(stats.failed == 0)

#[test]
fn find_absent():
    let idx = find_first([10, 20, 30], 99)
    stats = assert_eq_int(stats, option_or(idx, -1), -1, "not found -> -1")
    assert(stats.failed == 0)
```

---

## Testing expected panics

Mom does not yet have a `#[should_panic]` attribute (planned for stage-2). To test that a code path panics, structure the test to avoid the panic and verify error-path behavior instead:

```mom
// Instead of testing that out-of-bounds panics, test the guard condition:
#[test]
fn index_guard():
    let xs = [1, 2, 3]
    assert(len(xs) > 0)          // safe: list is non-empty
    assert(2 < len(xs))          // safe: index 2 is valid
    print(xs[2])                 // 3
```

For functions that return `Result` or `Option` on invalid input, test those return values directly (see above).

---

## Benchmarking with `#[bench]`

The `#[bench]` attribute marks a function for the `mom bench` command. Benchmark functions have the same shape as test functions:

```mom
#[bench]
fn bench_fib_20():
    fib(20)

#[bench]
fn bench_sort_1000():
    let mut xs = []
    for i in range(0, 1000):
        push(xs, 1000 - i)
    sort(xs)
```

Run benchmarks:

```sh
mom bench            # all #[bench] functions under src/ and tests/
mom bench src/       # only under src/
```

The runner reports the wall-clock time for each function. Benchmarks do not contribute to the pass/fail exit code.

---

## Organizing tests

### Co-located tests

The simplest layout: test functions live in the same file as the code they test.

```
src/
  math.mom        // contains both fn gcd() and #[test] fn test_gcd()
```

This is convenient for small modules. Use it when the test count is low and the test code is unlikely to be shipped to users.

### Dedicated `tests/` directory

For larger projects, put tests in a separate directory:

```
src/
  math.mom
tests/
  math_test.mom
  string_test.mom
```

Run with `mom test tests/`. Test files do not need a `main()` function; the runner provides the entry point.

### Integration tests

Integration tests exercise the public interface of a module as a whole. Place them under `tests/integration/` and run them separately:

```sh
mom test tests/integration/
```

An integration test file looks exactly like a unit test file — it imports the modules under test and calls their public functions:

```mom
// tests/integration/pipeline_test.mom
use std::fmt
use std::math

#[test]
fn pipeline_roundtrip():
    let value = 20 |> fn(x: Int) => x + 1 |> fn(x: Int) => x * 2
    assert(value == 42)

#[test]
fn join_after_map():
    let xs = [1, 2, 3]
    let doubled = map(fn(x: Int) => x * 2, xs)
    let mut stats = new_stats()
    stats = assert_eq_int(stats, len(doubled), 3, "length preserved")
    print(join_ints(doubled, ", "))   // 2, 4, 6
    assert(stats.failed == 0)
```

---

## Full worked example

The following test file covers a small `stats` utility using `TestStats` throughout:

```mom
// tests/stats_test.mom
use std::test

fn mean(xs: [Int]) -> Int:
    if len(xs) == 0:
        return 0
    sum(xs) / len(xs)

fn range_span(xs: [Int]) -> Int:
    if len(xs) == 0:
        return 0
    let s = sort(xs)
    s[len(s) - 1] - s[0]

#[test]
fn test_mean():
    let mut stats = new_stats()
    stats = assert_eq_int(stats, mean([2, 4, 6]),     4, "mean [2,4,6]")
    stats = assert_eq_int(stats, mean([1, 1, 1, 1]),  1, "mean all-ones")
    stats = assert_eq_int(stats, mean([]),             0, "mean empty")
    print(stats.summary())
    assert(stats.failed == 0)

#[test]
fn test_range_span():
    let mut stats = new_stats()
    stats = assert_eq_int(stats, range_span([3, 1, 4, 1, 5]), 4, "span of [3,1,4,1,5]")
    stats = assert_eq_int(stats, range_span([7]),              0, "span of singleton")
    stats = assert_eq_int(stats, range_span([]),               0, "span of empty")
    print(stats.summary())
    assert(stats.failed == 0)
```

Run it:

```sh
mom test tests/stats_test.mom
```

Expected output:

```
ok    mean [2,4,6]
ok    mean all-ones
ok    mean empty
passed=3 failed=0
ok    span of [3,1,4,1,5]
ok    span of singleton
ok    span of empty
passed=3 failed=0
```

Exit code: `0`.

---

## Quick reference

| Mechanism | When to use |
|---|---|
| `assert(cond)` | Single boolean condition; stops on first failure |
| `assert_eq_int(stats, a, b, label)` | Integer equality with descriptive failure output |
| `assert_true(stats, cond, label)` | Boolean true; accumulates in `TestStats` |
| `assert_false(stats, cond, label)` | Boolean false; accumulates in `TestStats` |
| `#[test]` | Mark a function for `mom test` |
| `#[bench]` | Mark a function for `mom bench` |
| `mom test [dir]` | Discover and run tests; exit 0 = pass, 1 = fail |
| `mom bench [dir]` | Discover and run benchmarks; reports wall time |
