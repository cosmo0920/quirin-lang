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

//! Card **MEMBER-EFFECT-STORES**, body half (gateway rule 38k): a store
//! `P->F… = V` into a member of the caller's object, read as a **hand-off**
//! of `V`'s custody.
//!
//! USER DECISIONS 2026-11-25 (verbatim): "Option 1: caller-slot premise", and
//! "Plus, banned for just written down as English description. We must write
//! down with logical symbols for them." Every condition below is therefore a
//! formula of the C04.18.9 typed rendering over the MES terms
//! (`qcore_frontend::premise_formula::RENDERED_TERMS`, block `MES`), checked
//! by the premise-formula validator ([`validated`]); the prose beside each is
//! a gloss.
//!
//! ## The admission: every formula of [`CONDITIONS`] holds
//!
//! | id | formula |
//! |---|---|
//! | `mes_place_shape` | `(#mes_formal_root() && #mes_in(#mes_place,#mes_parent))` |
//! | `mes_formal_kept` | `forall (#s: mes_site) { !#mes_formal_written(#s) }` |
//! | `mes_value_shape` | `((#mes_owned_result() \|\| #mes_local_acquisition()) && #mes_pointer_casts())` |
//! | `mes_h1_place_only_at_store` | `forall (#s: mes_site) { (#mes_occurs(#s,#mes_place) => (#s == #mes_store \|\| #mes_null_test(#s))) }` |
//! | `mes_h2_local_kept` | `forall (#s: mes_site) { (!#mes_local_copy(#s) && (#mes_local_mention(#s) => !#mes_after(#s,#mes_store))) }` |
//! | `mes_h3_no_stale_value` | `forall (#o: obj) { (#mes_stored(#o) => (#mes_owed(#o) && #mes_live(#o))) }` |
//! | `mes_not_in_loop` | `!#mes_in_loop(#mes_store)` |
//! | `mes_nothing_released_after` | `forall (#s: mes_site) { (#mes_after(#s,#mes_store) => ((!#mes_formal_passed(#s) && !#mes_indirect(#s)) && (#mes_call(#s) => #mes_borrowed_only(#s)))) }` |
//! | `mes_owner_untracked` | `(#mes_untracked_owner() && !#mes_bound_old())` |
//! | `mes_summary_withheld` | `!#mes_published()` |
//! | `mes_premise_stated` | `#mes_premise_carried()` |
//!
//! The first seven are decided by the parser half
//! (`legacy_c::member_handoff`), lexically over the fused body tokens and
//! from the Clang AST of the store; H3 and the owner are decided on the flow
//! (`legacy_c::pointer_store_flow`); the summary is withheld by rule 10v's
//! gate (`has_pointer_store`); the premise is carried by [`carry_premise`].
//!
//! ## The stated premise (P-CG11-CALLER-SLOT)
//!
//! [`PREMISE_ID`] `mes_caller_slot`:
//! `forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }`:
//! no object the place designates immediately before the store is one whose
//! custody this body owes. Stated, not proved; every row whose CFG holds an
//! admitted hand-off lists it in its assumptions ([`carry_premise`]), and a
//! row with a hand-off that does not list it is refused
//! (`mes_refuse_premise_missing`).
//!
//! ## The discharge
//!
//! On admission the store is read as the consumption of the value's identity
//! (`resource_consumed(v, nullable)`): the body's obligation for `V` is
//! discharged by the hand-off and by nothing else; the place's other bindings
//! are forgotten as for every rule-10v store.
//!
//! ## Named refusals ([`Reason`])
//!
//! Each refusal is the formula [`Reason::formula`] names; the node is then the
//! gate's refusal, byte for byte (or rule 38i's name of it).
//!
//! The Lean model is `QCore.MemberHandoff`; [`DECISION_VECTORS`] and the
//! formula table are shared with it.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The value code a rule-38k store carries
/// (`pointer_store::ValueClass::MemberHandoff`).
pub const HANDOFF: &str = "member-handoff";

/// The rule a row's premise is carried under.
pub const RULE: &str = "38k";

/// The synthetic local a stored call's result is bound to before the
/// hand-off consumes it. Its prefix is reserved: no source spelling has it.
pub const VALUE_NAME: &str = "qcore_member_handoff__value";

/// The callee name the flow's consumption is applied under (never a call of
/// the program; no contract table holds it).
pub const CONSUMER: &str = "qcore_member_handoff__consume";

/// The id of the stated premise.
pub const PREMISE_ID: &str = "mes_caller_slot";

/// P-CG11-CALLER-SLOT as a formula.
pub const PREMISE: &str = "forall (#o: obj) { (#mes_holds(#mes_place,#o) => !#mes_owed(#o)) }";

/// The admission conditions: id and formula. All hold for an admitted store.
pub const CONDITIONS: &[(&str, &str)] = &[
    (
        "mes_place_shape",
        "(#mes_formal_root() && #mes_in(#mes_place,#mes_parent))",
    ),
    (
        "mes_formal_kept",
        "forall (#s: mes_site) { !#mes_formal_written(#s) }",
    ),
    (
        "mes_value_shape",
        "((#mes_owned_result() || #mes_local_acquisition()) && #mes_pointer_casts())",
    ),
    (
        "mes_h1_place_only_at_store",
        "forall (#s: mes_site) { (#mes_occurs(#s,#mes_place) => (#s == #mes_store || #mes_null_test(#s))) }",
    ),
    (
        "mes_h2_local_kept",
        "forall (#s: mes_site) { (!#mes_local_copy(#s) && (#mes_local_mention(#s) => !#mes_after(#s,#mes_store))) }",
    ),
    (
        "mes_h3_no_stale_value",
        "forall (#o: obj) { (#mes_stored(#o) => (#mes_owed(#o) && #mes_live(#o))) }",
    ),
    ("mes_not_in_loop", "!#mes_in_loop(#mes_store)"),
    (
        "mes_nothing_released_after",
        "forall (#s: mes_site) { (#mes_after(#s,#mes_store) => ((!#mes_formal_passed(#s) && !#mes_indirect(#s)) && (#mes_call(#s) => #mes_borrowed_only(#s)))) }",
    ),
    (
        "mes_owner_untracked",
        "(#mes_untracked_owner() && !#mes_bound_old())",
    ),
    ("mes_summary_withheld", "!#mes_published()"),
    ("mes_premise_stated", "#mes_premise_carried()"),
];

/// Why a candidate store is not read as a hand-off. Each reason is a formula
/// ([`Self::formula`]) whose truth refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// The stored call reallocates the place's own value under a nullable
    /// result.
    ReallocSelf,
    /// The value is neither an owned call result nor a local, or a cast is
    /// not pointer-to-pointer.
    ValueShape,
    /// The store's root is a local, even one copied from a formal.
    LocalAliasOfFormal,
    /// The place is past a pointer hop below the formal, or a union or
    /// bit-field member.
    PlaceShape,
    /// The formal is written or address-taken.
    ReboundFormal,
    /// The store lies in a loop, or the body jumps.
    Loop,
    /// The place is used other than at the store and in null tests.
    PlaceUsed,
    /// The local is copied, passed, returned, address-taken, assigned twice,
    /// or mentioned after the store.
    LocalUsed,
    /// After the store the formal is handed on whole, or a call may release
    /// or consume.
    BorrowedUseAfter,
    /// The owner is tracked or unresolved.
    Owner,
    /// The projection binds the place's old value.
    OwnedOldValue,
    /// The stored value is not null or one live identity this body owes.
    StaleValue,
    /// The row does not list the stated premise.
    PremiseMissing,
}

impl Reason {
    pub const ALL: [Self; 13] = [
        Self::ReallocSelf,
        Self::ValueShape,
        Self::LocalAliasOfFormal,
        Self::PlaceShape,
        Self::ReboundFormal,
        Self::Loop,
        Self::PlaceUsed,
        Self::LocalUsed,
        Self::BorrowedUseAfter,
        Self::Owner,
        Self::OwnedOldValue,
        Self::StaleValue,
        Self::PremiseMissing,
    ];

    /// The reason's id.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::ReallocSelf => "mes_refuse_realloc_self",
            Self::ValueShape => "mes_refuse_value_shape",
            Self::LocalAliasOfFormal => "mes_refuse_local_alias_of_formal",
            Self::PlaceShape => "mes_refuse_place_shape",
            Self::ReboundFormal => "mes_refuse_rebound_formal",
            Self::Loop => "mes_refuse_loop",
            Self::PlaceUsed => "mes_refuse_place_used",
            Self::LocalUsed => "mes_refuse_local_used",
            Self::BorrowedUseAfter => "mes_refuse_borrowed_use_after",
            Self::Owner => "mes_refuse_owner",
            Self::OwnedOldValue => "mes_refuse_owned_old_value",
            Self::StaleValue => "mes_refuse_stale_value",
            Self::PremiseMissing => "mes_refuse_premise_missing",
        }
    }

    /// The formula whose truth refuses.
    #[must_use]
    pub fn formula(self) -> &'static str {
        match self {
            Self::ReallocSelf => "(#mes_reallocates() && !#mes_nonnull_clause())",
            Self::ValueShape => {
                "!((#mes_owned_result() || #mes_local_acquisition()) && #mes_pointer_casts())"
            }
            Self::LocalAliasOfFormal => "!#mes_formal_root()",
            Self::PlaceShape => "!#mes_in(#mes_place,#mes_parent)",
            Self::ReboundFormal => "exists (#s: mes_site) { #mes_formal_written(#s) }",
            Self::Loop => "#mes_in_loop(#mes_store)",
            Self::PlaceUsed => {
                "exists (#s: mes_site) { (#mes_occurs(#s,#mes_place) && !(#s == #mes_store || #mes_null_test(#s))) }"
            }
            Self::LocalUsed => {
                "exists (#s: mes_site) { (#mes_local_copy(#s) || (#mes_local_mention(#s) && #mes_after(#s,#mes_store))) }"
            }
            Self::BorrowedUseAfter => {
                "exists (#s: mes_site) { (#mes_after(#s,#mes_store) && ((#mes_formal_passed(#s) || #mes_indirect(#s)) || (#mes_call(#s) && !#mes_borrowed_only(#s)))) }"
            }
            Self::Owner => "!#mes_untracked_owner()",
            Self::OwnedOldValue => "#mes_bound_old()",
            Self::StaleValue => {
                "exists (#o: obj) { (#mes_stored(#o) && !(#mes_owed(#o) && #mes_live(#o))) }"
            }
            Self::PremiseMissing => "!#mes_premise_carried()",
        }
    }

    /// The code the Lean vectors spell.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::ReallocSelf => "reallocSelf",
            Self::ValueShape => "valueShape",
            Self::LocalAliasOfFormal => "localAliasOfFormal",
            Self::PlaceShape => "placeShape",
            Self::ReboundFormal => "reboundFormal",
            Self::Loop => "loop",
            Self::PlaceUsed => "placeUsed",
            Self::LocalUsed => "localUsed",
            Self::BorrowedUseAfter => "borrowedUseAfter",
            Self::Owner => "owner",
            Self::OwnedOldValue => "ownedOldValue",
            Self::StaleValue => "staleValue",
            Self::PremiseMissing => "premiseMissing",
        }
    }

    /// The parity and diagnostic text: the id and its formula.
    #[must_use]
    pub fn statement(self) -> String {
        format!("{}: {}", self.id(), self.formula())
    }
}

/// What the stored value is, as the parser reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    /// A call whose contract in force returns `Owned`.
    OwnedResult,
    /// A local's name.
    LocalAcquisition,
    /// A call whose contract returns `Reallocated`.
    Reallocated,
    /// Anything else.
    Other,
}

/// The facts the decision reads (`QCore.MemberHandoff.Facts`, field for
/// field). `true` is the admitting answer of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Facts {
    pub value: ValueKind,
    pub nonnull_clause: bool,
    pub pointer_casts: bool,
    pub formal_root: bool,
    pub place_shape: bool,
    pub formal_kept: bool,
    pub not_in_loop: bool,
    pub place_only_at_store: bool,
    pub local_kept: bool,
    pub nothing_released_after: bool,
    pub untracked_owner: bool,
    pub old_value_unbound: bool,
    pub value_live_owed: bool,
}

/// The decision: the first failing condition's reason, in the order the
/// conditions are asked (`QCore.MemberHandoff.decide`).
///
/// # Errors
///
/// The [`Reason`] that refuses.
pub fn decide(f: &Facts) -> Result<(), Reason> {
    match f.value {
        ValueKind::Reallocated if !f.nonnull_clause => return Err(Reason::ReallocSelf),
        ValueKind::OwnedResult | ValueKind::LocalAcquisition if f.pointer_casts => {}
        _ => return Err(Reason::ValueShape),
    }
    let checks = [
        (f.formal_root, Reason::LocalAliasOfFormal),
        (f.place_shape, Reason::PlaceShape),
        (f.formal_kept, Reason::ReboundFormal),
        (f.not_in_loop, Reason::Loop),
        (f.place_only_at_store, Reason::PlaceUsed),
        (f.local_kept, Reason::LocalUsed),
        (f.nothing_released_after, Reason::BorrowedUseAfter),
        (f.untracked_owner, Reason::Owner),
        (f.old_value_unbound, Reason::OwnedOldValue),
        (f.value_live_owed, Reason::StaleValue),
    ];
    for (holds, reason) in checks {
        if !holds {
            return Err(reason);
        }
    }
    Ok(())
}

/// Every fact admitting, for `value`.
#[must_use]
pub fn admitting(value: ValueKind) -> Facts {
    Facts {
        value,
        nonnull_clause: false,
        pointer_casts: true,
        formal_root: true,
        place_shape: true,
        formal_kept: true,
        not_in_loop: true,
        place_only_at_store: true,
        local_kept: true,
        nothing_released_after: true,
        untracked_owner: true,
        old_value_unbound: true,
        value_live_owed: true,
    }
}

/// The decision vectors shared with `proof/QCore/MemberHandoff.lean`
/// (`decisionVectors`): a fixture spelling, the one fact that differs from
/// [`admitting`] (`""` for none, `value=…` for the value kind) and the
/// decision (`admit` or a [`Reason::code`]). Each spelling is a body of
/// `tests/member_effect_stores.rs`.
pub const DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("p->buf = q_alloc()", "", "admit"),
    ("p->buf = l", "value=local", "admit"),
    ("p->in.buf = q_alloc()", "", "admit"),
    ("p->any = (void *) l", "value=local", "admit"),
    (
        "p->buf = q_realloc(p->buf, n)",
        "value=reallocated",
        "reallocSelf",
    ),
    ("p->buf = q_borrow_get()", "value=other", "valueShape"),
    ("p->n = (long) l", "pointer_casts", "valueShape"),
    (
        "m = p; m->buf = q_alloc()",
        "formal_root",
        "localAliasOfFormal",
    ),
    ("p->next->buf = q_alloc()", "place_shape", "placeShape"),
    ("p = r; p->buf = q_alloc()", "formal_kept", "reboundFormal"),
    ("while (c) p->buf = q_alloc()", "not_in_loop", "loop"),
    (
        "p->buf = q_alloc(); use(p->buf)",
        "place_only_at_store",
        "placeUsed",
    ),
    ("p->buf = l; q_free(l)", "local_kept", "localUsed"),
    (
        "p->buf = q_alloc(); q_take(p)",
        "nothing_released_after",
        "borrowedUseAfter",
    ),
    ("o->buf = q_alloc() (o tracked)", "untracked_owner", "owner"),
    (
        "p->buf = q_alloc() (old value bound)",
        "old_value_unbound",
        "ownedOldValue",
    ),
    ("q_free(l); p->buf = l", "value_live_owed", "staleValue"),
];

/// The facts a vector's differing-fact spelling stands for.
#[must_use]
pub fn vector_facts(differs: &str) -> Facts {
    let mut facts = admitting(ValueKind::OwnedResult);
    match differs {
        "" => {}
        "value=local" => facts.value = ValueKind::LocalAcquisition,
        "value=reallocated" => facts.value = ValueKind::Reallocated,
        "value=other" => facts.value = ValueKind::Other,
        "pointer_casts" => facts.pointer_casts = false,
        "formal_root" => facts.formal_root = false,
        "place_shape" => facts.place_shape = false,
        "formal_kept" => facts.formal_kept = false,
        "not_in_loop" => facts.not_in_loop = false,
        "place_only_at_store" => facts.place_only_at_store = false,
        "local_kept" => facts.local_kept = false,
        "nothing_released_after" => facts.nothing_released_after = false,
        "untracked_owner" => facts.untracked_owner = false,
        "old_value_unbound" => facts.old_value_unbound = false,
        "value_live_owed" => facts.value_live_owed = false,
        other => panic!("unknown differing fact `{other}`"),
    }
    facts
}

/// Every formula this rule states: the premise, the conditions and the
/// refusals, as `(id, formula)`.
#[must_use]
pub fn formulas() -> Vec<(&'static str, &'static str)> {
    let mut all = vec![(PREMISE_ID, PREMISE)];
    all.extend(CONDITIONS.iter().copied());
    all.extend(
        Reason::ALL
            .iter()
            .map(|reason| (reason.id(), reason.formula())),
    );
    all
}

/// Whether every formula of [`formulas`] is the canonical printing of a
/// C04.18.9 typed-rendering formula over registered terms (the
/// premise-formula validator, `guarded_proposition`). Read once.
///
/// # Errors
///
/// The validator's message for the first formula it refuses.
pub fn validated() -> Result<(), String> {
    static VALIDATED: OnceLock<Result<(), String>> = OnceLock::new();
    VALIDATED
        .get_or_init(|| {
            for (id, formula) in formulas() {
                qcore_frontend::premise_formula::guarded_proposition(id, formula)?;
            }
            Ok(())
        })
        .clone()
}

/// The premise as a row lists it: its id and its formula.
#[must_use]
pub fn premise_statement() -> String {
    format!("{PREMISE_ID}: {PREMISE}")
}

/// The diagnostic a row with a hand-off but without the stated premise
/// carries (`mes_refuse_premise_missing`).
#[must_use]
pub fn premise_missing_description() -> String {
    format!(
        "gateway rule {RULE} admits a member hand-off only with its premise stated ({})",
        Reason::PremiseMissing.statement()
    )
}

/// Whether `cfg` holds a rule-38k hand-off store.
#[must_use]
pub fn has_handoff(cfg: &crate::legacy_c::LegacyCfg) -> bool {
    cfg.nodes.iter().any(|node| {
        matches!(
            &node.instruction,
            crate::legacy_c::LegacyInstruction::PointerStore { store, .. } if store.value == HANDOFF
        )
    })
}

/// Whether `summary` lists the stated premise among its assumptions.
#[must_use]
pub fn premise_carried(summary: &crate::legacy_c::LegacyFunctionSummary) -> bool {
    let statement = premise_statement();
    summary
        .rule_obligations
        .premises
        .iter()
        .any(|carriage| carriage.rule == RULE && carriage.name == statement)
}

/// Every row whose CFG holds a hand-off lists the stated premise, as long as
/// every formula of the rule validates; a row that still does not list it
/// (`mes_refuse_premise_missing`) is refused.
pub fn carry_premise(functions: &mut BTreeMap<String, crate::legacy_c::LegacyFunctionSummary>) {
    let valid = validated().is_ok();
    for summary in functions.values_mut() {
        if !has_handoff(&summary.cfg) {
            continue;
        }
        if valid {
            summary
                .rule_obligations
                .premises
                .insert(crate::arm_obligations::Carriage {
                    rule: RULE.to_owned(),
                    name: premise_statement(),
                });
        }
        enforce_premise(summary);
    }
}

/// `mes_refuse_premise_missing`: a row with a hand-off whose assumptions do
/// not list the premise keeps an `Unsupported` diagnostic, so it is never
/// `Safe`.
pub fn enforce_premise(summary: &mut crate::legacy_c::LegacyFunctionSummary) {
    if !has_handoff(&summary.cfg) || premise_carried(summary) {
        return;
    }
    let line = summary
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.line)
        .min()
        .unwrap_or(0);
    summary.diagnostics.push(crate::legacy_c::LegacyDiagnostic {
        function: summary.function.clone(),
        line,
        kind: crate::legacy_c::LegacyDiagnosticKind::Unsupported {
            description: premise_missing_description(),
        },
    });
    summary.diagnostics.sort();
    summary.diagnostics.dedup();
    summary.status = crate::legacy_c::verdict::classify(&summary.diagnostics);
}

/// The parity row of one consultation that reached a candidate.
pub(crate) fn record(function: &str, line: usize, place: &str, answer: Result<(), Reason>) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "function": function,
        "line": line,
        "position": "member-handoff",
        "rule": RULE,
        "place": place,
        "answer": match answer {
            Ok(()) => "admit".to_owned(),
            Err(reason) => reason.statement(),
        },
    }));
}

// ---------------------------------------------------------------------------
// Lexical readings over the fused body tokens (the parser half's facts).
// ---------------------------------------------------------------------------

const KEYWORDS: [&str; 10] = [
    "if", "while", "for", "switch", "return", "sizeof", "do", "else", "case", "default",
];

/// The `[open, close]` token ranges of every loop body, or `None` when the
/// body jumps (`goto`, `setjmp`, `longjmp`, `__builtin_setjmp`). An unbraced
/// loop body that does not end at its first `;` (a nested statement) extends
/// to the end of the body: an over-approximation.
#[must_use]
pub fn loop_ranges(body: &[&str]) -> Option<Vec<(usize, usize)>> {
    if body.iter().any(|t| {
        matches!(
            *t,
            "goto" | "setjmp" | "longjmp" | "_setjmp" | "sigsetjmp" | "__builtin_setjmp"
        )
    }) {
        return None;
    }
    let mut ranges = Vec::new();
    for (index, token) in body.iter().enumerate() {
        let start = match *token {
            "for" | "while" => {
                if body.get(index + 1) != Some(&"(") {
                    return None;
                }
                let close = matching(body, index + 1, "(", ")")?;
                close + 1
            }
            "do" => index + 1,
            _ => continue,
        };
        let end = match body.get(start).copied() {
            Some("{") => matching(body, start, "{", "}")?,
            Some(";") => start,
            Some("if" | "for" | "while" | "do" | "switch") => body.len(),
            Some(_) => (start..body.len()).find(|&at| body[at] == ";")?,
            None => return None,
        };
        ranges.push((start, end));
    }
    Some(ranges)
}

/// The index of the token closing the `open` at `start`.
#[must_use]
pub fn matching(texts: &[&str], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(start) {
        if *text == open {
            depth += 1;
        } else if *text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// Whether each token lies in a parenthesised `sizeof ( … )` operand
/// (unevaluated, C11 6.5.3.4p2 for a non-VLA operand). `sizeof X` without
/// parentheses marks nothing: its occurrences count as evaluated.
#[must_use]
pub fn unevaluated(body: &[&str]) -> Vec<bool> {
    let mut marks = vec![false; body.len()];
    for (index, token) in body.iter().enumerate() {
        if *token == "sizeof"
            && body.get(index + 1) == Some(&"(")
            && let Some(close) = matching(body, index + 1, "(", ")")
        {
            for mark in &mut marks[index + 1..=close] {
                *mark = true;
            }
        }
    }
    marks
}

/// Whether the tokens starting at `at` spell the null pointer constant
/// (`NULL`, `0`, `( void * ) 0`, `( ( void * ) 0 )`), and its length.
fn null_constant(body: &[&str], at: usize) -> Option<usize> {
    match body.get(at..) {
        Some(["NULL" | "0", ..]) => Some(1),
        Some(["(", "void", "*", ")", "0", ..]) => Some(5),
        Some(["(", "(", "void", "*", ")", "0", ")", ..]) => Some(7),
        _ => None,
    }
}

/// Whether the token run `[start, end)` (a whole value: a name or a member
/// path) is read only as a truth value.
#[must_use]
pub fn null_test(body: &[&str], start: usize, end: usize) -> bool {
    let previous = start.checked_sub(1).map(|at| body[at]);
    let next = body.get(end).copied();
    let closes = |next: Option<&str>| matches!(next, Some(")" | "&&" | "||" | "?" | ";" | ","));
    // `X == NULL`, `X != NULL`.
    if matches!(next, Some("==" | "!=")) && null_constant(body, end + 1).is_some() {
        let after = end + 1 + null_constant(body, end + 1).unwrap_or(0);
        return closes(body.get(after).copied());
    }
    // `NULL == X`, `NULL != X`.
    if matches!(previous, Some("==" | "!="))
        && start >= 2
        && (0..start - 1)
            .rev()
            .any(|from| null_constant(body, from).is_some_and(|length| from + length == start - 1))
    {
        return closes(next);
    }
    // `!X`.
    if previous == Some("!") {
        return closes(next);
    }
    // `if (X)`, `while (X)`, and an operand of `&&` / `||`.
    let opens = matches!(previous, Some("&&" | "||"))
        || (previous == Some("(")
            && start >= 2
            && matches!(body[start - 2], "if" | "while" | "&&" | "||" | "!"));
    opens && closes(next)
}

/// The member path `F[.G…]` starting with `->` at `arrow` (the token after
/// the base), and the index after it.
#[must_use]
pub fn member_path(body: &[&str], arrow: usize) -> Option<(Vec<String>, usize)> {
    if body.get(arrow) != Some(&"->") {
        return None;
    }
    let first = body.get(arrow + 1)?;
    if !crate::typed_field_access::is_name(first) {
        return None;
    }
    let mut path = vec![(*first).to_owned()];
    let mut at = arrow + 2;
    while body.get(at) == Some(&".")
        && let Some(member) = body.get(at + 1)
        && crate::typed_field_access::is_name(member)
    {
        path.push((*member).to_owned());
        at += 2;
    }
    Some((path, at))
}

/// H1 and the formal's later mentions, over the fused body `body` with the
/// store's left-hand base at `store_base` and its statement ending at
/// `store_end` (the `;`). `base` is the formal, `place` its member path.
///
/// # Errors
///
/// [`Reason::PlaceUsed`] or [`Reason::BorrowedUseAfter`].
pub fn place_and_formal(
    body: &[&str],
    base: &str,
    place: &[String],
    store_base: usize,
    store_end: usize,
) -> Result<(), Reason> {
    let quiet = unevaluated(body);
    for (index, token) in body.iter().enumerate() {
        if *token != base
            || index == store_base
            || quiet[index]
            || matches!(index.checked_sub(1).map(|at| body[at]), Some("->" | "."))
        {
            continue;
        }
        let after = index > store_end;
        match member_path(body, index + 1) {
            Some((path, end)) => {
                let shared = path.iter().zip(place).take_while(|(a, b)| a == b).count();
                if shared == 0 {
                    // A sibling member of the formal's object: disjoint storage.
                    continue;
                }
                if shared < path.len().min(place.len()) {
                    // Diverges below a shared prefix: a sibling sub-object.
                    continue;
                }
                if path.len() == place.len()
                    && !matches!(
                        body.get(end).copied(),
                        Some("->" | "[" | "." | "(" | "=" | "++" | "--")
                    )
                    && body.get(index.wrapping_sub(1)).copied() != Some("&")
                    && null_test(body, index, end)
                {
                    continue;
                }
                // The place itself used, a prefix of it used whole, or the
                // place read through.
                return Err(Reason::PlaceUsed);
            }
            None => {
                if !after || null_test(body, index, index + 1) {
                    continue;
                }
                return Err(Reason::BorrowedUseAfter);
            }
        }
    }
    Ok(())
}

/// `#mes_indirect` at a site after `store_end`: a `->` whose base is not the
/// formal `base` itself, a subscript, or a unary `*`, outside a `sizeof`
/// operand. Any of them could reach the place through an alias of the
/// formal's object.
#[must_use]
pub fn indirect_after(body: &[&str], base: &str, store_end: usize) -> bool {
    let quiet = unevaluated(body);
    (store_end + 1..body.len()).any(|index| {
        if quiet[index] {
            return false;
        }
        let previous = index.checked_sub(1).map(|at| body[at]);
        match body[index] {
            "->" => {
                previous != Some(base)
                    || matches!(index.checked_sub(2).map(|at| body[at]), Some("->" | "."))
            }
            "[" => true,
            "*" => matches!(
                previous,
                Some(
                    "(" | "="
                        | ";"
                        | "{"
                        | "}"
                        | ","
                        | "return"
                        | "!"
                        | "&&"
                        | "||"
                        | "?"
                        | ":"
                        | "=="
                        | "!="
                )
            ),
            _ => false,
        }
    })
}

/// The calls lexically after `store_end`: each callee name.
#[must_use]
pub fn calls_after(body: &[&str], store_end: usize) -> Vec<String> {
    let mut calls = Vec::new();
    for index in store_end + 1..body.len() {
        let name = body[index];
        if body.get(index + 1) == Some(&"(")
            && crate::typed_field_access::is_name(name)
            && !KEYWORDS.contains(&name)
            && !matches!(index.checked_sub(1).map(|at| body[at]), Some("->" | "."))
            // `( T ) ( x )`: a cast's type name followed by a parenthesis.
            && !(index >= 1
                && body[index - 1] == "("
                && body.get(index + 1) == Some(&")"))
        {
            calls.push(name.to_owned());
        }
    }
    calls
}

/// H2 over the fused body: the local `name` is assigned exactly once, before
/// the store, mentioned before it only as that assignment, its declaration, a
/// null test, a member or element access, a dereference or a `sizeof`
/// operand, and never mentioned after `store_end`. `store_value` is the
/// index of the value's own mention in the store.
///
/// # Errors
///
/// [`Reason::LocalUsed`].
pub fn local_kept(
    body: &[&str],
    name: &str,
    store_value: usize,
    store_end: usize,
) -> Result<(), Reason> {
    let quiet = unevaluated(body);
    let mut assignments = 0usize;
    for (index, token) in body.iter().enumerate() {
        if *token != name
            || index == store_value
            || matches!(index.checked_sub(1).map(|at| body[at]), Some("->" | "."))
        {
            continue;
        }
        if quiet[index] {
            continue;
        }
        if index > store_end {
            return Err(Reason::LocalUsed);
        }
        let previous = index.checked_sub(1).map(|at| body[at]);
        let next = body.get(index + 1).copied();
        match next {
            Some("=") => {
                assignments += 1;
                continue;
            }
            Some("->" | "[") => continue,
            _ => {}
        }
        if previous == Some("*") && matches!(next, Some(";" | "," | ")")) {
            // A declarator `T *L;` (a pointer object is never multiplied).
            continue;
        }
        if null_test(body, index, index + 1) {
            continue;
        }
        return Err(Reason::LocalUsed);
    }
    if assignments == 1 {
        Ok(())
    } else {
        Err(Reason::LocalUsed)
    }
}

#[cfg(test)]
#[path = "../tests/unit/member_handoff.rs"]
mod tests;
