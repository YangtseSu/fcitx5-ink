// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

/// Open the packaged model without following a leaf symlink. The vendored
/// recognizer only needs a regular file that it can mmap; accepting only a
/// regular file also rejects FIFOs and directories before the map.
pub(crate) fn open_file_no_follow(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "handwriting model is not a regular file",
        ));
    }
    Ok(file)
}
