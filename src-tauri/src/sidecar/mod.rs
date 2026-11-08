pub mod protocol;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use protocol::{methods, Request, Response};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
struct Shared {
    state: Mutex<Option<Value>>,

    pending: Mutex<HashMap<u64, Option<Value>>>,
}

pub struct Sidecar {
    child: Mutex<Option<Child>>,

    stdin: Mutex<Option<std::process::ChildStdin>>,

    shared: Arc<Shared>,

    next_id: AtomicU64,

    running: AtomicBool,

    socket_path: PathBuf,
}

impl Sidecar {
    pub fn start(
        binary: &Path,
        socket_path: &Path,
        config_json: &str,
    ) -> Result<Self, SidecarError> {
        let program = find_binary(binary)
            .ok_or_else(|| SidecarError::NotFound(binary.display().to_string()))?;

        let _ = std::fs::remove_file(socket_path);

        let mut command = Command::new(&program);
        command
            .arg("--socket")
            .arg(socket_path)
            .arg("--config")
            .arg(config_json)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|err| SidecarError::Spawn {
            program: program.display().to_string(),
            message: err.to_string(),
        })?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let sidecar = Self {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(stdin),
            shared: Arc::new(Shared::default()),
            next_id: AtomicU64::new(1),
            running: AtomicBool::new(true),
            socket_path: socket_path.to_path_buf(),
        };

        if let Some(stdout) = stdout {
            let shared = Arc::clone(&sidecar.shared);
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    handle_line(&shared, line);
                }
            });
        }

        if let Some(stderr) = stderr {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    log::warn!("sidecar: {}", line.trim());
                }
            });
        }

        sidecar.wait_until_ready()?;

        Ok(sidecar)
    }

    fn wait_until_ready(&self) -> Result<(), SidecarError> {
        let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;

        while std::time::Instant::now() < deadline {
            if !self.running.load(Ordering::Relaxed) {
                return Err(SidecarError::Exited);
            }
            if let Ok(response) = self.call(methods::PING, None, Duration::from_millis(500)) {
                if response.error.is_none() {
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(50));
        }

        Err(SidecarError::Timeout)
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn call(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Response, SidecarError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = Request::call(id, method, params);

        let line = serde_json::to_string(&request)
            .map_err(|err| SidecarError::Protocol(err.to_string()))?;

        {
            let mut stdin = self.stdin.lock().map_err(|_| SidecarError::Exited)?;
            let stdin = stdin.as_mut().ok_or(SidecarError::Exited)?;
            writeln!(stdin, "{line}").map_err(|_| SidecarError::Exited)?;
            stdin.flush().map_err(|_| SidecarError::Exited)?;
        }

        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            let mut pending = self
                .shared
                .pending
                .lock()
                .map_err(|_| SidecarError::Exited)?;
            if let Some(response) = pending.remove(&id) {
                let Some(response) = response else {
                    return Err(SidecarError::Protocol(
                        "the sidecar closed the request".to_string(),
                    ));
                };
                let text = serde_json::to_string(&response)
                    .map_err(|err| SidecarError::Protocol(err.to_string()))?;
                return serde_json::from_str(&text)
                    .map_err(|err| SidecarError::Protocol(err.to_string()));
            }
            drop(pending);
            thread::sleep(Duration::from_millis(10));
        }

        Err(SidecarError::Timeout)
    }

    pub fn state(&self) -> Option<Value> {
        self.shared
            .state
            .lock()
            .ok()
            .and_then(|state| state.clone())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);

        if let Ok(mut stdin) = self.stdin.lock() {
            stdin.take();
        }

        if let Ok(mut child) = self.child.lock() {
            if let Some(child) = child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            *child = None;
        }

        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_line(shared: &Arc<Shared>, line: &str) {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        log::warn!("sidecar sent a line that is not JSON");
        return;
    };

    let Some(object) = value.as_object() else {
        return;
    };

    if let Some(id) = object.get("id").and_then(Value::as_u64) {
        let mut pending = match shared.pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        pending.insert(id, Some(value.clone()));
        return;
    }

    if object.get("method").and_then(Value::as_str) == Some(methods::STATE_CHANGED) {
        if let Some(state) = object.get("params") {
            let mut current = match shared.state.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            *current = Some(state.clone());
        }
    }
}

pub fn serve_socket<F>(path: &Path, handler: F) -> std::io::Result<thread::JoinHandle<()>>
where
    F: Fn(&str) -> String + Send + Sync + 'static,
{
    let _ = std::fs::remove_file(path);

    let listener = UnixListener::bind(path)?;
    let handler = Arc::new(handler);

    Ok(thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let handler = Arc::clone(&handler);
            thread::spawn(move || {
                serve_connection(stream, handler.as_ref());
            });
        }
    }))
}

fn serve_connection(stream: UnixStream, handler: &dyn Fn(&str) -> String) {
    let reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });
    let mut writer = stream;

    for line in reader.lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let answer = handler(&line);
        if writeln!(writer, "{answer}").is_err() {
            break;
        }
        let _ = writer.flush();
    }
}

fn find_binary(binary: &Path) -> Option<PathBuf> {
    if binary.is_file() {
        return Some(binary.to_path_buf());
    }

    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let name = binary
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ha-panel-sidecar");
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let Some(file_name) = file_name.to_str() else {
                        continue;
                    };
                    if file_name.starts_with(name) && entry.path().is_file() {
                        return Some(entry.path());
                    }
                }
            }
        }
    }

    crate::platform::which("ha-panel-sidecar")
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("the sidecar binary {0} was not found")]
    NotFound(String),

    #[error("cannot start the sidecar {program}: {message}")]
    Spawn { program: String, message: String },

    #[error("the sidecar is not running")]
    Exited,

    #[error("the sidecar did not answer in time")]
    Timeout,

    #[error("the sidecar sent an invalid message: {0}")]
    Protocol(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_that_is_not_json_is_ignored_rather_than_fatal() {
        let shared = Arc::new(Shared::default());
        handle_line(&shared, "this is not json");
        assert!(shared.state.lock().unwrap().is_none());
    }

    #[test]
    fn a_state_notification_updates_the_shared_state() {
        let shared = Arc::new(Shared::default());
        let line = r#"{"jsonrpc":"2.0","method":"stateChanged","params":{"connected":true}}"#;
        handle_line(&shared, line);

        let state = shared.state.lock().unwrap().clone().expect("state");
        assert_eq!(state["connected"], serde_json::Value::Bool(true));
    }

    #[test]
    fn a_response_is_matched_to_its_request() {
        let shared = Arc::new(Shared::default());
        handle_line(&shared, r#"{"jsonrpc":"2.0","id":4,"result":{"ok":true}}"#);

        let pending = shared.pending.lock().unwrap();
        let response = pending.get(&4).expect("pending").clone().expect("response");
        assert_eq!(response["result"]["ok"], serde_json::Value::Bool(true));
    }
}
