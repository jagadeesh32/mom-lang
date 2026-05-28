//! Phase 5.1 benchmark harness — `mom bench`.
//!
//! Discovery rules (in order):
//!   1. Every `*.mom` file under `benches/`.
//!   2. Every `*_bench.mom` file under `src/`.
//!   3. Every `.mom` file under `src/` or `tests/` that declares at
//!      least one `#[bench]` function. Each such function runs as
//!      its own bench entry.
//!
//! Files in (1) and (2) execute their `main`; files in (3) execute
//! each `#[bench]`-tagged function with zero arguments. All entries
//! get warmup iterations followed by N measurement iterations; wall
//! clock samples are aggregated into min / median / mean / stddev /
//! max in nanoseconds. The harness is deterministic in sample count
//! and ordering; it does not parallelize because the bootstrap
//! interpreter is single-threaded.
//!
//! The native stage-2 will replace this with a real `Bencher` API
//! that lets `#[bench]` functions report custom throughput numbers;
//! the stage-0 contract is `#[bench] fn name()` only.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::ast::{Item, Program, TypeRef};
use crate::diagnostic::{Diagnostic, LangResult};
use crate::{borrow, interpreter, parse_source, typechecker};

/// Stage-0 `Bencher` struct + `iter` method. The harness prepends this
/// to any source that defines a `#[bench] fn name(b: Bencher)`. The
/// native stage-2 ships the real `Bencher` API from `std::test`, at
/// which point this shim is removed.
const BENCHER_SHIM: &str = "\
struct Bencher { iters: Int }

impl Bencher {
    fn iter(self, body: fn() -> Int) -> Int {
        let mut total = 0
        let mut i = 0
        while i < self.iters {
            total = body()
            i = i + 1
        }
        total
    }
}";

#[derive(Debug, Clone)]
pub struct BenchOptions {
    pub iterations: usize,
    pub warmup: usize,
    pub json: bool,
}

impl BenchOptions {
    pub fn new() -> Self {
        Self {
            iterations: 10,
            warmup: 3,
            json: false,
        }
    }
}

impl Default for BenchOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct BenchOutcome {
    pub path: PathBuf,
    /// `Some(name)` when the entry is a `#[bench] fn name()` inside a
    /// regular source file; `None` when the entry runs the file's
    /// `main` (the benches/ + `_bench.mom` discovery paths).
    pub function: Option<String>,
    pub iterations: usize,
    pub samples_ns: Vec<u128>,
    pub min_ns: u128,
    pub median_ns: u128,
    pub mean_ns: u128,
    pub max_ns: u128,
    pub stddev_ns: u128,
    pub failed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct BenchReport {
    pub outcomes: Vec<BenchOutcome>,
}

impl BenchReport {
    pub fn total(&self) -> usize {
        self.outcomes.len()
    }
    pub fn passed(&self) -> usize {
        self.outcomes.iter().filter(|o| !o.failed).count()
    }
    pub fn failed(&self) -> usize {
        self.outcomes.iter().filter(|o| o.failed).count()
    }
    pub fn all_passed(&self) -> bool {
        self.outcomes.iter().all(|o| !o.failed)
    }
}

pub fn run_all(root: &Path, options: &BenchOptions) -> LangResult<BenchReport> {
    let plan = discover(root)?;
    let mut outcomes: Vec<BenchOutcome> = Vec::with_capacity(plan.len());
    for entry in plan {
        outcomes.push(run_one(&entry, options));
    }
    Ok(BenchReport { outcomes })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchKind {
    /// `#[bench] fn name()` — called once per measurement iteration.
    Plain,
    /// `#[bench] fn name(b: Bencher)` — called once with a `Bencher`
    /// the body uses to drive its inner timing loop. The harness
    /// auto-injects a `Bencher` struct with a default `iters` count.
    WithBencher,
}

#[derive(Debug, Clone)]
enum BenchEntry {
    /// Run the file's `main()`.
    Main(PathBuf),
    /// Run a specific `#[bench] fn name(...)` inside the file.
    Attr {
        path: PathBuf,
        function: String,
        kind: BenchKind,
    },
}

fn discover(root: &Path) -> LangResult<Vec<BenchEntry>> {
    let mut entries: Vec<BenchEntry> = Vec::new();

    let bench_dir = root.join("benches");
    walk(&bench_dir, &mut |path| {
        if has_extension(path, "mom") {
            entries.push(BenchEntry::Main(path.to_path_buf()));
        }
    })?;

    let src_dir = root.join("src");
    let mut scanned: Vec<PathBuf> = Vec::new();
    walk(&src_dir, &mut |path| {
        if has_extension(path, "mom") {
            scanned.push(path.to_path_buf());
        }
    })?;
    let tests_dir = root.join("tests");
    walk(&tests_dir, &mut |path| {
        if has_extension(path, "mom") {
            scanned.push(path.to_path_buf());
        }
    })?;

    for path in scanned {
        let is_main_entry = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|name| name.ends_with("_bench"))
            .unwrap_or(false);
        if is_main_entry {
            entries.push(BenchEntry::Main(path.clone()));
            continue;
        }
        // Cheap pre-filter so we only parse files that mention `#[`.
        if let Ok(source) = fs::read_to_string(&path) {
            if !source.contains("#[") {
                continue;
            }
            for (function, kind) in extract_bench_functions(&source) {
                entries.push(BenchEntry::Attr {
                    path: path.clone(),
                    function,
                    kind,
                });
            }
        }
    }

    entries.sort_by(|a, b| entry_key(a).cmp(&entry_key(b)));
    Ok(entries)
}

fn entry_key(entry: &BenchEntry) -> (PathBuf, Option<String>) {
    match entry {
        BenchEntry::Main(p) => (p.clone(), None),
        BenchEntry::Attr { path, function, .. } => (path.clone(), Some(function.clone())),
    }
}

fn extract_bench_functions(source: &str) -> Vec<(String, BenchKind)> {
    let program = match parse_source(source) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<(String, BenchKind)> = Vec::new();
    collect_bench_fns(&program, &mut out);
    out
}

fn collect_bench_fns(program: &Program, out: &mut Vec<(String, BenchKind)>) {
    for item in &program.items {
        if let Item::Function(f) = item {
            if !f.attrs.iter().any(|a| a == "bench") {
                continue;
            }
            let kind = match f.params.as_slice() {
                [] => BenchKind::Plain,
                [param] if is_bencher_type(&param.ty) => BenchKind::WithBencher,
                _ => continue,
            };
            out.push((f.name.clone(), kind));
        }
    }
}

fn is_bencher_type(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::Named(name) => name == "Bencher",
        _ => false,
    }
}

fn run_one(entry: &BenchEntry, options: &BenchOptions) -> BenchOutcome {
    let (path, function, kind) = match entry {
        BenchEntry::Main(p) => (p.clone(), None, BenchKind::Plain),
        BenchEntry::Attr {
            path,
            function,
            kind,
        } => (path.clone(), Some(function.clone()), *kind),
    };
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(err) => {
            return failure(&path, function, options, format!("read failed: {err}"));
        }
    };

    let run = || -> LangResult<()> {
        match &function {
            None => execute_main(&source).map(|_| ()),
            Some(name) => execute_bench_fn(&source, name, kind).map(|_| ()),
        }
    };

    for _ in 0..options.warmup {
        if let Err(diag) = run() {
            return failure(&path, function, options, format!("warmup failed: {diag}"));
        }
    }

    let iterations = options.iterations.max(1);
    let mut samples_ns: Vec<u128> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        if let Err(diag) = run() {
            return failure(
                &path,
                function,
                options,
                format!("iteration failed: {diag}"),
            );
        }
        samples_ns.push(start.elapsed().as_nanos());
    }

    let (min_ns, median_ns, mean_ns, max_ns, stddev_ns) = summarize(&samples_ns);
    BenchOutcome {
        path,
        function,
        iterations,
        samples_ns,
        min_ns,
        median_ns,
        mean_ns,
        max_ns,
        stddev_ns,
        failed: false,
        message: String::new(),
    }
}

fn failure(
    path: &Path,
    function: Option<String>,
    options: &BenchOptions,
    message: String,
) -> BenchOutcome {
    BenchOutcome {
        path: path.to_path_buf(),
        function,
        iterations: options.iterations,
        samples_ns: Vec::new(),
        min_ns: 0,
        median_ns: 0,
        mean_ns: 0,
        max_ns: 0,
        stddev_ns: 0,
        failed: true,
        message,
    }
}

fn execute_main(source: &str) -> LangResult<String> {
    let program = parse_source(source)?;
    typechecker::TypeChecker::new().check_program(&program)?;
    borrow::BorrowChecker::new().check_program(&program)?;
    interpreter::Interpreter::new().run_program(&program)
}

fn execute_bench_fn(source: &str, function: &str, kind: BenchKind) -> LangResult<String> {
    // Rewrite the source so the requested `#[bench] fn name(...)` is
    // driven by a fresh `main`. The stage-0 interpreter only enters
    // via `main`, so we shim by:
    //   * plain:        `main` calls `name()`
    //   * with-bencher: prepend a stage-0 `Bencher` struct + iter impl,
    //                   then call `name(Bencher { iters: 50 })`
    let augmented = match kind {
        BenchKind::Plain => format!(
            "{source}\n\n// auto-generated by `mom bench` driver\nfn __mom_bench_main() {{ {function}() }}\n"
        ),
        BenchKind::WithBencher => format!(
            "{BENCHER_SHIM}\n\n{source}\n\n// auto-generated by `mom bench` driver\nfn __mom_bench_main() {{ {function}(Bencher {{ iters: 50 }}) }}\n"
        ),
    };
    let mut program = parse_source(&augmented)?;
    // Promote __mom_bench_main → main if no main exists. If a main
    // already exists, leave it and instead just trust the user's main
    // already drives `function()` somewhere — but the simpler path is
    // to *replace* any main entirely so our shim wins.
    let mut filtered_items: Vec<Item> = Vec::new();
    for item in program.items.drain(..) {
        match &item {
            Item::Function(f) if f.name == "main" => { /* drop user main */ }
            _ => filtered_items.push(item),
        }
    }
    for item in &mut filtered_items {
        if let Item::Function(f) = item {
            if f.name == "__mom_bench_main" {
                f.name = "main".to_string();
            }
        }
    }
    program.items = filtered_items;
    typechecker::TypeChecker::new().check_program(&program)?;
    borrow::BorrowChecker::new().check_program(&program)?;
    interpreter::Interpreter::new().run_program(&program)
}

fn summarize(samples: &[u128]) -> (u128, u128, u128, u128, u128) {
    if samples.is_empty() {
        return (0, 0, 0, 0, 0);
    }
    let mut sorted: Vec<u128> = samples.to_vec();
    sorted.sort_unstable();
    let min_ns = sorted[0];
    let max_ns = *sorted.last().unwrap();
    let median_ns = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2
    };
    let sum: u128 = sorted.iter().sum();
    let mean_ns = sum / sorted.len() as u128;
    // Welford-style would be nicer; for ~10 samples this is plenty.
    let variance: u128 = sorted
        .iter()
        .map(|s| {
            let d = if *s > mean_ns {
                s - mean_ns
            } else {
                mean_ns - s
            };
            d * d
        })
        .sum::<u128>()
        / sorted.len() as u128;
    let stddev_ns = isqrt(variance);
    (min_ns, median_ns, mean_ns, max_ns, stddev_ns)
}

fn isqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut x = value;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    x
}

pub fn render_text(report: &BenchReport) -> String {
    let mut out = String::new();
    for outcome in &report.outcomes {
        let label = format_label(outcome);
        if outcome.failed {
            out.push_str(&format!("FAIL {label} — {}\n", outcome.message));
            continue;
        }
        out.push_str(&format!(
            "bench {label} ({} iters): min={} median={} mean={} stddev={} max={}\n",
            outcome.iterations,
            human(outcome.min_ns),
            human(outcome.median_ns),
            human(outcome.mean_ns),
            human(outcome.stddev_ns),
            human(outcome.max_ns),
        ));
    }
    out
}

fn format_label(outcome: &BenchOutcome) -> String {
    match &outcome.function {
        None => outcome.path.display().to_string(),
        Some(name) => format!("{}::{name}", outcome.path.display()),
    }
}

pub fn render_json(report: &BenchReport) -> String {
    let mut out = String::new();
    out.push_str("{\"benches\":[");
    for (i, outcome) in report.outcomes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"path\":\"{}\",\"function\":{},\"iterations\":{},\"min_ns\":{},\"median_ns\":{},\"mean_ns\":{},\"max_ns\":{},\"stddev_ns\":{},\"failed\":{}",
            escape(&outcome.path.display().to_string()),
            match &outcome.function {
                Some(name) => format!("\"{}\"", escape(name)),
                None => "null".to_string(),
            },
            outcome.iterations,
            outcome.min_ns,
            outcome.median_ns,
            outcome.mean_ns,
            outcome.max_ns,
            outcome.stddev_ns,
            outcome.failed,
        ));
        if outcome.failed {
            out.push_str(&format!(",\"message\":\"{}\"", escape(&outcome.message)));
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn human(ns: u128) -> String {
    if ns < 1_000 {
        format!("{}ns", ns)
    } else if ns < 1_000_000 {
        let us = ns as f64 / 1_000.0;
        format!("{:.2}us", us)
    } else if ns < 1_000_000_000 {
        let ms = ns as f64 / 1_000_000.0;
        format!("{:.2}ms", ms)
    } else {
        let s = ns as f64 / 1_000_000_000.0;
        format!("{:.2}s", s)
    }
}

fn walk(dir: &Path, on_file: &mut dyn FnMut(&Path)) -> LangResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|err| {
        Diagnostic::at_start(format!("failed to read '{}': {err}", dir.display()))
    })?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::at_start(format!("failed to read '{}': {err}", dir.display()))
        })?;
        paths.push(entry.path());
    }
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, on_file)?;
        } else {
            on_file(&path);
        }
    }
    Ok(())
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some(ext)
}
