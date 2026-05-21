# mom — Language Philosophy

mom is engineered as a **practical, enterprise-grade systems language**, not
an academic experiment. Every design choice is judged by a single question:

> *Does this help engineers ship safe, fast, maintainable systems faster?*

If the answer is no, the feature is cut.

## Five guiding principles

### 1. Simple by default, powerful by extension
Surface syntax must be teachable to a working Python developer in a single
afternoon. Advanced features (generics, traits, actors, FFI, comptime) are
**opt-in** — they never block the reading or writing of straightforward code.

### 2. Safe by default
The language refuses to compile programs that would suffer from null
dereferences, use-after-free, data races, or out-of-bounds access without an
explicit `unsafe` opt-out. Safety is enforced by the compiler, not by
discipline.

### 3. Predictable performance
No hidden allocations, no surprise GC pauses, no implicit copying of large
values, no runtime metaclass machinery. What you write is what executes. If
a function looks O(1), it is O(1).

### 4. Fast to compile
Builds must remain in the seconds-not-minutes range, even at 1M+ LOC, so
that the iteration loop never breaks engineer flow. The compiler is
designed for parallel module compilation, content-addressed caching, and
incremental relinking.

### 5. Self-hosting, no heavy runtime
mom binaries are standalone native executables. There is no JVM, no CLR,
no Node.js, no Python interpreter, no bundled garbage collector. The
runtime is a small, optional library that can be replaced or stripped for
embedded and kernel targets.

## What mom is not

- mom is **not** a scripting language. There is no global interpreter or
  REPL-first development model (the bootstrap interpreter is a development
  aid, not the deployment target).
- mom is **not** a research vehicle. Linear types, dependent types,
  effect systems, and similar academic features are explicitly out of
  scope unless they pull their weight on real workloads.
- mom is **not** a Rust replacement and not a Go replacement. It draws
  ideas from both but trades different complexity for different ergonomics.

## Comparison snapshot

| Property              | mom               | Rust         | Go         | Zig            | Python  |
|-----------------------|-------------------|--------------|------------|----------------|---------|
| Memory model          | regions + linear  | borrow chk   | tracing GC | manual + alloc | tracing GC |
| Concurrency model     | actors + async    | async + threads | goroutines | none built-in | GIL threads |
| Compile time          | very fast (Zig-like) | slow      | fast       | very fast      | n/a     |
| Self-hosted compiler  | yes (planned)     | yes          | yes        | yes            | no      |
| Runtime size          | tiny (opt-in)     | tiny         | medium     | tiny           | large   |
| FFI to C/C++          | first-class       | strong       | cgo (slow) | first-class    | ctypes  |
| Null safety           | yes (Option[T])   | yes          | no         | no             | no      |
| Pattern matching      | exhaustive        | exhaustive   | none       | basic          | structural (3.10+) |
| Learning curve        | low–medium        | high         | low        | medium         | very low |

The aim is the **bottom-left corner** of that table: the ergonomics of
Python and Go with the safety and performance of Rust and Zig, and the
fault tolerance of Erlang.
