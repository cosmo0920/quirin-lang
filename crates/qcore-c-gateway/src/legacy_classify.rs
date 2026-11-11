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

//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the statement classifier
//! (ROUTE-MAP section 2.2): `classify_simple` and its guards, the typed
//! fallbacks of `classify_simple_unguarded`, the element, sub-object and
//! stepped routes and `classify_simple_base`.
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

impl Parser<'_> {
    /// Classify one complete statement, then apply card
    /// **W43-DESCRIPTOR-RESOURCE-TRACKING**'s guard: a statement that reads a
    /// tracked descriptor is admitted only by an instruction that states that
    /// read's custody effect exactly
    /// ([`descriptor_custody::admits_descriptor_read`]), and a descriptor
    /// handed to a callee no declaration covers is refused. With no descriptor
    /// name in the body the guard is skipped, so every heap-only body keeps
    /// exactly the classification it had.
    ///
    /// Card **W45-PARENTHESISED-CALLEE** wraps it: an admitted statement that
    /// applies a call to a callee expression that is not a name refuses.
    ///
    /// Card **GAPO-RETURN-CORRECTNESS** (rule 15x): every guard that replaces
    /// an admission by a refusal hands back the whole probe state the
    /// admission moved ([`probe_state::ParserProbeState`]), not only the site
    /// and element counters, and the descriptor guard, which restored
    /// nothing, does too.
    pub(super) fn classify_simple(&mut self, tokens: &[Token]) -> LegacyInstruction {
        let instruction = self.classify_simple_recorded(tokens);
        // Card SCALAR-STATEMENT-FORMS: measurement only (feature
        // `scalar-form-census`, off by default).
        #[cfg(feature = "scalar-form-census")]
        if let LegacyInstruction::Unknown { description, .. } = &instruction {
            let texts = tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            crate::scalar_form_census::record(
                &self.typed_path_function,
                tokens.first().map_or(0, |token| token.line),
                &texts,
                description,
                "classified",
                None,
            );
        }
        instruction
    }

    fn classify_simple_recorded(&mut self, tokens: &[Token]) -> LegacyInstruction {
        let saved = self.probe_state();
        let instruction = self.classify_simple_descriptors(tokens, &saved);
        let instruction = self.refuse_opaque_call(tokens, instruction, &saved);
        // Card CG25-RESOURCE-KINDS (rule 11v): a tracked in-place object is
        // custody, never data.
        let instruction = self.in_place_guard(tokens, instruction, &saved);
        // Rule 36a: only an old refusal in a complete checked caller.
        let instruction = self.self_init_call(tokens, instruction);
        let instruction = self.tracked_self_init_call(tokens, instruction);
        // Card CHAINED-ASSIGNMENT-INNER-DESTINATION (rule 5i, amended): an
        // admission that would drop a write inside an assigned value is
        // replaced by the refusal; a refusal stays itself.
        let instruction = self.chained_assignment_guard(tokens, instruction, &saved);
        // Card W45-FB-BACKLOG-CUSTODY, rule 4: an admission that hands an
        // iteration's protected node or element to a position that may unlink
        // or release it is replaced by the refusal; a refusal stays itself.
        if self.list_frames.is_empty() || matches!(instruction, LegacyInstruction::Unknown { .. }) {
            return instruction;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some(description) = self.list_iteration_refusal(&texts) else {
            return instruction;
        };
        self.restore_probe_state(&saved);
        LegacyInstruction::Unknown {
            description: description.to_owned(),
            writes: self.refused_writes(&texts),
        }
    }

    /// The W43 descriptor guard; a refusal of an admission hands back
    /// `saved` (rule 15x).
    pub(super) fn classify_simple_descriptors(
        &mut self,
        tokens: &[Token],
        saved: &probe_state::ParserProbeState,
    ) -> LegacyInstruction {
        let instruction = self.classify_simple_unguarded(tokens);
        let instruction = self.scalar_statement_alternative(tokens, instruction);
        if self.descriptor_names.is_empty() {
            return instruction;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some(read) =
            descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
        else {
            return instruction;
        };
        if !descriptor_custody::admits_descriptor_read(&instruction, &self.descriptor_names) {
            if !matches!(instruction, LegacyInstruction::Unknown { .. }) {
                self.restore_probe_state(saved);
            }
            return LegacyInstruction::Unknown {
                description: format!(
                    "tracked descriptor `{read}` read by a custody-neutral scalar route is \
                     outside the resource projection"
                ),
                writes: self.refused_writes(&texts),
            };
        }
        if let LegacyInstruction::ContractCall {
            function, nested, ..
        } = &instruction
            && let Some(callee) = std::iter::once(function.as_str())
                .chain(nested.iter().map(|nested| nested.function.as_str()))
                .find(|callee| !self.declared_callees.contains(*callee))
        {
            self.restore_probe_state(saved);
            return LegacyInstruction::Unknown {
                description: format!(
                    "tracked descriptor `{read}` passed to `{callee}`, whose effect on it no \
                     declaration states"
                ),
                writes: self.refused_writes(&texts),
            };
        }
        instruction
    }

    /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: the classifier the descriptor
    /// guard above wraps. Card W43-INDIRECT-ACCESS-GENERAL's typed-scalar route
    /// is inside the guard, so every instruction it admits is checked by
    /// [`descriptor_custody::admits_descriptor_read`] like any other.
    ///
    /// Card **W43-INDIRECT-ACCESS-GENERAL**: the statement classifier, with
    /// one route consulted only after the existing classifier has refused.
    ///
    /// The route is tried for exactly the refusals whose cause is the access
    /// itself — the lexical indirect-access gate, an unsupported assignment
    /// expression, an unrecognized destination, an unmodeled increment — and
    /// for a `*B` store the existing classifier named for an out-parameter
    /// slot. Every statement the existing classifier admits keeps its exact
    /// lowering, so the route can only replace a refusal (or an unsanctioned
    /// `*B` store, itself refused in transfer) by a statement whose own
    /// obligations the flow analysis then checks.
    pub(super) fn classify_simple_unguarded(&mut self, tokens: &[Token]) -> LegacyInstruction {
        let instruction = self.classify_simple_base(tokens);
        // Card W43-INDEXED-RESOURCE-COLLECTIONS: an element store or release
        // the classifier refused, over a pointer-to-pointer base. The refusal
        // is carried as the node's fallback, byte for byte.
        if matches!(instruction, LegacyInstruction::Unknown { .. })
            && let Some(element) = self.element_statement(tokens, &instruction)
        {
            self.element_lowerings += 1;
            self.record_legacy_typed_parity(tokens, "indexed_element", &element);
            return element;
        }
        // Card W43-INDEX-TRAVERSAL: the formal-mutation gate refuses `q++` for
        // every protected name, and protection reaches a *local* through a
        // copy (`q = p;` with `p` later released). A step of a local changes
        // no caller slot and no entry identity: the flow rule derives `q` from
        // its own binding and refuses a step that would lose a live object's
        // only name. W48 bottom-up batch 2 additionally admits a self-step
        // of a scalar-pointer formal whose AST uses it only in scalar reads
        // and discarded self-steps, with neither entry custody nor an out-slot contract.
        // Every other formal and every other shape keeps the gate.
        if let LegacyInstruction::Unknown { description, .. } = &instruction
            && description == FORMAL_MUTATION_REFUSAL
            && !contains_call(tokens)
            && !contains_braces(tokens)
        {
            let texts = tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            if let Some(step) =
                crate::index_traversal::pointer_step(&texts, &self.typed_model, &self.typed_types)
                && ((!self.formal_parameters.contains(&step.destination)
                    && !self.formal_parameters.contains(&step.source))
                    || (step.destination == step.source
                        && self.formal_parameters.contains(&step.source)
                        && crate::index_traversal::formal_step_eligible(
                            matches!(
                                self.typed_types.pointer_kind(&step.source),
                                Some(crate::typed_field_access::PointerKind::Scalar)
                            ),
                            self.fixed_formals.contains(&step.source),
                            self.typed_positions_index().is_some_and(|index| {
                                crate::index_traversal::read_only_formal(
                                    &index.function,
                                    &step.source,
                                )
                            }),
                        )))
            {
                let instruction = LegacyInstruction::PointerStep {
                    destination: step.destination,
                    source: step.source,
                    offset: step.offset,
                };
                self.record_legacy_typed_parity(tokens, "index_traversal", &instruction);
                return instruction;
            }
            return instruction;
        }
        let consulted = match &instruction {
            LegacyInstruction::Unknown { description, .. } => {
                TYPED_SCALAR_CONSULTED.contains(&description.as_str())
            }
            LegacyInstruction::Alias { destination, .. }
            | LegacyInstruction::Clear { destination } => is_indirect_place(destination),
            _ => false,
        };
        // Card W45-SUB-OBJECT-STORES: a sub-object store whose value is one
        // whole call; the typed routes below never read a call.
        if contains_call(tokens)
            && let Some(store) = self.sub_object_store_statement(tokens, &instruction)
        {
            self.record_legacy_typed_parity(tokens, "sub_object_store_call", &store);
            return store;
        }
        if !consulted || contains_call(tokens) || contains_braces(tokens) {
            return instruction;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some(plan) = crate::typed_field_access::typed_scalar_statement(
            &texts,
            &self.typed_model,
            &self.typed_types,
        ) else {
            // Card W44-INDIRECT-CHAIN-ACCESS: the same typed route with
            // record-pointer member chains as accesses, in the same position
            // and for the same refusals. See `crate::indirect_chain_access`.
            if let Some(chained) = crate::indirect_chain_access::chain_statement(
                &texts,
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
            ) {
                if crate::typed_access_path::parity_enabled() {
                    let new = self.typed_path_plan(tokens);
                    crate::typed_access_path::record_parity(
                        &self.typed_path_function,
                        tokens.first().map_or(0, |token| token.line),
                        &texts,
                        "chain",
                        Some((&chained.plan, &chained.chains)),
                        new.as_ref(),
                    );
                }
                return self.chain_access_instruction(chained, true);
            }
            // Card W45-SUB-OBJECT-STORES: a sub-object store through a
            // resolved, typed base, after both typed access routes declined.
            if let Some(store) = self.sub_object_store_statement(tokens, &instruction) {
                self.record_legacy_typed_parity(tokens, "sub_object_store", &store);
                return store;
            }
            // Card W43-INDEX-TRAVERSAL: a statement that moves a declared
            // pointer (`p++`, `p += k`, `p = q + k`) is consulted in the same
            // position and for the same refusals, after the typed access route.
            // It binds nothing new: the flow analysis derives the result from
            // the source's own binding (see `LegacyInstruction::PointerStep`).
            if matches!(instruction, LegacyInstruction::Unknown { .. })
                && let Some(step) = crate::index_traversal::pointer_step(
                    &texts,
                    &self.typed_model,
                    &self.typed_types,
                )
            {
                let instruction = LegacyInstruction::PointerStep {
                    destination: step.destination,
                    source: step.source,
                    offset: step.offset,
                };
                self.record_legacy_typed_parity(tokens, "index_traversal", &instruction);
                return instruction;
            }
            // W46-ACCESS-PATH-STATEMENTS: only a residual access refusal
            // reaches Clang. Every established admitting shape route above
            // remains authoritative until its pinned parity is measured.
            if matches!(&instruction, LegacyInstruction::Unknown { description, .. }
                if TYPED_SCALAR_CONSULTED.contains(&description.as_str()))
                && let Some(typed) = self.typed_path_statement(tokens, &instruction)
            {
                return typed;
            }
            // Card OUT-PARAMETER-BODY-SUMMARY (rule 10k(5)): a `*B = V` store
            // the importer named for an out slot, after every lexical route
            // above declined, whose AST reading is a scalar store (the
            // destination's type a scalar, the owner-aware enum arm
            // included). Only that lowering replaces the store: a pointer
            // store, a refusal or a plan with no access keeps the out-slot
            // reading, which transfer refuses unless a contract sanctions it.
            if matches!(&instruction,
                    LegacyInstruction::Alias { destination, .. }
                    | LegacyInstruction::Clear { destination } if is_indirect_place(destination))
                && let Some(typed @ LegacyInstruction::IndirectScalarAccess { .. }) =
                    self.typed_path_statement(tokens, &instruction)
            {
                return typed;
            }
            return instruction;
        };
        if crate::typed_access_path::parity_enabled() {
            let new = self.typed_path_plan(tokens);
            crate::typed_access_path::record_parity(
                &self.typed_path_function,
                tokens.first().map_or(0, |token| token.line),
                &texts,
                "scalar",
                Some((&plan, &[])),
                new.as_ref(),
            );
        }
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.bases,
            destination: plan.destination,
            // The statement's only writes are its typed destination: the
            // object `*B`/`B[I]` designates, never the slot of `B` itself, and
            // a declared scalar, which binds nothing. So every name whose slot
            // no other lvalue designates is spared, and every other binding is
            // poisoned exactly as the refusal's frame would have poisoned it.
            // (`may_write_names` would read `* p =` as a write of `p`, which is
            // right for an unparsed refusal and wrong for this parsed store.)
            frame: plan
                .writes_through_pointer
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: Vec::new(),
            borrowed_destination: false,
            readback: None,
        }
    }

    /// Card **W45-SUB-OBJECT-STORES**: a statement the lexical gate refused as
    /// a store into a sub-object, read through a resolved, typed base; see
    /// [`sub_object_store`]. Consulted only for exactly the gate's refusal,
    /// only after every older route declined, and never for a statement that
    /// reads a tracked descriptor (a descriptor is custody, never data: the
    /// store keeps its refusal byte for byte). `None` keeps the refusal.
    pub(super) fn sub_object_store_statement(
        &mut self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<LegacyInstruction> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if description != "indirect memory access is outside the resource projection"
            || contains_braces(tokens)
        {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !self.descriptor_names.is_empty()
            && descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        let equal = texts.iter().position(|text| *text == "=")?;
        if texts.get(equal + 1) == Some(&"=") {
            return None;
        }
        let destination = sub_object_store::read_destination(
            &texts[..equal],
            &self.typed_model,
            &self.typed_types,
            &self.sub_object_roots,
        )
        .ok()?;
        let right = &texts[equal + 1..];
        let syntax = sub_object_store::read_value(
            right,
            destination.member,
            &self.typed_model,
            &self.typed_types,
        )
        .ok()?;
        let (value, value_bases) = match syntax {
            sub_object_store::ValueSyntax::Scalar(bases) => (
                sub_object_store::StoredValue::Scalar {
                    bases: bases.clone(),
                },
                bases,
            ),
            sub_object_store::ValueSyntax::Untracked => {
                (sub_object_store::StoredValue::Untracked, Vec::new())
            }
            // A formal's pointer is the caller's object: storing it into a
            // record hands that object on in a way no summary mode states
            // (the derived summary would call the formal borrowed), so the
            // store keeps its refusal.
            sub_object_store::ValueSyntax::Name(name) if self.formal_parameters.contains(&name) => {
                return None;
            }
            sub_object_store::ValueSyntax::Name(name) => {
                (sub_object_store::StoredValue::Name(name), Vec::new())
            }
            sub_object_store::ValueSyntax::Call(length) => {
                let line = tokens.first().map_or(0, |token| token.line);
                let mut rewritten = vec![
                    Token {
                        text: sub_object_store::STORE_TEMPORARY.to_owned(),
                        line,
                    },
                    Token {
                        text: "=".to_owned(),
                        line,
                    },
                ];
                rewritten.extend(tokens[tokens.len() - length..].iter().cloned());
                let saved = self.probe_state();
                let call = self.classify_simple_base(&rewritten);
                // A call whose every parameter is borrowed and whose result
                // carries no custody. Every other call keeps the refusal.
                let admitted = matches!(
                    &call,
                    LegacyInstruction::ContractCall {
                        destination: Some(temporary),
                        contract,
                        nested,
                        aggregate_reads,
                        aggregate_addresses,
                        ..
                    } if temporary == sub_object_store::STORE_TEMPORARY
                        && nested.is_empty()
                        && aggregate_reads.is_empty()
                        && aggregate_addresses.is_empty()
                        && contract
                            .parameters
                            .iter()
                            .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
                        // Card W45-FB-RETRY-LIFETIME: a container effect
                        // moves custody despite borrowed modes.
                        && !contract.has_container_effect()
                        // An owned result would be a tracked identity stored
                        // into a record: it keeps the refusal.
                        && matches!(contract.result, ResourceReturnMode::None)
                );
                if !admitted {
                    self.restore_probe_state(&saved);
                    return None;
                }
                (
                    sub_object_store::StoredValue::Call(Box::new(call)),
                    Vec::new(),
                )
            }
        };
        Some(LegacyInstruction::SubObjectStore {
            alias_read: self.read_alias(destination.base.as_deref()),
            destination,
            value,
            value_bases,
            fallback: Box::new(instruction.clone()),
        })
    }

    /// Card **W44-INDIRECT-CHAIN-ACCESS**: the typed access an admitted chain
    /// statement or condition lowers to — card W43-INDIRECT-ACCESS-GENERAL's
    /// instruction, built exactly as `classify_simple_unguarded` builds it,
    /// with the chains' link obligations attached. `statement` selects the
    /// write frame, which a condition never has.
    pub(super) fn chain_access_instruction(
        &self,
        chained: crate::indirect_chain_access::ChainPlan,
        statement: bool,
    ) -> LegacyInstruction {
        let plan = chained.plan;
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.bases,
            destination: plan.destination,
            frame: (statement && plan.writes_through_pointer)
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: chained.chains,
            borrowed_destination: false,
            readback: None,
        }
    }

    /// Card **W43-INDEX-TRAVERSAL**: a refused statement that is one typed
    /// scalar access over the old value of a pointer it steps (`*p++ = c`,
    /// `c = *p++`) or of an index it steps (`buf[n++] = c`), lowered to the
    /// access followed by the step or the index's forgotten fact — the order C
    /// sequences them in. Consulted only for the refusals the typed access
    /// route is consulted for, and only for a statement carrying no call and
    /// no brace; see [`crate::index_traversal::stepped_access`] for the shape.
    /// `None` keeps the refusal.
    pub(super) fn stepped_access(
        &self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<Vec<LegacyInstruction>> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        let gated = description == FORMAL_MUTATION_REFUSAL;
        if !(gated || TYPED_SCALAR_CONSULTED.contains(&description.as_str()))
            || contains_call(tokens)
            || contains_braces(tokens)
        {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (plan, effects) =
            crate::index_traversal::stepped_access(&texts, &self.typed_model, &self.typed_types)?;
        // Past the formal-mutation gate only when nothing the statement writes
        // is a formal: every step and forget names a local, and so does the
        // scalar destination. A write through a formal's object is not a
        // write of the formal.
        if gated
            && (effects.iter().any(|effect| match effect {
                crate::index_traversal::TraversalEffect::Step(step) => {
                    self.formal_parameters.contains(&step.destination)
                }
                crate::index_traversal::TraversalEffect::Forget(name) => {
                    self.formal_parameters.contains(name)
                }
            }) || plan
                .destination
                .as_ref()
                .is_some_and(|name| self.formal_parameters.contains(name)))
        {
            return None;
        }
        let mut sequence = vec![LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.bases,
            destination: plan.destination,
            // Exactly the typed route's frame: the access writes only its
            // typed destination, and the steps below write only their names.
            frame: plan
                .writes_through_pointer
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: Vec::new(),
            borrowed_destination: false,
            readback: None,
        }];
        sequence.extend(effects.into_iter().map(|effect| match effect {
            crate::index_traversal::TraversalEffect::Step(step) => LegacyInstruction::PointerStep {
                destination: step.destination,
                source: step.source,
                offset: step.offset,
            },
            crate::index_traversal::TraversalEffect::Forget(name) => {
                LegacyInstruction::ForgetScalar { destination: name }
            }
        }));
        Some(sequence)
    }

    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the syntactic premises of
    /// every element operation. The base is declared `T **BASE` at every
    /// declaration and is a slot no other lvalue designates (so no callee can
    /// rebind it); a named index is a declared integer with the same slot
    /// premise, and a `name++` index is additionally a counter that cannot
    /// wrap. Everything flow-sensitive is decided by `transfer`.
    pub(super) fn element_premises(
        &self,
        base: &str,
        index: &crate::indexed_collection::IndexSpelling,
    ) -> bool {
        use crate::indexed_collection::IndexSpelling;
        // W43-DESCRIPTOR-RESOURCE-TRACKING integration: an index that may hold
        // a tracked descriptor would be read as custody-neutral data.
        let neutral = |name: &str| !self.descriptor_names.contains(name);
        self.collection_types.pointer_element(base)
            && self.frame_protected.contains(base)
            && neutral(base)
            && match index {
                IndexSpelling::Literal(_) => true,
                IndexSpelling::Name(name) => {
                    self.collection_types.integer(name).is_some()
                        && self.frame_protected.contains(name)
                        && neutral(name)
                }
                IndexSpelling::PostIncrement(name) => {
                    self.collection_types.counter(name)
                        && self.frame_protected.contains(name)
                        && neutral(name)
                }
            }
    }

    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: lower a refused
    /// `BASE[INDEX] = CALL(...)`, `BASE[INDEX] = NULL` or `RELEASE(BASE[INDEX])`.
    ///
    /// The call is classified by the ordinary classifier with the element
    /// place renamed to a plain temporary, so every premise of a contracted
    /// call — the declared-ABI and literal-premise withdrawals, the borrowed
    /// actuals, the nested-call rule — is the one it always was. Only an
    /// acquiring call whose every parameter is `Borrowed`, or a
    /// one-parameter release with no result, is taken; anything else keeps
    /// the refusal, with the site counter restored.
    pub(super) fn element_statement(
        &mut self,
        tokens: &[Token],
        fallback: &LegacyInstruction,
    ) -> Option<LegacyInstruction> {
        use crate::indexed_collection::IndexSpelling;
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        // W43-DESCRIPTOR-RESOURCE-TRACKING integration: a statement that reads
        // a tracked descriptor keeps the classifier's refusal, which the
        // descriptor guard then sees exactly as it did before this card.
        if !self.descriptor_names.is_empty()
            && descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        let line = tokens.first().map_or(0, |token| token.line);
        let synthetic = |text: &str| Token {
            text: text.to_owned(),
            line,
        };
        if let Some(((base, index), value)) = crate::indexed_collection::store_statement(&texts) {
            if !self.element_premises(&base, &index) {
                return None;
            }
            if let IndexSpelling::PostIncrement(counter) = &index
                && value.contains(&counter.as_str())
            {
                return None;
            }
            let place = crate::indexed_collection::place_spelling(&base, &index.term());
            let call = if crate::indexed_collection::is_null_value(value) {
                None
            } else {
                let mut rewritten = vec![synthetic(ELEMENT_TEMPORARY), synthetic("=")];
                rewritten.extend(tokens[texts.len() - value.len()..].iter().cloned());
                let saved = self.probe_state();
                let mut instruction = self.classify_simple_base(&rewritten);
                let admitted = matches!(
                    &instruction,
                    LegacyInstruction::ContractCall {
                        destination: Some(destination),
                        contract,
                        ..
                    } if destination == ELEMENT_TEMPORARY
                        && matches!(contract.result, ResourceReturnMode::Owned { .. })
                        // W43-DESCRIPTOR-RESOURCE-TRACKING integration: a
                        // collection holds heap elements only; a stream, a
                        // directory or a descriptor keeps the refusal.
                        && contract.result_kind.is_heap()
                        && contract
                            .parameters
                            .iter()
                            .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
                );
                if !admitted {
                    self.restore_probe_state(&saved);
                    return None;
                }
                if let LegacyInstruction::ContractCall { destination, .. } = &mut instruction {
                    *destination = Some(place);
                }
                Some(Box::new(instruction))
            };
            return Some(LegacyInstruction::ElementStore {
                base,
                index,
                call,
                fallback: Box::new(fallback.clone()),
            });
        }
        let (callee, (base, index)) = crate::indexed_collection::release_statement(&texts)?;
        if !self.element_premises(&base, &index) {
            return None;
        }
        let rewritten = [
            synthetic(&callee),
            synthetic("("),
            synthetic(ELEMENT_TEMPORARY),
            synthetic(")"),
        ];
        let saved = self.probe_state();
        let instruction = self.classify_simple_base(&rewritten);
        self.restore_probe_state(&saved);
        let LegacyInstruction::ContractCall {
            function,
            destination: None,
            arguments,
            contract,
            nested,
            ..
        } = instruction
        else {
            return None;
        };
        let [ResourceParameterMode::Released { nullable }] = contract.parameters.as_slice() else {
            return None;
        };
        // W43-DESCRIPTOR-RESOURCE-TRACKING integration: only a heap release
        // discharges a (heap) collection element; `fclose(v[i])` or
        // `close(fds[i])` keeps the refusal.
        if !contract.parameter_kind(0).is_heap() {
            return None;
        }
        (function == callee
            && arguments == [Some(ELEMENT_TEMPORARY.to_owned())]
            && matches!(contract.result, ResourceReturnMode::None)
            && nested.is_empty())
        .then(|| LegacyInstruction::ElementRelease {
            base,
            index,
            callee,
            nullable: *nullable,
            fallback: Box::new(fallback.clone()),
        })
    }

    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: whether a `for` loop is a
    /// canonical counted loop whose exit leaves `INDEX = max(0, BOUND)`: the
    /// header spells `INDEX = 0; INDEX < BOUND; INDEX++`, the init and
    /// increment clauses were lowered to exactly `INDEX = 0` and a forget of
    /// `INDEX`, both names are declared integers whose slot no other lvalue
    /// designates, an unsigned index cannot wrap before the bound, and the
    /// body neither writes either name nor leaves or enters except through
    /// the header.
    pub(super) fn counted_loop(
        &self,
        init: Option<&(LegacyInstruction, usize)>,
        condition: &[Token],
        increment: &[Token],
        lowered_increment: Option<&(LegacyInstruction, usize)>,
        body: &[Token],
    ) -> Option<(String, crate::indexed_collection::Term)> {
        let texts = |tokens: &[Token]| {
            tokens
                .iter()
                .map(|token| token.text.clone())
                .collect::<Vec<_>>()
        };
        let (condition_texts, increment_texts, body_texts) =
            (texts(condition), texts(increment), texts(body));
        let (Some((LegacyInstruction::Clear { destination }, _)), Some((step, _))) =
            (init, lowered_increment)
        else {
            return None;
        };
        let init_view = [destination.as_str(), "=", "0"];
        let condition_view = condition_texts
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let increment_view = increment_texts
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let (index, bound) = crate::indexed_collection::counted_header(
            &init_view,
            &condition_view,
            &increment_view,
        )?;
        if index != *destination
            || !matches!(step, LegacyInstruction::ForgetScalar { destination } if *destination == index)
            || !self.frame_protected.contains(&index)
            || !self.collection_types.counted(&index, &bound)
        {
            return None;
        }
        let bound_name = match &bound {
            crate::indexed_collection::BoundSpelling::Name(name) => {
                if !self.frame_protected.contains(name) {
                    return None;
                }
                Some(name.as_str())
            }
            crate::indexed_collection::BoundSpelling::Literal(_) => None,
        };
        let body_view = body_texts.iter().map(String::as_str).collect::<Vec<_>>();
        crate::indexed_collection::body_keeps_exit_equation(&body_view, &index, bound_name)
            .then(|| (index.clone(), bound.term()))
    }

    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: a refused `if` condition that
    /// tests one element of a pointer-to-pointer base against the null
    /// constant.
    pub(super) fn element_condition(
        &self,
        condition: &[Token],
    ) -> Option<(String, crate::indexed_collection::IndexSpelling, bool)> {
        // Card W45-PARENTHESISED-CALLEE.
        if self.opaque_callee(condition) {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let ((base, index), true_is_null) = crate::indexed_collection::null_condition(&texts)?;
        self.element_premises(&base, &index)
            .then_some((base, index, true_is_null))
    }

    pub(super) fn classify_simple_base(&mut self, tokens: &[Token]) -> LegacyInstruction {
        // Retain the enclosing function and subsequent cleanup, but never
        // mistake nested initializers or GNU statement expressions for a
        // single modeled call/assignment. Their custody effects are unknown.
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        // Card W45-FB-BACKLOG-CUSTODY: an element statement expression
        // (`container_of`) over list memory is rule 3's, before the braces
        // refuse it.
        if let Some(instruction) = self.list_element_binding(&texts) {
            return instruction;
        }
        if contains_braces(tokens) {
            return LegacyInstruction::Unknown {
                description: "braced initializer, expression, or declaration is not modeled".into(),
                // A GNU statement expression's inner statements are inside this
                // same token run — `consume_to_semicolon` tracks delimiters —
                // so the mention set covers them too.
                writes: self.refused_writes(&texts),
            };
        }
        let assignment = texts.iter().position(|text| *text == "=");
        let mentions_formal = texts
            .iter()
            .any(|name| self.protected_formals.contains(*name));
        let has_increment = texts
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]));
        let mut expression_depth = 0usize;
        let nested_assignment = texts.iter().any(|text| match *text {
            "(" | "[" => {
                expression_depth += 1;
                false
            }
            ")" | "]" => {
                expression_depth = expression_depth.saturating_sub(1);
                false
            }
            "=" => expression_depth > 0,
            _ => false,
        });
        if mentions_formal && (has_increment || nested_assignment) {
            return LegacyInstruction::Unknown {
                description: FORMAL_MUTATION_REFUSAL.into(),
                writes: self.refused_writes(&texts),
            };
        }
        // Resource places currently use names, without lexical binding IDs.
        // A local declaration must not reuse a formal's entry custody or Out
        // slot. Conservatively reject ambiguous declaration prefixes too;
        // initializer references to a formal remain outside this prefix.
        let declaration = &texts[..assignment.unwrap_or(texts.len())];
        let declaration_like = declaration.first().is_some_and(|first| {
            DECLARATION_SPECIFIERS.contains(first)
                || (is_identifier(first)
                    && declaration.get(1).is_some_and(|second| {
                        is_identifier(second)
                            || *second == "*"
                            || (*second == "(" && declaration.get(2) == Some(&"*"))
                    }))
        });
        let mut nesting = 0usize;
        let mut initializer = false;
        let mut may_shadow_formal = false;
        if declaration_like {
            for text in &texts {
                match *text {
                    "(" | "[" => nesting += 1,
                    ")" | "]" => nesting = nesting.saturating_sub(1),
                    "=" if nesting == 0 => initializer = true,
                    "," if nesting == 0 => initializer = false,
                    name if !initializer && self.formal_parameters.contains(name) => {
                        may_shadow_formal = true;
                    }
                    _ => {}
                }
            }
        }
        if may_shadow_formal {
            return LegacyInstruction::Unknown {
                description: "local declaration may shadow a formal resource identity".into(),
                writes: self.refused_writes(&texts),
            };
        }
        // AOM-POISON-NOOP-INCREMENT: even a custody-neutral scalar write
        // must forget its old value. Otherwise `n = 0; n++; if (n)` drops
        // a reachable branch. Admit only a complete unary statement over a
        // declared scalar; nested writes and pointer arithmetic stay refused.
        if has_increment {
            let expression = strip_parentheses(&texts);
            let operand = match expression {
                ["+", "+", rest @ ..] | ["-", "-", rest @ ..] => Some(rest),
                [rest @ .., "+", "+"] | [rest @ .., "-", "-"] => Some(rest),
                _ => None,
            };
            if let Some([name]) = operand.map(strip_parentheses)
                && crate::typed_field_access::is_scalar_initializer(
                    name,
                    &["1"],
                    &self.typed_model,
                    &self.typed_types,
                )
            {
                return LegacyInstruction::ForgetScalar {
                    destination: (*name).to_owned(),
                };
            }
            return LegacyInstruction::Unknown {
                description: "unmodeled increment or decrement expression".into(),
                writes: self.refused_writes(&texts),
            };
        }
        // Fixed scalar queries are data, but their numerical result is not
        // modeled. Overwrite any old scalar/null fact with OpaqueValue rather
        // than leaving it stale or binding the predicate's marker as a value.
        if let Some(equal) = assignment
            && let Some(destination) = direct_assignment_destination(&texts[..equal])
            && let Some(right) = crate::type_query::scalar_operands(
                &texts[equal + 1..],
                &self.typed_model,
                &self.typed_types,
            )
            && !right.contains(&"=")
            && !right
                .windows(2)
                .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
            && crate::typed_field_access::is_scalar_initializer(
                &destination,
                &right,
                &self.typed_model,
                &self.typed_types,
            )
        {
            return LegacyInstruction::OpaqueValue {
                destination,
                alias_read: None,
            };
        }
        // W42-INDIRECT-ACCESS: the bounded counterpart of the scalar-field
        // no-op below.  A base this function may bind cannot be erased under
        // premise 8; lower it to an explicit instruction so transfer can
        // require the identity to be settled and live at this program point.
        if !contains_call(tokens)
            && let Some(base) = crate::typed_field_access::scalar_member_store_base(
                &texts,
                &self.typed_model,
                &self.typed_types,
            )
        {
            let instruction = LegacyInstruction::ScalarMemberStore {
                typed_alternative: None,
                base: base.to_owned(),
                alias_read: self.read_alias(Some(base)),
            };
            self.record_legacy_typed_parity(tokens, "scalar_member_store", &instruction);
            return instruction;
        }
        // One typed exception to the lexical indirect-access gate below: a
        // direct scalar-field access whose declared types make it
        // custody-neutral. Scalar destinations still lose their old facts. The whitelist is
        // conjunctive and every one of its premises is checked against
        // declarations read from this same translation unit; see
        // [`crate::typed_field_access`]. It runs after the formal-mutation and
        // shadowing gates, which still govern, and never for a statement
        // carrying a call, whose own rules continue to apply.
        if !contains_call(tokens)
            && let Some(bases) = crate::typed_field_access::scalar_field_noop_pointer_bases(
                &texts,
                &self.typed_model,
                &self.typed_types,
            )
        {
            let effect = self.scalar_effect(&texts);
            let destination = match &effect {
                LegacyInstruction::ForgetScalar { destination } => Some(destination.clone()),
                _ => None,
            };
            let instruction = match effect {
                LegacyInstruction::NoOp | LegacyInstruction::ForgetScalar { .. } => {
                    LegacyInstruction::IndirectScalarAccess {
                        typed_alternative: None,
                        alias_reads: bases
                            .iter()
                            .filter_map(|base| self.read_alias(Some(base)))
                            .collect(),
                        bases,
                        destination,
                        frame: None,
                        chains: Vec::new(),
                        borrowed_destination: false,
                        readback: None,
                    }
                }
                other => other,
            };
            self.record_legacy_typed_parity(tokens, "scalar_field_noop", &instruction);
            return instruction;
        }
        // O04/C02-P: the borrowed pointer-field actuals of a contracted call,
        // and an assignment whose whole right-hand side is one admitted direct
        // member read. Both are *reads*; the destination side of the lexical
        // gate below is untouched, which is what keeps such a place from ever
        // being bound. See [`crate::borrowed_actual`].
        let mut borrowed_actuals = crate::borrowed_actual::borrowed_actual_spans(
            &texts,
            self.contracts,
            &self.typed_model,
            &self.typed_types,
            &self.typed_bases,
        );
        // O04-READ-BOUND: the complementary read half of B08's premise set — a
        // whole `Borrowed` actual `BASE - > FIELD` on an aggregate base *this*
        // body bound. Its span leaves the lexical gate exactly as a C02-P
        // borrowed actual does; unlike one, it names a place the projection
        // binds, so [`transfer`] revalidates it flow-sensitively before the call
        // takes effect. The call index is this function's own `find_call`
        // result, so the arguments read here are exactly the arguments the
        // `ContractCall` below will read.
        let bound_reads = find_call(&texts).map_or_else(Vec::new, |(call_index, _)| {
            crate::aggregate_store::bound_read_actuals(
                &texts,
                call_index,
                self.contracts,
                &self.typed_model,
                &self.typed_types,
                &self.store_bases,
            )
        });
        borrowed_actuals.extend(bound_reads.iter().map(|read| (read.start, read.end)));
        // O04-B09B10: the member-address half of the same premise set — a whole
        // `Borrowed` actual `& BASE - > FIELD` whose member is declared with no
        // `*`. Its span leaves the lexical gate exactly as a bound read does;
        // when the base is one this body bound, [`transfer`] additionally
        // requires the aggregate to be live here. See
        // [`crate::aggregate_store::member_address_actuals`].
        let member_addresses = find_call(&texts).map_or_else(Vec::new, |(call_index, _)| {
            crate::aggregate_store::member_address_actuals(
                &texts,
                call_index,
                self.contracts,
                &self.typed_model,
                &self.typed_types,
                &self.store_bases,
                &self.typed_bases,
            )
        });
        borrowed_actuals.extend(
            member_addresses
                .iter()
                .map(|address| (address.start, address.end)),
        );
        // Card NEIGHBOURS-CHAINED-TAIL: an actual beyond the declared parameter
        // vector of a callee whose declaration carries
        // `resource_variadic_data_tail()`, when that actual is exactly one
        // admitted bounded typed access chain. Its span leaves the lexical gate
        // exactly as a C02-P borrowed actual does, and for the same reason: the
        // premises are the same predicate over the same declaration facts, and
        // the position is decided by the callee's declaration alone. Without
        // this the gate reports `indirect memory access` before
        // `tail_arguments_are_data` is ever consulted, which is the state card
        // NEIGHBOURS measured at `flb_input_chunk.c` L19581.
        //
        // Exempting the span is never on its own an admission: the tail rule
        // below still requires EVERY tail actual of this call to be data, so a
        // sibling actual carrying a tracked identity still refuses the whole
        // statement.
        let tail_accesses = find_call(&texts).map_or_else(Vec::new, |(call_index, function)| {
            let Some(contract) = self.contracts.contract(function) else {
                return Vec::new();
            };
            call_argument_spans(&texts, call_index).map_or_else(Vec::new, |spans| {
                crate::variadic_nested::tail_access_spans(
                    &texts,
                    &spans,
                    contract.parameters.len(),
                    self.variadic_tails.contains(function),
                    &self.typed_model,
                    &self.typed_types,
                    &self.typed_bases,
                )
            })
        });
        borrowed_actuals.extend(tail_accesses);
        // Card CONSTRUCTOR-RESIDUALS: an outer argument that is exactly a
        // dereference of an admitted nested call's result — `errno`, spelled
        // `(*__errno_location ())` after preprocessing. Its span leaves the
        // lexical gate exactly as a C02-P borrowed actual does; the decision is
        // [`crate::variadic_nested::admitted_nested_calls`]' own, premises N1-N10,
        // and exempting the span is never on its own an admission: the nested
        // rule below is consulted again and still governs the statement.
        let dereferenced = find_call(&texts).map_or_else(Vec::new, |(call_index, function)| {
            let Some(contract) = self.contracts.contract(function) else {
                return Vec::new();
            };
            let Some(spans) = call_argument_spans(&texts, call_index) else {
                return Vec::new();
            };
            crate::variadic_nested::dereferenced_argument_spans(
                &texts,
                call_index,
                &spans,
                contract,
                self.variadic_tails.contains(function),
                self.contracts,
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
            )
        });
        borrowed_actuals.extend(dereferenced);
        // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: the whole actuals the call
        // argument rule admitted for this statement, when no route above
        // already exempted them. Empty outside that rule's probe.
        let admitted_arguments = self.admitted_argument_spans(&texts, &borrowed_actuals);
        borrowed_actuals.extend(admitted_arguments);
        if let Some(equal) = assignment
            && !contains_call(tokens)
            && let Some(destination) = direct_assignment_destination(&texts[..equal])
            && !destination.contains(['.', '[', '*'])
            && !destination.contains("->")
            && crate::borrowed_actual::is_opaque_field_read(
                &texts[equal + 1..],
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
            )
        {
            return LegacyInstruction::OpaqueValue {
                destination,
                alias_read: self.read_alias(texts.get(equal + 1).copied()),
            };
        }
        // O00 blocker B08: one destination shape leaves the lexical gate — a
        // single `->` selection naming an admitted member of a local aggregate
        // pointer this body acquired. Every premise is typed and checked
        // against declarations from this same token stream; see
        // [`crate::aggregate_store`]. The *value* side is classified below and
        // the aggregate's liveness is required flow-sensitively by [`transfer`].
        let admitted_store = assignment.and_then(|equal| {
            let store = crate::aggregate_store::admitted_store(
                &texts[..equal],
                &self.store_bases,
                &self.typed_model,
                &self.typed_types,
            )?;
            // The importer's own canonical place must be exactly the admitted
            // one, or the destination this statement writes is not the one the
            // premises were checked against.
            (direct_assignment_destination(&texts[..equal]).as_deref() == Some(&store.place))
                .then_some(store)
        });
        // Card W44-COLLECTION-ALLOCATION: a `sizeof` dereference operand whose
        // type cannot be variably modified is never evaluated (C11 6.5.3.4p2),
        // so the gate below sees it as the scalar marker. Only for a statement
        // whose first call is an ordinary callee; everything after the gate
        // reads the original tokens. Empty (the gate unchanged) otherwise.
        let unevaluated = if find_call(&texts)
            .is_some_and(|(_, callee)| !TYPE_OPERATORS.contains(&callee))
        {
            let mut spans = crate::sizeof_operand::unevaluated_spans(&texts, &self.sizeof_operands);
            // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): operands Clang
            // shows unevaluated join the lexically admitted ones.
            let confirmed = self.clang_unevaluated_spans(tokens, &spans);
            spans.extend(confirmed);
            spans.sort_unstable();
            spans
        } else {
            Vec::new()
        };
        let gate_view = |from: usize| {
            if unevaluated.is_empty() {
                outside_spans(&texts, &borrowed_actuals, from)
            } else {
                crate::sizeof_operand::gate_view(&texts, &borrowed_actuals, &unevaluated, from)
            }
        };
        // Explicit Out stores remain a separate modeled instruction. Reading
        // through a pointer, member or subscript has no heap/liveness model and
        // must never disappear into scalar NoOp or borrowed-call argument erasure.
        let unsupported_access = if let Some(equal) = assignment {
            let destination = direct_assignment_destination(&texts[..equal]);
            // Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: a store into a
            // sub-object the reader no longer names is refused here too.
            (admitted_store.is_none()
                && (destination.as_deref().is_some_and(|place| {
                    place.contains('.') || place.contains('[') || place.contains("->")
                }) || names_a_sub_object(&texts[..equal])))
                || contains_unmodeled_access(&gate_view(equal + 1))
        } else if declaration_like
            && texts
                .first()
                .is_some_and(|first| DECLARATION_SPECIFIERS.contains(first))
        {
            // Declarator stars do not evaluate a pointer. A plain object
            // declaration with constant array bounds and no initializer
            // acquires nothing and evaluates nothing, so it is custody-neutral;
            // every other bracketed or dotted declaration still needs an
            // importer we do not have.
            let old = (texts.contains(&"[") && !constant_array_declaration(&texts))
                || texts.contains(&".");
            // Card MEMBER-ACCESS-PROJECTION (rule 38h): or Clang's AST shows
            // the same declaration with a constant-expression bound.
            self.typed_declaration_gate(tokens, old, declaration_like)
        } else {
            // Rule 38h: a typedef-specified declaration the token test reads
            // as an access, when Clang's AST shows it is one.
            self.typed_declaration_gate(
                tokens,
                contains_unmodeled_access(&gate_view(0)),
                declaration_like,
            )
        };
        // Card W45-CONTRACT-DEFERRED-REVIEW: a held call is refused before the
        // lexical access gate too, and ahead of every withdrawal, so the
        // statement names the pending review rather than an access.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| review_hold::held_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: review_hold::hold_description(callee),
                writes: self.refused_writes(&texts),
            };
        }
        // Card W45-CONTRACT-REVISION-MODE: likewise for a pin mismatch.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| pin_hold::held_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: pin_hold::hold_description(callee),
                writes: self.refused_writes(&texts),
            };
        }
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: likewise for a declaration its
        // own analysed body refutes.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| refuted_contract::held_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: refuted_contract::hold_description(callee),
                writes: self.refused_writes(&texts),
            };
        }
        // Card W48-RK-LOG-CALLER-FRAME (rule 32a): likewise for a call inside
        // its own caller frame.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| caller_frame_hold::held_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: caller_frame_hold::hold_description(callee),
                writes: self.refused_writes(&texts),
            };
        }
        // Card CONTRACT-SUPERSESSION: likewise for a function whose contract
        // bundles conflict.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| super::conflict_hold::held_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: super::conflict_hold::hold_description(callee),
                writes: self.refused_writes(&texts),
            };
        }
        // Card W45-SR5-SOUNDNESS: a name the source spelled with a reserved
        // prefix, escaped, refuses as an uncontracted call under its own name.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| reserved_spelling::escaped_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: reserved_spelling::escaped_call_description(callee),
                writes: self.refused_writes(&texts),
            };
        }
        // Card HASH-CONTRACTS. A call the premise rewrite withdrew is refused
        // *before* the lexical access gate, so the statement reports why the
        // declaration did not cover it rather than the access the withdrawn
        // declaration would have admitted. Both are the same `Unknown`; only
        // the attribution differs, and the informative one is the premise.
        if let Some(description) =
            find_call(&texts).and_then(|(_, function)| literal_premise::unmet_description(function))
        {
            return LegacyInstruction::Unknown {
                description,
                writes: self.refused_writes(&texts),
            };
        }
        // Card LOADER-ABI, for the same reason: a withdrawn declaration is the
        // informative attribution, and the access the withdrawn declaration
        // would have admitted is not.
        if let Some((refusal, callee)) =
            find_call(&texts).and_then(|(_, function)| declared_abi::withdrawn_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: format!(
                    "call to `{callee}` whose declared contract was withdrawn against its C \
                     declaration in this translation unit ({})",
                    refusal.as_str()
                ),
                writes: self.refused_writes(&texts),
            };
        }
        // Card LOADER-ABI-MODES, in the same position and for the same reason:
        // a declaration the callee's own body contradicts is the informative
        // attribution.
        if let Some((refusal, callee)) =
            find_call(&texts).and_then(|(_, function)| declared_modes::withdrawn_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: format!(
                    "call to `{callee}` whose declared contract was withdrawn against the \
                     summary derived from its body in this translation unit ({})",
                    refusal.as_str()
                ),
                writes: self.refused_writes(&texts),
            };
        }
        if unsupported_access {
            #[cfg(feature = "indirect-census")]
            {
                // Which arm of the gate's disjunction fired, recomputed from the
                // same locals it was computed from. Measurement only.
                let position = if let Some(equal) = assignment {
                    if admitted_store.is_none()
                        && (direct_assignment_destination(&texts[..equal])
                            .as_deref()
                            .is_some_and(|place| {
                                place.contains('.') || place.contains('[') || place.contains("->")
                            })
                            || names_a_sub_object(&texts[..equal]))
                    {
                        crate::indirect_census::Position::AssignmentDestination
                    } else {
                        crate::indirect_census::Position::AssignmentValue
                    }
                } else if declaration_like
                    && texts
                        .first()
                        .is_some_and(|first| DECLARATION_SPECIFIERS.contains(first))
                {
                    crate::indirect_census::Position::Declaration
                } else if find_call(&texts).is_some() {
                    crate::indirect_census::Position::CallArgument
                } else {
                    crate::indirect_census::Position::ExpressionStatement
                };
                self.census_indirect(
                    &texts,
                    &borrowed_actuals,
                    &crate::indirect_census::Refusal {
                        from: assignment.map_or(0, |equal| equal + 1),
                        assignment,
                        line: tokens.first().map_or(0, |token| token.line),
                        site: crate::indirect_census::RefusalSite::LexicalGate,
                        position,
                        call_contracted: find_call(&texts)
                            .map(|(_, function)| self.contracts.contract(function).is_some()),
                    },
                );
            }
            return LegacyInstruction::Unknown {
                description: "indirect memory access is outside the resource projection".into(),
                writes: self.refused_writes(&texts),
            };
        }
        let call = find_call(&texts);

        if let Some((call_index, function)) = call {
            if matches!(
                function,
                "sizeof" | "_Alignof" | "alignof" | "typeof" | "__typeof" | "__typeof__"
            ) {
                return LegacyInstruction::Unknown {
                    description: format!("C operator `{function}` requires type/operand analysis"),
                    writes: self.refused_writes(&texts),
                };
            }
            // Card O04-B09B10, a fail-closed repair of a pre-existing hole this
            // card's own subject made reachable. `find_call` returns the
            // *first* call of a statement, and everything below models exactly
            // that one. A second call nested inside an argument was therefore
            // dropped without a diagnostic: `k_borrow(k_take(1))` reported
            // `Safe` with no leak, discarding an owned non-null result the
            // top-level form `k_take(1);` refuses. Every nested call now
            // refuses. The C type operators keep their exemption — they
            // evaluate no operand and appear inside admitted arguments such as
            // `flb_calloc(n, sizeof(T))` — and an *outer* call this analyzer
            // does not model already refuses on its own.
            if let Some(contract) = self.contracts.contract(function).cloned() {
                // Card O04-TRACE, the two halves of card O04-B09B10's measured
                // residual, decided together because the subject needs both:
                // sibling nested calls may be admitted, and a declared variadic
                // tail may cover actuals beyond the declared parameter vector.
                let argument_spans = call_argument_spans(&texts, call_index);
                let variadic_tail = self.variadic_tails.contains(function);
                let nested = argument_spans.as_ref().and_then(|spans| {
                    crate::variadic_nested::admitted_nested_calls(
                        &texts,
                        call_index,
                        spans,
                        &contract,
                        variadic_tail,
                        self.contracts,
                        &self.typed_model,
                        &self.typed_types,
                        &self.typed_bases,
                    )
                });
                // Every nested call the premises do not admit keeps card
                // O04-B09B10's refusal, with its exact existing wording.
                if nested.is_none()
                    && let Some(other) = nested_call(&texts, call_index)
                {
                    return LegacyInstruction::Unknown {
                        description: format!(
                            "nested call `{other}` inside a modeled call is outside the resource projection"
                        ),
                        writes: self.refused_writes(&texts),
                    };
                }
                let mut call_depth = 0usize;
                let call_end =
                    texts
                        .iter()
                        .enumerate()
                        .skip(call_index + 1)
                        .find_map(|(index, text)| {
                            match *text {
                                "(" => call_depth += 1,
                                ")" => {
                                    call_depth = call_depth.saturating_sub(1);
                                    if call_depth == 0 {
                                        return Some(index);
                                    }
                                }
                                _ => {}
                            }
                            None
                        });
                let direct_call = call_index == 0
                    || assignment.is_some_and(|equal| {
                        call_index == equal + 1
                            && direct_assignment_destination(&texts[..equal]).is_some()
                    });
                // O00 blocker B08, two refinements that fire **only** for a
                // destination [`crate::aggregate_store`] admitted, so no other
                // statement in the corpus can move:
                //
                // 1. The aggregate's own name is the modeled destination, not
                //    "a formal embedded in an unmodeled expression". Its
                //    liveness and single identity are required flow-sensitively
                //    by [`transfer`], which is a stronger check than the
                //    syntactic one this gate performs. A protected name in the
                //    *right-hand side* still refuses.
                // 2. One leading parenthesized cast between `=` and the call
                //    does not break the direct flow of the call's result into
                //    the member: it converts the result's declared type, and
                //    this projection tracks identities, not types. The call
                //    must still be the whole remainder of the statement.
                let (mentions_formal, direct_call) = match (&admitted_store, assignment) {
                    (Some(_), Some(equal)) => (
                        texts[equal + 1..]
                            .iter()
                            .any(|name| self.protected_formals.contains(*name)),
                        call_index == equal + 1 || is_leading_cast(&texts[equal + 1..call_index]),
                    ),
                    _ => (mentions_formal, direct_call),
                };
                if mentions_formal && (!direct_call || call_end != Some(texts.len() - 1)) {
                    return LegacyInstruction::Unknown {
                        description:
                            "resource call with a formal embedded in an unmodeled expression".into(),
                        writes: self.refused_writes(&texts),
                    };
                }
                let destination =
                    assignment.and_then(|equal| direct_assignment_destination(&texts[..equal]));
                // A call result carrying custody may only be stored into a
                // member declared with exactly one `*`; a scalar member cannot
                // hold an identity, and no other member shape is admitted.
                // Card W43-LIBC-RESULT-ALIAS: a derived result is never stored
                // into a member either. A field link would make an escape of
                // the aggregate cascade to the *base*, which the derived
                // pointer does not own.
                if let Some(store) = &admitted_store
                    && !matches!(contract.result, ResourceReturnMode::None)
                    && (!store.pointer_field
                        || matches!(contract.result, ResourceReturnMode::Derived { .. }))
                {
                    #[cfg(feature = "indirect-census")]
                    self.census_indirect(
                        &texts,
                        &borrowed_actuals,
                        &crate::indirect_census::Refusal {
                            from: 0,
                            assignment,
                            line: tokens.first().map_or(0, |token| token.line),
                            site: crate::indirect_census::RefusalSite::NonPointerResultStore,
                            position: crate::indirect_census::Position::AssignmentDestination,
                            // This arm is only reached through a contracted
                            // call, so the answer is fixed.
                            call_contracted: Some(true),
                        },
                    );
                    return LegacyInstruction::Unknown {
                        description: "indirect memory access is outside the resource projection"
                            .into(),
                        writes: self.refused_writes(&texts),
                    };
                }
                // Card W43-LIBC-RESULT-ALIAS: a discarded derived result is
                // exactly a discarded `None` — it owns nothing, and no later
                // statement can use a value nothing holds (`memcpy(d, s, n);`).
                if !matches!(
                    contract.result,
                    ResourceReturnMode::None | ResourceReturnMode::Derived { .. }
                ) && destination.is_none()
                {
                    return LegacyInstruction::Unknown {
                        description: format!("discarded owned result of resource API `{function}`"),
                        writes: self.refused_writes(&texts),
                    };
                }
                let nested = nested.unwrap_or_default();
                let raw_arguments = argument_spans.as_ref().map(|spans| {
                    spans
                        .iter()
                        .map(|&(start, end)| &texts[start..end])
                        .collect::<Vec<_>>()
                });
                // Card O04-TRACE, a fail-closed repair. Everything below zips
                // the declared parameter vector against the actuals, so an
                // actual *beyond* that vector was silently dropped — the same
                // class of hole card O04-B09B10 found for a nested call, and
                // the reason card O04-ARMS had to declare `flb_error` at one
                // arity. An excess actual is now admitted only under a declared
                // variadic data tail, and only when this call site's own tail
                // actuals are data. Under-supply is unchanged: a missing
                // custody-bearing actual already reports on its own.
                let mut widened_tail = None;
                if let Some(arguments) = &raw_arguments
                    && arguments.len() > contract.parameters.len()
                    && !(variadic_tail
                        && crate::variadic_nested::tail_arguments_are_data_with_nested(
                            // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: an admitted
                            // tail read is data; every other actual is unchanged.
                            &self.tail_view(function, arguments),
                            contract.parameters.len(),
                            &nested,
                            &self.typed_model,
                            &self.typed_types,
                            &self.typed_bases,
                        ))
                {
                    // Card CG26-VARIADICS (rule 11w): only under the statement
                    // fallback's widening, each trailing actual read as a
                    // declared `Borrowed` one (T7, T8).
                    if variadic_tail
                        && self.variadic_tail_widening.get()
                        && self.tail_widened(
                            function,
                            arguments,
                            contract.parameters.len(),
                            &nested,
                        )
                    {
                        widened_tail = Some(arguments.len() - contract.parameters.len());
                    } else {
                        return LegacyInstruction::Unknown {
                            description: format!(
                                "arguments beyond the declared parameters of `{function}` are outside the resource projection"
                            ),
                            writes: self.refused_writes(&texts),
                        };
                    }
                }
                let contract = match widened_tail {
                    Some(tail) => crate::variadic_tail::borrowed_tail_contract(&contract, tail),
                    None => contract,
                };
                // O04-B09B10: an argument this statement's member-address rule
                // admitted is not "an unmodeled formal address". Its premises —
                // a complete `struct` base, a member declared with no `*`, a
                // declared `Borrowed` position, and a live aggregate required
                // flow-sensitively — are exactly the frame condition this gate
                // otherwise has no way to establish. Every other `&` argument
                // mentioning a protected name keeps its refusal.
                let admitted_addresses = member_addresses
                    .iter()
                    .map(|address| &texts[address.start..address.end])
                    .collect::<Vec<_>>();
                // Card O04-TRACE: likewise, an *admitted nested call* is no
                // longer "an unmodeled borrowed expression" — its declared
                // effects are applied below, in order, before the outer call's.
                // Every other call-bearing argument keeps its refusal, because
                // every other one was refused outright above.
                if raw_arguments.as_ref().is_some_and(|arguments| {
                    contract.parameters.iter().zip(arguments).enumerate().any(
                        |(index, (mode, argument))| {
                            !nested.iter().any(|nested| nested.argument == index)
                                && !admitted_addresses.contains(argument)
                                // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: an
                                // admitted read keeps its base's obligation.
                                && !self.argument_admitted(function, index, argument)
                                // Card CG25-RESOURCE-KINDS (rule 11v): `&NAME`
                                // of a tracked object at an in-place position
                                // is that position's place, decided by
                                // `in_place_actuals` below.
                                && !self.in_place_address(&contract, index, argument)
                                // Card SLOT-MOVE-AND-INTERIOR-BORROW (rules
                                // 12x, 12z): `&NAME` at a slot position is
                                // decided by the slot's own rule.
                                && !self.slot_address(function, index, argument)
                                && (((!matches!(mode, ResourceParameterMode::Out { .. })
                                    && argument.contains(&"&"))
                                    || (matches!(mode, ResourceParameterMode::Borrowed)
                                        && canonical_place(argument).is_none()))
                                    // Card W44-COLLECTION-ALLOCATION: a name in
                                    // an unevaluated `sizeof` operand is passed
                                    // nowhere; with no operand this is every
                                    // token of the argument, as before.
                                    && crate::sizeof_operand::evaluated(
                                        argument,
                                        argument_spans
                                            .as_ref()
                                            .and_then(|spans| spans.get(index))
                                            .map_or(0, |span| span.0),
                                        &unevaluated,
                                    )
                                    .any(|name| self.protected_formals.contains(name)))
                        },
                    )
                }) {
                    return LegacyInstruction::Unknown {
                        description: "call has an unmodeled formal address or borrowed expression"
                            .into(),
                        writes: self.refused_writes(&texts),
                    };
                }
                let mut arguments = contract
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, mode)| {
                        raw_arguments
                            .as_ref()
                            .and_then(|arguments| arguments.get(index))
                            .and_then(|argument| contract_argument_place(argument, *mode))
                    })
                    .collect::<Vec<_>>();
                // Card CG25-RESOURCE-KINDS (rule 11v): an in-place position's
                // place is the automatic object its `&NAME` actual designates.
                if let Err(description) = self.in_place_actuals(
                    &contract,
                    raw_arguments.as_ref(),
                    destination.as_deref(),
                    &mut arguments,
                ) {
                    return LegacyInstruction::Unknown {
                        description,
                        writes: self.refused_writes(&texts),
                    };
                }
                // A proved null actual of the removed result is a value,
                // not a missing argument. `0` cannot name a C object. Raw
                // NULL identifiers are excluded: preprocessing must establish
                // their value before this new route interprets them.
                if let ResourceReturnMode::Removed { parameter, .. } = contract.result
                    && raw_arguments
                        .as_ref()
                        .and_then(|args| args.get(parameter))
                        .is_some_and(|argument| {
                            !argument
                                .iter()
                                .any(|word| matches!(*word, "NULL" | "nullptr"))
                                && return_call::is_null_pointer_constant(argument)
                        })
                {
                    arguments[parameter] = Some("0".to_owned());
                }
                // Card W43-LIBC-RESULT-ALIAS: a derived result names its base,
                // and the base must be something this projection can look up.
                // `derived_base` reads a place, a place plus or minus one
                // offset token (which stays inside the same object), or a base
                // no tracked identity can stand for. Any other expression (a
                // cast, a call, a member address) could denote a tracked object
                // under a name the binding would lose, so a later release of
                // that object would not invalidate the result: a *bound* result
                // over one is refused rather than bound to nothing. A discarded
                // result binds nothing and keeps its ordinary argument places.
                let mut offset_base = false;
                if let ResourceReturnMode::Derived {
                    parameter, extent, ..
                } = contract.result
                {
                    match raw_arguments
                        .as_ref()
                        .and_then(|arguments| arguments.get(parameter))
                        .map(|argument| match derived_base(argument) {
                            // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a
                            // borrowed result's parent may be spelled through a
                            // pointer cast of a place.
                            DerivedBase::Unmodeled
                                if extent == qcore_schemas::contract::DerivedExtent::Borrowed =>
                            {
                                cast_place(argument).map_or(DerivedBase::Unmodeled, |place| {
                                    DerivedBase::Place {
                                        place,
                                        offset: false,
                                    }
                                })
                            }
                            base => base,
                        }) {
                        Some(DerivedBase::Place { place, offset }) => {
                            arguments[parameter] = Some(place);
                            offset_base = offset;
                        }
                        Some(DerivedBase::Untracked) => arguments[parameter] = None,
                        Some(DerivedBase::Unmodeled) | None if destination.is_some() => {
                            return LegacyInstruction::Unknown {
                                description: format!(
                                    "derived result of `{function}` from an unmodeled base expression"
                                ),
                                writes: self.refused_writes(&texts),
                            };
                        }
                        Some(DerivedBase::Unmodeled) | None => {}
                    }
                }
                // Card W43-RESULT-ALIAS-RETURNED: `memcpy(p + n, …)` returns
                // `p + n`, not `p`. A pointer offset from its base still points
                // into the base's object (C11 6.5.6p8), so it is derived with
                // the *interior* extent, never as an exact alias of the base.
                let contract = if offset_base {
                    interior_extent(contract)
                } else {
                    contract
                };
                let produced_count = usize::from(!matches!(
                    contract.result,
                    ResourceReturnMode::None
                        | ResourceReturnMode::Derived { .. }
                        | ResourceReturnMode::Removed { .. }
                )) + contract
                    .parameters
                    .iter()
                    .filter(|mode| matches!(mode, ResourceParameterMode::Out { .. }))
                    .count();
                let produced_sites = (0..produced_count)
                    .map(|_| {
                        let site = self.next_site;
                        self.next_site += 1;
                        site
                    })
                    .collect();
                let mut alias_reads = self.call_alias_reads(&arguments, &contract);
                // The admitted nested call's own reads carry the same
                // may-alias obligation the outer call's reads do.
                for nested in &nested {
                    let inner = self.call_alias_reads(&nested.arguments, &nested.contract);
                    for read in inner {
                        if !alias_reads
                            .iter()
                            .any(|existing| existing.place == read.place)
                        {
                            alias_reads.push(read);
                        }
                    }
                }
                // Card W45-FB-RETRY-LIFETIME: a declared container effect's
                // operands, read from the actuals as spelled.
                let container = container_publication::operands(
                    &contract,
                    raw_arguments.as_deref(),
                    &self.caller_formals,
                );
                // Capture only the final accepted local fragment. These records
                // do not assert that a later whole-function projection succeeds.
                for selected in &nested {
                    if selected.function == "strerror"
                        || (selected.function == "__errno_location" && selected.dereferenced)
                    {
                        use crate::reading_site_evidence::{Capture, Route, Sibling, Span};
                        crate::reading_site_evidence::capture(Capture {
                            function: &self.typed_path_function,
                            declaration: &selected.function,
                            route: Route::NestedArgument,
                            full: self.tokens,
                            statement: tokens,
                            call: Span {
                                start: selected.start,
                                end: selected.end,
                            },
                            outer_argument: Some(Span {
                                start: selected.argument_start,
                                end: selected.argument_end,
                            }),
                            siblings: nested
                                .iter()
                                .map(|n| Sibling {
                                    function: n.function.clone(),
                                    argument: n.argument,
                                    call: Span {
                                        start: n.start,
                                        end: n.end,
                                    },
                                    outer_argument: Span {
                                        start: n.argument_start,
                                        end: n.argument_end,
                                    },
                                    dereferenced: n.dereferenced,
                                })
                                .collect(),
                            contract: &selected.contract,
                            environment: self.contracts,
                            checked_predicates: [
                                "whole-argument-or-checked-dereference",
                                "all-sibling-nested-sites-accepted",
                                "one-level-no-inner-call",
                                "in-force-loaded-contract",
                                "declared-no-custody-result",
                                "declared-all-parameters-borrowed-no-container-effect",
                                "outer-position-borrowed-or-declared-tail",
                                "inner-arity-and-data-actuals-accepted",
                                "enclosing-call-final-local-checks-accepted",
                            ]
                            .into_iter()
                            .map(|id| (id.to_owned(), true))
                            .collect(),
                        });
                    }
                }
                return LegacyInstruction::ContractCall {
                    container,
                    retention: None,
                    function: function.to_owned(),
                    destination,
                    arguments,
                    contract,
                    produced_sites,
                    aggregate_reads: bound_reads.iter().map(|read| read.place.clone()).collect(),
                    aggregate_consumed: bound_reads
                        .iter()
                        .filter(|read| read.custody)
                        .map(|read| read.place.clone())
                        .collect(),
                    aggregate_addresses: member_addresses
                        .iter()
                        .filter_map(|address| address.bound_owner.clone())
                        .collect(),
                    alias_reads,
                    nested: nested
                        .into_iter()
                        .map(|nested| NestedContractCall {
                            function: nested.function,
                            contract: nested.contract,
                            arguments: nested.arguments,
                        })
                        .collect(),
                };
            }
            // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the seven call holds
            // (review, pin and refutation holds, escaped spelling, literal
            // premise, ABI and mode withdrawals) are answered before the
            // lexical gate above, for this same `function` (`find_call` over
            // the same texts) by the same pure name readers, so a held callee
            // never reaches this arm; the repeated checks that stood here were
            // unreachable and are removed.
            // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): an extension
            // node, a returns-twice call or a non-local exit is named; only the
            // wording changes.
            if let Some(description) = crate::call_exits::undeclarable_call(function) {
                return LegacyInstruction::Unknown {
                    description,
                    writes: self.refused_writes(&texts),
                };
            }
            // Card CALLEE-RETENTION-CLASS1 (rule 12t): a retaining callee,
            // classified under its custody contract where its retentions can
            // be decided, refused where they cannot.
            if let Some(instruction) = self.retaining_call(tokens, &texts, call_index, function) {
                return instruction;
            }
            // Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): a callee whose
            // return-code disposition this use selects no class of.
            if let Some(description) = relational_route::untested(self.contracts, function) {
                return LegacyInstruction::Unknown {
                    description,
                    writes: self.refused_writes(&texts),
                };
            }
            return LegacyInstruction::Unknown {
                description: format!("call to uncontracted function `{function}`"),
                writes: self.refused_writes(&texts),
            };
        }

        // O04: a statement over declared scalars, enumeration constants and
        // numeric literals alone is custody-neutral — this projection tracks
        // pointer identities, and a declared scalar is never bound to one. It
        // retains explicit zero assignments and variable-to-variable aliases;
        // every other admitted scalar write forgets its old branch fact.
        let scalar_only = !contains_call(tokens)
            && crate::typed_field_access::is_scalar_only_noop(
                &texts,
                &self.typed_model,
                &self.typed_types,
            );
        if let Some(store) = &admitted_store
            && let Some(equal) = assignment
        {
            let right = &texts[equal + 1..];
            let Some(value) = crate::aggregate_store::classify_value(
                store,
                right,
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
                is_null_constant,
            ) else {
                // A value shape the premises do not cover keeps the existing
                // refusal and the existing diagnostic.
                #[cfg(feature = "indirect-census")]
                self.census_indirect(
                    &texts,
                    &borrowed_actuals,
                    &crate::indirect_census::Refusal {
                        from: equal + 1,
                        assignment,
                        line: tokens.first().map_or(0, |token| token.line),
                        site: crate::indirect_census::RefusalSite::UnclassifiedStoreValue,
                        position: crate::indirect_census::Position::AssignmentValue,
                        call_contracted: find_call(&texts)
                            .map(|(_, function)| self.contracts.contract(function).is_some()),
                    },
                );
                return LegacyInstruction::Unknown {
                    description: "indirect memory access is outside the resource projection".into(),
                    writes: self.refused_writes(&texts),
                };
            };
            return match value {
                crate::aggregate_store::StoredValue::Untracked => {
                    LegacyInstruction::AggregateStore {
                        place: store.place.clone(),
                    }
                }
                crate::aggregate_store::StoredValue::Null => LegacyInstruction::Clear {
                    destination: store.place.clone(),
                },
                crate::aggregate_store::StoredValue::Tracked(source) => LegacyInstruction::Alias {
                    destination: store.place.clone(),
                    source,
                },
            };
        }
        if let Some(equal) = assignment {
            let Some(destination) = direct_assignment_destination(&texts[..equal]) else {
                if scalar_only {
                    return self.scalar_effect(&texts);
                }
                return LegacyInstruction::Unknown {
                    description: "unrecognized assignment destination".to_owned(),
                    writes: self.refused_writes(&texts),
                };
            };
            let right = &texts[equal + 1..];
            if is_null_constant(right) {
                return LegacyInstruction::Clear {
                    destination: destination.to_owned(),
                };
            }
            // The declaration form `int flag = FLAG_OFF;`, whose leading type
            // specifiers the statement walker refuses.
            let scalar_only = scalar_only
                || (!contains_call(tokens)
                    && crate::typed_field_access::is_scalar_initializer(
                        &destination,
                        right,
                        &self.typed_model,
                        &self.typed_types,
                    ));
            if let Some(source) = canonical_place(right) {
                // An enumeration constant names no place this projection can
                // ever bind, so aliasing it only reports an unresolved alias.
                if scalar_only && self.typed_model.is_enum_constant(&source) {
                    return self.scalar_effect(&texts);
                }
                return LegacyInstruction::Alias {
                    destination: destination.to_owned(),
                    source,
                };
            }
            if scalar_only {
                return self.scalar_effect(&texts);
            }
            return LegacyInstruction::Unknown {
                description: "unsupported assignment expression".to_owned(),
                writes: self.refused_writes(&texts),
            };
        }

        LegacyInstruction::NoOp
    }
}
