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

//! Card **C-GATEWAY-RETURN-CALL-W4**: the modeled call in `return` position —
//! wave 33's withdrawn sub-shape B, landed on a rule that answers the
//! soundness ground it was withdrawn on rather than setting that ground aside.
//!
//! # What was withdrawn, and on which two grounds
//!
//! Card `C-GATEWAY-RETURN-POSITION` admitted a pointer-valued bounded typed
//! access chain in the `return` position and **withdrew** `return CALL(args)`
//! on two separate grounds (`docs/reports/c-gateway-return-position/README.md`
//! §3):
//!
//! 1. a **yield** ground — only 12 of the 335 such callees carry a *declared*
//!    contract, so the rest would exchange a return refusal for a
//!    `call to uncontracted function` refusal and be no better decided;
//! 2. a **soundness** ground — applying a callee's parameter custody at a
//!    return position would let a `Consumed`/`Released` mode **discharge** a
//!    live obligation of the caller at that exit, which is invariant 1's
//!    forbidden direction, on a route whose exit observer
//!    [`super::observe_exit`] has no arm for a call result at all.
//!
//! Ground 1 counted one of the two routes constraint 4 admits. A call is
//! modeled through a declared contract **or** through a summary
//! `observations_match_contract` validated, and
//! [`super::analyze_defined_function`] builds every function's CFG against
//! `derivation.environment()` — the declared environment *grown with* those
//! validated summaries. So the reach question is not "how many callees are
//! declared" but "how many have a contract in force", and this card
//! re-measures it (report §3).
//!
//! **Ground 2 is not refuted by a larger reachable set**, and this module does
//! not treat it as if it were. A yield argument cannot answer a soundness
//! argument. What follows is the rule that answers it.
//!
//! # The rule: a return-position call is the assignment it already is
//!
//! `return f(a);` and `{ T = f(a); return T; }` are the same C, for a fresh `T`
//! of the result's type. The gateway already models the second form — the
//! call through [`super::Parser::classify_simple`] and the return through
//! [`super::LegacyInstruction::EscapeIfOwned`] — and has since Cycle 9. This
//! module therefore adds **no** new instruction and **no** new
//! [`super::ReturnedValue`] constructor. It performs that desugaring:
//! [`super::Parser`] binds the call's result to one reserved synthetic place
//! and the exit escapes that place, so the new case *reduces* to the case the
//! analyzer already decides. That is the answer to ground 2's mechanical half —
//! `observe_exit` needs no arm for a call result, because after the desugaring
//! there is no call at the exit.
//!
//! # Why the discharging direction is excluded, per result mode
//!
//! `ResourceReturnMode` had exactly three constructors when this route was
//! written, and the soundness question splits cleanly along them. Write `S` for the abstract state before
//! the call and `S'` for the state [`super::apply_contract_call`] leaves.
//!
//! * [`ResourceReturnMode::None`] — the destination is *removed* from the
//!   pointer environment, so the exit's `EscapeIfOwned` finds no place and
//!   observes [`super::ReturnedValue::None`]. Nothing escapes. Any resource
//!   this body still holds is therefore still held at the exit, and
//!   `report_leaks` reports it. This is the **accusing** direction: the
//!   `Unknown` it replaces marked the path unmodeled, and an unmodeled exit is
//!   one `report_leaks` skips.
//!
//! * [`ResourceReturnMode::Owned`] — `apply_contract_call` calls
//!   `create_resource` on a site drawn from `produced_sites`, and those sites
//!   come from the parser's monotonically increasing `next_site` counter. The
//!   site is therefore **fresh**: it is not a site of `S`, at all, by
//!   construction rather than by analysis. Escaping it moves *that* site to
//!   `Escaped` and no other. So the escape discharges nothing this body owed
//!   before the call — which is exactly the property ground 2 demands, stated
//!   about the one mode that can produce custody.
//!
//! * [`ResourceReturnMode::Reallocated`] — **refused, by name.** Here the
//!   result denotes the identity the caller passed at the named parameter:
//!   `apply_contract_call` records `conditional_replacements[site] = old`, so
//!   escaping the fresh site would carry the *old* site's obligation out with
//!   it. That is precisely "a `Consumed`/`Released` mode discharging a live
//!   obligation of the caller at that exit", and it is the case ground 2 was
//!   right about. It keeps the refusal it has today.
//!
//! * [`ResourceReturnMode::Derived`] (card W43-LIBC-RESULT-ALIAS, a fourth
//!   constructor) — **refused, by name**, for the same reason: the result is
//!   the argument's own value or a pointer into its object, so returning it
//!   hands the caller an alias of an object whose custody is not fresh.
//!
//!   **Amended by card W43-RESULT-ALIAS-RETURNED.** The refusal above is still
//!   the answer on every CFG the analyzer decides a function on, except one: a
//!   CFG built with `admit_derived_returns` (`build_derived_return_cfg`)
//!   desugars the derived call exactly as this module desugars an owned one.
//!   That CFG is analysed only under an entry whose result is itself `Derived`
//!   from a borrowed formal, and there the exit escapes *nothing*: the
//!   returned pointer owns nothing, and the summary hands the caller the same
//!   non-owning relation the callee's contract stated. See
//!   `legacy_summary`'s derived-result route for the validation.
//!
//! The **parameter** modes are applied by `apply_contract_call` unchanged, and
//! that is not a new trust: a statement-position call has applied them at every
//! other position since Cycle 9, and constraint 4's contract-or-validated-
//! summary discipline is the mechanism that licenses them. What this card does
//! *not* do is let the return position license anything the call itself does
//! not already license.
//!
//! # Two premises that are checked rather than assumed
//!
//! **The synthetic place must not collide.** [`RETURN_RESULT_PLACE`] is a place
//! in the same namespace as this body's C names, so a body that spells it would
//! have its own binding overwritten. The admission therefore requires the name
//! to appear nowhere in the analysed token stream and **refuses** otherwise
//! ([`place_is_unspelled`]). The name is reserved to the implementation by
//! C11 §7.1.3, so this is a fail-closed guard on an already-improbable case and
//! not a probability argument.
//!
//! **A rejected probe must leave no trace.** The desugaring asks
//! `classify_simple` to classify the synthetic statement and then *inspects*
//! the answer, so that one lookup — the existing one — decides which contract
//! is in force, and the withdrawal rewriting of constraint 8, the literal
//! premises, the nested-call admission and the variadic tail all apply exactly
//! as they do to a real assignment. `classify_simple`'s only mutation of the
//! parser is `next_site`, so a probe whose answer this module declines restores
//! it and is observationally identical to never having run. That is what keeps
//! site numbering — and therefore certificate identity — unmoved for every
//! `return` this card does not admit.
//!
//! # What is deliberately not admitted
//!
//! A callee with **no contract in force** is not admitted, so this card's
//! bucket (2) — "exchange the return leaf for a different refusal" — is empty
//! by construction, exactly as sub-shape A's was. Clearing the return refusal
//! for an uncontracted callee would report `call to uncontracted function` in
//! its place and decide nothing, which is the outcome wave 33 named as
//! unacceptable to publish as progress; the measurement of what that would have
//! yielded is published in the report instead (`RETURN-CALL-UNCONTRACTED-EXCHANGE`).
//! A call that is not the **whole** return expression — a cast of one, a
//! conditional over two, arithmetic on a result — is not admitted either: the
//! desugaring's equivalence argument is about `return f(a);` and nothing else.
//!
//! **Amended by card W43-RETURN-EXPRESSION (deliberately).** The paragraph
//! above still describes this card's *admitting* route, which still admits no
//! uncontracted callee. W43's census found that exact exchange — a whole-call
//! return whose synthetic assignment the classifier refuses — to be the
//! dominant refused family of the corpus, and [`lowers_refused_whole_call`] now
//! lowers it to the classifier's own refusal as a re-attribution that decides
//! nothing (the function stays non-`Safe`; see that function). Separately, W43
//! measured that `report_leaks` runs at **every** exit: an exit-node refusal
//! does not suppress a leak accusation, so "an unmodeled exit is one
//! `report_leaks` skips" in the `None` bullet above describes the exit
//! *residual* observation (`observe_residual`'s `indeterminate`), not the leak
//! report (`RETURN-REFUSAL-LEAK-REPORT-PREMISE`).

use qcore_schemas::contract::ResourceReturnMode;

use super::{Token, is_identifier, matching};

/// The synthetic place a `return CALL(…)` result is bound to.
///
/// Reserved to the implementation by C11 §7.1.3 (leading double underscore at
/// file scope), and additionally *checked* absent from the analysed stream by
/// [`place_is_unspelled`] before any admission.
///
/// **This name can reach a published diagnostic**, and is spelled to be read
/// there: an admitted `return f(…)` whose result is a nullable owned resource
/// reports "nullable resource `__qcore_returned_value` used by a non-null
/// contract" against the function's own entry contract. That row is a *true*
/// statement — the value this return hands back may be null, and nothing in
/// force says the function's result may be — and it is `Unknown`-class, so it
/// is the accusing direction rather than a false `Safe`. The corpus re-pin
/// attributes ten such rows across five functions
/// (`docs/reports/c-gateway-return-call-w4/README.md` §5). Rendering it as a
/// phrase rather than a place is the open residual
/// `RETURN-CALL-SYNTHETIC-PLACE-PRESENTED`; it is presentation, and deliberately
/// not taken here, because changing how a place is rendered touches every
/// diagnostic that carries one.
pub(crate) const RETURN_RESULT_PLACE: &str = "__qcore_returned_value";

/// The callee of an expression that is **exactly** one call `NAME ( … )`.
///
/// The closing parenthesis must be the last token, so a cast of a call, a
/// conditional over two calls, a call inside arithmetic and a call followed by
/// a selection all answer `None`. Whether `NAME` is a callee this projection
/// models is not decided here — that is the contract environment's answer, read
/// through `classify_simple`.
pub(crate) fn whole_expression_call<'a>(tokens: &'a [Token], texts: &[&'a str]) -> Option<&'a str> {
    if texts.len() < 3 {
        return None;
    }
    let name = *texts.first()?;
    if !is_identifier(name) || texts.get(1) != Some(&"(") {
        return None;
    }
    let close = matching(tokens, 1, "(", ")")?;
    (close == texts.len() - 1).then_some(name)
}

/// Does escaping this result at the exit discharge only custody the call itself
/// produced?
///
/// `None` escapes nothing and `Owned` escapes a site drawn fresh from the
/// parser's site counter, so neither can move a site the body already held.
/// `Reallocated` carries the replaced parameter's obligation and is refused;
/// see this module's header for the full argument.
pub(crate) fn escapes_only_fresh_custody(result: ResourceReturnMode) -> bool {
    match result {
        ResourceReturnMode::None | ResourceReturnMode::Owned { .. } => true,
        // Card W43-LIBC-RESULT-ALIAS: returning a pointer derived from an
        // argument escapes an alias of that argument's object, not fresh
        // custody, so it is refused by name exactly as a reallocation is.
        ResourceReturnMode::Reallocated { .. }
        | ResourceReturnMode::Derived { .. }
        | ResourceReturnMode::Removed { .. } => false,
    }
}

/// Card **W43-RESULT-ALIAS-RETURNED**: the place a null-constant `return`
/// escapes in a CFG built for derived-result validation.
///
/// `canonical_place` refuses `NULL` as a place, so no statement can ever bind
/// it: the escape's transfer finds no binding and is the `NoOp` it replaces.
/// Only `observe_exit` reads the name, to tell "returns a null pointer
/// constant" from "returns some pointer this projection does not track", which
/// a `NoOp` exit cannot distinguish.
pub(crate) const NULL_RETURN_PLACE: &str = "NULL";

/// Card **W43-RESULT-ALIAS-RETURNED**: is the whole `return` expression a null
/// pointer constant (C11 6.3.2.3p3)?
///
/// Read structurally after outer parentheses: `0`, `NULL`, `nullptr`, or a
/// cast of `0` to `void *` (glibc's preprocessed `NULL` is `((void *)0)`).
/// Anything else, including a cast of `0` to another pointer type, answers
/// `false` and is observed as an untracked return, which the derived route
/// refuses.
pub(crate) fn is_null_pointer_constant(texts: &[&str]) -> bool {
    let mut texts = texts;
    loop {
        match texts {
            ["0"] | ["NULL"] | ["nullptr"] => return true,
            ["(", "void", "*", ")", rest @ ..] => texts = rest,
            ["(", inner @ .., ")"] if balanced(inner) => texts = inner,
            _ => return false,
        }
    }
}

/// Whether every parenthesis in `texts` closes inside it, so stripping one
/// enclosing pair keeps the expression's structure.
fn balanced(texts: &[&str]) -> bool {
    let mut depth = 0usize;
    for text in texts {
        match *text {
            "(" => depth += 1,
            ")" => match depth.checked_sub(1) {
                Some(next) => depth = next,
                None => return false,
            },
            _ => {}
        }
    }
    depth == 0
}

/// Is `place` spelled nowhere in `tokens`?
///
/// The synthetic destination shares a namespace with this body's C names, so a
/// collision would overwrite a real binding. Absence is required, not assumed.
pub(crate) fn place_is_unspelled(tokens: &[Token], place: &str) -> bool {
    !tokens.iter().any(|token| token.text == place)
}

/// Callee spellings that take a *type* operand or denote a translation-time
/// constant. They are call-shaped to [`is_identifier`] but are not calls, so
/// "call to uncontracted function `sizeof`" would misattribute the refusal.
/// The C keywords among them are also refused by `typed_field_access::is_name`;
/// they are listed here so this rule is self-contained.
const NOT_A_CALLEE: &[&str] = &[
    "sizeof",
    "_Alignof",
    "alignof",
    "__alignof",
    "__alignof__",
    "_Generic",
    "__typeof",
    "__typeof__",
    "typeof",
    "__builtin_offsetof",
    "__builtin_types_compatible_p",
    "__builtin_va_arg",
    "__extension__",
];

/// Card **W43-RETURN-EXPRESSION**: may the ordinary statement classifier's
/// *refusal* of `PLACE = f(args);` stand in for the return-position refusal of
/// `return f(args);`?
///
/// # The family
///
/// The W43 shape census measured the refused return expressions of the pinned
/// corpus in the analyzer's own token view: the dominant family is exactly one
/// call `NAME ( … )` spanning the whole expression whose synthetic assignment
/// the statement classifier **refuses** — an uncontracted callee, a declared
/// literal premise the site does not meet, a withdrawn declaration, or an
/// argument the classifier will not model. The return-position refusal
/// ("unmodeled call, aggregate or indirect access in return expression")
/// reports none of those causes; the classifier's own refusal names them.
///
/// # Why this is sound, and why it decides nothing
///
/// `return f(a);` and `{ T = f(a); return T; }` are the same C for a fresh `T`,
/// which is the desugaring card C-GATEWAY-RETURN-CALL-W4 already performs for
/// an *admitted* call. This rule performs the same desugaring for a *refused*
/// one and keeps the classifier's answer verbatim:
///
/// * the evaluation node is the classifier's [`super::LegacyInstruction::Unknown`]
///   — an `Unsupported` diagnostic at the `return`'s own line — so the path is
///   poisoned exactly as before and the function cannot become `Safe` or
///   acquire a validated summary through it. A refusal never touches resource
///   states and applies the same bounded binding poison, so the exit state
///   `report_leaks` reads is the one the old exit-node refusal left: no leak is
///   hidden and none is manufactured;
/// * the exit is a `NoOp`: it escapes nothing, so no obligation can be
///   discharged on this route (invariant 1's forbidden direction is not
///   reachable here at all);
/// * a declined probe restores the site counter, so site numbering and
///   certificate identity are unchanged.
///
/// It is a **re-attribution**, not a verdict change: every function that loses
/// its return-position refusal through this rule keeps an `Unsupported`
/// refusal at the same line naming what actually blocks it.
///
/// # What stays refused, with the return-position wording
///
/// * any classifier answer other than a refusal — an admitted call is the
///   W4/B01 routes' business, and a `Reallocated` result they decline keeps
///   the return refusal rather than being lowered here;
/// * a call under a cast, a call inside a larger expression, a conditional, a
///   comma expression, and every call-shaped type operator in [`NOT_A_CALLEE`];
/// * a body that spells the reserved synthetic place (checked by the caller).
pub(crate) fn lowers_refused_whole_call(tokens: &[Token], texts: &[&str], refused: bool) -> bool {
    refused
        && whole_expression_call(tokens, texts)
            .is_some_and(|callee| !NOT_A_CALLEE.contains(&callee))
}

/// The synthetic `PLACE = ` prefix, carrying the `return` statement's own line
/// so every diagnostic the call reports is attributed where the source spells
/// it (constraint 12a leaves the presented coordinate derived from this
/// internal one).
pub(crate) fn assignment_prefix(line: usize) -> [Token; 2] {
    [
        Token {
            text: RETURN_RESULT_PLACE.to_owned(),
            line,
        },
        Token {
            text: "=".to_owned(),
            line,
        },
    ]
}
