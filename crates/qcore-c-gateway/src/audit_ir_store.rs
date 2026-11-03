//! Bounded, content-addressed CFG artifacts. These are not certificates.
//!
//! Reading a CFG requires independently held construction context and a fresh
//! source-built CFG. Full equality is mandatory: this slice does not skip any
//! source/typed reading or admit persisted analysis results as trusted input.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::legacy_c::{LegacyCfg, LegacyInstruction};

pub const FORMAT: &str = "quirin-audit-ir-cfg/1";
pub const MAX_OBJECT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_INSTRUCTION_DEPTH: usize = 32;

/// A validated lowercase SHA-256. A digest alone conveys no source authority.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ObjectId(String);

impl ObjectId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ObjectId {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("IR object ID is not a lowercase SHA-256".into());
        }
        Ok(Self(value))
    }
}

impl From<ObjectId> for String {
    fn from(value: ObjectId) -> Self {
        value.0
    }
}

/// Expected references must come from the caller's actual unit/construction,
/// never be copied from a presented fragment to authenticate that fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentContext {
    pub unit: ObjectId,
    pub construction: ObjectId,
}

/// Actual construction inputs, shared by reference between body variants.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructionContext {
    pub format: String,
    pub function: String,
    pub tokens: ObjectId,
    pub contracts: ObjectId,
    pub admit_derived_returns: bool,
    pub own: Option<crate::legacy_c::ResourceFunctionContract>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CfgObject {
    format: String,
    requires_fresh_lowering: bool,
    context: FragmentContext,
    cfg: LegacyCfg,
}

/// A bounded store. Its generic objects are data, never proof or trust labels.
#[derive(Clone, Debug)]
pub struct FragmentStore {
    root: PathBuf,
    max_bytes: usize,
}

impl FragmentStore {
    pub fn new(root: impl Into<PathBuf>, max_bytes: usize) -> Result<Self, String> {
        if max_bytes == 0 || max_bytes > MAX_OBJECT_BYTES {
            return Err("IR object limit must be between 1 and 64 MiB".into());
        }
        Ok(Self {
            root: root.into(),
            max_bytes,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Encode with a hard output limit and publish without exposing
    /// partially written final objects. Existing objects are checked exactly.
    pub fn write_object<T: Serialize>(&self, value: &T) -> Result<ObjectId, String> {
        let bytes = self.encode(value)?;
        let id = ObjectId(qcore_schemas::sha256_hex(&bytes));
        let directory = self.root.join("objects");
        fs::create_dir_all(&directory).map_err(io_error)?;
        self.publish(&directory.join(format!("{}.qc", id.0)), &bytes)?;
        Ok(id)
    }

    /// Decode only bounded, hash-matching, canonical objects. Decoding is not
    /// evidence admission; callers must independently authenticate references.
    pub fn read_object<T: DeserializeOwned + Serialize>(&self, id: &ObjectId) -> Result<T, String> {
        let bytes = self.checked_bytes(id)?;
        let value: T = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if self.encode(&value)? != bytes {
            return Err("IR object is noncanonical or loses fields on decoding".into());
        }
        Ok(value)
    }

    pub fn write_cfg(
        &self,
        context: &FragmentContext,
        cfg: &LegacyCfg,
    ) -> Result<ObjectId, String> {
        check_instruction_depth(cfg)?;
        // Borrow the CFG during encoding; do not clone a complete body.
        #[derive(Serialize)]
        struct BorrowedCfg<'a> {
            format: &'static str,
            requires_fresh_lowering: bool,
            context: &'a FragmentContext,
            cfg: &'a LegacyCfg,
        }
        let id = self.write_object(&BorrowedCfg {
            format: FORMAT,
            requires_fresh_lowering: true,
            context,
            cfg,
        })?;
        self.read_cfg(&id, context, cfg)?;
        Ok(id)
    }

    /// Lossless readback, conditional on fresh source equality. The returned
    /// data grants no certificate authority and is not used to skip lowering.
    pub fn read_cfg(
        &self,
        id: &ObjectId,
        expected: &FragmentContext,
        freshly_built: &LegacyCfg,
    ) -> Result<LegacyCfg, String> {
        let object: CfgObject = self.read_object(id)?;
        check_instruction_depth(&object.cfg)?;
        if object.format != FORMAT || !object.requires_fresh_lowering {
            return Err("unsupported IR format or replay policy".into());
        }
        if &object.context != expected {
            return Err("IR construction context mismatch".into());
        }
        self.check_context(expected, &freshly_built.function)?;
        if &object.cfg != freshly_built {
            return Err("IR body differs from fresh source lowering".into());
        }
        Ok(object.cfg)
    }

    /// Publish a complete unit index under both its unit and content IDs. No
    /// mutable latest pointer can hide an older, different capture.
    pub(crate) fn publish_unit<T: Serialize + DeserializeOwned>(
        &self,
        unit: &ObjectId,
        value: &T,
    ) -> Result<ObjectId, String> {
        let id = self.write_object(value)?;
        let _: T = self.read_object(&id)?;
        let directory = self.root.join("units").join(unit.as_str());
        fs::create_dir_all(&directory).map_err(io_error)?;
        self.publish(
            &directory.join(format!("{}.qc", id.0)),
            &self.encode(value)?,
        )?;
        Ok(id)
    }

    pub(crate) fn checked_bytes(&self, id: &ObjectId) -> Result<Vec<u8>, String> {
        let bytes = self.read_bytes(&self.root.join("objects").join(format!("{}.qc", id.0)))?;
        if qcore_schemas::sha256_hex(&bytes) != id.0 {
            return Err("IR object digest mismatch".into());
        }
        Ok(bytes)
    }

    pub(crate) fn check_context(
        &self,
        context: &FragmentContext,
        function: &str,
    ) -> Result<(), String> {
        let unit: serde_json::Value = serde_json::from_slice(&self.checked_bytes(&context.unit)?)
            .map_err(|e| e.to_string())?;
        if unit["format"] != FORMAT || unit["requires_fresh_lowering"] != true {
            return Err("unsupported IR unit context".into());
        }
        let construction: ConstructionContext = self.read_object(&context.construction)?;
        if construction.format != FORMAT || construction.function != function {
            return Err("IR construction function/format mismatch".into());
        }
        self.checked_bytes(&construction.tokens)?;
        self.checked_bytes(&construction.contracts)?;
        Ok(())
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, String> {
        struct Bounded {
            bytes: Vec<u8>,
            max: usize,
        }
        impl Write for Bounded {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if bytes.len() > self.max - self.bytes.len() {
                    return Err(std::io::Error::other("IR object exceeds byte limit"));
                }
                // Explicit reservation avoids geometric capacity growth past
                // the limit when a serialization arrives in small chunks.
                let needed = self.bytes.len() + bytes.len();
                if needed > self.bytes.capacity() {
                    let capacity = self
                        .bytes
                        .capacity()
                        .saturating_mul(2)
                        .max(256)
                        .max(needed)
                        .min(self.max);
                    self.bytes.reserve_exact(capacity - self.bytes.len());
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut output = Bounded {
            bytes: Vec::new(),
            max: self.max_bytes,
        };
        serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
        Ok(output.bytes)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        let file = File::open(path).map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file() || metadata.len() > self.max_bytes as u64 {
            return Err("IR object is not a bounded regular file".into());
        }
        let mut bytes = Vec::new();
        file.take(self.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > self.max_bytes {
            return Err("IR object exceeds byte limit".into());
        }
        Ok(bytes)
    }

    fn publish(&self, destination: &Path, bytes: &[u8]) -> Result<(), String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temporary = destination.with_extension(format!(
            "tmp-{}-{}.qc",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(io_error)?;
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _cleanup = Cleanup(temporary.clone());
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        match fs::hard_link(&temporary, destination) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if self.read_bytes(destination)? == bytes {
                    Ok(())
                } else {
                    Err("existing IR object has different bytes".into())
                }
            }
            Err(error) => Err(io_error(error)),
        }
    }
}

fn io_error(error: std::io::Error) -> String {
    format!("IR artifact I/O: {error}")
}

// Preflight recursive instructions iteratively before calling serde. This is
// an artifact-format limit, not an admission change to the C/Clang readers.
fn check_instruction_depth(cfg: &LegacyCfg) -> Result<(), String> {
    use LegacyInstruction::*;
    for node in &cfg.nodes {
        let mut pending = vec![(&node.instruction, 1)];
        while let Some((instruction, depth)) = pending.pop() {
            if depth > MAX_INSTRUCTION_DEPTH {
                return Err("IR instruction nesting exceeds 32".into());
            }
            let mut push = |instruction| pending.push((instruction, depth + 1));
            match instruction {
                ScalarMemberStore {
                    typed_alternative, ..
                }
                | IndirectScalarAccess {
                    typed_alternative, ..
                } => {
                    if let Some(instruction) = typed_alternative {
                        push(instruction.as_ref());
                    }
                }
                ElementStore { call, fallback, .. } => {
                    if let Some(instruction) = call {
                        push(instruction.as_ref());
                    }
                    push(fallback.as_ref());
                }
                ElementRelease { fallback, .. }
                | SelfInitCall { fallback, .. }
                | BranchOnElementNull { fallback, .. }
                | SubObjectStore { fallback, .. } => push(fallback.as_ref()),
                PointerStore {
                    access, fallback, ..
                }
                | TrackedSelfInit {
                    access, fallback, ..
                } => {
                    push(access.as_ref());
                    push(fallback.as_ref());
                }
                Acquire { .. }
                | Alias { .. }
                | Clear { .. }
                | Release { .. }
                | Escape { .. }
                | EscapeIfOwned { .. }
                | ExactFormalReturn { .. }
                | ContractCall { .. }
                | BranchOnNull { .. }
                | BranchOnValidity { .. }
                | BranchOnResult { .. }
                | BranchOnDistinct { .. }
                | CellProtocolStep { .. }
                | OpaqueValue { .. }
                | VaArgPointer { .. }
                | ForgetScalar { .. }
                | CountedLoopHeader { .. }
                | CountedLoopStep { .. }
                | PointerStep { .. }
                | ArgumentReads { .. }
                | AggregateStore { .. }
                | NoOp
                | Unknown { .. } => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/audit_ir_fragments.rs"]
mod tests;
