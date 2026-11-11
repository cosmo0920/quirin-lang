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

//! Instrumented census of the `indirect memory access is outside the resource
//! projection` refusal (card **C-GATEWAY-INDIRECT-CENSUS-2**).
//!
//! # What this module is, and what it is not
//!
//! It is a **measurement**, not a rule. Nothing here is consulted by any
//! analysis, certificate, summary or report: the sink is written at the moment
//! the classifier has *already decided* to refuse, and is read only by the
//! census entry point. It moves no verdict, admits nothing and refuses nothing.
//!
//! The whole module — and every call site of [`record`] in
//! [`crate::legacy_c`] — is behind the off-by-default cargo feature
//! `indirect-census`. With the feature off, none of this code exists in the
//! compiled crate, which is the sense in which it is inert in production. The
//! byte-identity assertion the card publishes measures the complementary half:
//! that with the feature **on**, the pinned corpus audit still produces the
//! archived artifacts byte for byte, so the sink perturbs no decision either.
//!
//! # Why the analyzer's own view, and not the source text
//!
//! The abandoned first census card classified the **original** source line of
//! each refusal. Refusals are recorded at the **preprocessed** line
//! (constraint 12a), and card `C-GATEWAY-CONSTANT-DO-WHILE` measured the gap
//! between the two at roughly a factor of 57 for macro idioms, so every
//! macro-expanded access was misclassified. This module reads the token texts
//! the gate itself read, the declaration facts the gate itself consulted
//! ([`crate::typed_field_access::TypedAccessModel`],
//! [`crate::typed_field_access::FunctionTypes`]) and the never-bound base set
//! the gate itself holds ([`crate::borrowed_actual::UntrackedBases`]). There is
//! therefore no second reader that could disagree with the first.
//!
//! # The premise axis
//!
//! `base_may_bind` is [`FunctionTypes::may_bind`], the analyzer's own
//! over-approximation of the names a function may bind to a resource identity.
//! Its negation is literally constraint 5's load-bearing premise — "a name this
//! function never binds to a resource identity" — so an arbiter's admission
//! space is bounded by the `base_may_bind == false` population and cannot be
//! argued larger without weakening the premise the fail-closed pin of card
//! `remaining-c02` already caught once.
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::borrowed_actual::UntrackedBases;
use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// Which of the classifier's three `indirect memory access` constructors fired.
///
/// They are distinguished because they refuse for different reasons and an
/// arbiter would have to answer a different question at each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum RefusalSite {
    /// The lexical access gate: the statement's destination is an indirect
    /// place, or a token run outside every exempted span contains `->`, `.`,
    /// `[`, `]` or a dereferencing `*`.
    LexicalGate,
    /// A contracted call whose owned result would be stored into an admitted
    /// aggregate member that is not declared with exactly one `*`.
    NonPointerResultStore,
    /// An admitted aggregate store whose *value* shape
    /// [`crate::aggregate_store::classify_value`] does not cover.
    UnclassifiedStoreValue,
}

/// Which sub-branch of the lexical gate's disjunction fired, i.e. the syntactic
/// position of the refused access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Position {
    /// An assignment whose destination place contains `.`, `[` or `->` and
    /// which the aggregate-store whitelist did not admit. A **write**.
    AssignmentDestination,
    /// An assignment whose destination is admitted or plain, but whose value
    /// side carries an access outside every exempted span. A **read**.
    AssignmentValue,
    /// A declaration statement with a non-constant bracket group or a `.`.
    Declaration,
    /// A statement with no `=` at all that contains a call: the access is in an
    /// argument or in the callee expression.
    CallArgument,
    /// A statement with no `=` and no call: a bare expression statement.
    ExpressionStatement,
}

/// The kind of storage an access chain's base denotes, as far as the
/// analyzer's own declaration reader will commit to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum BaseKind {
    /// A formal parameter of this function.
    Formal,
    /// A name this function declares, and whose declared type the reader read.
    AutomaticLocal,
    /// A name the reader read no declaration for in this function: a file-scope
    /// object, a `static`, an enumeration constant or a name the reader
    /// refused. Fail-closed lumping: every one of these is outside the
    /// function's own declaration set.
    OutsideFunctionDeclarations,
    /// The chain hangs off a call's result rather than off a name.
    CallResult,
    /// The declaration reader refused this whole function, so no name in it has
    /// a committed type.
    UnreadableDeclarations,
}

/// One step of an access chain, in the spelling the token stream carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Step {
    /// `->`
    Arrow,
    /// `.`
    Dot,
    /// `[` … `]`
    Subscript,
    /// A prefix `*` applied to the base.
    Deref,
}

/// The class of the member a chain ends at, in the vocabulary the analyzer's
/// own record reader already uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum MemberClass {
    /// A declared scalar member: the class constraint 5's whitelist is about.
    Scalar,
    /// A member declared exactly `struct TAG *` — a chain may continue through
    /// it.
    RecordPointer,
    /// A member declared with exactly one `*` over a resolved specifier list,
    /// other than the `struct TAG *` case.
    SinglePointer,
    /// A member declared exactly `void * name`.
    VoidPointer,
    /// A non-pointer object member that is not a scalar: a record- or
    /// union-valued sub-object.
    AggregateValued,
    /// A pointer member of some depth the borrowable set admits but no narrower
    /// set does.
    OtherPointer,
    /// The chain does not end at a member of a record this reader resolved: an
    /// array, a bitfield, a function pointer, a `union` member, an unresolved
    /// typedef, or a base whose type the reader will not commit to.
    Unresolved,
    /// The chain ends at a subscript or a bare dereference, so there is no
    /// member to classify.
    NoMember,
}

/// One access chain inside one refused statement.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Chain {
    pub base: String,
    pub base_kind: BaseKind,
    /// [`FunctionTypes::may_bind`]: whether this function may bind the base to
    /// a resource identity. **Constraint 5's premise, read positively.**
    pub base_may_bind: bool,
    /// [`UntrackedBases::never_bound`]: the same question as the admission
    /// rules already ask it, which additionally requires the declaration reader
    /// to have succeeded.
    pub base_never_bound: bool,
    /// Whether the base's declared type is a record pointer this reader
    /// committed to.
    pub base_record_pointer: bool,
    /// The record tag the chain's LAST selection was resolved against, when the
    /// reader committed to one. This is what lets the census join a refused
    /// access to the record-body reading that refused its member.
    pub tag: Option<String>,
    pub steps: Vec<Step>,
    pub depth: usize,
    pub member_class: MemberClass,
    /// The final member's spelling, when the chain ends at one.
    pub member: Option<String>,
    /// Card **BASE-TYPE-READER-CENSUS**: why the base's declared type did not
    /// commit to a record tag, in the declaration reader's own vocabulary, or
    /// [`BaseTypeCause::Committed`] when it did.
    pub base_cause: BaseTypeCause,
    /// The step index (0-based, among `steps`) at which a tag the reader **had
    /// committed to** was lost — a member that is not a `struct TAG *`, or a
    /// subscript. `None` when the base never had a tag, or when the chain kept
    /// one throughout.
    ///
    /// This is what separates the two halves of the wave-36 card's `tag: null`
    /// row: `base_cause != Committed` is a **base**-type question, and
    /// `base_cause == Committed` with a `tag_lost_step` is a *member* question
    /// the wave-36 card already owns.
    pub tag_lost_step: Option<usize>,
    /// For a chain hanging off a call's result: the callee's spelling where the
    /// token view carries one.
    pub callee: Option<String>,
}

/// One refused statement: one leaf of the corpus's largest refusal family.
#[derive(Clone, Debug, serde::Serialize)]
pub struct IndirectLeaf {
    /// Grouping key: the ordinal of the translation-unit analysis this record
    /// was produced in. The gateway has no unit label at this depth, so the
    /// census joins to the published document by this ordinal plus the
    /// `(function, line)` pair; see the card's report.
    pub unit_seq: u64,
    pub function: String,
    /// The **internal preprocessed** line, which is the leaf's identity
    /// (constraint 12a). No presented coordinate is read here.
    pub line: usize,
    pub site: RefusalSite,
    pub position: Position,
    /// Whether the refused statement contains a call at all.
    pub contains_call: bool,
    /// Whether the statement's first call — the one the classifier models — is
    /// to a function a declared `.qcore` contract or a validated summary
    /// covers.
    ///
    /// This separates the two futures of a refused call statement, and a census
    /// that could not tell them apart would report every `f(p->x)` as an
    /// exchange. `None` when the statement carries no call.
    pub call_contracted: Option<bool>,
    /// The number of tokens in the refused statement, as a crude size axis.
    pub tokens: usize,
    /// Every access chain the gate's own token view carries.
    pub chains: Vec<Chain>,
    /// Card **W43-INDIRECT-ACCESS-GENERAL**: the complete token text of the
    /// refused statement, exactly as the classifier read it, so the shape
    /// census classifies the analyzer's own view rather than source text.
    pub texts: Vec<String>,
    /// The names of this statement the body assigns a declared *derived*
    /// result (`strchr`, `memcpy`, …). A label, not a flow fact: the name may
    /// be rebound elsewhere.
    pub derived_names: Vec<String>,
    /// The names of this statement the body assigns a declared *owned* or
    /// reallocated result. The same kind of label.
    pub owned_names: Vec<String>,
    /// Per identifier of the statement, the declared-type label the
    /// function's declaration reader gives it: `scalar`, `record`,
    /// `record_pointer:TAG`, `opaque:CAUSE`, or absent when undeclared.
    pub declared: std::collections::BTreeMap<String, String>,
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**: census-only labels of one function
/// body's `NAME = CALLEE (` assignments, keyed by the callee's declared result.
#[derive(Clone, Debug, Default)]
pub struct BodyBindings {
    pub derived: BTreeSet<String>,
    pub owned: BTreeSet<String>,
}

/// The census stream, opened once on first use.
///
/// The inner `None` means the census is not collecting: either
/// [`CENSUS_PATH_VARIABLE`] names no path, or the path could not be opened. A
/// census that cannot write is **silent rather than fatal** — it must not be
/// able to change what an audit run does.
static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);
static UNIT_SEQ: AtomicU64 = AtomicU64::new(0);

/// The environment variable that turns collection on, and names the file the
/// records are streamed to as JSON lines. This, together with the
/// `indirect-census` cargo feature, is the census's whole entry point.
pub const CENSUS_PATH_VARIABLE: &str = "QCORE_INDIRECT_CENSUS";

/// Note that a new translation-unit analysis has started.
///
/// The gateway carries no unit label at the depth the refusal happens, so this
/// ordinal is the grouping key the census joins on; see the card's report.
pub fn next_unit() {
    UNIT_SEQ.fetch_add(1, Ordering::Relaxed);
}

/// The current unit ordinal, so a sibling census (card W43-INDEX-TRAVERSAL's
/// traversal stream) joins on the same grouping key.
pub(crate) fn unit_seq() -> u64 {
    UNIT_SEQ.load(Ordering::Relaxed)
}

fn emit(leaf: &IndirectLeaf) {
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(CENSUS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(leaf) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

/// One record body the declaration reader met, and what it did with it.
///
/// `read_record_body` is **total-or-reject over the whole body**: a single
/// token it will not parse — a function-pointer member's `(`, a bitfield's
/// `:`, a nested anonymous aggregate's `{` — refuses the record entirely, so
/// every member of it, including the plain scalars beside the offending one,
/// becomes unclassifiable. The census records which token did it, because that
/// is the difference between "this record is genuinely opaque" and "one member
/// of it is, and the rest were collateral".
#[derive(Clone, Debug, serde::Serialize)]
pub struct RecordBodyNote {
    pub tag: String,
    /// The first token of the body this reader refuses on, if any.
    pub refused_on: Option<String>,
    /// Tokens in the body, as a size axis.
    pub body_tokens: usize,
}

static RECORDS: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// The environment variable naming the record-body census stream.
pub const RECORD_CENSUS_PATH_VARIABLE: &str = "QCORE_INDIRECT_CENSUS_RECORDS";

/// Record one record-body reading. A no-op unless the variable names a path.
pub(crate) fn record_body(tag: &str, body: &[&str]) {
    if std::env::var_os(RECORD_CENSUS_PATH_VARIABLE).is_none() {
        return;
    }
    let note = RecordBodyNote {
        tag: tag.to_owned(),
        // The reader's own answer (card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS:
        // a function-pointer member no longer refuses the body).
        refused_on: crate::typed_field_access::record_body_refusal(body)
            .map(|text| (*text).to_owned()),
        body_tokens: body.len(),
    };
    let Ok(mut guard) = RECORDS.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(RECORD_CENSUS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&note) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

/// Everything the classifier holds that this census reads.
///
/// Borrowed rather than owned: the census must not be able to retain, mutate or
/// otherwise influence the analyzer's state.
pub(crate) struct Context<'a> {
    pub(crate) function: &'a str,
    pub(crate) formal_parameters: &'a BTreeSet<String>,
    pub(crate) typed_model: &'a TypedAccessModel,
    pub(crate) typed_types: &'a FunctionTypes,
    pub(crate) typed_bases: &'a UntrackedBases,
}

/// Record one refusal.
///
/// `texts` is the complete token text of the refused statement — the
/// classifier's own contract with its callers — `exempt` the spans the gate
/// excluded before looking, and `from` the first index the gate looked at.
/// `line` is the statement's preprocessed line.
pub(crate) struct Refusal {
    pub(crate) from: usize,
    pub(crate) assignment: Option<usize>,
    pub(crate) line: usize,
    pub(crate) site: RefusalSite,
    pub(crate) position: Position,
    /// Whether this statement's first call is contracted, supplied by the
    /// classifier from the same `ResourceContracts` the gate itself holds.
    pub(crate) call_contracted: Option<bool>,
}

pub(crate) fn record(
    context: &Context<'_>,
    texts: &[&str],
    exempt: &[(usize, usize)],
    refusal: &Refusal,
    bindings: &BodyBindings,
) {
    let &Refusal {
        from,
        assignment,
        line,
        site,
        position,
        call_contracted,
    } = refusal;
    if std::env::var_os(CENSUS_PATH_VARIABLE).is_none() {
        return;
    }
    // The gate looks at the destination side for a write and at the value side
    // for a read; the census reads the same window.
    let window: Vec<usize> = match position {
        Position::AssignmentDestination => (0..assignment.unwrap_or(texts.len())).collect(),
        _ => (from..texts.len())
            .filter(|index| {
                !exempt
                    .iter()
                    .any(|(start, end)| index >= start && index < end)
            })
            .collect(),
    };
    emit(&IndirectLeaf {
        unit_seq: UNIT_SEQ.load(Ordering::Relaxed),
        function: context.function.to_owned(),
        line,
        site,
        position,
        contains_call: texts
            .iter()
            .enumerate()
            .any(|(index, text)| is_name(text) && texts.get(index + 1) == Some(&"(")),
        call_contracted,
        tokens: texts.len(),
        chains: chains(context, texts, &window),
        texts: texts.iter().map(|text| (*text).to_owned()).collect(),
        derived_names: names_in(texts, &bindings.derived),
        owned_names: names_in(texts, &bindings.owned),
        declared: texts
            .iter()
            .filter(|text| is_name(text))
            .filter_map(|name| {
                declared_label(context, name).map(|label| ((*name).to_owned(), label))
            })
            .collect(),
    });
}

fn names_in(texts: &[&str], names: &BTreeSet<String>) -> Vec<String> {
    texts
        .iter()
        .filter(|text| names.contains(**text))
        .map(|text| (*text).to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The declaration reader's own answer for one name, as a census label.
pub(crate) fn declared_label(context: &Context<'_>, name: &str) -> Option<String> {
    let Some(label) = declared_type_label(context, name) else {
        if context.typed_model.is_enum_constant(name) {
            return Some("enum_constant".to_owned());
        }
        return context
            .typed_model
            .scalar_type_name(&[name])
            .then(|| "scalar_type_name".to_owned());
    };
    Some(match context.typed_types.scalar_element(name) {
        Some(element) => format!("{label}|element:{element:?}"),
        None => label,
    })
}

fn declared_type_label(context: &Context<'_>, name: &str) -> Option<String> {
    Some(match context.typed_types.declared_type(name)? {
        DeclaredType::Scalar => "scalar".to_owned(),
        DeclaredType::Record(tag) => format!("record:{tag}"),
        DeclaredType::RecordPointer(tag) => format!("record_pointer:{tag}"),
        DeclaredType::Opaque => match context.typed_types.census.opaque.get(name) {
            Some(cause) => format!("opaque:{cause:?}"),
            None => "opaque:Unrecorded".to_owned(),
        },
    })
}

/// Every maximal access chain in the gate's own view of the statement.
///
/// `window` is the ascending list of token indices the gate looked at; the walk
/// follows *that* order, so a chain the gate could not see is not reported and
/// an adjacency the gate's own span filtering created is reported as the gate
/// saw it.
pub(crate) fn chains(context: &Context<'_>, texts: &[&str], window: &[usize]) -> Vec<Chain> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < window.len() {
        let index = window[cursor];
        let text = texts[index];
        // A prefix `*` that is not a multiplication starts a dereference chain.
        let deref = text == "*"
            && cursor
                .checked_sub(1)
                .and_then(|previous| texts.get(window[previous]).copied())
                .is_none_or(|left| !(is_name(left) || left.bytes().all(|b| b.is_ascii_digit())));
        if deref {
            // `* f ( … )` dereferences a call's **result**, not the callee
            // name. `(*__errno_location ())` is the corpus's own instance, and
            // lumping it with `*p` for a declared pointer `p` would put a
            // shape with no base at all into a shape an arbiter would legislate
            // about bases for.
            let dereferences_call = window
                .get(cursor + 1)
                .is_some_and(|&at| is_name(texts[at]) && texts.get(at + 1) == Some(&"("));
            if dereferences_call {
                out.push(Chain {
                    base: String::new(),
                    base_kind: BaseKind::CallResult,
                    base_may_bind: true,
                    base_never_bound: false,
                    base_record_pointer: false,
                    depth: 1,
                    steps: vec![Step::Deref],
                    member_class: MemberClass::NoMember,
                    member: None,
                    tag: None,
                    base_cause: BaseTypeCause::NoBaseName,
                    tag_lost_step: None,
                    callee: window.get(cursor + 1).map(|&at| texts[at].to_owned()),
                });
                cursor += 2;
                continue;
            }
            if let Some(chain) = walk(context, texts, window, cursor + 1, true) {
                cursor = chain.1;
                out.push(chain.0);
                continue;
            }
            cursor += 1;
            continue;
        }
        if is_name(text)
            && texts.get(index + 1) != Some(&"(")
            && let Some(chain) = walk(context, texts, window, cursor, false)
        {
            if !chain.0.steps.is_empty() {
                out.push(chain.0);
            }
            cursor = chain.1;
            continue;
        }
        // A chain hanging off a closing parenthesis: a call's result, or a
        // parenthesized or cast sub-expression. The base is not a name, so no
        // declared type, `may_bind` answer or never-bound answer exists for it
        // — every one of those is reported at its fail-closed value, which is
        // what an arbiter would have to work with.
        if text == ")"
            && let Some(step) = next_step(texts, window, cursor + 1)
        {
            out.push(Chain {
                base: String::new(),
                base_kind: BaseKind::CallResult,
                base_may_bind: true,
                base_never_bound: false,
                base_record_pointer: false,
                depth: 1,
                steps: vec![step],
                member_class: MemberClass::Unresolved,
                member: None,
                tag: None,
                base_cause: BaseTypeCause::NoBaseName,
                tag_lost_step: None,
                callee: callee_before(texts, index),
            });
        }
        cursor += 1;
    }
    out
}

/// The callee spelling of the call whose `)` sits at `close`, when the token
/// run is one. Card **BASE-TYPE-READER-CENSUS**: a chain hanging off a call's
/// result is blocked by a *function return type* reader, so the census names
/// the function rather than reporting an anonymous base.
fn callee_before(texts: &[&str], close: usize) -> Option<String> {
    let mut depth = 0usize;
    let mut index = close;
    loop {
        match texts.get(index).copied() {
            Some(")") => depth += 1,
            Some("(") => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    break;
                }
            }
            None => return None,
            _ => {}
        }
        index = index.checked_sub(1)?;
    }
    let name = texts.get(index.checked_sub(1)?)?;
    is_name(name).then(|| (*name).to_owned())
}

/// The selection step at `cursor` in the gate's view, if there is one.
fn next_step(texts: &[&str], window: &[usize], cursor: usize) -> Option<Step> {
    match texts.get(*window.get(cursor)?).copied()? {
        "-" if texts.get(*window.get(cursor + 1)?) == Some(&">") => Some(Step::Arrow),
        "." => Some(Step::Dot),
        "[" => Some(Step::Subscript),
        _ => None,
    }
}

/// Walk one chain from `cursor` in `window`, returning it and the next cursor.
fn walk(
    context: &Context<'_>,
    texts: &[&str],
    window: &[usize],
    cursor: usize,
    dereferenced: bool,
) -> Option<(Chain, usize)> {
    let base_index = *window.get(cursor)?;
    let base = texts.get(base_index)?;
    if !is_name(base) {
        return None;
    }
    let mut steps = Vec::new();
    if dereferenced {
        steps.push(Step::Deref);
    }
    // The record tag the chain currently stands on, when the reader commits to
    // one. `None` ends every classification at `Unresolved`.
    let mut tag = match context.typed_types.declared_type(base) {
        Some(DeclaredType::RecordPointer(tag)) => Some(tag.clone()),
        Some(DeclaredType::Record(tag)) => Some(tag.clone()),
        _ => None,
    };
    let base_record_pointer = matches!(
        context.typed_types.declared_type(base),
        Some(DeclaredType::RecordPointer(_))
    );
    let mut member: Option<String> = None;
    // The tag the LAST selection was resolved against, which is the record whose
    // body reading decided the member's class.
    let mut selected_tag: Option<String> = None;
    let mut member_class = MemberClass::NoMember;
    // Card BASE-TYPE-READER-CENSUS: the step at which a tag the reader HAD
    // committed to was lost, which separates a base-type question from a
    // member question.
    let mut tag_lost_step: Option<usize> = None;
    let mut at = cursor + 1;
    while let Some(&index) = window.get(at) {
        match texts.get(index).copied() {
            Some("-")
                if texts.get(window.get(at + 1).copied().unwrap_or(usize::MAX)) == Some(&">") =>
            {
                let Some(&name_index) = window.get(at + 2) else {
                    break;
                };
                let Some(name) = texts.get(name_index).filter(|text| is_name(text)) else {
                    break;
                };
                steps.push(Step::Arrow);
                selected_tag = tag.clone();
                (member_class, tag) = classify(context, tag.as_deref(), name);
                if selected_tag.is_some() && tag.is_none() && tag_lost_step.is_none() {
                    tag_lost_step = Some(steps.len() - 1);
                }
                member = Some((*name).to_owned());
                at += 3;
            }
            Some(".") => {
                let Some(&name_index) = window.get(at + 1) else {
                    break;
                };
                let Some(name) = texts.get(name_index).filter(|text| is_name(text)) else {
                    break;
                };
                steps.push(Step::Dot);
                selected_tag = tag.clone();
                (member_class, tag) = classify(context, tag.as_deref(), name);
                if selected_tag.is_some() && tag.is_none() && tag_lost_step.is_none() {
                    tag_lost_step = Some(steps.len() - 1);
                }
                member = Some((*name).to_owned());
                at += 2;
            }
            Some("[") => {
                // Skip to the matching `]` in the gate's own view.
                let mut depth = 0usize;
                let mut scan = at;
                let close = loop {
                    let Some(&probe) = window.get(scan) else {
                        break None;
                    };
                    match texts.get(probe).copied() {
                        Some("[") => depth += 1,
                        Some("]") => {
                            depth -= 1;
                            if depth == 0 {
                                break Some(scan);
                            }
                        }
                        _ => {}
                    }
                    scan += 1;
                };
                let Some(close) = close else { break };
                steps.push(Step::Subscript);
                if tag.is_some() && tag_lost_step.is_none() {
                    tag_lost_step = Some(steps.len() - 1);
                }
                member = None;
                member_class = MemberClass::NoMember;
                selected_tag = None;
                tag = None;
                at = close + 1;
            }
            _ => break,
        }
    }
    let base_kind = base_kind(context, base);
    let chain = Chain {
        base: (*base).to_owned(),
        base_kind,
        base_may_bind: context.typed_types.may_bind(base),
        base_never_bound: context.typed_bases.never_bound(base),
        base_record_pointer,
        depth: steps.len(),
        steps,
        member_class,
        member,
        tag: selected_tag,
        base_cause: base_cause(context, base),
        tag_lost_step,
        callee: None,
    };
    Some((chain, at.max(cursor + 1)))
}

/// Classify one member selection, and say which tag the chain now stands on.
fn classify(
    context: &Context<'_>,
    tag: Option<&str>,
    member: &str,
) -> (MemberClass, Option<String>) {
    let Some(tag) = tag else {
        return (MemberClass::Unresolved, None);
    };
    let model = context.typed_model;
    if let Some(next) = model.record_pointer_field(tag, member) {
        return (MemberClass::RecordPointer, Some(next.to_owned()));
    }
    if model.scalar_field(tag, member) {
        return (MemberClass::Scalar, None);
    }
    if model.void_pointer_field(tag, member) {
        return (MemberClass::VoidPointer, None);
    }
    if model.single_pointer_field(tag, member) {
        return (MemberClass::SinglePointer, None);
    }
    if model.addressable_field(tag, member) {
        return (MemberClass::AggregateValued, None);
    }
    if model.borrowable_field(tag, member) {
        return (MemberClass::OtherPointer, None);
    }
    (MemberClass::Unresolved, None)
}

/// Why the base's declared type did not commit to a record tag — the question
/// residual `BASE-TYPE-READER-UNMEASURED` names (card
/// **BASE-TYPE-READER-CENSUS**).
///
/// Every answer is carried out of the reader rather than re-derived here: the
/// refusal rule and the specifier-level cause were written by
/// `FunctionTypes::read` at the return that produced them. The two `Unrecorded`
/// variants exist so that a gap in that plumbing is **published as a gap**
/// rather than silently attributed to a neighbouring cause.
fn base_cause(context: &Context<'_>, base: &str) -> BaseTypeCause {
    if !context.typed_types.is_readable() {
        return BaseTypeCause::FunctionRefused(
            context
                .typed_types
                .census
                .refusal
                .clone()
                .unwrap_or(DeclRefusal::Unrecorded),
        );
    }
    match context.typed_types.declared_type(base) {
        Some(DeclaredType::Record(_)) | Some(DeclaredType::RecordPointer(_)) => {
            BaseTypeCause::Committed
        }
        Some(DeclaredType::Scalar) => BaseTypeCause::DeclaredScalar,
        Some(DeclaredType::Opaque) => BaseTypeCause::DeclaredOpaque(
            context
                .typed_types
                .census
                .opaque
                .get(base)
                .cloned()
                .unwrap_or(OpaqueCause::Unrecorded),
        ),
        None => BaseTypeCause::NotDeclared(if context.typed_model.is_enum_constant(base) {
            UndeclaredKind::EnumConstant
        } else if context.typed_model.census_declares_at_file_scope(base) {
            UndeclaredKind::FileScopeDeclaration
        } else {
            UndeclaredKind::Unknown
        }),
    }
}

fn base_kind(context: &Context<'_>, base: &str) -> BaseKind {
    if !context.typed_types.is_readable() {
        return BaseKind::UnreadableDeclarations;
    }
    if context.formal_parameters.contains(base) {
        return BaseKind::Formal;
    }
    if context.typed_types.declared_type(base).is_some() {
        return BaseKind::AutomaticLocal;
    }
    BaseKind::OutsideFunctionDeclarations
}

// ---------------------------------------------------------------------------
// Card BASE-TYPE-READER-CENSUS (wave 37)
// ---------------------------------------------------------------------------
//
// `C-GATEWAY-INDIRECT-CENSUS-2` measured the *member* reader and found that
// 78.5% of its unresolved members were never a member question at all: the
// chain's `tag` was already absent, so the reader had not committed to the
// **base**'s declared type. Residual `BASE-TYPE-READER-UNMEASURED` records that
// the base-type reader — `TypedAccessModel::classify_specifiers`, typedef
// resolution and `read_locals`/`read_parameters` — had never been measured.
//
// The vocabulary below is that reader's own. Every variant is set at the exact
// `DeclaredType::Opaque` (or refusal) return it names, by the reader itself,
// through a note sink that is `()` with the feature off. There is therefore
// **one** reader here too: the census does not re-derive a cause it could
// disagree with, it carries out the answer the reader already gave.

/// Which rule of the total-or-reject declaration reader refused a whole
/// function, so that no name in it has a committed type.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum DeclRefusal {
    /// The parameter token slice carries one of `( ) { } . ; =` — a
    /// function-pointer parameter, a K&R definition, or a parenthesized
    /// declarator. Carries the offending token.
    ParameterListToken(String),
    /// `split_declarator` did not read a parameter group.
    ParameterDeclarator,
    /// `statement_end` found no end for a local declaration run.
    LocalStatementEnd,
    /// A local declaration split to no groups at all.
    LocalEmptyDeclaration,
    /// `readable_declarator` refused a local declarator on `{ } ( ) :` — a
    /// function-pointer local, a bitfield, a parenthesized declarator. Carries
    /// the offending token.
    LocalDeclaratorToken(String),
    /// `split_declarator` did not read the first group of a local declaration.
    LocalSplitDeclarator,
    /// `read_declarator` did not read a later group of a local declaration.
    LocalReadDeclarator,
    /// The function was refused but the note sink recorded no rule. Published
    /// rather than folded into another bucket: an unattributed refusal is a
    /// finding about this census, not evidence about the reader.
    Unrecorded,
}

/// Why a base name the declaration reader *read* has no committed record type.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum OpaqueCause {
    /// The declarator carries an array modifier.
    ArrayDeclarator,
    /// Two or more `*` over a specifier list the reader did commit to.
    PointerDepthAboveOne,
    /// Exactly one `*` over a specifier list resolving to `Scalar` — `char *`,
    /// `int *`. The reader commits to the pointee and still answers `Opaque`,
    /// because only `struct TAG *` is a `RecordPointer`.
    PointerToScalar,
    /// `volatile` or `_Atomic` in the specifier list.
    RejectingQualifier,
    /// The specifier list is empty once ignorable specifiers are removed.
    NoSpecifiers,
    /// A `union` specifier. Never admitted: a union member may overlap a
    /// pointer.
    UnionSpecifier,
    /// A bare `struct` keyword with no tag.
    BareTagKeyword,
    /// The specifier is a typedef name whose declaration **defines** the
    /// aggregate — `typedef struct foo_s { … } foo_t;`. The reader records the
    /// definition's head and `classify_specifiers` deliberately answers
    /// `Opaque` for it (card ABI-TYPEDEF-ADDRESS-OF-MEMBER kept that answer
    /// byte-identical). Carries the head, e.g. `struct flb_input_instance`.
    TypedefDefinesAggregate(String),
    /// Two disagreeing `typedef` declarations of the name; the table holds
    /// `None`.
    TypedefDisagreeing,
    /// The specifier is a single name this unit declares no `typedef` for.
    TypedefUnknown(String),
    /// The typedef chain reached `TYPEDEF_DEPTH_LIMIT`.
    TypedefDepthLimit,
    /// A specifier list of several tokens that are not all arithmetic
    /// keywords.
    UnreadableSpecifiers,
    /// A redeclaration in an inner scope with a different declared type; the
    /// reader makes the name unusable rather than keeping either type.
    ShadowedByDifferentType,
    /// The reader answered `Opaque` and the note sink recorded no rule.
    /// Published rather than folded away.
    Unrecorded,
    /// Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: a function-pointer formal
    /// (`R ( * cb ) ( P )`), read as a name of no committed type rather than
    /// refusing the function's declarations.
    FunctionPointerDeclarator,
}

/// Why a base name has no declaration in this function at all.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum UndeclaredKind {
    /// An enumeration constant this unit declares.
    EnumConstant,
    /// A name the same declaration reader reads as a **file-scope** declarator
    /// of this unit. Approximate in the refusing direction: a file-scope
    /// declaration this reader will not read is absent here and reported
    /// `Unknown`.
    FileScopeDeclaration,
    /// No declaration of this name anywhere the census can see.
    Unknown,
}

/// Why a chain's base has no record tag for its first selection to resolve
/// against — the question residual `BASE-TYPE-READER-UNMEASURED` names.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum BaseTypeCause {
    /// The reader committed: `struct TAG` or `struct TAG *`. Not a cause; a
    /// chain answering this and still ending `Unresolved` lost its tag at a
    /// *member*, which is the wave-36 card's question and not this one's.
    Committed,
    /// The chain has no base name: a call's result, or a parenthesized or cast
    /// sub-expression.
    NoBaseName,
    /// The whole function's declarations were refused.
    FunctionRefused(DeclRefusal),
    /// The declarations were read and this name is not among them.
    NotDeclared(UndeclaredKind),
    /// The reader committed to `Scalar`. A selection on a scalar is not a
    /// record question at all, and no base-type reader change makes it one.
    DeclaredScalar,
    /// The reader read the declaration and would not commit to the type.
    DeclaredOpaque(OpaqueCause),
}

/// The note sink the declaration reader writes one specifier-level cause into.
pub type OpaqueNote = Option<OpaqueCause>;

/// The note sink `FunctionTypes::read` writes its refusal and its per-name
/// `Opaque` causes into.
#[derive(Clone, Debug, Default)]
pub struct DeclCensus {
    pub refusal: Option<DeclRefusal>,
    pub opaque: std::collections::BTreeMap<String, OpaqueCause>,
}

/// Card **W43-RETURN-EXPRESSION**: the environment variable naming the
/// return-expression census stream. Read only with the `indirect-census`
/// feature on; with it unset, [`record_return`] is a no-op. Like the other
/// census streams it is written only after the parser has already decided to
/// refuse, so it can neither admit nor refuse anything.
pub const RETURN_CENSUS_PATH_VARIABLE: &str = "QCORE_RETURN_CENSUS";

static RETURNS: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// One call occurrence inside a refused return expression, as the classifier's
/// own contract environment and token stream answer for it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ReturnCall {
    pub callee: String,
    /// A declared `.qcore` contract or validated summary is in force.
    pub contracted: bool,
    /// The contract's result mode, spelled as its `Debug` text.
    pub result_mode: Option<String>,
    /// The callee has a body in this translation unit.
    pub defined_in_unit: bool,
}

/// One refused return expression: one leaf of the `unmodeled call, aggregate
/// or indirect access in return expression` family.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ReturnLeaf {
    pub unit_seq: u64,
    pub function: String,
    /// The internal preprocessed line of the `return` statement.
    pub line: usize,
    /// The complete return expression the parser delimited.
    pub tokens: Vec<String>,
    /// Every call occurrence in token order.
    pub calls: Vec<ReturnCall>,
    /// Every access chain in the expression.
    pub chains: Vec<Chain>,
}

/// Record one already-decided return-expression refusal. A no-op unless
/// [`RETURN_CENSUS_PATH_VARIABLE`] names a path.
pub(crate) fn record_return(
    context: &Context<'_>,
    line: usize,
    texts: &[&str],
    calls: Vec<ReturnCall>,
) {
    if std::env::var_os(RETURN_CENSUS_PATH_VARIABLE).is_none() {
        return;
    }
    let window = (0..texts.len()).collect::<Vec<_>>();
    let leaf = ReturnLeaf {
        unit_seq: UNIT_SEQ.load(Ordering::Relaxed),
        function: context.function.to_owned(),
        line,
        tokens: texts.iter().map(|text| (*text).to_owned()).collect(),
        calls,
        chains: chains(context, texts, &window),
    };
    let Ok(mut guard) = RETURNS.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(RETURN_CENSUS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&leaf) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

// ---------------------------------------------------------------------------
// Card W43-INDIRECT-ACCESS-GENERAL
// ---------------------------------------------------------------------------

/// The environment variable naming the statement census stream: one record per
/// statement the classifier refused, or lowered to an indirect destination
/// only an out-parameter contract can sanction. Read only with the
/// `indirect-census` feature on.
pub const STATEMENT_CENSUS_PATH_VARIABLE: &str = "QCORE_STATEMENT_CENSUS";

static STATEMENTS: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// One classified statement of the statement census.
#[derive(Clone, Debug, serde::Serialize)]
pub struct StatementRecord {
    pub unit_seq: u64,
    pub function: String,
    /// The statement's attributed internal preprocessed line.
    pub line: usize,
    /// The refusal description, or `indirect destination `*p``.
    pub outcome: String,
    pub texts: Vec<String>,
    pub derived_names: Vec<String>,
    pub owned_names: Vec<String>,
    pub declared: std::collections::BTreeMap<String, String>,
    /// [`FunctionTypes::may_bind`] per identifier of the statement.
    pub may_bind: Vec<String>,
    /// Every access chain in the whole statement, as the indirect census
    /// reads it (member classes in the record reader's own vocabulary).
    pub chains: Vec<Chain>,
}

/// Record one already-classified statement. A no-op unless
/// [`STATEMENT_CENSUS_PATH_VARIABLE`] names a path.
pub(crate) fn record_statement(
    context: &Context<'_>,
    line: usize,
    texts: &[&str],
    outcome: String,
    bindings: &BodyBindings,
) {
    if std::env::var_os(STATEMENT_CENSUS_PATH_VARIABLE).is_none() {
        return;
    }
    let record = StatementRecord {
        unit_seq: UNIT_SEQ.load(Ordering::Relaxed),
        function: context.function.to_owned(),
        line,
        outcome,
        texts: texts.iter().map(|text| (*text).to_owned()).collect(),
        derived_names: names_in(texts, &bindings.derived),
        owned_names: names_in(texts, &bindings.owned),
        declared: texts
            .iter()
            .filter(|text| is_name(text))
            .filter_map(|name| {
                declared_label(context, name).map(|label| ((*name).to_owned(), label))
            })
            .collect(),
        may_bind: texts
            .iter()
            .filter(|text| is_name(text) && context.typed_types.may_bind(text))
            .map(|text| (*text).to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        chains: chains(context, texts, &(0..texts.len()).collect::<Vec<_>>()),
    };
    let Ok(mut guard) = STATEMENTS.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(STATEMENT_CENSUS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&record) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

/// Card **W43-RESULT-ALIAS-RETURNED**: the environment variable naming the
/// derived-result summary census stream. Read only with the `indirect-census`
/// feature on; with it unset, [`record_derived`] is a no-op. It is written only
/// after the derived route has already decided, so it can neither admit nor
/// refuse anything.
pub const DERIVED_CENSUS_PATH_VARIABLE: &str = "QCORE_DERIVED_CENSUS";

static DERIVED: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

/// One decision of the derived-result route for one candidate function.
#[derive(Clone, Debug, serde::Serialize)]
pub struct DerivedLeaf {
    pub unit_seq: u64,
    pub function: String,
    /// `admitted` or the refused shape's name.
    pub outcome: String,
    /// The admitted summary's result mode, spelled as its `Debug` text.
    pub result: Option<String>,
}

/// Record one already-decided derived-result route outcome. A no-op unless
/// [`DERIVED_CENSUS_PATH_VARIABLE`] names a path.
pub(crate) fn record_derived(function: &str, outcome: &str, result: Option<String>) {
    if std::env::var_os(DERIVED_CENSUS_PATH_VARIABLE).is_none() {
        return;
    }
    let leaf = DerivedLeaf {
        unit_seq: UNIT_SEQ.load(Ordering::Relaxed),
        function: function.to_owned(),
        outcome: outcome.to_owned(),
        result,
    };
    let Ok(mut guard) = DERIVED.lock() else {
        return;
    };
    let opened = guard.get_or_insert_with(|| {
        let path = std::env::var_os(DERIVED_CENSUS_PATH_VARIABLE)?;
        File::create(path).ok().map(BufWriter::new)
    });
    let Some(writer) = opened.as_mut() else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&leaf) {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}
