# Mom Application Areas

Mom is designed for situations where you need C's performance and control but cannot afford C's safety problems. Below are the primary application areas with concrete rationale for each.

---

## Systems Programming

Mom compiles to native binaries with a minimal C runtime. It is a direct replacement for C in low-level systems code.

**Use cases:**
- Operating system components (drivers, schedulers, memory allocators)
- Embedded firmware
- Bootloaders and boot services
- Custom kernels

**Why Mom:**
- No GC by default (zero-pause guarantees)
- Regions let you manage arena allocation without fighting a borrow checker
- `unsafe` blocks are auditable islands, not pervasive escape hatches
- Cross-compilation is first-class (`mom build --target aarch64-linux`)

```mom
// Example: a ring buffer that never allocates
struct RingBuf:
    data: [Byte]
    head: Int
    tail: Int
    cap:  Int

impl RingBuf:
    fn push(self, b: Byte) -> Bool:
        let next = (self.tail + 1) % self.cap
        if next == self.head: false
        else:
            self.data[self.tail] = b
            self.tail = next
            true
```

---

## Distributed Systems

Mom's actor model and typed channels map directly onto distributed system concepts: services are actors, messages are typed, supervision handles node restarts.

**Use cases:**
- Microservice orchestration
- Event-driven pipelines
- Message-passing middleware
- Service meshes

**Why Mom:**
- Actors are lightweight (no OS thread per actor)
- Channels are bounded by default — backpressure is built in
- Supervision trees encode restart policies as code, not ops config
- Type-checked messages prevent protocol mismatches at compile time

```mom
enum WorkerMsg:
    Process(String)
    Drain
    Stop

actor Worker:
    state processed: Int
    receive:
        Process(item) => Worker { processed: self.processed + 1 }
        Drain         => self
        Stop          => self
```

---

## AI Infrastructure

AI training and inference pipelines need fine-grained memory control, zero-copy tensor passing, and predictable latency. Mom's region allocator and explicit ownership make it a natural fit.

**Use cases:**
- Tensor runtime engines
- Model serving infrastructure
- Custom BLAS/LAPACK wrappers
- Inference servers with strict P99 latency requirements

**Why Mom:**
- Region allocator for per-request tensor arenas (O(1) free)
- `extern c` binds directly to CUDA, OpenBLAS, oneDNN without marshalling overhead
- Channels + actors for pipeline parallelism (data loading, preprocessing, inference)
- Compile-time constants (`comptime`) for kernel parameters

---

## Networking

Network servers require high throughput, low latency, and correct protocol handling. Mom's async runtime and channel primitives are designed for exactly this.

**Use cases:**
- High-performance HTTP/2, gRPC, WebSocket servers
- Custom TCP/UDP protocol implementations
- DNS resolvers, load balancers, proxies
- Network function virtualization (NFV)

**Why Mom:**
- Async/await with a work-stealing executor (no callback hell)
- Zero-copy parsing with `&[Byte]` borrows
- Actor-per-connection model scales to millions of connections
- Built-in `std::net` with TCP, UDP, TLS, HTTP

```mom
async fn handle(conn: TcpConn):
    let req = await conn.read_request()?
    let resp = route(req)
    await conn.write_response(resp)?
```

---

## High-Performance Backends

Backend services that process large volumes of structured data (APIs, databases, streaming processors) benefit from Mom's type safety and performance.

**Use cases:**
- REST/gRPC API servers
- Database engines and query processors
- Log aggregation pipelines
- Stream processing (Kafka-style)

**Why Mom:**
- Struct packing and cache-friendly data layouts
- Pattern matching on message types with zero overhead
- `Result[T, E]` error propagation — no exception overhead
- Compile to a single static binary with no runtime dependency

---

## Command-Line Tools

Mom produces small, fast, standalone binaries. CLI tools start instantly, use little memory, and can be distributed as single files.

**Use cases:**
- Developer tools (formatters, linters, build systems)
- Data transformation utilities
- System administration scripts that need reliability

**Why Mom:**
- Native binary, no runtime install required
- Pattern matching makes argument parsing clean
- `args()` built-in for CLI arguments

```mom
fn main():
    let argv = args()
    match len(argv):
        1 => print("usage: tool <input>")
        _ => process(argv[1])
```

---

## WebAssembly

Mom can target WebAssembly for browser and edge runtimes via the C/LLVM backend.

**Use cases:**
- Browser plugins and extensions
- Edge computing (Cloudflare Workers, Fastly Compute)
- Portable computation modules

**Why Mom:**
- Minimal runtime maps cleanly to Wasm linear memory
- No GC means no stop-the-world pauses in the browser
- Small binary sizes

---

## What Mom Is Not Optimized For

| Area | Why | Better choice |
|---|---|---|
| Scripting / automation | No REPL-first workflow, no dynamic typing | Python, Ruby |
| Mobile app UI | No UI framework (yet) | Swift, Kotlin |
| Data science / notebooks | No Jupyter integration | Python, Julia |
| Enterprise business logic | No reflection, no dynamic dispatch at runtime | Java, C# |
