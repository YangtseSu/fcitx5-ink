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

/// One panel-input request. The addon owns the server; the panel connects for
/// each request, writes one JSON line and reads one JSON line.
pub fn send_text(socket: &Path, text: &str) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket)
        .map_err(|error| format!("connect {}: {error}", socket.display()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("write timeout: {error}"))?;

    let request = serde_json::json!({ "op": "text", "text": text });
    writeln!(stream, "{request}").map_err(|error| format!("write: {error}"))?;

    let mut reply = String::new();
    BufReader::new(stream)
        .read_line(&mut reply)
        .map_err(|error| format!("read: {error}"))?;
    let reply: serde_json::Value =
        serde_json::from_str(reply.trim()).map_err(|error| format!("reply: {error}"))?;
    if reply.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(reply
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unavailable")
            .to_owned())
    }
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
            assert!(request.contains("\"op\":\"text\""));
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
}
