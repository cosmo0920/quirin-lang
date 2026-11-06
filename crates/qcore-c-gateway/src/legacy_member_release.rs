//! Card **W45-CALLEE-SUMMARY-CASCADE**: the member-release shapes of corpus
//! destructor bodies, and the call-site guard that keeps a summary of such a
//! body from hiding a release from its caller.
//!
//! The card's cascade census ranks the blocker families in the bodies of the
//! uncontracted corpus callees by how many Unknown functions their removal
//! would unblock. Outside the sub-object stores and return expressions that
//! other wave-45 cards own, the largest family is the destructor body:
//!
//! ```c
//! void rd_kafka_acl_result_destroy(rd_kafka_acl_result_t *acl_res) {
//!         if (acl_res->error)
//!                 rd_kafka_error_destroy(acl_res->error);
//!         rd_free(acl_res);
//! }
//! ```
//!
//! Two statements refuse there, and the refusal of the first poisons the
//! formal, so the last line reads `unresolved resource identity`. This module
//! admits exactly those two shapes and adds the guard their summaries need.
//!
//! ## 1. A pointer member handed to a nullable release
//!
//! Card W44-INDIRECT-CALL-ARGUMENT-ACCESS admits a typed pointer read
//! (`B->F`, `B[I]`, `*B`) as a whole actual only at a `Borrowed` position or
//! in a declared data tail. [`admits_release`] adds one custody position:
//! `Released { nullable: true }`, for a pointer-valued read with at least one
//! dereferenced base (so never `S.F` over a local record). The value read out
//! of memory is an object the projection does not track, so the release is
//! **outside the resource projection** and changes no abstract map: the
//! admitted call is lowered with that position read as `Borrowed`
//! ([`rewrite_released_positions`]). What the read keeps is the existing
//! obligation block, unchanged: the typed read of every base (unbound, a
//! non-null untracked binding, or exactly one known live identity; a
//! released base is a `use_after_release` candidate), then `ArgumentReads`,
//! which refuses a base whose target owns sub-custody (a field link, a
//! contained child or an indexed collection) and a place that is itself bound.
//! Those two refusals are exactly the ways the read value could be a tracked
//! identity, so no tracked identity is ever released by this route.
//!
//! **A member release is custody, so it is admitted only where no second use
//! of the released place can follow it** ([`AdmissibleReleases`]). The first
//! integration review of this card measured that reading the untracked member
//! as "outside the property" certified `free(n->buf); free(n->buf); free(n);`
//! `Safe`, and a caller releasing the member again after a callee that
//! released it. A place `B->F` is admissible in a body only when, lexically:
//!
//! * **P1, the base is a formal that is never copied.** `B` is a formal
//!   parameter, and every occurrence of the name `B` in the body is a member
//!   access `B->G`, a null test of `B` in an `if` condition, or the whole
//!   single actual of a call whose contract releases its only parameter. No
//!   assignment, declaration, address, cast or other actual of `B` exists, so
//!   no alias of the base does.
//! * **P2, one release and nothing after it.** Exactly one occurrence of
//!   `B->F` is a whole actual at a `Released { nullable: true }` position (the
//!   release). Every other occurrence is a null test in an `if` condition that
//!   precedes the release in token order. So the place is never copied into a
//!   local or another member, never passed elsewhere, never written, and never
//!   read after the release.
//! * **P3, executed at most once.** The body has no `goto` (nor `setjmp` or
//!   `longjmp`), and the release is inside no loop (`do … while (0)` is not
//!   one, constraint 5c), so along every path it runs at most once and every
//!   later statement comes later in token order.
//! * **P4, the base is released afterwards on every path.** A statement
//!   `R ( B ) ;` of the body's own block (not under any `if`, loop or label)
//!   follows the release, `R`'s contract releases its only parameter, and no
//!   `return` lies between them. So every path through the member release
//!   releases `B`, every later access through `B` is a use of a released
//!   identity, and a validated summary of the body states `Released` for `B`:
//!   a body that releases through a formal without releasing the formal gets
//!   no rule 1 and no summary.
//!
//! Further premises, stated rather than hidden:
//!
//! * **Only `nullable`.** A non-null release contract demands a non-null
//!   actual, and the projection establishes nothing about the nullness of an
//!   untracked member, so `Released { nullable: false }` keeps the refusal.
//! * **Never an address.** `&B->F` is an interior pointer of `*B`; handing it
//!   to a release is the invalid release the derived-result rule accuses. Only
//!   the exact `B -> F` shape is admitted, so a caller never releases an
//!   interior pointer through this route.
//! * **Distinct members own distinct objects.** Two members of one record
//!   holding the same pointer are not detected. This is the destructor's own
//!   ownership premise, stated here; P2 refuses every copy the body itself
//!   makes.
//! * **A stored pointer is tracked only through a bound place or a
//!   sub-custody relation.** This is W44-INDIRECT-CALL-ARGUMENT-ACCESS's
//!   premise 4, unchanged; a card that stores a tracked identity into a member
//!   without recording either must refuse this read (see the report).
//! * **A declared contract's `Borrowed` covers the objects reachable through
//!   the argument** (P-B08-SHALLOW, unchanged): a declaration that releases a
//!   member while stating `Borrowed` is the declaration's premise.
//!
//! ## 2. A pointer member null test in a controlling expression
//!
//! [`member_null_test`] reads `B->F`, `!B->F`, `B->F == N`, `B->F != N`,
//! `N == B->F` and `N != B->F` (after enclosing parentheses, `N` a null
//! pointer constant) with `B` declared `struct T *` and `F` a pointer member
//! of `T`'s single complete definition. It lowers to the same typed read
//! (`IndirectScalarAccess` over `B`, no destination, no frame) that a scalar
//! member test lowers to: both successors carry the state unchanged, so the
//! test refines nothing, and `B` keeps its dereference obligation at the
//! header. Reading a member's nullness releases, binds and transfers nothing.
//!
//! The pre-existing aggregate null test of card O04-B08 lowered `b->item == 0`
//! to a bare `BranchOnNull` and carried no obligation on `b`, so a test through
//! a released `b` was `Safe` (`AGGREGATE-NULL-TEST-AFTER-RELEASE`).
//! [`oblige_member_null_tests`] puts the same typed read before every
//! `BranchOnNull` whose place dereferences a pointer, over every pointer the
//! place reads through (`b` for `b->item`; `a` and `a->b` for `a->b->c`).
//!
//! ## 3. The call-site guard (why a summary may carry rule 1)
//!
//! A W4 summary states what a body does to its formals. Rule 1 lets a
//! validated body also release an object reachable **through** a formal,
//! which no summary mode states. A caller whose actual owns sub-custody could
//! therefore hold that very object as a tracked child and release it again
//! after the call. [`guard_unit_callees`] closes that channel on the caller's
//! side. Before each call to a function this translation unit defines, that
//! the environment's trusted texts do not declare, and whose body **may
//! release through a formal** ([`member_releasing_functions`]: a body with a
//! call whose whole actual is a pointer read, or that calls such a body, to a
//! fixed point), it inserts `ArgumentReads` over the call's plain-name
//! actuals, which records `UnknownAlias` for any actual whose target owns
//! sub-custody. The selection is lexical and ignores contracts, so it is a
//! superset of the bodies that used rule 1, and the guard only ever refuses.
//! A callee with no body here has no derived summary (W4 summaries never cross
//! a unit), and a declared callee is governed by its declaration, never by an
//! inference (constraint 8); both keep the declaration's stated premise
//! (P-B08-SHALLOW) and carry no guard. Two coarser selections were measured
//! and rejected: every unit-defined callee guarded declared header bodies
//! (`flb_input_chunk_create` left `Safe` over the declared `static inline`
//! `flb_free`), and every undeclared one refused the callers of validated
//! `Removed` wrappers, whose owner actual owns sub-custody by construction.
//! The inserted node is appended and every edge into the call is redirected
//! to it, so no existing node's identity moves.
//!
//! ## 4. A callee that releases the member again (card W45-SR5-SOUNDNESS)
//!
//! P1 accepts `R(B)` for any `R` whose contract in force releases its only
//! parameter, and derived summaries are in force. STABILITY-REEVAL-5 measured
//! `q_nullable_free(n->buf); node_destroy(n);` `Safe` at function and unit
//! level with `node_destroy` releasing `n->buf` itself: a double release
//! (`MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE`). The review repair of section 1
//! made the release the last use of its place *within the body*; the second
//! release lived in the callee.
//!
//! Every `R(B)` of the body, before or after the release, is now read for
//! what `R` does to `B->F`. A callee this unit defines has a member effect
//! ([`MemberEffect`]): its only formal `p` is used only as `p->G`, in null
//! tests and as the whole single actual of `R2(p)` (whose effect is read the
//! same way, to a fixed point; a cycle is unknown), and it then touches and
//! releases known members; any other use leaves it unknown. The place is
//! admitted only when no such `R` touches `F`. When one releases `F` and
//! follows the release unconditionally (P4's shape), the release statement
//! refuses under [`double_release_description`] and the function reports a
//! `DoubleRelease` at that line, a provisional candidate
//! ([`callee_double_releases`]); when one only touches `F`, or is unknown,
//! the place is withheld and the statement keeps its refusal (`Unknown`). A
//! callee with no body here is governed by its declaration: a declared
//! release does not cascade to members (P-B08-SHALLOW, stated, not proved).
//! A callee the unit defines under a declaration is read too; the reading
//! only withholds. `QCore.MemberReleaseCallee`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceParameterMode};

use super::call_argument_access::{AdmittedArgument, ArgumentRead};
use super::{
    LegacyCfg, LegacyCfgNode, LegacyInstruction, Token, call_argument_spans, is_null_constant,
    matching, strip_parentheses,
};
use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// Rule 1: whether a recognised actual at `index` of a call under `contract`
/// is admitted as a pointer member handed to a nullable release.
pub(super) fn admits_release(
    contract: Option<&ResourceFunctionContract>,
    index: usize,
    read: &ArgumentRead,
) -> bool {
    read.pointer_valued
        && !read.bases.is_empty()
        && matches!(
            contract.and_then(|contract| contract.parameters.get(index)),
            Some(ResourceParameterMode::Released { nullable: true })
        )
}

/// Whether every used actual admitted by rule 1 sits at a `Released
/// { nullable: true }` position of the contracted call the probe classified.
/// Anything else (an element store around the call, a contract the
/// classification rewrote) declines the whole admission, which keeps the
/// original refusal.
pub(super) fn released_positions_rewritable(
    result: &LegacyInstruction,
    used: &[AdmittedArgument],
) -> bool {
    let releases = used
        .iter()
        .filter(|entry| entry.release)
        .collect::<Vec<_>>();
    if releases.is_empty() {
        return true;
    }
    let LegacyInstruction::ContractCall {
        function, contract, ..
    } = result
    else {
        return false;
    };
    releases.iter().all(|entry| {
        entry.callee == *function
            && matches!(
                contract.parameters.get(entry.index),
                Some(ResourceParameterMode::Released { nullable: true })
            )
    })
}

/// Rule 1's lowering: each position a used rule-1 actual occupies is read as
/// `Borrowed`, because the object released there is outside the projection.
pub(super) fn rewrite_released_positions(
    result: &mut LegacyInstruction,
    used: &[AdmittedArgument],
) {
    let LegacyInstruction::ContractCall { contract, .. } = result else {
        return;
    };
    for entry in used.iter().filter(|entry| entry.release) {
        if let Some(mode) = contract.parameters.get_mut(entry.index) {
            *mode = ResourceParameterMode::Borrowed;
        }
    }
}

/// Rule 2: the bases of a pointer member null test, or `None`.
pub(super) fn member_null_test(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<String> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        return member_null_test(rest, model, types);
    }
    if let Some(split) = comparison(texts) {
        let (left, right) = (&texts[..split], &texts[split + 2..]);
        if is_null_constant(right) {
            return pointer_member_base(left, model, types);
        }
        if is_null_constant(left) {
            return pointer_member_base(right, model, types);
        }
        return None;
    }
    pointer_member_base(texts, model, types)
}

/// The index of the only top-level `==` or `!=` in `texts`.
fn comparison(texts: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    let mut found = None;
    for index in 0..texts.len() {
        match texts[index] {
            "(" => depth += 1,
            ")" => depth = depth.checked_sub(1)?,
            "=" | "!"
                if depth == 0
                    && texts.get(index + 1) == Some(&"=")
                    && (index == 0 || !matches!(texts[index - 1], "=" | "!" | "<" | ">")) =>
            {
                if found.is_some() {
                    return None;
                }
                found = Some(index);
            }
            _ => {}
        }
    }
    found
}

/// `B -> F` with `B` declared `struct T *` and `F` a pointer member of `T`.
fn pointer_member_base(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<String> {
    let texts = strip_parentheses(texts);
    let [base, "-", ">", field] = texts else {
        return None;
    };
    if !is_name(base) || !is_name(field) {
        return None;
    }
    let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
        return None;
    };
    (model.borrowable_field(tag, field) && !model.scalar_field(tag, field))
        .then(|| (*base).to_owned())
}

/// Rule 3: before every call to a function this unit defines whose body may
/// release through a formal ([`member_releasing_functions`]) and that the
/// environment does not declare, the sub-custody guard over the call's
/// plain-name actuals.
pub(super) fn guard_unit_callees(
    cfg: &mut LegacyCfg,
    tokens: &[Token],
    contracts: &ResourceContracts,
) {
    // Both read lazily: most CFGs call no such function.
    let mut releasing = None::<BTreeSet<String>>;
    let mut declared = None::<BTreeSet<String>>;
    let mut guarded = BTreeMap::<String, bool>::new();
    let mut is_guarded = |name: &str| {
        *guarded.entry(name.to_owned()).or_insert_with(|| {
            releasing
                .get_or_insert_with(|| member_releasing_functions(tokens))
                .contains(name)
                && !declared
                    .get_or_insert_with(|| super::descriptor_custody::declared_callees(contracts))
                    .contains(name)
        })
    };
    let count = cfg.nodes.len();
    for id in 0..count {
        let mut actuals = Vec::<String>::new();
        for (function, arguments) in guarded_calls(&cfg.nodes[id].instruction) {
            if !is_guarded(function) {
                continue;
            }
            for argument in arguments.iter().flatten() {
                if is_name(argument) && !actuals.contains(argument) {
                    actuals.push(argument.clone());
                }
            }
        }
        if actuals.is_empty() {
            continue;
        }
        let line = cfg.nodes[id].line;
        insert_before(
            cfg,
            id,
            LegacyInstruction::ArgumentReads {
                names: Vec::new(),
                pointer_bases: actuals,
                pointer_places: Vec::new(),
            },
            line,
        );
    }
}

/// Rule 1's lexical premises P1–P4 (module documentation), read once per
/// body: the `(base, member)` places whose single release may be admitted.
///
/// Card W45-SR5-SOUNDNESS adds `double_releases`: the places whose release a
/// later `R(B)` repeats because `R`'s body, defined in this unit, releases the
/// same member of its formal (section 4 of the module documentation). Such a
/// place is never admitted, and its release statement refuses under
/// [`double_release_description`].
#[derive(Clone, Debug, Default)]
pub(super) struct AdmissibleReleases {
    places: Places,
    double_releases: DoubleReleases,
}

impl AdmissibleReleases {
    /// Whether the whole actual `argument` is `B -> F` for an admissible place.
    pub(super) fn admits(&self, argument: &[&str]) -> bool {
        match strip_parentheses(argument) {
            [base, "-", ">", field] => self
                .places
                .contains(&((*base).to_owned(), (*field).to_owned())),
            _ => false,
        }
    }

    /// Card W45-SR5-SOUNDNESS: the refusal of a release statement whose
    /// actual is a place a later callee releases again, if `argument` is one.
    pub(super) fn double_release(&self, argument: &[&str]) -> Option<String> {
        match strip_parentheses(argument) {
            [base, "-", ">", field] => self
                .double_releases
                .get(&((*base).to_owned(), (*field).to_owned()))
                .map(|callee| double_release_description(base, field, callee)),
            _ => None,
        }
    }

    /// Card MEMBER-EFFECT-SUMMARY (rule 38j): these places together with the
    /// release places the member-effect body check admitted.
    pub(super) fn with_places(mut self, places: &BTreeSet<(String, String)>) -> Self {
        self.places.extend(places.iter().cloned());
        self
    }

    pub(super) fn read(
        tokens: &[Token],
        body_start: usize,
        formals: &BTreeSet<String>,
        contracts: &ResourceContracts,
    ) -> Self {
        let Some(body_end) = matching(tokens, body_start, "{", "}") else {
            return Self::default();
        };
        let texts = tokens[body_start + 1..body_end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (places, double_releases) = admissible_places(tokens, &texts, formals, contracts);
        Self {
            places,
            double_releases,
        }
    }
}

/// Card W45-SR5-SOUNDNESS: the refusal a member release carries when a later
/// `R(B)` of the same body releases the member again through `R`'s formal.
#[must_use]
pub(super) fn double_release_description(base: &str, field: &str, callee: &str) -> String {
    format!(
        "release of `{base}->{field}` before `{callee}({base})`, whose body releases the same \
         member again ({DOUBLE_RELEASE_KEY})"
    )
}

/// The stable key ending [`double_release_description`].
const DOUBLE_RELEASE_KEY: &str = "member-released-again-by-callee";

/// Card W45-SR5-SOUNDNESS: the double release each such refusal stands for,
/// one diagnostic per refused node of `cfg`, at the node's line. The member
/// has no acquisition in this body; the line of its first release stands in
/// for one. A provisional `Unsafe` candidate: the second release happens when
/// the member is non-null at the call, which nothing here excludes.
pub(super) fn callee_double_releases(cfg: &LegacyCfg) -> Vec<super::LegacyDiagnostic> {
    cfg.nodes
        .iter()
        .filter(|node| {
            matches!(&node.instruction, LegacyInstruction::Unknown { description, .. }
                if description.starts_with("release of `")
                    && description.ends_with(&format!("({DOUBLE_RELEASE_KEY})")))
        })
        .map(|node| super::LegacyDiagnostic {
            function: cfg.function.clone(),
            line: node.line,
            kind: super::LegacyDiagnosticKind::DoubleRelease {
                acquisition_line: node.line,
            },
        })
        .collect()
}

pub(super) const NON_LOCAL_JUMPS: &[&str] = &[
    "goto",
    "setjmp",
    "longjmp",
    "_setjmp",
    "sigsetjmp",
    "siglongjmp",
];

/// Whether `contract` releases its only parameter.
pub(super) fn releases_its_only_parameter(contract: Option<&ResourceFunctionContract>) -> bool {
    contract.is_some_and(|contract| {
        contract.parameters.len() == 1
            && matches!(
                contract.parameters[0],
                ResourceParameterMode::Released { .. }
            )
    })
}

/// Every call argument of `texts` that is exactly `B -> F` (parenthesised or
/// not): the index of `B` mapped to (callee, argument index).
pub(super) fn member_actuals<'a>(texts: &[&'a str]) -> BTreeMap<usize, (&'a str, usize)> {
    let mut actuals = BTreeMap::new();
    for at in 0..texts.len() {
        if !is_name(texts[at]) || texts.get(at + 1) != Some(&"(") || NOT_CALLS.contains(&texts[at])
        {
            continue;
        }
        let Some(spans) = call_argument_spans(texts, at) else {
            continue;
        };
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let argument = &texts[start..end];
            let stripped = strip_parentheses(argument);
            if let [_, "-", ">", _] = stripped {
                let offset = argument.len() - stripped.len();
                actuals.insert(start + offset / 2, (texts[at], index));
            }
        }
    }
    actuals
}

/// Whether the actual at `index` of `callee` sits at a release position of
/// the contract in force (declared, or a derived summary).
fn releases_position(contracts: &ResourceContracts, callee: &str, index: usize) -> bool {
    matches!(
        contracts
            .contract(callee)
            .and_then(|contract| contract.parameters.get(index)),
        Some(ResourceParameterMode::Released { .. })
    )
}

/// Card W45-SR5-SOUNDNESS: what a callee this unit defines may do to the
/// members of its only formal (module documentation, section 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum MemberEffect {
    /// Every use of the formal `p` is a member access `p->G`, a null test of
    /// `p`, or the whole single actual of `R(p)` where `R` has no body here
    /// (its declaration governs, P-B08-SHALLOW) or a characterised one.
    /// `touched`: every member accessed other than by a null test, through
    /// the whole chain; `released`: every member handed whole to a release
    /// position, through the whole chain.
    Characterised {
        touched: BTreeSet<String>,
        released: BTreeSet<String>,
    },
    /// Any other use: a copy, an address, another actual, an uncontracted
    /// callee, a recursive chain.
    Unknown,
}

/// The member effect of `callee`'s body, or `None` when this unit defines no
/// body for it. `visiting` breaks cycles, which are [`MemberEffect::Unknown`].
pub(super) fn member_effect(
    tokens: &[Token],
    callee: &str,
    contracts: &ResourceContracts,
    visiting: &mut BTreeSet<String>,
) -> Option<MemberEffect> {
    let (body_start, _) = super::find_function_body(tokens, callee)?;
    if !visiting.insert(callee.to_owned()) {
        return Some(MemberEffect::Unknown);
    }
    let effect = characterise(tokens, callee, body_start, contracts, visiting);
    visiting.remove(callee);
    Some(effect)
}

fn characterise(
    tokens: &[Token],
    callee: &str,
    body_start: usize,
    contracts: &ResourceContracts,
    visiting: &mut BTreeSet<String>,
) -> MemberEffect {
    let formals = super::function_parameters(tokens, callee).unwrap_or_default();
    let [Some(formal)] = formals.as_slice() else {
        return MemberEffect::Unknown;
    };
    let Some(body_end) = matching(tokens, body_start, "{", "}") else {
        return MemberEffect::Unknown;
    };
    let texts = tokens[body_start + 1..body_end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let actuals = member_actuals(&texts);
    let mut touched = BTreeSet::new();
    let mut released = BTreeSet::new();
    for at in 0..texts.len() {
        if texts[at] != formal {
            continue;
        }
        let previous = at.checked_sub(1).map(|p| texts[p]);
        if previous == Some(".") || (previous == Some(">") && at >= 2 && texts[at - 2] == "-") {
            continue; // a member name that spells the formal's name
        }
        if let (Some("-"), Some(">"), Some(field)) = (
            texts.get(at + 1).copied(),
            texts.get(at + 2).copied(),
            texts.get(at + 3).copied(),
        ) && is_name(field)
        {
            if is_null_test(&texts, at, 4) {
                continue;
            }
            touched.insert(field.to_owned());
            if actuals
                .get(&at)
                .is_some_and(|(inner, index)| releases_position(contracts, inner, *index))
            {
                released.insert(field.to_owned());
            }
            continue;
        }
        if is_null_test(&texts, at, 1) {
            continue;
        }
        if previous == Some("(")
            && texts.get(at + 1) == Some(&")")
            && at >= 2
            && is_name(texts[at - 2])
            && !NOT_CALLS.contains(&texts[at - 2])
        {
            let inner = texts[at - 2];
            match member_effect(tokens, inner, contracts, visiting) {
                // No body here: the declaration governs what it does
                // through its argument (P-B08-SHALLOW); an undeclared one
                // is not characterised.
                None if contracts.contract(inner).is_some() => continue,
                Some(MemberEffect::Characterised {
                    touched: inner_touched,
                    released: inner_released,
                }) => {
                    touched.extend(inner_touched);
                    released.extend(inner_released);
                    continue;
                }
                None | Some(MemberEffect::Unknown) => return MemberEffect::Unknown,
            }
        }
        return MemberEffect::Unknown;
    }
    MemberEffect::Characterised { touched, released }
}

/// The admissible `(base, member)` places of one body.
type Places = BTreeSet<(String, String)>;

/// Card W45-SR5-SOUNDNESS: each place a later callee releases again, with
/// that callee.
type DoubleReleases = BTreeMap<(String, String), String>;

/// P1–P4 over the body's tokens (without its outer braces), with card
/// W45-SR5-SOUNDNESS's condition on every `R(B)`; `tokens` is the whole unit,
/// where the callees' bodies are read.
fn admissible_places(
    tokens: &[Token],
    texts: &[&str],
    formals: &BTreeSet<String>,
    contracts: &ResourceContracts,
) -> (Places, DoubleReleases) {
    let mut admitted = BTreeSet::new();
    let mut double_releases = BTreeMap::new();
    if texts.iter().any(|text| NON_LOCAL_JUMPS.contains(text)) {
        return (admitted, double_releases);
    }
    let depth = brace_depths(texts);
    let loops = loop_spans(texts, &depth);
    let actuals = member_actuals(texts);
    'formal: for base in formals {
        let mut members = BTreeMap::<&str, Vec<usize>>::new();
        let mut base_releases = Vec::new();
        for at in 0..texts.len() {
            if texts[at] != base {
                continue;
            }
            let previous = at.checked_sub(1).map(|p| texts[p]);
            if previous == Some(".") || (previous == Some(">") && at >= 2 && texts[at - 2] == "-") {
                continue; // a member name that spells the formal's name
            }
            if let (Some("-"), Some(">"), Some(field)) = (
                texts.get(at + 1).copied(),
                texts.get(at + 2).copied(),
                texts.get(at + 3).copied(),
            ) && is_name(field)
            {
                members.entry(field).or_default().push(at);
                continue;
            }
            // A null test of the base itself reads its value and copies
            // nothing (`if (!buckets) return;`).
            if is_null_test(texts, at, 1) {
                continue;
            }
            // `R ( B )` with `R` releasing its only parameter.
            if previous == Some("(")
                && texts.get(at + 1) == Some(&")")
                && at >= 2
                && is_name(texts[at - 2])
                && releases_its_only_parameter(contracts.contract(texts[at - 2]))
            {
                base_releases.push(at - 2);
                continue;
            }
            continue 'formal; // P1: any other use of the base
        }
        for (field, occurrences) in members {
            let releases = occurrences
                .iter()
                .copied()
                .filter(|at| {
                    actuals.get(at).is_some_and(|(callee, index)| {
                        matches!(
                            contracts
                                .contract(callee)
                                .and_then(|contract| contract.parameters.get(*index)),
                            Some(ResourceParameterMode::Released { nullable: true })
                        )
                    })
                })
                .collect::<Vec<_>>();
            let [release] = releases[..] else {
                continue; // P2: none, or more than one
            };
            if !occurrences
                .iter()
                .all(|at| *at == release || (*at < release && is_null_test(texts, *at, 4)))
            {
                continue; // P2: another use of the place
            }
            if loops
                .iter()
                .any(|(start, end)| *start < release && release < *end)
            {
                continue; // P3
            }
            let unconditionally_after = |call: usize| {
                call > release
                    && depth[call] == 0
                    && (call == 0 || matches!(texts[call - 1], ";" | "{" | "}"))
                    && texts.get(call + 4) == Some(&";")
                    && !texts[release..call].contains(&"return")
            };
            let released_after = base_releases
                .iter()
                .any(|call| unconditionally_after(*call));
            // Card W45-SR5-SOUNDNESS: every `R(B)` of the body, before or
            // after the release, is read for what `R` does to `B->F`. A
            // callee this unit defines releases it again (a double release
            // when it follows on every path), touches it or is not
            // characterised: the place is withheld. A callee with no body
            // here is governed by its declaration (P-B08-SHALLOW).
            let mut withheld = false;
            for call in &base_releases {
                let callee = texts[*call];
                match member_effect(tokens, callee, contracts, &mut BTreeSet::new()) {
                    None => {}
                    Some(MemberEffect::Characterised { touched, released }) => {
                        if released.contains(field) && unconditionally_after(*call) {
                            double_releases
                                .entry((base.clone(), field.to_owned()))
                                .or_insert_with(|| callee.to_owned());
                        }
                        withheld |= touched.contains(field);
                    }
                    Some(MemberEffect::Unknown) => withheld = true,
                }
            }
            if released_after && !withheld {
                admitted.insert((base.clone(), field.to_owned()));
            }
        }
    }
    (admitted, double_releases)
}

/// The brace depth of every token, 0 in the body's own block.
pub(super) fn brace_depths(texts: &[&str]) -> Vec<usize> {
    let mut depth = 0usize;
    texts
        .iter()
        .map(|text| {
            let here = if *text == "}" {
                depth.saturating_sub(1)
            } else {
                depth
            };
            match *text {
                "{" => depth += 1,
                "}" => depth = depth.saturating_sub(1),
                _ => {}
            }
            here
        })
        .collect()
}

/// The index of the token closing the bracket opened at `open`.
fn close_of(texts: &[&str], open: usize, left: &str, right: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        if *text == left {
            depth += 1;
        } else if *text == right {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// The end of the block enclosing `at` (an over-approximation of a braceless
/// statement's extent).
fn enclosing_block_end(texts: &[&str], depth: &[usize], at: usize) -> usize {
    (at + 1..texts.len())
        .find(|index| texts[*index] == "}" && depth[*index] < depth[at])
        .unwrap_or(texts.len())
}

/// Every loop's token span, over-approximated where the body is braceless.
/// `do { … } while ( 0 )` is not a loop (constraint 5c).
pub(super) fn loop_spans(texts: &[&str], depth: &[usize]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for at in 0..texts.len() {
        match texts[at] {
            "for" | "while" => {
                // The `while` of a `do … while` is the do's.
                if texts[at] == "while"
                    && at > 0
                    && texts[at - 1] == "}"
                    && (0..at - 1)
                        .rev()
                        .find(|open| close_of(texts, *open, "{", "}") == Some(at - 1))
                        .is_some_and(|open| open > 0 && texts[open - 1] == "do")
                {
                    continue;
                }
                let Some(close) = (texts.get(at + 1) == Some(&"("))
                    .then(|| close_of(texts, at + 1, "(", ")"))
                    .flatten()
                else {
                    spans.push((at, texts.len()));
                    continue;
                };
                let end = if texts.get(close + 1) == Some(&"{") {
                    close_of(texts, close + 1, "{", "}").unwrap_or(texts.len())
                } else {
                    enclosing_block_end(texts, depth, at)
                };
                spans.push((at, end));
            }
            "do" => {
                if texts.get(at + 1) == Some(&"{")
                    && let Some(close) = close_of(texts, at + 1, "{", "}")
                {
                    if texts.get(close + 1..close + 5) == Some(&["while", "(", "0", ")"][..]) {
                        continue;
                    }
                    spans.push((at, close + 1));
                } else {
                    spans.push((at, enclosing_block_end(texts, depth, at)));
                }
            }
            _ => {}
        }
    }
    spans
}

/// Whether the place of `length` tokens at `at` (`B` alone, or `B -> F`) is a
/// null test of it inside an `if` condition: its only neighbours up to that
/// condition's parentheses are `(`, `)`, `!`, `&&`, `||`, a comparison with a
/// null pointer constant, or `__builtin_expect ( … , constant )`.
pub(super) fn is_null_test(texts: &[&str], at: usize, length: usize) -> bool {
    let Some((condition_open, condition_close)) = (0..at).rev().find_map(|keyword| {
        (texts[keyword] == "if" && texts.get(keyword + 1) == Some(&"("))
            .then(|| close_of(texts, keyword + 1, "(", ")").map(|close| (keyword + 1, close)))
            .flatten()
            .filter(|(_, close)| *close >= at + length)
    }) else {
        return false;
    };
    // Left of the place.
    let mut expect = false;
    let mut cursor = at;
    loop {
        let Some(previous) = cursor.checked_sub(1) else {
            return false;
        };
        match texts[previous] {
            "(" if previous == condition_open => break,
            "(" => match previous.checked_sub(1).map(|before| texts[before]) {
                Some("__builtin_expect") => {
                    expect = true;
                    cursor = previous - 1;
                }
                Some(before) if is_name(before) || before == ")" || before == "]" => return false,
                _ => cursor = previous,
            },
            "!" => break,
            "&" | "|" if previous >= 1 && texts[previous - 1] == texts[previous] => break,
            _ => return false,
        }
    }
    // Right of the place.
    let mut cursor = at + length;
    loop {
        match texts.get(cursor).copied() {
            Some(")") if cursor == condition_close => return true,
            Some(")") => cursor += 1,
            Some("&" | "|") if texts.get(cursor + 1) == texts.get(cursor) => return true,
            Some(",")
                if expect
                    && texts.get(cursor + 1).is_some_and(|text| {
                        text.chars().all(|character| character.is_ascii_digit())
                    })
                    && texts.get(cursor + 2) == Some(&")") =>
            {
                cursor += 2;
            }
            Some("=" | "!") if texts.get(cursor + 1) == Some(&"=") => {
                let start = cursor + 2;
                let mut end = start;
                let mut depth = 0usize;
                while let Some(text) = texts.get(end).copied() {
                    match text {
                        "(" => depth += 1,
                        ")" if depth == 0 => break,
                        ")" => depth -= 1,
                        "," if depth == 0 => break,
                        "&" | "|" if depth == 0 && texts.get(end + 1) == Some(&text) => break,
                        _ => {}
                    }
                    end += 1;
                }
                if end == start || !is_null_constant(&texts[start..end]) {
                    return false;
                }
                cursor = end;
            }
            _ => return false,
        }
    }
}

/// The fix to the O04-B08 aggregate null test (`AGGREGATE-NULL-TEST-AFTER-RELEASE`):
/// before every `BranchOnNull` whose place dereferences a pointer, the typed
/// read of every pointer the place reads through, appended as its own node
/// with every edge into the test redirected to it (as the guard is).
pub(super) fn oblige_member_null_tests(cfg: &mut LegacyCfg) {
    let count = cfg.nodes.len();
    for id in 0..count {
        let LegacyInstruction::BranchOnNull { source, .. } = &cfg.nodes[id].instruction else {
            continue;
        };
        let bases = dereferenced_prefixes(source);
        if bases.is_empty() {
            continue;
        }
        let line = cfg.nodes[id].line;
        insert_before(
            cfg,
            id,
            LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                bases,
                alias_reads: Vec::new(),
                destination: None,
                frame: None,
                chains: Vec::new(),
                borrowed_destination: false,
                readback: None,
            },
            line,
        );
    }
}

/// Every prefix of a place spelling that is dereferenced by a following `->`:
/// `a->b->c` gives `a` and `a->b`; `s.f` gives nothing.
pub(super) fn dereferenced_prefixes(place: &str) -> Vec<String> {
    place
        .match_indices("->")
        .map(|(index, _)| place[..index].to_owned())
        .collect()
}

/// Append `instruction` as a node whose only successor is `id`, redirecting
/// every edge into `id` (and the entry) to it.
fn insert_before(cfg: &mut LegacyCfg, id: usize, instruction: LegacyInstruction, line: usize) {
    let inserted = cfg.nodes.len();
    cfg.nodes.push(LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction,
        successors: vec![id],
        line,
        is_exit: false,
        loop_exit_edge: None,
    });
    for (other, node) in cfg.nodes.iter_mut().enumerate() {
        if other == inserted {
            continue;
        }
        for successor in &mut node.successors {
            if *successor == id {
                *successor = inserted;
            }
        }
    }
    if cfg.entry == id {
        cfg.entry = inserted;
    }
}

pub(super) const NOT_CALLS: &[&str] = &[
    "if", "while", "for", "switch", "return", "sizeof", "_Alignof",
];

/// The functions this token stream defines whose body may release an object
/// through a formal, so whose validated summary may hide that release: a body
/// with a call whose whole actual is a pointer read (`B->F`, `B[I]`, `*B`,
/// parenthesised or not), which rule 1 may admit, and, to a fixed point, a
/// body that calls one of them. It is purely lexical and ignores contracts,
/// so it is a superset of the bodies that used rule 1 under any environment.
///
/// It is computed afresh on every call and memoised nowhere (card
/// W45-MEMO-MEMORY). The per-thread memo it had saved no measurable time, and
/// since W45-GATEWAY-MEMO-KEY its key, a copy of every token text of the last
/// unit, stayed resident in every worker thread. `QCore.MemoKey.run_eq_map`
/// is why removing it changes no answer: that memo was observationally this
/// function.
pub(super) fn member_releasing_functions(tokens: &[Token]) -> BTreeSet<String> {
    // Every file-scope `NAME ( ... ) { ... }`, with its body.
    let mut bodies = BTreeMap::<String, (usize, usize)>::new();
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].text != "{" {
            index += 1;
            continue;
        }
        let Some(close) = matching(tokens, index, "{", "}") else {
            break;
        };
        if let Some(name) = definition_name(tokens, index) {
            bodies.entry(name).or_insert((index, close));
        }
        index = close + 1;
    }
    let mut calls = BTreeMap::<&str, BTreeSet<String>>::new();
    let mut releasing = BTreeSet::<String>::new();
    for (name, (open, close)) in &bodies {
        let texts = tokens[open + 1..*close]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let callees = calls.entry(name.as_str()).or_default();
        for at in 0..texts.len() {
            if !is_name(texts[at])
                || texts.get(at + 1) != Some(&"(")
                || NOT_CALLS.contains(&texts[at])
            {
                continue;
            }
            callees.insert(texts[at].to_owned());
            let Some(spans) = call_argument_spans(&texts, at) else {
                continue;
            };
            if spans
                .iter()
                .any(|(start, end)| pointer_read_shape(&texts[*start..*end]))
            {
                releasing.insert(name.clone());
            }
        }
    }
    loop {
        let before = releasing.len();
        for (name, callees) in &calls {
            if !releasing.contains(*name) && callees.iter().any(|callee| releasing.contains(callee))
            {
                releasing.insert((*name).to_owned());
            }
        }
        if releasing.len() == before {
            return releasing;
        }
    }
}

/// The name a `{` at file scope opens the body of: `NAME ( ... ) {`.
fn definition_name(tokens: &[Token], open: usize) -> Option<String> {
    if open == 0 || tokens[open - 1].text != ")" {
        return None;
    }
    let mut depth = 0usize;
    let mut at = open - 1;
    loop {
        match tokens[at].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        at = at.checked_sub(1)?;
    }
    let name = &tokens[at.checked_sub(1)?].text;
    is_name(name).then(|| name.clone())
}

/// `B -> F`, `B [ ... ]` or `* B`, parenthesised or not: the actuals rule 1
/// may admit.
fn pointer_read_shape(argument: &[&str]) -> bool {
    match strip_parentheses(argument) {
        [base, "-", ">", field] => is_name(base) && is_name(field),
        ["*", base] => is_name(base),
        [base, "[", .., "]"] => is_name(base),
        _ => false,
    }
}

/// Every call an instruction makes, with its actuals: a contracted call and
/// its admitted nested calls, and the call inside an element store.
fn guarded_calls(instruction: &LegacyInstruction) -> Vec<(&str, &[Option<String>])> {
    match instruction {
        LegacyInstruction::ContractCall {
            function,
            arguments,
            nested,
            ..
        } => {
            let mut calls = vec![(function.as_str(), arguments.as_slice())];
            calls.extend(
                nested
                    .iter()
                    .map(|call| (call.function.as_str(), call.arguments.as_slice())),
            );
            calls
        }
        LegacyInstruction::ElementStore {
            call: Some(call), ..
        } => guarded_calls(call),
        _ => Vec::new(),
    }
}

/// The distinct callees a set of guarded calls names (tests only).
#[cfg(test)]
pub(super) fn guarded_callees(cfg: &LegacyCfg) -> BTreeSet<String> {
    cfg.nodes
        .iter()
        .filter_map(|node| match &node.instruction {
            LegacyInstruction::ArgumentReads {
                names,
                pointer_places,
                ..
            } if names.is_empty() && pointer_places.is_empty() => node
                .successors
                .first()
                .map(|next| &cfg.nodes[*next].instruction),
            _ => None,
        })
        .flat_map(|instruction| {
            guarded_calls(instruction)
                .into_iter()
                .map(|(function, _)| function.to_owned())
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/legacy_member_release.rs"]
mod tests;
