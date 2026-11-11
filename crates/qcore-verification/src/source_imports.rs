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

//! Runtime source-bundle resolution. Lookup identity is separate from the
//! caller's path spelling and immutable source snapshot. No capability is
//! transferred by an edge; the driver checks each staged file independently.
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use qcore_frontend::frontend::{
    SemanticPass, SourceImportInventory, SourceModuleMode, inspect_source_imports,
};
use qcore_frontend::lexer::Span;

use crate::driver::{FileDiagnostic, SourceFile, SourceLocation};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceLoading {
    /// Pure virtual filesystem over the supplied snapshots. Never reads disk.
    #[default]
    SnapshotsOnly,
    /// Explicitly permit disk reads for imports absent from the snapshots.
    FileSystem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportLimits {
    pub modules: usize,
    pub depth: usize,
    pub bytes: usize,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            modules: 1024,
            depth: 128,
            bytes: 16 * 1024 * 1024,
        }
    }
}

/// One resolved directive, indexed into `SourceBundleEvidence::sources`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeImportEdge {
    pub importer: usize,
    pub target: usize,
    pub span: Span,
    pub spelling: String,
}

/// Exact retained originals, never the blanked checking projection. This binds
/// the selected text; it is not a filesystem/execution attestation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceBundleEvidence {
    pub roots: Vec<usize>,
    pub sources: Vec<SourceFile>,
    pub imports: Vec<RuntimeImportEdge>,
    /// Dependency-before-importer completion order, checked before publication.
    pub completion_order: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct ResolvedSourceBundle {
    evidence: SourceBundleEvidence,
    inventories: Vec<SourceImportInventory>,
}

impl ResolvedSourceBundle {
    pub fn evidence(&self) -> &SourceBundleEvidence {
        &self.evidence
    }

    pub(crate) fn mode(&self) -> SourceModuleMode {
        let modes = self
            .evidence
            .roots
            .iter()
            .map(|i| self.inventories[*i].mode)
            .collect::<Vec<_>>();
        if modes.contains(&SourceModuleMode::Executable) {
            SourceModuleMode::Executable
        } else if modes.contains(&SourceModuleMode::Library) {
            SourceModuleMode::Library
        } else {
            SourceModuleMode::Contract
        }
    }

    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the mode the file at
    /// `index` of the evidence declared (its header, blanked when staged).
    pub(crate) fn file_mode(&self, index: usize) -> Option<SourceModuleMode> {
        self.inventories.get(index).map(|inventory| inventory.mode)
    }

    pub(crate) fn staged(&self) -> Vec<SourceFile> {
        self.evidence
            .sources
            .iter()
            .zip(&self.inventories)
            .map(|(file, inventory)| {
                let mut bytes = file.source.as_bytes().to_vec();
                for span in inventory
                    .mode_span
                    .iter()
                    .copied()
                    .chain(inventory.runtime_imports.iter().map(|i| i.span))
                {
                    for byte in &mut bytes[span.start..span.end] {
                        if *byte != b'\n' && *byte != b'\r' {
                            *byte = b' ';
                        }
                    }
                }
                SourceFile::new(
                    &file.path,
                    String::from_utf8(bytes).expect("blanking whole token spans preserves UTF-8"),
                )
            })
            .collect()
    }
}

/// All `roots` name supplied snapshots. Other snapshots are an available pool,
/// not implicit roots; unreachable snapshots are not compiled. Root order and
/// source-order depth-first dependency discovery determine the output order.
/// Duplicate supplied lookup identities always refuse, even with equal text.
pub fn resolve_source_bundle(
    roots: &[String],
    snapshots: &[SourceFile],
    loading: SourceLoading,
    limits: ImportLimits,
) -> Result<ResolvedSourceBundle, Vec<FileDiagnostic>> {
    let fallback = snapshots
        .first()
        .cloned()
        .unwrap_or_else(|| SourceFile::new("<input>", ""));
    let fail = |message| {
        vec![import_diagnostic(
            &fallback,
            Span { start: 0, end: 0 },
            "Q014",
            message,
        )]
    };
    if roots.is_empty() {
        return Err(fail("no source-bundle roots were provided".into()));
    }
    if limits.depth > 128 || limits.modules > 1024 || limits.bytes > 16 * 1024 * 1024 {
        return Err(fail("runtime import limits may tighten, but not exceed, depth 128, modules 1024 and 16777216 bytes".into()));
    }
    // A snapshot-only compilation has no ambient working-directory dependency.
    let base = if loading == SourceLoading::FileSystem {
        Some(
            std::env::current_dir()
                .map_err(|e| fail(format!("cannot determine import base directory: {e}")))?,
        )
    } else {
        None
    };
    let mut resolver = Resolver {
        loading,
        limits,
        base,
        supplied: BTreeMap::new(),
        supplied_names: BTreeMap::new(),
        indices: BTreeMap::new(),
        active: BTreeSet::new(),
        completed: BTreeSet::new(),
        evidence: SourceBundleEvidence {
            roots: Vec::new(),
            sources: Vec::new(),
            imports: Vec::new(),
            completion_order: Vec::new(),
        },
        inventories: Vec::new(),
        bytes: 0,
    };
    let mut supplied_bytes = 0usize;
    if snapshots.len() > limits.modules {
        return Err(fail(
            "supplied snapshot count exceeds runtime import module bound".into(),
        ));
    }
    for file in snapshots {
        supplied_bytes = supplied_bytes
            .checked_add(file.source.len())
            .ok_or_else(|| fail("supplied snapshot byte count overflow".into()))?;
        if supplied_bytes > limits.bytes {
            return Err(fail(
                "supplied snapshots exceed runtime import byte bound".into(),
            ));
        }
        let key = resolver.key(Path::new(&file.path)).map_err(fail)?;
        if let Some(previous) = resolver.supplied.insert(key.clone(), file.clone()) {
            return Err(fail(format!(
                "ambiguous source snapshots `{}` and `{}` have the same lookup identity",
                previous.path, file.path
            )));
        }
        resolver.supplied_names.insert(file.path.clone(), key);
    }
    let mut root_keys = Vec::new();
    for root in roots {
        let key = resolver
            .supplied_names
            .get(root)
            .cloned()
            .ok_or_else(|| fail(format!("root `{root}` is not a supplied snapshot")))?;
        if root_keys.contains(&key) {
            return Err(fail(format!("duplicate source root `{root}`")));
        }
        root_keys.push(key.clone());
        let file = resolver.supplied[&key].clone();
        let index = resolver.insert(key, file, None)?;
        resolver.evidence.roots.push(index);
    }
    // Retain the caller's root order before discovering any dependencies. This
    // preserves the old multi-file ordering when no import is present.
    for key in root_keys {
        resolver.visit(key, 0, None)?;
    }
    // Recheck the finite graph evidence independently of DFS control state.
    let positions = resolver
        .evidence
        .completion_order
        .iter()
        .enumerate()
        .map(|(rank, index)| (*index, rank))
        .collect::<BTreeMap<_, _>>();
    if positions.len() != resolver.evidence.sources.len()
        || resolver.evidence.imports.iter().any(|edge| {
            !matches!((positions.get(&edge.target), positions.get(&edge.importer)), (Some(target), Some(importer)) if target < importer)
        }) {
        return Err(fail("runtime import completion evidence does not describe a closed acyclic bundle".into()));
    }
    Ok(ResolvedSourceBundle {
        evidence: resolver.evidence,
        inventories: resolver.inventories,
    })
}

struct Resolver {
    loading: SourceLoading,
    limits: ImportLimits,
    base: Option<PathBuf>,
    supplied: BTreeMap<PathBuf, SourceFile>,
    supplied_names: BTreeMap<String, PathBuf>,
    indices: BTreeMap<PathBuf, usize>,
    active: BTreeSet<PathBuf>,
    completed: BTreeSet<PathBuf>,
    evidence: SourceBundleEvidence,
    inventories: Vec<SourceImportInventory>,
    bytes: usize,
}

impl Resolver {
    fn key(&self, path: &Path) -> Result<PathBuf, String> {
        if self.loading == SourceLoading::SnapshotsOnly {
            return Ok(virtual_key(path));
        }
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            self.base.as_ref().expect("filesystem base").join(path)
        };
        match std::fs::canonicalize(&absolute) {
            Ok(key) => Ok(key),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Existing parents must be resolved *before* appending the
                // missing leaf. Never collapse a disk `symlink/..` lexically.
                let parent = absolute
                    .parent()
                    .ok_or_else(|| format!("invalid source path `{}`", path.display()))?;
                if let Ok(parent) = std::fs::canonicalize(parent) {
                    return Ok(parent.join(
                        absolute
                            .file_name()
                            .ok_or_else(|| "source path has no filename".to_owned())?,
                    ));
                }
                if absolute.components().any(|c| c == Component::ParentDir) {
                    return Err(format!(
                        "cannot resolve parent identity of `{}`: {error}",
                        path.display()
                    ));
                }
                Ok(virtual_key(&absolute))
            }
            Err(error) => Err(format!(
                "cannot resolve source identity `{}`: {error}",
                path.display()
            )),
        }
    }

    fn insert(
        &mut self,
        key: PathBuf,
        file: SourceFile,
        origin: Option<(usize, Span)>,
    ) -> Result<usize, Vec<FileDiagnostic>> {
        let fail = |code, message| {
            vec![match origin {
                Some((i, span)) => {
                    import_diagnostic(&self.evidence.sources[i], span, code, message)
                }
                None => import_diagnostic(&file, Span { start: 0, end: 0 }, code, message),
            }]
        };
        if self.evidence.sources.len() >= self.limits.modules {
            return Err(fail("Q014", "runtime import module bound exceeded".into()));
        }
        let bytes = self
            .bytes
            .checked_add(file.source.len())
            .ok_or_else(|| fail("Q014", "runtime import byte count overflow".into()))?;
        if bytes > self.limits.bytes {
            return Err(fail("Q014", "runtime import byte bound exceeded".into()));
        }
        let inventory = inspect_source_imports(&file.source)
            .map_err(|e| vec![import_diagnostic(&file, e.span, "Q000", e.message)])?;
        qcore_frontend::source_extension::validate_source_extension(&file.path, inventory.mode)
            .map_err(|m| fail("Q403", m))?;
        if inventory.mode == SourceModuleMode::Contract && !inventory.runtime_imports.is_empty() {
            return Err(vec![import_diagnostic(
                &file,
                inventory.runtime_imports[0].span,
                "Q014",
                "contract modules cannot import runtime source".into(),
            )]);
        }
        let index = self.evidence.sources.len();
        self.indices.insert(key, index);
        self.evidence.sources.push(file);
        self.inventories.push(inventory);
        self.bytes = bytes;
        Ok(index)
    }

    fn visit(
        &mut self,
        key: PathBuf,
        depth: usize,
        origin: Option<(usize, Span)>,
    ) -> Result<(), Vec<FileDiagnostic>> {
        let index = self.indices[&key];
        let (owner, span) = origin.unwrap_or((index, Span { start: 0, end: 0 }));
        let fail = |m| {
            vec![import_diagnostic(
                &self.evidence.sources[owner],
                span,
                "Q014",
                m,
            )]
        };
        if self.active.contains(&key) {
            return Err(fail(format!(
                "runtime import cycle reaches `{}`",
                self.evidence.sources[index].path
            )));
        }
        if origin.is_some() {
            let inventory = &self.inventories[index];
            if inventory.mode == SourceModuleMode::Contract
                || (inventory.mode == SourceModuleMode::Executable && inventory.mode_span.is_some())
            {
                return Err(fail(format!(
                    "runtime dependency `{}` must be unmarked or module library; explicit executable and contract modules cannot be imported",
                    self.evidence.sources[index].path
                )));
            }
        }
        if depth > self.limits.depth {
            return Err(fail("runtime import depth bound exceeded".into()));
        }
        if self.completed.contains(&key) {
            return Ok(());
        }
        self.active.insert(key.clone());
        let imports = self.inventories[index].runtime_imports.clone();
        for import in imports {
            let failure = |m| {
                vec![import_diagnostic(
                    &self.evidence.sources[index],
                    import.span,
                    "Q014",
                    m,
                )]
            };
            let reference = runtime_path(&import.value).map_err(failure)?;
            let parent = Path::new(&self.evidence.sources[index].path)
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let path = parent.join(reference);
            let target_key = self.key(&path).map_err(failure)?;
            let target = if let Some(target) = self.indices.get(&target_key) {
                *target
            } else {
                let file = if let Some(file) = self.supplied.get(&target_key) {
                    file.clone()
                } else if self.loading == SourceLoading::FileSystem {
                    let mut source = String::new();
                    let remaining = self.limits.bytes.saturating_sub(self.bytes);
                    let handle = std::fs::File::open(&path).map_err(|e| {
                        failure(format!(
                            "cannot load runtime import `{}`: {e}",
                            path.display()
                        ))
                    })?;
                    if !handle
                        .metadata()
                        .map_err(|e| failure(format!("cannot inspect runtime import: {e}")))?
                        .is_file()
                    {
                        return Err(failure(format!(
                            "runtime import `{}` is not a regular file",
                            path.display()
                        )));
                    }
                    handle
                        .take(remaining as u64 + 1)
                        .read_to_string(&mut source)
                        .map_err(|e| {
                            failure(format!(
                                "cannot read UTF-8 runtime import `{}`: {e}",
                                path.display()
                            ))
                        })?;
                    if source.len() > remaining {
                        return Err(failure("runtime import byte bound exceeded".into()));
                    }
                    if self.key(&path).map_err(failure)? != target_key {
                        return Err(failure(
                            "runtime import identity changed while loading".into(),
                        ));
                    }
                    SourceFile::new(
                        path.to_str()
                            .ok_or_else(|| failure("runtime import path is not UTF-8".into()))?,
                        source,
                    )
                } else {
                    return Err(failure(format!(
                        "runtime import `{}` is absent from supplied snapshots (filesystem loading is disabled)",
                        path.display()
                    )));
                };
                self.insert(target_key.clone(), file, Some((index, import.span)))?
            };
            self.evidence.imports.push(RuntimeImportEdge {
                importer: index,
                target,
                span: import.span,
                spelling: import.value,
            });
            self.visit(target_key, depth + 1, Some((index, import.span)))?;
        }
        self.active.remove(&key);
        self.completed.insert(key);
        self.evidence.completion_order.push(index);
        Ok(())
    }
}

fn runtime_path(spelling: &str) -> Result<PathBuf, String> {
    let path = Path::new(spelling);
    // Backslashes and escapes already refuse in the structural inspector;
    // reject Windows drive prefixes consistently on Unix hosts too.
    if path.is_absolute()
        || spelling.contains(':')
        || spelling.ends_with('/')
        || path.file_name().is_none()
    {
        return Err(format!(
            "runtime import `{spelling}` must name a relative .qr file"
        ));
    }
    match path.extension().and_then(|s| s.to_str()) {
        None => Ok(PathBuf::from(format!("{spelling}.qr"))),
        Some("qr") => Ok(path.to_owned()),
        _ => Err(format!(
            "runtime import `{spelling}` must use .qr or omit its extension"
        )),
    }
}

/// Virtual paths are lexical because no disk identity is consulted. Relative
/// and absolute virtual roots stay distinct, and leading parent steps remain.
fn virtual_key(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir
                if matches!(out.components().next_back(), Some(Component::Normal(_))) =>
            {
                out.pop();
            }
            Component::ParentDir if out.has_root() && out.parent().is_none() => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub(crate) fn import_diagnostic(
    file: &SourceFile,
    span: Span,
    code: &str,
    message: String,
) -> FileDiagnostic {
    let offset = span.start.min(file.source.len());
    let before = &file.source[..offset];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    FileDiagnostic {
        code: code.into(),
        pass: SemanticPass::NameResolution,
        location: SourceLocation {
            path: file.path.clone(),
            line: before.bytes().filter(|b| *b == b'\n').count() + 1,
            column: before[line_start..].chars().count() + 1,
            line_source: file.source[line_start..]
                .lines()
                .next()
                .unwrap_or("")
                .into(),
        },
        message,
    }
}
