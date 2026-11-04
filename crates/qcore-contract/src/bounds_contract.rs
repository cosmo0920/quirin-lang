//! Bounds contract v1: reviewed index and extent declarations (card
//! **CG16-INDEX-AND-OFFSET-EXTENTS**, contract specification C09.1, grammar
//! G05b).
//!
//! # Consumer selection
//!
//! `requires bounds_contract_v1;` is this consumer's selector, exactly as
//! `requires resource_contract_v1;` is the resource loader's (C02). A module
//! is loaded total-or-reject: every declaration must carry the selector once
//! and map completely into [`BoundsFunctionContract`], or the whole selection
//! is refused and no partial environment escapes. The two vocabularies are
//! disjoint, so the resource-v1 loader rejects a bounds module (no resource
//! selector, unknown predicates) and this loader rejects a resource module (no
//! bounds selector): neither consumer can silently drop the other's clauses
//! and publish a full-contract result.
//!
//! # Vocabulary
//!
//! ```text
//! requires bounds_contract_v1;
//! requires bounds_extent(P, COUNT, UNIT);      // P points to >= COUNT UNIT at entry
//! requires bounds_index(I, COUNT);             // 0 <= I < COUNT at entry
//! requires bounds_global_extent(G, N, UNIT);   // C object G has N UNIT (external storage)
//! ensures normal bounds_return_extent(COUNT, UNIT);  // the result points to COUNT UNIT
//! ensures normal bounds_preserves_extents();   // the call resizes, releases nothing
//! ensures normal bounds_frame_reads(P);        // CG24: reads only through P
//! ensures normal bounds_frame_writes(P);       // CG24: may write what P reaches
//! ensures normal bounds_frame_moves(P);        // CG24: may resize, move, release it
//! ensures normal bounds_return_derived(P, OFF, LEN, UNIT); // CG24: result is P + OFF, LEN UNIT
//! quant checks P, Idx #i: PREDICATE;         // QUANT-INDEX-CHECKS: every index through P
//! COUNT = a scalar parameter of the declaration | a canonical decimal literal
//! UNIT  = elements | bytes
//! ```
//!
//! # One member clause, three kinds (card NONNULL-MEMBER-UNIFIED-QUALIFIER)
//!
//! USER DECISION 2026-10-16 ("As shown above", "Migrate, one spelling", "Yes,
//! all kinds"): the null property's member facts are one clause.
//!
//! ```text
//! requires nonnull_member(TAG, FIELD);                                  // rule 14t
//! requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];    // rule 14x
//! requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];  // rule 14z
//! ```
//!
//! `P` is required with `[[scope]]` and rejected otherwise; any other
//! qualifier is rejected; the retired words `nonnull_member_stable` /
//! `nonnull_member_scope` are rejected with a diagnostic naming the new form.
//! Each kind loads to exactly the fact its retired spelling did
//! ([`BoundsClause::NonnullMember`], [`BoundsClause::NonnullMemberStable`],
//! [`BoundsClause::NonnullMemberScope`]; `QCore.NullMemberQualifier`). A
//! named premise stands beside any of them.
//!
//! # Quantified index checks (card QUANT-INDEX-CHECKS)
//!
//! `quant checks P, Idx #i: PREDICATE;` binds `#i` universally: every index
//! the function reads or writes through the pointer formal `P` satisfies
//! `PREDICATE` (comparisons `==`, `<`, `>`, `<=`, `>=` of `#i` against a bound
//! term, joined by `&&` and `||`). It is a check of the body and an obligation
//! of each caller (the actual's extent covers every admitted index), never a
//! postcondition, so it has its own introducer and no exit selector. The
//! reader is [`crate::bounds_quant`]; the decisions are
//! [`qcore_core::index_quant`]'s.
//!
//! # Frames and derived relations (card CG24-ALIAS-EXTENT-FRAME-VOCABULARY)
//!
//! A frame is **closed**: a declaration that states one states a mode for every
//! pointer parameter and every *slot* parameter (`ptr<mut, ptr<mut, u8>>`, a C
//! `T **`, admitted only in a frame), and the callee touches nothing else and
//! keeps nothing derived from a parameter beyond the call (its `Declared`
//! premise). Frames are not custody modes: `resource_borrowed` says nothing
//! about writes or reallocation, and these clauses say nothing about custody.
//! For a slot, `bounds_frame_moves` is the in-place reallocation of the object
//! the slot designates (the slot is rewritten; the base kept or moved).
//! `bounds_return_derived` states the result's relation to one base, or to
//! several (multi-base: the result is one of them); it excludes
//! `bounds_return_extent` and a base the call may move. The decisions are
//! [`qcore_core::index_extent_frames`].
//!
//! `elements` counts objects of the C pointee type of the parameter (or the
//! result, or the object's element type) as the audited C declaration states
//! it; `bytes` counts bytes. `Idx<N>` in C09.1 is conceptual notation for
//! `bounds_index`; it is not syntax, and this vocabulary does not touch the
//! closed index evaluator (`index.closed`) or `Idx + Ordered`.
//!
//! # Review
//!
//! A person states an external fact; until a person has reviewed it, it is
//! **held**. The marker reuses C06.1's semantics with this consumer's own
//! spelling, so the resource-v1 marker never reaches it implicitly:
//!
//! ```text
//! requires bounds_review_pending("REASON"[, "OWNER"][, rewrite_before_acceptance]);
//! requires bounds_review_pending(PRECONDITION, "REASON"[, "OWNER"][, …]);
//! ensures normal bounds_review_pending(POSTCONDITION, "REASON"[, "OWNER"][, …]);
//! ```
//!
//! Resolution is an explicit edit of the text after review: removing the
//! marker (or rewriting the clause when it carries
//! `rewrite_before_acceptance`). Nothing here, no loader, assistant or plugin,
//! can approve a declaration. A contract is applied as one conjunction, so one
//! held clause holds the whole declaration; the report names the clause. A
//! resolved declaration is a `Declared` assumption, never a proof of its C
//! body. Every declaration's normalised text is digested, so a changed clause
//! never reuses an earlier admission.
//!
//! # Pins (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS)
//!
//! A declaration an audit route transports from a provider crate is compared
//! with its crate's bounds lock (C06.2; `qcore_audit::bounds_selection`). One
//! whose pin is not in force is **held** by
//! [`BoundsContracts::hold_for_pin_mismatch`], exactly as a review marker holds
//! it: [`BoundsFunctionContract::held`] reads both, so it discharges nothing
//! and accuses nothing, and a re-pin after review is the human act that
//! resolves it. The hold is part of the environment digest.

use std::collections::BTreeMap;

use qcore_core::index_extent_frames::FrameMode;
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::digest::sha256_hex;

/// The selector.
pub const BOUNDS_CONTRACT_MARKER: &str = "bounds_contract_v1";
/// The vocabulary version, part of every bounds row's evidence identity.
pub const BOUNDS_CONTRACT_VERSION: &str = "qcore.bounds.contract.v1";
/// The review marker of this consumer.
pub const BOUNDS_REVIEW_MARKER: &str = "bounds_review_pending";
/// The disposition word, as in C06.1.
pub const REWRITE_BEFORE_ACCEPTANCE: &str = "rewrite_before_acceptance";

/// Every identifier this vocabulary reserves (the source-spelling census
/// checks them against the contract roots).
pub const RESERVED_SPELLINGS: [&str; 20] = [
    BOUNDS_CONTRACT_MARKER,
    BOUNDS_REVIEW_MARKER,
    "bounds_extent",
    "bounds_index",
    "bounds_global_extent",
    "bounds_return_extent",
    "bounds_preserves_extents",
    REWRITE_BEFORE_ACCEPTANCE,
    "bounds_frame_reads",
    "bounds_frame_writes",
    "bounds_frame_moves",
    "bounds_return_derived",
    // Card NULL-CANDIDATE-TRIAGE (G05e).
    "nonnull_member",
    "nonnull_result",
    // Card NULL-MEMBER-INVARIANT-SCOPE (G05f). Retired by card
    // NONNULL-MEMBER-UNIFIED-QUALIFIER (merged into `nonnull_member`, no
    // alias); still reserved, so no other vocabulary takes them.
    "nonnull_member_stable",
    "nonnull_member_scope",
    // Card NULL-TRIAGE-UNRESOLVED-PATHS (G05e).
    "nonnull_result_range",
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (G05j).
    RESULT_SET,
    BOUNDS_REVISION,
    // Card CELL-RESULT-SET-ALIGNMENT (G05j as amended).
    WRITTEN_SET,
];

/// Card CELL-GUARD-RESULT-RANGE-CHECKED: the set a `result_set` states.
pub use qcore_core::result_set::ResultSet;

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4, grammar G05j): the result-set
/// clause, `ensures normal result_set(SET);`.
pub const RESULT_SET: &str = "result_set";
/// Card CELL-GUARD-RESULT-RANGE-CHECKED: the revision marker of a C library
/// result set, `requires bounds_revision(flexible);`.
pub const BOUNDS_REVISION: &str = "bounds_revision";
/// Card CELL-RESULT-SET-ALIGNMENT (G05j as amended; USER DECISIONS
/// 2026-11-04): a slot's written set, `ensures normal written_set(SLOT[K],
/// SET) when #result == C;`.
pub const WRITTEN_SET: &str = "written_set";

const SCALAR_TYPES: [&str; 11] = [
    "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];
const POINTER_TYPES: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];
/// Card CG24: a pointer to a pointer (a C `T **`), admitted only in a frame.
const SLOT_TYPES: [&str; 1] = ["ptr<mut, ptr<mut, u8>>"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundsUnit {
    Elements,
    Bytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind {
    Scalar,
    Pointer,
    /// Card CG24: a pointer to a pointer, named only by a frame clause.
    Slot,
}

impl ParameterKind {
    /// A C pointer at the call (a pointer or a slot).
    pub fn is_pointer(self) -> bool {
        matches!(self, Self::Pointer | Self::Slot)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsParameter {
    pub name: String,
    pub kind: ParameterKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundsCount {
    Const(u128),
    /// A scalar parameter of the same declaration (its value at entry).
    Parameter(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundsClause {
    Extent {
        parameter: String,
        count: BoundsCount,
        unit: BoundsUnit,
    },
    Index {
        parameter: String,
        count: BoundsCount,
    },
    GlobalExtent {
        object: String,
        count: u128,
        unit: BoundsUnit,
    },
    ReturnExtent {
        count: BoundsCount,
        unit: BoundsUnit,
    },
    PreservesExtents,
    /// Card CG24: the callee's frame at one pointer or slot parameter.
    Frame {
        parameter: String,
        mode: FrameMode,
    },
    /// Card CG24: the result is `parameter + offset`, with `length` units.
    ReturnDerived {
        parameter: String,
        offset: BoundsCount,
        length: BoundsCount,
        unit: BoundsUnit,
    },
    /// Card QUANT-INDEX-CHECKS: every index through `parameter` satisfies
    /// `predicate` over the bound variable `#variable`.
    QuantCheck {
        parameter: String,
        variable: String,
        predicate: qcore_core::index_quant::QuantPredicate,
    },
    /// Card NULL-SAFETY-PROPERTY (C09.3, USER DECISION 2026-10-10): `quant
    /// null-deref #B;` -- the function dereferences the pointer formal `B`,
    /// which must be non-null. Read by the null-dereference property only;
    /// the bounds consumer never reads it ([`BoundsContracts::for_bounds`]).
    NullDeref {
        parameter: String,
    },
    /// Card NULL-DEREF-QTT-GROUNDING (C09.3, USER DECISION 2026-10-11, "Two
    /// levels"): `quant live #B;` -- the custody liveness part of a
    /// dereference: the identity behind `B` has not been released at the
    /// use. Custody reads resource v1, never this clause: an audit route
    /// admits it only where the function's own resource v1 declaration states
    /// it ([`crate::deref_parts::check_live_backing`]). Neither the bounds
    /// consumer ([`BoundsContracts::for_bounds`]) nor the null property reads
    /// it.
    Live {
        parameter: String,
    },
    /// Card NULL-CANDIDATE-TRIAGE (C09.3, grammar G05e, rule 14t):
    /// `requires nonnull_member(TAG, FIELD);` -- every `struct TAG`'s
    /// `FIELD` is non-null wherever it is read. A type-level declaration the
    /// host function carries; checked against every store the analysed units
    /// make (a store not proved non-null withdraws it), its zero-initialised
    /// construction a listed premise. Read by the null property only.
    NonnullMember {
        tag: String,
        field: String,
    },
    /// Card NULL-CANDIDATE-TRIAGE (rule 14u): `ensures normal
    /// nonnull_result();` -- the function's result is non-null, checked
    /// against every analysed body that defines it. Null property only.
    NonnullResult,
    /// Card NULL-MEMBER-INVARIANT-SCOPE (C09.3, grammar G05f, rule 14x):
    /// `requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];`
    /// (spelled `nonnull_member_stable(TAG, FIELD)` until card
    /// NONNULL-MEMBER-UNIFIED-QUALIFIER) -- `struct TAG`'s `FIELD`
    /// is never made NULL on an existing object: every store the analysed
    /// units make to it is proved non-null, except an initializer list,
    /// which writes a fresh object. A non-null value is then kept for the
    /// object's lifetime (the state "set" is never left). A type-level
    /// declaration its host, the record's constructor, carries. Null
    /// property only.
    NonnullMemberStable {
        tag: String,
        field: String,
    },
    /// Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): `requires
    /// nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];` (spelled
    /// `nonnull_member_scope(TAG, FIELD, P)` until card
    /// NONNULL-MEMBER-UNIFIED-QUALIFIER) -- on entry to the declared
    /// function, the `struct TAG` object its pointer parameter `P` names
    /// has a non-null `FIELD`. Every caller's actual is checked; the body
    /// reads it non-null at entry. Null property only.
    NonnullMemberScope {
        tag: String,
        field: String,
        parameter: String,
    },
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (C09.3, grammar G05e, rule 15q):
    /// `ensures normal nonnull_result_range(LO, HI);` -- the function's integer
    /// result lies in `[LO, HI]`, checked against every analysed body that
    /// defines it. The null path search reads it as a bound (it decides a
    /// comparison it excludes), never as a range the function returns. Null
    /// property only.
    NonnullResultRange {
        lo: i128,
        hi: i128,
    },
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4, grammar G05j, rule 38e;
    /// USER DECISION 2026-11-03, option C): `ensures normal result_set(SET);`
    /// -- the C library function's integer result is a member of `SET`
    /// (literals, `[a, b]`, `MIN`/`MAX` of the result type, `|` union, `&`
    /// intersection, `^` symmetric difference). A value set only: it states
    /// no custody, and is admitted only on a flexible declaration
    /// (`requires bounds_revision(flexible);`) whose one clause it is. Read
    /// where it is in force by the null property's bound of a call result and
    /// by the result-set derivation (rule 38e); the bounds consumer never
    /// reads it.
    ResultSet {
        set: qcore_core::result_set::ResultSet,
    },
    /// Card CELL-RESULT-SET-ALIGNMENT (G05j as amended; USER DECISIONS
    /// 2026-11-04: "Basically, Q1(a) and Q2(i) ...", "Write [1, MAX]", and the
    /// per-element and per-class refinements):
    /// `ensures normal written_set(SLOT[K], SET) when #result == C;` -- on a
    /// return of `C`, the `int` the call stored at `SLOT[K]` is a member of
    /// `SET`. A value set only, no custody. On a flexible C library
    /// declaration beside its `result_set` it is the C library's statement;
    /// on a pinned declaration it is read only where `pin --bounds
    /// --derive-results` derived it from the body chain (rule 38e as
    /// amended). The bounds consumer never reads it.
    WrittenSet {
        parameter: String,
        index: u32,
        set: qcore_core::result_set::ResultSet,
        class: i128,
    },
    /// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (contract specification
    /// C09.3, USER DECISION 2026-10-15, "Only nonnull_member* need it"):
    /// `requires resource_premise(NAME, "STATEMENT");` beside a
    /// `nonnull_member` clause of any kind (card NONNULL-MEMBER-UNIFIED-QUALIFIER,
    /// USER DECISION 2026-10-16 "Yes, all kinds"; before it, beside the stable
    /// and scope kinds only) of the same declaration -- the member facts that
    /// declaration states are in force only under the named premise, and every
    /// null row resting on one lists it. Admitted beside no other bounds or
    /// null clause. Changes no mode; read by the null property only.
    Premise {
        name: String,
        statement: String,
    },
}

impl BoundsClause {
    /// Card NULL-CANDIDATE-TRIAGE: a clause only the null property reads.
    #[must_use]
    pub fn is_null_clause(&self) -> bool {
        matches!(
            self,
            Self::NullDeref { .. }
                | Self::NonnullMember { .. }
                | Self::NonnullResult
                | Self::NonnullMemberStable { .. }
                | Self::NonnullMemberScope { .. }
                | Self::NonnullResultRange { .. }
                | Self::ResultSet { .. }
                | Self::WrittenSet { .. }
                | Self::Premise { .. }
        )
    }
}

impl BoundsClause {
    pub fn is_precondition(&self) -> bool {
        matches!(
            self,
            Self::Extent { .. }
                | Self::Index { .. }
                | Self::GlobalExtent { .. }
                | Self::QuantCheck { .. }
        )
    }
}

/// One review marker's note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewNote {
    pub reason: String,
    pub owner: Option<String>,
    pub rewrite_before_acceptance: bool,
}

impl ReviewNote {
    /// C06.1's category, read from the structure, never from the prose.
    pub fn category(&self) -> &'static str {
        if self.rewrite_before_acceptance {
            "must-be-rewritten-before-acceptance"
        } else {
            "acceptable-as-written-after-review"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsClauseEntry {
    pub clause: BoundsClause,
    /// The clause's own expression text, as written (without a marker).
    pub text: String,
    pub review: Option<ReviewNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsFunctionContract {
    pub name: String,
    pub module: String,
    pub parameters: Vec<BoundsParameter>,
    /// `None` for `unit`.
    pub result: Option<ParameterKind>,
    pub clauses: Vec<BoundsClauseEntry>,
    pub contract_review: Option<ReviewNote>,
    /// SHA-256 of the normalised declaration text (tokens joined by one space).
    pub text_sha256: String,
    /// Card NULL-DEREF-QTT-GROUNDING: SHA-256 of the declaration's canonical
    /// form (the signature and the sorted canonical clause set, every
    /// shorthand elaborated and agreeing overlaps merged), so two texts that
    /// mean the same carry the same digest. The pin stays over
    /// [`Self::text_sha256`], the text as written.
    pub canonical_sha256: String,
    /// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: set when an audit route found
    /// the declaration's pin not in force; it is then held.
    pub pin_hold: Option<PinHold>,
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4): `requires
    /// bounds_revision(flexible);` -- a C library result set, bound by name
    /// and declared signature at any revision (C06.2's flexible mode, under
    /// the standing assumption `libc-c-standard-abi`), never pinned. Only a
    /// declaration whose one clause is a `result_set` carries it.
    pub flexible: bool,
    /// The declared result type's spelling (`i32`, …), `None` for `unit`.
    pub result_type: Option<String>,
}

/// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: why a declaration's pin is not in
/// force (C06.2's reason spelling and the one-line account).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinHold {
    /// `no-lock`, `not-in-lock`, `contract-changed`, `source-changed` or
    /// `tip-unresolved`.
    pub reason: String,
    pub detail: String,
}

/// The reason code a pin hold carries, as resource pins name it.
pub use qcore_core::index_extent::CONTRACT_PIN_MISMATCH;

impl BoundsFunctionContract {
    /// Held: one marker anywhere holds the whole conjunction, and so does a
    /// pin not in force (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS).
    pub fn held(&self) -> bool {
        self.contract_review.is_some()
            || self.clauses.iter().any(|c| c.review.is_some())
            || self.pin_hold.is_some()
    }

    /// Every hold, named: `NAME: CLAUSE — REASON [owner OWNER] (CATEGORY)`.
    pub fn holds(&self) -> Vec<String> {
        let render = |scope: &str, note: &ReviewNote| {
            let owner = note
                .owner
                .as_ref()
                .map(|owner| format!(" owner {owner}"))
                .unwrap_or_default();
            format!(
                "{}: {scope} — {}{owner} ({}; sha256 {})",
                self.name,
                note.reason,
                note.category(),
                self.text_sha256
            )
        };
        let mut out = Vec::new();
        if let Some(note) = &self.contract_review {
            out.push(render("contract", note));
        }
        for clause in &self.clauses {
            if let Some(note) = &clause.review {
                out.push(render(&clause.text, note));
            }
        }
        if let Some(hold) = &self.pin_hold {
            out.push(format!(
                "{CONTRACT_PIN_MISMATCH}: {}: contract — pin {}: {} (re-pin after review; sha256 {})",
                self.name, hold.reason, hold.detail, self.text_sha256
            ));
        }
        out
    }

    pub fn parameter(&self, name: &str) -> Option<(usize, &BoundsParameter)> {
        self.parameters
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.name == name)
    }

    /// Card CG24: the closed frame, one mode per parameter (`None` for a
    /// scalar), or `None` when the declaration states no frame.
    pub fn frame(&self) -> Option<Vec<Option<FrameMode>>> {
        let mut modes = vec![None; self.parameters.len()];
        let mut any = false;
        for entry in &self.clauses {
            if let BoundsClause::Frame { parameter, mode } = &entry.clause
                && let Some((index, _)) = self.parameter(parameter)
            {
                modes[index] = Some(*mode);
                any = true;
            }
        }
        any.then_some(modes)
    }

    /// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the named premises the
    /// declaration's member facts rest on (`NAME`, `STATEMENT`), in clause
    /// order.
    pub fn premises(&self) -> Vec<(&str, &str)> {
        self.clauses
            .iter()
            .filter_map(|entry| match &entry.clause {
                BoundsClause::Premise { name, statement } => {
                    Some((name.as_str(), statement.as_str()))
                }
                _ => None,
            })
            .collect()
    }

    /// Card CELL-GUARD-RESULT-RANGE-CHECKED: the result set the declaration
    /// states: a `result_set`, or a `nonnull_result_range(LO, HI)` read as the
    /// interval `[LO, HI]` of the result type. `None` for any other
    /// declaration or a result type with no integer universe.
    pub fn result_values(&self) -> Option<qcore_core::result_set::ResultSet> {
        let universe = qcore_core::result_set::universe(self.result_type.as_deref()?)?;
        self.clauses.iter().find_map(|entry| match &entry.clause {
            BoundsClause::ResultSet { set } => Some(set.clone()),
            BoundsClause::NonnullResultRange { lo, hi } => Some(
                qcore_core::result_set::ResultSet::interval(universe, *lo, *hi),
            ),
            _ => None,
        })
    }

    /// Card CG24: the parameters a `bounds_return_derived` names, in order.
    pub fn derived_bases(&self) -> Vec<&str> {
        self.clauses
            .iter()
            .filter_map(|entry| match &entry.clause {
                BoundsClause::ReturnDerived { parameter, .. } => Some(parameter.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// A loaded bounds environment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoundsContracts {
    pub functions: BTreeMap<String, BoundsFunctionContract>,
    /// SHA-256 of each selected module's text, by path.
    pub sources: BTreeMap<String, String>,
    /// Declarations an analysed body refutes: withdrawn from every dependent
    /// use ([`Self::contract`]); the defining body still reads its own.
    pub refuted: std::collections::BTreeSet<String>,
}

impl BoundsContracts {
    /// Card NULL-SAFETY-PROPERTY (rule 14o): the environment the bounds
    /// consumer reads. Every `quant null-deref` clause is removed, a
    /// declaration left with no clause is removed, and a module left with no
    /// declaration leaves the identity, so a null-dereference module changes
    /// no bounds row, no bounds dependency and no bounds digest.
    #[must_use]
    pub fn for_bounds(&self) -> Self {
        let mut view = self.clone();
        let mut kept_modules = std::collections::BTreeSet::new();
        view.functions.retain(|_, contract| {
            let before = contract.clauses.len();
            // Card NULL-DEREF-QTT-GROUNDING: nor the liveness part, which
            // custody owns; card NULL-CANDIDATE-TRIAGE: nor any clause only
            // the null property reads.
            contract.clauses.retain(|entry| {
                !(entry.clause.is_null_clause()
                    || matches!(entry.clause, BoundsClause::Live { .. }))
            });
            let keep = !(before > 0 && contract.clauses.is_empty());
            if keep {
                kept_modules.insert(contract.module.clone());
            }
            keep
        });
        let null_only = self
            .functions
            .values()
            .map(|contract| contract.module.clone())
            .filter(|module| !kept_modules.contains(module))
            .collect::<std::collections::BTreeSet<_>>();
        view.sources.retain(|path, _| !null_only.contains(path));
        view
    }

    /// Card NULL-SAFETY-PROPERTY: every declaration with a `quant null-deref`
    /// clause, by name.
    pub fn null_derefs(&self) -> impl Iterator<Item = (&String, &BoundsFunctionContract)> {
        self.functions.iter().filter(|(_, contract)| {
            contract
                .clauses
                .iter()
                .any(|entry| matches!(entry.clause, BoundsClause::NullDeref { .. }))
        })
    }

    /// Card NULL-CANDIDATE-TRIAGE: every declaration with a clause only the
    /// null property reads (`quant null-deref`, `nonnull_member`,
    /// `nonnull_result`), by name.
    pub fn null_declarations(&self) -> impl Iterator<Item = (&String, &BoundsFunctionContract)> {
        self.functions
            .iter()
            .filter(|(_, contract)| contract.clauses.iter().any(|e| e.clause.is_null_clause()))
    }

    /// The environment's identity: the version and every module digest.
    pub fn digest(&self) -> String {
        let mut text = String::from(BOUNDS_CONTRACT_VERSION);
        for (path, sha) in &self.sources {
            text.push('\n');
            text.push_str(path);
            text.push(' ');
            text.push_str(sha);
        }
        for name in &self.refuted {
            text.push_str("\nrefuted ");
            text.push_str(name);
        }
        // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: a pin hold changes what the
        // environment decides, so it is part of its identity (absent when no
        // declaration is held, so earlier digests are unchanged).
        for (name, contract) in &self.functions {
            if let Some(hold) = &contract.pin_hold {
                text.push_str("\npin-held ");
                text.push_str(name);
                text.push(' ');
                text.push_str(&hold.reason);
            }
        }
        sha256_hex(text.as_bytes())
    }

    /// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: hold `name`, whose pin is not
    /// in force for `reason` (C06.2's spelling).
    ///
    /// # Errors
    ///
    /// A name the environment does not declare.
    pub fn hold_for_pin_mismatch(
        &mut self,
        name: &str,
        reason: &str,
        detail: &str,
    ) -> Result<(), String> {
        let contract = self
            .functions
            .get_mut(name)
            .ok_or_else(|| format!("no bounds declaration `{name}` to hold"))?;
        contract.pin_hold = Some(PinHold {
            reason: reason.to_owned(),
            detail: detail.to_owned(),
        });
        Ok(())
    }

    /// The declaration a caller may rely on: none for a refuted name.
    pub fn contract(&self, name: &str) -> Option<&BoundsFunctionContract> {
        if self.refuted.contains(name) {
            return None;
        }
        self.functions.get(name)
    }

    /// The declaration of a body the consumer analyses, refuted or not.
    pub fn own(&self, name: &str) -> Option<&BoundsFunctionContract> {
        self.functions.get(name)
    }
}

fn word(token: &TokenKind) -> Option<&str> {
    match token {
        TokenKind::Word(word) => Some(word),
        _ => None,
    }
}

fn tokens_of(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect())
}

fn type_kind(source: &str) -> Result<ParameterKind, String> {
    let tokens = tokens_of(source)?;
    let same = |spelling: &str| tokens_of(spelling).ok().as_ref() == Some(&tokens);
    if SCALAR_TYPES.iter().any(|ty| same(ty)) {
        Ok(ParameterKind::Scalar)
    } else if POINTER_TYPES.iter().any(|ty| same(ty)) {
        Ok(ParameterKind::Pointer)
    } else if SLOT_TYPES.iter().any(|ty| same(ty)) {
        Ok(ParameterKind::Slot)
    } else {
        Err(format!("unsupported bounds parameter type `{source}`"))
    }
}

/// Card NONNULL-MEMBER-UNIFIED-QUALIFIER: the kind a `nonnull_member` clause
/// states; `Plain` when it has no qualifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MemberQualifier {
    Plain,
    Stable,
    Scope,
}

impl MemberQualifier {
    /// The clause's one spelling of this kind.
    fn form(self) -> &'static str {
        match self {
            Self::Plain => "nonnull_member(TAG, FIELD)",
            Self::Stable => "nonnull_member(TAG, FIELD) where quant #scope [[stable]]",
            Self::Scope => "nonnull_member(TAG, FIELD, P) where quant #scope [[scope]]",
        }
    }
}

const QUALIFIER_SHAPE: &str = "nonnull-member-qualifier-shape: a member clause's qualifier is `where quant #scope [[stable]]` or `where quant #scope [[scope]]`";

/// The qualifier after `where`: exactly `quant #scope [[stable]]` or `quant
/// #scope [[scope]]`.
fn member_qualifier(tokens: &[TokenKind]) -> Result<MemberQualifier, String> {
    let [
        TokenKind::Word(quant),
        TokenKind::Symbol(Symbol::Hash),
        TokenKind::Word(scope),
        TokenKind::Symbol(Symbol::LeftBracket),
        TokenKind::Symbol(Symbol::LeftBracket),
        TokenKind::Word(word),
        TokenKind::Symbol(Symbol::RightBracket),
        TokenKind::Symbol(Symbol::RightBracket),
    ] = tokens
    else {
        return Err(QUALIFIER_SHAPE.into());
    };
    if quant != "quant" || scope != "scope" {
        return Err(QUALIFIER_SHAPE.into());
    }
    match word.as_str() {
        "stable" => Ok(MemberQualifier::Stable),
        "scope" => Ok(MemberQualifier::Scope),
        other => Err(format!(
            "nonnull-member-qualifier-unknown: `[[{other}]]` is no member qualifier; the qualifiers are `[[stable]]` and `[[scope]]`"
        )),
    }
}

/// Split `NAME ( ARGS )` into the name and its top-level comma-separated
/// argument token groups. Anything else is `None`.
fn call(tokens: &[TokenKind]) -> Option<(&str, Vec<&[TokenKind]>)> {
    let name = word(tokens.first()?)?;
    if tokens.get(1)? != &TokenKind::Symbol(Symbol::LeftParen)
        || tokens.last()? != &TokenKind::Symbol(Symbol::RightParen)
    {
        return None;
    }
    let inner = &tokens[2..tokens.len() - 1];
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, token) in inner.iter().enumerate() {
        match token {
            TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen) => depth = depth.checked_sub(1)?,
            TokenKind::Symbol(Symbol::Comma) if depth == 0 => {
                groups.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    if !inner.is_empty() {
        groups.push(&inner[start..]);
    }
    if groups.iter().any(|group| group.is_empty()) {
        return None;
    }
    Some((name, groups))
}

fn single_word(group: &[TokenKind]) -> Option<&str> {
    match group {
        [token] => word(token),
        _ => None,
    }
}

fn canonical_literal(group: &[TokenKind]) -> Option<u128> {
    match group {
        [TokenKind::Integer(digits)]
            if digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0')) =>
        {
            digits.parse().ok()
        }
        _ => None,
    }
}

/// Card NULL-TRIAGE-UNRESOLVED-PATHS: a canonical decimal literal with an
/// optional leading `-` (never `-0`), inside the 64-bit range.
fn signed_literal(group: &[TokenKind]) -> Option<i128> {
    let (negative, digits) = match group {
        [TokenKind::Symbol(Symbol::Minus), rest @ ..] => (true, rest),
        rest => (false, rest),
    };
    let value = i128::try_from(canonical_literal(digits)?).ok()?;
    if negative && value == 0 {
        return None;
    }
    let value = if negative { -value } else { value };
    (i128::from(i64::MIN)..=i128::from(u64::MAX))
        .contains(&value)
        .then_some(value)
}

fn unit(group: &[TokenKind]) -> Result<BoundsUnit, String> {
    match single_word(group) {
        Some("elements") => Ok(BoundsUnit::Elements),
        Some("bytes") => Ok(BoundsUnit::Bytes),
        _ => Err("a bounds unit is `elements` or `bytes`".into()),
    }
}

fn plain_string(token: &TokenKind) -> Result<String, String> {
    match token {
        TokenKind::StringLiteral(text)
            if !text.trim().is_empty()
                && !text.contains('\\')
                && !text.chars().any(char::is_control) =>
        {
            Ok(text.clone())
        }
        _ => Err("a review reason or owner is one plain, non-blank string".into()),
    }
}

fn review_note(groups: &[&[TokenKind]]) -> Result<ReviewNote, String> {
    let mut strings = Vec::new();
    let mut rewrite = false;
    for (index, group) in groups.iter().enumerate() {
        match group {
            [token @ TokenKind::StringLiteral(_)] if !rewrite => strings.push(plain_string(token)?),
            [TokenKind::Word(word)]
                if word == REWRITE_BEFORE_ACCEPTANCE
                    && index + 1 == groups.len()
                    && !strings.is_empty() =>
            {
                rewrite = true;
            }
            _ => return Err("malformed bounds review note".into()),
        }
    }
    match strings.as_slice() {
        [reason] => Ok(ReviewNote {
            reason: reason.clone(),
            owner: None,
            rewrite_before_acceptance: rewrite,
        }),
        [reason, owner] => Ok(ReviewNote {
            reason: reason.clone(),
            owner: Some(owner.clone()),
            rewrite_before_acceptance: rewrite,
        }),
        [] => Err("a bounds review marker gives no reason; a reason is mandatory".into()),
        _ => Err("a bounds review marker takes a reason and an optional owner".into()),
    }
}

fn render(tokens: &[TokenKind]) -> String {
    let mut out = String::new();
    for token in tokens {
        let piece = match token {
            TokenKind::Word(w) | TokenKind::Integer(w) | TokenKind::Float(w) => w.clone(),
            TokenKind::StringLiteral(s) => format!("\"{s}\""),
            TokenKind::CharLiteral(s) => format!("'{s}'"),
            TokenKind::Symbol(Symbol::LeftParen) => "(".into(),
            TokenKind::Symbol(Symbol::RightParen) => ")".into(),
            TokenKind::Symbol(Symbol::Comma) => ",".into(),
            // Card NULL-TRIAGE-UNRESOLVED-PATHS: a negative literal's sign
            // (no tracked bounds text spelled one before).
            TokenKind::Symbol(Symbol::Minus) => "-".into(),
            // Card NONNULL-MEMBER-UNIFIED-QUALIFIER: a member qualifier,
            // `where quant #scope [[stable]]` (no tracked bounds text spelled
            // `#`, `[` or `]` in a `requires` or `ensures` clause before).
            TokenKind::Symbol(Symbol::Hash) => "#".into(),
            TokenKind::Symbol(Symbol::LeftBracket) => "[".into(),
            TokenKind::Symbol(Symbol::RightBracket) => "]".into(),
            other => format!("{other:?}"),
        };
        if !out.is_empty()
            && !matches!(piece.as_str(), "(" | ")" | "," | "]")
            && !out.ends_with('(')
            && !out.ends_with(" -")
            && !out.ends_with("(-")
            && !out.ends_with('#')
            && !out.ends_with('[')
        {
            out.push(' ');
        }
        out.push_str(&piece);
    }
    out
}

struct Reader<'a> {
    parameters: &'a [BoundsParameter],
    result: Option<ParameterKind>,
}

impl crate::bounds_quant::Formals for Reader<'_> {
    fn is_scalar(&self, name: &str) -> bool {
        self.parameters
            .iter()
            .any(|p| p.name == name && p.kind == ParameterKind::Scalar)
    }
    fn is_pointer(&self, name: &str) -> bool {
        self.parameters
            .iter()
            .any(|p| p.name == name && p.kind == ParameterKind::Pointer)
    }
}

impl Reader<'_> {
    fn count(&self, group: &[TokenKind]) -> Result<BoundsCount, String> {
        if let Some(value) = canonical_literal(group) {
            return Ok(BoundsCount::Const(value));
        }
        match single_word(group) {
            Some(name)
                if self
                    .parameters
                    .iter()
                    .any(|p| p.name == name && p.kind == ParameterKind::Scalar) =>
            {
                Ok(BoundsCount::Parameter(name.to_owned()))
            }
            _ => Err("a bounds count is a canonical decimal literal or a scalar parameter".into()),
        }
    }

    fn parameter(&self, group: &[TokenKind], kind: ParameterKind) -> Result<String, String> {
        match single_word(group) {
            Some(name)
                if self
                    .parameters
                    .iter()
                    .any(|p| p.name == name && p.kind == kind) =>
            {
                Ok(name.to_owned())
            }
            _ => Err(format!(
                "a bounds clause names a {} parameter of its declaration",
                match kind {
                    ParameterKind::Scalar => "scalar",
                    ParameterKind::Pointer => "pointer",
                    ParameterKind::Slot => "slot",
                }
            )),
        }
    }

    /// Card NONNULL-MEMBER-UNIFIED-QUALIFIER (C09.3, grammar G05e/G05f, rules
    /// 14t, 14x, 14z; USER DECISION 2026-10-16): a clause headed
    /// `nonnull_member` in one of its three kinds, or a retired word. `None`
    /// for any other head. The kinds load to exactly the facts of the three
    /// spellings this replaces (`QCore.NullMemberQualifier`).
    fn member_clause(
        &self,
        requires: bool,
        tokens: &[TokenKind],
    ) -> Result<Option<BoundsClause>, String> {
        match tokens.first() {
            Some(TokenKind::Word(w)) if w == "nonnull_member_stable" => {
                return Err(format!(
                    "nonnull-member-retired-spelling: `nonnull_member_stable` was merged into `nonnull_member` (no alias); write `requires {};`",
                    MemberQualifier::Stable.form()
                ));
            }
            Some(TokenKind::Word(w)) if w == "nonnull_member_scope" => {
                return Err(format!(
                    "nonnull-member-retired-spelling: `nonnull_member_scope` was merged into `nonnull_member` (no alias); write `requires {};`",
                    MemberQualifier::Scope.form()
                ));
            }
            Some(TokenKind::Word(w)) if w == "nonnull_member" => {}
            _ => return Ok(None),
        }
        // The qualifier follows the argument list, after a `where` outside
        // every parenthesis.
        let mut depth = 0usize;
        let mut split = None;
        for (index, token) in tokens.iter().enumerate() {
            match token {
                TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
                TokenKind::Symbol(Symbol::RightParen) => depth = depth.saturating_sub(1),
                TokenKind::Word(w) if depth == 0 && w == "where" => {
                    split = Some(index);
                    break;
                }
                _ => {}
            }
        }
        let (head, qualifier) = match split {
            Some(index) => (&tokens[..index], member_qualifier(&tokens[index + 1..])?),
            None => (tokens, MemberQualifier::Plain),
        };
        if !requires {
            return Err(
                "nonnull-member-requires: a `nonnull_member` clause is a precondition (`requires`), never an `ensures`"
                    .into(),
            );
        }
        let shape = || {
            format!(
                "nonnull-member-shape: write `{}`, `{}` or `{}`, with TAG a C struct tag (never a parameter of the declaration) and FIELD a member",
                MemberQualifier::Plain.form(),
                MemberQualifier::Stable.form(),
                MemberQualifier::Scope.form()
            )
        };
        let (_, groups) = call(head).ok_or_else(shape)?;
        let tag_field =
            |tag: &[TokenKind], field: &[TokenKind]| -> Result<(String, String), String> {
                let tag = single_word(tag)
                    .filter(|w| !self.parameters.iter().any(|p| p.name == *w))
                    .ok_or_else(shape)?;
                let field = single_word(field).ok_or_else(shape)?;
                Ok((tag.to_owned(), field.to_owned()))
            };
        match (groups.as_slice(), qualifier) {
            ([tag, field], MemberQualifier::Plain) => {
                let (tag, field) = tag_field(tag, field)?;
                Ok(Some(BoundsClause::NonnullMember { tag, field }))
            }
            ([tag, field], MemberQualifier::Stable) => {
                let (tag, field) = tag_field(tag, field)?;
                Ok(Some(BoundsClause::NonnullMemberStable { tag, field }))
            }
            ([tag, field, p], MemberQualifier::Scope) => {
                let (tag, field) = tag_field(tag, field)?;
                let parameter = self.parameter(p, ParameterKind::Pointer).map_err(|_| {
                    format!(
                        "nonnull-member-scope-parameter: `{}`'s P is a pointer parameter of the declaration",
                        MemberQualifier::Scope.form()
                    )
                })?;
                Ok(Some(BoundsClause::NonnullMemberScope {
                    tag,
                    field,
                    parameter,
                }))
            }
            ([_, _], MemberQualifier::Scope) => Err(format!(
                "nonnull-member-scope-needs-parameter: `[[scope]]` states the member of a pointer parameter P; write `{}`",
                MemberQualifier::Scope.form()
            )),
            ([_, _, _], MemberQualifier::Plain | MemberQualifier::Stable) => Err(format!(
                "nonnull-member-parameter-needs-scope: a third argument P is admitted only with `where quant #scope [[scope]]`; write `{}`",
                MemberQualifier::Scope.form()
            )),
            _ => Err(shape()),
        }
    }

    fn clause(&self, requires: bool, tokens: &[TokenKind]) -> Result<BoundsClause, String> {
        // Card NONNULL-MEMBER-UNIFIED-QUALIFIER: the member clause, in its
        // three kinds, and the retired words.
        if let Some(clause) = self.member_clause(requires, tokens)? {
            return Ok(clause);
        }
        let (name, groups) = call(tokens).ok_or("malformed bounds clause")?;
        match (requires, name, groups.as_slice()) {
            (true, "bounds_extent", [p, n, u]) => Ok(BoundsClause::Extent {
                parameter: self.parameter(p, ParameterKind::Pointer)?,
                count: self.count(n)?,
                unit: unit(u)?,
            }),
            (true, "bounds_index", [i, n]) => Ok(BoundsClause::Index {
                parameter: self.parameter(i, ParameterKind::Scalar)?,
                count: self.count(n)?,
            }),
            (true, "bounds_global_extent", [g, n, u]) => Ok(BoundsClause::GlobalExtent {
                object: single_word(g)
                    .filter(|g| !self.parameters.iter().any(|p| p.name == *g))
                    .ok_or("a global extent names a C object, not a parameter")?
                    .to_owned(),
                count: canonical_literal(n).ok_or("a global extent's count is a literal")?,
                unit: unit(u)?,
            }),
            (false, "bounds_return_extent", [n, u]) => {
                if self.result != Some(ParameterKind::Pointer) {
                    return Err("a return extent needs a pointer result".into());
                }
                Ok(BoundsClause::ReturnExtent {
                    count: self.count(n)?,
                    unit: unit(u)?,
                })
            }
            (false, "bounds_preserves_extents", []) => Ok(BoundsClause::PreservesExtents),
            // Card NULL-CANDIDATE-TRIAGE (G05e): the declared non-null result.
            (false, "nonnull_result", []) => {
                if self.result != Some(ParameterKind::Pointer) {
                    return Err(
                        "nonnull-result-pointer: a non-null result needs a pointer result".into(),
                    );
                }
                Ok(BoundsClause::NonnullResult)
            }
            // Card NULL-TRIAGE-UNRESOLVED-PATHS (G05e): an integer result's
            // range, two canonical (optionally negative) decimal literals.
            (false, "nonnull_result_range", [lo, hi]) => {
                if self.result != Some(ParameterKind::Scalar) {
                    return Err(
                        "nonnull-result-range-scalar: a result range needs a scalar result".into(),
                    );
                }
                let (Some(lo), Some(hi)) = (signed_literal(lo), signed_literal(hi)) else {
                    return Err(
                        "nonnull-result-range-shape: `nonnull_result_range(LO, HI)` takes two canonical decimal literals"
                            .into(),
                    );
                };
                if lo > hi {
                    return Err("nonnull-result-range-empty: LO exceeds HI".into());
                }
                Ok(BoundsClause::NonnullResultRange { lo, hi })
            }
            (false, "bounds_frame_reads" | "bounds_frame_writes" | "bounds_frame_moves", [p]) => {
                let parameter = single_word(p)
                    .filter(|name| {
                        self.parameters
                            .iter()
                            .any(|q| q.name == *name && q.kind.is_pointer())
                    })
                    .ok_or("a frame clause names a pointer or slot parameter")?;
                Ok(BoundsClause::Frame {
                    parameter: parameter.to_owned(),
                    mode: match name {
                        "bounds_frame_reads" => FrameMode::Reads,
                        "bounds_frame_writes" => FrameMode::Writes,
                        _ => FrameMode::Moves,
                    },
                })
            }
            (false, "bounds_return_derived", [p, off, len, u]) => {
                if self.result != Some(ParameterKind::Pointer) {
                    return Err("a derived result needs a pointer result".into());
                }
                Ok(BoundsClause::ReturnDerived {
                    parameter: self.parameter(p, ParameterKind::Pointer)?,
                    offset: self.count(off)?,
                    length: self.count(len)?,
                    unit: unit(u)?,
                })
            }
            _ => Err(format!(
                "unsupported bounds clause `{}` under `{}`",
                render(tokens),
                if requires {
                    "requires"
                } else {
                    "ensures normal"
                }
            )),
        }
    }
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (G05j): read `result_set(SET)` in
/// the result type's universe. `None` when the clause has another head.
fn result_set_clause(
    tokens: &[TokenKind],
    result_type: Option<&str>,
) -> Option<Result<qcore_core::result_set::ResultSet, String>> {
    use qcore_core::result_set::universe;
    if !matches!(tokens.first(), Some(TokenKind::Word(w)) if w == RESULT_SET) {
        return None;
    }
    let shape = |detail: &str| {
        format!(
            "result-set-shape: {detail}; write `result_set(SET)` with literals, `[a, b]`, `MIN`, `MAX`, `|` (union), `^` (symmetric difference), `&` (intersection) and parentheses"
        )
    };
    Some((|| {
        let Some(universe) = result_type.and_then(universe) else {
            return Err(
                "result-set-integer: a result set needs an integer result (`i32`, `i64`, `usize`, …)"
                    .to_owned(),
            );
        };
        if tokens.get(1) != Some(&TokenKind::Symbol(Symbol::LeftParen))
            || tokens.last() != Some(&TokenKind::Symbol(Symbol::RightParen))
            || tokens.len() < 4
        {
            return Err(shape("the clause is `result_set(SET)`"));
        }
        set_of_tokens(&tokens[2..tokens.len() - 1], universe)
    })())
}

/// Card CELL-RESULT-SET-ALIGNMENT (G05j as amended; USER DECISIONS
/// 2026-11-04, Q1 (a) with its refinements): read `written_set(SLOT[K], SET)
/// when #result == C` (`SLOT` alone is `SLOT[0]`). `None` for another head.
fn written_set_clause(
    tokens: &[TokenKind],
    parameters: &[BoundsParameter],
) -> Option<Result<(String, u32, qcore_core::result_set::ResultSet, i128), String>> {
    if !matches!(tokens.first(), Some(TokenKind::Word(w)) if w == WRITTEN_SET) {
        return None;
    }
    let shape = || {
        format!(
            "written-set-shape: write `{WRITTEN_SET}(SLOT[K], SET) when #result == C` over a pointer parameter"
        )
    };
    Some((|| {
        if tokens.get(1) != Some(&TokenKind::Symbol(Symbol::LeftParen)) {
            return Err(shape());
        }
        let mut depth = 0usize;
        let mut close = None;
        for (at, token) in tokens.iter().enumerate().skip(1) {
            match token {
                TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
                TokenKind::Symbol(Symbol::RightParen) => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(at);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close.ok_or_else(shape)?;
        let Some(TokenKind::Word(parameter)) = tokens.get(2) else {
            return Err(shape());
        };
        if !parameters
            .iter()
            .any(|p| &p.name == parameter && p.kind == ParameterKind::Pointer)
        {
            return Err(format!(
                "written-set-slot: `{parameter}` is no pointer parameter of the declaration"
            ));
        }
        let (index, comma) = match &tokens[3..] {
            [
                TokenKind::Symbol(Symbol::LeftBracket),
                TokenKind::Integer(digits),
                TokenKind::Symbol(Symbol::RightBracket),
                TokenKind::Symbol(Symbol::Comma),
                ..,
            ] if digits == "0" || !digits.starts_with('0') => {
                (digits.parse::<u32>().map_err(|_| shape())?, 6)
            }
            [TokenKind::Symbol(Symbol::Comma), ..] => (0, 3),
            _ => return Err(shape()),
        };
        if comma + 1 >= close {
            return Err(shape());
        }
        let int = (i128::from(i32::MIN), i128::from(i32::MAX));
        let set = set_of_tokens(&tokens[comma + 1..close], int)?;
        let class = match &tokens[close + 1..] {
            [
                TokenKind::Word(when),
                TokenKind::Symbol(Symbol::Hash),
                TokenKind::Word(result),
                TokenKind::Symbol(Symbol::EqualEqual),
                value @ ..,
            ] if when == "when" && result == "result" => match value {
                [TokenKind::Integer(digits)] => canonical_class(digits, false),
                [TokenKind::Symbol(Symbol::Minus), TokenKind::Integer(digits)] => {
                    canonical_class(digits, true)
                }
                _ => None,
            },
            _ => None,
        }
        .ok_or_else(|| {
            format!(
                "written-set-class: a written set states its success class, `when #result == C`"
            )
        })?;
        Ok((parameter.clone(), index, set, class))
    })())
}

fn canonical_class(digits: &str, negative: bool) -> Option<i128> {
    let spelled = if negative {
        format!("-{digits}")
    } else {
        digits.to_owned()
    };
    qcore_core::result_set::canonical_literal(&spelled)
}

/// The set a G05j token run denotes in `universe` (shared with the cell
/// clauses' `live_set(SET)` / `absent_set(SET)`, card
/// CELL-RESULT-SET-ALIGNMENT).
///
/// # Errors
///
/// `result-set-shape` / `result-set-empty` / `result-set-outside-type`.
pub(crate) fn set_of_tokens(
    inner: &[TokenKind],
    universe: qcore_core::result_set::Interval,
) -> Result<qcore_core::result_set::ResultSet, String> {
    use qcore_core::result_set::{SetToken, canonical_literal, parse};
    let shape = |detail: &str| {
        format!(
            "result-set-shape: {detail}; write literals, `[a, b]`, `MIN`, `MAX`, `|` (union), `^` (symmetric difference), `&` (intersection) and parentheses"
        )
    };
    {
        let mut set_tokens = Vec::new();
        let mut index = 0;
        while index < inner.len() {
            let token = match &inner[index] {
                TokenKind::Symbol(Symbol::Minus) => match inner.get(index + 1) {
                    Some(TokenKind::Integer(digits)) => {
                        index += 1;
                        SetToken::Literal(
                            canonical_literal(&format!("-{digits}"))
                                .ok_or_else(|| shape("a literal is a canonical decimal"))?,
                        )
                    }
                    _ => return Err(shape("`-` stands only before a literal")),
                },
                TokenKind::Integer(digits) => SetToken::Literal(
                    canonical_literal(digits)
                        .ok_or_else(|| shape("a literal is a canonical decimal"))?,
                ),
                TokenKind::Word(word) => SetToken::Word(word.clone()),
                TokenKind::Symbol(Symbol::Pipe) => SetToken::Or,
                TokenKind::Symbol(Symbol::Ampersand) => SetToken::And,
                TokenKind::Symbol(Symbol::Caret) => SetToken::Xor,
                TokenKind::Symbol(Symbol::LeftBracket) => SetToken::LeftBracket,
                TokenKind::Symbol(Symbol::RightBracket) => SetToken::RightBracket,
                TokenKind::Symbol(Symbol::LeftParen) => SetToken::LeftParen,
                TokenKind::Symbol(Symbol::RightParen) => SetToken::RightParen,
                TokenKind::Symbol(Symbol::Comma) => SetToken::Comma,
                other => return Err(shape(&format!("`{other:?}` is not part of a set"))),
            };
            set_tokens.push(token);
            index += 1;
        }
        let expr = parse(&set_tokens)?;
        expr.eval(universe).map_err(|error| error.to_string())
    }
}

fn load_function(path: &str, decl: &SourceDeclaration) -> Result<BoundsFunctionContract, String> {
    if decl.kind != DeclarationKind::ExternFunction || decl.c_ffi.is_some() {
        return Err("bounds modules accept only extern functions".into());
    }
    let mut parameters = Vec::new();
    for parameter in &decl.parameters {
        if parameter.usage.is_some() {
            return Err("a bounds declaration takes no usage annotation".into());
        }
        if parameters
            .iter()
            .any(|p: &BoundsParameter| p.name == parameter.name.value)
        {
            return Err(format!("duplicate parameter `{}`", parameter.name.value));
        }
        parameters.push(BoundsParameter {
            name: parameter.name.value.clone(),
            kind: type_kind(&parameter.type_source)?,
        });
    }
    let result = match decl.result_type.as_ref().map(|r| r.source.trim()) {
        Some("unit") => None,
        Some(source) => Some(type_kind(source)?),
        None => return Err("a bounds declaration states its result type".into()),
    };
    let result_type = decl
        .result_type
        .as_ref()
        .map(|r| r.source.trim().to_owned())
        .filter(|source| source != "unit");
    let mut flexible = 0usize;
    let contract = decl
        .function_contract
        .as_ref()
        .ok_or("missing bounds contract")?;
    let reader = Reader {
        parameters: &parameters,
        result,
    };
    let mut selectors = 0;
    let mut contract_review = None;
    let mut clauses: Vec<BoundsClauseEntry> = Vec::new();
    // Card NULL-DEREF-QTT-GROUNDING: which canonical parts a shorthand gave.
    let mut elaboration = crate::deref_parts::Elaboration::default();
    let mut normalised = vec![format!(
        "extern fn {}({}) -> {}",
        decl.name,
        decl.parameters
            .iter()
            .map(|p| format!("{}: {}", p.name.value, p.type_source.trim()))
            .collect::<Vec<_>>()
            .join(", "),
        decl.result_type.as_ref().map_or("", |r| r.source.trim())
    )];
    for clause in &contract.clauses {
        let clause = &clause.value;
        // Card NULL-SAFETY-PROPERTY: the collocated null-dereference check.
        // Card NULL-DEREF-QTT-GROUNDING: and the liveness part and the
        // composite shorthand, every one elaborated into canonical parts
        // ([`crate::deref_parts`]).
        if let Some(form) = crate::deref_parts::PartForm::of(&clause.kind) {
            let tokens = tokens_of(&clause.expression_source)?;
            let [TokenKind::Symbol(Symbol::Hash), TokenKind::Word(name)] = tokens.as_slice() else {
                return Err(form.shape_error());
            };
            let parameter = reader
                .parameter(&[TokenKind::Word(name.clone())], ParameterKind::Pointer)
                .map_err(|_| form.parameter_error(name))?;
            normalised.push(form.written(&parameter));
            let parts = match form {
                crate::deref_parts::PartForm::NullDeref => vec![BoundsClauseEntry {
                    clause: BoundsClause::NullDeref {
                        parameter: parameter.clone(),
                    },
                    text: format!("quant null-deref #{parameter}"),
                    review: None,
                }],
                crate::deref_parts::PartForm::Live => {
                    vec![crate::deref_parts::live_part(&parameter)]
                }
                crate::deref_parts::PartForm::Deref | crate::deref_parts::PartForm::Explicit => {
                    crate::deref_parts::shorthand_parts(&parameter, &reader)?
                }
            };
            elaboration.add(&mut clauses, parts, form)?;
            continue;
        }
        // Card QUANT-INDEX-CHECKS: the collocated index check.
        if clause.kind == SourceContractClauseKind::QuantCheck {
            let tokens = tokens_of(&clause.expression_source)?;
            let text = crate::bounds_quant::clause_text(&tokens);
            normalised.push(text.clone());
            let read = crate::bounds_quant::read_quant_check(&tokens, &reader)?;
            let entry = BoundsClauseEntry {
                clause: BoundsClause::QuantCheck {
                    parameter: read.parameter,
                    variable: read.variable,
                    predicate: read.predicate,
                },
                text,
                review: None,
            };
            // Card NULL-DEREF-QTT-GROUNDING: an explicit part overlapping a
            // shorthand's is merged when they agree, refused when not.
            elaboration.add(
                &mut clauses,
                vec![entry],
                crate::deref_parts::PartForm::Explicit,
            )?;
            continue;
        }
        if clause.kind == SourceContractClauseKind::QuantProposition {
            // Whole-graph validation below checks the typed formula. Retain
            // it in source identity; it adds no interpreted bounds fact.
            normalised.push(format!("quant proposition {}", clause.expression_source));
            continue;
        }
        let requires = match &clause.kind {
            SourceContractClauseKind::Requires => true,
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => false,
            _ => return Err("bounds contracts admit only `requires` and `ensures normal`".into()),
        };
        let tokens = tokens_of(&clause.expression_source)?;
        normalised.push(format!(
            "{} {}",
            if requires {
                "requires"
            } else {
                "ensures normal"
            },
            render(&tokens)
        ));
        if requires
            && matches!(tokens.as_slice(), [TokenKind::Word(w)] if w == BOUNDS_CONTRACT_MARKER)
        {
            selectors += 1;
            continue;
        }
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4): the flexible marker of
        // a C library result set.
        if let Some((BOUNDS_REVISION, groups)) = call(&tokens) {
            if !requires || !matches!(groups.as_slice(), [[TokenKind::Word(w)]] if w == "flexible")
            {
                return Err(format!(
                    "bounds-revision-shape: the only revision marker of a bounds declaration is `requires {BOUNDS_REVISION}(flexible);`"
                ));
            }
            flexible += 1;
            continue;
        }
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (G05j): the result set.
        if let Some(set) = result_set_clause(&tokens, result_type.as_deref()) {
            if requires {
                return Err(
                    "result-set-ensures: a result set is an `ensures normal` postcondition".into(),
                );
            }
            let set = set?;
            clauses.push(BoundsClauseEntry {
                text: format!("{RESULT_SET}({})", set.canonical()),
                clause: BoundsClause::ResultSet { set },
                review: None,
            });
            continue;
        }
        // Card CELL-RESULT-SET-ALIGNMENT (G05j as amended): a slot's written set.
        if let Some(written) = written_set_clause(&tokens, &parameters) {
            if requires {
                return Err(
                    "written-set-ensures: a written set is an `ensures normal` postcondition"
                        .into(),
                );
            }
            let (parameter, index, set, class) = written?;
            clauses.push(BoundsClauseEntry {
                text: format!(
                    "{WRITTEN_SET}({parameter}[{index}], {}) when #result == {class}",
                    set.canonical()
                ),
                clause: BoundsClause::WrittenSet {
                    parameter,
                    index,
                    set,
                    class,
                },
                review: None,
            });
            continue;
        }
        // Graph-v2 attachments are reference metadata, never bounds/null facts.
        // Full declaration validation below checks the original premise target.
        if crate::premise_flow::is_clause(&clause.expression_source)
            || crate::contract_lifecycle::is_clause(&clause.expression_source)
            || crate::guarded_decomposition::is_source_clause(&clause.expression_source)
        {
            if !requires {
                return Err("bounds flow: only requires flow metadata is admitted".into());
            }
            continue;
        }
        if crate::hypothesis::is_clause(&clause.expression_source) {
            if !requires || clause.expression_source.starts_with("resource_premise_all") {
                return Err("bounds hypothesis: only requires atoms and same-name decompositions are admitted".into());
            }
            continue;
        }
        if let Some((BOUNDS_REVIEW_MARKER, groups)) = call(&tokens) {
            let wraps = groups
                .first()
                .is_some_and(|group| matches!(group.first(), Some(TokenKind::Word(_))));
            if !wraps {
                if !requires {
                    return Err("a contract-level bounds review marker is a `requires`".into());
                }
                if contract_review.is_some() || clauses.iter().any(|c| c.review.is_some()) {
                    return Err("duplicate bounds review marker".into());
                }
                contract_review = Some(review_note(&groups)?);
                continue;
            }
            if contract_review.is_some() {
                return Err("duplicate bounds review marker".into());
            }
            let inner = groups[0];
            if matches!(call(inner), Some((BOUNDS_REVIEW_MARKER, _))) {
                return Err("a bounds review marker cannot wrap another marker".into());
            }
            if matches!(inner, [TokenKind::Word(w)] if w == BOUNDS_CONTRACT_MARKER) {
                return Err("the bounds selector cannot be held on its own".into());
            }
            let note = review_note(&groups[1..])?;
            let parsed = reader.clause(requires, inner)?;
            clauses.push(BoundsClauseEntry {
                clause: parsed,
                text: render(inner),
                review: Some(note),
            });
            continue;
        }
        // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the named premise, read
        // by the resource loader's own reader (the format's check already
        // refused any other shape); admitted only beside a scoped member
        // form, checked after the loop.
        if let Some(premise) = crate::contract_loader::premise_clause(&clause.expression_source)? {
            if !requires {
                return Err("a named premise is a `requires` clause".into());
            }
            clauses.push(BoundsClauseEntry {
                clause: BoundsClause::Premise {
                    name: premise.name,
                    statement: premise.statement,
                },
                text: render(&tokens),
                review: None,
            });
            continue;
        }
        let parsed = reader.clause(requires, &tokens)?;
        clauses.push(BoundsClauseEntry {
            clause: parsed,
            text: render(&tokens),
            review: None,
        });
    }
    if selectors != 1 {
        return Err(format!(
            "a bounds declaration carries exactly one `requires {BOUNDS_CONTRACT_MARKER};`"
        ));
    }
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4): a result set is a C library
    // fact, flexible, and the declaration's one clause; the flexible marker
    // stands on nothing else (no other bounds or null clause, no custody).
    let states_set = clauses
        .iter()
        .any(|entry| matches!(entry.clause, BoundsClause::ResultSet { .. }));
    if flexible > 1 {
        return Err(format!(
            "bounds-revision-shape: duplicate `requires {BOUNDS_REVISION}(flexible);`"
        ));
    }
    if states_set && flexible == 0 {
        return Err(format!(
            "result-set-needs-flexible: a `result_set` states a C library function's results and is admitted only on a declaration carrying `requires {BOUNDS_REVISION}(flexible);`"
        ));
    }
    // Card CELL-RESULT-SET-ALIGNMENT: a flexible declaration states its one
    // `result_set` and, beside it, the slots' written sets; nothing else.
    let value_sets_only = clauses.iter().all(|entry| {
        entry.review.is_none()
            && matches!(
                entry.clause,
                BoundsClause::ResultSet { .. } | BoundsClause::WrittenSet { .. }
            )
    });
    if flexible == 1 && !(states_set && value_sets_only) {
        return Err(format!(
            "bounds-revision-flexible-scope: `requires {BOUNDS_REVISION}(flexible);` stands only on a declaration whose clauses are one `result_set` and its slots' `written_set`s"
        ));
    }
    // Static conditions: at most one fact per position.
    let mut seen = std::collections::BTreeSet::new();
    for entry in &clauses {
        let key = match &entry.clause {
            BoundsClause::Extent { parameter, .. } => format!("extent {parameter}"),
            BoundsClause::Index { parameter, .. } => format!("index {parameter}"),
            BoundsClause::GlobalExtent { object, .. } => format!("global {object}"),
            BoundsClause::ReturnExtent { .. } => "return".into(),
            BoundsClause::PreservesExtents => "preserves".into(),
            BoundsClause::Frame { parameter, .. } => format!("frame {parameter}"),
            BoundsClause::ReturnDerived { parameter, .. } => format!("derived {parameter}"),
            BoundsClause::QuantCheck { parameter, .. } => format!("quant {parameter}"),
            BoundsClause::NullDeref { parameter } => format!("null-deref {parameter}"),
            BoundsClause::Live { parameter } => format!("live {parameter}"),
            BoundsClause::NonnullMember { tag, field } => format!("nonnull-member {tag}.{field}"),
            BoundsClause::NonnullResult => "nonnull-result".into(),
            BoundsClause::NonnullMemberStable { tag, field } => {
                format!("nonnull-member-stable {tag}.{field}")
            }
            BoundsClause::NonnullMemberScope {
                tag,
                field,
                parameter,
            } => format!("nonnull-member-scope {parameter} {tag}.{field}"),
            BoundsClause::NonnullResultRange { .. } => "nonnull-result-range".into(),
            BoundsClause::ResultSet { .. } => "result-set".into(),
            BoundsClause::WrittenSet {
                parameter, index, ..
            } => format!("written-set {parameter}[{index}]"),
            BoundsClause::Premise { name, .. } => format!("premise {name}"),
        };
        if !seen.insert(key.clone()) {
            return Err(format!("duplicate bounds clause for {key}"));
        }
    }
    // Card QUANT-INDEX-CHECKS: a pointer states its extent or its index
    // check, not both (the check's caller obligation is the extent it needs).
    for entry in &clauses {
        if let BoundsClause::QuantCheck { parameter, .. } = &entry.clause
            && seen.contains(&format!("extent {parameter}"))
        {
            if elaboration.from_shorthand(&format!("quant {parameter}")) {
                return Err(format!(
                    "quant-deref-conflict: `{parameter}` states a bounds_extent, incompatible with `quant deref #{parameter}`'s extent part (one fact per pointer: write `quant checks {parameter}, Idx #i: #i == 0;` or the extent, not both)"
                ));
            }
            return Err(format!(
                "quant-check-with-extent: `{parameter}` states both a bounds_extent and a quant check"
            ));
        }
    }
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15):
    // a premise names what a member fact rests on, so it stands only beside
    // a member clause; card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION
    // 2026-10-16, "Yes, all kinds"): beside any of its three kinds.
    if clauses
        .iter()
        .any(|entry| matches!(entry.clause, BoundsClause::Premise { .. }))
        && !clauses.iter().any(|entry| {
            matches!(
                entry.clause,
                BoundsClause::NonnullMember { .. }
                    | BoundsClause::NonnullMemberStable { .. }
                    | BoundsClause::NonnullMemberScope { .. }
            )
        })
    {
        return Err(
            "premise-beside-member: a bounds declaration admits `requires resource_premise(NAME, \"STATEMENT\");` only beside a `nonnull_member` clause (plain, `[[stable]]` or `[[scope]]`)"
                .into(),
        );
    }
    let legacy = clauses
        .iter()
        .filter_map(|entry| match &entry.clause {
            BoundsClause::Premise { name, statement } => {
                Some(crate::contract_loader::DeclaredPremise {
                    name: name.clone(),
                    statement: statement.clone(),
                    origin: crate::contract_loader::PremiseOrigin::Declared,
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let graph = crate::hypothesis::declaration(decl, &legacy)?;
    if !graph.requirements.is_empty() {
        return Err("formal entry obligations have no interpreted source valuation; no summary is installed".into());
    }

    crate::bounds_hypotheses::require_neutral_erasure(&graph, &legacy)?;
    crate::premise_flow::declaration(decl, &graph)?;
    crate::contract_lifecycle::declaration(decl, &graph)?;
    crate::guarded_decomposition::declaration(decl, &graph)?;
    frame_conditions(&parameters, &clauses)?;
    let canonical_sha256 =
        crate::deref_parts::canonical_sha256(&normalised[0], &clauses, contract_review.as_ref());
    Ok(BoundsFunctionContract {
        name: decl.name.clone(),
        module: path.to_owned(),
        parameters,
        result,
        clauses,
        contract_review,
        text_sha256: sha256_hex(normalised.join(";\n").as_bytes()),
        canonical_sha256,
        pin_hold: None,
        flexible: flexible == 1,
        result_type,
    })
}

/// Card CG24's static conditions: a closed frame names every pointer and slot
/// parameter; a slot appears only in a frame; preservation excludes a move;
/// a derived result excludes a return extent and a moved base.
fn frame_conditions(
    parameters: &[BoundsParameter],
    clauses: &[BoundsClauseEntry],
) -> Result<(), String> {
    let framed = clauses
        .iter()
        .filter_map(|entry| match &entry.clause {
            BoundsClause::Frame { parameter, mode } => Some((parameter.as_str(), *mode)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let has_slot = parameters.iter().any(|p| p.kind == ParameterKind::Slot);
    if !framed.is_empty() || has_slot {
        for parameter in parameters.iter().filter(|p| p.kind.is_pointer()) {
            if !framed.contains_key(parameter.name.as_str()) {
                return Err(format!(
                    "a frame is closed: pointer parameter `{}` states no frame",
                    parameter.name
                ));
            }
        }
    }
    let moves = framed.values().any(|mode| *mode == FrameMode::Moves);
    if moves
        && clauses
            .iter()
            .any(|entry| entry.clause == BoundsClause::PreservesExtents)
    {
        return Err("bounds_preserves_extents() contradicts a bounds_frame_moves".into());
    }
    let derived = clauses
        .iter()
        .filter_map(|entry| match &entry.clause {
            BoundsClause::ReturnDerived { parameter, .. } => Some(parameter.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !derived.is_empty() {
        if clauses
            .iter()
            .any(|entry| matches!(entry.clause, BoundsClause::ReturnExtent { .. }))
        {
            return Err("a derived result excludes a bounds_return_extent".into());
        }
        if let Some(base) = derived
            .iter()
            .find(|base| framed.get(**base) == Some(&FrameMode::Moves))
        {
            return Err(format!(
                "a derived result cannot alias `{base}`, which the call may move"
            ));
        }
    }
    Ok(())
}

/// Load every selected module, total-or-reject.
pub fn load_bounds_contract_sources(
    sources: &BTreeMap<String, String>,
) -> Result<BoundsContracts, String> {
    if sources.is_empty() {
        return Err("no bounds contract inputs selected".into());
    }
    let mut contracts = BoundsContracts::default();
    for (path, source) in sources {
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in &parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            if contracts.functions.contains_key(&decl.name) {
                return Err(format!("{context}: duplicate bounds contract"));
            }
            let loaded = load_function(path, decl).map_err(|e| format!("{context}: {e}"))?;
            contracts.functions.insert(decl.name.clone(), loaded);
        }
        contracts
            .sources
            .insert(path.clone(), sha256_hex(source.as_bytes()));
    }
    Ok(contracts)
}
