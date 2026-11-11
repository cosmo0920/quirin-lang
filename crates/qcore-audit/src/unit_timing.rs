// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME**, measurement only (feature
//! `unit-timing`, off by default and never enabled by any workspace member;
//! with it off this module and its one call site in `pinned_route` are
//! compiled out entirely). Linux only (it reads `/proc`).
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
            .and_then(|link| {
                link.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        Some(Self {
            file: file.to_string_lossy().into_owned(),
            thread,
            start: now(),
        })
    }
}

/// The analysing thread's stack as the C library reports it
/// (`pthread_getattr_np`: its usable size) and the deepest page of it ever
/// touched, read from `/proc/self/pagemap` (a page is touched when it is
/// present or swapped). Pages stay mapped once touched, so the depth from the
/// stack's top to the lowest touched page is the thread's high-water mark so
/// far, in KiB (an over-estimate when the C library hands the thread a stack an
/// earlier thread already used). `None` when either read fails.
fn stack_touched() -> Option<serde_json::Value> {
    use std::io::{Read, Seek, SeekFrom};
    let (low, size) = unsafe {
        let mut attr = std::mem::MaybeUninit::<libc::pthread_attr_t>::uninit();
        if libc::pthread_getattr_np(libc::pthread_self(), attr.as_mut_ptr()) != 0 {
            return None;
        }
        let mut attr = attr.assume_init();
        let mut address = std::ptr::null_mut();
        let mut size = 0usize;
        let got = libc::pthread_attr_getstack(&attr, &mut address, &mut size);
        libc::pthread_attr_destroy(&mut attr);
        if got != 0 {
            return None;
        }
        (address as usize, size)
    };
    const PAGE: usize = 4096;
    let pages = size / PAGE;
    let mut map = std::fs::File::open("/proc/self/pagemap").ok()?;
    map.seek(SeekFrom::Start(((low / PAGE) * 8) as u64)).ok()?;
    let mut entries = vec![0u8; pages * 8];
    map.read_exact(&mut entries).ok()?;
    // The stack grows down: the first touched page from the low end is the
    // deepest point reached.
    let lowest = entries.as_chunks::<8>().0.iter().position(|entry| {
        let bits = u64::from_le_bytes(*entry);
        bits & (0b11 << 62) != 0
    })?;
    Some(serde_json::json!({
        "stack_kib": size / 1024,
        "touched_kib": (pages - lowest) * PAGE / 1024,
    }))
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
            "stack": stack_touched(),
        });
        let _guard = WRITE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(mut out) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(out, "{line}");
        }
    }
}
