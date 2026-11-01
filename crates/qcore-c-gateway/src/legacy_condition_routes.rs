//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the expression and condition
//! readers the statement and condition routes ask (ROUTE-MAP sections 3 and
//! 6): writes, scalar effects, the supported test, the condition call,
//! access, null and distinct routes, alias reads.
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

impl Parser<'_> {
    /// The complete expression was delimited by the parser; malformed
    /// control flow never reaches this constructor.
    pub(super) fn expression_writes(&self, tokens: &[Token]) -> RefusedWrites {
        self.refused_writes(
            &tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
        )
    }

    /// Custody-neutral scalar syntax can still overwrite a branch fact.
    /// Keep a single recognized scalar write; refuse multiple writes rather
    /// than silently retaining either old value.
    pub(super) fn scalar_effect(&self, texts: &[&str]) -> LegacyInstruction {
        let written = may_write_names(texts)
            .into_iter()
            .filter(|name| {
                crate::typed_field_access::is_scalar_initializer(
                    name,
                    &["1"],
                    &self.typed_model,
                    &self.typed_types,
                )
            })
            .collect::<Vec<_>>();
        match written.as_slice() {
            [] => LegacyInstruction::NoOp,
            [destination] => LegacyInstruction::ForgetScalar {
                destination: destination.clone(),
            },
            _ => LegacyInstruction::Unknown {
                description: "multiple scalar writes in one expression are not modeled".into(),
                writes: self.refused_writes(texts),
            },
        }
    }

    /// Whether one expression position is resource-neutral.
    ///
    /// Card W4-EXPR-POS. The existing lexical predicate is consulted **first**
    /// and unchanged, so every expression this project accepted before is
    /// accepted here by the same code; only when it refuses does the typed,
    /// environment-aware admission of [`crate::neutral_position`] get a say.
    /// That admission is where the contract environment finally reaches the
    /// classification — the architectural change
    /// `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` §5 asks for — and it
    /// admits nothing the lexical predicate would have had to loosen for.
    /// Card REACH-GATE threads the position's *role* through: a condition
    /// discards the value it computes into a control-flow decision, while a
    /// `return` expression hands it to the caller. Only the pointer-valued leaf
    /// of the access-chain family reads the difference; every other family
    /// decides identically in both roles.
    pub(super) fn supported_expression(&self, tokens: &[Token], role: ExpressionRole) -> bool {
        // Card W45-PARENTHESISED-CALLEE: a call through a callee expression
        // that is not a name is never neutral.
        if self.opaque_callee(tokens) {
            return false;
        }
        // Expression-position writes need an evaluation model. In particular
        // a condition must not erase an increment into a neutral CFG node.
        if tokens.windows(2).any(|pair| {
            matches!(
                (pair[0].text.as_str(), pair[1].text.as_str()),
                ("+", "+") | ("-", "-")
            )
        }) {
            return false;
        }
        if tokens.iter().enumerate().any(|(i, token)| {
            let previous = i.checked_sub(1).map(|p| tokens[p].text.as_str());
            token.text == "="
                && tokens.get(i + 1).is_none_or(|t| t.text != "=")
                && (!matches!(previous, Some("=" | "!" | "<" | ">"))
                    || (matches!(previous, Some("<" | ">"))
                        && i.checked_sub(2).map(|p| tokens[p].text.as_str()) == previous))
        }) {
            return false;
        }
        // A scalar member of a record pointer still evaluates its base.  The
        // older lexical route used to accept that spelling as neutral and
        // erase the read before the typed position routes could check it.
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if crate::typed_field_access::scalar_field_noop_pointer_bases(
            &texts,
            &self.typed_model,
            &self.typed_types,
        )
        .is_some_and(|bases| !bases.is_empty())
        {
            return false;
        }
        if supported_neutral_expression(tokens) {
            return true;
        }
        crate::neutral_position::is_neutral_expression(
            &texts,
            role,
            self.contracts,
            &self.typed_model,
            &self.typed_types,
        )
    }

    /// The one unconditionally evaluated modeled call in a controlling
    /// expression.
    ///
    /// Cards B02 and B03 decompose a condition into one isolated call and a
    /// custody-neutral residual. The lexical boundary rejects every shape in
    /// which C may skip the call. The residual is checked by the existing
    /// expression-position rule, and the call is checked by the ordinary
    /// statement classifier, so there is one contract-effect model.
    ///
    /// A declined speculative classification restores `next_site`, preserving
    /// site numbering and certificate identity for every expression this rule
    /// keeps refused.
    pub(super) fn condition_call_instruction(
        &mut self,
        condition: &[Token],
    ) -> Option<LegacyInstruction> {
        // Card W45-PARENTHESISED-CALLEE.
        if self.opaque_callee(condition) {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (start, end) = crate::legacy_condition_call::unconditional_call_span(&texts)?;
        // This widening is for a call whose contract is established outside
        // the translation unit. A locally defined callee can carry a
        // provisional neutral summary while its recursive component is being
        // validated; reading that assumption back here would widen the
        // pointer-actual boundary by circularity rather than by this card's
        // declared-contract premise.
        if find_function_body(self.tokens, texts[start]).is_some() {
            return None;
        }

        let mut residual = Vec::with_capacity(condition.len() - (end - start) + 1);
        residual.extend_from_slice(&condition[..start]);
        residual.push(Token {
            text: "0".to_owned(),
            line: condition[start].line,
        });
        residual.extend_from_slice(&condition[end..]);
        if !self.supported_expression(&residual, ExpressionRole::Decides) {
            return None;
        }

        // Card GAPO-RETURN-CORRECTNESS (rule 15x): a declined probe hands
        // back the whole probe state.
        self.probe(|parser| {
            let instruction = parser.classify_simple(&condition[start..end]);
            if matches!(instruction, LegacyInstruction::ContractCall { .. }) {
                probe_state::Probe::Keep(Some(instruction))
            } else {
                probe_state::Probe::Decline(None)
            }
        })
    }

    /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused controlling
    /// expression lowers to.
    ///
    /// Asked only where every existing route refused the condition: it is not
    /// a supported neutral expression, no contracted call was isolated from
    /// it, and (for the branch positions) it is no null test. The condition
    /// must carry no call and no brace, and
    /// [`crate::typed_field_access::typed_scalar_condition`] must admit it:
    /// every memory access in it is `*B`, `B[I]`, `B->F` or `S.F` over the
    /// declarations the statement route reads, and nothing in it writes. It
    /// lowers to [`LegacyInstruction::IndirectScalarAccess`] with no
    /// destination and no frame — a read that changes no binding — whose bases
    /// keep their dereference obligation at the header. Bounds are not
    /// decided.
    pub(super) fn condition_access(&self, condition: &[Token]) -> Option<Box<LegacyInstruction>> {
        let access = self.condition_access_routes(condition)?;
        let access = Box::new(self.scalar_condition_alternative(condition, *access));
        // Card W47-ACCESS-PATH-POSITIONS: parity only.
        self.record_typed_position_legacy(condition, "condition", "condition-access", &access);
        Some(access)
    }

    pub(super) fn condition_access_routes(
        &self,
        condition: &[Token],
    ) -> Option<Box<LegacyInstruction>> {
        // Card W45-PARENTHESISED-CALLEE: `self.opaque_callee`.
        if contains_call(condition) || contains_braces(condition) || self.opaque_callee(condition) {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some(plan) = crate::typed_field_access::typed_scalar_condition(
            &texts,
            &self.typed_model,
            &self.typed_types,
        ) else {
            // Card W44-INDIRECT-CHAIN-ACCESS: the same typed read with
            // record-pointer member chains as accesses.
            let Some(chained) = crate::indirect_chain_access::chain_condition(
                &texts,
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
            ) else {
                // Card W45-CALLEE-SUMMARY-CASCADE: a pointer member null test
                // is the same custody-neutral typed read of its base.
                let base =
                    member_release::member_null_test(&texts, &self.typed_model, &self.typed_types)?;
                return Some(Box::new(LegacyInstruction::IndirectScalarAccess {
                    typed_alternative: None,
                    alias_reads: self.read_alias(Some(&base)).into_iter().collect(),
                    bases: vec![base],
                    destination: None,
                    frame: None,
                    chains: Vec::new(),
                    borrowed_destination: false,
                    readback: None,
                }));
            };
            return Some(Box::new(self.chain_access_instruction(chained, false)));
        };
        Some(Box::new(LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.bases,
            destination: None,
            frame: None,
            chains: Vec::new(),
            borrowed_destination: false,
            readback: None,
        }))
    }

    /// The null test a branch condition expresses, including the one
    /// aggregate-field form card O04-B08 admits.
    ///
    /// The general refusal of `->` in a condition is unchanged: only a null
    /// test whose *entire* operand is an admitted single-selection **pointer**
    /// field of an admitted aggregate base is read here, and only when the
    /// condition carries no call, no braces and no other unmodeled access.
    pub(super) fn null_condition(&self, condition: &[Token]) -> Option<PointerNullCondition> {
        // Card W45-PARENTHESISED-CALLEE.
        if self.opaque_callee(condition) {
            return None;
        }
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: a test of a descriptor name is
        // a validity test, never a null test — descriptor `0` is valid, so a
        // null-shaped test of one (`if (fd)`) must not remove its site.
        if !self.descriptor_names.is_empty() {
            let texts = condition
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            if supported_neutral_expression(condition)
                && let Some(test) =
                    descriptor_custody::validity_test(&texts, &self.descriptor_names)
            {
                return Some(PointerNullCondition {
                    source: test.source,
                    true_is_null: false,
                    validity: Some((test.when_true, test.when_false)),
                });
            }
        }
        // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): a validity test of a
        // cell-creating call's witness decides the creation's class.
        if !self.cell_witness_names.is_empty() {
            let texts = condition
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            if supported_neutral_expression(condition)
                && let Some(test) =
                    descriptor_custody::validity_test(&texts, &self.cell_witness_names)
            {
                return Some(PointerNullCondition {
                    source: test.source,
                    true_is_null: false,
                    validity: Some((test.when_true, test.when_false)),
                });
            }
        }
        if let Some(existing) = pointer_null_condition(condition) {
            return Some(existing);
        }
        if contains_call(condition) || contains_braces(condition) {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        self.aggregate_null_condition(&texts)
    }

    pub(super) fn aggregate_null_condition(&self, texts: &[&str]) -> Option<PointerNullCondition> {
        let texts = strip_parentheses(texts);
        if let Some(rest) = texts.strip_prefix(&["!"]) {
            let mut condition = self.aggregate_null_condition(rest)?;
            condition.true_is_null = !condition.true_is_null;
            return Some(condition);
        }
        if let Some(place) = self.admitted_field_operand(texts) {
            return Some(PointerNullCondition {
                source: place,
                true_is_null: false,
                validity: None,
            });
        }
        for index in 0..texts.len().saturating_sub(1) {
            if !matches!(texts[index], "=" | "!") || texts[index + 1] != "=" {
                continue;
            }
            let left = &texts[..index];
            let right = &texts[index + 2..];
            let source = if is_null_constant(left) {
                self.admitted_field_operand(right)?
            } else if is_null_constant(right) {
                self.admitted_field_operand(left)?
            } else {
                return None;
            };
            return Some(PointerNullCondition {
                source,
                true_is_null: texts[index] == "=",
                validity: None,
            });
        }
        None
    }

    /// The place named by an operand that is exactly `BASE - > FIELD` on an
    /// admitted base whose member is a single-star pointer.
    pub(super) fn admitted_field_operand(&self, texts: &[&str]) -> Option<String> {
        let texts = strip_parentheses(texts);
        let [base, "-", ">", field] = texts else {
            return None;
        };
        let store = crate::aggregate_store::admitted_field(
            base,
            field,
            &self.store_bases,
            &self.typed_model,
            &self.typed_types,
        )?;
        store.pointer_field.then_some(store.place)
    }

    /// C02-P-ALIAS: the pointer-inequality guard a branch condition expresses.
    ///
    /// Exactly `LEFT == RIGHT` or `LEFT != RIGHT`, optionally negated by
    /// leading `!` and parenthesized, where both operands are plain declared
    /// names the may-alias rule relates to each other. Anything else — a
    /// member access, a call, a cast, a comparison against a constant — yields
    /// `None`, and the branch keeps the shape it has today.
    pub(super) fn distinct_condition(
        &self,
        condition: &[Token],
    ) -> Option<PointerDistinctCondition> {
        // Card W45-PARENTHESISED-CALLEE: `self.opaque_callee`.
        if !supported_neutral_expression(condition) || self.opaque_callee(condition) {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let mut found = self.distinct_condition_text(&texts)?;
        // The rule is symmetric in its two names; normalise so that a
        // certificate cannot depend on the spelling order of the comparison.
        if found.left > found.right {
            std::mem::swap(&mut found.left, &mut found.right);
        }
        Some(found)
    }

    pub(super) fn distinct_condition_text(
        &self,
        texts: &[&str],
    ) -> Option<PointerDistinctCondition> {
        let texts = strip_parentheses(texts);
        if let Some(rest) = texts.strip_prefix(&["!"]) {
            let mut condition = self.distinct_condition_text(rest)?;
            condition.true_is_distinct = !condition.true_is_distinct;
            return Some(condition);
        }
        let mut depth = 0usize;
        for index in 0..texts.len().saturating_sub(1) {
            match texts[index] {
                "(" | "[" => depth += 1,
                ")" | "]" => depth = depth.checked_sub(1)?,
                "=" | "!" if depth == 0 && texts[index + 1] == "=" => {
                    let (left, right) = (
                        strip_parentheses(&texts[..index]),
                        strip_parentheses(&texts[index + 2..]),
                    );
                    let ([left], [right]) = (left, right) else {
                        return None;
                    };
                    if !is_identifier(left) || !is_identifier(right) || left == right {
                        return None;
                    }
                    // Both operands must be names this rule relates, or the
                    // guard proves nothing it could discharge.
                    if !self
                        .alias_classes
                        .peers(left)
                        .iter()
                        .any(|peer| peer == right)
                    {
                        return None;
                    }
                    return Some(PointerDistinctCondition {
                        left: (*left).to_owned(),
                        right: (*right).to_owned(),
                        true_is_distinct: texts[index] == "!",
                    });
                }
                _ => {}
            }
        }
        None
    }

    /// C02-P-ALIAS: the may-alias obligations of one modeled call's *read*
    /// arguments, in a stable order and deduplicated by root name.
    ///
    /// An `Out` position is deliberately excluded: its actual is the address of
    /// a slot the callee writes, not a pointer this call dereferences
    /// (`C02-P-ALIAS-USES`).
    pub(super) fn call_alias_reads(
        &self,
        arguments: &[Option<String>],
        contract: &ResourceFunctionContract,
    ) -> Vec<AliasPeers> {
        let mut roots = BTreeSet::new();
        for (index, mode) in contract.parameters.iter().enumerate() {
            if matches!(mode, ResourceParameterMode::Out { .. }) {
                continue;
            }
            let Some(place) = arguments.get(index).and_then(Option::as_deref) else {
                continue;
            };
            if let Some(root) = crate::may_alias::place_root(place) {
                roots.insert(root.to_owned());
            }
        }
        roots
            .into_iter()
            .filter_map(|place| {
                let peers = self.alias_classes.peers(&place);
                (!peers.is_empty()).then_some(AliasPeers { place, peers })
            })
            .collect()
    }

    /// C02-P-ALIAS: the may-alias obligation of one admitted opaque field
    /// read's base.
    pub(super) fn read_alias(&self, base: Option<&str>) -> Option<AliasPeers> {
        let place = base?;
        let peers = self.alias_classes.peers(place);
        (!peers.is_empty()).then(|| AliasPeers {
            place: place.to_owned(),
            peers,
        })
    }
}
