//! Phase 5.1 profiler harness — `mom prof`.
//!
//! The native stage-2 ships a sampling profiler with off-CPU support
//! and full pprof / OTLP output. The bootstrap stage-0 here is a
//! tracing profiler: the interpreter's `call_function` and
//! `call_lambda` paths drive an `Interpreter::probe` hook that
//! records function entry / exit timestamps. From those events we
//! produce three output formats:
//!
//!   * **text**    — sorted table of per-function self / total ns / calls
//!   * **folded**  — Brendan Gregg flamegraph "folded stack" format
//!   * **pprof**   — JSON-shaped pprof-lite (samples, locations, fns)
//!
//! Folded output is the default because it round-trips into both
//! `flamegraph.pl` and the in-tree `mom dbg` viewer.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use crate::diagnostic::LangResult;
use crate::{borrow, interpreter, parse_source, typechecker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfFormat {
    Text,
    Folded,
    Pprof,
    /// pprof's canonical proto3 wire format. Bytes returned from
    /// `render` are the raw `Profile` message; callers usually want
    /// to `gzip -c` before piping into `go tool pprof`.
    PprofProto,
}

impl ProfFormat {
    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "text" => Some(Self::Text),
            "folded" => Some(Self::Folded),
            "pprof" => Some(Self::Pprof),
            "pprof-proto" | "proto" => Some(Self::PprofProto),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfReport {
    pub total_calls: u64,
    pub total_ns: u128,
    pub functions: Vec<FunctionStat>,
    pub folded: BTreeMap<String, u128>,
}

#[derive(Debug, Clone)]
pub struct FunctionStat {
    pub name: String,
    pub calls: u64,
    pub self_ns: u128,
    pub total_ns: u128,
}

/// Mutable state behind a `Rc<RefCell<_>>` so the interpreter can
/// pump enter/exit events without owning the profiler. Externally
/// you only construct it via `Profiler::new()`.
#[derive(Debug, Default)]
pub struct ProfilerState {
    started_at: Option<Instant>,
    stack: Vec<Frame>,
    fn_self_ns: BTreeMap<String, u128>,
    fn_total_ns: BTreeMap<String, u128>,
    fn_calls: BTreeMap<String, u64>,
    folded: BTreeMap<String, u128>,
    open_in_fn: BTreeMap<String, u32>,
}

#[derive(Debug, Clone)]
struct Frame {
    name: String,
    entered_ns: u128,
    child_ns: u128,
}

impl ProfilerState {
    pub fn enter(&mut self, name: &str) {
        let now_ns = self.now_ns();
        let depth = *self.open_in_fn.get(name).unwrap_or(&0);
        self.open_in_fn.insert(name.to_string(), depth + 1);
        if depth == 0 {
            *self.fn_calls.entry(name.to_string()).or_insert(0) += 1;
        } else {
            // Re-entrant call to the same function (recursion): still
            // counts as a separate invocation for "calls".
            *self.fn_calls.entry(name.to_string()).or_insert(0) += 1;
        }
        self.stack.push(Frame {
            name: name.to_string(),
            entered_ns: now_ns,
            child_ns: 0,
        });
    }

    pub fn exit(&mut self, name: &str) {
        let now_ns = self.now_ns();
        let frame = match self.stack.pop() {
            Some(frame) => frame,
            None => return,
        };
        if frame.name != name {
            // Mismatched exit — push it back so we don't corrupt the
            // accounting and bail out quietly.
            self.stack.push(frame);
            return;
        }
        let elapsed = now_ns.saturating_sub(frame.entered_ns);
        let self_time = elapsed.saturating_sub(frame.child_ns);

        let depth = *self.open_in_fn.get(name).unwrap_or(&1);
        if depth <= 1 {
            self.open_in_fn.remove(name);
            *self.fn_total_ns.entry(name.to_string()).or_insert(0) += elapsed;
        } else {
            self.open_in_fn.insert(name.to_string(), depth - 1);
        }
        *self.fn_self_ns.entry(name.to_string()).or_insert(0) += self_time;

        if let Some(parent) = self.stack.last_mut() {
            parent.child_ns = parent.child_ns.saturating_add(elapsed);
        }

        let stack_key = self.current_stack_key(name);
        *self.folded.entry(stack_key).or_insert(0) += self_time;
    }

    fn now_ns(&mut self) -> u128 {
        let started = *self
            .started_at
            .get_or_insert_with(Instant::now);
        started.elapsed().as_nanos()
    }

    fn current_stack_key(&self, leaf: &str) -> String {
        let mut parts: Vec<&str> = self.stack.iter().map(|f| f.name.as_str()).collect();
        parts.push(leaf);
        parts.join(";")
    }

    pub fn report(&self) -> ProfReport {
        let mut functions: Vec<FunctionStat> = self
            .fn_self_ns
            .iter()
            .map(|(name, self_ns)| FunctionStat {
                name: name.clone(),
                calls: *self.fn_calls.get(name).unwrap_or(&0),
                self_ns: *self_ns,
                total_ns: *self.fn_total_ns.get(name).unwrap_or(self_ns),
            })
            .collect();
        functions.sort_by(|a, b| b.self_ns.cmp(&a.self_ns).then_with(|| a.name.cmp(&b.name)));
        let total_calls = functions.iter().map(|f| f.calls).sum();
        let total_ns = self.folded.values().sum();
        ProfReport {
            total_calls,
            total_ns,
            functions,
            folded: self.folded.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Profiler {
    state: Rc<RefCell<ProfilerState>>,
}

impl Profiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&self) -> Rc<RefCell<ProfilerState>> {
        Rc::clone(&self.state)
    }

    pub fn report(&self) -> ProfReport {
        self.state.borrow().report()
    }
}

pub fn profile_source(source: &str) -> LangResult<(String, ProfReport)> {
    let program = parse_source(source)?;
    typechecker::TypeChecker::new().check_program(&program)?;
    borrow::BorrowChecker::new().check_program(&program)?;
    let profiler = Profiler::new();
    let mut interp = interpreter::Interpreter::new();
    interp.attach_probe(profiler.handle());
    let out = interp.run_program(&program)?;
    Ok((out, profiler.report()))
}

pub fn render(report: &ProfReport, format: ProfFormat) -> String {
    match format {
        ProfFormat::Text => render_text(report),
        ProfFormat::Folded => render_folded(report),
        ProfFormat::Pprof => render_pprof(report),
        ProfFormat::PprofProto => {
            // Binary; round-trip through latin-1 so the CLI's print!
            // can stream it. Tests use `render_pprof_proto_bytes`
            // directly to avoid the round-trip.
            let bytes = render_pprof_proto_bytes(report);
            // Treat each byte as a unicode code point ≤ 0xFF so the
            // String stays byte-faithful for the I/O path. Callers
            // that want truly binary output should use the bytes API.
            bytes.iter().map(|b| *b as char).collect()
        }
    }
}

pub fn render_pprof_proto_bytes(report: &ProfReport) -> Vec<u8> {
    // pprof.proto canonical schema (subset used here):
    //   message Profile {
    //     repeated ValueType sample_type = 1;
    //     repeated Sample    sample      = 2;
    //     repeated Location  location    = 4;
    //     repeated Function  function    = 5;
    //     repeated string    string_table = 6;
    //   }
    //   message ValueType { int64 type = 1; int64 unit = 2; }
    //   message Sample    { repeated uint64 location_id = 1; repeated int64 value = 2; }
    //   message Location  { uint64 id = 1; repeated Line line = 4; }
    //   message Line      { uint64 function_id = 1; }
    //   message Function  { uint64 id = 1; int64 name = 2; }
    //
    // Strings live in the string_table, referenced by 0-based index;
    // the empty string is mandatory at index 0.
    let mut strings: Vec<String> = vec![String::new()];
    let intern = |s: &str, strings: &mut Vec<String>| -> i64 {
        if let Some(i) = strings.iter().position(|x| x == s) {
            return i as i64;
        }
        strings.push(s.to_string());
        (strings.len() - 1) as i64
    };

    let cpu_idx = intern("cpu", &mut strings);
    let unit_idx = intern("nanoseconds", &mut strings);

    // Build per-function string indices and ids.
    // Functions are 1-indexed in the pprof wire form.
    let mut fn_name_to_id: std::collections::BTreeMap<String, u64> =
        std::collections::BTreeMap::new();
    let mut next_fn_id: u64 = 1;
    for f in &report.functions {
        if !fn_name_to_id.contains_key(&f.name) {
            fn_name_to_id.insert(f.name.clone(), next_fn_id);
            next_fn_id += 1;
        }
    }
    // Also include any function names that only appear in a folded
    // stack (e.g. <lambda>) but not in the per-fn table.
    for (stack, _) in &report.folded {
        for name in stack.split(';') {
            if !fn_name_to_id.contains_key(name) {
                fn_name_to_id.insert(name.to_string(), next_fn_id);
                next_fn_id += 1;
            }
        }
    }

    // Each function gets one Location with the same id.
    // (The stage-0 profiler has no line-number info; locations map 1:1 to functions.)
    let mut out = Vec::new();

    // sample_type (field 1, message)
    {
        let mut vt = Vec::new();
        proto_write_varint_field(&mut vt, 1, cpu_idx as u64);
        proto_write_varint_field(&mut vt, 2, unit_idx as u64);
        proto_write_length_delimited(&mut out, 1, &vt);
    }

    // sample (field 2, message) — one per folded stack
    for (stack, count) in &report.folded {
        let mut sample = Vec::new();
        // location_id (packed uint64, field 1). pprof accepts either
        // packed or non-packed; emit non-packed for simplicity.
        // Sample stack is "main;helper;leaf" — pprof expects leaf
        // first, so reverse.
        let frames: Vec<&str> = stack.split(';').collect();
        for frame in frames.iter().rev() {
            if let Some(&id) = fn_name_to_id.get(*frame) {
                proto_write_varint_field(&mut sample, 1, id);
            }
        }
        // value (field 2) — single int64
        proto_write_varint_field(&mut sample, 2, *count as u64);
        proto_write_length_delimited(&mut out, 2, &sample);
    }

    // location (field 4, message) — one per function id
    for (_name, id) in &fn_name_to_id {
        let mut loc = Vec::new();
        proto_write_varint_field(&mut loc, 1, *id);
        // Line message (field 4)
        let mut line = Vec::new();
        proto_write_varint_field(&mut line, 1, *id); // function_id
        proto_write_length_delimited(&mut loc, 4, &line);
        proto_write_length_delimited(&mut out, 4, &loc);
    }

    // function (field 5, message)
    for (name, id) in &fn_name_to_id {
        let mut func = Vec::new();
        proto_write_varint_field(&mut func, 1, *id);
        let name_idx = intern(name, &mut strings);
        proto_write_varint_field(&mut func, 2, name_idx as u64);
        proto_write_length_delimited(&mut out, 5, &func);
    }

    // string_table (field 6, repeated string)
    for s in &strings {
        proto_write_length_delimited(&mut out, 6, s.as_bytes());
    }

    out
}

// --- minimal proto3 encoder ----------------------------------------------

fn proto_write_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    out.push(value as u8);
}

fn proto_write_tag(out: &mut Vec<u8>, field: u32, wire_type: u8) {
    proto_write_varint(out, ((field as u64) << 3) | wire_type as u64);
}

fn proto_write_varint_field(out: &mut Vec<u8>, field: u32, value: u64) {
    proto_write_tag(out, field, 0);
    proto_write_varint(out, value);
}

fn proto_write_length_delimited(out: &mut Vec<u8>, field: u32, payload: &[u8]) {
    proto_write_tag(out, field, 2);
    proto_write_varint(out, payload.len() as u64);
    out.extend_from_slice(payload);
}

fn render_text(report: &ProfReport) -> String {
    let mut out = String::new();
    out.push_str("function                              calls       self          total\n");
    out.push_str("--------                              -----       ----          -----\n");
    for f in &report.functions {
        out.push_str(&format!(
            "{name:<37} {calls:>5}   {self_ns:>10}   {total_ns:>10}\n",
            name = truncate(&f.name, 37),
            calls = f.calls,
            self_ns = human(f.self_ns),
            total_ns = human(f.total_ns),
        ));
    }
    out.push_str(&format!(
        "\ntotal: {} calls across {} unique fns, wall {}\n",
        report.total_calls,
        report.functions.len(),
        human(report.total_ns),
    ));
    out
}

fn render_folded(report: &ProfReport) -> String {
    let mut out = String::new();
    for (stack, count) in &report.folded {
        out.push_str(&format!("{stack} {count}\n"));
    }
    out
}

fn render_pprof(report: &ProfReport) -> String {
    // pprof's canonical wire form is protobuf. The stage-0 dump is a
    // JSON shape that mirrors the proto field names so downstream
    // tools can convert with `jq` or a one-liner.
    let mut out = String::new();
    out.push_str("{\"sample_type\":[{\"type\":\"cpu\",\"unit\":\"nanoseconds\"}],\"sample\":[");
    let mut first = true;
    for (stack, count) in &report.folded {
        if !first {
            out.push(',');
        }
        first = false;
        let names: Vec<&str> = stack.split(';').collect();
        out.push_str("{\"value\":[");
        out.push_str(&count.to_string());
        out.push_str("],\"location\":[");
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{{\"function\":\"{}\"}}", escape(name)));
        }
        out.push_str("]}");
    }
    out.push_str("],\"function\":[");
    for (i, f) in report.functions.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"calls\":{},\"self_ns\":{},\"total_ns\":{}}}",
            escape(&f.name),
            f.calls,
            f.self_ns,
            f.total_ns,
        ));
    }
    out.push_str("]}");
    out
}

fn truncate(input: &str, max: usize) -> String {
    if input.len() <= max {
        input.to_string()
    } else {
        format!("{}…", &input[..max - 1])
    }
}

fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
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
        format!("{:.2}us", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", ns as f64 / 1_000_000_000.0)
    }
}
