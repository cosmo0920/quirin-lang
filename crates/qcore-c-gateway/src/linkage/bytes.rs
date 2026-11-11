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

//! Bounds-checked byte access shared by every symbol reader.
//!
//! Every reader in [`super`] is a function of `&[u8]`, so a malformed or
//! truncated artifact must surface as a named [`ReadError`], never as a panic
//! or an out-of-range slice. Offsets and counts read from an artifact are
//! untrusted: each is checked before it is used.

use std::fmt;

/// Why an artifact could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadError {
    /// What was being read (`ELF section header`, `ar member header`, …).
    pub what: String,
    /// Why it could not be.
    pub reason: String,
}

impl ReadError {
    pub fn new(what: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            what: what.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.what, self.reason)
    }
}

impl std::error::Error for ReadError {}

/// Byte order of the integers an artifact stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

/// A checked view of an artifact's bytes.
#[derive(Clone, Copy, Debug)]
pub struct Bytes<'a> {
    data: &'a [u8],
    what: &'static str,
}

impl<'a> Bytes<'a> {
    pub fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, what }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    fn out_of_range(&self, offset: u64, length: u64) -> ReadError {
        ReadError::new(
            self.what,
            format!(
                "{length} byte(s) at offset {offset} lie outside the {} byte(s) read",
                self.data.len()
            ),
        )
    }

    /// `length` bytes at `offset`.
    pub fn slice(&self, offset: u64, length: u64) -> Result<&'a [u8], ReadError> {
        let start = usize::try_from(offset).map_err(|_| self.out_of_range(offset, length))?;
        let length_usize =
            usize::try_from(length).map_err(|_| self.out_of_range(offset, length))?;
        let end = start
            .checked_add(length_usize)
            .ok_or_else(|| self.out_of_range(offset, length))?;
        self.data
            .get(start..end)
            .ok_or_else(|| self.out_of_range(offset, length))
    }

    pub fn u8(&self, offset: u64) -> Result<u8, ReadError> {
        Ok(self.slice(offset, 1)?[0])
    }

    pub fn u16(&self, offset: u64, endian: Endian) -> Result<u16, ReadError> {
        let bytes: [u8; 2] = self.slice(offset, 2)?.try_into().expect("two bytes");
        Ok(match endian {
            Endian::Little => u16::from_le_bytes(bytes),
            Endian::Big => u16::from_be_bytes(bytes),
        })
    }

    pub fn u32(&self, offset: u64, endian: Endian) -> Result<u32, ReadError> {
        let bytes: [u8; 4] = self.slice(offset, 4)?.try_into().expect("four bytes");
        Ok(match endian {
            Endian::Little => u32::from_le_bytes(bytes),
            Endian::Big => u32::from_be_bytes(bytes),
        })
    }

    pub fn u64(&self, offset: u64, endian: Endian) -> Result<u64, ReadError> {
        let bytes: [u8; 8] = self.slice(offset, 8)?.try_into().expect("eight bytes");
        Ok(match endian {
            Endian::Little => u64::from_le_bytes(bytes),
            Endian::Big => u64::from_be_bytes(bytes),
        })
    }

    /// A NUL-terminated string starting at `offset`, which must be UTF-8.
    pub fn c_string(&self, offset: u64) -> Result<&'a str, ReadError> {
        let start = usize::try_from(offset).map_err(|_| self.out_of_range(offset, 1))?;
        let rest = self
            .data
            .get(start..)
            .ok_or_else(|| self.out_of_range(offset, 1))?;
        let end = rest.iter().position(|byte| *byte == 0).ok_or_else(|| {
            ReadError::new(
                self.what,
                format!("the string at offset {offset} is not NUL-terminated"),
            )
        })?;
        std::str::from_utf8(&rest[..end]).map_err(|_| {
            ReadError::new(
                self.what,
                format!("the string at offset {offset} is not UTF-8"),
            )
        })
    }

    /// An unsigned LEB128 at `*offset`, advancing it.
    pub fn uleb128(&self, offset: &mut u64) -> Result<u64, ReadError> {
        let mut value: u64 = 0;
        let mut shift = 0u32;
        loop {
            let byte = self.u8(*offset)?;
            *offset += 1;
            if shift >= 64 {
                return Err(ReadError::new(
                    self.what,
                    "a ULEB128 value does not fit in 64 bits",
                ));
            }
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    }
}

/// A multiplication of an untrusted count by an entry size, checked.
pub fn checked_extent(what: &str, count: u64, size: u64) -> Result<u64, ReadError> {
    count
        .checked_mul(size)
        .ok_or_else(|| ReadError::new(what, format!("{count} entries of {size} bytes overflow")))
}
