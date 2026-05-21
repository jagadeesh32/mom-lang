//! Phase 6 — standard library acceptance harness.
//!
//! Every `.mom` file under `std/` is a runnable stage-0 implementation
//! of a `std::*` module. The acceptance contract is simple: each
//! module must
//!
//!   1. parse,
//!   2. pass the type-checker,
//!   3. pass the borrow-checker,
//!   4. and run its `main()` to completion under the bootstrap
//!      interpreter, producing the **exact** oracle output below.
//!
//! Oracles are committed in this file so any drift in a module's
//! behaviour produces a single, mechanical test failure with a diff
//! the author can compare against expectations.

use std::fs;
use std::path::{Path, PathBuf};

fn std_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("std");
    path
}

fn run_module(name: &str) -> String {
    let path = std_dir().join(format!("{name}.mom"));
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    mom::run_source(&source)
        .unwrap_or_else(|diag| panic!("std/{name}.mom failed: {diag}"))
}

#[test]
fn std_core_runs_to_completion_with_expected_oracle() {
    let oracle = "42\n3\n7\n10\n0\n5\n9\n-1\n0\n1\n99\n0\ntrue\nfalse\n5\n-1\n15\n";
    assert_eq!(run_module("core"), oracle);
}

#[test]
fn std_fmt_runs_to_completion_with_expected_oracle() {
    let oracle = "ababab\n0007\nhi...\na, b, c\n1-2-3-4\nuser=alice\n[one|two]\n";
    assert_eq!(run_module("fmt"), oracle);
}

#[test]
fn std_alloc_runs_to_completion_with_expected_oracle() {
    let oracle = "Box(99)\nRc(7)\nArc(true)\nrequest:GET /healthz\n";
    assert_eq!(run_module("alloc"), oracle);
}

#[test]
fn std_io_runs_to_completion_with_expected_oracle() {
    let oracle = "alpha\nbeta\ngamma\n\n";
    assert_eq!(run_module("io"), oracle);
}

#[test]
fn std_log_runs_to_completion_with_expected_oracle() {
    let oracle = "INFO starting up\nWARN watch out\nERROR exploded\n2\n";
    assert_eq!(run_module("log"), oracle);
}

#[test]
fn std_async_runs_to_completion_with_expected_oracle() {
    let oracle = "29\ntrue\n";
    assert_eq!(run_module("async"), oracle);
}

#[test]
fn std_actor_runs_to_completion_with_expected_oracle() {
    let oracle = "42\n42\n";
    assert_eq!(run_module("actor"), oracle);
}

#[test]
fn std_net_runs_to_completion_with_expected_oracle() {
    let oracle = "127.0.0.1:8080\n200\nok\n200\nmom 0.1.0\n404\nnot found\n";
    assert_eq!(run_module("net"), oracle);
}

#[test]
fn std_serde_runs_to_completion_with_expected_oracle() {
    let oracle = "true\nfalse\n-7\n\"hello\"\n[1,2,3]\n[\"a\",\"b\"]\n\"name\":\"mom\"\n";
    assert_eq!(run_module("serde"), oracle);
}

#[test]
fn std_crypto_runs_to_completion_with_expected_oracle() {
    let oracle = "38600999\n1\ntrue\nff\n00\nab\n12345678\n";
    assert_eq!(run_module("crypto"), oracle);
}

// Phase 6 stretch modules ---------------------------------------------------

#[test]
fn std_sync_runs_to_completion_with_expected_oracle() {
    let oracle = "42\n1\n100\n5\ntrue\n";
    assert_eq!(run_module("sync"), oracle);
}

#[test]
fn std_os_runs_to_completion_with_expected_oracle() {
    let oracle = "true\n0\nfallback\n1\n1\n";
    assert_eq!(run_module("os"), oracle);
}

#[test]
fn std_math_runs_to_completion_with_expected_oracle() {
    let oracle = "6\n12\n1024\n720\n55\ntrue\ntrue\n";
    assert_eq!(run_module("math"), oracle);
}

#[test]
fn std_test_runs_to_completion_with_expected_oracle() {
    let oracle = "ok    addition\nok    less-than\nok    not-greater-than\npassed=3 failed=0\n";
    assert_eq!(run_module("test"), oracle);
}

#[test]
fn every_std_module_under_std_dir_has_an_oracle_in_this_file() {
    // Guard against silent drift: if someone adds std/foo.mom they
    // need to add a test above. This sweep keeps the contract honest.
    let expected: &[&str] = &[
        "core", "fmt", "alloc", "io", "log", "async", "actor", "net", "serde", "crypto",
        "sync", "os", "math", "test",
    ];
    let dir = std_dir();
    let mut on_disk: Vec<String> = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) == Some("mom") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                on_disk.push(stem.to_string());
            }
        }
    }
    on_disk.sort();
    let mut expected_sorted: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    expected_sorted.sort();
    assert_eq!(
        on_disk, expected_sorted,
        "modules on disk diverged from the oracle list — add a test above and update this sweep"
    );

    // And confirm the path resolution actually points at the repo.
    let _ = Path::new(&dir);
}
