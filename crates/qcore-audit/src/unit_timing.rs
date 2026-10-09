//! Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME**, measurement only (feature
//! `unit-timing`, off by default and never enabled by any workspace member;
//! with it off this module and its one call site in `pinned_route` are
//! compiled out entirely).
//!
//! When `QCORE_UNIT_TIMING` names a file, each pinned-route unit analysis
//! appends one JSON line: the unit's source path, the analysing thread's
//! kernel id (`/proc/thread-self`, the id a debugger's wall-clock sampler
//! records) and its start and end as Unix seconds. The lines are written as
//! units finish (analysis order, not unit order); nothing the route reports
//! reads them. A failed write is ignored: the measurement never changes the
//! run.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The variable naming the timing file.
pub(crate) const PATH_VARIABLE: &str = "QCORE_UNIT_TIMING";

static WRITE: Mutex<()> = Mutex::new(());

/// One unit's analysis on this thread; recorded when dropped.
pub(crate) struct Span {
    file: String,
    thread: String,
    start: f64,
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64())
}

impl Span {
    /// `None` unless the timing file is named.
    pub(crate) fn start(file: &Path) -> Option<Self> {
        std::env::var_os(PATH_VARIABLE)?;
        let thread = std::fs::read_link("/proc/thread-self")
            .ok()
            .and_then(|link| link.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_default();
        Some(Self {
            file: file.to_string_lossy().into_owned(),
            thread,
            start: now(),
        })
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let end = now();
        let Some(path) = std::env::var_os(PATH_VARIABLE) else {
            return;
        };
        let line = serde_json::json!({
            "file": self.file,
            "thread": self.thread,
            "start": self.start,
            "end": end,
        });
        let _guard = WRITE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(mut out) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(out, "{line}");
        }
    }
}
