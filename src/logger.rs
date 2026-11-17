//! Tiny file logger: `cs2-res-switcher.log` next to the executable.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use windows::Win32::System::SystemInformation::GetLocalTime;

static LOG: Mutex<Option<File>> = Mutex::new(None);

/// Truncate the log when it grows past this size at startup.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

pub fn init() {
    let path = crate::config::exe_dir().join("cs2-res-switcher.log");
    let too_big = std::fs::metadata(&path)
        .map(|m| m.len() > MAX_LOG_BYTES)
        .unwrap_or(false);

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(!too_big)
        .truncate(too_big)
        .open(&path)
        .ok();

    if let Ok(mut guard) = LOG.lock() {
        *guard = file;
    }
}

pub fn write(level: &str, msg: &str) {
    // SAFETY: GetLocalTime has no preconditions.
    let t = unsafe { GetLocalTime() };
    let line = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} [{level}] {msg}\n",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    );

    #[cfg(debug_assertions)]
    eprint!("{line}");

    if let Ok(mut guard) = LOG.lock() {
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::logger::write("INFO", &format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::logger::write("ERROR", &format!($($arg)*)) };
}
