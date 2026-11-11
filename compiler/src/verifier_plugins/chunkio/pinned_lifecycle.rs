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

//! Lifecycle compositions derived from the selected functions' **own pinned**
//! ChunkIO translation units.
//!
//! Card `W43-CHUNK-SOURCE-PROFILE`. W42 composed the ChunkIO teardown evidence
//! over committed distilled C fixtures, so no published lifecycle row carried a
//! composition taken from its own pinned source
//! (`CHUNK-LIFECYCLE-COMPOSITION-FIXTURE-BOUND`). This module runs the same
//! rechecked importers the pinned audit runs — the frame helper, the memory
//! backend, the list unlink, the counters, the null/memory `cio_chunk_close`
//! composition and the chunk-ring traversal — over the pinned units themselves,
//! and turns what they accept into lifecycle rows.
//!
//! Two sources of units are accepted, and both are bound to the pinned bytes:
//!
//! * [`load_vendored_units`] reads the verbatim copies under
//!   `compiler/tests/fixtures/pinned/fluent-bit/`, refuses any file whose
//!   digest differs from the manifest, and refuses a `cio_chunk.c` whose digest
//!   differs from the one `contract/profiles/fluent-bit/profile.json` pins
//!   independently. This is what the standing gate runs.
//! * [`PinnedChunkUnits::from_options`] takes units built from the pinned
//!   compilation database itself; the integration test uses it, when the
//!   pinned checkout is supplied, to measure that the vendored route and the
//!   pinned build derive byte-identical rows.
//!
//! A role is never read from a name. A row is classified only when an importer
//! accepts that function's complete body against its reference, and every
//! source path the importer does not compose is published as an uncovered path
//! with its own reason.

use std::collections::BTreeMap;
use std::path::Path;

use qcore_verification::chunk_lifecycle_coverage::{
    CompositionSource, LifecycleRole, ModelComposition, SourceDerivation, UncoveredPath,
};
use qcore_verification::memory_close_composition::{
    MemoryCloseDependencies, read_memory_close_source,
};

use super::lifecycle_coverage::{
    ClassifiedFunction, bound_program_digest, memory_close_program, stream_ring_close_program,
};
use super::traversal::{TraversalKind, check_traversal_source};
use super::{PINNED_FLUENT_BIT_REVISION, import_chunk_frame_helper, memory_close, teardown_source};
use crate::legacy_c::{C11TranslationUnit, CCompilerDialect, CSourceInput, LegacyFunctionScope};

/// The library label the pinned audit gives `cio_chunk.c`'s functions.
pub const CHUNKIO_LIBRARY: &str = "chunkio";
/// The selected unit whose functions this route classifies.
pub const CHUNK_UNIT: &str = "lib/chunkio/src/cio_chunk.c";
/// Dependency units: the memory backend and the frame helper.
pub const MEMFS_UNIT: &str = "lib/chunkio/src/cio_memfs.c";
pub const ERROR_UNIT: &str = "lib/chunkio/src/cio_error.c";

/// How many members the ring-close composition instantiates. Two, so the
/// traversal's advance over a saved next pointer is exercised, not just a
/// single visit; the kernel's identity profile bounds it either way.
pub const RING_MEMBERS: u32 = 2;

/// One pinned translation unit with the preprocessing configuration the pinned
/// compilation database records for it.
#[derive(Clone, Debug)]
pub struct PinnedUnit {
    pub path: String,
    pub source: String,
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    pub dialect: CCompilerDialect,
}

impl PinnedUnit {
    pub fn sha256(&self) -> String {
        qcore_schemas::sha256_hex(self.source.as_bytes())
    }

    /// The unit exactly as the pinned audit hands it to the gateway: the
    /// compatibility projection's archived standard-input transport and the
    /// main-source definition scope.
    pub fn unit(&self) -> C11TranslationUnit<'_> {
        C11TranslationUnit {
            source: &self.source,
            include_paths: self.include_paths.clone(),
            definitions: self.definitions.clone(),
            preprocessor_options: Vec::new(),
            dialect: self.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: CSourceInput::HistoricalStdin,
        }
    }
}

/// The three pinned units the lifecycle route imports.
#[derive(Clone, Debug)]
pub struct PinnedChunkUnits {
    chunk: PinnedUnit,
    memfs: PinnedUnit,
    error: PinnedUnit,
}

impl PinnedChunkUnits {
    /// Bind caller-built units. The chunk unit must carry the profile-pinned
    /// `cio_chunk.c` digest; the other two are dependencies whose own
    /// certificates the importers recheck.
    pub fn from_options(
        chunk: PinnedUnit,
        memfs: PinnedUnit,
        error: PinnedUnit,
    ) -> Result<Self, String> {
        for (unit, expected) in [
            (&chunk, CHUNK_UNIT),
            (&memfs, MEMFS_UNIT),
            (&error, ERROR_UNIT),
        ] {
            if unit.path != expected {
                return Err(format!(
                    "expected pinned unit `{expected}`, got `{}`",
                    unit.path
                ));
            }
        }
        let pinned = profile_pinned_digest(CHUNK_UNIT)?;
        if chunk.sha256() != pinned {
            return Err(format!(
                "`{CHUNK_UNIT}` is not the profile-pinned unit: {} != {pinned}",
                chunk.sha256()
            ));
        }
        Ok(Self {
            chunk,
            memfs,
            error,
        })
    }

    pub fn chunk(&self) -> &PinnedUnit {
        &self.chunk
    }

    /// Every unit's digest, for the published derivation.
    pub fn digests(&self) -> BTreeMap<String, String> {
        [&self.chunk, &self.memfs, &self.error]
            .into_iter()
            .map(|unit| (unit.path.clone(), unit.sha256()))
            .collect()
    }
}

/// The independently pinned digest of an audited ChunkIO file.
pub fn profile_pinned_digest(path: &str) -> Result<String, String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    let library = profile
        .library(CHUNKIO_LIBRARY)
        .ok_or("the fluent-bit profile declares no chunkio library")?;
    let pin = profile
        .pin_for(CHUNKIO_LIBRARY)
        .ok_or("the chunkio library has no pin")?;
    if pin.git_commit != PINNED_FLUENT_BIT_REVISION {
        return Err("the chunkio pin is not the pinned Fluent Bit revision".into());
    }
    library
        .audited_files
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.sha256.clone())
        .ok_or_else(|| format!("`{path}` is not an audited file of the pinned profile"))
}

#[derive(serde::Deserialize)]
struct Manifest {
    source_revision: String,
    files: BTreeMap<String, ManifestFile>,
    units: BTreeMap<String, ManifestUnit>,
}

#[derive(serde::Deserialize)]
struct ManifestFile {
    origin: String,
    sha256: String,
}

#[derive(serde::Deserialize)]
struct ManifestUnit {
    definitions: Vec<String>,
    include_roots: Vec<String>,
    dialect: String,
}

/// Load the vendored verbatim copies of the pinned units.
///
/// Every file the manifest lists is read and digest-checked, every file under
/// the directory must be listed, and each unit's include roots must name
/// directories inside the vendored tree. A single differing byte is refused.
pub fn load_vendored_units(root: &Path) -> Result<PinnedChunkUnits, String> {
    let text = std::fs::read_to_string(root.join("manifest.json"))
        .map_err(|error| format!("vendored manifest unavailable: {error}"))?;
    let manifest: Manifest =
        serde_json::from_str(&text).map_err(|error| format!("vendored manifest: {error}"))?;
    if manifest.source_revision != PINNED_FLUENT_BIT_REVISION {
        return Err("vendored units are not at the pinned revision".into());
    }
    for (path, file) in &manifest.files {
        if !matches!(
            file.origin.as_str(),
            "pinned-source" | "pinned-build-generated"
        ) {
            return Err(format!("vendored `{path}` has an unknown origin"));
        }
        let expected_prefix = if file.origin == "pinned-source" {
            "source/"
        } else {
            "build/"
        };
        if !path.starts_with(expected_prefix) || path.contains("..") {
            return Err(format!("vendored `{path}` is outside its origin tree"));
        }
        let bytes = std::fs::read(root.join(path))
            .map_err(|error| format!("vendored `{path}` unavailable: {error}"))?;
        if !matches_checkout_digest(&bytes, &file.sha256) {
            return Err(format!("vendored `{path}` differs from its pinned digest"));
        }
    }
    let mut present = Vec::new();
    collect_files(root, root, &mut present)?;
    for path in present {
        if path != "manifest.json" && !manifest.files.contains_key(&path) {
            return Err(format!("vendored tree carries unlisted file `{path}`"));
        }
    }
    let unit = |path: &str| -> Result<PinnedUnit, String> {
        let recorded = manifest
            .units
            .get(path)
            .ok_or_else(|| format!("vendored manifest names no unit `{path}`"))?;
        let key = format!("source/{path}");
        if !manifest.files.contains_key(&key) {
            return Err(format!("vendored unit `{path}` is not a listed file"));
        }
        let source = std::fs::read_to_string(root.join(&key))
            .map_err(|e| e.to_string())?
            .replace("\r\n", "\n");
        let mut include_paths = Vec::new();
        for label in &recorded.include_roots {
            if !(label.starts_with("source/") || label.starts_with("build/"))
                || label.contains("..")
            {
                return Err(format!(
                    "include root `{label}` is outside the vendored tree"
                ));
            }
            let directory = root.join(label);
            if !directory.is_dir() {
                return Err(format!(
                    "include root `{label}` is not a vendored directory"
                ));
            }
            include_paths.push(directory.to_string_lossy().into_owned());
        }
        let dialect = match recorded.dialect.as_str() {
            "gnu17" => CCompilerDialect::Gnu17,
            other => return Err(format!("unsupported recorded dialect `{other}`")),
        };
        Ok(PinnedUnit {
            path: path.to_owned(),
            source,
            include_paths,
            definitions: recorded.definitions.clone(),
            dialect,
        })
    };
    PinnedChunkUnits::from_options(unit(CHUNK_UNIT)?, unit(MEMFS_UNIT)?, unit(ERROR_UNIT)?)
}

fn matches_checkout_digest(bytes: &[u8], expected: &str) -> bool {
    if qcore_schemas::sha256_hex(bytes) == expected {
        return true;
    }
    if !cfg!(windows) {
        return false;
    }
    let normalized = bytes
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect::<Vec<_>>();
    qcore_schemas::sha256_hex(&normalized.join(&b'\n')) == expected
}

fn collect_files(root: &Path, directory: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else {
            out.push(
                path.strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}

/// The reason every path the close route does not compose carries.
pub const UNCOVERED_CLOSE_PATH: &str = "not the null or memory-backed close path; the \
     filesystem-backed close (conditional nested cleanup with unresolved OS custody) and the \
     unsupported-backend close (opaque backend custody) are not composed by this route";

/// Derive every lifecycle row this route can establish from the pinned units.
///
/// Rows are returned only for functions whose complete body an importer
/// accepted. Every other selected function keeps the pinned projection's own
/// refusal as its fail-closed default in [`super::lifecycle_coverage`].
pub fn derive_pinned_lifecycle(
    units: &PinnedChunkUnits,
) -> Result<Vec<ClassifiedFunction>, String> {
    let revision = PINNED_FLUENT_BIT_REVISION;
    let chunk = units.chunk.unit();
    let memfs = units.memfs.unit();
    let error = units.error.unit();
    let counter = teardown_source::check_counter_sources(&chunk, revision)?
        .into_iter()
        .find(|certificate| certificate.field() == "total_chunks" && certificate.delta() == -1)
        .ok_or("the pinned unit has no total-chunk decrement certificate")?;
    let dependencies = MemoryCloseDependencies {
        reset: import_chunk_frame_helper(&error, revision, "cio_error_reset")?,
        backend: teardown_source::check_memory_backend_source(&memfs, revision)?,
        unlink: teardown_source::check_list_unlink_source(&chunk, revision)?,
        counter,
    };
    let reading =
        read_memory_close_source(&chunk, revision, &dependencies, &memory_close::contracts())?;
    let certificate = reading.certificate;
    let traversal = check_traversal_source(&chunk, revision, TraversalKind::Chunks)?;

    let unit_sha256 = units.chunk.sha256();
    let mut dependency_units = units.digests();
    dependency_units.remove(CHUNK_UNIT);
    let source = |function: &str| CompositionSource {
        library: CHUNKIO_LIBRARY.into(),
        file: "cio_chunk.c".into(),
        function: function.into(),
        source_revision: revision.into(),
        unit_sha256: unit_sha256.clone(),
    };
    let label = |index: usize| -> String {
        match reading
            .entries
            .iter()
            .find(|entry| entry.source_path == index)
        {
            Some(entry) if entry.nonnull => format!("source path {index}: memory-backed chunk"),
            Some(_) => format!("source path {index}: null chunk"),
            None => format!("source path {index}"),
        }
    };
    let checked: Vec<String> = certificate
        .checked_paths()
        .iter()
        .map(|&i| label(i))
        .collect();
    let uncovered: Vec<UncoveredPath> = certificate
        .uncovered_paths()
        .iter()
        .map(|&index| UncoveredPath {
            path: label(index),
            reason: UNCOVERED_CLOSE_PATH.into(),
        })
        .collect();

    // `cio_chunk_close(struct cio_chunk *, int)`: the memory path must reach the
    // same legs whatever the `delete` argument is.
    let close_with = |delete: bool| -> Result<ModelComposition, String> {
        let program = memory_close_program(&certificate, delete)?;
        let digest = bound_program_digest(&unit_sha256, "cio_chunk_close", &program);
        ModelComposition::prefix(digest, &program)
    };
    let close = close_with(false)?;
    let close_deleting = close_with(true)?;
    if close.discharged() != close_deleting.discharged()
        || close.retained_live() != close_deleting.retained_live()
    {
        return Err("the memory close path depends on its delete argument".into());
    }

    let ring_program = stream_ring_close_program(&traversal, &certificate, RING_MEMBERS)?;
    let ring = ModelComposition::prefix(
        bound_program_digest(&unit_sha256, "cio_chunk_close_stream", &ring_program),
        &ring_program,
    )?;
    let ring_checked = vec![format!(
        "every member of the stream's chunk ring, visited once through a saved next pointer and \
         closed through cio_chunk_close's checked paths ({})",
        checked.join("; ")
    )];
    let ring_uncovered = uncovered
        .iter()
        .map(|path| UncoveredPath {
            path: format!("a ring member closed through cio_chunk_close {}", path.path),
            reason: path.reason.clone(),
        })
        .collect();

    Ok(vec![
        ClassifiedFunction {
            function: "cio_chunk_close".into(),
            role: LifecycleRole::DestructionDependent,
            model: Some(close),
            delete_only: None,
            error_exits_modeled: false,
            wrapper_obligations_discharged: false,
            derivation: Some(SourceDerivation {
                source: source("cio_chunk_close"),
                dependency_units: dependency_units.clone(),
                checked_paths: checked,
                uncovered_paths: uncovered,
            }),
        },
        ClassifiedFunction {
            function: "cio_chunk_close_stream".into(),
            role: LifecycleRole::DestructionDependent,
            model: Some(ring),
            delete_only: None,
            error_exits_modeled: false,
            wrapper_obligations_discharged: false,
            derivation: Some(SourceDerivation {
                source: source("cio_chunk_close_stream"),
                dependency_units,
                checked_paths: ring_checked,
                uncovered_paths: ring_uncovered,
            }),
        },
    ])
}

#[cfg(test)]
#[path = "../../../tests/unit/pinned_lifecycle.rs"]
mod tests;
