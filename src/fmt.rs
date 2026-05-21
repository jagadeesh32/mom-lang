//! Phase 5 formatter — `mom fmt`.
//!
//! Operates on source text directly. The rules are intentionally narrow
//! so the formatter is deterministic, idempotent, and never reshapes
//! authored layout in surprising ways. The native `mom` toolchain in
//! Phase 6+ will swap this for a full AST-based printer; this is the
//! bootstrap implementation.
//!
//! Rules:
//!   1. Re-indent every logical line to `<depth> * 4` spaces, where
//!      `depth` is the running brace nesting at the start of the line.
//!      Lines that close a brace de-dent first so `}` aligns with its
//!      opener.
//!   2. Trim trailing whitespace.
//!   3. Collapse 3+ consecutive blank lines into a single blank line.
//!   4. Normalize `,X` → `, X` and `:X` → `: X` for non-space `X`
//!      (skipped inside strings, char escapes, and comments).
//!   5. Ensure exactly one trailing newline.
//!
//! Strings (`"…"`), line comments (`// …`), and block comments
//! (`/* … */`) are passed through verbatim — their interior is never
//! touched.

const INDENT: &str = "    ";

pub fn format_source(source: &str) -> String {
    // Phase 5.1 native parity: try the AST-based pretty-printer first.
    // Safety check: parse the printer's output and re-format. If both
    // passes produce identical text, the printer is stable for this
    // source and we keep the result. Anything else (parse failure on
    // either pass, or instability) falls back to the deterministic text
    // re-indenter so a parse-broken source is never mangled.
    if let Ok(program) = crate::parse_source(source) {
        let pretty = crate::fmt_ast::format_program(&program);
        if let Ok(reparsed) = crate::parse_source(&pretty) {
            let pretty2 = crate::fmt_ast::format_program(&reparsed);
            if pretty == pretty2 {
                return pretty;
            }
        }
    }
    let normalized = normalize_lines(source);
    let reindented = reindent(&normalized);
    collapse_blank_runs(&reindented)
}

pub fn is_formatted(source: &str) -> bool {
    format_source(source) == source
}

fn normalize_lines(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed_end = trim_trailing_ws(line);
        let spaced = normalize_separators(trimmed_end);
        out.push_str(&spaced);
        out.push('\n');
    }
    out
}

fn trim_trailing_ws(line: &str) -> &str {
    let mut end = line.len();
    let bytes = line.as_bytes();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    &line[..end]
}

/// Insert a single space after `,` and `:` when the following byte is
/// non-space, but skip while inside a string or comment.
fn normalize_separators(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut state = ScanState::Code;
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut prev: u8 = 0;

    while i < bytes.len() {
        let b = bytes[i];
        match state {
            ScanState::Code => {
                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    state = ScanState::LineComment;
                    out.push('/');
                    out.push('/');
                    prev = b'/';
                    i += 2;
                    continue;
                }
                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    state = ScanState::BlockComment;
                    out.push('/');
                    out.push('*');
                    prev = b'*';
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    state = ScanState::StringLit;
                    out.push('"');
                    prev = b'"';
                    i += 1;
                    continue;
                }

                let expand = match b {
                    b',' => {
                        i + 1 < bytes.len() && !is_breakable(bytes[i + 1])
                    }
                    b':' => {
                        // Skip if part of `::` on either side.
                        let next_is_colon =
                            i + 1 < bytes.len() && bytes[i + 1] == b':';
                        let prev_is_colon = prev == b':';
                        i + 1 < bytes.len()
                            && !is_breakable(bytes[i + 1])
                            && !next_is_colon
                            && !prev_is_colon
                    }
                    _ => false,
                };

                if expand {
                    out.push(b as char);
                    out.push(' ');
                    prev = b' ';
                    i += 1;
                    continue;
                }

                out.push(b as char);
                prev = b;
                i += 1;
            }
            ScanState::StringLit => {
                out.push(b as char);
                if b == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::LineComment => {
                out.push(b as char);
                i += 1;
            }
            ScanState::BlockComment => {
                out.push(b as char);
                if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    out.push('/');
                    i += 2;
                    state = ScanState::Code;
                    continue;
                }
                i += 1;
            }
        }
    }

    out
}

#[derive(Copy, Clone)]
enum ScanState {
    Code,
    StringLit,
    LineComment,
    BlockComment,
}

fn is_breakable(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn reindent(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut depth: usize = 0;

    for raw in source.lines() {
        if raw.trim().is_empty() {
            out.push('\n');
            continue;
        }

        let stripped = raw.trim_start();
        let (open, close) = brace_delta(stripped);
        let line_depth = depth.saturating_sub(leading_closes(stripped));
        for _ in 0..line_depth {
            out.push_str(INDENT);
        }
        out.push_str(stripped);
        out.push('\n');

        depth = depth + open;
        depth = depth.saturating_sub(close);
    }

    out
}

/// How many `{` and `}` occur on this line, ignoring those inside
/// strings or comments.
fn brace_delta(line: &str) -> (usize, usize) {
    let mut open = 0usize;
    let mut close = 0usize;
    let mut state = ScanState::Code;
    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        match state {
            ScanState::Code => {
                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    return (open, close);
                }
                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    state = ScanState::BlockComment;
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    state = ScanState::StringLit;
                    i += 1;
                    continue;
                }
                if b == b'{' {
                    open += 1;
                } else if b == b'}' {
                    close += 1;
                }
                i += 1;
            }
            ScanState::StringLit => {
                if b == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::BlockComment => {
                if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    i += 2;
                    state = ScanState::Code;
                    continue;
                }
                i += 1;
            }
            ScanState::LineComment => unreachable!(),
        }
    }
    (open, close)
}

/// Number of `}` at the very start of a line (before any other token).
fn leading_closes(stripped: &str) -> usize {
    let bytes = stripped.as_bytes();
    let mut i = 0;
    let mut n = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'}' {
            n += 1;
            i += 1;
            continue;
        }
        if b == b' ' || b == b'\t' {
            i += 1;
            continue;
        }
        break;
    }
    n
}

fn collapse_blank_runs(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut blanks: usize = 0;
    let mut wrote_any = false;

    for line in source.lines() {
        if line.trim().is_empty() {
            blanks += 1;
            continue;
        }
        if wrote_any {
            let to_emit = blanks.min(1);
            for _ in 0..to_emit {
                out.push('\n');
            }
        }
        out.push_str(line);
        out.push('\n');
        wrote_any = true;
        blanks = 0;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotent_on_simple_fn() {
        let src = "fn main() {\n    print(\"ok\")\n}\n";
        assert_eq!(format_source(src), src);
    }

    #[test]
    fn fixes_indentation() {
        let src = "fn main() {\nprint(\"ok\")\n}\n";
        let expected = "fn main() {\n    print(\"ok\")\n}\n";
        assert_eq!(format_source(src), expected);
    }

    #[test]
    fn collapses_blank_lines() {
        let src = "fn a() {}\n\n\n\nfn b() {}\n";
        let expected = "fn a() {}\n\nfn b() {}\n";
        assert_eq!(format_source(src), expected);
    }

    #[test]
    fn comma_normalization_skips_strings() {
        let src = "let s = \"a,b,c\"\n";
        assert_eq!(format_source(src), src);
    }

    #[test]
    fn imports_are_canonicalized_by_ast_printer() {
        // The AST stores import paths with `.` separators (the grammar
        // accepts both `use std::io` and `import std.io`). The Phase 5.1
        // AST printer canonicalizes to `import std.io` so the in-tree
        // formatter produces one shape regardless of what the author
        // typed.
        let src = "use std::io\n";
        assert_eq!(format_source(src), "import std.io\n");
        let canonical = "import std.io\n";
        assert_eq!(format_source(canonical), canonical);
    }
}
