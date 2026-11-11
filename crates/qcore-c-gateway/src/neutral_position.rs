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

//! Cards W4-EXPR-POS and REACH-GATE: the typed neutral-expression-position
//! admission — its POSITION half (families F1–F3) and its INDIRECT half (F4).
//!
//! # What this module decides
//!
//! `legacy_c`'s six expression positions — a branch condition, a `while`
//! condition, a `for`-loop condition, a `do`/`while` condition, a `switch`
//! expression and a `return` expression — are classified by one shared token
//! predicate, `legacy_c::supported_neutral_expression`. That predicate is
//! purely lexical: it refuses any expression carrying a call, a brace or an
//! access its `contains_unmodeled_access` scan cannot rule out, and it has no
//! access to the contract environment. Card C03-REACH measured the consequence
//! (`docs/reports/remaining-c03-reach/README.md` §3.2): the six families this
//! one predicate produces are 618 of the retained instructions that keep the
//! neutral-frame recursive route's corpus reach at zero.
//!
//! `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` §5 specifies the widening
//! and names its architectural cost: the premise the rule needs is the contract
//! environment `build_function_cfg` is *already handed*, and the work is to
//! make it reachable from the predicate rather than to loosen the predicate
//! where it lives. That is what this module is. The predicate stays exactly as
//! it is; `Parser` gains a method that consults it **first** and only then asks
//! this module, so the admission is strictly additive: an expression this
//! project accepted before is accepted by the same code, for the same reason.
//!
//! # The three admitted families, each conjunctive and typed
//!
//! **F1, the neutral call.** A call `f ( … )` is resource-neutral when the
//! contract *in force at this point of the bottom-up walk* declares every
//! parameter [`ResourceParameterMode::Borrowed`] and the result
//! [`ResourceReturnMode::None`], the written argument count equals the declared
//! parameter count, and every argument is itself admitted by this same
//! predicate. Under those conditions the call has no custody effect the
//! projection could observe: the result is untracked, and — by F2 and F3 — no
//! argument can name a pointer at all.
//!
//! **F2, the typed scalar operand.** Every identifier outside an admitted call
//! span is an enumeration constant or a variable this function declares with a
//! non-pointer, non-array arithmetic or enumeration type; every other token is
//! a numeric literal or one of C02's admitted operators. `&`, `*`, `[`, `]`,
//! `{`, `}`, `,` and `;` are absent from that operator set, so an address, an
//! indirection, a subscript, an aggregate and the comma operator each refuse.
//!
//! **F3, the typed direct scalar-field read.** Exactly card C02's whitelist,
//! evaluated in an expression position instead of a statement position: a
//! direct `base -> field` or `base . field` where `base` is declared exactly
//! `struct TAG *` or `struct TAG`, `TAG` is defined once with every member read
//! and no `union`, bitfield or nested anonymous aggregate, `field` is a
//! non-pointer, non-array arithmetic or enumeration member without `volatile`
//! or `_Atomic`, the access does not continue, and `base` is **not** a name
//! this function may bind to a resource identity.
//!
//! F2 and F3 are not re-implemented here. Both are
//! [`crate::typed_field_access::scalar_statement_accesses`], the single walker
//! card C02 landed and `QCore.LegacyCAnalyzer`'s whitelist theorems already
//! cover; this module calls it through
//! [`crate::typed_field_access::is_scalar_operand_expression`] on the tokens
//! that lie outside every admitted call span.
//!
//! **F4, the bounded typed access chain (card REACH-GATE).** The INDIRECT half
//! of the joint gate card C03-REACH measured. A `BASE (-> LINK)* -> FIELD`
//! chain of at most [`ACCESS_CHAIN_LIMIT`] selections, where `BASE` is declared
//! exactly `struct TAG *` (or `struct TAG` for a leading `.`) and is a name this
//! body **never binds**, every intermediate `LINK` is a
//! [`TypedAccessModel::record_pointer_field`] re-entered in the declaration
//! model per link, and the access does not continue into `.`, `[`, `(` or
//! another `->`. Its leaf may be
//!
//! * a **scalar** member — admitted in every position and inside an admitted
//!   call's arguments, because the value it produces is not a pointer at all;
//!   this is F3 generalised from one selection to a chain, and nothing else;
//! * a **single-pointer** member — admitted **only** where the position itself
//!   discards the value into a control-flow decision
//!   ([`ExpressionRole::Decides`]), only outside every admitted call's argument
//!   list, and only in an expression carrying no assignment.
//!
//! # Why the permissive direction is safe here
//!
//! Card W4-EXPR-POS's sentence was "an admitted neutral expression contains no
//! pointer-valued operand anywhere". F4 reads *through* record pointers, so that
//! sentence is **refined**, not violated. Its `Returned` half survives verbatim;
//! the general statement becomes:
//!
//! > **An admitted neutral expression binds, releases, escapes and addresses
//! > nothing the projection tracks.** Every call it contains is declared
//! > all-`Borrowed` / `None` at exactly its written arity over arguments that
//! > name no pointer at all; every pointer it names appears only as the base or
//! > an intermediate link of an F4 chain whose base this body never binds, so
//! > no place along that chain is a place the projection binds; and the value
//! > such a chain produces is either a scalar, or — only in a position that
//! > discards it into a control-flow decision, in an expression carrying no
//! > assignment and outside every call argument list — one pointer that is
//! > therefore stored nowhere, passed to nothing and returned to no caller.
//!
//! Each clause is load-bearing and each is checked:
//!
//! * **the base is never bound.** [`FunctionTypes::may_bind`] is the same
//!   over-approximation C02's premise 8 uses: a name written, address-taken or
//!   named anywhere in a call's argument list is bound, and a protected formal
//!   is bound at entry. The projection's own tracked `->` places come from
//!   card O04-B08's aggregate store, whose base must be a body-acquired name —
//!   which is bound by construction — so a never-bound base carries no tracked
//!   field place, at any chain depth. This is what makes reading through the
//!   chain custody-neutral rather than merely convenient, and it is why card
//!   O04-READ-BOUND's *bound*-base reads stay refused here: their premise is a
//!   flow-sensitive liveness revalidation that a condition, carrying no modeled
//!   call, has nothing to attach to.
//! * **the value is discarded, or it escapes nothing.** A pointer leaf was
//!   originally admitted only for [`ExpressionRole::Decides`], the `return`
//!   position keeping the strict no-pointer rule because its value flows to the
//!   caller and `legacy_c`'s `EscapeIfOwned` models only a *canonical place*:
//!   a compound return expression that produced a pointer would escape it
//!   unmodeled. **Card C-GATEWAY-RETURN-POSITION amends that deliberately.**
//!   The reasoning is kept and its answer is supplied rather than avoided: by
//!   the base premise `!may_bind(base)`, no place along an admitted chain is a
//!   place the projection binds, so the pointer a chain produces is not a
//!   tracked identity and there is nothing for the exit to escape. Such a
//!   `return` is therefore admitted and answered with `returned_place: None`,
//!   which makes the node a `NoOp`; [`ExpressionRole::Returned`] carries the
//!   argument and the obligation it places on `legacy_c`'s `return` parser.
//!   What is *not* admitted is any widening of `EscapeIfOwned`: the discharging
//!   direction is untouched, and a `return` of a plain canonical place keeps
//!   exactly the escape it had. The admitted set in the two roles is now
//!   **equal**, and that equality is the rule: the return position admits what
//!   a condition position admits, and the difference between them — the
//!   pointer the chain produces — escapes nothing.
//! * **no assignment, no call argument, no address.** `&`, `*`, `[`, `]`, `{`,
//!   `}`, `,` and `;` are refused by the operand walker, an assignment is
//!   refused whenever a pointer leaf is in play, and an admitted call's
//!   arguments are walked with the pointer leaf switched off, so W4-EXPR-POS's
//!   argument reasoning is preserved word for word.
//!
//! That is what makes turning the node into a `NoOp` sound rather than merely
//! convenient. A statement-position call is modeled — it checks the liveness of
//! its `Borrowed` actuals, and card C02-P-ALIAS attaches a may-alias obligation
//! to each of its read arguments. Skipping that modeling would be exactly the
//! C00 hazard the project already paid for once. Here nothing is skipped,
//! because there is nothing to model: no argument can carry a tracked identity,
//! and `crate::may_alias`'s obligation is attached where it belongs — to the
//! modeled reads of a *statement*, which this module never reclassifies. A
//! condition was already outside that obligation before this card
//! (`C02-P-ALIAS-USES`, a retained pre-existing answer) and still is.
//!
//! # What is deliberately not admitted
//!
//! * A call whose callee has **no** contract in force, or whose contract
//!   carries any non-`Borrowed` parameter or a non-`None` result. An
//!   `Acquire`, `Release`, `Escape`, `Out`, `Stored`, `Removed` or
//!   `Reallocated` callee refuses the whole expression.
//! * A call whose written arity differs from its declared parameter count.
//!   A variadic callee therefore refuses at every arity but its declared one,
//!   which is the same treatment `O04-ARMS-VARIADIC` records.
//! * A call through a function pointer, a member (`s.f(…)`, `p->f(…)`) or an
//!   expression (`(*fp)(…)`): none of those is a plain declared callee name,
//!   and each keeps the existing refusal.
//! * `sizeof`, `_Alignof` and every other reserved word in call position: they
//!   are call-shaped to [`legacy_c::is_identifier`] and are not admitted here,
//!   so the whole expression refuses exactly as it does today.
//! * Any expression in which *some* call-shaped occurrence is not admitted.
//!   The rule is all-or-nothing over the expression: one unadmitted call
//!   refuses the whole position, so no unmodelled call can hide beside an
//!   admitted one.
//! * An access chain whose base is a name this body may bind, at any depth.
//!   That is card O04-READ-BOUND's subject and its premise — a flow-sensitive
//!   liveness revalidation attached to a modeled call — is unavailable in an
//!   expression position, so it stays refused here (`REACH-GATE-BOUND-BASE`).
//! * A chain longer than [`ACCESS_CHAIN_LIMIT`] selections, a chain through a
//!   member this reader did not fully type, a leaf that is an aggregate, an
//!   array or a multi-star pointer, and any access continuing into `.`, `[`,
//!   `(` or a further `->`.
//! * An explicit dereference `* p` or a subscript `a [ i ]` anywhere, in any
//!   position and in any role (`REACH-GATE-DEREF`).
//! * A pointer-valued leaf in a `return` expression, inside an admitted call's
//!   argument list, or in an expression carrying an assignment.
//!
//! Nothing here changes a diagnostic code, an admission of a *statement*, the
//! null-test refinement, the pointer-inequality refinement or any contract.

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode, ResourceReturnMode};

use crate::legacy_c::is_identifier;
use crate::typed_field_access::{
    DeclaredType, FunctionTypes, TypedAccessModel, is_admitted_numeric_literal,
    is_admitted_operand_punctuation, is_name, is_scalar_operand_expression,
};

/// Nested admitted-call depth bound. A deeper expression is refused rather
/// than walked, in the style of the bounded access chain of
/// [`crate::borrowed_actual`]: the bound is a refusal, never a truncation.
const CALL_NESTING_LIMIT: usize = 4;

/// Maximum number of member selections one admitted F4 access chain may
/// contain, the same bound card O04-ARMS set for a borrowed actual
/// (`borrowed_actual::ACCESS_CHAIN_LIMIT`). A longer chain is refused, never
/// truncated.
const ACCESS_CHAIN_LIMIT: usize = 4;

/// Token-count bound for one classified expression. Beyond it the expression is
/// refused, so the walk is finite on any input the importer can hand it.
const EXPRESSION_TOKEN_LIMIT: usize = 4096;

/// What the position does with the value the classified expression produces.
///
/// The distinction is load-bearing for card REACH-GATE's pointer-valued leaf
/// and for nothing else: F1, F2, F3 and the scalar-leaf half of F4 decide
/// identically in both roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpressionRole {
    /// A branch, loop, `for`, `do`/`while` or `switch` condition. The value
    /// decides control flow here and is bound to no place, passed to no callee
    /// and returned to no caller.
    Decides,
    /// A `return` expression. The value flows to the caller, and `legacy_c`
    /// models that escape only for a *canonical place*.
    ///
    /// Card W4-EXPR-POS's strict no-pointer sentence was kept verbatim in this
    /// role until card **C-GATEWAY-RETURN-POSITION**, which amended it
    /// deliberately. The two roles now admit the **same** expressions; what the
    /// distinction still decides is what `legacy_c`'s `return` parser does with
    /// the admission:
    ///
    /// * an expression the `Returned` rule admits keeps its
    ///   `canonical_place` answer, so a `return p;` still escapes `p` exactly
    ///   as before — the discharging direction is untouched by this card;
    /// * an expression **only** the `Decides` rule admits is one whose value is
    ///   a pointer read through an F4 chain, and the parser **must** answer it
    ///   with `returned_place: None`, making the node a `NoOp`.
    ///
    /// That obligation is not a convenience. `canonical_place` would happily
    /// spell `view->head->next`; answering with `EscapeIfOwned` on it would be
    /// the *discharging* direction on a place the admission never established,
    /// which invariant 1 forbids. It is sound to answer `None` because the F4
    /// base premise is `!FunctionTypes::may_bind(base)`, while every tracked
    /// `->` place the projection holds (card O04-B08's aggregate store) is
    /// rooted at a body-acquired name, which `may_bind` reports bound by
    /// construction — so no place along an admitted chain, at any depth, is a
    /// place the projection binds.
    ///
    /// **Measured boundary (`RETURN-POSITION-ESCAPE-UNREACHABLE`).** Those two
    /// facts together make the `None` answer *unobservable today*: an
    /// `EscapeIfOwned` on an admitted chain would consult
    /// `state.pointers` for a place that is never there, and act as a no-op.
    /// The arm is therefore a **backstop against a future card that weakens the
    /// base premise**, justified by the argument above and by
    /// `QCore.ReturnPositionEscape`, not by a test that distinguishes it — a
    /// mutation replacing it with `canonical_place(&texts)` leaves the suite
    /// green, and that was measured rather than assumed. Do not read the green
    /// suite as evidence for this arm; read the theorem.
    Returned,
}

/// Whether an expression position is resource-neutral under the contract
/// environment in force.
///
/// This is consulted **only after** `legacy_c::supported_neutral_expression`
/// has already refused, so a `true` here is always a new admission and a
/// `false` here always leaves the existing refusal in place.
pub(crate) fn is_neutral_expression(
    texts: &[&str],
    role: ExpressionRole,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    if texts.is_empty() || texts.len() > EXPRESSION_TOKEN_LIMIT {
        return false;
    }
    neutral(
        texts,
        contracts,
        model,
        types,
        0,
        role == ExpressionRole::Decides,
    )
}

fn neutral(
    texts: &[&str],
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    depth: usize,
    pointer_leaf: bool,
) -> bool {
    if depth > CALL_NESTING_LIMIT {
        return false;
    }
    // Query operands are replaced only for this custody predicate. The caller
    // still builds/refines control flow from the original expression bytes.
    let queries = crate::type_query::scalar_operands(texts, model, types);
    if queries.is_some()
        && (contains_assignment(texts)
            || texts
                .windows(2)
                .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"])))
    {
        // This position emits NoOp, so it has no place to forget the stale
        // scalar facts a mutation would invalidate. Statement assignments use
        // OpaqueValue instead; expression mutations need their own lowering.
        return false;
    }
    let texts = queries.as_deref().unwrap_or(texts);
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut index = 0usize;
    while index + 1 < texts.len() {
        if !is_call_shaped(texts, index) {
            index += 1;
            continue;
        }
        // All-or-nothing: an occurrence this rule cannot admit refuses the
        // whole expression, exactly as `contains_call` refuses it today.
        let Some(end) = admitted_neutral_call(texts, index, contracts, model, types, depth) else {
            return false;
        };
        spans.push((index, end));
        index = end;
    }
    let outside = outside_spans(texts, &spans);
    // Card W4-EXPR-POS's F2/F3 answer first and unchanged, so this module's own
    // additivity mirrors the one `Parser::supported_expression` establishes for
    // the lexical predicate: F4 is consulted only where F2/F3 already refused.
    if is_scalar_operand_expression(&outside, model, types) {
        return true;
    }
    admitted_read_run(&outside, model, types, pointer_leaf)
}

/// Whether `texts[index]` opens a call-shaped occurrence.
///
/// Deliberately the *same* test `legacy_c::find_call` makes, including its
/// treatment of reserved words as identifiers, so that the set of occurrences
/// this rule must account for is exactly the set the existing lexical predicate
/// refuses on.
fn is_call_shaped(texts: &[&str], index: usize) -> bool {
    is_identifier(texts[index]) && texts.get(index + 1) == Some(&"(")
}

/// The index just past the closing parenthesis of an admitted neutral call at
/// `index`, or `None`.
fn admitted_neutral_call(
    texts: &[&str],
    index: usize,
    contracts: &ResourceContracts,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    depth: usize,
) -> Option<usize> {
    let name = texts[index];
    // A reserved word is never a declared callee.
    if !is_name(name) {
        return None;
    }
    // The callee must be a plain name, not a selected member and not a
    // declarator following a type specifier or a literal.
    if index > 0 {
        let previous = texts[index - 1];
        if previous == "."
            || (previous == ">" && index >= 2 && texts[index - 2] == "-")
            || is_identifier(previous)
            || previous.as_bytes().first().is_some_and(u8::is_ascii_digit)
        {
            return None;
        }
    }
    let close = matching_close(texts, index + 1)?;
    let contract = contracts.contract(name)?;
    if contract.result != ResourceReturnMode::None {
        return None;
    }
    if !contract
        .parameters
        .iter()
        .all(|mode| *mode == ResourceParameterMode::Borrowed)
        // Card W45-FB-RETRY-LIFETIME: a container effect is not neutral.
        || contract.has_container_effect()
    {
        return None;
    }
    let inner = &texts[index + 2..close];
    let arguments = if inner.is_empty() {
        Vec::new()
    } else {
        split_arguments(inner)
    };
    // An exact arity, so a variadic or mis-declared callee cannot pass an
    // argument this rule never examined.
    if arguments.len() != contract.parameters.len() {
        return None;
    }
    for argument in &arguments {
        // An argument is walked with the pointer leaf switched off, in every
        // role. Card W4-EXPR-POS's sentence about call arguments — that no
        // argument of an admitted call can carry a tracked identity, because
        // none of them names a pointer at all — therefore survives card
        // REACH-GATE unchanged.
        if argument.is_empty() || !neutral(argument, contracts, model, types, depth + 1, false) {
            return None;
        }
    }
    Some(close + 1)
}

/// **F4.** Whether an operand run outside every admitted call span is a
/// sequence of admitted bounded typed access chains, declared scalars,
/// enumeration constants, numeric literals and admitted punctuation.
///
/// Consulted only after [`is_scalar_operand_expression`] has refused, so it can
/// only add admissions. `pointer_leaf` says whether the position discards the
/// value it produces; see [`ExpressionRole`].
fn admitted_read_run(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    pointer_leaf: bool,
) -> bool {
    // A function whose declarations this reader refused admits nothing, exactly
    // as in C02's walker.
    if !types.is_readable() {
        return false;
    }
    // The same exclusions C02's walker makes, by the same decision: an address,
    // an indirection, a subscript, an aggregate, the comma operator and a
    // statement separator each refuse (`REACH-GATE-DEREF`).
    if texts
        .iter()
        .any(|text| matches!(*text, "&" | "*" | "[" | "]" | "{" | "}" | "," | ";"))
    {
        return false;
    }
    // A pointer value is admitted only where it is discarded. An assignment
    // could store it, so an expression carrying one keeps the strict rule.
    if pointer_leaf && contains_assignment(texts) {
        return false;
    }
    let mut cursor = 0usize;
    while cursor < texts.len() {
        let text = texts[cursor];
        if is_name(text) {
            let arrow = texts.get(cursor + 1) == Some(&"-") && texts.get(cursor + 2) == Some(&">");
            let dot = texts.get(cursor + 1) == Some(&".");
            if arrow || dot {
                let Some(end) = admitted_chain(texts, cursor, model, types, pointer_leaf) else {
                    return false;
                };
                cursor = end;
                continue;
            }
            if model.is_enum_constant(text) {
                cursor += 1;
                continue;
            }
            // A remaining identifier must be a declared scalar. A pointer-typed
            // name standing on its own is refused: this rule reads *through*
            // record pointers, it never admits one as a bare operand.
            if types.declared_type(text) != Some(&DeclaredType::Scalar) {
                return false;
            }
            cursor += 1;
            continue;
        }
        if is_admitted_numeric_literal(text) {
            cursor += 1;
            continue;
        }
        if text == "-" && texts.get(cursor + 1) == Some(&">") {
            // An arrow whose base this reader did not admit.
            return false;
        }
        if !is_admitted_operand_punctuation(text) {
            return false;
        }
        cursor += 1;
    }
    true
}

/// The index just past an admitted F4 access chain starting at `start`, or
/// `None`.
///
/// The chain discipline is card O04-ARMS's, re-derived here for a read that no
/// call consumes: the base is declared exactly `struct TAG *` (or `struct TAG`
/// for a leading `.`) and is a name this body never binds; each selection
/// before the last is a fully typed [`TypedAccessModel::record_pointer_field`]
/// continued with `->`, with the model re-entered at the pointee; and the leaf
/// must not continue into `.`, `[`, `(` or a further `->`.
fn admitted_chain(
    texts: &[&str],
    start: usize,
    model: &TypedAccessModel,
    types: &FunctionTypes,
    pointer_leaf: bool,
) -> Option<usize> {
    let base = texts.get(start).copied().filter(|text| is_name(text))?;
    // C02 premise 8, verbatim: a base this body may bind can hold a released,
    // escaped or reallocated identity. Refusing it is what keeps every place
    // along the chain outside the projection's own bindings.
    if types.may_bind(base) {
        return None;
    }
    let (mut field_index, arrow) =
        if texts.get(start + 1) == Some(&"-") && texts.get(start + 2) == Some(&">") {
            (start + 3, true)
        } else if texts.get(start + 1) == Some(&".") {
            (start + 2, false)
        } else {
            return None;
        };
    let mut tag = match (types.declared_type(base), arrow) {
        (Some(DeclaredType::RecordPointer(tag)), true) => tag.clone(),
        (Some(DeclaredType::Record(tag)), false) => tag.clone(),
        _ => return None,
    };
    for _ in 0..ACCESS_CHAIN_LIMIT {
        let field = texts
            .get(field_index)
            .copied()
            .filter(|field| is_name(field))?;
        let continues_with_arrow =
            texts.get(field_index + 1) == Some(&"-") && texts.get(field_index + 2) == Some(&">");
        if continues_with_arrow {
            tag = model.record_pointer_field(&tag, field)?.to_owned();
            field_index += 3;
            continue;
        }
        // The leaf. A scalar member is admitted in every role; a member holding
        // exactly one pointer only where the position discards the value.
        let admitted = model.scalar_field(&tag, field)
            || (pointer_leaf && model.single_pointer_field(&tag, field));
        if !admitted {
            return None;
        }
        match texts.get(field_index + 1) {
            Some(&".") | Some(&"[") | Some(&"(") => return None,
            _ => {}
        }
        return Some(field_index + 1);
    }
    None
}

/// Whether the run carries an assignment, simple or compound.
///
/// An `=` is a comparison only when it is the first half of `==` or the second
/// half of `==`, `!=`, `<=` or `>=`. Every other `=` writes, including the
/// compound forms `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=` and `^=`, whose
/// preceding operator is deliberately absent from the accepted set.
fn contains_assignment(texts: &[&str]) -> bool {
    texts.iter().enumerate().any(|(index, text)| {
        *text == "="
            && texts.get(index + 1) != Some(&"=")
            && !matches!(
                index.checked_sub(1).and_then(|at| texts.get(at)).copied(),
                Some("=") | Some("!") | Some("<") | Some(">")
            )
    })
}

/// The index of the `)` matching the `(` at `open`, or `None` when the
/// expression is unbalanced.
fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" => depth += 1,
            ")" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// One call's arguments, split on the commas that are not inside a nested
/// grouping. `[` and `{` are counted so that a comma inside one is not a
/// separator; both refuse later anyway, in [`is_scalar_operand_expression`].
fn split_arguments<'a>(texts: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut groups: Vec<Vec<&'a str>> = vec![Vec::new()];
    let mut depth = 0usize;
    for text in texts {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                groups.push(Vec::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = groups.last_mut() {
            last.push(*text);
        }
    }
    groups
}

/// The tokens outside every admitted call span.
///
/// Deleting a whole `NAME ( … )` span can only bring operators or parentheses
/// into contact. It can never manufacture an admitted access out of neighbours
/// that did not have one. A deleted span contributes no identifier to the
/// result; a surviving `*`, `&`, `[` or `]` is refused by both operand walkers;
/// a surviving `-` `>` pair with no admitted base before it is refused by name
/// in [`admitted_read_run`]; and a surviving `.` cannot acquire a fresh base,
/// because [`admitted_neutral_call`] refuses a callee preceded by an identifier
/// or by `.`, so no admitted span is ever adjacent to one on either side.
fn outside_spans<'a>(texts: &[&'a str], spans: &[(usize, usize)]) -> Vec<&'a str> {
    texts
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            !spans
                .iter()
                .any(|(start, end)| index >= start && index < end)
        })
        .map(|(_, text)| *text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::contract::{ResourceApiKind, ResourceFunctionContract};
    use std::collections::BTreeSet;

    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphabetic() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else if byte.is_ascii_digit() {
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    /// A neutral contract of the given arity: exactly what
    /// `legacy_neutral::neutral_contract` seeds and what ordinary W4 inference
    /// publishes for an effect-free body.
    fn neutral_contract(arity: usize) -> ResourceFunctionContract {
        ResourceFunctionContract::new(
            vec![ResourceParameterMode::Borrowed; arity],
            ResourceReturnMode::None,
        )
    }

    fn environment(entries: Vec<(&str, ResourceFunctionContract)>) -> ResourceContracts {
        let mut contracts = ResourceContracts::default();
        for (name, contract) in entries {
            contracts.insert_contract(name, contract);
        }
        contracts
    }

    /// Declaration facts for a fixture: `unit` is the translation-unit text and
    /// `body` the tokens between the function's braces. No formal is protected,
    /// which is the neutral route's own entry shape.
    fn facts(unit: &str, body: &str) -> (TypedAccessModel, FunctionTypes) {
        let unit_tokens = texts(unit);
        let model = TypedAccessModel::read(&unit_tokens);
        let body_tokens = texts(body);
        let types = FunctionTypes::read(&model, &[], &body_tokens, &BTreeSet::new());
        (model, types)
    }

    fn admits_in(
        role: ExpressionRole,
        unit: &str,
        body: &str,
        expression: &str,
        env: &ResourceContracts,
    ) -> bool {
        let (model, types) = facts(unit, body);
        let tokens = texts(expression);
        is_neutral_expression(&tokens, role, env, &model, &types)
    }

    /// The `Decides` role — a branch, loop or `switch` position. Every
    /// W4-EXPR-POS assertion below reads the same in either role; only card
    /// REACH-GATE's pointer-valued leaf distinguishes them, and those
    /// assertions name their role explicitly.
    fn admits(unit: &str, body: &str, expression: &str, env: &ResourceContracts) -> bool {
        admits_in(ExpressionRole::Decides, unit, body, expression, env)
    }

    fn returns(unit: &str, body: &str, expression: &str, env: &ResourceContracts) -> bool {
        admits_in(ExpressionRole::Returned, unit, body, expression, env)
    }

    const SCALAR_BODY: &str = "unsigned long v ; int n ;";
    const NO_UNIT: &str = "struct r { int a ; struct q * p ; } ;";

    #[test]
    fn the_digits10_return_expression_is_admitted() {
        let env = environment(vec![("digits10", neutral_contract(1))]);
        assert!(admits(
            NO_UNIT,
            SCALAR_BODY,
            "12 + digits10 ( v / 1000000000000UL )",
            &env
        ));
    }

    #[test]
    fn a_call_without_a_contract_refuses() {
        let env = environment(Vec::new());
        assert!(!admits(NO_UNIT, SCALAR_BODY, "12 + digits10 ( v )", &env));
    }

    #[test]
    fn a_releasing_callee_refuses() {
        let env = environment(vec![(
            "digits10",
            ResourceFunctionContract::from(ResourceApiKind::Release),
        )]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "digits10 ( v )", &env));
    }

    #[test]
    fn an_acquiring_callee_refuses() {
        let env = environment(vec![(
            "make",
            ResourceFunctionContract::from(ResourceApiKind::Acquire),
        )]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "make ( )", &env));
    }

    #[test]
    fn an_owned_result_refuses() {
        let env = environment(vec![(
            "peek",
            ResourceFunctionContract::new(
                [ResourceParameterMode::Borrowed],
                ResourceReturnMode::Owned { nullable: true },
            ),
        )]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "peek ( v )", &env));
    }

    #[test]
    fn a_wrong_arity_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( v , n )", &env));
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( )", &env));
    }

    #[test]
    fn a_pointer_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct q { int a ; } ;";
        let body = "struct q * p ;";
        assert!(!admits(unit, body, "f ( p )", &env));
    }

    #[test]
    fn an_address_of_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( & n )", &env));
    }

    #[test]
    fn a_dereferenced_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct q { int a ; } ;";
        let body = "int * q ;";
        assert!(!admits(unit, body, "f ( * q )", &env));
    }

    #[test]
    fn a_subscripted_argument_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct q { int a ; } ;";
        let body = "int a [ 4 ] ;";
        assert!(!admits(unit, body, "f ( a [ 0 ] )", &env));
    }

    #[test]
    fn a_member_call_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let unit = "struct r { int a ; } ;";
        let body = "struct r * s ; unsigned long v ;";
        assert!(!admits(unit, body, "s -> f ( v )", &env));
        assert!(!admits(unit, body, "s . f ( v )", &env));
    }

    #[test]
    fn a_function_pointer_call_refuses() {
        let env = environment(vec![("fp", neutral_contract(1))]);
        let body = "unsigned long v ;";
        assert!(!admits(NO_UNIT, body, "( * fp ) ( v )", &env));
    }

    #[test]
    fn scalar_sizeof_is_admitted_without_borrowing_a_callee_contract() {
        let env = environment(vec![("sizeof", neutral_contract(1))]);
        assert!(admits(NO_UNIT, SCALAR_BODY, "sizeof ( n )", &env));
        assert!(admits(
            NO_UNIT,
            SCALAR_BODY,
            "sizeof ( n )",
            &environment(vec![])
        ));
        assert!(!admits(NO_UNIT, SCALAR_BODY, "sizeof ( n [ 2 ] )", &env));
    }

    #[test]
    fn one_unadmitted_call_refuses_the_whole_expression() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( n ) + g ( n )", &env));
    }

    #[test]
    fn a_comma_operator_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( n ) , n", &env));
    }

    #[test]
    fn a_nested_neutral_call_is_admitted() {
        let env = environment(vec![("f", neutral_contract(1)), ("g", neutral_contract(1))]);
        assert!(admits(NO_UNIT, SCALAR_BODY, "f ( g ( n ) )", &env));
    }

    #[test]
    fn a_nested_unadmitted_call_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( g ( n ) )", &env));
    }

    #[test]
    fn a_direct_scalar_field_read_is_admitted() {
        let env = environment(Vec::new());
        let unit = "struct r { int a ; struct q * p ; } ;";
        let body = "struct r * s ;";
        assert!(admits(unit, body, "s -> a", &env));
    }

    #[test]
    fn a_pointer_field_read_is_admitted_only_where_the_value_is_discarded() {
        let env = environment(Vec::new());
        let unit = "struct r { int a ; struct q * p ; } ;";
        let body = "struct r * s ;";
        // Deliberate pin update, card REACH-GATE. W4-EXPR-POS pinned this shape
        // refused in every position because F3's leaf had to be scalar. F4
        // admits it exactly where the position discards the value; both
        // load-bearing halves of the old pin are kept below.
        assert!(admits(unit, body, "s -> p", &env));
        assert!(!returns(unit, body, "s -> p", &env));
        let bound_body = "struct r * s ; use ( s ) ;";
        assert!(!admits(unit, bound_body, "s -> p", &env));
        assert!(!returns(unit, bound_body, "s -> p", &env));
    }

    #[test]
    fn a_bound_base_refuses() {
        let env = environment(Vec::new());
        let unit = "struct r { int a ; } ;";
        // `s` appears in a call argument list, so `bound_names` records it.
        let body = "struct r * s ; use ( s ) ;";
        assert!(!admits(unit, body, "s -> a", &env));
    }

    #[test]
    fn an_unreadable_declaration_refuses_every_expression() {
        let env = environment(vec![("f", neutral_contract(1))]);
        let (model, types) = facts(NO_UNIT, "struct { int a ; } anon ;");
        assert!(!types.is_readable());
        let tokens = texts("f ( 1 )");
        assert!(!is_neutral_expression(
            &tokens,
            ExpressionRole::Decides,
            &env,
            &model,
            &types
        ));
    }

    #[test]
    fn an_empty_expression_refuses() {
        let env = environment(Vec::new());
        assert!(!admits(NO_UNIT, SCALAR_BODY, "", &env));
    }

    #[test]
    fn a_deeply_nested_call_chain_refuses_at_the_bound() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(admits(
            NO_UNIT,
            SCALAR_BODY,
            "f ( f ( f ( f ( n ) ) ) )",
            &env
        ));
        assert!(!admits(
            NO_UNIT,
            SCALAR_BODY,
            "f ( f ( f ( f ( f ( f ( n ) ) ) ) ) )",
            &env
        ));
    }

    #[test]
    fn an_unbalanced_call_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(NO_UNIT, SCALAR_BODY, "f ( n", &env));
    }

    // ---------------------------------------------------------------------
    // Card REACH-GATE: F4, the bounded typed access chain.
    // ---------------------------------------------------------------------

    /// A three-record chain: `struct a` -> `struct b` -> `struct c`, with a
    /// scalar leaf on each and a pointer leaf on the first.
    const CHAIN_UNIT: &str = "struct c { int leaf ; } ; \
         struct b { int mid ; struct c * down ; } ; \
         struct a { int top ; struct b * next ; struct b * other ; } ;";
    const CHAIN_BODY: &str = "struct a * head ; int n ;";

    #[test]
    fn a_chain_to_a_scalar_leaf_is_admitted_in_either_role() {
        let env = environment(Vec::new());
        assert!(admits(CHAIN_UNIT, CHAIN_BODY, "head -> next -> mid", &env));
        assert!(returns(CHAIN_UNIT, CHAIN_BODY, "head -> next -> mid", &env));
        assert!(admits(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next -> down -> leaf",
            &env
        ));
        assert!(returns(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next -> down -> leaf",
            &env
        ));
    }

    #[test]
    fn a_chain_to_a_pointer_leaf_splits_on_the_role() {
        let env = environment(Vec::new());
        assert!(admits(CHAIN_UNIT, CHAIN_BODY, "head -> next -> down", &env));
        assert!(!returns(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next -> down",
            &env
        ));
    }

    #[test]
    fn a_chain_past_the_depth_bound_refuses() {
        // Five selections: one more than ACCESS_CHAIN_LIMIT. The record model
        // is deliberately made cyclic so the chain is well typed at every link
        // and only the bound refuses it.
        let unit = "struct l { int leaf ; struct l * next ; } ;";
        let body = "struct l * head ;";
        assert!(admits(
            unit,
            body,
            "head -> next -> next -> leaf",
            &env_empty()
        ));
        assert!(admits(
            unit,
            body,
            "head -> next -> next -> next -> leaf",
            &env_empty()
        ));
        assert!(!admits(
            unit,
            body,
            "head -> next -> next -> next -> next -> leaf",
            &env_empty()
        ));
    }

    fn env_empty() -> ResourceContracts {
        environment(Vec::new())
    }

    #[test]
    fn a_chain_through_an_untyped_link_refuses() {
        // `opaque` is `void *`: not a record pointer, so the chain cannot be
        // continued through it.
        let unit = "struct a { void * opaque ; int top ; } ;";
        let body = "struct a * head ;";
        assert!(!admits(
            unit,
            body,
            "head -> opaque -> anything",
            &env_empty()
        ));
    }

    #[test]
    fn a_bound_base_refuses_a_chain_in_either_role() {
        let env = environment(Vec::new());
        // `head` appears in a call argument list, so `bound_names` records it.
        let body = "struct a * head ; use ( head ) ;";
        assert!(!admits(CHAIN_UNIT, body, "head -> next -> mid", &env));
        assert!(!returns(CHAIN_UNIT, body, "head -> next -> mid", &env));
        assert!(!admits(CHAIN_UNIT, body, "head -> next", &env));
    }

    #[test]
    fn a_released_base_is_refused_by_the_same_premise() {
        let env = environment(vec![(
            "release",
            ResourceFunctionContract::from(ResourceApiKind::Release),
        )]);
        // The body releases through `head`, so it is bound and every read
        // through it refuses — the C02 §2 hazard, in an expression position.
        let body = "struct a * head ; release ( head ) ;";
        assert!(!admits(CHAIN_UNIT, body, "head -> next -> mid", &env));
        assert!(!admits(CHAIN_UNIT, body, "head -> next", &env));
    }

    #[test]
    fn an_address_of_a_chain_refuses_in_a_condition() {
        let env = environment(Vec::new());
        assert!(!admits(CHAIN_UNIT, CHAIN_BODY, "& head -> next", &env));
        assert!(!returns(CHAIN_UNIT, CHAIN_BODY, "& head -> next", &env));
    }

    #[test]
    fn a_dereference_or_subscript_refuses_in_a_condition() {
        let env = environment(Vec::new());
        assert!(!admits(CHAIN_UNIT, CHAIN_BODY, "* head -> next", &env));
        assert!(!admits(CHAIN_UNIT, CHAIN_BODY, "head -> next [ 0 ]", &env));
    }

    #[test]
    fn an_assignment_refuses_a_pointer_leaf() {
        let env = environment(Vec::new());
        let body = "struct a * head ; struct b * held ;";
        // `held = head->next` would store the pointer this rule only ever
        // discards, so the whole expression refuses.
        assert!(!admits(CHAIN_UNIT, body, "held = head -> next", &env));
        // A comparison is not an assignment and stays admitted.
        assert!(admits(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next != head -> other",
            &env
        ));
        assert!(admits(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next -> mid <= 3",
            &env
        ));
    }

    #[test]
    fn a_pointer_leaf_refuses_inside_an_admitted_call_argument() {
        let env = environment(vec![("f", neutral_contract(1))]);
        // The scalar leaf of the same chain is admitted there; the pointer leaf
        // is not, so W4-EXPR-POS's argument sentence survives verbatim.
        assert!(admits(
            CHAIN_UNIT,
            CHAIN_BODY,
            "f ( head -> next -> mid )",
            &env
        ));
        assert!(!admits(CHAIN_UNIT, CHAIN_BODY, "f ( head -> next )", &env));
    }

    #[test]
    fn a_bare_pointer_name_is_never_an_operand() {
        let env = environment(Vec::new());
        assert!(!admits(CHAIN_UNIT, CHAIN_BODY, "head", &env));
        assert!(!admits(CHAIN_UNIT, CHAIN_BODY, "head != 0", &env));
    }

    #[test]
    fn a_chain_continuing_into_a_call_or_subscript_refuses() {
        let env = environment(vec![("f", neutral_contract(1))]);
        assert!(!admits(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next -> mid ( n )",
            &env
        ));
        assert!(!admits(
            CHAIN_UNIT,
            CHAIN_BODY,
            "head -> next -> mid [ n ]",
            &env
        ));
    }

    #[test]
    fn a_union_or_incomplete_record_refuses_the_chain() {
        let unit = "union u { int leaf ; void * p ; } ; \
             struct a { int top ; union u * bad ; } ;";
        let body = "struct a * head ;";
        assert!(!admits(unit, body, "head -> bad -> leaf", &env_empty()));
    }

    #[test]
    fn a_record_value_base_admits_only_its_first_selection_by_dot() {
        let unit = "struct b { int mid ; struct c * down ; } ; \
             struct c { int leaf ; } ;";
        let body = "struct b value ;";
        assert!(admits(unit, body, "value . mid", &env_empty()));
        assert!(admits(unit, body, "value . down -> leaf", &env_empty()));
        // A second `.` would cross a nested aggregate sub-object.
        let nested = "struct c { int leaf ; } ; struct b { struct c inner ; } ;";
        assert!(!admits(
            nested,
            "struct b value ;",
            "value . inner . leaf",
            &env_empty()
        ));
    }

    #[test]
    fn a_scalar_only_expression_still_decides_by_the_earlier_family() {
        // Additivity: F2's answer is unchanged and is consulted first.
        let env = environment(Vec::new());
        assert!(admits(NO_UNIT, SCALAR_BODY, "n + 1", &env));
        assert!(returns(NO_UNIT, SCALAR_BODY, "n + 1", &env));
    }
}
