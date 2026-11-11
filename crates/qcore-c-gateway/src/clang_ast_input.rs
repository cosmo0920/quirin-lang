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

//! Disposable seekable Clang transport, never an evidence cache.
use std::fs::{File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const MEMORY_LIMIT: usize = 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(crate) enum AstInput {
    Memory(Cursor<Vec<u8>>),
    Spool(Spool),
}

pub(crate) struct Spool {
    file: Option<File>,
    read_position: u64,
    advised_position: u64,
    path: PathBuf,
    linked: bool,
}

impl Drop for Spool {
    fn drop(&mut self) {
        drop(self.file.take());
        if self.linked {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

impl AstInput {
    pub(crate) fn capture(mut input: impl Read) -> io::Result<Self> {
        let mut memory = Vec::new();
        let mut chunk = [0; 64 * 1024];
        loop {
            let count = match input.read(&mut chunk) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => result?,
            };
            if count == 0 {
                return Ok(Self::Memory(Cursor::new(memory)));
            }
            if memory.len() + count <= MEMORY_LIMIT {
                memory.extend_from_slice(&chunk[..count]);
                continue;
            }
            let mut spool = Spool::create()?;
            spool.file.as_mut().unwrap().write_all(&memory)?;
            drop(memory);
            spool.file.as_mut().unwrap().write_all(&chunk[..count])?;
            io::copy(&mut input, spool.file.as_mut().unwrap())?;
            // Flush dirty pages before giving the OS permission to discard them.
            // On tmpfs this cannot release backing storage; the measurement
            // must disclose the selected temporary filesystem.
            spool.file.as_ref().unwrap().sync_data()?;
            spool.discard_cache(0, 0);
            spool.file.as_mut().unwrap().seek(SeekFrom::Start(0))?;
            return Ok(Self::Spool(spool));
        }
    }
}

impl Spool {
    fn discard_cache(&self, offset: u64, length: u64) {
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        {
            use std::os::fd::AsRawFd;
            unsafe extern "C" {
                fn posix_fadvise(
                    fd: std::ffi::c_int,
                    offset: i64,
                    length: i64,
                    advice: std::ffi::c_int,
                ) -> std::ffi::c_int;
            }
            // DONTNEED is advisory only: never a memory bound or evidence.
            // These offsets come from this file's reads, within i64 file size.
            if let (Ok(offset), Ok(length)) = (i64::try_from(offset), i64::try_from(length)) {
                unsafe {
                    posix_fadvise(self.file.as_ref().unwrap().as_raw_fd(), offset, length, 4);
                }
            }
        }
        #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
        let _ = (offset, length);
    }

    fn create() -> io::Result<Self> {
        // Honour TMPDIR through the standard library. A tmpfs spool still
        // counts against the cgroup; it is not a claim of disk-backed paging.
        loop {
            let path = std::env::temp_dir().join(format!(
                "quirin-clang-{}-{}.qc",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.read(true).write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                // FILE_FLAG_DELETE_ON_CLOSE also covers process termination.
                options.custom_flags(0x04000000);
            }
            match options.open(&path) {
                Ok(file) => {
                    #[allow(unused_mut)]
                    let mut spool = Self {
                        file: Some(file),
                        read_position: 0,
                        advised_position: 0,
                        path,
                        linked: true,
                    };
                    // An open unlinked file is reclaimed even on process death.
                    #[cfg(unix)]
                    {
                        std::fs::remove_file(&spool.path)?;
                        spool.linked = false;
                    }
                    #[cfg(windows)]
                    {
                        spool.linked = false;
                    } // The handle owns delete-on-close.
                    return Ok(spool);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }
}

impl Read for AstInput {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Memory(cursor) => cursor.read(buf),
            Self::Spool(spool) => {
                let count = spool.file.as_mut().unwrap().read(buf)?;
                spool.read_position += count as u64;
                if spool.read_position.saturating_sub(spool.advised_position) >= MEMORY_LIMIT as u64
                    || count == 0
                {
                    spool.discard_cache(
                        spool.advised_position,
                        spool.read_position.saturating_sub(spool.advised_position),
                    );
                    spool.advised_position = spool.read_position;
                }
                Ok(count)
            }
        }
    }
}

impl Seek for AstInput {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        match self {
            Self::Memory(cursor) => cursor.seek(position),
            Self::Spool(spool) => {
                let position = spool.file.as_mut().unwrap().seek(position)?;
                spool.read_position = position;
                spool.advised_position = position;
                Ok(position)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_keeps_scalar_duplicate_key_and_invalid_text_behavior() {
        let texts: &[&[u8]] = &[
            br#"{"inner":[],"inner":null,"n":1.25e-3,"s":"[\\\"{"}"#,
            br#"[null,true,false,1,-1,18446744073709551616]"#,
            br#"{"x":"\uD800"}"#,
            b"[1,]",
            b"{",
            b"null false",
            b"\"\xff\"",
            b"1e9999",
        ];
        for bytes in texts {
            let before = crate::deep_unit_ast::from_slice::<serde_json::Value>(bytes);
            let after =
                crate::deep_unit_ast::from_reader::<serde_json::Value>(&mut Cursor::new(bytes));
            match (before, after) {
                (Ok(before), Ok(after)) => assert_eq!(before, after),
                (Err(before), Err(after)) => assert_eq!(before.to_string(), after.to_string()),
                result => panic!("reader differs on {bytes:?}: {result:?}"),
            }
        }
    }

    #[test]
    fn large_spool_round_trip_and_cleanup() {
        let bytes: Vec<_> = (0..3 * MEMORY_LIMIT + 17)
            .map(|i| (i % 251) as u8)
            .collect();
        let mut input = AstInput::capture(Cursor::new(&bytes)).unwrap();
        let path = match &input {
            AstInput::Spool(s) => s.path.clone(),
            _ => panic!("large output must spool"),
        };
        for _ in 0..2 {
            input.rewind().unwrap();
            if let AstInput::Spool(spool) = &input {
                assert_eq!(spool.read_position, 0);
                assert_eq!(spool.advised_position, 0);
            }
            let mut read = Vec::new();
            input.read_to_end(&mut read).unwrap();
            assert_eq!(read, bytes);
        }
        drop(input);
        assert!(!path.exists());
    }

    #[test]
    fn spooled_json_preserves_values_depth_errors_and_malformed_suffixes() {
        for depth in [0, 1, 126, 127, 128, 255, 256, 257] {
            for suffix in ["", " trailing", "{}"] {
                let bytes = format!(
                    "{}\"{}\"{}{}",
                    "[".repeat(depth),
                    "x".repeat(MEMORY_LIMIT + 1),
                    "]".repeat(depth),
                    suffix
                )
                .into_bytes();
                let before = crate::deep_unit_ast::from_slice::<serde_json::Value>(&bytes);
                let mut input = AstInput::capture(Cursor::new(&bytes)).unwrap();
                let after = crate::deep_unit_ast::from_reader::<serde_json::Value>(&mut input);
                match (before, after) {
                    (Ok(before), Ok(after)) => assert_eq!(before, after),
                    (Err(before), Err(after)) => assert_eq!(before.to_string(), after.to_string()),
                    _ => panic!("reader disagrees at depth {depth}, suffix {suffix}"),
                }
            }
        }
    }
}
