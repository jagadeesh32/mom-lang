//! Phase 5.1 debugger driver — `mom dbg`.
//!
//! Speaks the [Debug Adapter Protocol] (DAP) over stdio so editors
//! that ship a DAP client — VS Code, Helix, Zed, JetBrains — can
//! attach without any custom plugin work. The stage-0 surface is
//! intentionally small: it supports just enough of the protocol to
//! launch a `.mom` source file, capture its stdout, and announce
//! when the run terminates. Full breakpoint, stepping, and
//! variables support lands once the native stage-2 backend emits
//! DWARF v5 / CodeView with source-line records.
//!
//! Supported requests:
//!
//!   * `initialize`         → capabilities + `initialized` event
//!   * `launch`             → run the program (interpreter), stream
//!                            its `print(...)` output as `output`
//!                            events, finish with `terminated` +
//!                            `exited`
//!   * `configurationDone`  → ack
//!   * `threads`            → single main thread (id = 1)
//!   * `stackTrace`         → minimal one-frame trace
//!   * `scopes` / `variables` → empty (stage-0)
//!   * `continue` / `next` / `stepIn` / `stepOut` / `pause` → ack
//!   * `disconnect`         → ack + exit
//!
//! Framing is `Content-Length: <n>\r\n\r\n<json>`, identical to
//! `mom lsp`. JSON is hand-rolled; the wire reader and writer are
//! shared with the rest of Phase 5 to keep the workspace
//! dependency-free.
//!
//! [Debug Adapter Protocol]: https://microsoft.github.io/debug-adapter-protocol/

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::run_source;

pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut server = DbgServer::new();

    loop {
        let message = match read_message(&mut reader)? {
            Some(m) => m,
            None => return Ok(()),
        };
        let outgoing = server.handle(&message);
        for out in outgoing {
            match out {
                Outgoing::Send(payload) => write_message(&mut writer, &payload)?,
                Outgoing::Exit => return Ok(()),
            }
        }
    }
}

pub struct DbgServer {
    next_seq: u64,
    /// Set after `initialize`; gates other requests so we don't
    /// respond to a `launch` that arrives before the handshake.
    initialized: bool,
}

#[derive(Debug, Clone)]
pub enum Outgoing {
    Send(String),
    Exit,
}

impl Default for DbgServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DbgServer {
    pub fn new() -> Self {
        Self {
            next_seq: 1,
            initialized: false,
        }
    }

    pub fn handle(&mut self, request_json: &str) -> Vec<Outgoing> {
        let command = match json_string_field(request_json, "command") {
            Some(c) => c,
            None => return Vec::new(),
        };
        let request_seq = json_raw_field(request_json, "seq").unwrap_or_else(|| "0".to_string());

        match command.as_str() {
            "initialize" => {
                self.initialized = true;
                let body = INITIALIZE_BODY.to_string();
                let response = self.response(&request_seq, &command, true, Some(&body), None);
                let initialized = self.event("initialized", None);
                vec![Outgoing::Send(response), Outgoing::Send(initialized)]
            }
            "configurationDone" => {
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    true,
                    None,
                    None,
                ))]
            }
            "launch" => self.on_launch(request_json, &request_seq, &command),
            "threads" => {
                let body = "{\"threads\":[{\"id\":1,\"name\":\"main\"}]}";
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    true,
                    Some(body),
                    None,
                ))]
            }
            "stackTrace" => {
                let body = "{\"stackFrames\":[{\"id\":1,\"name\":\"main\",\"line\":1,\"column\":1}],\"totalFrames\":1}";
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    true,
                    Some(body),
                    None,
                ))]
            }
            "scopes" => {
                let body = "{\"scopes\":[]}";
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    true,
                    Some(body),
                    None,
                ))]
            }
            "variables" => {
                let body = "{\"variables\":[]}";
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    true,
                    Some(body),
                    None,
                ))]
            }
            "continue" | "next" | "stepIn" | "stepOut" | "pause" => {
                let body = "{\"allThreadsContinued\":true}";
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    true,
                    Some(body),
                    None,
                ))]
            }
            "disconnect" => {
                let response = self.response(&request_seq, &command, true, None, None);
                vec![Outgoing::Send(response), Outgoing::Exit]
            }
            _ => {
                vec![Outgoing::Send(self.response(
                    &request_seq,
                    &command,
                    false,
                    None,
                    Some(&format!("unsupported request '{command}'")),
                ))]
            }
        }
    }

    fn on_launch(&mut self, request_json: &str, request_seq: &str, command: &str) -> Vec<Outgoing> {
        if !self.initialized {
            return vec![Outgoing::Send(self.response(
                request_seq,
                command,
                false,
                None,
                Some("received 'launch' before 'initialize'"),
            ))];
        }
        let program = match json_string_field(request_json, "program") {
            Some(p) => p,
            None => {
                return vec![Outgoing::Send(self.response(
                    request_seq,
                    command,
                    false,
                    None,
                    Some("launch.arguments.program is required"),
                ))];
            }
        };

        let response = self.response(request_seq, command, true, None, None);
        let mut out: Vec<Outgoing> = vec![Outgoing::Send(response)];

        let path = PathBuf::from(&program);
        match fs::read_to_string(&path) {
            Ok(source) => match run_source(&source) {
                Ok(stdout) => {
                    if !stdout.is_empty() {
                        out.push(Outgoing::Send(self.event(
                            "output",
                            Some(&format!(
                                "{{\"category\":\"stdout\",\"output\":{}}}",
                                json_string(&stdout)
                            )),
                        )));
                    }
                    out.push(Outgoing::Send(self.event("terminated", None)));
                    out.push(Outgoing::Send(
                        self.event("exited", Some("{\"exitCode\":0}")),
                    ));
                }
                Err(diag) => {
                    out.push(Outgoing::Send(self.event(
                        "output",
                        Some(&format!(
                            "{{\"category\":\"stderr\",\"output\":{}}}",
                            json_string(&format!("{diag}\n"))
                        )),
                    )));
                    out.push(Outgoing::Send(self.event("terminated", None)));
                    out.push(Outgoing::Send(
                        self.event("exited", Some("{\"exitCode\":1}")),
                    ));
                }
            },
            Err(err) => {
                out.push(Outgoing::Send(self.event(
                    "output",
                    Some(&format!(
                        "{{\"category\":\"stderr\",\"output\":{}}}",
                        json_string(&format!("failed to read '{}': {err}\n", path.display()))
                    )),
                )));
                out.push(Outgoing::Send(self.event("terminated", None)));
                out.push(Outgoing::Send(
                    self.event("exited", Some("{\"exitCode\":2}")),
                ));
            }
        }

        out
    }

    fn response(
        &mut self,
        request_seq: &str,
        command: &str,
        success: bool,
        body: Option<&str>,
        message: Option<&str>,
    ) -> String {
        let seq = self.bump_seq();
        let mut out = format!(
            "{{\"seq\":{seq},\"type\":\"response\",\"request_seq\":{request_seq},\
             \"command\":\"{command}\",\"success\":{success}",
        );
        if let Some(msg) = message {
            out.push_str(&format!(",\"message\":{}", json_string(msg)));
        }
        if let Some(body) = body {
            out.push_str(&format!(",\"body\":{body}"));
        }
        out.push('}');
        out
    }

    fn event(&mut self, event: &str, body: Option<&str>) -> String {
        let seq = self.bump_seq();
        let mut out = format!("{{\"seq\":{seq},\"type\":\"event\",\"event\":\"{event}\"");
        if let Some(body) = body {
            out.push_str(&format!(",\"body\":{body}"));
        }
        out.push('}');
        out
    }

    fn bump_seq(&mut self) -> u64 {
        let s = self.next_seq;
        self.next_seq += 1;
        s
    }
}

const INITIALIZE_BODY: &str = "{\
\"supportsConfigurationDoneRequest\":true,\
\"supportsFunctionBreakpoints\":false,\
\"supportsConditionalBreakpoints\":false,\
\"supportsEvaluateForHovers\":false,\
\"supportsStepBack\":false,\
\"supportsSetVariable\":false,\
\"supportsRestartFrame\":false,\
\"supportsGotoTargetsRequest\":false,\
\"supportsStepInTargetsRequest\":false,\
\"supportsCompletionsRequest\":false,\
\"supportsRestartRequest\":false,\
\"supportsExceptionOptions\":false,\
\"supportsValueFormattingOptions\":false,\
\"supportsExceptionInfoRequest\":false,\
\"supportTerminateDebuggee\":true,\
\"supportSuspendDebuggee\":false,\
\"supportsDelayedStackTraceLoading\":false,\
\"supportsLoadedSourcesRequest\":false,\
\"supportsLogPoints\":false,\
\"supportsTerminateThreadsRequest\":false,\
\"supportsSetExpression\":false,\
\"supportsTerminateRequest\":true,\
\"supportsDataBreakpoints\":false,\
\"supportsReadMemoryRequest\":false,\
\"supportsWriteMemoryRequest\":false,\
\"supportsDisassembleRequest\":false,\
\"supportsCancelRequest\":false,\
\"supportsBreakpointLocationsRequest\":false,\
\"supportsClipboardContext\":false,\
\"supportsSteppingGranularity\":false,\
\"supportsInstructionBreakpoints\":false,\
\"supportsExceptionFilterOptions\":false,\
\"supportsSingleThreadExecutionRequests\":false\
}";

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
    use std::path::PathBuf;

    fn tmp_path(label: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        dir.push(format!("mom-dbg-{label}-{pid}-{nanos}.mom"));
        dir
    }

    fn join(messages: &[Outgoing]) -> String {
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
    fn initialize_replies_with_capabilities_and_event() {
        let mut server = DbgServer::new();
        let out = server
            .handle("{\"seq\":1,\"type\":\"request\",\"command\":\"initialize\",\"arguments\":{}}");
        let joined = join(&out);
        assert!(joined.contains("\"command\":\"initialize\""));
        assert!(joined.contains("supportsConfigurationDoneRequest"));
        assert!(joined.contains("\"event\":\"initialized\""));
    }

    #[test]
    fn launch_runs_program_and_emits_terminated() {
        let path = tmp_path("ok");
        std::fs::write(&path, "fn main() { print(\"hi\") }\n").unwrap();
        let mut server = DbgServer::new();
        let _ = server.handle("{\"seq\":1,\"command\":\"initialize\",\"arguments\":{}}");
        let req = format!(
            "{{\"seq\":2,\"command\":\"launch\",\"arguments\":{{\"program\":{}}}}}",
            json_string(&path.display().to_string())
        );
        let out = server.handle(&req);
        let joined = join(&out);
        assert!(joined.contains("\"command\":\"launch\""));
        assert!(joined.contains("\"category\":\"stdout\""));
        assert!(joined.contains("\"event\":\"terminated\""));
        assert!(joined.contains("\"event\":\"exited\""));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn launch_before_initialize_fails() {
        let mut server = DbgServer::new();
        let out =
            server.handle("{\"seq\":1,\"command\":\"launch\",\"arguments\":{\"program\":\"x\"}}");
        let joined = join(&out);
        assert!(joined.contains("\"success\":false"));
        assert!(joined.contains("before 'initialize'"));
    }

    #[test]
    fn disconnect_exits() {
        let mut server = DbgServer::new();
        let _ = server.handle("{\"seq\":1,\"command\":\"initialize\",\"arguments\":{}}");
        let out = server.handle("{\"seq\":2,\"command\":\"disconnect\"}");
        assert!(matches!(out.last(), Some(Outgoing::Exit)));
    }

    #[test]
    fn threads_returns_single_main_thread() {
        let mut server = DbgServer::new();
        let _ = server.handle("{\"seq\":1,\"command\":\"initialize\",\"arguments\":{}}");
        let out = server.handle("{\"seq\":2,\"command\":\"threads\"}");
        let joined = join(&out);
        assert!(joined.contains("\"name\":\"main\""));
    }
}
