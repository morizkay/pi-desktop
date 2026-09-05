use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static REQ_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiSessionSummary {
    pub path: String,
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub updated_at: String,
}

pub struct PiRpcProcess {
    child: Child,
    alive: Arc<AtomicBool>,
    stdin: Mutex<ChildStdin>,
    pending: Arc<Mutex<HashMap<String, std::sync::mpsc::Sender<Value>>>>,
}

pub struct PiRpcManager {
    process: Mutex<Option<PiRpcProcess>>,
    cwd: Mutex<String>,
    session_path: Mutex<Option<String>>,
    transition: Mutex<()>,
}

impl PiRpcManager {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
            cwd: Mutex::new(default_cwd()),
            session_path: Mutex::new(None),
            transition: Mutex::new(()),
        }
    }

    pub fn pi_binary() -> String {
        std::env::var("PI_DESKTOP_PI_BIN").unwrap_or_else(|_| "pi".to_string())
    }

    pub fn sessions_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("PI_CODING_AGENT_SESSION_DIR") {
            return PathBuf::from(dir);
        }
        crate::pi_config::agent_dir().join("sessions")
    }

    pub fn list_sessions(project_cwd: Option<&str>) -> Result<Vec<PiSessionSummary>, String> {
        let base = Self::sessions_dir();
        if !base.is_dir() {
            return Ok(vec![]);
        }

        let mut sessions = Vec::new();
        collect_sessions(&base, &mut sessions)?;

        if let Some(cwd) = project_cwd {
            let normalized = normalize_cwd(cwd);
            sessions.retain(|s| normalize_cwd(&s.cwd) == normalized);
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn ensure_running(&self, app: &AppHandle, cwd: &str) -> Result<(), String> {
        let mut proc_guard = self.process.lock().map_err(|e| e.to_string())?;
        if let Some(proc) = proc_guard.as_mut() {
            if proc.alive.load(Ordering::SeqCst)
                && proc.child.try_wait().map_err(|e| e.to_string())?.is_none()
            {
                return Ok(());
            }
        }
        if let Some(mut dead) = proc_guard.take() {
            dead.alive.store(false, Ordering::SeqCst);
            let _ = dead.child.kill();
            let _ = dead.child.wait();
        }

        let cwd = cwd.to_string();
        *self.cwd.lock().map_err(|e| e.to_string())? = cwd.clone();

        let mut command = Command::new(Self::pi_binary());
        command.args(["--mode", "rpc"]);
        if let Some(path) = self.get_session_path().filter(|p| Path::new(p).is_file()) {
            command.args(["--session", &path]);
        }
        let mut child = command
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to spawn pi: {e}. Install pi-coding-agent or set PI_DESKTOP_PI_BIN."
                )
            })?;

        let stdin = child.stdin.take().ok_or("pi stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("pi stdout unavailable")?;

        let pending: Arc<Mutex<HashMap<String, std::sync::mpsc::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = pending.clone();
        let app_handle = app.clone();

        let alive = Arc::new(AtomicBool::new(true));
        let reader_alive = alive.clone();
        if let Some(stderr) = child.stderr.take() {
            let app = app.clone();
            let active = alive.clone();
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    if !active.load(Ordering::SeqCst) {
                        break;
                    }
                    let _ = app.emit(
                        "pi-rpc-event",
                        json!({"type":"harness_log", "message":line}),
                    );
                }
            });
        }
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if !reader_alive.load(Ordering::SeqCst) {
                    break;
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                    continue;
                };

                if value.get("type").and_then(|t| t.as_str()) == Some("response") {
                    if let Some(id) = value.get("id").and_then(|i| i.as_str()) {
                        if let Ok(mut map) = pending_reader.lock() {
                            if let Some(tx) = map.remove(id) {
                                let _ = tx.send(value);
                                continue;
                            }
                        }
                    }
                }

                let _ = app_handle.emit("pi-rpc-event", value);
            }
            let was_alive = reader_alive.swap(false, Ordering::SeqCst);
            if let Ok(mut map) = pending_reader.lock() {
                map.clear();
            }
            if was_alive {
                let _ = app_handle.emit("pi-rpc-disconnected", ());
            }
        });

        *proc_guard = Some(PiRpcProcess {
            child,
            alive,
            stdin: Mutex::new(stdin),
            pending,
        });
        Ok(())
    }

    pub fn rpc_call(&self, app: &AppHandle, cwd: &str, command: Value) -> Result<Value, String> {
        let ty = command
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if ty.is_empty() {
            return Err("RPC command must have a type".into());
        }
        if ty == "extension_ui_response" {
            return Err("Use the extension response channel".into());
        }
        let changes_session = matches!(
            ty.as_str(),
            "switch_session" | "new_session" | "fork" | "clone"
        );
        let _transition = if changes_session {
            Some(
                self.transition
                    .try_lock()
                    .map_err(|_| "A session change is already in progress")?,
            )
        } else {
            None
        };
        // Reject invalid targets before any live session state changes.
        if ty == "switch_session" {
            let path = command
                .get("sessionPath")
                .and_then(Value::as_str)
                .ok_or("Missing session path")?;
            session_summary_from_file(Path::new(path)).ok_or("Invalid session file")?;
        }
        self.ensure_running(app, cwd)?;
        let id = REQ_ID.fetch_add(1, Ordering::Relaxed);
        let req_id = format!("req-{id}");

        let mut cmd = command;
        if let Some(obj) = cmd.as_object_mut() {
            obj.insert("id".to_string(), json!(req_id));
        }

        let rx = self.send_request(&req_id, &cmd)?;

        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(v) => {
                validate_response(&v)?;
                if changes_session {
                    let state = self.rpc_call(app, cwd, json!({"type":"get_state"}))?;
                    self.update_identity(&state)?;
                } else if ty == "get_state" {
                    self.update_identity(&v)?;
                }
                Ok(v)
            }
            Err(_) => {
                if let Ok(proc_guard) = self.process.lock() {
                    if let Some(proc) = proc_guard.as_ref() {
                        if let Ok(mut pending) = proc.pending.lock() {
                            pending.remove(&req_id);
                        }
                    }
                }
                Err("Pi connection lost or request timed out. Acceptance is unknown; reconnect and check the transcript before retrying.".to_string())
            }
        }
    }

    // The process lock covers only insertion/write, never waiting for a response.
    fn send_request(
        &self,
        req_id: &str,
        command: &Value,
    ) -> Result<std::sync::mpsc::Receiver<Value>, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        let guard = self.process.lock().map_err(|e| e.to_string())?;
        let proc = guard.as_ref().ok_or("Pi disconnected")?;
        {
            let mut pending = proc.pending.lock().map_err(|e| e.to_string())?;
            if !proc.alive.load(Ordering::SeqCst) {
                return Err("Pi disconnected".into());
            }
            pending.insert(req_id.to_string(), tx);
        }
        let sent = (|| {
            let mut stdin = proc.stdin.lock().map_err(|e| e.to_string())?;
            writeln!(*stdin, "{}", command).map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())
        })();
        if let Err(error) = sent {
            if let Ok(mut pending) = proc.pending.lock() {
                pending.remove(req_id);
            }
            return Err(error);
        }
        Ok(rx)
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut proc_guard = self.process.lock().map_err(|e| e.to_string())?;
        if let Some(mut proc) = proc_guard.take() {
            proc.alive.store(false, Ordering::SeqCst);
            if let Ok(mut map) = proc.pending.lock() {
                map.clear();
            }
            let _ = proc.child.kill();
            let _ = proc.child.wait();
        }
        Ok(())
    }

    fn update_identity(&self, state: &Value) -> Result<(), String> {
        let path = state
            .pointer("/data/sessionFile")
            .and_then(Value::as_str)
            .map(String::from);
        if let Some(summary) = path
            .as_ref()
            .and_then(|p| session_summary_from_file(Path::new(p)))
        {
            *self.cwd.lock().map_err(|e| e.to_string())? = summary.cwd;
        }
        self.set_session_path(path)
    }

    pub fn send_extension_response(&self, response: Value) -> Result<(), String> {
        if response.get("type").and_then(Value::as_str) != Some("extension_ui_response")
            || response.get("id").and_then(Value::as_str).is_none()
        {
            return Err("Invalid extension UI response".into());
        }
        let guard = self.process.lock().map_err(|e| e.to_string())?;
        let proc = guard.as_ref().ok_or("Pi disconnected")?;
        let mut stdin = proc.stdin.lock().map_err(|e| e.to_string())?;
        writeln!(*stdin, "{}", response).map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())
    }

    pub fn new_session(&self, app: &AppHandle, cwd: Option<String>) -> Result<Value, String> {
        let target = cwd.unwrap_or(self.get_cwd()?);
        let target = Path::new(&target)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !target.is_dir() {
            return Err("Workspace must be a directory".into());
        }
        let target = target.to_string_lossy().into_owned();
        if normalize_cwd(&self.get_cwd()?) != target {
            // RPC new_session has no cwd field; workspace changes require a fresh process.
            let state = self.rpc_call(app, &self.get_cwd()?, json!({"type":"get_state"}))?;
            if state.pointer("/data/isStreaming").and_then(Value::as_bool) == Some(true) {
                return Err("Stop the agent before changing workspace".into());
            }
            let previous_cwd = self.get_cwd()?;
            let previous_path = self.get_session_path();
            // Honor the current extension's before-switch veto before replacing its process.
            self.rpc_call(app, &previous_cwd, json!({"type":"new_session"}))?;
            self.stop()?;
            self.set_session_path(None)?;
            *self.cwd.lock().map_err(|e| e.to_string())? = target.clone();
            // Starting RPC already creates a fresh session; do not discard it a second time.
            if let Err(error) = self.rpc_call(app, &target, json!({"type":"get_state"})) {
                self.stop()?;
                *self.cwd.lock().map_err(|e| e.to_string())? = previous_cwd;
                self.set_session_path(previous_path)?;
                let _ = app.emit("pi-rpc-disconnected", ());
                return Err(format!("Could not open workspace: {error}. Previous session retained for reconnect."));
            }
            return Ok(
                json!({"type":"response", "success":true, "command":"new_session", "data":{"cancelled":false}}),
            );
        }
        self.rpc_call(app, &target, json!({"type":"new_session"}))
    }

    pub fn set_session_path(&self, path: Option<String>) -> Result<(), String> {
        *self.session_path.lock().map_err(|e| e.to_string())? = path;
        Ok(())
    }

    pub fn get_session_path(&self) -> Option<String> {
        self.session_path.lock().ok().and_then(|g| g.clone())
    }

    pub fn get_cwd(&self) -> Result<String, String> {
        Ok(self.cwd.lock().map_err(|e| e.to_string())?.clone())
    }

    pub fn set_cwd(&self, cwd: String) -> Result<(), String> {
        if self.process.lock().map_err(|e| e.to_string())?.is_some() {
            return Err("Use New session to change the live workspace".into());
        }
        let path = Path::new(&cwd).canonicalize().map_err(|e| e.to_string())?;
        if !path.is_dir() {
            return Err("Workspace must be a directory".into());
        }
        *self.cwd.lock().map_err(|e| e.to_string())? = path.to_string_lossy().into_owned();
        Ok(())
    }
}

fn default_cwd() -> String {
    std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string())
}

fn normalize_cwd(cwd: &str) -> String {
    let p = cwd.trim();
    if p.is_empty() {
        return String::new();
    }
    Path::new(p)
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string())
}

fn collect_sessions(dir: &Path, out: &mut Vec<PiSessionSummary>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_sessions(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            if let Some(summary) = session_summary_from_file(&path) {
                out.push(summary);
            }
        }
    }
    Ok(())
}

fn session_summary_from_file(path: &Path) -> Option<PiSessionSummary> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut session_id = String::new();
    let mut cwd = String::new();
    let mut updated_at = String::new();
    let mut named_title = None;
    let mut found_prompt = false;
    let mut title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Session")
        .to_string();

    for line in content.split('\n') {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if ty == "session" {
            session_id = v
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            cwd = v
                .get("cwd")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            updated_at = v
                .get("timestamp")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
        }
        if let Some(ts) = v.get("timestamp").and_then(Value::as_str) {
            updated_at = ts.to_string();
        }
        if ty == "session_info" {
            if let Some(name) = v.get("name").and_then(Value::as_str) {
                named_title = Some(name.to_string());
            }
        }
        if ty == "message" {
            if let Some(msg) = v.get("message") {
                if !found_prompt && msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                    if let Some(text) = first_text_content(msg.get("content")) {
                        title = text.chars().take(80).collect();
                        found_prompt = true;
                    }
                }
            }
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                updated_at = ts.to_string();
            }
        }
    }

    if session_id.is_empty() {
        return None;
    }

    let meta = std::fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()
        .map(|t| {
            use std::time::UNIX_EPOCH;
            t.duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default()
        })
        .unwrap_or_else(|| updated_at.clone());

    Some(PiSessionSummary {
        path: path.to_string_lossy().into_owned(),
        id: session_id,
        cwd,
        title: named_title.unwrap_or(title),
        updated_at: if updated_at.is_empty() {
            modified
        } else {
            updated_at
        },
    })
}

fn first_text_content(content: Option<&Value>) -> Option<String> {
    let content = content?;
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }
    let arr = content.as_array()?;
    for item in arr {
        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
            return item.get("text").and_then(|t| t.as_str()).map(String::from);
        }
    }
    None
}

fn validate_response(value: &Value) -> Result<(), String> {
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Invalid Pi response")
            .to_string());
    }
    if value.pointer("/data/cancelled").and_then(Value::as_bool) == Some(true) {
        return Err("Request cancelled by a Pi extension".into());
    }
    Ok(())
}
impl Drop for PiRpcManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_request_does_not_block_approval_or_abort_writes() {
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let manager = PiRpcManager::new();
        *manager.process.lock().unwrap() = Some(PiRpcProcess {
            child,
            stdin: Mutex::new(stdin),
            alive: Arc::new(AtomicBool::new(true)),
            pending: Arc::new(Mutex::new(HashMap::new())),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let lines: Vec<_> = BufReader::new(stdout)
                .lines()
                .take(3)
                .map(Result::unwrap)
                .collect();
            tx.send(lines).unwrap();
        });
        let pending = manager
            .send_request("prompt", &json!({"id":"prompt", "type":"prompt"}))
            .unwrap();
        assert!(pending.recv_timeout(Duration::from_millis(5)).is_err());
        manager
            .send_extension_response(
                json!({"type":"extension_ui_response", "id":"dialog", "confirmed":true}),
            )
            .unwrap();
        let _abort = manager
            .send_request("abort", &json!({"id":"abort", "type":"abort"}))
            .unwrap();
        let lines = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&lines[1]).unwrap()["id"],
            "dialog"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&lines[2]).unwrap()["type"],
            "abort"
        );
        manager.stop().unwrap();
    }
    #[test]
    fn response_failure_and_cancel_are_errors() {
        assert!(validate_response(&json!({"success":false,"error":"no"})).is_err());
        assert!(validate_response(&json!({"success":true,"data":{"cancelled":true}})).is_err());
        assert!(validate_response(&json!({"success":true})).is_ok());
    }
    #[test]
    fn lf_framing_preserves_unicode_separators() {
        let input = "{\"text\":\"a\u{2028}b\u{2029}c\"}\r\n";
        let lines: Vec<_> = BufReader::new(input.as_bytes()).lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(lines[0].as_ref().unwrap()).unwrap()["text"],
            "a\u{2028}b\u{2029}c"
        );
    }
}
