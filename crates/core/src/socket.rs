// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn default_path() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join("fcitx5-ink/panel-input.sock"))
}

fn round_trip(socket: &Path, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut stream = UnixStream::connect(socket)
        .map_err(|error| format!("connect {}: {error}", socket.display()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("write timeout: {error}"))?;

    writeln!(stream, "{request}").map_err(|error| format!("write: {error}"))?;

    let mut reply = String::new();
    BufReader::new(stream)
        .read_line(&mut reply)
        .map_err(|error| format!("read: {error}"))?;
    serde_json::from_str(reply.trim()).map_err(|error| format!("reply: {error}"))
}

fn checked(reply: serde_json::Value) -> Result<serde_json::Value, String> {
    if reply.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        Ok(reply)
    } else {
        Err(reply
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unavailable")
            .to_owned())
    }
}

/// One panel-input request. The addon owns the server; the panel connects for
/// each request, writes one JSON line and reads one JSON line.
pub fn send_text(socket: &Path, text: &str) -> Result<(), String> {
    checked(round_trip(
        socket,
        &serde_json::json!({ "op": "text", "text": text }),
    )?)
    .map(|_| ())
}

/// Re-read `config.toml` in the addon and return the effective hotkey.
pub fn reload(socket: &Path) -> Result<String, String> {
    let reply = checked(round_trip(socket, &serde_json::json!({ "op": "reload" }))?)?;
    reply
        .get("hotkey")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "reply is missing hotkey".to_owned())
}

/// Ask the addon to validate a hotkey with fcitx5's own parser.
pub fn validate_hotkey(socket: &Path, hotkey: &str) -> Result<(), String> {
    checked(round_trip(
        socket,
        &serde_json::json!({ "op": "validate", "hotkey": hotkey }),
    )?)
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    fn listener(reply: &'static str) -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("panel.sock");
        let listener = UnixListener::bind(&path).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            assert!(request.ends_with('\n'));
            stream.write_all(reply.as_bytes()).unwrap();
            stream.write_all(b"\n").unwrap();
        });
        (directory, path)
    }

    #[test]
    fn accepts_ok_reply() {
        let (_directory, path) = listener(r#"{"ok":true}"#);
        assert_eq!(send_text(&path, "你"), Ok(()));
    }

    #[test]
    fn surfaces_addon_error() {
        let (_directory, path) = listener(r#"{"ok":false,"error":"no_focus"}"#);
        assert_eq!(send_text(&path, "你"), Err("no_focus".to_owned()));
    }

    #[test]
    fn reload_returns_the_effective_hotkey() {
        let (_directory, path) = listener(r#"{"ok":true,"hotkey":"Control+Alt+J"}"#);
        assert_eq!(reload(&path), Ok("Control+Alt+J".to_owned()));
    }

    #[test]
    fn validation_reports_a_rejected_hotkey() {
        let (_directory, path) = listener(r#"{"ok":false,"error":"invalid_hotkey"}"#);
        assert_eq!(
            validate_hotkey(&path, "Ctrl+Alt+H"),
            Err("invalid_hotkey".to_owned())
        );
    }
}
