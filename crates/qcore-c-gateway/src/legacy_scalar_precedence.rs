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

//! LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS: a separately read scalar plan
//! may discharge a lexical access's overapproximate flow obligation. The old
//! instruction remains the fallback. Admission of syntax never selects a plan:
//! transfer must discharge every typed obligation on the incoming state.

use super::*;

impl Parser<'_> {
    pub(super) fn scalar_statement_alternative(
        &self,
        tokens: &[Token],
        old: LegacyInstruction,
    ) -> LegacyInstruction {
        if contains_call(tokens) || contains_braces(tokens) {
            return old;
        }
        // W48-BOTTOM-UP-BATCH5: the lexical opaque-field reader does not
        // distinguish data from a pointer value. A complete typed scalar
        // assignment retains all reads and forgets only its scalar result.
        if let LegacyInstruction::OpaqueValue {
            destination,
            alias_read,
        } = &old
        {
            let saved = qcore_schemas::contract_consultation::snapshot();
            let candidate = self.typed_path_statement(tokens, &old);
            if let Some(candidate) =
                crate::scalar_read_identity::retain(destination, alias_read.as_ref(), candidate)
            {
                return candidate;
            }
            qcore_schemas::contract_consultation::restore(&saved);
            return old;
        }
        if !eligible(&old) {
            return old;
        }
        let saved = qcore_schemas::contract_consultation::snapshot();
        let candidate = self.typed_path_statement(tokens, &old);
        // Only a retained plan may contribute dependencies. Lazy AST facts
        // and parity records are value-determined and need no rollback.
        let (old, kept) = attach(old, candidate);
        if !kept {
            qcore_schemas::contract_consultation::restore(&saved);
        }
        old
    }

    pub(super) fn scalar_condition_alternative(
        &self,
        tokens: &[Token],
        old: LegacyInstruction,
    ) -> LegacyInstruction {
        if !eligible(&old)
            || contains_call(tokens)
            || contains_braces(tokens)
            || self.opaque_callee(tokens)
        {
            return old;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names).is_some() {
            return old;
        }
        let saved = qcore_schemas::contract_consultation::snapshot();
        let candidate = tokens
            .first()
            .and_then(|first| {
                self.typed_positions_index()?.condition(
                    &texts,
                    first.line,
                    |name| self.position_never_bound(name),
                    None,
                )
            })
            .and_then(|plan| {
                if plan.borrowed_destination
                    || plan.pointer_store.is_some()
                    || plan.readback.is_some()
                {
                    return None;
                }
                Some(LegacyInstruction::IndirectScalarAccess {
                    typed_alternative: None,
                    bases: plan.scalar.bases,
                    alias_reads: plan
                        .scalar
                        .record_bases
                        .iter()
                        .filter_map(|base| self.read_alias(Some(base)))
                        .collect(),
                    destination: None,
                    frame: None,
                    chains: plan.chains,
                    borrowed_destination: false,
                    readback: None,
                })
            });
        let (old, kept) = attach(old, candidate);
        if !kept {
            qcore_schemas::contract_consultation::restore(&saved);
        }
        old
    }
}

fn eligible(instruction: &LegacyInstruction) -> bool {
    matches!(
        instruction,
        LegacyInstruction::ScalarMemberStore {
            typed_alternative: None,
            ..
        } | LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            borrowed_destination: false,
            readback: None,
            ..
        }
    )
}

/// Equal effects are checked independently of obligations. In particular a
/// different refused-write frame, pointer-valued destination or readback is
/// never hidden under a scalar alternative. Preserve the old alias reads even
/// when Clang's declaration reader contributes fewer names.
fn attach(
    mut old: LegacyInstruction,
    candidate: Option<LegacyInstruction>,
) -> (LegacyInstruction, bool) {
    let Some(mut candidate) = candidate else {
        return (old, false);
    };
    let LegacyInstruction::IndirectScalarAccess {
        typed_alternative: None,
        alias_reads,
        destination,
        frame,
        borrowed_destination: false,
        readback: None,
        ..
    } = &mut candidate
    else {
        return (old, false);
    };
    let (old_destination, old_frame, old_aliases) = match &old {
        LegacyInstruction::ScalarMemberStore { alias_read, .. } => {
            (None, None, alias_read.iter().cloned().collect::<Vec<_>>())
        }
        LegacyInstruction::IndirectScalarAccess {
            destination,
            frame,
            alias_reads,
            borrowed_destination: false,
            readback: None,
            ..
        } => (destination.clone(), frame.clone(), alias_reads.clone()),
        _ => return (old, false),
    };
    if *destination != old_destination || *frame != old_frame {
        return (old, false);
    }
    for read in old_aliases {
        if !alias_reads.contains(&read) {
            alias_reads.push(read);
        }
    }
    if candidate == old {
        return (old, false);
    }
    match &mut old {
        LegacyInstruction::ScalarMemberStore {
            typed_alternative, ..
        }
        | LegacyInstruction::IndirectScalarAccess {
            typed_alternative, ..
        } => {
            *typed_alternative = Some(Box::new(candidate));
        }
        _ => unreachable!("eligible scalar instruction"),
    }
    (old, true)
}

/// Purely scalar alternatives allocate no site, invoke no call and do not
/// change transfer context. Null-premise recording and consultation state are
/// isolated alongside diagnostics; a declined attempt leaves none behind.
pub(super) fn alternative(instruction: &LegacyInstruction) -> Option<&LegacyInstruction> {
    match instruction {
        LegacyInstruction::ScalarMemberStore {
            typed_alternative, ..
        }
        | LegacyInstruction::IndirectScalarAccess {
            typed_alternative, ..
        } => typed_alternative.as_deref(),
        _ => None,
    }
}

/// Run the exact old instruction without recursively consulting its candidate.
pub(super) fn original(instruction: &LegacyInstruction) -> LegacyInstruction {
    let mut original = instruction.clone();
    match &mut original {
        LegacyInstruction::ScalarMemberStore {
            typed_alternative, ..
        }
        | LegacyInstruction::IndirectScalarAccess {
            typed_alternative, ..
        } => {
            *typed_alternative = None;
        }
        _ => {}
    }
    original
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access() -> LegacyInstruction {
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            bases: vec!["p".into()],
            alias_reads: Vec::new(),
            destination: None,
            frame: None,
            chains: Vec::new(),
            borrowed_destination: false,
            readback: None,
        }
    }

    #[test]
    fn different_scalar_effects_and_pointer_values_cannot_be_alternatives() {
        for change in 0..4 {
            let old = access();
            let mut typed = access();
            if let LegacyInstruction::IndirectScalarAccess {
                destination,
                frame,
                borrowed_destination,
                readback,
                ..
            } = &mut typed
            {
                match change {
                    0 => *destination = Some("flag".into()),
                    1 => *frame = Some(RefusedWrites::Unbounded),
                    2 => *borrowed_destination = true,
                    _ => *readback = Some("p->next".into()),
                }
            }
            assert_eq!(attach(old.clone(), Some(typed)), (old, false));
        }
    }

    #[test]
    fn a_candidate_with_fewer_alias_names_keeps_every_original_read() {
        let read = AliasPeers {
            place: "p".into(),
            peers: vec!["q".into()],
        };
        let old = LegacyInstruction::ScalarMemberStore {
            typed_alternative: None,
            base: "p".into(),
            alias_read: Some(read.clone()),
        };
        let (wrapped, kept) = attach(old.clone(), Some(access()));
        assert!(kept);
        let Some(LegacyInstruction::IndirectScalarAccess { alias_reads, .. }) =
            alternative(&wrapped)
        else {
            panic!("scalar alternative retained")
        };
        assert_eq!(alias_reads, &[read]);
        assert_eq!(original(&wrapped), old);
    }
}
