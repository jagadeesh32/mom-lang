use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use mom::build::{build, BuildOptions};
use mom::diagnostic::{Diagnostic, LangResult};
use mom::lint::{Category, LintConfig, Severity};
use mom::manifest::Manifest;

fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            if let Err(diagnostic) = run_cli() {
                eprintln!("error: {diagnostic}");
                process::exit(1);
            }
        })
        .expect("failed to spawn main thread")
        .join()
        .expect("main thread panicked");
}

fn run_cli() -> LangResult<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "tokens" => {
            let source = read_path(args.next())?;
            for token in mom::lex_source(&source)? {
                println!(
                    "{:?} @ {}:{}",
                    token.kind, token.span.line, token.span.column
                );
            }
        }
        "ast" => {
            let source = read_path(args.next())?;
            let program = mom::parse_source(&source)?;
            println!("{program:#?}");
        }
        "check" => {
            let source = read_path(args.next())?;
            let report = mom::check_source(&source)?;
            println!(
                "ok: {} function(s), {} known type(s)",
                report.functions.len(),
                report.types.len()
            );
        }
        "run" => {
            let source = read_path(args.next())?;
            let output = mom::run_source(&source)?;
            print!("{output}");
        }
        "build" => {
            run_build(args, false)?;
        }
        "build-run" => {
            run_build(args, true)?;
        }
        "emit-c" => {
            let source_path = args
                .next()
                .ok_or_else(|| Diagnostic::at_start("expected a source file path for 'emit-c'"))?;
            let source = fs::read_to_string(&source_path).map_err(|err| {
                Diagnostic::at_start(format!("failed to read '{source_path}': {err}"))
            })?;
            let program = mom::parse_source(&source)?;
            mom::typechecker::TypeChecker::new().check_program(&program)?;
            let output = mom::codegen::Codegen::new().emit_program(&program)?;
            print!("{}", output.c_source);
        }
        "fmt" => run_fmt(args)?,
        "lint" => run_lint(args)?,
        "doc" => run_doc(args)?,
        "test" => run_test(args)?,
        "bench" => run_bench(args)?,
        "new" => run_new(args)?,
        "init" => run_init(args)?,
        "pkg" => run_pkg(args)?,
        "lsp" => {
            mom::lsp::run().map_err(|err| Diagnostic::at_start(format!("lsp i/o error: {err}")))?
        }
        "dbg" => run_dbg(args)?,
        "prof" => run_prof(args)?,
        "selfhost" => run_selfhost(args)?,
        "version" | "--version" | "-V" => {
            println!("mom {}", env!("CARGO_PKG_VERSION"));
        }
        "--help" | "-h" | "help" => print_usage(),
        other => {
            return Err(Diagnostic::at_start(format!(
                "unknown command '{other}'. Try 'mom help'"
            )));
        }
    }

    Ok(())
}

fn run_build<I: Iterator<Item = String>>(mut args: I, execute: bool) -> LangResult<()> {
    let mut source_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut optimize = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output_path =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        Diagnostic::at_start("expected a path after '-o'")
                    })?));
            }
            "--release" | "-O" => {
                optimize = true;
            }
            other if other.starts_with('-') => {
                return Err(Diagnostic::at_start(format!(
                    "unknown build flag '{other}'"
                )));
            }
            other => {
                if source_path.is_some() {
                    return Err(Diagnostic::at_start(format!(
                        "unexpected positional argument '{other}'"
                    )));
                }
                source_path = Some(PathBuf::from(other));
            }
        }
    }

    let source_path =
        source_path.ok_or_else(|| Diagnostic::at_start("expected a source file path"))?;
    let default_output = derive_output_path(&source_path);
    let output_path = output_path.unwrap_or(default_output);

    let mut options = BuildOptions::new(source_path.clone(), output_path.clone());
    options.optimize = optimize;

    let report = build(&options)?;
    eprintln!(
        "built {} ({})",
        report.output.display(),
        if report.from_cache { "cached" } else { "fresh" }
    );

    if execute {
        let status = Command::new(&output_path).status().map_err(|err| {
            Diagnostic::at_start(format!(
                "failed to execute '{}': {err}",
                output_path.display()
            ))
        })?;
        if !status.success() {
            process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}

fn run_fmt<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let mut check = false;
    let mut paths: Vec<PathBuf> = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            other if other.starts_with('-') => {
                return Err(Diagnostic::at_start(format!("unknown fmt flag '{other}'")));
            }
            other => paths.push(PathBuf::from(other)),
        }
    }

    if paths.is_empty() {
        return Err(Diagnostic::at_start(
            "expected at least one file path for 'fmt'",
        ));
    }

    let mut unformatted: Vec<PathBuf> = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).map_err(|err| {
            Diagnostic::at_start(format!("failed to read '{}': {err}", path.display()))
        })?;
        let formatted = mom::fmt::format_source(&source);
        if formatted == source {
            continue;
        }
        if check {
            unformatted.push(path);
            continue;
        }
        fs::write(&path, formatted).map_err(|err| {
            Diagnostic::at_start(format!("failed to write '{}': {err}", path.display()))
        })?;
        eprintln!("formatted {}", path.display());
    }

    if check && !unformatted.is_empty() {
        for path in &unformatted {
            eprintln!("would reformat {}", path.display());
        }
        process::exit(1);
    }

    Ok(())
}

fn run_lint<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let path = args
        .next()
        .ok_or_else(|| Diagnostic::at_start("expected a source file path for 'lint'"))?;
    let source = fs::read_to_string(&path)
        .map_err(|err| Diagnostic::at_start(format!("failed to read '{path}': {err}")))?;
    let program = mom::parse_source(&source)?;

    let config = match Manifest::find(Path::new(&path)) {
        Some(manifest_path) => {
            let manifest = Manifest::load(manifest_path)?;
            LintConfig::from_manifest(&manifest)
        }
        None => LintConfig::default(),
    };

    let report = mom::lint::lint_program(&program, &config);
    if report.findings.is_empty() {
        println!("no lint findings");
        return Ok(());
    }

    for finding in &report.findings {
        println!(
            "{sev}: {cat}.{rule}: {msg} at {file}:{line}:{col}",
            sev = finding.severity.label(),
            cat = finding.category.slug(),
            rule = finding.rule,
            msg = finding.message,
            file = path,
            line = finding.span.line,
            col = finding.span.column,
        );
    }

    let deny = report.deny_count();
    let warn = report.warn_count();
    eprintln!("lint summary: {deny} denied, {warn} warned");
    if deny > 0 {
        let _ = Category::Correctness;
        process::exit(1);
    }
    Ok(())
}

fn run_doc<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let path = args
        .next()
        .ok_or_else(|| Diagnostic::at_start("expected a source file path for 'doc'"))?;
    let source = fs::read_to_string(&path)
        .map_err(|err| Diagnostic::at_start(format!("failed to read '{path}': {err}")))?;
    let program = mom::parse_source(&source)?;
    let crate_name = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("crate")
        .to_string();
    let docs = mom::doc::generate_docs(&source, &program, &crate_name)?;
    print!("{docs}");
    Ok(())
}

fn run_test<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let report = mom::test_runner::run_all(&root)?;

    if report.total() == 0 {
        println!("no tests discovered under {}", root.display());
        return Ok(());
    }

    for outcome in &report.outcomes {
        let mark = if outcome.passed { "ok" } else { "FAIL" };
        println!(
            "{mark} {} ({}ms)",
            outcome.path.display(),
            outcome.elapsed_ms
        );
        if !outcome.passed {
            println!("    {}", outcome.message);
        }
    }
    println!(
        "summary: {} passed, {} failed",
        report.passed(),
        report.failed()
    );
    if !report.all_passed() {
        process::exit(1);
    }
    Ok(())
}

fn run_bench<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let mut root: Option<PathBuf> = None;
    let mut options = mom::bench::BenchOptions::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iter" | "--iterations" => {
                let value = args
                    .next()
                    .ok_or_else(|| Diagnostic::at_start("expected a number after '--iter'"))?;
                options.iterations = value.parse::<usize>().map_err(|_| {
                    Diagnostic::at_start(format!("invalid iteration count '{value}'"))
                })?;
            }
            "--warmup" => {
                let value = args
                    .next()
                    .ok_or_else(|| Diagnostic::at_start("expected a number after '--warmup'"))?;
                options.warmup = value
                    .parse::<usize>()
                    .map_err(|_| Diagnostic::at_start(format!("invalid warmup count '{value}'")))?;
            }
            "--json" => options.json = true,
            other if other.starts_with('-') => {
                return Err(Diagnostic::at_start(format!(
                    "unknown bench flag '{other}'"
                )));
            }
            other => {
                if root.is_some() {
                    return Err(Diagnostic::at_start(format!(
                        "unexpected positional argument '{other}'"
                    )));
                }
                root = Some(PathBuf::from(other));
            }
        }
    }

    let root = root.unwrap_or_else(|| PathBuf::from("."));
    let report = mom::bench::run_all(&root, &options)?;

    if report.total() == 0 {
        eprintln!(
            "no benches discovered under {} (expected benches/**/*.mom or src/**/*_bench.mom)",
            root.display()
        );
        return Ok(());
    }

    if options.json {
        print!("{}", mom::bench::render_json(&report));
    } else {
        print!("{}", mom::bench::render_text(&report));
        eprintln!(
            "summary: {} passed, {} failed",
            report.passed(),
            report.failed()
        );
    }
    if !report.all_passed() {
        process::exit(1);
    }
    Ok(())
}

fn run_new<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let target = args
        .next()
        .ok_or_else(|| Diagnostic::at_start("expected a directory name for 'new'"))?;
    let report = mom::scaffold::new_project(Path::new(&target))?;
    eprintln!("created project {}", report.root.display());
    for file in report.files {
        eprintln!("  + {}", file.display());
    }
    Ok(())
}

fn run_init<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let target = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let report = mom::scaffold::init_project(&target)?;
    eprintln!("initialized {}", report.root.display());
    for file in report.files {
        eprintln!("  + {}", file.display());
    }
    Ok(())
}

fn run_pkg<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let action = args.next().ok_or_else(|| {
        Diagnostic::at_start("expected a 'pkg' subcommand: list/add/remove/audit")
    })?;
    let manifest_path = Manifest::find(Path::new(".")).ok_or_else(|| {
        Diagnostic::at_start("no mom.toml found in the current directory or any parent")
    })?;
    let mut manifest = Manifest::load(manifest_path)?;

    match action.as_str() {
        "list" => {
            let deps = mom::pkg::list(&manifest);
            if deps.is_empty() {
                println!("(no dependencies)");
            } else {
                for (name, version) in deps {
                    println!("{name} = \"{version}\"");
                }
            }
        }
        "add" => {
            let name = args
                .next()
                .ok_or_else(|| Diagnostic::at_start("pkg add: expected a package name"))?;
            let version = args.next().unwrap_or_else(|| "*".to_string());
            mom::pkg::add(&mut manifest, &name, &version)?;
            eprintln!("added {name} = \"{version}\"");
        }
        "remove" => {
            let name = args
                .next()
                .ok_or_else(|| Diagnostic::at_start("pkg remove: expected a package name"))?;
            if mom::pkg::remove(&mut manifest, &name)? {
                eprintln!("removed {name}");
            } else {
                eprintln!("no such dependency: {name}");
            }
        }
        "audit" => {
            let findings = mom::pkg::audit(&manifest);
            if findings.is_empty() {
                println!("ok: no audit findings");
            } else {
                for finding in &findings {
                    println!("{}: {}", finding.name, finding.issue);
                }
                process::exit(1);
            }
        }
        other => {
            return Err(Diagnostic::at_start(format!(
                "unknown pkg subcommand '{other}'. Try list/add/remove/audit"
            )));
        }
    }

    let _ = Severity::Warn;
    Ok(())
}

fn run_dbg<I: Iterator<Item = String>>(_args: I) -> LangResult<()> {
    mom::dbg::run().map_err(|err| Diagnostic::at_start(format!("dbg i/o error: {err}")))
}

fn run_prof<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let mut source_path: Option<PathBuf> = None;
    let mut format = mom::prof::ProfFormat::Folded;
    let mut output: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--format" => {
                let value = args.next().ok_or_else(|| {
                    Diagnostic::at_start("expected text|folded|pprof after '--format'")
                })?;
                format = mom::prof::ProfFormat::parse(&value).ok_or_else(|| {
                    Diagnostic::at_start(format!(
                        "invalid --format '{value}'. Use text, folded, or pprof"
                    ))
                })?;
            }
            "-o" | "--output" => {
                output =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        Diagnostic::at_start("expected a path after '-o'")
                    })?));
            }
            other if other.starts_with('-') => {
                return Err(Diagnostic::at_start(format!("unknown prof flag '{other}'")));
            }
            other => {
                if source_path.is_some() {
                    return Err(Diagnostic::at_start(format!(
                        "unexpected positional argument '{other}'"
                    )));
                }
                source_path = Some(PathBuf::from(other));
            }
        }
    }

    let source_path = source_path
        .ok_or_else(|| Diagnostic::at_start("expected a source file path for 'prof'"))?;
    let source = fs::read_to_string(&source_path).map_err(|err| {
        Diagnostic::at_start(format!("failed to read '{}': {err}", source_path.display()))
    })?;
    let (stdout, report) = mom::prof::profile_source(&source)?;
    let rendered = mom::prof::render(&report, format);

    if !stdout.is_empty() {
        eprint!("{stdout}");
    }

    if let Some(path) = output {
        fs::write(&path, &rendered).map_err(|err| {
            Diagnostic::at_start(format!("failed to write '{}': {err}", path.display()))
        })?;
        eprintln!("wrote profile to {}", path.display());
    } else {
        print!("{rendered}");
    }
    Ok(())
}

/// `mom selfhost <file.mom> [-o OUT] [--run]` — drive the stage-1
/// mom-in-mom compiler end-to-end:
///   1. interpret `compiler/src/main.mom` with MOM_INPUT/MOM_OUTPUT set
///   2. link the emitted C with `compiler/runtime.c` via `$CC` (default `cc`)
///   3. with `--run`, immediately execute the resulting binary
fn run_selfhost<I: Iterator<Item = String>>(mut args: I) -> LangResult<()> {
    let mut source_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut execute = false;
    let mut keep_c: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output_path =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        Diagnostic::at_start("expected a path after '-o'")
                    })?));
            }
            "--emit-c" => {
                keep_c = Some(PathBuf::from(args.next().ok_or_else(|| {
                    Diagnostic::at_start("expected a path after '--emit-c'")
                })?));
            }
            "--run" => execute = true,
            other if other.starts_with('-') => {
                return Err(Diagnostic::at_start(format!(
                    "unknown selfhost flag '{other}'"
                )));
            }
            other => {
                if source_path.is_some() {
                    return Err(Diagnostic::at_start(format!(
                        "unexpected positional argument '{other}'"
                    )));
                }
                source_path = Some(PathBuf::from(other));
            }
        }
    }

    let source_path = source_path
        .ok_or_else(|| Diagnostic::at_start("expected a source .mom file for 'selfhost'"))?;

    // Repo root is the directory containing `compiler/src/main.mom`.
    let exe = std::env::current_exe()
        .map_err(|err| Diagnostic::at_start(format!("cannot resolve current executable: {err}")))?;
    let from_exe: Option<PathBuf> = exe
        .ancestors()
        .find(|p| p.join("compiler/src/main.mom").exists())
        .map(|p| p.to_path_buf());
    let from_cwd: Option<PathBuf> = std::env::current_dir().ok().and_then(|cwd| {
        cwd.ancestors()
            .find(|p| p.join("compiler/src/main.mom").exists())
            .map(|p| p.to_path_buf())
    });
    let repo_root = from_exe
        .or(from_cwd)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let compiler_src = repo_root.join("compiler/src/main.mom");
    let runtime_c = repo_root.join("compiler/runtime.c");
    let runtime_include = repo_root.join("compiler");
    if !compiler_src.exists() {
        return Err(Diagnostic::at_start(format!(
            "stage-1 compiler not found at {} (run from the mom repo root)",
            compiler_src.display()
        )));
    }
    if !runtime_c.exists() {
        return Err(Diagnostic::at_start(format!(
            "stage-1 runtime not found at {}",
            runtime_c.display()
        )));
    }

    let work_dir = repo_root.join("target/selfhost");
    fs::create_dir_all(&work_dir)
        .map_err(|err| Diagnostic::at_start(format!("cannot create work dir: {err}")))?;

    let stem = source_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".into());
    let c_out = keep_c.unwrap_or_else(|| work_dir.join(format!("{stem}.c")));
    let bin_out = output_path.unwrap_or_else(|| work_dir.join(&stem));

    // Step 1: run stage-1 compiler with env vars
    let exe_str = exe.as_os_str().to_owned();
    let status = Command::new(&exe_str)
        .arg("run")
        .arg(&compiler_src)
        .env("MOM_INPUT", &source_path)
        .env("MOM_OUTPUT", &c_out)
        .status()
        .map_err(|err| Diagnostic::at_start(format!("failed to spawn stage-1: {err}")))?;
    if !status.success() {
        return Err(Diagnostic::at_start(
            "stage-1 compiler exited non-zero (input outside the stage-1 subset?)",
        ));
    }
    eprintln!("emitted {}", c_out.display());

    // Step 2: cc the generated C with the runtime
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let status = Command::new(&cc)
        .arg("-std=c99")
        .arg("-O2")
        .arg("-I")
        .arg(&runtime_include)
        .arg(&c_out)
        .arg(&runtime_c)
        .arg("-o")
        .arg(&bin_out)
        .status()
        .map_err(|err| Diagnostic::at_start(format!("failed to spawn cc ('{cc}'): {err}")))?;
    if !status.success() {
        return Err(Diagnostic::at_start("cc failed on stage-1 output"));
    }
    eprintln!("linked {}", bin_out.display());

    if execute {
        let status = Command::new(&bin_out).status().map_err(|err| {
            Diagnostic::at_start(format!("failed to execute '{}': {err}", bin_out.display()))
        })?;
        if !status.success() {
            process::exit(status.code().unwrap_or(1));
        }
    }
    Ok(())
}

fn derive_output_path(source_path: &PathBuf) -> PathBuf {
    let stem = source_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "a".to_string());
    let project_dir = source_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut candidate = project_dir
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() || p.join("mom.toml").exists())
        .map(|p| p.to_path_buf())
        .unwrap_or(project_dir);
    candidate.push("target");
    candidate.push("mom-bin");
    candidate.push(stem);
    candidate
}

fn read_path(path: Option<String>) -> LangResult<String> {
    let path = path.ok_or_else(|| Diagnostic::at_start("expected a source file path"))?;
    fs::read_to_string(&path)
        .map_err(|err| Diagnostic::at_start(format!("failed to read '{path}': {err}")))
}

fn print_usage() {
    println!(
        "mom {version} - a modern, safe, fast systems programming language\n\n\
Usage:\n  \
  mom tokens     <file.mom>            Lex a source file and print tokens\n  \
  mom ast        <file.mom>            Parse a source file and print the AST\n  \
  mom check      <file.mom>            Type-check a source file\n  \
  mom run        <file.mom>            Run via the bootstrap interpreter\n  \
  mom build      <file.mom> [-o OUT] [--release]\n                                       Compile to a native binary\n  \
  mom build-run  <file.mom> [-o OUT] [--release]\n                                       Compile and immediately execute\n  \
  mom emit-c     <file.mom>            Show the generated C source\n\n\
Phase 5 tooling:\n  \
  mom fmt        <file.mom> [--check]  Format source (in-place; --check exits 1 if dirty)\n  \
  mom lint       <file.mom>            Run the linter against mom.toml [lints] config\n  \
  mom doc        <file.mom>            Emit Markdown API docs for pub items\n  \
  mom test       [dir]                 Discover and run *.mom tests under <dir>\n  \
  mom bench      [dir] [--iter N] [--warmup N] [--json]\n                                       Run benches/**/*.mom + src/**/*_bench.mom\n  \
  mom new        <dir>                 Scaffold a new mom project\n  \
  mom init       [dir]                 Scaffold in an existing directory\n  \
  mom pkg        list|add|remove|audit Manage dependencies in mom.toml\n  \
  mom lsp                              Run the LSP server on stdio\n  \
  mom dbg                              Run the DAP debugger driver on stdio\n  \
  mom prof       <file.mom> [--format text|folded|pprof] [-o OUT]\n                                       Profile a program via the interpreter\n\n\
Self-host bootstrap:\n  \
  mom selfhost   <file.mom> [-o OUT] [--run] [--emit-c PATH]\n                                       Drive the mom-in-mom stage-1 compiler:\n                                       lex → parse → emit C → cc → (optionally) run\n\n\
  mom version                          Print compiler version\n  \
  mom help                             Show this help\n\n\
Phase 1 native codegen supports the Int/Bool/Unit subset (functions,\n\
arithmetic, comparisons, if/while/for-in/return, print, recursion).\n\
Stage-1.2 mom-in-mom (compiler/src/main.mom) additionally compiles:\n\
strings, +/-/*/`/`/`%`, str()/len()/println, and for-in-range loops.\n\
See docs/plan.md for the full roadmap.",
        version = env!("CARGO_PKG_VERSION"),
    );
}
