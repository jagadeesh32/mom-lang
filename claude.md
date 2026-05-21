I want to design and build a modern self-hosted systems programming language from scratch.

Vision:
Create a language that combines:
- Python-like simplicity and readability
- C-level performance
- Rust-like memory safety and security
- Erlang-style concurrency and fault tolerance
- Functional programming support
- Zig-like extremely fast build system and compilation speed
- Native interoperability with C and C++ libraries/modules
- Self-hosting compiler architecture without depending on external runtimes

The language should:
- compile directly to native machine code
- avoid heavy external runtimes if possible
- have minimal runtime overhead
- eventually compile itself (self-hosting compiler)
- be suitable for operating systems, distributed systems, AI infrastructure, networking, and high-performance backend systems

Help me design this language like a senior compiler engineer and programming language architect.

========================
LANGUAGE REQUIREMENTS
========================

1. Core Philosophy
- Simplicity over unnecessary complexity
- Safe by default
- Fast compilation
- Minimal hidden behavior
- Easy debugging
- Explicit concurrency
- Predictable performance
- Excellent developer experience
- Small runtime footprint
- Native execution without VM dependency

2. Syntax Design
Design syntax that is:
- readable like Python
- structured like Rust
- lightweight like Go/Zig

Include examples for:
- variables
- constants
- functions
- lambdas
- structs
- enums
- interfaces/traits
- generics
- pattern matching
- modules/packages
- error handling
- async/await
- actors/processes
- concurrency primitives
- immutable by default
- optional mutability
- compile-time features
- metaprogramming/macros

3. Memory Safety
Design a simpler alternative to Rust ownership if possible:
- memory-safe by default
- avoid garbage collector if possible
- prevent:
  - null pointer bugs
  - use-after-free
  - data races
  - buffer overflows
- explain tradeoffs

4. Concurrency Model
Combine:
- Erlang actor-style concurrency
- async runtime
- lightweight processes
- message passing
- fault isolation
- supervised tasks

Show examples.

5. Functional Programming Features
Include:
- immutable data
- higher-order functions
- pattern matching
- pipelines
- algebraic data types
- pure functions support

6. Fast Build System
Design:
- Zig-like ultra-fast incremental compilation
- parallel builds
- caching
- reproducible builds
- built-in package manager
- cross compilation support

7. C and C++ Interoperability
Support:
- importing C libraries directly
- using existing C++ modules
- FFI design
- ABI compatibility
- gradual migration from C/C++ projects

Show examples:
- calling C functions
- importing C++ libraries
- linking native modules

8. Self-Hosting Compiler Architecture
Design the language so that:
- the compiler can eventually be rewritten in the language itself
- it can bootstrap itself
- it does not require JVM, .NET, Node.js, Python, or other heavy runtimes
- generated binaries are standalone native executables
- runtime dependencies are minimal
- startup speed is extremely fast

Explain:
- bootstrap stages
- compiler evolution strategy
- self-hosting challenges
- how Zig/Rust/Go handled bootstrapping

========================
COMPILER DESIGN
========================

Design the compiler architecture:
- lexer
- parser
- AST
- semantic analysis
- type inference
- memory safety checker
- concurrency safety checker
- IR design
- optimizer
- LLVM backend or custom backend
- linker
- runtime system

Explain:
- compile pipeline
- performance optimizations
- build speed optimizations

========================
TOOLING
========================

Design:
- formatter
- linter
- package manager
- debugger support
- language server (LSP)
- testing framework
- documentation generator

========================
ROADMAP
========================

Create:
1. MVP roadmap
2. Folder structure
3. Compiler implementation phases
4. Syntax specification
5. Grammar
6. Example programs
7. Bootstrap strategy
8. Timeline estimation
9. Team requirements
10. Risks and engineering challenges

========================
IMPLEMENTATION
========================

Recommend:
- best implementation language
  (Rust, Zig, or C++)
- best parser libraries
- LLVM vs custom backend
- runtime architecture
- concurrency runtime model

Start by:
1. defining language philosophy
2. proposing language name ideas
3. creating syntax examples
4. designing grammar
5. implementing the first lexer
6. implementing parser architecture
7. defining type system

Keep everything practical and engineering-focused, not academic.
