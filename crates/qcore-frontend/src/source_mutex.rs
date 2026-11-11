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

//! Exact source lowering for the bounded lexical-guard mutex profile
//! `source.mutex.guard.v1`, over `CheckedMutexInvariantV1`.
//!
//! The admitted surface is deliberately small and fully enumerated below. A
//! program is either entirely inside it and lowers to a complete
//! [`MutexInvariantProgram`] that the core checker validates independently, or
//! it is refused with a precise diagnostic. There is no partially lowered
//! middle state and no optimistic fallback: constructs this profile models as
//! refusals become real `UnsupportedConstruct`/`ExtendProtection` events, so
//! the core checker issues the refusal, and constructs the profile cannot even
//! represent are refused here at `Q600`.
//!
//! ```text
//! module      ::= (error-declaration | entry)*
//! entry       ::= "fn" NAME "(" ")" "->" "unit" "{" scope "}"
//! scope       ::= "thread" "scope" NAME "{" outer* "}"
//! outer       ::= "let" NAME "=" "protected" INT ";"          -- one protected resource
//!               | "let" NAME "=" "mutex" "guards" NAME ";" -- fixed footprint, declared here
//!               | "let" NAME "=" "mutex" "rank" INT "guards" NAME ";" -- with a declared lock rank
//!               | "let" NAME "=" "fact" NAME ";"           -- a guard-scoped refinement fact
//!               | "read" NAME ";" | "write" NAME ";"       -- unguarded access
//!               | "wait" NAME ";"                          -- announced acquisition
//!               | "use" NAME ";"
//!               | "thread" "scope" NAME "{" child* "}"     -- one child task, joined here
//!               | "thread" "scope" NAME "share" NAME "under" NAME "{" child* "}"
//!                                                          -- a declared crossing
//!               | "with" "lock" NAME "as" NAME "{" inner* "}"
//! inner       ::= "open" NAME ";" | "close" NAME ";" | "close" NAME "unchanged" ";"
//!               | "grow" NAME "guards" NAME ";"           -- protection growth
//!               | ("read" | "write") NAME "with" NAME ";"
//!               | "let" NAME "=" "borrow" NAME "with" NAME ";"
//!               | "drop" NAME ";"
//!               | "assert" NAME "with" NAME ";" | "use" NAME ";"
//!               | "wait" NAME ";"
//!               | "thread" "scope" NAME "{" child* "}"
//!               | "thread" "scope" NAME "under" NAME "{" child* "}" -- guard-local
//!               | "thread" "scope" NAME "share" NAME "under" NAME "{" child* "}"
//!                                                          -- a declared crossing
//!               | "raise" NAME "::" NAME ";"               -- typed-error exit, last only
//! child       ::= any `outer` or `inner` form except a declaration and except
//!                 `drop`; a child task inherits the enclosing critical
//!                 section lexically but holds none of its authority
//! ```
//!
//! Guard names bind only inside their `with lock` body (the W6 frontend-gate
//! rule), so a guard-derived name cannot be mentioned after the block at all —
//! that is a `Q013` name-resolution refusal, not a concurrency one. A borrow
//! that is still live when the block ends is `Q615`.
//!
//! Three completions land in card T03-F, each bounded and each stated as a rule
//! rather than a convenience:
//!
//! * `close GUARD unchanged;` selects the **checked** `NoWriteOccurred`
//!   witness. It is admitted only when every statement since this guard's
//!   `open` was a read or a fact use — neither can write — which is the shape
//!   this lowering can decide; a section
//!   in that shape adds no `InvariantPredicateDeclared` premise. Any other
//!   shape is refused here (`Q600`) — never silently downgraded to the declared
//!   witness, and never assumed.
//! * `let f = fact r;` declares a refinement fact, `assert f with g;`
//!   establishes it under one guard and `use f;` relies on it. Each assertion
//!   mints a fresh model identity, so relying on a fact after its guard was
//!   released is the core's `StaleProtectedFact` (`Q616`), and relying on one
//!   never asserted is `UnknownFact`.
//! * a nested `thread scope c { … }` is one child task, created where the block
//!   opens and joined where it closes. The child receives no share and no
//!   custody: every access it makes is either mutex-mediated or refused. A
//!   child block written *inside* a critical section is refused by the core's
//!   own `JoinWhileGuardHeld` rule, and a child that names the enclosing
//!   guard is refused with `WrongGuardAffinity` (`Q612`).
//!
//! Card CT33-RANKS adds the ranked declaration `let m = mutex rank N guards r;`
//! and nothing else: `with lock` is unchanged, and the acquisition order a
//! nested section already produces is what the core's opt-in rank discipline
//! decides. So a lock-order cycle is now *writable* — two sections that nest
//! `a` inside `b` and `b` inside `a` — and is refused at check time with
//! `Q624`, while a module that ranks only some of its mutexes is refused with
//! `Q625`. Declaring no rank at all leaves every existing module byte-identical.
//! A passing rank check is not a deadlock-freedom proof; see the core module's
//! `§ Lock ranks` for exactly what it does and does not establish.
//!
//! Card T03-GUARD-JOIN adds one opt-in form on top of that last rule.
//! `thread scope c under g { … }`, written inside `g`'s own section, lowers its
//! join to the core's `GuardLocalJoin` instead of `Join`. `under` names the
//! enclosing section explicitly — outside one, or naming a different guard, it
//! is refused here — so the admission is requested at the source rather than
//! inferred from where the block happens to sit. Whether the join is *admitted*
//! is entirely the core's decision: it checks that the child was created inside
//! this same section, that the section is the only one held with its invariant
//! closed and no live borrow, and that the whole joined subtree named no mutex,
//! guard or guard-scoped fact anywhere. A plain `thread scope c { … }` inside a
//! section is unchanged and still refuses at `Q600`, so nothing already written
//! moves.
//!
//! Card CT34-GROWTH adds `grow GUARD guards RESOURCE;`, admitted only inside a
//! critical section. It names this section's own guard; the core reads the
//! generation off that guard, so a growth can never name a mutex its holder
//! does not hold. Whether the growth is *admitted* is entirely the core's
//! decision — it must sit inside an open invariant, over memory no live
//! generation protects, with the growing task holding the generation's full
//! share and being its creator — and this lowering only mints the new declared
//! predicate identity the growth moves to, because a wider protected region is
//! a different proposition. A section that grew therefore cannot use
//! `close GUARD unchanged;`: growth is a change to the region, so the decidable
//! read-only shape is gone and the checked witness is refused here rather than
//! silently downgraded. The pre-existing `ExtendProtection` refusal (`Q618`) is
//! untouched and has no source form at all; a module that never writes `grow`
//! is byte-identical to before, its declared invariant identities included.
//!
//! Because a child in this profile can hold no share and no custody, the work a
//! guard-local child can do is whatever this profile does not model: it may not
//! touch a declared resource at all. The admission is about the *join*, not
//! about giving the child authority.
//!
//! Card MUTEX-SHARE-CROSS adds the last of these opt-in forms, and it is CT46's
//! composition: `thread scope c share buf under m { … }` **declares** that a
//! `protected N` payload crosses into this child, and that the mutex generation
//! `m` is the protocol under which it may be reached. The three child-task
//! headers are told apart by one token at a fixed position — nothing, `under`
//! for the guard-local join, `share` for a declared crossing — so a child
//! without the clause lowers exactly as it did.
//!
//! The decision is deliberately split three ways and must stay split.
//! *Classification* is `qcore_core::transfer`'s: its fifth admissible row
//! (`ProtectedResource` under `ShareUnderMutex`, delivering a mutex share)
//! answers whether this kind crosses this way at all, and refuses a generation,
//! a guard or a fact at the payload position by name. The *protocol* gates are
//! this lowering's, because the judgment cannot see a generation's declared
//! footprint: the named generation must exist (absent) and must protect this
//! very payload (mismatched). And *verification* is the core's own — a child
//! that reaches the payload outside a section of that generation is refused by
//! the rule that a protected footprint has no unguarded access at all, reported
//! under the core's `Q611`. No rule of the mutex checker is re-implemented here.
//!
//! Beside them the declaration is **load-bearing**: a declaring child reaches
//! the payload it named and no other declared resource (`Frame::admits_reach`,
//! applied at every resource-resolution site). Like the rank and growth
//! disciplines this is opt-in and refuses only — a child with no clause is
//! unaffected, and a reach this rule allows still faces the core's guard rules
//! unchanged. The soundness statement is `QCore.MutexShareCross`: at every
//! point of every interleaving, an admitted reach of a protected payload names
//! a guard its own task holds over that payload's protector. Bounded to one
//! wrapper, one payload, one mutex and no nesting; a multi-task mutex trace
//! still receives no C (`MUTEX-SHARE-CROSS-EMISSION`).
//!
//! What this lowering does **not** establish: source-to-trace refinement, the
//! real C layout of the declared intervals (`DeclaredFootprintShape`), or the
//! invariant predicate itself (`InvariantPredicateDeclared`, on every section
//! that closes by declaration). The task-profile `spawn`/`join` expression
//! forms, captures, owned values and completion capsules remain refused.
use std::collections::BTreeMap;

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::prelude::concurrency::mutex_invariant::{
    AccessFootprint, AccessKind, FULL_ACCESS, InvariantWitness, LockRank, MutexId,
    MutexInvariantDiagnostic, MutexInvariantEvent, MutexInvariantProgram, ShareGrant,
    SharedResourceId, UnsupportedSynchronization,
};

/// The task the entry function itself runs as. Child tasks are the nested
/// `thread scope` blocks; the `spawn`/`join` expression forms of the scoped
/// task profile are refused, because their captures, owned values and
/// completion capsules have no representation in this model.
const ROOT_TASK: u32 = 0;

/// One protected resource declared by `let NAME = protected N;`, or by
/// `let NAME = protected TYPE;` (card `W48-QR-SCALAR-TYPES-R6`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexSourceResource {
    pub name: String,
    pub resource: SharedResourceId,
    pub bytes: u32,
    /// The payload's §G02t type for `protected TYPE`, whose size is `bytes`;
    /// `None` for a `protected N` byte region, every resource before the card.
    pub payload: Option<qcore_core::scalar_types::ScalarType>,
}

/// One mutex generation declared by `let NAME = mutex guards R;`, or by the
/// ranked spelling `let NAME = mutex rank N guards R;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexSourceGeneration {
    pub name: String,
    pub mutex: MutexId,
    pub protects: SharedResourceId,
    /// The declared lock rank, or `None` for the unranked spelling. This
    /// records what the source said; whether a module may mix the two is the
    /// core's R1 decision, not this record's.
    pub rank: Option<LockRank>,
}

/// A complete lowering of one entry function. The core checker still has to
/// validate `program`; nothing here is evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredMutexModule {
    pub entry: String,
    pub program: MutexInvariantProgram,
    /// Source span of the event at the same index. Every event has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for refusals that name no event (end-of-trace sweep).
    pub scope_span: Span,
    pub resources: Vec<MutexSourceResource>,
    pub generations: Vec<MutexSourceGeneration>,
    /// `exit` id of an `ErrorExit` event to the raised source error name.
    pub error_exits: BTreeMap<u32, String>,
    /// The builder's recorded target, exactly when a protected payload is an
    /// `isize` or `usize` (card `W48-QR-SCALAR-TYPES-R6`): its width sizes the
    /// payload, so it is a named assumption of the result.
    pub target_assumption: Option<qcore_core::scalar_types::RecordedTarget>,
}

impl LoweredMutexModule {
    /// The typed protected payloads, for the backend (card
    /// `W48-QR-SCALAR-TYPES-R6`). Empty for a module declaring only
    /// `protected N` regions.
    pub fn payloads(&self) -> qcore_core::mutex_c_emit::ProtectedPayloads {
        self.resources
            .iter()
            .filter_map(|resource| resource.payload.map(|payload| (resource.resource, payload)))
            .collect()
    }
    /// Present one core diagnostic at its own source span, under the code the
    /// core assigns it. The code is the core's; only the location is added.
    pub fn locate(&self, diagnostic: &MutexInvariantDiagnostic) -> SourceDiagnostic {
        let span = event_index(diagnostic)
            .and_then(|index| self.event_spans.get(index).copied())
            .unwrap_or(self.scope_span);
        SourceDiagnostic {
            code: diagnostic.code(),
            pass: SemanticPass::Concurrency,
            span,
            // `Display` already prefixes the code; the presentation boundary
            // adds it too, so carry the structured refusal only.
            message: format!("{diagnostic:?}"),
        }
    }
}

fn event_index(diagnostic: &MutexInvariantDiagnostic) -> Option<usize> {
    use MutexInvariantDiagnostic as D;
    match diagnostic {
        D::UnprotectedFootprint { event, .. }
        | D::UnguardedProtectedAccess { event, .. }
        | D::WrongGuardAffinity { event, .. }
        | D::InvariantOpenAtUnlock { event, .. }
        | D::InvariantOpenAtErrorExit { event, .. }
        | D::GuardBorrowEscape { event, .. }
        | D::StaleProtectedFact { event, .. }
        | D::FootprintAlreadyProtected { event, .. }
        | D::DynamicProtectedShape { event, .. }
        | D::NonquiescentMutexDestruction { event, .. }
        | D::ForgedSynchronizationWitness { event, .. }
        | D::UnsupportedSynchronizationConstruct { event, .. }
        | D::DeadMutexGeneration { event, .. }
        | D::LockRankViolation { event, .. }
        | D::ProtectionGrowthOutsideOpenInvariant { event, .. }
        | D::ProtectionGrowthInvariantUnchanged { event, .. }
        | D::DeadGuard { event, .. }
        | D::DoubleOpenInvariant { event, .. }
        | D::WriteWithClosedInvariant { event, .. }
        | D::InvalidInvariantWitness { event, .. }
        | D::UnknownTask { event, .. }
        | D::TerminalTask { event, .. }
        | D::DuplicateTask { event, .. }
        | D::UnknownJoin { event, .. }
        | D::NonquiescentJoin { event, .. }
        | D::JoinWhileGuardHeld { event, .. }
        | D::DeadResource { event, .. }
        | D::UnauthorizedAccess { event, .. }
        | D::WrongCustodian { event, .. }
        | D::NonquiescentDestruction { event, .. }
        | D::DuplicateLockCapability { event, .. }
        | D::UnknownMutex { event, .. }
        | D::IdentityNotFresh { event, .. }
        | D::UnknownBorrow { event, .. }
        | D::UnknownFact { event, .. } => Some(*event),
        D::MutexAuthorityNotConserved { event, .. }
        | D::InvalidFootprint { event }
        // R1's created-but-unranked half names its `CreateMutex`; its
        // dangling-declaration half names no event at all, and falls back to
        // the scope span like the end-of-trace sweep does.
        | D::MixedLockRankDeclaration { event, .. } => *event,
        D::DataRace { first, .. } => Some(*first),
        D::OutstandingTask { .. }
        | D::OutstandingGuard { .. }
        | D::OutstandingMutex { .. }
        | D::OutstandingWaiter { .. } => None,
    }
}

/// Select this route. Detection is syntactic and route-selecting only: it
/// grants no support, and a detected module that leaves the admitted grammar is
/// refused rather than handed to another route.
pub fn has_source_mutex(source: &SourceProgram) -> bool {
    // `mutex rank` selects this route as surely as `mutex guards` does: without
    // it, card CT33-RANKS's ranked declaration would fall through to another
    // route and be refused for the wrong reason.
    //
    // Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: never at a declaration's
    // own name, as every token-stream selector now reads (the census found no
    // declaration that spells the pair; the scope is uniform).
    let scope = crate::route_selection::SelectorScope::of(source);
    source.lexical_tokens().windows(2).any(|pair| {
        word(pair.first(), "mutex")
            && (word(pair.get(1), "guards") || word(pair.get(1), "rank"))
            && scope.reads(source, pair[0].span)
    })
}

/// Whether this module opts into card CT33-RANKS's lock-rank discipline.
///
/// Syntactic and whole-module, like [`has_source_mutex`]: it grants nothing on
/// its own, and a module it selects still has to satisfy the core's R1 (every
/// generation ranked) and R2 (strictly increasing acquisition).
fn declares_lock_rank(source: &SourceProgram) -> bool {
    source
        .lexical_tokens()
        .windows(2)
        .any(|pair| word(pair.first(), "mutex") && word(pair.get(1), "rank"))
}

fn failure(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q600",
        pass: SemanticPass::Concurrency,
        span,
        message: message.into(),
    }
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Word(w)) if w == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == expected)
}

fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(w) => Some(w),
        _ => None,
    }
}

fn integer(token: Option<&Token>) -> Option<u32> {
    match &token?.kind {
        TokenKind::Integer(text) => text.parse().ok(),
        _ => None,
    }
}

/// Statement tokens, excluding any nested block. A structured statement's
/// header ends at its first `{`.
fn header_tokens(source: &SourceProgram, span: Span) -> Vec<Token> {
    let mut tokens = source
        .lexical_tokens()
        .iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .cloned()
        .collect::<Vec<_>>();
    if let Some(brace) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        tokens.truncate(brace);
    }
    tokens
}

/// The critical section a statement is lexically inside, if any. A child task
/// written inside a section inherits this frame: that is exactly how a child
/// comes to *name* a guard it does not hold, and the core then refuses the act
/// with `WrongGuardAffinity` rather than the name being unresolvable.
#[derive(Clone, Copy)]
struct Section<'a> {
    name: &'a str,
    guard: u32,
    mutex: MutexId,
    /// The task that executed the `Lock` minting `guard`.
    owner: u32,
}

/// Card `MUTEX-SHARE-CROSS`: the crossing a child declared with
/// `thread scope NAME share PLACE under MUTEX { … }`.
///
/// Both names are resolved before the frame is built, so a frame carrying one
/// of these has already passed the judgment and both protocol gates: the
/// generation exists and it protects this very payload.
#[derive(Clone, Copy)]
struct DeclaredShare<'a> {
    resource: SharedResourceId,
    resource_name: &'a str,
    mutex_name: &'a str,
}

/// Which task's statements are being lowered, under which section, and — card
/// `MUTEX-SHARE-CROSS` — under which declared crossing, if any.
#[derive(Clone, Copy)]
struct Frame<'a> {
    task: u32,
    section: Option<Section<'a>>,
    /// `None` for every task written before this card and for every child that
    /// declares no crossing: such a frame reaches whatever is lexically in
    /// scope, exactly as before, and the core's own access rules are the only
    /// thing deciding it. `Some` opts the child into the stricter discipline —
    /// it may reach the payload it declared and nothing else.
    shared: Option<DeclaredShare<'a>>,
}

impl<'a> Frame<'a> {
    const fn root() -> Self {
        Self {
            task: ROOT_TASK,
            section: None,
            shared: None,
        }
    }

    const fn is_root_task(&self) -> bool {
        self.task == ROOT_TASK
    }

    /// Card `MUTEX-SHARE-CROSS`, the load-bearing half of the declaration: a
    /// child that declared a crossing may reach the payload it named and no
    /// other declared resource.
    ///
    /// This is a *strengthening* and it is opt-in. A frame with no declared
    /// share answers `Ok` for everything, so every module written before this
    /// card lowers byte-identically; a declaring child is held to what it
    /// declared, which is what makes the declaration mean something rather
    /// than decorate a reach the child had anyway. It never admits anything:
    /// a reach this rule allows still has to satisfy the core's own guard
    /// rules, and those are untouched.
    fn admits_reach(
        &self,
        resource: SharedResourceId,
        span: Span,
        state: &Lowering,
    ) -> Result<(), SourceDiagnostic> {
        let Some(share) = self.shared else {
            return Ok(());
        };
        if share.resource == resource {
            return Ok(());
        }
        let reached = state
            .resources
            .iter()
            .find(|declared| declared.resource == resource)
            .map(|declared| declared.name.as_str())
            .unwrap_or("that resource");
        Err(failure(
            span,
            format!(
                "this child declared the crossing `share {} under {}`, so it reaches `{}` and \
                 no other declared resource; `{}` was not shared with it",
                share.resource_name, share.mutex_name, share.resource_name, reached
            ),
        ))
    }
}

#[derive(Default)]
struct Lowering {
    /// The builder's recorded target (card `W48-QR-SCALAR-TYPES-R6`).
    target: Option<qcore_core::scalar_types::RecordedTarget>,
    events: Vec<MutexInvariantEvent>,
    spans: Vec<Span>,
    resources: Vec<MutexSourceResource>,
    generations: Vec<MutexSourceGeneration>,
    resource_of: BTreeMap<String, SharedResourceId>,
    bytes_of: BTreeMap<SharedResourceId, u32>,
    mutex_of: BTreeMap<String, MutexId>,
    /// Declared lock ranks, keyed by model generation — card `CT33-RANKS`.
    /// Empty unless at least one `mutex rank N guards R` declaration appeared,
    /// which is exactly the core's opt-in condition.
    lock_ranks: BTreeMap<MutexId, LockRank>,
    /// Whether this module declares any lock rank at all, decided once over
    /// the whole source. It gates the *nesting* admission below, so it must not
    /// depend on how far lowering has got.
    ranked: bool,
    /// The declared invariant identity each generation currently carries — card
    /// `CT34-GROWTH`. Seeded at the declaration and moved by every admitted
    /// growth, because a wider protected region is a different proposition. A
    /// `close GUARD;` names whatever its generation carries *now*.
    invariant_of: BTreeMap<MutexId, u32>,
    /// Mints those identities. It advances at a declaration and at a growth, so
    /// a module that never grows reproduces the previous
    /// `invariant == mutex` numbering exactly.
    next_invariant: u32,
    /// Declared refinement facts: source name to the protected resource the
    /// fact is about.
    fact_resource: BTreeMap<String, SharedResourceId>,
    /// The model fact identity a source fact name currently denotes. The
    /// declaration seeds it with an identity no `AssertFact` ever mints, so a
    /// use before any assertion is refused by the core as an unknown fact
    /// rather than by the grammar.
    fact_identity: BTreeMap<String, u32>,
    /// Borrow bindings of one critical section, keyed by that section's guard.
    borrows: BTreeMap<u32, BTreeMap<String, u32>>,
    /// Guards whose invariant is currently open, and whether every statement
    /// since the `open` has been a read. This is the decidable shape for the
    /// checked `NoWriteOccurred` witness; anything else leaves it `false`.
    open_regions: BTreeMap<u32, bool>,
    error_exits: BTreeMap<u32, String>,
    next_guard: u32,
    next_borrow: u32,
    next_fact: u32,
    next_task: u32,
    next_exit: u32,
}

impl Lowering {
    fn push(&mut self, span: Span, event: MutexInvariantEvent) {
        self.events.push(event);
        self.spans.push(span);
    }

    fn footprint(&self, resource: SharedResourceId) -> AccessFootprint {
        AccessFootprint::new(resource, 0, self.bytes_of[&resource])
    }

    /// Every statement that is not a read makes the enclosing open regions
    /// undecidable for the checked witness. Conservative on purpose: a child
    /// task's statements, a borrow, an assertion and an acquisition all count.
    fn invalidate_open_regions(&mut self) {
        for decidable in self.open_regions.values_mut() {
            *decidable = false;
        }
    }

    fn mint_fact(&mut self) -> u32 {
        self.next_fact += 1;
        self.next_fact
    }

    fn mint_invariant(&mut self) -> u32 {
        self.next_invariant += 1;
        self.next_invariant
    }
}

/// Lower one entry function to a checked-profile program.
///
/// The returned program still requires
/// `qcore_core::prelude::concurrency::mutex_invariant::check_mutex_invariant_program`; lowering
/// establishes nothing on its own.
pub fn lower_source_mutex(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredMutexModule, Vec<SourceDiagnostic>> {
    lower_source_mutex_for_target(source, typed, entry, None)
}

/// [`lower_source_mutex`] under the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R6`), which alone sizes a `protected isize` or
/// `protected usize` payload; with none such a declaration is refused.
pub fn lower_source_mutex_for_target(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&qcore_core::scalar_types::RecordedTarget>,
) -> Result<LoweredMutexModule, Vec<SourceDiagnostic>> {
    crate::module_imports::check_source_imports(source)?;
    lower(source, typed, entry, target).map_err(|diagnostic| vec![diagnostic])
}

fn lower(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&qcore_core::scalar_types::RecordedTarget>,
) -> Result<LoweredMutexModule, SourceDiagnostic> {
    let module_span = typed
        .declarations
        .first()
        .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
    if !source.task_expressions().is_empty() {
        return Err(failure(
            source
                .task_expressions()
                .first()
                .map_or(module_span, |expression| expression.span),
            "the scoped task profile's `spawn`/`join` expressions are outside \
             source.mutex.guard.v1; the profile refuses completion-capsule composition. Its own \
             child-task form is a nested `thread scope NAME { … }`",
        ));
    }
    let mut entry_declaration = None;
    for declaration in &source.declarations {
        match declaration.value.kind {
            DeclarationKind::ErrorType => {}
            DeclarationKind::Function if declaration.value.name == entry => {
                entry_declaration = Some(declaration);
            }
            _ => {
                return Err(failure(
                    declaration.span,
                    format!(
                        "source.mutex.guard.v1 admits one entry function and payload-free error \
                         declarations only; `{}` is outside the profile",
                        declaration.value.name
                    ),
                ));
            }
        }
    }
    let Some(declaration) = entry_declaration else {
        return Err(failure(
            module_span,
            format!("entry function `{entry}` has no source body in this module"),
        ));
    };
    if declaration.value.function_contract.is_some() {
        return Err(failure(
            declaration.span,
            "source contracts require an implemented consumer; the checked mutex profile \
             does not enforce them",
        ));
    }
    if !declaration.value.parameters.is_empty()
        || declaration
            .value
            .result_type
            .as_ref()
            .and_then(SourceResultType::executable_type)
            != Some(ExecutableResultType::Unit)
    {
        return Err(failure(
            declaration.span,
            "the checked mutex entry must be a parameter-free `-> unit` function",
        ));
    }
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| failure(declaration.span, "entry function requires a source body"))?;
    let [scope_statement] = body.statements.as_slice() else {
        return Err(failure(
            body.span,
            "the checked mutex entry body must be exactly one `thread scope` statement",
        ));
    };
    let StatementKind::ThreadScope {
        body: scope_body, ..
    } = &scope_statement.value
    else {
        return Err(failure(
            scope_statement.span,
            "the checked mutex entry body must be exactly one `thread scope` statement",
        ));
    };

    let mut state = Lowering {
        // Card CT33-RANKS: whether this module opted into the rank discipline
        // is decided once, syntactically, over the whole source, before any
        // statement is lowered. Reading it off the declarations seen so far
        // would make a nested section's admission depend on where the
        // declaration happened to sit.
        ranked: declares_lock_rank(source),
        target: target.cloned(),
        ..Lowering::default()
    };
    for statement in &scope_body.statements {
        if lower_statement(source, statement, Frame::root(), false, &mut state)?.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit belongs inside a critical section",
            ));
        }
    }
    // Scope exit destroys every generation and then every resource, in reverse
    // declaration order. Quiescence is checked by the core, not asserted here.
    let exit_span = Span {
        start: scope_body.span.end.saturating_sub(1),
        end: scope_body.span.end,
    };
    for generation in state.generations.clone().into_iter().rev() {
        state.push(
            exit_span,
            MutexInvariantEvent::DestroyMutex {
                task: ROOT_TASK,
                mutex: generation.mutex,
            },
        );
    }
    for resource in state.resources.clone().into_iter().rev() {
        state.push(
            exit_span,
            MutexInvariantEvent::Destroy {
                task: ROOT_TASK,
                resource: resource.resource,
            },
        );
    }

    let program = MutexInvariantProgram {
        initial_shares: state
            .resources
            .iter()
            .map(|resource| ShareGrant {
                task: ROOT_TASK,
                footprint: AccessFootprint::new(resource.resource, 0, resource.bytes),
                share: FULL_ACCESS,
            })
            .collect(),
        initial_custody: state
            .resources
            .iter()
            .map(|resource| (resource.resource, ROOT_TASK))
            .collect(),
        events: state.events,
        lock_ranks: state.lock_ranks,
    };
    let target_assumption = state
        .resources
        .iter()
        .any(|resource| {
            resource
                .payload
                .is_some_and(|payload| payload.is_target_width())
        })
        .then(|| target.cloned())
        .flatten();
    Ok(LoweredMutexModule {
        entry: entry.to_owned(),
        program,
        event_spans: state.spans,
        scope_span: scope_statement.span,
        resources: state.resources,
        generations: state.generations,
        error_exits: state.error_exits,
        target_assumption,
    })
}

/// Lower one statement of any admitted body: the thread scope's own, a
/// critical section's, or a child task's.
///
/// Returns the `exit` identity when the statement is the enclosing critical
/// section's typed-error exit. `last` says whether this statement ends its
/// block, which only a typed-error exit uses.
fn lower_statement(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    frame: Frame<'_>,
    last: bool,
    state: &mut Lowering,
) -> Result<Option<u32>, SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    match &statement.value {
        StatementKind::Let {
            binding: Some(binding),
        } if word(tokens.get(3), "borrow") => {
            state.invalidate_open_regions();
            lower_borrow(&tokens, span, binding, frame, state).map(|()| None)
        }
        StatementKind::Let { binding: Some(_) } => {
            state.invalidate_open_regions();
            if !frame.is_root_task() || frame.section.is_some() {
                return Err(failure(
                    span,
                    "source.mutex.guard.v1 declares protected resources, mutex generations and \
                     refinement facts at thread-scope level in the creating task only",
                ));
            }
            lower_declaration(&tokens, span, state).map(|()| None)
        }
        StatementKind::Drop { place: Some(place) } => {
            state.invalidate_open_regions();
            lower_end_borrow(span, place, frame, state).map(|()| None)
        }
        StatementKind::Raise => {
            state.invalidate_open_regions();
            lower_raise(&tokens, span, frame, last, state).map(Some)
        }
        StatementKind::WithLock { guard, body } => {
            state.invalidate_open_regions();
            // Card CT33-RANKS. A section nested inside another is admitted only
            // when this module declared a lock order for it. That is the whole
            // rule: you may nest exactly where you have said in which order,
            // and the core's R2 then decides whether this particular nesting
            // respects it. Without a declared rank there is no order to check
            // against, so the existing represented refusal stands unchanged —
            // which is what leaves every module written before this card
            // byte-identical.
            if frame.section.is_some() && !state.ranked {
                // Represented refusal: the construct becomes a real event the
                // core rejects, rather than being inexpressible here.
                state.push(
                    span,
                    MutexInvariantEvent::UnsupportedConstruct {
                        task: frame.task,
                        construct: UnsupportedSynchronization::NestedProtectingMutex,
                    },
                );
                return Ok(None);
            }
            lower_critical_section(source, span, &tokens, guard, body, frame, state).map(|()| None)
        }
        StatementKind::ThreadScope { name: child, body } => {
            state.invalidate_open_regions();
            lower_child_task(source, span, &tokens, child, body, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "open") => {
            state.invalidate_open_regions();
            let section = require_section(span, frame, "open")?;
            expect_guarded_form(&tokens, span, 3, 1, section, "`open GUARD;`")?;
            state.push(
                span,
                MutexInvariantEvent::OpenInvariant {
                    task: frame.task,
                    guard: section.guard,
                },
            );
            // Only the holder's own region is tracked: a child's `open` is
            // refused by the core, and no region may be opened for it.
            if section.owner == frame.task {
                state.open_regions.insert(section.guard, true);
            }
            Ok(None)
        }
        StatementKind::Expression if word(tokens.first(), "close") => {
            let section = require_section(span, frame, "close")?;
            let checked = if tokens.len() == 4 && word(tokens.get(2), "unchanged") {
                if !symbol(tokens.get(3), Symbol::Semicolon) || !names_guard(&tokens, 1, section) {
                    return Err(failure(
                        span,
                        "restoring an unwritten invariant is exactly `close GUARD unchanged;` for \
                         this section's own guard",
                    ));
                }
                true
            } else {
                expect_guarded_form(&tokens, span, 3, 1, section, "`close GUARD;`")?;
                false
            };
            let decidable = state.open_regions.get(&section.guard).copied();
            state.invalidate_open_regions();
            state.open_regions.remove(&section.guard);
            let witness = if checked {
                if decidable != Some(true) {
                    return Err(failure(
                        span,
                        "`close GUARD unchanged;` requires an open region this profile can decide: \
                         every statement since `open GUARD;` must be a read or a fact use. Use \
                         `close GUARD;` to \
                         restore the invariant by declaration instead",
                    ));
                }
                // Checked, not declared: the core verifies that no write
                // occurred while the invariant was open, so this section adds
                // no `InvariantPredicateDeclared` premise.
                InvariantWitness::NoWriteOccurred
            } else {
                // The predicate is declared, never proved: this is exactly
                // where `InvariantPredicateDeclared` enters the certificate. A
                // `close` names whichever identity its generation carries now —
                // the declared one, or the one a growth in this section moved
                // it to (card CT34-GROWTH).
                InvariantWitness::Declared {
                    invariant: state
                        .invariant_of
                        .get(&section.mutex)
                        .copied()
                        .unwrap_or(section.mutex),
                }
            };
            state.push(
                span,
                MutexInvariantEvent::CloseInvariant {
                    task: frame.task,
                    guard: section.guard,
                    witness,
                },
            );
            Ok(None)
        }
        StatementKind::Expression if word(tokens.first(), "grow") => {
            // A growth changes what the protected region *is*, so like a write
            // it ends every decidable read-only region: `close g unchanged;`
            // after one is refused here rather than downgraded.
            state.invalidate_open_regions();
            lower_grow(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "assert") => {
            state.invalidate_open_regions();
            lower_assert_fact(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "use") => {
            lower_use_fact(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression if word(tokens.first(), "wait") => {
            state.invalidate_open_regions();
            lower_wait(&tokens, span, frame, state).map(|()| None)
        }
        StatementKind::Expression => lower_access(&tokens, span, frame, state).map(|()| None),
        StatementKind::Defer => Err(failure(
            span,
            "deferred cleanup inside a critical section is outside source.mutex.guard.v1; the \
             invariant must be restored by an explicit `close` before the section exits",
        )),
        _ => Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's grammar",
        )),
    }
}

fn names_guard(tokens: &[Token], index: usize, section: Section<'_>) -> bool {
    name(tokens.get(index)) == Some(section.name)
}

fn require_section<'a>(
    span: Span,
    frame: Frame<'a>,
    what: &str,
) -> Result<Section<'a>, SourceDiagnostic> {
    frame.section.ok_or_else(|| {
        failure(
            span,
            format!("`{what}` names a guard and is admitted only inside a critical section"),
        )
    })
}

fn expect_guarded_form(
    tokens: &[Token],
    span: Span,
    length: usize,
    guard_index: usize,
    section: Section<'_>,
    shape: &str,
) -> Result<(), SourceDiagnostic> {
    if tokens.len() != length
        || !names_guard(tokens, guard_index, section)
        || !symbol(tokens.get(length - 1), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            format!("this statement is exactly {shape} for this section's own guard"),
        ));
    }
    Ok(())
}

/// `read NAME;` / `write NAME;` and `read NAME with GUARD;` /
/// `write NAME with GUARD;`.
fn lower_access(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let kind = if word(tokens.first(), "read") {
        AccessKind::Read
    } else if word(tokens.first(), "write") {
        AccessKind::Write
    } else {
        return Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's grammar",
        ));
    };
    if kind == AccessKind::Write {
        state.invalidate_open_regions();
    }
    let guarded = tokens.len() == 5;
    if guarded {
        let section = require_section(span, frame, "a guarded access")?;
        if !word(tokens.get(2), "with")
            || !names_guard(tokens, 3, section)
            || !symbol(tokens.get(4), Symbol::Semicolon)
        {
            return Err(failure(
                span,
                "a guarded access is exactly `read NAME with GUARD;` or \
                 `write NAME with GUARD;`",
            ));
        }
        let resource = resolve_resource(name(tokens.get(1)), span, state)?;
        frame.admits_reach(resource, span, state)?;
        let footprint = state.footprint(resource);
        state.push(
            span,
            MutexInvariantEvent::Access {
                task: frame.task,
                footprint,
                kind,
                guard: Some(section.guard),
            },
        );
        return Ok(());
    }
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(
            span,
            "an unguarded access is exactly `read NAME;` or `write NAME;`",
        ));
    }
    let resource = resolve_resource(name(tokens.get(1)), span, state)?;
    frame.admits_reach(resource, span, state)?;
    let footprint = state.footprint(resource);
    state.push(
        span,
        MutexInvariantEvent::Access {
            task: frame.task,
            footprint,
            kind,
            guard: None,
        },
    );
    Ok(())
}

/// `let NAME = borrow RESOURCE with GUARD;`
fn lower_borrow(
    tokens: &[Token],
    span: Span,
    binding: &str,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "a guard-derived borrow")?;
    if tokens.len() != 8
        || !symbol(tokens.get(2), Symbol::Equal)
        || !word(tokens.get(5), "with")
        || !names_guard(tokens, 6, section)
        || !symbol(tokens.get(7), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            "a critical section declares only `let NAME = borrow RESOURCE with GUARD;`",
        ));
    }
    let resource = resolve_resource(name(tokens.get(4)), span, state)?;
    frame.admits_reach(resource, span, state)?;
    let footprint = state.footprint(resource);
    state.next_borrow += 1;
    let borrow = state.next_borrow;
    state
        .borrows
        .entry(section.guard)
        .or_default()
        .insert(binding.to_owned(), borrow);
    state.push(
        span,
        MutexInvariantEvent::BorrowUnderGuard {
            task: frame.task,
            guard: section.guard,
            footprint,
            borrow,
        },
    );
    Ok(())
}

/// `drop NAME;` for a borrow of the acting task's own critical section.
fn lower_end_borrow(
    span: Span,
    place: &str,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "drop")?;
    if section.owner != frame.task {
        // Deliberately a grammar refusal, not a represented one: the model's
        // `EndBorrow` checks only that the acting task is live, so lowering a
        // child's `drop` of its parent's borrow would be *accepted* there.
        return Err(failure(
            span,
            "a guard-derived borrow is released by the task that took it; a child task may not \
             end its parent's borrow",
        ));
    }
    let borrow = state
        .borrows
        .get_mut(&section.guard)
        .and_then(|borrows| borrows.remove(place))
        .ok_or_else(|| {
            failure(
                span,
                format!("`{place}` is not a guard-derived borrow of this critical section"),
            )
        })?;
    state.push(
        span,
        MutexInvariantEvent::EndBorrow {
            task: frame.task,
            borrow,
        },
    );
    Ok(())
}

/// `raise ERROR::VARIANT;` — the typed-error exit of a critical section.
fn lower_raise(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    last: bool,
    state: &mut Lowering,
) -> Result<u32, SourceDiagnostic> {
    let section = require_section(span, frame, "raise")?;
    if section.owner != frame.task {
        return Err(failure(
            span,
            "a typed-error exit leaves the critical section its own task opened",
        ));
    }
    if !last {
        return Err(failure(
            span,
            "a typed-error exit must be the last statement of its critical section",
        ));
    }
    if tokens.len() != 5
        || !symbol(tokens.get(2), Symbol::ColonColon)
        || !symbol(tokens.get(4), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            "a typed-error exit is exactly `raise ERROR::VARIANT;`",
        ));
    }
    let error = format!(
        "{}::{}",
        name(tokens.get(1)).unwrap_or_default(),
        name(tokens.get(3)).unwrap_or_default()
    );
    state.next_exit += 1;
    state.error_exits.insert(state.next_exit, error);
    Ok(state.next_exit)
}

/// `assert FACT with GUARD;` — establish a guard-scoped refinement fact.
fn lower_assert_fact(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "assert")?;
    if tokens.len() != 5
        || !word(tokens.get(2), "with")
        || !names_guard(tokens, 3, section)
        || !symbol(tokens.get(4), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            "establishing a guard-scoped fact is exactly `assert FACT with GUARD;` for this \
             section's own guard",
        ));
    }
    let fact_name = name(tokens.get(1))
        .ok_or_else(|| failure(span, "a fact assertion names one declared fact"))?
        .to_owned();
    let resource = *state.fact_resource.get(&fact_name).ok_or_else(|| {
        failure(
            span,
            format!("`{fact_name}` is not a declared refinement fact"),
        )
    })?;
    let footprint = state.footprint(resource);
    // Each assertion mints a fresh identity: re-establishing a fact under a
    // later guard is a new fact, never a revival of the dropped one.
    let fact = state.mint_fact();
    state.fact_identity.insert(fact_name, fact);
    state.push(
        span,
        MutexInvariantEvent::AssertFact {
            task: frame.task,
            guard: section.guard,
            footprint,
            fact,
        },
    );
    Ok(())
}

/// `grow GUARD guards RESOURCE;` — card `CT34-GROWTH`'s protection growth.
///
/// It names this section's own guard, and the core reads the generation off it,
/// so a growth can never name a mutex its holder does not hold. Whether the
/// growth is *admitted* is entirely the core's decision (W1-W5 of that module's
/// `§ Protection growth`): this lowering only refuses shapes it cannot
/// represent, and hands everything else over as a real event. In particular a
/// growth written by a child task inside its parent's section lowers, and the
/// core refuses it with `WrongGuardAffinity` (`Q612`); a growth outside an open
/// invariant lowers, and the core refuses it with `Q626`.
fn lower_grow(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let section = require_section(span, frame, "grow")?;
    if tokens.len() != 5
        || !names_guard(tokens, 1, section)
        || !word(tokens.get(2), "guards")
        || !symbol(tokens.get(4), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            "growing a protected footprint is exactly `grow GUARD guards RESOURCE;` for this \
             section's own guard",
        ));
    }
    let resource = resolve_resource(name(tokens.get(3)), span, state)?;
    frame.admits_reach(resource, span, state)?;
    let footprint = state.footprint(resource);
    // A wider protected region is a different proposition, so the declared
    // predicate identity moves with the shape. The core requires an identity
    // this generation has never carried; minting a fresh one here is what makes
    // that requirement satisfiable from source.
    let invariant = state.mint_invariant();
    state.invariant_of.insert(section.mutex, invariant);
    state.push(
        span,
        MutexInvariantEvent::GrowProtection {
            task: frame.task,
            guard: section.guard,
            footprint,
            invariant,
        },
    );
    Ok(())
}

/// `use FACT;` — rely on a guard-scoped refinement fact.
fn lower_use_fact(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(span, "relying on a fact is exactly `use FACT;`"));
    }
    let fact_name =
        name(tokens.get(1)).ok_or_else(|| failure(span, "a fact use names one declared fact"))?;
    let fact = *state.fact_identity.get(fact_name).ok_or_else(|| {
        failure(
            span,
            format!("`{fact_name}` is not a declared refinement fact"),
        )
    })?;
    state.push(
        span,
        MutexInvariantEvent::UseFact {
            task: frame.task,
            fact,
        },
    );
    Ok(())
}

/// `wait MUTEX;` — announce an acquisition this task has not performed.
fn lower_wait(
    tokens: &[Token],
    span: Span,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(
            span,
            "announcing an acquisition is exactly `wait MUTEX;`",
        ));
    }
    let mutex_name =
        name(tokens.get(1)).ok_or_else(|| failure(span, "an announcement names one mutex"))?;
    let mutex = *state
        .mutex_of
        .get(mutex_name)
        .ok_or_else(|| failure(span, format!("`{mutex_name}` is not a declared mutex")))?;
    state.push(
        span,
        MutexInvariantEvent::RegisterWaiter {
            task: frame.task,
            mutex,
        },
    );
    Ok(())
}

/// Card `MUTEX-SHARE-CROSS`. Decide the crossing
/// `thread scope NAME share PLACE under MUTEX { … }` — CT46's composition:
/// a payload that is sendable and not freely shareable crossing a task
/// boundary **because** a verified mutex protocol protects it.
///
/// The decision is in two halves, and they are deliberately not merged.
///
/// * The **classification** half is `qcore_core::transfer`'s, consulted here
///   and not restated: may this kind cross under this mode at all, and does
///   what it delivers match what the child receives. That judgment refuses a
///   generation, a guard or a fact at the payload position by name, and it
///   refuses the payload under any other sharing mode — an `own<Buffer>`'s
///   counted read share and a disjoint write lease both name the wrong
///   protocol for it.
/// * The **protocol** half is this function's: the named generation must
///   exist (absent) and must protect this very payload (mismatched). Neither
///   is a classification question — the judgment cannot see a generation's
///   declared footprint — and neither is the core's, because the core is
///   handed a trace in which the crossing has already been decided.
///
/// The third refusal the card names, **unverified**, is neither of these: it
/// is the unedited `CheckedMutexInvariantV1` checker's own verdict on the
/// trace this lowering produces. A child that reaches the payload outside a
/// section of the declared generation is refused by `canAccessUnguarded`'s
/// rule that a protected footprint has no unguarded access at all, reported
/// under the core's own code. Nothing about that rule is re-implemented here.
fn lower_declared_share<'a>(
    span: Span,
    header: &'a [Token],
    frame: Frame<'_>,
    state: &Lowering,
) -> Result<DeclaredShare<'a>, SourceDiagnostic> {
    use qcore_core::transfer::{
        AdmittedTransfer, ShareProtocol, TransferKind, TransferMode, admits_capture,
    };

    if header.len() != 7 || !word(header.get(5), "under") {
        return Err(failure(
            span,
            "a declared crossing is exactly `thread scope NAME share PLACE under MUTEX { … }`",
        ));
    }
    // Bounded slice: one wrapper, one payload, one mutex, no nesting. A child
    // inside a declaring child is refused by name rather than admitted on an
    // untested reading of what it would inherit.
    if frame.shared.is_some() {
        return Err(failure(
            span,
            "a declared crossing does not nest: this child is already inside one",
        ));
    }
    let Some(resource_name) = name(header.get(4)) else {
        return Err(failure(
            span,
            "a declared crossing names the payload it shares",
        ));
    };
    let Some(mutex_name) = name(header.get(6)) else {
        return Err(failure(
            span,
            "a declared crossing names the mutex generation that protects the payload",
        ));
    };

    // The judgment, over what the parent holds, the mode the header asks for,
    // and what the child receives. A name that is a generation rather than a
    // payload is classified as one, so `share m under m` refuses here with the
    // judgment's own reason rather than falling through to a protocol gate
    // that would have complained about the wrong thing.
    // Classified from the payload name alone. The protocol name is a separate
    // question and must not enter here, or a generation shared under something
    // that is not a generation would be reported as an *unmodeled* form rather
    // than as the non-value it is.
    // Card `MUTEX-SHARE-CROSS-ARMS`: the lookup is performed **once**, here,
    // and the classification reads its result. Behaviour is unchanged — the
    // same map at the same key — but the refusal below is now visibly the
    // `None` of the very lookup that classified this name, rather than a second
    // lookup a reader has to reason is the same one.
    let declared_resource = state.resource_of.get(resource_name).copied();
    let place = if declared_resource.is_some() {
        TransferKind::ProtectedResource
    } else if state.mutex_of.contains_key(resource_name) {
        TransferKind::MutexGeneration
    } else {
        TransferKind::Unmodeled
    };
    match admits_capture(
        place,
        TransferMode::ShareUnderMutex,
        TransferKind::MutexShare,
    ) {
        Ok(AdmittedTransfer::ShareUnderMutex(ShareProtocol::CheckedMutex)) => {}
        // **Unreachable, and kept.** For this mode the judgment's admitting arm
        // constructs the protocol as a *literal*, so an admitted capture is the
        // pattern above for every kind there is
        // (`QCore.MutexShareCrossArms.theProtocolResidueIsEmpty`) and this
        // binder can never be bound. It stays as a fail-closed backstop on the
        // strength of that theorem, not of the suite: under the pre-card
        // pass-through judgment quoted in `qcore_core::transfer`, an
        // `own<Buffer>` arrives here admitted under `CountedReadShares` with
        // the right delivered kind, and this arm is the only refusal between
        // that value and an unprotected crossing
        // (`thePassThroughJudgmentWouldReachTheProtocolArm`).
        Ok(other) => {
            return Err(failure(
                span,
                format!(
                    "the transferability judgment admits this crossing as {other:?}, which is \
                     not a mutex-mediated share"
                ),
            ));
        }
        Err(refusal) => {
            return Err(failure(
                span,
                format!(
                    "the transferability judgment refuses this crossing: `{resource_name}` does \
                     not cross a task boundary under `share PLACE under MUTEX` ({refusal:?}); \
                     the checked-mutex route shares only a `protected N` payload, and only \
                     under the generation that protects it"
                ),
            ));
        }
    }

    // **Unreachable, and kept.** `place` is `ProtectedResource` only in the
    // branch this very lookup's `Some` selected, and
    // `QCore.MutexShareCrossArms.theLookupAlwaysHits` proves no other kind is
    // admitted above, so nothing reaches this `else`. It stays as a fail-closed
    // backstop on the strength of that theorem and of
    // `theJudgmentCannotTellTwoKindsOfOneClassApart` — which shows the arm is
    // dead by the *uniqueness* of the payload's class alone, so a second
    // mutex-bearing kind (`FB-SYSTEM-EXTENSIONS`) revives it — and never on the
    // strength of the suite, which cannot reach it to tell it from its negation
    // (residual `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`).
    let Some(resource) = declared_resource else {
        return Err(failure(
            span,
            format!(
                "`{resource_name}` is not a protected resource of this module, so there is no \
                 payload for a mutex protocol to protect"
            ),
        ));
    };
    // **Absent.** A crossing whose protocol names nothing is refused, never
    // treated as an unprotected share.
    let Some(&mutex) = state.mutex_of.get(mutex_name) else {
        return Err(failure(
            span,
            format!(
                "`{mutex_name}` is not a mutex generation of this module: a payload crosses \
                 this boundary only through a verified mutex protocol, and this one names none"
            ),
        ));
    };
    // **Mismatched.** A generation that protects some *other* resource verifies
    // perfectly well and still says nothing about this payload, which is
    // exactly the shape a reader is most likely to write by accident.
    let protects = state
        .generations
        .iter()
        .find(|generation| generation.mutex == mutex)
        .map(|generation| generation.protects);
    if protects != Some(resource) {
        let guarded = protects
            .and_then(|protected| {
                state
                    .resources
                    .iter()
                    .find(|declared| declared.resource == protected)
            })
            .map(|declared| declared.name.as_str())
            .unwrap_or("nothing this module declares");
        return Err(failure(
            span,
            format!(
                "`{mutex_name}` guards `{guarded}`, not `{resource_name}`: the protocol a \
                 crossing names must protect the payload it shares"
            ),
        ));
    }
    Ok(DeclaredShare {
        resource,
        resource_name,
        mutex_name,
    })
}

/// A nested `thread scope NAME { … }`: one child task, created where the block
/// begins and joined where it ends.
///
/// This is the whole of the admitted task composition. The child receives no
/// share and no custody, so every access it makes to a declared resource is
/// either mutex-mediated or refused for lack of authority; and because the
/// join is emitted at the block's end, a block written inside a critical
/// section is refused by the core's own `JoinWhileGuardHeld` rule rather than
/// by this lowering.
fn lower_child_task(
    source: &SourceProgram,
    span: Span,
    header: &[Token],
    child_name: &str,
    body: &SourceBlock,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if !word(header.first(), "thread") || !word(header.get(1), "scope") {
        return Err(failure(
            span,
            "a child task is exactly `thread scope NAME { … }`, or \
             `thread scope NAME under GUARD { … }` for a guard-local child",
        ));
    }
    // Card T03-GUARD-JOIN: `under GUARD` requests the guard-local join. It names
    // the enclosing section explicitly, so the admission is opted into at the
    // source rather than inferred from position; a plain `thread scope NAME`
    // inside a section keeps the core's `JoinWhileGuardHeld` refusal.
    // Card `MUTEX-SHARE-CROSS`: `share PLACE under MUTEX` declares the
    // crossing. One token at a fixed position tells the three header forms
    // apart — `share` here, `under` for the guard-local join, nothing for the
    // plain child — so no existing header is re-read and no module that does
    // not spell `share` changes at all.
    let declared_share = if word(header.get(3), "share") {
        Some(lower_declared_share(span, header, frame, state)?)
    } else {
        None
    };
    let guard_local = match header.len() {
        3 => None,
        7 if declared_share.is_some() => None,
        5 if word(header.get(3), "under") => {
            let Some(section) = frame.section else {
                return Err(failure(
                    span,
                    "`thread scope NAME under GUARD { … }` names the critical section the child \
                     is local to, so it belongs inside a `with lock` body",
                ));
            };
            if !names_guard(header, 4, section) {
                return Err(failure(
                    span,
                    "a guard-local child names its own enclosing section's guard: \
                     `thread scope NAME under GUARD { … }`",
                ));
            }
            Some(section)
        }
        _ => {
            return Err(failure(
                span,
                "a child task is exactly `thread scope NAME { … }`, or \
                 `thread scope NAME under GUARD { … }` for a guard-local child",
            ));
        }
    };
    let _ = child_name;
    state.next_task += 1;
    let child = state.next_task;
    let header_span = Span {
        start: span.start,
        end: body.span.start,
    };
    state.push(
        header_span,
        MutexInvariantEvent::Spawn {
            parent: frame.task,
            child,
        },
    );
    let child_frame = Frame {
        task: child,
        section: frame.section,
        shared: declared_share,
    };
    for statement in &body.statements {
        if lower_statement(source, statement, child_frame, false, state)?.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit belongs inside a critical section",
            ));
        }
    }
    state.push(
        Span {
            start: body.span.end.saturating_sub(1),
            end: body.span.end,
        },
        match guard_local {
            Some(section) => MutexInvariantEvent::GuardLocalJoin {
                parent: frame.task,
                child,
                guard: section.guard,
            },
            None => MutexInvariantEvent::Join {
                parent: frame.task,
                child,
            },
        },
    );
    Ok(())
}

fn lower_declaration(
    tokens: &[Token],
    span: Span,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let binding = name(tokens.get(1))
        .ok_or_else(|| failure(span, "a declaration binds one name"))?
        .to_owned();
    if !word(tokens.first(), "let") || !symbol(tokens.get(2), Symbol::Equal) {
        return Err(failure(
            span,
            "statement is outside source.mutex.guard.v1's thread-scope grammar",
        ));
    }
    if word(tokens.get(3), "protected") {
        if tokens.len() != 6 || !symbol(tokens.get(5), Symbol::Semicolon) {
            return Err(failure(
                span,
                "a shared resource declaration is exactly `let NAME = protected BYTES;`",
            ));
        }
        // Card `W48-QR-SCALAR-TYPES-R6`: `protected TYPE` declares one value
        // of a §G02t type, whose byte extent is the type's size — decided
        // here, once, and then exactly a `protected N` region's to the model.
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `protected f32`/`protected f64`
        // too, of four and eight bytes; the route has no value position, so
        // the cell is those bytes, whose float reading is IEEE's.
        let payload = name(tokens.get(4))
            .and_then(qcore_core::scalar_types::ScalarType::parse_admitting_floats);
        let bytes = if let Some(payload) = payload {
            let size = payload.size_bytes(state.target.as_ref()).map_err(|_| {
                failure(
                    span,
                    qcore_core::scalar_values::ValueRefusal::NoRecordedTarget { ty: payload }
                        .to_string(),
                )
            })?;
            u32::try_from(size).expect("a scalar is at most eight bytes")
        } else {
            integer(tokens.get(4))
                .filter(|bytes| *bytes > 0)
                .ok_or_else(|| {
                    failure(
                        span,
                        "a shared resource needs a positive decimal byte length",
                    )
                })?
        };
        let resource = state.resources.len() as SharedResourceId + 1;
        state.bytes_of.insert(resource, bytes);
        if state
            .resource_of
            .insert(binding.clone(), resource)
            .is_some()
        {
            return Err(failure(span, format!("`{binding}` is already declared")));
        }
        state.resources.push(MutexSourceResource {
            name: binding,
            resource,
            bytes,
            payload,
        });
        return Ok(());
    }
    if word(tokens.get(3), "mutex") {
        // Card `CT33-RANKS`: `mutex rank N guards R` is the ranked spelling of
        // the same declaration. Whether a rank is declared is decided here and
        // nowhere else; the module-wide "all or none" rule (R1) is the core's,
        // so a partly ranked module reaches the checker and is refused there
        // with `Q625` rather than being second-guessed by this grammar.
        let ranked = word(tokens.get(4), "rank");
        // `integer` is `u32`-valued, so a sign, an overflow or a non-literal
        // all land on the same refusal rather than being silently truncated.
        let rank: Option<LockRank> = if ranked {
            Some(integer(tokens.get(5)).ok_or_else(|| {
                failure(
                    span,
                    "a lock rank is one decimal literal in `0 ..= 4294967295`, as in \
                     `let NAME = mutex rank 1 guards RESOURCE;`",
                )
            })?)
        } else {
            None
        };
        let (length, guards_at, resource_at) = if ranked { (9, 6, 7) } else { (7, 4, 5) };
        if tokens.len() != length
            || !word(tokens.get(guards_at), "guards")
            || !symbol(tokens.get(length - 1), Symbol::Semicolon)
        {
            return Err(failure(
                span,
                "a mutex declaration is exactly `let NAME = mutex guards RESOURCE;`, or \
                 `let NAME = mutex rank N guards RESOURCE;` to declare a lock rank; the \
                 protected footprint is fixed at this declaration",
            ));
        }
        let protected = name(tokens.get(resource_at))
            .ok_or_else(|| failure(span, "a mutex declaration names one shared resource"))?;
        let resource = *state.resource_of.get(protected).ok_or_else(|| {
            failure(
                span,
                format!("`{protected}` is not a declared shared resource"),
            )
        })?;
        let mutex = state.generations.len() as MutexId + 1;
        if state.mutex_of.insert(binding.clone(), mutex).is_some() {
            return Err(failure(span, format!("`{binding}` is already declared")));
        }
        if let Some(rank) = rank {
            state.lock_ranks.insert(mutex, rank);
        }
        let footprint = state.footprint(resource);
        // The counter advances once per declaration, so in a module that never
        // grows this is the generation's own identity, exactly as before.
        let invariant = state.mint_invariant();
        state.invariant_of.insert(mutex, invariant);
        state.push(
            span,
            MutexInvariantEvent::CreateMutex {
                task: ROOT_TASK,
                mutex,
                protects: vec![footprint],
                invariant,
            },
        );
        state.generations.push(MutexSourceGeneration {
            name: binding,
            mutex,
            protects: resource,
            rank,
        });
        return Ok(());
    }
    if word(tokens.get(3), "fact") {
        if tokens.len() != 6 || !symbol(tokens.get(5), Symbol::Semicolon) {
            return Err(failure(
                span,
                "a refinement fact declaration is exactly `let NAME = fact RESOURCE;`",
            ));
        }
        let about = name(tokens.get(4))
            .ok_or_else(|| failure(span, "a fact declaration names one shared resource"))?;
        let resource = *state
            .resource_of
            .get(about)
            .ok_or_else(|| failure(span, format!("`{about}` is not a declared shared resource")))?;
        if state.fact_resource.contains_key(&binding) {
            return Err(failure(span, format!("`{binding}` is already declared")));
        }
        // A declared-but-unasserted fact denotes an identity no `AssertFact`
        // ever mints, so `use` before `assert` is refused by the core as an
        // unknown fact rather than by this grammar.
        let unasserted = state.mint_fact();
        state.fact_resource.insert(binding.clone(), resource);
        state.fact_identity.insert(binding, unasserted);
        return Ok(());
    }
    Err(failure(
        span,
        "source.mutex.guard.v1 declares only `protected` resources, `mutex guards` generations and \
         `fact` refinements at thread-scope level",
    ))
}

fn lower_critical_section(
    source: &SourceProgram,
    span: Span,
    header: &[Token],
    guard_name: &str,
    body: &SourceBlock,
    frame: Frame<'_>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    if header.len() != 5
        || !word(header.first(), "with")
        || !word(header.get(1), "lock")
        || !word(header.get(3), "as")
    {
        return Err(failure(
            span,
            "a critical section is exactly `with lock MUTEX as GUARD { … }`",
        ));
    }
    let mutex_name =
        name(header.get(2)).ok_or_else(|| failure(span, "a critical section names one mutex"))?;
    let mutex = *state
        .mutex_of
        .get(mutex_name)
        .ok_or_else(|| failure(span, format!("`{mutex_name}` is not a declared mutex")))?;
    state.next_guard += 1;
    let guard = state.next_guard;
    let header_span = Span {
        start: span.start,
        end: body.span.start,
    };
    state.push(
        header_span,
        MutexInvariantEvent::Lock {
            task: frame.task,
            mutex,
            guard,
        },
    );

    let inner = Frame {
        task: frame.task,
        section: Some(Section {
            name: guard_name,
            guard,
            mutex,
            owner: frame.task,
        }),
        // A critical section does not change which crossing its task is under:
        // a declaring child that opens a section is still held to what it
        // declared, which is the whole point — the declaration governs the
        // reaches *inside* the lock, not only the ones outside it.
        shared: frame.shared,
    };
    let mut error_exit = None;
    for (index, statement) in body.statements.iter().enumerate() {
        if error_exit.is_some() {
            return Err(failure(
                statement.span,
                "a typed-error exit ends its critical section; no statement may follow it",
            ));
        }
        let last = index + 1 == body.statements.len();
        error_exit = lower_statement(source, statement, inner, last, state)?;
    }
    state.open_regions.remove(&guard);
    state.borrows.remove(&guard);
    match error_exit {
        Some(exit) => state.push(
            body.statements
                .last()
                .map_or(body.span, |statement| statement.span),
            MutexInvariantEvent::ErrorExit {
                task: frame.task,
                guard,
                exit,
            },
        ),
        None => state.push(
            Span {
                start: body.span.end.saturating_sub(1),
                end: body.span.end,
            },
            MutexInvariantEvent::Unlock {
                task: frame.task,
                guard,
            },
        ),
    }
    Ok(())
}

fn resolve_resource(
    place: Option<&str>,
    span: Span,
    state: &Lowering,
) -> Result<SharedResourceId, SourceDiagnostic> {
    let place = place.ok_or_else(|| failure(span, "expected one shared resource name"))?;
    state
        .resource_of
        .get(place)
        .copied()
        .ok_or_else(|| failure(span, format!("`{place}` is not a declared shared resource")))
}
