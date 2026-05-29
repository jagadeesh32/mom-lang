//! Phase 1 build driver.
//!
//! Pipeline:
//!     source.mom → lex → parse → typecheck → codegen → C source
//!                → cc → object file → linker → native binary
//!
//! Output binaries are cached in `target/mom-cache/<hash>/` keyed by:
//!     - the source content
//!     - the compiler version
//!     - the running compiler binary's identity (so a rebuilt compiler
//!       with new codegen invalidates stale cache entries)
//!     - the chosen C compiler name
//!     - opt-level
//!
//! A second build of an unchanged source skips `cc` and `ld` entirely.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::codegen::Codegen;
use crate::diagnostic::{Diagnostic, LangResult};
use crate::{parse_source, typechecker::TypeChecker};

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub optimize: bool,
    pub keep_intermediate: bool,
    pub cache_dir: PathBuf,
    pub project_root: PathBuf,
    pub c_compiler: String,
}

impl BuildOptions {
    pub fn new(source_path: PathBuf, output_path: PathBuf) -> Self {
        let project_root = detect_project_root(&source_path);
        let cache_dir = project_root.join("target").join("mom-cache");
        let c_compiler = env::var("CC").unwrap_or_else(|_| "cc".to_string());
        Self {
            source_path,
            output_path,
            optimize: false,
            keep_intermediate: false,
            cache_dir,
            project_root,
            c_compiler,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildReport {
    pub output: PathBuf,
    pub from_cache: bool,
    pub c_source_path: PathBuf,
}

pub fn build(options: &BuildOptions) -> LangResult<BuildReport> {
    let source = fs::read_to_string(&options.source_path).map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to read source '{}': {err}",
            options.source_path.display()
        ))
    })?;

    let runtime_dir = locate_runtime_dir(&options.project_root)?;
    let runtime_c = runtime_dir.join("runtime.c");
    let runtime_h_dir = runtime_dir.clone();

    let runtime_c_bytes = fs::read(&runtime_c).map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to read runtime '{}': {err}",
            runtime_c.display()
        ))
    })?;

    let key = build_key(
        &source,
        &runtime_c_bytes,
        &options.c_compiler,
        options.optimize,
        &compiler_identity(),
    );

    let cache_entry = options.cache_dir.join(&key);
    fs::create_dir_all(&cache_entry).map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to create cache dir '{}': {err}",
            cache_entry.display()
        ))
    })?;
    let cached_binary = cache_entry.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let cached_c = cache_entry.join("program.c");

    if cached_binary.exists() {
        copy_file(&cached_binary, &options.output_path)?;
        return Ok(BuildReport {
            output: options.output_path.clone(),
            from_cache: true,
            c_source_path: cached_c,
        });
    }

    // Frontend
    let program = parse_source(&source)?;
    let _ = TypeChecker::new().check_program(&program)?;

    // Codegen → C source
    let codegen_output = Codegen::new().emit_program(&program)?;
    fs::write(&cached_c, &codegen_output.c_source).map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to write generated C source '{}': {err}",
            cached_c.display()
        ))
    })?;

    // Link via cc
    let mut cmd = Command::new(&options.c_compiler);
    cmd.arg("-std=c99")
        .arg("-Wall")
        .arg("-Wextra")
        .arg(if options.optimize { "-O2" } else { "-O0" })
        .arg("-I")
        .arg(&runtime_h_dir)
        .arg(&cached_c)
        .arg(&runtime_c)
        .arg("-o")
        .arg(&cached_binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to invoke C compiler '{}': {err}",
            options.c_compiler
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::at_start(format!(
            "C compiler failed:\n{stderr}\n--- generated C ({}) ---",
            cached_c.display()
        )));
    }

    copy_file(&cached_binary, &options.output_path)?;

    Ok(BuildReport {
        output: options.output_path.clone(),
        from_cache: false,
        c_source_path: cached_c,
    })
}

fn build_key(
    source: &str,
    runtime: &[u8],
    cc: &str,
    optimize: bool,
    compiler_identity: &str,
) -> String {
    let mut hasher = Hasher64::new();
    hasher.write(b"mom-build/v2");
    hasher.write(env!("CARGO_PKG_VERSION").as_bytes());
    // Identity of the running compiler binary. Without this a rebuilt
    // compiler (new codegen, same source) would serve a stale binary
    // from the cache. See `compiler_identity`.
    hasher.write(compiler_identity.as_bytes());
    hasher.write(cc.as_bytes());
    hasher.write(if optimize { b"O2" } else { b"O0" });
    hasher.write(source.as_bytes());
    hasher.write(runtime);
    format!("{:016x}", hasher.finish())
}

/// A string that uniquely identifies the running compiler binary so that
/// rebuilding the compiler invalidates stale cache entries produced by the
/// old codegen.
///
/// Preferred: the current executable's file length + modified time, which
/// changes whenever the binary is recompiled but stays stable for repeated
/// builds with the same binary (so the cache still hits). If `current_exe()`
/// or its metadata is unavailable, fall back to the crate version.
fn compiler_identity() -> String {
    let fallback = || format!("ver:{}", env!("CARGO_PKG_VERSION"));
    let exe = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return fallback(),
    };
    let meta = match fs::metadata(&exe) {
        Ok(m) => m,
        Err(_) => return fallback(),
    };
    let len = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("exe:{len}:{mtime}")
}

fn copy_file(src: &Path, dst: &Path) -> LangResult<()> {
    if let Some(parent) = dst.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                Diagnostic::at_start(format!(
                    "failed to create output dir '{}': {err}",
                    parent.display()
                ))
            })?;
        }
    }
    fs::copy(src, dst).map_err(|err| {
        Diagnostic::at_start(format!(
            "failed to copy '{}' → '{}': {err}",
            src.display(),
            dst.display()
        ))
    })?;
    Ok(())
}

fn detect_project_root(source_path: &Path) -> PathBuf {
    let mut candidate = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    loop {
        if candidate.join("Cargo.toml").exists() || candidate.join("mom.toml").exists() {
            return candidate;
        }
        match candidate.parent() {
            Some(parent) => candidate = parent.to_path_buf(),
            None => break,
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn locate_runtime_dir(project_root: &Path) -> LangResult<PathBuf> {
    let candidate = project_root.join("compiler");
    if candidate.join("runtime.c").exists() {
        return Ok(candidate);
    }
    // Fall back to a sibling of the Cargo manifest (works for cargo run).
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fallback = manifest.join("compiler");
    if fallback.join("runtime.c").exists() {
        return Ok(fallback);
    }
    Err(Diagnostic::at_start(format!(
        "could not locate mom runtime; searched '{}' and '{}'",
        candidate.display(),
        fallback.display()
    )))
}

/* ------------------------------------------------------------------ */
/* Tiny FNV-1a 64-bit hasher used for content-addressed cache keys.   */
/* Cryptographic strength is not required; collision avoidance with   */
/* 64 bits is more than enough for build-cache use.                   */
/* ------------------------------------------------------------------ */

struct Hasher64 {
    state: u64,
}

impl Hasher64 {
    fn new() -> Self {
        Self {
            state: 0xcbf29ce484222325,
        }
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.state ^= *b as u64;
            self.state = self.state.wrapping_mul(0x100000001b3);
        }
    }
    fn finish(self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_changes_with_compiler_identity() {
        // Same source/runtime/cc/opt, different compiler identity → the
        // cache key must differ so a rebuilt compiler can't serve a stale
        // binary. Two synthetic identities stand in for "old" vs "new"
        // compiler binaries.
        let src = "fn main() { print(1) }";
        let runtime = b"runtime-bytes";
        let old = build_key(src, runtime, "cc", false, "exe:100:111");
        let new = build_key(src, runtime, "cc", false, "exe:200:222");
        assert_ne!(old, new, "key must change when compiler identity changes");

        // ...and stay stable for the same identity, so repeated builds with
        // the SAME compiler still hit the cache.
        let again = build_key(src, runtime, "cc", false, "exe:100:111");
        assert_eq!(old, again, "key must be deterministic for one compiler");
    }

    #[test]
    fn compiler_identity_is_deterministic() {
        // Whatever strategy is used, calling it twice in one process must
        // yield the same value (same binary on disk).
        assert_eq!(compiler_identity(), compiler_identity());
    }
}
