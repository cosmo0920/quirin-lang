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
    pub steps: Vec<Step>,
    pub depth: usize,
    pub member_class: MemberClass,
    /// The final member's spelling, when the chain ends at one.
    pub member: Option<String>,
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
    });
}

/// Every maximal access chain in the gate's own view of the statement.
///
/// `window` is the ascending list of token indices the gate looked at; the walk
/// follows *that* order, so a chain the gate could not see is not reported and
/// an adjacency the gate's own span filtering created is reported as the gate
/// saw it.
fn chains(context: &Context<'_>, texts: &[&str], window: &[usize]) -> Vec<Chain> {
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
            });
        }
        cursor += 1;
    }
    out
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
    let mut member_class = MemberClass::NoMember;
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
                (member_class, tag) = classify(context, tag.as_deref(), name);
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
                (member_class, tag) = classify(context, tag.as_deref(), name);
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
                member = None;
                member_class = MemberClass::NoMember;
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
