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

//! A schemas-only proposal producer for two bounded ChunkIO teardown families.
//!
//! This process reads the supplied typed paths independently and computes what
//! they do: for the unlink family, the link-cell writes and helper calls in
//! each neighbor-alias class; for the destructor family, which declared owned
//! fields are released, in which order, on which path, in every realizable
//! nullable state. It evaluates no contract and receives no expected result.
//! The receiver recomputes the same reading from its own import and evaluates
//! the pinned contract itself before anything is admitted.
use qcore_schemas::SemanticWitness;
use qcore_schemas::plugin::*;
use qcore_schemas::teardown_summary::{
    FieldDestructorRecord, FieldReleaseCaseRecord, LinkWriteRecord, ListUnlinkRecord,
    TeardownContractWitness, TeardownExpressionRecord, TeardownFamilyInput, TeardownFunctionRecord,
    TeardownOperationRecord, TeardownProposalRecord, TeardownSourceInput, TeardownValueRecord,
    UnlinkCaseRecord,
};
use std::collections::{BTreeMap, BTreeSet};

/// Bounds applied to a supplied projection before any structural reading.
/// These apply to direct Rust callers as well as to transported JSON.
const MAX_PATHS: usize = 64;
const MAX_OPERATIONS: usize = 256;
const MAX_FIELDS: usize = 8;

/// The unlink replay's fixed node numbering: the entry is node 1, its previous
/// neighbor node 2, and its next neighbor node 3 unless the two alias.
const ENTRY: u32 = 1;
const PREVIOUS: u32 = 2;

fn bounded(function: &TeardownFunctionRecord) -> bool {
    !function.function.is_empty()
        && !function.source_revision.is_empty()
        && !function.signature.is_empty()
        && !function.paths.is_empty()
        && function.paths.len() <= MAX_PATHS
        && function
            .paths
            .iter()
            .all(|path| !path.operations.is_empty() && path.operations.len() <= MAX_OPERATIONS)
}

// ---------------------------------------------------------------------------
// Intrusive-list unlink
// ---------------------------------------------------------------------------

fn null_literal(value: &TeardownValueRecord) -> bool {
    match &value.expression {
        TeardownExpressionRecord::Integer { value } => value == "0",
        TeardownExpressionRecord::Cast { cast, value: inner } => {
            cast == "NullToPointer" && value.c_type.ends_with('*') && null_literal(inner)
        }
        _ => false,
    }
}

struct LinkReplay<'a> {
    memory: BTreeMap<(u32, String), Option<u32>>,
    writes: Vec<LinkWriteRecord>,
    helper_calls: u32,
    node_c_type: &'a str,
}

impl LinkReplay<'_> {
    fn value(
        &self,
        value: &TeardownValueRecord,
        parameters: &[Option<u32>],
        symbols: &BTreeMap<u32, Option<u32>>,
    ) -> Option<Option<u32>> {
        match &value.expression {
            TeardownExpressionRecord::Parameter { index } if value.c_type == self.node_c_type => {
                parameters.get(*index as usize).copied()
            }
            TeardownExpressionRecord::Symbol { operation } if value.c_type == self.node_c_type => {
                symbols.get(operation).copied()
            }
            TeardownExpressionRecord::Cast { cast, value: inner }
                if matches!(cast.as_str(), "BitCast" | "NoOp")
                    && value.c_type.ends_with('*')
                    && inner.c_type.ends_with('*') =>
            {
                self.value(inner, parameters, symbols)
            }
            TeardownExpressionRecord::Cast { cast, value: inner }
                if cast == "NullToPointer"
                    && value.c_type.ends_with('*')
                    && null_literal(inner) =>
            {
                Some(None)
            }
            _ => None,
        }
    }

    fn place(
        &self,
        place: &TeardownValueRecord,
        parameters: &[Option<u32>],
        symbols: &BTreeMap<u32, Option<u32>>,
    ) -> Option<(u32, String)> {
        let TeardownExpressionRecord::Field { base, name } = &place.expression else {
            return None;
        };
        if place.c_type != self.node_c_type || !matches!(name.as_str(), "prev" | "next") {
            return None;
        }
        let cell = (self.value(base, parameters, symbols)??, name.clone());
        self.memory.contains_key(&cell).then_some(cell)
    }

    fn run(
        &mut self,
        operations: &[TeardownOperationRecord],
        parameters: &[Option<u32>],
        helper: Option<&TeardownFunctionRecord>,
    ) -> Option<()> {
        let mut symbols: BTreeMap<u32, Option<u32>> = BTreeMap::new();
        let Some((TeardownOperationRecord::Return, body)) = operations.split_last() else {
            return None;
        };
        for (index, operation) in body.iter().enumerate() {
            let index = u32::try_from(index).ok()?;
            match operation {
                TeardownOperationRecord::Read { place } => {
                    let cell = self.place(place, parameters, &symbols)?;
                    symbols.insert(index, self.memory[&cell]);
                }
                TeardownOperationRecord::Write { place, value } => {
                    let cell = self.place(place, parameters, &symbols)?;
                    let value = self.value(value, parameters, &symbols)?;
                    self.memory.insert(cell.clone(), value);
                    self.writes.push(LinkWriteRecord {
                        node: cell.0,
                        field: cell.1,
                        value,
                    });
                }
                TeardownOperationRecord::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    let helper = helper?;
                    if *function != helper.function
                        || *signature != helper.signature
                        || result_type != "void"
                        || arguments.len() != 2
                        || parameters != [Some(ENTRY)]
                    {
                        return None;
                    }
                    let arguments = arguments
                        .iter()
                        .map(|argument| self.value(argument, parameters, &symbols))
                        .collect::<Option<Vec<_>>>()?;
                    // The call must use the entry's own original neighbors.
                    if arguments
                        != [
                            self.memory[&(ENTRY, "prev".into())],
                            self.memory[&(ENTRY, "next".into())],
                        ]
                    {
                        return None;
                    }
                    self.helper_calls = self.helper_calls.checked_add(1)?;
                    let [inner] = helper.paths.as_slice() else {
                        return None;
                    };
                    self.run(&inner.operations, &arguments, None)?;
                }
                TeardownOperationRecord::LocalWrite { value, .. } => {
                    self.value(value, parameters, &symbols)?;
                }
                TeardownOperationRecord::LocalEnd { .. } => (),
                TeardownOperationRecord::Guard { .. } | TeardownOperationRecord::Return => {
                    return None;
                }
            }
        }
        Some(())
    }
}

fn derive_unlink(
    entry: &TeardownFunctionRecord,
    helper: &TeardownFunctionRecord,
    node_c_type: &str,
) -> Option<ListUnlinkRecord> {
    if !bounded(entry)
        || !bounded(helper)
        || node_c_type.is_empty()
        || entry.function == helper.function
        || entry.source_revision != helper.source_revision
        || entry.signature != format!("void ({node_c_type})")
        || helper.signature != format!("void ({node_c_type}, {node_c_type})")
        || entry.paths.len() != 1
        || helper.paths.len() != 1
    {
        return None;
    }
    let mut cases = Vec::new();
    for neighbors_alias in [false, true] {
        let next = if neighbors_alias {
            PREVIOUS
        } else {
            PREVIOUS + 1
        };
        let initial = BTreeMap::from([
            ((ENTRY, "prev".to_owned()), Some(PREVIOUS)),
            ((ENTRY, "next".to_owned()), Some(next)),
            ((PREVIOUS, "next".to_owned()), Some(ENTRY)),
            ((next, "prev".to_owned()), Some(ENTRY)),
        ]);
        let mut replay = LinkReplay {
            memory: initial,
            writes: Vec::new(),
            helper_calls: 0,
            node_c_type,
        };
        replay.run(&entry.paths[0].operations, &[Some(ENTRY)], Some(helper))?;
        cases.push(UnlinkCaseRecord {
            neighbors_alias,
            helper_calls: replay.helper_calls,
            writes: replay.writes,
        });
    }
    Some(ListUnlinkRecord {
        function: entry.function.clone(),
        helper_function: helper.function.clone(),
        source_revision: entry.source_revision.clone(),
        node_c_type: node_c_type.to_owned(),
        cases,
        profile_assumptions: entry.profile_assumptions.clone(),
    })
}

// ---------------------------------------------------------------------------
// Nullable-field destructor
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Pointer {
    Root,
    Field(String),
    Null,
    Boolean(bool),
}

struct FieldReplay<'a> {
    fields: &'a [String],
    initial: &'a BTreeSet<String>,
    released: Vec<String>,
    symbols: BTreeMap<u32, Pointer>,
}

impl FieldReplay<'_> {
    fn truth(value: &Pointer) -> bool {
        !matches!(value, Pointer::Null | Pointer::Boolean(false))
    }

    fn value(&self, value: &TeardownValueRecord) -> Option<Pointer> {
        match &value.expression {
            TeardownExpressionRecord::Parameter { index: 0 } => Some(Pointer::Root),
            TeardownExpressionRecord::Symbol { operation } => self.symbols.get(operation).cloned(),
            TeardownExpressionRecord::Integer { value } if value == "0" => Some(Pointer::Null),
            TeardownExpressionRecord::Cast { value, .. } => self.value(value),
            TeardownExpressionRecord::Unary { operator, value } if operator == "!" => {
                Some(Pointer::Boolean(!Self::truth(&self.value(value)?)))
            }
            TeardownExpressionRecord::Binary {
                operator,
                left,
                right,
            } if matches!(operator.as_str(), "==" | "!=") => {
                let left = self.value(left)?;
                let right = self.value(right)?;
                let same = left == right
                    || matches!(
                        (&left, &right),
                        (Pointer::Null, Pointer::Boolean(false))
                            | (Pointer::Boolean(false), Pointer::Null)
                    );
                Some(Pointer::Boolean(if operator == "==" {
                    same
                } else {
                    !same
                }))
            }
            _ => None,
        }
    }

    /// `Some(true)` when this path is the realizable one for the state,
    /// `Some(false)` when a guard excluded it, `None` on any unmodeled shape.
    fn run(&mut self, operations: &[TeardownOperationRecord]) -> Option<bool> {
        for (index, operation) in operations.iter().enumerate() {
            let index = u32::try_from(index).ok()?;
            match operation {
                TeardownOperationRecord::Read { place } => {
                    let TeardownExpressionRecord::Field { base, name } = &place.expression else {
                        return None;
                    };
                    let field = match self.value(base)? {
                        Pointer::Root => name.clone(),
                        Pointer::Field(parent) if !self.released.contains(&parent) => {
                            format!("{parent}__{name}")
                        }
                        _ => return None,
                    };
                    if !self.fields.contains(&field) {
                        return None;
                    }
                    self.symbols.insert(
                        index,
                        if self.initial.contains(&field) {
                            Pointer::Field(field)
                        } else {
                            Pointer::Null
                        },
                    );
                }
                TeardownOperationRecord::Guard { condition, taken } => {
                    if Self::truth(&self.value(condition)?) != *taken {
                        return Some(false);
                    }
                }
                TeardownOperationRecord::Call { arguments, .. } => {
                    let [argument] = arguments.as_slice() else {
                        return None;
                    };
                    match self.value(argument)? {
                        Pointer::Null => (),
                        Pointer::Field(field) => {
                            if self.released.contains(&field)
                                || self.initial.iter().any(|live| {
                                    live.starts_with(&format!("{field}__"))
                                        && !self.released.contains(live)
                                })
                            {
                                return None;
                            }
                            self.released.push(field);
                        }
                        _ => return None,
                    }
                }
                TeardownOperationRecord::LocalWrite { value, .. } => {
                    self.value(value)?;
                }
                TeardownOperationRecord::LocalEnd { .. } => (),
                TeardownOperationRecord::Return => return Some(true),
                TeardownOperationRecord::Write { .. } => return None,
            }
        }
        None
    }
}

fn derive_destructor(
    destructor: &TeardownFunctionRecord,
    root_c_type: &str,
    declared: &[String],
) -> Option<FieldDestructorRecord> {
    let unique: BTreeSet<&String> = declared.iter().collect();
    if !bounded(destructor)
        || root_c_type.is_empty()
        || declared.is_empty()
        || declared.len() > MAX_FIELDS
        || unique.len() != declared.len()
        || declared.iter().any(String::is_empty)
        || destructor.signature != format!("void ({root_c_type})")
    {
        return None;
    }
    // The core reads its own inventory as an ordered set, so the proposal must
    // enumerate the same nullable states in the same order.
    let fields: Vec<String> = unique.into_iter().cloned().collect();
    let mut cases = Vec::new();
    for mask in 0..(1usize << fields.len()) {
        let live: BTreeSet<String> = fields
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, field)| field.clone())
            .collect();
        // A null parent has no independently live child storage.
        if live.iter().any(|field| {
            field
                .rsplit_once("__")
                .is_some_and(|(parent, _)| !live.contains(parent))
        }) {
            continue;
        }
        let mut selected = Vec::new();
        for (index, path) in destructor.paths.iter().enumerate() {
            let mut replay = FieldReplay {
                fields: &fields,
                initial: &live,
                released: Vec::new(),
                symbols: BTreeMap::new(),
            };
            if replay.run(&path.operations)? {
                selected.push((u32::try_from(index).ok()?, replay.released));
            }
        }
        let [(source_path, released)] = selected.as_slice() else {
            return None;
        };
        cases.push(FieldReleaseCaseRecord {
            initially_live: live.into_iter().collect(),
            released_in_order: released.clone(),
            source_path: *source_path,
        });
    }
    Some(FieldDestructorRecord {
        function: destructor.function.clone(),
        source_revision: destructor.source_revision.clone(),
        root_c_type: root_c_type.to_owned(),
        declared_owned_fields: declared.to_vec(),
        cases,
        profile_assumptions: destructor.profile_assumptions.clone(),
    })
}

fn derive(source: &TeardownSourceInput) -> Option<TeardownProposalRecord> {
    if !source.version_matches() {
        return None;
    }
    match &source.family {
        TeardownFamilyInput::ListUnlink {
            entry,
            helper,
            node_c_type,
        } => derive_unlink(entry, helper, node_c_type).map(TeardownProposalRecord::ListUnlink),
        TeardownFamilyInput::FieldDestructor {
            destructor,
            root_c_type,
            declared_owned_fields,
        } => derive_destructor(destructor, root_c_type, declared_owned_fields)
            .map(TeardownProposalRecord::FieldDestructor),
    }
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || !matches!(
            request.target.profile.as_str(),
            "qcore.c_list_unlink_contract.v1" | "qcore.c_field_destructor_contract.v1"
        )
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    let summary = derive(request.teardown_source.as_ref()?)?;
    // The declared profile and the projected family must agree; a projection
    // read under the other family's profile is declined, not answered.
    let matches_profile = match &summary {
        TeardownProposalRecord::ListUnlink(_) => {
            request.target.profile == "qcore.c_list_unlink_contract.v1"
        }
        TeardownProposalRecord::FieldDestructor(_) => {
            request.target.profile == "qcore.c_field_destructor_contract.v1"
        }
    };
    if !matches_profile {
        return None;
    }
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::TeardownContract(Box::new(TeardownContractWitness {
                task_digest: request.task_digest.clone(),
                summary,
            })),
        }),
    })
}

#[cfg(test)]
mod tests;
