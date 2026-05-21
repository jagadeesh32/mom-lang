//! Phase 5 language-server skeleton — `mom lsp`.
//!
//! Speaks LSP over stdio using the standard
//! `Content-Length: <n>\r\n\r\n<json>` framing. Supports the minimum
//! viable surface so editors can attach:
//!
//!   * `initialize` / `initialized` / `shutdown` / `exit`
//!   * `textDocument/didOpen` and `textDocument/didChange`
//!   * `textDocument/publishDiagnostics` (server → client)
//!
//! Diagnostics come straight from `check_source` so the same errors a
//! `mom check` run produces show up in the editor. Completions, hover,
//! and code actions are explicitly deferred to Phase 5.1.
//!
//! JSON is hand-rolled rather than pulling in a crate so the Phase 5
//! deliverable keeps the workspace dependency-free.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use crate::check_source;
use crate::diagnostic::Diagnostic;

pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut server = Server::default();

    loop {
        let message = match read_message(&mut reader)? {
            Some(m) => m,
            None => return Ok(()),
        };
        if let Some(action) = server.handle(&message) {
            match action {
                Action::Reply(payload) => write_message(&mut writer, &payload)?,
                Action::Notify(payload) => write_message(&mut writer, &payload)?,
                Action::Exit => return Ok(()),
            }
        }
    }
}

#[derive(Default)]
struct Server {
    documents: HashMap<String, String>,
    shutdown_requested: bool,
}

enum Action {
    Reply(String),
    Notify(String),
    Exit,
}

impl Server {
    fn handle(&mut self, message: &str) -> Option<Action> {
        let method = json_string_field(message, "method")?;
        let id = json_raw_field(message, "id");
        match method.as_str() {
            "initialize" => Some(Action::Reply(reply(
                id.as_deref().unwrap_or("null"),
                INITIALIZE_RESULT,
            ))),
            "initialized" => None,
            "shutdown" => {
                self.shutdown_requested = true;
                Some(Action::Reply(reply(
                    id.as_deref().unwrap_or("null"),
                    "null",
                )))
            }
            "exit" => Some(Action::Exit),
            "textDocument/didOpen" => self.on_did_open(message),
            "textDocument/didChange" => self.on_did_change(message),
            _ => None,
        }
    }

    fn on_did_open(&mut self, message: &str) -> Option<Action> {
        let uri = json_string_field(message, "uri")?;
        let text = json_string_field(message, "text").unwrap_or_default();
        self.documents.insert(uri.clone(), text.clone());
        Some(Action::Notify(diagnostics_notification(&uri, &text)))
    }

    fn on_did_change(&mut self, message: &str) -> Option<Action> {
        let uri = json_string_field(message, "uri")?;
        let text = json_string_field(message, "text").unwrap_or_default();
        self.documents.insert(uri.clone(), text.clone());
        Some(Action::Notify(diagnostics_notification(&uri, &text)))
    }
}

fn diagnostics_notification(uri: &str, source: &str) -> String {
    let diagnostics: Vec<Diagnostic> = match check_source(source) {
        Ok(_) => Vec::new(),
        Err(diag) => vec![diag],
    };

    let mut diag_json = String::from("[");
    for (i, d) in diagnostics.iter().enumerate() {
        if i > 0 {
            diag_json.push(',');
        }
        let line = d.span.line.saturating_sub(1);
        let col = d.span.column.saturating_sub(1);
        diag_json.push_str(&format!(
            "{{\"range\":{{\"start\":{{\"line\":{line},\"character\":{col}}},\
             \"end\":{{\"line\":{line},\"character\":{col}}}}},\
             \"severity\":1,\"source\":\"mom\",\"message\":{}}}",
            json_string(&d.message)
        ));
    }
    diag_json.push(']');

    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\
         \"params\":{{\"uri\":{},\"diagnostics\":{}}}}}",
        json_string(uri),
        diag_json,
    )
}

fn reply(id: &str, result: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}"
    )
}

const INITIALIZE_RESULT: &str = "{\"capabilities\":{\"textDocumentSync\":1,\
\"diagnosticProvider\":{\"interFileDependencies\":false,\"workspaceDiagnostics\":false}},\
\"serverInfo\":{\"name\":\"mom-lsp\",\"version\":\"0.1.0\"}}";

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header)?;
        if n == 0 {
            return Ok(None);
        }
        let line = header.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let length = content_length.unwrap_or(0);
    if length == 0 {
        return Ok(Some(String::new()));
    }
    let mut buf = vec![0u8; length];
    reader.read_exact(&mut buf)?;
    Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
}

fn write_message<W: Write>(writer: &mut W, payload: &str) -> io::Result<()> {
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )?;
    writer.flush()
}

// -- microscopic JSON extractor ----------------------------------------------
//
// The LSP protocol leaves us no escape from JSON, but Phase 5 stays
// dependency-free. We only need to pluck a couple of values out of the
// incoming message; full JSON parsing can wait for Phase 6 stdlib.

fn json_string_field(message: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let mut search_from = 0;
    while let Some(start) = message[search_from..].find(&needle) {
        let absolute = search_from + start;
        let after = absolute + needle.len();
        let rest = &message[after..];
        let trimmed = rest.trim_start();
        if let Some(value) = read_json_string(trimmed) {
            return Some(value);
        }
        search_from = after;
    }
    None
}

fn json_raw_field(message: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = message.find(&needle)?;
    let after = start + needle.len();
    let rest = message[after..].trim_start();
    let mut end = 0;
    let bytes = rest.as_bytes();
    while end < bytes.len() {
        let c = bytes[end];
        if c == b',' || c == b'}' || c == b']' {
            break;
        }
        end += 1;
    }
    Some(rest[..end].trim().to_string())
}

fn read_json_string(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut out = String::new();
    let mut i = 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                other => out.push(other as char),
            }
            i += 2;
            continue;
        }
        if b == b'"' {
            return Some(out);
        }
        out.push(b as char);
        i += 1;
    }
    None
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
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
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_method_field() {
        let msg = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}";
        assert_eq!(json_string_field(msg, "method"), Some("initialize".into()));
    }

    #[test]
    fn extracts_text_field_with_escapes() {
        let msg = "{\"uri\":\"file:///a.mom\",\"text\":\"fn main() {}\\n\"}";
        assert_eq!(json_string_field(msg, "text"), Some("fn main() {}\n".into()));
    }

    #[test]
    fn diagnostics_empty_when_valid() {
        let payload = diagnostics_notification("file:///ok.mom", "fn main() {}\n");
        assert!(payload.contains("\"diagnostics\":[]"));
    }
}
