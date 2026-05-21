//! Phase 5 tooling: formatter, linter, manifest, scaffold, test runner,
//! bench harness, profiler, and debugger driver.

use std::fs;
use std::path::PathBuf;

use mom::bench;
use mom::dbg::{DbgServer, Outgoing};
use mom::fmt;
use mom::lint::{lint_program, Category, LintConfig, Severity};
use mom::manifest::{Manifest, Value};
use mom::prof;
use mom::scaffold;
use mom::test_runner;

fn tmpdir(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.push(format!("mom-test-{label}-{pid}-{nanos}"));
    dir
}

#[test]
fn fmt_is_idempotent() {
    let src = "fn main() {\n    print(\"ok\")\n}\n";
    let once = fmt::format_source(src);
    let twice = fmt::format_source(&once);
    assert_eq!(once, twice);
    assert_eq!(once, src);
}

#[test]
fn fmt_reindents_messy_input() {
    let messy = "fn main() {\n  print(\"hi\")\n     print(\"there\")\n}\n";
    let cleaned = fmt::format_source(messy);
    assert_eq!(
        cleaned,
        "fn main() {\n    print(\"hi\")\n    print(\"there\")\n}\n"
    );
}

#[test]
fn fmt_collapses_blank_runs() {
    let messy = "fn a() {}\n\n\n\n\nfn b() {}\n";
    let cleaned = fmt::format_source(messy);
    assert_eq!(cleaned, "fn a() {}\n\nfn b() {}\n");
}

#[test]
fn fmt_normalizes_spacing_via_ast_printer() {
    // The textual re-indenter can't fix bad spacing inside argument
    // lists; the AST-based printer can. After this change, both forms
    // collapse to the canonical layout.
    let messy = "fn add( a:Int,b:Int )->Int{ a+b }\n";
    let cleaned = fmt::format_source(messy);
    assert_eq!(cleaned, "fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n");
}

#[test]
fn fmt_ast_normalizes_struct_literal_spacing() {
    let messy = "struct P{x:Int,y:Int}\nfn main(){let p=P{x:1,y:2}\nprint(p.x)}\n";
    let cleaned = fmt::format_source(messy);
    assert!(cleaned.contains("struct P { x: Int, y: Int }"));
    assert!(cleaned.contains("let p = P { x: 1, y: 2 }"));
}

#[test]
fn fmt_preserves_string_contents() {
    let src = "fn main() { print(\"a,b,c::d\") }\n";
    let out = fmt::format_source(src);
    assert!(out.contains("\"a,b,c::d\""));
}

#[test]
fn lint_default_warns_on_match_after_wildcard() {
    let src = "fn classify(x: Int) -> String {\n    \
                  match x {\n        \
                      _ => \"any\",\n        \
                      1 => \"one\",\n    \
                  }\n}\n";
    let program = mom::parse_source(src).unwrap();
    let report = lint_program(&program, &LintConfig::default());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == Category::Suspicious && f.rule == "unreachable-arm"),
        "expected unreachable-arm finding, got {:?}",
        report.findings
    );
}

#[test]
fn lint_correctness_shadowing_denies_by_default() {
    let src = "fn main() {\n    \
                  let x = 1\n    \
                  let x = 2\n    \
                  print(x)\n\
                  }\n";
    let program = mom::parse_source(src).unwrap();
    let report = lint_program(&program, &LintConfig::default());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.severity == Severity::Deny && f.rule == "shadowing"),
        "expected shadowing deny, got {:?}",
        report.findings
    );
}

#[test]
fn manifest_round_trips_basic_sections() {
    let src = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"1.0\"\n";
    let manifest = Manifest::parse(PathBuf::from("mom.toml"), src).unwrap();
    assert_eq!(
        manifest
            .section("package")
            .and_then(|t| t.get("name"))
            .and_then(|v| v.as_str()),
        Some("demo")
    );
    let rendered = manifest.render();
    let reparsed = Manifest::parse(PathBuf::from("mom.toml"), &rendered).unwrap();
    assert_eq!(
        reparsed.section("dependencies").unwrap().get("foo"),
        Some(&Value::String("1.0".to_string()))
    );
}

#[test]
fn manifest_upsert_replaces_value() {
    let mut manifest = Manifest::parse(
        PathBuf::from("mom.toml"),
        "[dependencies]\nfoo = \"1.0\"\n",
    )
    .unwrap();
    manifest.upsert("dependencies", "foo", Value::String("2.0".into()));
    assert_eq!(
        manifest
            .section("dependencies")
            .and_then(|t| t.get("foo"))
            .and_then(|v| v.as_str()),
        Some("2.0")
    );
}

#[test]
fn scaffold_new_creates_canonical_layout() {
    let dir = tmpdir("scaffold-new");
    let report = scaffold::new_project(&dir).unwrap();
    assert!(dir.join("mom.toml").is_file());
    assert!(dir.join("src/main.mom").is_file());
    assert!(dir.join("tests/smoke_test.mom").is_file());
    assert!(report.files.iter().any(|f| f.ends_with("main.mom")));
    let main = fs::read_to_string(dir.join("src/main.mom")).unwrap();
    assert!(main.contains("fn main"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_runner_discovers_and_runs() {
    let dir = tmpdir("runner");
    scaffold::new_project(&dir).unwrap();
    let report = test_runner::run_all(&dir).unwrap();
    assert!(report.total() >= 1);
    assert!(
        report.all_passed(),
        "scaffolded test should pass: {:?}",
        report.outcomes
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_runner_reports_failures() {
    let dir = tmpdir("runner-fail");
    let tests = dir.join("tests");
    fs::create_dir_all(&tests).unwrap();
    // This program triggers a parse error.
    fs::write(tests.join("broken_test.mom"), "fn main() { let x =\n").unwrap();
    let report = test_runner::run_all(&dir).unwrap();
    assert_eq!(report.total(), 1);
    assert!(!report.all_passed());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn lint_config_reads_manifest_overrides() {
    let manifest = Manifest::parse(
        PathBuf::from("mom.toml"),
        "[lints]\ndefault = \"deny\"\nstyle = \"deny\"\n",
    )
    .unwrap();
    let config = LintConfig::from_manifest(&manifest);
    let src = "fn BadName() {}\n";
    let program = mom::parse_source(src).unwrap();
    let report = lint_program(&program, &config);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.severity == Severity::Deny && f.rule == "naming"),
        "expected style.naming to be denied: {:?}",
        report.findings
    );
}

// ---------------------------------------------------------------------------
// Phase 5.1 — bench harness
// ---------------------------------------------------------------------------

#[test]
fn bench_discovers_benches_dir_and_runs_iterations() {
    let dir = tmpdir("bench-discover");
    let benches = dir.join("benches");
    fs::create_dir_all(&benches).unwrap();
    fs::write(
        benches.join("a_bench.mom"),
        "fn main() { print(\"hi\") }\n",
    )
    .unwrap();

    let mut options = bench::BenchOptions::new();
    options.iterations = 4;
    options.warmup = 1;

    let report = bench::run_all(&dir, &options).unwrap();
    assert_eq!(report.total(), 1);
    assert!(report.all_passed(), "{:?}", report.outcomes);
    let outcome = &report.outcomes[0];
    assert_eq!(outcome.iterations, 4);
    assert_eq!(outcome.samples_ns.len(), 4);
    assert!(outcome.median_ns >= outcome.min_ns);
    assert!(outcome.max_ns >= outcome.median_ns);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn bench_discovers_src_bench_suffix() {
    let dir = tmpdir("bench-src-suffix");
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("hot_bench.mom"), "fn main() {}\n").unwrap();
    fs::write(src.join("regular.mom"), "fn main() {}\n").unwrap();

    let report = bench::run_all(&dir, &bench::BenchOptions::new()).unwrap();
    assert_eq!(report.total(), 1, "{:?}", report.outcomes);
    assert!(
        report.outcomes[0]
            .path
            .file_name()
            .and_then(|s| s.to_str())
            == Some("hot_bench.mom")
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn bench_reports_failure_on_broken_source() {
    let dir = tmpdir("bench-broken");
    let benches = dir.join("benches");
    fs::create_dir_all(&benches).unwrap();
    fs::write(benches.join("oops.mom"), "fn main() { let x =\n").unwrap();

    let mut options = bench::BenchOptions::new();
    options.iterations = 1;
    options.warmup = 0;
    let report = bench::run_all(&dir, &options).unwrap();
    assert_eq!(report.failed(), 1);
    assert!(!report.all_passed());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn bench_discovers_pound_bench_attribute_functions_in_src() {
    let dir = tmpdir("bench-attr");
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("hot.mom"),
        "#[bench]\nfn fast() {}\n\nfn helper() {}\n\nfn main() { helper() }\n",
    )
    .unwrap();

    let mut options = bench::BenchOptions::new();
    options.iterations = 2;
    options.warmup = 0;
    let report = bench::run_all(&dir, &options).unwrap();
    assert_eq!(report.total(), 1, "{:?}", report.outcomes);
    assert_eq!(
        report.outcomes[0].function.as_deref(),
        Some("fast"),
        "{:?}",
        report.outcomes[0]
    );
    assert!(report.all_passed());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn bench_discovers_pound_bench_with_bencher_parameter() {
    let dir = tmpdir("bench-bencher");
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("hot.mom"),
        "#[bench]\nfn squared(b: Bencher) {\n    \
            b.iter(fn() => 42 * 42)\n\
         }\n\nfn main() {}\n",
    )
    .unwrap();

    let mut options = bench::BenchOptions::new();
    options.iterations = 2;
    options.warmup = 0;
    let report = bench::run_all(&dir, &options).unwrap();
    assert_eq!(report.total(), 1, "{:?}", report.outcomes);
    assert_eq!(report.outcomes[0].function.as_deref(), Some("squared"));
    assert!(
        report.all_passed(),
        "bencher-form bench should run: {:?}",
        report.outcomes[0]
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn bench_render_text_and_json_contain_summary_fields() {
    let dir = tmpdir("bench-render");
    let benches = dir.join("benches");
    fs::create_dir_all(&benches).unwrap();
    fs::write(benches.join("x_bench.mom"), "fn main() {}\n").unwrap();
    let mut options = bench::BenchOptions::new();
    options.iterations = 2;
    options.warmup = 0;
    let report = bench::run_all(&dir, &options).unwrap();

    let text = bench::render_text(&report);
    assert!(text.contains("median="));
    assert!(text.contains("stddev="));

    let json = bench::render_json(&report);
    assert!(json.contains("\"median_ns\""));
    assert!(json.contains("\"stddev_ns\""));
    assert!(json.starts_with("{\"benches\":["));
    fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// Phase 5.1 — profiler
// ---------------------------------------------------------------------------

#[test]
fn prof_records_called_functions_and_self_time() {
    let src = "fn helper(x: Int) -> Int { return x + 1 }\n\
               fn main() {\n    \
                   let mut i = 0\n    \
                   while i < 5 {\n        \
                       i = helper(i)\n    \
                   }\n    \
                   print(i)\n\
               }\n";
    let (stdout, report) = prof::profile_source(src).unwrap();
    assert!(stdout.contains("5"));
    let names: Vec<&str> = report.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"main"), "names = {:?}", names);
    assert!(names.contains(&"helper"), "names = {:?}", names);
    let helper = report
        .functions
        .iter()
        .find(|f| f.name == "helper")
        .unwrap();
    assert_eq!(helper.calls, 5);
}

#[test]
fn prof_folded_lines_contain_call_stacks() {
    let src = "fn helper() {}\nfn main() { helper() }\n";
    let (_out, report) = prof::profile_source(src).unwrap();
    let folded = prof::render(&report, prof::ProfFormat::Folded);
    assert!(folded.contains("main;helper"));
}

#[test]
fn prof_pprof_emits_function_table() {
    let src = "fn main() {}\n";
    let (_out, report) = prof::profile_source(src).unwrap();
    let pprof = prof::render(&report, prof::ProfFormat::Pprof);
    assert!(pprof.contains("\"sample_type\""));
    assert!(pprof.contains("\"function\""));
}

#[test]
fn prof_format_parser_round_trips_known_modes() {
    assert_eq!(prof::ProfFormat::parse("text"), Some(prof::ProfFormat::Text));
    assert_eq!(
        prof::ProfFormat::parse("folded"),
        Some(prof::ProfFormat::Folded)
    );
    assert_eq!(
        prof::ProfFormat::parse("pprof"),
        Some(prof::ProfFormat::Pprof)
    );
    assert_eq!(
        prof::ProfFormat::parse("pprof-proto"),
        Some(prof::ProfFormat::PprofProto)
    );
    assert_eq!(
        prof::ProfFormat::parse("proto"),
        Some(prof::ProfFormat::PprofProto)
    );
    assert_eq!(prof::ProfFormat::parse("garbage"), None);
}

#[test]
fn prof_pprof_proto_bytes_include_function_names_in_string_table() {
    let src = "fn helper() {}\nfn main() { helper() }\n";
    let (_out, report) = prof::profile_source(src).unwrap();
    let bytes = prof::render_pprof_proto_bytes(&report);
    // string_table entries land as length-delimited field 6 (tag = (6<<3)|2 = 0x32).
    // Confirm both "main" and "helper" appear somewhere in the byte stream.
    assert!(
        bytes
            .windows(b"main".len())
            .any(|w| w == b"main"),
        "pprof.proto missing 'main' name"
    );
    assert!(
        bytes
            .windows(b"helper".len())
            .any(|w| w == b"helper"),
        "pprof.proto missing 'helper' name"
    );
    // The first field tag must be 0x0a (sample_type: field 1, wire-type 2).
    assert_eq!(bytes[0], 0x0a, "pprof.proto should start with sample_type tag");
}

// ---------------------------------------------------------------------------
// Phase 5.1 — debugger driver (DAP)
// ---------------------------------------------------------------------------

fn dbg_join(messages: &[Outgoing]) -> String {
    messages
        .iter()
        .map(|o| match o {
            Outgoing::Send(s) => s.clone(),
            Outgoing::Exit => String::from("<exit>"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn dbg_initialize_returns_capabilities_and_event() {
    let mut server = DbgServer::new();
    let out = server.handle("{\"seq\":1,\"command\":\"initialize\",\"arguments\":{}}");
    let joined = dbg_join(&out);
    assert!(joined.contains("\"success\":true"));
    assert!(joined.contains("supportsConfigurationDoneRequest"));
    assert!(joined.contains("\"event\":\"initialized\""));
}

#[test]
fn dbg_launch_executes_and_terminates() {
    let dir = tmpdir("dbg-launch");
    fs::create_dir_all(&dir).unwrap();
    let program = dir.join("hello.mom");
    fs::write(&program, "fn main() { print(\"hi\") }\n").unwrap();

    let mut server = DbgServer::new();
    let _ = server.handle("{\"seq\":1,\"command\":\"initialize\",\"arguments\":{}}");
    let launch = format!(
        "{{\"seq\":2,\"command\":\"launch\",\"arguments\":{{\"program\":\"{}\"}}}}",
        program.display()
    );
    let out = server.handle(&launch);
    let joined = dbg_join(&out);
    assert!(joined.contains("\"command\":\"launch\""));
    assert!(joined.contains("\"event\":\"terminated\""));
    assert!(joined.contains("\"event\":\"exited\""));
    assert!(joined.contains("\"category\":\"stdout\""));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn dbg_disconnect_exits_after_response() {
    let mut server = DbgServer::new();
    let _ = server.handle("{\"seq\":1,\"command\":\"initialize\",\"arguments\":{}}");
    let out = server.handle("{\"seq\":2,\"command\":\"disconnect\"}");
    assert!(matches!(out.last(), Some(Outgoing::Exit)));
    let joined = dbg_join(&out[..out.len() - 1]);
    assert!(joined.contains("\"command\":\"disconnect\""));
    assert!(joined.contains("\"success\":true"));
}
