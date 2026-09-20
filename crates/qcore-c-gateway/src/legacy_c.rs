//! Conservative resource analysis for the Cycle 9 annotated legacy-C subset.
//!
//! The parser intentionally accepts more C than the analyzer models.  Every
//! construct outside the supported structured subset becomes an `Unknown`
//! instruction. `Safe` concerns the supported custody projection under its
//! declared resource contracts, not arbitrary C memory safety. Indirect reads,
//! aggregate/indexed accesses and ambiguous expression forms without a model
//! remain unsupported; symbolic resource place spelling is not access proof.

#[path = "legacy_joined_provenance.rs"]
mod joined_provenance;

#[path = "legacy_out_alias.rs"]
mod out_alias;

#[path = "legacy_return_call.rs"]
pub(crate) mod return_call;

#[path = "legacy_enumeration.rs"]
pub mod enumeration;

pub use enumeration::{
    EnumerationRefusal, SourceDefinitionInventory, UnanalyzableDefinition, UnanalyzableForm,
    enumerate_source_definitions,
};

#[path = "legacy_location.rs"]
pub mod location;

pub use location::{
    LocationRefusal, OriginFile, OriginalLocation, SourceLineMap, map_marked_preprocessing,
    map_preprocessed_lines, unlocated_site,
};

#[path = "legacy_preparation.rs"]
mod preparation;

pub use preparation::{
    PreparationMetrics, PreparationStageMetrics, PreparationStageOutcome, PreparedTranslationUnit,
    prepare_translation_unit,
};

#[path = "legacy_recursion.rs"]
pub(crate) mod recursion;

#[path = "legacy_literal_premise.rs"]
pub(crate) mod literal_premise;

#[path = "legacy_declared_abi.rs"]
pub mod declared_abi;

pub use declared_abi::{AbiAgreementRow, AbiOutcome, AbiRefusal, declared_abi_census};

#[path = "legacy_declared_modes.rs"]
pub mod declared_modes;

pub use declared_modes::{
    ModeAgreementRow, ModeOutcome, ModeRefusal, declared_mode_census, mode_contradiction,
};

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

pub type LegacyNodeId = usize;

pub use qcore_schemas::contract::{
    ContractTrust, ResourceApiKind, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
};

use crate::aggregate_store::AggregateStoreBases;
use crate::borrowed_actual::UntrackedBases;
// Card LOADER-ABI-MODES: every derivation this crate performs now runs inside
// `declared_modes::ModeGate`, which is why `derive_custody_summaries` is no
// longer named here. The gate settles the withdrawn set and hands back the
// derivation it settled on, so there is exactly one derivation per unit in the
// common case and no route that derives without the mode comparison.
use crate::legacy_summary::CustodyDerivation;
use crate::neutral_position::ExpressionRole;
pub use crate::publication::{IndirectPublication, PublicationFacts};
use crate::typed_field_access::{FunctionTypes, TypedAccessModel};

/// One admitted nested call (cards **O04-TRACE** and
/// **B04-NESTED-CALL-ARGUMENT**), as modeled.
///
/// Its declared effects are applied by [`transfer`] **before** the enclosing
/// call's, through the same `apply_contract_call` every other modeled call
/// goes through — the composition is one code path, not a second model. Under
/// the admission premises (`ResourceReturnMode::None`, every parameter
/// `Borrowed`, no produced site) that application is the identity on the
/// abstract state, which is exactly why the order C leaves unspecified cannot
/// be observed. The alias obligations of its own reads travel with the outer
/// call's in `alias_reads`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NestedContractCall {
    pub function: String,
    pub contract: ResourceFunctionContract,
    pub arguments: Vec<Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyInstruction {
    Acquire {
        destination: String,
        site: usize,
    },
    Alias {
        destination: String,
        source: String,
    },
    Clear {
        destination: String,
    },
    Release {
        source: String,
    },
    Escape {
        source: String,
    },
    /// Transfer a returned local resource when the expression names one.
    /// Non-resource scalar returns remain resource-neutral.
    EscapeIfOwned {
        source: String,
    },
    ContractCall {
        function: String,
        destination: Option<String>,
        arguments: Vec<Option<String>>,
        contract: ResourceFunctionContract,
        produced_sites: Vec<usize>,
        /// The admitted **bound reads** of this call (card O04-READ-BOUND): the
        /// canonical `BASE->FIELD` places occupying `Borrowed` positions that
        /// [`crate::aggregate_store::bound_read_actuals`] admitted on typed
        /// premises alone. [`transfer`] additionally requires each one, *here*,
        /// to resolve to a live aggregate this body acquired carrying the exact
        /// field link an admitted store recorded; otherwise the whole call
        /// refuses. Empty for every other statement in the corpus.
        aggregate_reads: Vec<String>,
        /// The subset of `aggregate_reads` at positions that *take* custody
        /// (card O04-B09B10). The call's own contract effect releases or
        /// consumes the linked identity; afterwards the member no longer holds
        /// anything this projection may denote, so [`transfer`] drops both the
        /// place binding and the field link. A second read of the same member
        /// then fails `bound_read_resolves` rather than releasing twice.
        aggregate_consumed: Vec<String>,
        /// The bases of this call's admitted **member-address** actuals whose
        /// aggregate *this* body bound (card O04-B09B10, O00 blockers
        /// **B09**/**B10**). Handing a callee the address of a member of a
        /// released, escaped, ambiguous, nullable or caller-held aggregate is
        /// refused by [`transfer`] before any effect of the call applies, in
        /// exactly the way a store through such an aggregate is. A
        /// member-address actual on a C02-P untracked base carries no entry
        /// here: that base can never bind a tracked identity at all.
        aggregate_addresses: Vec<String>,
        /// The C02-P-ALIAS may-alias obligations of this call's *read*
        /// arguments: one entry per distinct root name a modeled position
        /// dereferences that shares a declared record-pointer type with some
        /// other declared name of this function. [`transfer`] refuses the read
        /// when one of those peers was released on the path and neither
        /// discharge applies. Empty for every argument the rule does not
        /// relate, which is most of the corpus. See [`crate::may_alias`].
        alias_reads: Vec<AliasPeers>,
        /// Cards **O04-TRACE** and **B04-NESTED-CALL-ARGUMENT**: every sibling
        /// nested call this statement admits. Each declared effect applies
        /// before this call's own.
        ///
        /// It is admitted only when its declared effects are the *identity* on
        /// the abstract state — result `None`, every parameter `Borrowed` — so
        /// applying it first is well defined despite C leaving the evaluation
        /// order of arguments unspecified. Every other nested call is refused
        /// where it stands, with card O04-B09B10's diagnostic.
        nested: Vec<NestedContractCall>,
    },
    BranchOnNull {
        source: String,
        true_is_null: bool,
    },
    /// A branch on `LEFT == RIGHT` or `LEFT != RIGHT` between two plain names
    /// this function declares with the same record-pointer type (C02-P-ALIAS).
    ///
    /// It is *not* a null test — `BranchOnNull` still claims every one of
    /// those — and it refines nothing about custody. Its only effect is that
    /// the edge on which the two names are provably distinct carries that fact,
    /// which discharges the may-alias obligation for exactly that pair until
    /// either name is rebound. On the other edge nothing at all is assumed.
    BranchOnDistinct {
        left: String,
        right: String,
        /// Whether the first successor is the edge on which the two names are
        /// distinct (`!=`), rather than the edge on which they are equal.
        true_is_distinct: bool,
    },
    /// Bind `destination` to a value this function does not track: the read of
    /// an admitted direct member of an untracked aggregate (C02-P, O04).
    ///
    /// The resulting abstract pointer is exactly the one an [`Self::Alias`]
    /// from an unbound place would produce — unknown and possibly null, so
    /// every downstream use stays fail-closed. The only difference is that no
    /// unresolved-alias diagnostic is reported, because the typed premises of
    /// [`crate::borrowed_actual`] establish that the source place is one this
    /// function never binds. It never creates, releases or transfers custody.
    OpaqueValue {
        destination: String,
        /// The C02-P-ALIAS may-alias obligation of the base this read
        /// dereferences, when the rule relates it to another declared name.
        /// `None` whenever it relates nothing, which is the pre-existing
        /// answer.
        alias_read: Option<AliasPeers>,
    },
    /// Forget a declared scalar's old branch fact after a modeled write.
    /// An absent binding admits both scalar branches and carries no pointer
    /// identity. Resource obligations are retained independently.
    ForgetScalar {
        destination: String,
    },
    /// An exactly-shaped scalar store through a record pointer this function
    /// may bind.  Unlike the unbound scalar-field no-op, transfer must prove
    /// that `base` denotes one settled, live, non-null identity with no
    /// unresolved effect before the store is admitted.  The scalar member and
    /// scalar-only right-hand side carry no tracked identity, so both an
    /// accepted store and a refusal carrying `UnknownAlias` preserve the
    /// pointer, custody, field-link and scalar-fact maps.
    ScalarMemberStore {
        base: String,
        /// The same C02-P-ALIAS obligation as every other modeled read through
        /// a declared record pointer.
        alias_read: Option<AliasPeers>,
    },
    /// A store into an admitted field of an aggregate this function tracks
    /// whose stored value carries no tracked identity (O00 blocker B08): a
    /// scalar expression into a scalar member, or a declared pointer this
    /// function never binds into a single-star pointer member. See
    /// [`crate::aggregate_store`].
    ///
    /// It is *not* a no-op. Storing through an aggregate this function already
    /// released, escaped or never resolved is refused by [`transfer`], and the
    /// store clears whatever field link the member previously carried, so an
    /// overwritten resource is never silently discharged.
    AggregateStore {
        place: String,
    },
    NoOp,
    Unknown {
        description: String,
        /// Card **AOM-BINDING-POISON**: what this refusal is allowed to do to
        /// the *binding* environment. [`RefusedWrites::Unbounded`] is the
        /// pre-repair answer and the answer every structural control-flow refusal
        /// keeps; see [`RefusedWrites`] for the premises the bounded answer
        /// rests on.
        writes: RefusedWrites,
    },
}

/// Card **AOM-BINDING-POISON**. What a refused statement may do to the binding
/// environment, as opposed to the resource states — which it never touches.
///
/// Before this card every refusal set `unknown` and `may_be_null` on **every**
/// binding in the state, including bindings the refused text does not so much
/// as mention. Card AOM-DESTROY-SIBLINGS measured the cost of that: in
/// `flb_input_chunk_get_direct_routes` an acquisition at preprocessed line
/// 19357 lost its *name* to three unrelated refused stores into
/// `result[index].…`, so twelve declared releases of `result` could only refuse
/// and six leak leaves retired by **withdrawal** rather than by discharge.
///
/// The bounded answer is **not** a relaxation of any refusal: the statement
/// keeps its `Unsupported` diagnostic, still poisons the path
/// ([`is_path_poisoning`]) and still makes its function `Unknown`. It only
/// stops the refusal from destroying facts about names it cannot have written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusedWrites {
    /// Nothing bounds this refusal's effect on the binding environment, so
    /// every binding is poisoned. This is the answer for every refusal that
    /// does **not** stand for exactly one statement whose complete token text
    /// was read into it — a `goto` to an unresolved label, a malformed
    /// for-loop clause, a break or continue outside its construct, a duplicate
    /// or stray label. A parser-delimited complete condition or return
    /// expression now carries its own bounded frame (AOM-POISON-CONDITION-FRAME).
    Unbounded,
    /// This refusal stands for one statement, for-loop clause, or complete
    /// condition/return expression whose **complete** token text the parser read, and the set carried here is the
    /// explicit, closed list of places it **spares**. Everything not in it,
    /// including every place this projection has not bound yet, is poisoned
    /// exactly as before.
    ///
    /// A place is on that list only when all three premises hold:
    ///
    /// 1. it is a plain name ([`is_plain_name_place`]) — not a member, a
    ///    subscript or a `*out` slot, each of which denotes storage some other
    ///    lvalue can reach;
    /// 2. [`frame_protected_locals`] positively recognised it as an object of
    ///    **automatic** storage duration declared by this body, or one of its
    ///    formals, and no `&` anywhere in the body takes its address — so no
    ///    lvalue other than the name itself designates that slot;
    /// 3. the refused text contains no occurrence of the name in a position
    ///    [`may_write_names`] reads as a write.
    ///
    /// Under those three a refused statement cannot change the value of that
    /// slot: a store through a pointer, a subscript or a member selection
    /// writes the pointed-to object, and a call can reach only what its
    /// arguments and file-scope objects give it. The premise is the ordinary C
    /// object model, and the no-undefined-behaviour reading of it that the
    /// whole projection already rests on when it models `flb_free(p)` as
    /// releasing exactly `p`'s site: a write past the end of one object onto
    /// another is undefined, not a modelled effect.
    Sparing(BTreeSet<String>),
}

impl RefusedWrites {
    /// Whether this refusal poisons the binding of `place`.
    ///
    /// Fail-closed by construction: an unbounded refusal poisons everything,
    /// and a bounded one poisons everything outside a list it built positively.
    fn poisons(&self, place: &str) -> bool {
        match self {
            Self::Unbounded => true,
            Self::Sparing(spared) => !spared.contains(place),
        }
    }
}

/// Whether `place` names a slot rather than storage some other lvalue reaches.
///
/// [`canonical_place`] and [`direct_assignment_destination`] between them build
/// `NAME`, `NAME.f`, `NAME->f`, `NAME[0]` and `*NAME`; only the first is a
/// variable, and every other shape denotes memory an unrelated write can reach.
fn is_plain_name_place(place: &str) -> bool {
    !place.is_empty() && !place.contains(['.', '[', ']', '*', '-', '>', '(', ')'])
}

/// Card **AOM-BINDING-POISON**: the names one statement's text may write.
///
/// This is the load-bearing reader of the repair, so it is written as a
/// positive recognition of C's **write** positions, with everything it does not
/// recognise defaulting to *written*. The importer's lexer emits one token per
/// punctuation character, so `==` is `=` `=`, `+=` is `+` `=`, `++` is `+` `+`
/// and `->` is `-` `>`; the operator run after a name is read with that in
/// mind.
///
/// A name at index `i` is taken to be written when, skipping a run of `)` (so
/// `(x) = 1` and `((x)) = 1` are read like `x = 1`):
///
/// * the operator run that follows contains `=` and is not one of the four
///   comparisons `==`, `!=`, `<=`, `>=` — which covers `=` and every compound
///   assignment including `<<=` and `>>=`, and defaults *toward* written for
///   any run this reader does not know;
/// * or that run is `++` or `--`, or the two tokens before the name are;
/// * or the next token is `.`, so a member store into a struct **object** is
///   attributed to the object.
///
/// Every other occurrence is a read: a subscript base (`a[i] = v` writes
/// `*(a+i)`), a `->` base (`p->f = v` writes `*p`), a call argument passed by
/// value, and an operand. Taking a name's address is not read here at all —
/// [`frame_protected_locals`] has already removed every name a `&` reaches
/// anywhere in the body, which is what makes "a call can only write what it can
/// reach" true of the names this set does not mention.
fn may_write_names(texts: &[&str]) -> BTreeSet<String> {
    const OPERATORS: [&str; 12] = ["=", "!", "<", ">", "+", "-", "*", "/", "%", "&", "|", "^"];
    const COMPARISONS: [[&str; 2]; 4] = [["=", "="], ["!", "="], ["<", "="], [">", "="]];
    let mut written = BTreeSet::new();
    for (index, text) in texts.iter().enumerate() {
        // Reserved words are deliberately *not* excluded: over-recognising a
        // write costs only precision, while missing one would be unsound.
        if !is_identifier(text) {
            continue;
        }
        let mut prefix = index;
        while prefix > 0 && texts[prefix - 1] == "(" {
            prefix -= 1;
        }
        if prefix >= 2 && matches!(&texts[prefix - 2..prefix], ["+", "+"] | ["-", "-"]) {
            written.insert((*text).to_owned());
            continue;
        }
        let mut cursor = index + 1;
        while texts.get(cursor) == Some(&")") {
            cursor += 1;
        }
        if texts.get(cursor) == Some(&".") {
            written.insert((*text).to_owned());
            continue;
        }
        let end = (cursor..texts.len().min(cursor + 3))
            .take_while(|at| OPERATORS.contains(&texts[*at]))
            .last()
            .map_or(cursor, |at| at + 1);
        let run = &texts[cursor..end];
        let assigns = if run.starts_with(&["+", "+"]) || run.starts_with(&["-", "-"]) {
            true
        } else if COMPARISONS.iter().any(|pair| run.starts_with(pair)) {
            false
        } else {
            run.contains(&"=")
        };
        if assigns {
            written.insert((*text).to_owned());
        }
    }
    written
}

/// Card **AOM-BINDING-POISON**: the names of one function body whose *slot* no
/// lvalue but the name itself designates.
///
/// Three exclusions, each positive and each fail-closed — a name this reader
/// cannot classify is simply never added:
///
/// * only names [`crate::publication::body_declarations`] read out of this
///   body's own declaration runs, plus its formals, are candidates at all;
/// * a `static` (or `_Thread_local`) local is dropped: its object outlives the
///   call exactly as a file-scope object does, so a callee can reach it. So is
///   any name that is *also* a file-scope object of this translation unit,
///   which covers a declaration run the reader misread (`a * b;` contributes
///   `b`) whenever that name is in fact a global;
/// * any name a `&` in this body is followed by — after a run of `(` — is
///   dropped. To take the address of a declared object you must name it, and
///   after any parentheses that name comes first; `&p->f`, `&a[i]` and a
///   bitwise `x & y` all drop a name they did not need to, which is the
///   harmless direction.
///
/// The result is used only to *spare* bindings from a refused statement's
/// poison, never to admit anything: see [`RefusedWrites::Sparing`].
fn frame_protected_locals(
    unit: &[&str],
    body: &[&str],
    formals: &BTreeSet<String>,
) -> BTreeSet<String> {
    let (declared, static_duration) = crate::publication::body_declarations(body);
    let file_scope = crate::publication::file_scope_objects(unit);
    let mut protected = declared
        .into_iter()
        .chain(formals.iter().cloned())
        .filter(|name| {
            is_plain_name_place(name)
                && !static_duration.contains(name)
                && !file_scope.contains(name)
        })
        .collect::<BTreeSet<_>>();
    for (index, text) in body.iter().enumerate() {
        if *text != "&" {
            continue;
        }
        let mut cursor = index + 1;
        while body.get(cursor) == Some(&"(") {
            cursor += 1;
        }
        if let Some(name) = body.get(cursor) {
            protected.remove(*name);
        }
    }
    protected
}

/// One modeled read's C02-P-ALIAS obligation (card C02-P-ALIAS).
///
/// `place` is the root name the read dereferences — `storage` for the actual
/// `storage->cio`, `b` for the actual `b`. `peers` are the *other* declared
/// names of this function that share `place`'s declared `struct TAG *` type.
/// Releasing any peer leaves this read unresolved unless a discharge applies;
/// see [`crate::may_alias`]. Card ALIAS-LOCALS removed v1's additional
/// requirement that at least one of the two be a formal parameter, so a pair of
/// locals is now related exactly like any other pair.
///
/// Both fields are derived from the same preprocessed token stream every other
/// typed rule reads, so W4 derivation, W4 validation and
/// [`check_legacy_safety_certificate`] all reproduce them from the
/// certificate's own content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasPeers {
    pub place: String,
    pub peers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCfgNode {
    pub instruction: LegacyInstruction,
    pub successors: Vec<LegacyNodeId>,
    pub line: usize,
    pub is_exit: bool,
    /// Card **SCALAR-ROUTE-EXIT-EDGE**: the index in [`Self::successors`] that
    /// leaves the loop this node heads, for the three loop headers that have
    /// one — `while`, `do`-`while` and `for`. `None` on every other node,
    /// including a `for (;;)` header, which carries no exit successor at all.
    ///
    /// It is a **static** structural fact recorded where the successors are
    /// assigned, never re-derived from reachability: constraint 6 records that
    /// a fact read off an intermediate iterate depends on the work-list
    /// schedule, and this one gates a refusal, so it must not.
    ///
    /// Its single consumer is the `Infeasible` arm of the custody work list,
    /// which may not delete this edge. See that arm for why.
    pub loop_exit_edge: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCfg {
    pub function: String,
    pub entry: LegacyNodeId,
    pub nodes: Vec<LegacyCfgNode>,
    /// Card **CTWIN-PUBLISH**: the assignment destinations of this body whose
    /// object has **static storage duration** — file-scope objects it does not
    /// shadow, plus its own `static` locals. Derived from the same preprocessed
    /// token stream as every other typed premise, so certificate rederivation
    /// reproduces it; see [`crate::publication`].
    /// Card **CTWIN-RESIDUALS** groups the three facts that travel together;
    /// `destinations` is card CTWIN-PUBLISH's original set.
    pub publications: PublicationFacts,
    /// Unshadowed automatic local names, never address-taken, eligible for
    /// the bounded conditional-provenance analysis. Derived from source.
    pub conditional_names: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisStatus {
    Safe,
    Unsafe,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegacyDiagnosticKind {
    Leak {
        acquisition_line: usize,
    },
    DoubleRelease {
        acquisition_line: usize,
    },
    ReleaseAfterEscape {
        acquisition_line: usize,
    },
    CustodyConflict {
        acquisition_line: usize,
    },
    UnknownAlias {
        variable: String,
    },
    Unsupported {
        description: String,
    },
    /// A validated inferred custody summary contradicts the explicit contract
    /// in force for the same function (W4). The explicit contract still wins at
    /// every call site; the disagreement is reported instead of being silently
    /// overridden, and it keeps the defining function `Unknown`.
    SummaryConflict {
        inferred: Box<ResourceFunctionContract>,
        declared: Box<ResourceFunctionContract>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegacyDiagnostic {
    pub function: String,
    pub line: usize,
    pub kind: LegacyDiagnosticKind,
}

/// What one reachable exit of a function leaves behind, per resource identity.
///
/// The function-wide [`AnalysisStatus`] is a join over every path: one
/// unmodeled construct anywhere makes the whole function `Unknown` and says
/// nothing about the paths that *are* modeled. Card O04 needs the finer
/// statement — "on this failure arm the constructed identity is released
/// exactly once" — for a caller whose success exit publishes into aggregates
/// this projection cannot follow. This record supplies it without changing any
/// verdict: it is an additional observation, never an input to a status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyExitResidual {
    pub node: LegacyNodeId,
    /// Input line of the exit statement.
    pub line: usize,
    pub custody: ExitCustody,
    /// Every resource identity this function acquired that is still accounted
    /// for at this exit, in acquisition-site order. Identities the caller owns
    /// (formal parameters under a contract) are excluded: discharging them is
    /// the caller's obligation, not this exit's.
    pub sites: Vec<LegacyResidualSite>,
}

/// One resource identity's disposition at one exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyResidualSite {
    pub site: usize,
    /// Input line of the acquisition, or 0 when it is not a modeled call.
    pub acquisition_line: usize,
    pub disposition: SiteDisposition,
}

/// Whether an exit's custody obligations are settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitCustody {
    /// Every path reaching this exit is fully modeled, and no identity this
    /// function acquired is still live here.
    Discharged,
    /// Every path reaching this exit is fully modeled, and at least one
    /// identity is still live — a residual this analysis does claim.
    Residual,
    /// Some path reaching this exit carries an unmodeled construct, an
    /// unresolved effect or a widened alias set. No custody claim is made, in
    /// either direction.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyFunctionSummary {
    pub function: String,
    pub status: AnalysisStatus,
    pub diagnostics: Vec<LegacyDiagnostic>,
    /// Number of work-list transfers used to reach the CFG fixed point.
    pub fixed_point_steps: usize,
    pub cfg: LegacyCfg,
    /// Per-exit custody residuals, in CFG node order (O04).
    pub exits: Vec<LegacyExitResidual>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyProgramSummary {
    pub entry: String,
    pub status: AnalysisStatus,
    pub functions: BTreeMap<String, LegacyFunctionSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyCError {
    FunctionNotFound(String),
    MalformedFunction {
        function: String,
        line: usize,
    },
    ProductionParserUnavailable(String),
    ProductionParserRejected(String),
    /// The unit uses a construct whose meaning depends on the input identity
    /// its declared [`CSourceInput`] cannot supply. Rejected, never guessed.
    UnsupportedInputIdentity(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct C11TranslationUnit<'a> {
    pub source: &'a str,
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    /// Ordered, typed preprocessing options, applied after the legacy fields.
    pub preprocessor_options: Vec<CPreprocessorOption>,
    pub dialect: CCompilerDialect,
    pub function_scope: LegacyFunctionScope,
    /// How Clang is told to read this unit's primary input. See [`CSourceInput`].
    pub input: CSourceInput,
}

/// The declared identity of a translation unit's primary input.
///
/// Preprocessing is not a function of the source text alone: `__FILE__`,
/// `__FILE_NAME__`, `__BASE_FILE__` and quoted includes resolved against the
/// includer's directory all answer to the *input Clang was given*. Feeding text
/// through standard input answers `<stdin>` for the first three and resolves
/// quoted includes against the gateway process's working directory. This type
/// makes that choice explicit so a unit either sees the input identity it
/// declares, or is rejected for a construct the transport cannot answer.
///
/// This records how the input is transported. It is not a claim that the
/// gateway's staged preprocess/syntax/AST pipeline is equivalent to a direct
/// one-shot compilation.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub enum CSourceInput {
    /// A unit with no declared file: the text is piped to Clang as `-`.
    ///
    /// The unit's identity is the text plus its declared options; quoted
    /// includes resolve against the gateway process's recorded working
    /// directory, because an anonymous text has no includer directory of its
    /// own. Constructs that would have to invent a file name are rejected.
    #[default]
    AnonymousText,
    /// A declared file: Clang is run from `directory` and given `spelled`
    /// exactly as a compilation database records the input argument, so
    /// `__FILE__` and quoted includes answer as a direct compile does.
    ///
    /// The declared source bytes must equal the file's bytes; otherwise the
    /// unit is rejected rather than analyzed under a different text.
    DeclaredFile {
        /// The input argument exactly as the recorded compile command spells it.
        spelled: String,
        /// The absolute working directory the recorded compile command declares.
        directory: PathBuf,
    },
    /// The historical Cycle 19 compatibility projection: standard input with
    /// `<stdin>` accepted as the unit's file name.
    ///
    /// Retained only for the pinned compatibility adapter whose archived
    /// artifacts were produced under it. Selecting this is a declared
    /// assumption that the unit's filename-sensitive constructs do not matter
    /// to its projection; it is never an equivalence claim.
    HistoricalStdin,
}

impl CSourceInput {
    /// The input identity for a unit whose text is *derived* from this one, for
    /// example by appending a probe. Derived text is not the declared file, so
    /// a declared identity is dropped rather than reused for different bytes.
    pub fn for_derived_text(&self) -> Self {
        match self {
            Self::HistoricalStdin => Self::HistoricalStdin,
            Self::AnonymousText | Self::DeclaredFile { .. } => Self::AnonymousText,
        }
    }

    /// The name this input identity class records, so a transported question
    /// can name which class a unit declares and the core can refuse a claim
    /// that is not the class its own unit carries.
    ///
    /// This is deliberately a *class* and not a full identity:
    /// [`Self::for_derived_text`] collapses [`Self::DeclaredFile`] to
    /// [`Self::AnonymousText`], so a derived unit records neither the spelled
    /// input nor the directory the base unit declared. A route that derives
    /// text must therefore bind the **base** unit's own identity separately;
    /// the derived identity cannot stand in for it.
    #[must_use]
    pub const fn class(&self) -> &'static str {
        match self {
            Self::AnonymousText => "anonymous_text",
            Self::DeclaredFile { .. } => "declared_file",
            Self::HistoricalStdin => "historical_stdin",
        }
    }

    /// Whether the transport must reject `<stdin>` file-name expansions.
    fn rejects_stdin_file_names(&self) -> bool {
        matches!(self, Self::AnonymousText)
    }

    /// Whether the transport must reject deferred source-location builtins.
    ///
    /// These survive preprocessing and are resolved by the frontend, which the
    /// gateway always feeds preprocessed text through standard input. Only the
    /// pinned historical route keeps its previous silent behavior.
    fn rejects_source_location_builtins(&self) -> bool {
        !matches!(self, Self::HistoricalStdin)
    }
}

/// Append a *derivation recipe* to a base unit's text.
///
/// One route in this repository imports from text it derived rather than from
/// the declared file: the memory-close composition appends a constant probe so
/// a preprocessor-defined backend discriminant can be read as an ordinary
/// integer literal instead of being guessed. The derivation is a single
/// concatenation, written here once so every caller — the trusted decision
/// procedure, its transported projection and its verification task — derives
/// the same bytes from the same base and the same recipe.
#[must_use]
pub fn derived_source_text(base: &str, recipe: &str) -> String {
    format!("{base}{recipe}")
}

/// The translation unit a derived text is analysed as.
///
/// Every declared option of the base unit is carried across unchanged, because
/// the derived text is the base text plus a recipe and must be preprocessed the
/// same way. The one thing that is **not** carried across is the input
/// identity: [`CSourceInput::for_derived_text`] drops a declared file, because
/// derived text is not that file's bytes. A caller that needs to know which
/// base unit the derived text came from must bind the base unit's own identity;
/// see [`CSourceInput::class`].
#[must_use]
pub fn derived_translation_unit<'a>(
    base: &C11TranslationUnit<'_>,
    derived: &'a str,
) -> C11TranslationUnit<'a> {
    C11TranslationUnit {
        source: derived,
        include_paths: base.include_paths.clone(),
        definitions: base.definitions.clone(),
        preprocessor_options: base.preprocessor_options.clone(),
        dialect: base.dialect,
        function_scope: base.function_scope,
        input: base.input.for_derived_text(),
    }
}

/// Bounded preprocessing transport. These are arguments, never shell commands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CPreprocessorOption {
    Include(String),
    SystemInclude(String),
    QuoteInclude(String),
    Define(String),
    Undefine(String),
    FrontendFlag(CFrontendFlag),
}

/// One `-O` optimization level, spelled exactly as the build spelled it.
///
/// An optimization level is *not* a code-generation-only request: Clang's
/// preprocessor defines `__OPTIMIZE__` (and `__OPTIMIZE_SIZE__` for the size
/// levels) and drops `__NO_INLINE__` whenever a level above `-O0` is selected,
/// and glibc headers branch on those macros. Dropping the build's level
/// therefore analyses a *different* translation unit; the measurement backing
/// this is recorded in `docs/reports/remaining-g03-f/README.md`. Levels are a
/// closed set: a level this enum cannot name is refused, never approximated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum COptimizationLevel {
    /// `-O0`
    None,
    /// `-O1`
    Level1,
    /// `-O2`
    Level2,
    /// `-O3`
    Level3,
    /// `-Os`
    Size,
    /// `-Oz`
    MinimumSize,
    /// `-Og`
    Debug,
}

impl COptimizationLevel {
    /// The build spelling this level was admitted from, reproduced verbatim.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::None => "-O0",
            Self::Level1 => "-O1",
            Self::Level2 => "-O2",
            Self::Level3 => "-O3",
            Self::Size => "-Os",
            Self::MinimumSize => "-Oz",
            Self::Debug => "-Og",
        }
    }
}

/// Explicit build flags admitted to Clang preprocessing, syntax and typed AST.
/// Assembler requests are retained but no object/stack property is certified.
///
/// Every member is admitted because its effect on what this gateway reads —
/// the preprocessed token stream, the syntax phase and the typed AST — has a
/// recorded preservation argument, and it is reproduced verbatim on all three
/// Clang invocations by [`CFrontendFlag::append_to`]. A build option with no
/// such argument stays refused by the callers that build this type; there is no
/// catch-all member and no "ignore unknown flag" path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CFrontendFlag {
    AllWarnings,
    ExtraWarnings,
    /// Warn when an undefined preprocessor identifier starts with this prefix.
    UndefPrefix(String),
    DebugInformation,
    PositionIndependentCode,
    AssemblerNoExecStack,
    /// Select one Apple Clang architecture using its two-argument `-arch` form.
    Architecture(String),
    // --- Diagnostic-only warning selections -------------------------------
    // Each of the members below selects or deselects one Clang warning group.
    // Warning selection is measured to leave the preprocessed token stream
    // byte-identical and the predefined macro set unchanged, so transporting
    // one preserves the recorded compile command without changing the analysed
    // program. None of them is `-Werror`: no warning becomes a hard error, so
    // an admitted unit cannot be turned into a refusal by its warning set.
    /// `-Wstrict-aliasing`
    StrictAliasingWarnings,
    /// `-Wstrict-prototypes`
    StrictPrototypes,
    /// `-Wundef`
    UndefinedIdentifierWarnings,
    /// `-Wmissing-prototypes`
    MissingPrototypes,
    /// `-Winit-self`
    InitSelf,
    /// `-Wmissing-field-initializers`
    MissingFieldInitializers,
    /// `-Wdeclaration-after-statement`
    DeclarationAfterStatement,
    /// `-Waddress`
    AddressComparisons,
    /// `-Woverride-init`
    OverrideInit,
    /// `-Wwrite-strings`
    WriteStrings,
    /// `-Wvla`
    VariableLengthArrays,
    /// `-Wno-unused-parameter`
    NoUnusedParameter,
    /// `-Wno-unused-function`
    NoUnusedFunction,
    /// `-Wno-pragmas`
    NoPragmas,
    /// `-Wno-void-pointer-to-enum-cast`
    NoVoidPointerToEnumCast,
    // --- Code generation ---------------------------------------------------
    /// `-fno-strict-aliasing`: relaxes the aliasing rule the optimizer may
    /// assume. Measured to change neither the predefined macros nor the
    /// preprocessed text; it is transported so the recorded command is
    /// reproduced, and, exactly like [`Self::AssemblerNoExecStack`], no object
    /// or aliasing property is certified by admitting it.
    NoStrictAliasing,
    /// `-O0` … `-Og`: transported because the level is visible to the
    /// preprocessor (see [`COptimizationLevel`]).
    Optimization(COptimizationLevel),
}

impl CFrontendFlag {
    fn append_to(&self, command: &mut std::process::Command) {
        match self {
            Self::AllWarnings => command.arg("-Wall"),
            Self::ExtraWarnings => command.arg("-Wextra"),
            Self::UndefPrefix(prefix) => command.arg(format!("-Wundef-prefix={prefix}")),
            Self::DebugInformation => command.arg("-g"),
            Self::PositionIndependentCode => command.arg("-fPIC"),
            Self::AssemblerNoExecStack => command.arg("-Wa,--noexecstack"),
            Self::Architecture(architecture) => command.args(["-arch", architecture]),
            Self::StrictAliasingWarnings => command.arg("-Wstrict-aliasing"),
            Self::StrictPrototypes => command.arg("-Wstrict-prototypes"),
            Self::UndefinedIdentifierWarnings => command.arg("-Wundef"),
            Self::MissingPrototypes => command.arg("-Wmissing-prototypes"),
            Self::InitSelf => command.arg("-Winit-self"),
            Self::MissingFieldInitializers => command.arg("-Wmissing-field-initializers"),
            Self::DeclarationAfterStatement => command.arg("-Wdeclaration-after-statement"),
            Self::AddressComparisons => command.arg("-Waddress"),
            Self::OverrideInit => command.arg("-Woverride-init"),
            Self::WriteStrings => command.arg("-Wwrite-strings"),
            Self::VariableLengthArrays => command.arg("-Wvla"),
            Self::NoUnusedParameter => command.arg("-Wno-unused-parameter"),
            Self::NoUnusedFunction => command.arg("-Wno-unused-function"),
            Self::NoPragmas => command.arg("-Wno-pragmas"),
            Self::NoVoidPointerToEnumCast => command.arg("-Wno-void-pointer-to-enum-cast"),
            Self::NoStrictAliasing => command.arg("-fno-strict-aliasing"),
            Self::Optimization(level) => command.arg(level.spelling()),
        };
    }
}

impl C11TranslationUnit<'_> {
    pub(crate) fn append_frontend_flags(&self, command: &mut std::process::Command) {
        for option in &self.preprocessor_options {
            if let CPreprocessorOption::FrontendFlag(flag) = option {
                flag.append_to(command);
            }
        }
    }

    /// Append the primary input for the phase that reads the original source,
    /// and report whether the source text must also be piped to standard input.
    ///
    /// A declared file is checked against its own bytes first: analyzing text
    /// that differs from the file Clang reads would make the recorded input
    /// identity a different unit from the effective one.
    pub(crate) fn append_primary_input(
        &self,
        command: &mut std::process::Command,
    ) -> Result<bool, LegacyCError> {
        match &self.input {
            CSourceInput::AnonymousText | CSourceInput::HistoricalStdin => {
                command.arg("-");
                Ok(true)
            }
            CSourceInput::DeclaredFile { spelled, directory } => {
                if !directory.is_absolute() {
                    return Err(LegacyCError::UnsupportedInputIdentity(
                        "declared input directory must be absolute".into(),
                    ));
                }
                if spelled.is_empty() || spelled.starts_with('-') {
                    return Err(LegacyCError::UnsupportedInputIdentity(format!(
                        "declared input `{spelled}` is not a source path argument"
                    )));
                }
                let path = directory.join(spelled);
                let bytes = std::fs::read(&path).map_err(|error| {
                    LegacyCError::UnsupportedInputIdentity(format!(
                        "declared input `{}` is unreadable: {error}",
                        path.display()
                    ))
                })?;
                if bytes != self.source.as_bytes() {
                    return Err(LegacyCError::UnsupportedInputIdentity(format!(
                        "declared input `{}` does not hold the declared source bytes",
                        path.display()
                    )));
                }
                command.current_dir(directory).arg(spelled);
                Ok(false)
            }
        }
    }
}

/// Constructs whose expansion is decided by the input Clang was given.
///
/// The scan runs over preprocessed output, so a construct reaching the unit
/// through an included header is seen exactly like one spelled in the source.
fn reject_filename_sensitive_constructs(
    preprocessed: &str,
    input: &CSourceInput,
) -> Result<(), LegacyCError> {
    let bytes = preprocessed.as_bytes();
    let mut cursor = 0;
    let mut line = 1usize;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'\n' {
            line += 1;
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                line += usize::from(bytes[cursor] == b'\n');
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'"' || byte == b'\'' {
            let quote = byte;
            let start = cursor + 1;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else if bytes[cursor] == quote {
                    break;
                } else {
                    line += usize::from(bytes[cursor] == b'\n');
                    cursor += 1;
                }
            }
            // `__FILE__`, `__FILE_NAME__` and `__BASE_FILE__` all expand to the
            // input's name. Standard input names it `<stdin>`, which is not the
            // unit's file, so the expansion is a construct without an answer.
            if input.rejects_stdin_file_names()
                && &preprocessed[start..cursor.min(bytes.len())] == "<stdin>"
            {
                return Err(LegacyCError::UnsupportedInputIdentity(format!(
                    "preprocessed line {line}: a source file name macro \
                     (`__FILE__`, `__FILE_NAME__` or `__BASE_FILE__`) expanded to \
                     `<stdin>` because this unit declares no input file"
                )));
            }
            cursor = (cursor + 1).min(bytes.len());
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            let name = &preprocessed[start..cursor];
            // These survive preprocessing and are answered by the frontend,
            // which always reads preprocessed text from standard input here.
            if input.rejects_source_location_builtins()
                && matches!(name, "__builtin_FILE" | "__builtin_FILE_NAME")
            {
                return Err(LegacyCError::UnsupportedInputIdentity(format!(
                    "preprocessed line {line}: `{name}` is answered after \
                     preprocessing, where this gateway reads standard input"
                )));
            }
        } else {
            cursor += 1;
        }
    }
    Ok(())
}

impl CPreprocessorOption {
    fn append_to(&self, command: &mut std::process::Command) {
        let (flag, value) = match self {
            Self::Include(value) => ("-I", value),
            Self::SystemInclude(value) => ("-isystem", value),
            Self::QuoteInclude(value) => ("-iquote", value),
            Self::Define(value) => ("-D", value),
            Self::Undefine(value) => ("-U", value),
            Self::FrontendFlag(flag) => {
                flag.append_to(command);
                return;
            }
        };
        command.args([flag, value]);
    }
}

/// Explicitly gated C dialects accepted by the production-parser gateway.
/// Arbitrary compiler arguments are intentionally not accepted here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CCompilerDialect {
    #[default]
    C11,
    Gnu99,
    Gnu11,
    Gnu17,
}

impl CCompilerDialect {
    pub(crate) fn clang_argument(self) -> &'static str {
        match self {
            Self::C11 => "-std=c11",
            Self::Gnu99 => "-std=gnu99",
            Self::Gnu11 => "-std=gnu11",
            Self::Gnu17 => "-std=gnu17",
        }
    }
}

/// Selects whether header-expanded definitions participate in the proof.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegacyFunctionScope {
    /// Preserve the Cycle 17 behavior and analyze every preprocessed body.
    #[default]
    PreprocessedTranslationUnit,
    /// Analyze only functions whose definitions occur in the input source.
    MainSourceDefinitions,
}

/// Per-definition coverage carried by a whole-unit certificate.
///
/// A `Safe` certificate currently admits only [`Self::AnalyzedSafe`]. The
/// other variants make an excluded or unresolved row explicit if a future
/// deliberately partial scope needs to retain it; certificate replay refuses
/// either variant rather than silently removing it from the denominator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyCertificateCoverage {
    AnalyzedSafe,
    Excluded { reason: String },
    Unknown { reason: String },
}

/// One member of the certificate's complete selected target inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCertificateDefinition {
    pub name: String,
    pub coverage: LegacyCertificateCoverage,
}

/// The source artifacts from which an independent checker reconstructs the
/// selected target denominator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyCertificateScopeEvidence {
    /// Every definition in the authoritative preprocessed source is selected.
    PreprocessedTranslationUnit { raw_source: String },
    /// Main-source selection is reconstructed from the raw source and the
    /// line-marked preprocessing whose include-depth flags decide attribution.
    MainSourceDefinitions {
        raw_source: String,
        line_marked_source: String,
    },
}

/// Effective input identity bound into the certificate scope. These values are
/// recomputed from retained artifacts during replay; none is trusted as a
/// caller assertion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCertificateInputIdentity {
    pub raw_source_sha256: String,
    pub preprocessed_source_sha256: String,
    pub scope_evidence_sha256: String,
    pub analysis_request_sha256: String,
    pub dialect: CCompilerDialect,
    pub function_scope: LegacyFunctionScope,
}

/// Owned gateway request fields that can affect preprocessing, parsing or
/// typed interpretation. Retaining them lets replay recompute the request
/// digest. Compiler executable/version, target headers and filesystem
/// provenance remain the external `CompileTaskIdentity` boundary; this record
/// does not claim to replace full build provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCertificateRequest {
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    pub preprocessor_options: Vec<CPreprocessorOption>,
    pub input: CSourceInput,
}

/// The complete intended target scope and its coverage denominator.
///
/// `inventory_sha256` is a stable digest of the canonical definition rows.
/// Replay first reconstructs those rows independently from `evidence`, then
/// compares both the rows and digest. A reduced, duplicated or relabelled
/// inventory therefore cannot become a smaller `Safe` claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCertificateScope {
    pub definitions: Vec<LegacyCertificateDefinition>,
    pub inventory_sha256: String,
    pub input_identity: LegacyCertificateInputIdentity,
    pub request: LegacyCertificateRequest,
    pub evidence: LegacyCertificateScopeEvidence,
}

/// Caller-held expectation for the artifact, target denominator and scope a
/// certificate is allowed to describe. A checker that needs to authenticate
/// the intended artifact target supplies this independently of the certificate;
/// copying it out of the certificate proves only structural self-consistency.
/// Compiler/build provenance and the caller-intended contract/profile trust
/// context remain external to this target-scope record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCertificateExpectedContext {
    pub inventory_sha256: String,
    pub input_identity: LegacyCertificateInputIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacySafetyCertificate {
    pub preprocessed_source: String,
    pub entry: String,
    pub contracts: ResourceContracts,
    pub dialect: CCompilerDialect,
    pub function_scope: LegacyFunctionScope,
    pub summary: LegacyProgramSummary,
    /// Complete selected target inventory, independently reconstructible from
    /// the retained scope evidence.
    pub target_scope: LegacyCertificateScope,
    /// Compatibility index of analysed rows. Replay derives this from
    /// `target_scope`; it is never accepted as the coverage denominator.
    pub checked_functions: Vec<String>,
    /// Interprocedural custody summaries the analysis relied on (W4). Only
    /// [`ContractTrust::InferredValidated`] records appear here; declared
    /// contracts already travel in `contracts`. The recheck re-derives this
    /// list independently and refuses a certificate that does not reproduce it.
    pub summaries: Vec<ResourceSummaryRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnrestrictedLegacyResult {
    Safe(LegacySafetyCertificate),
    Unsafe(LegacyProgramSummary),
    Unknown {
        preprocessed_source: String,
        reason: String,
        partial: Option<LegacyProgramSummary>,
    },
}

fn digest_field(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn certificate_inventory_sha256(rows: &[LegacyCertificateDefinition]) -> String {
    let mut bytes = b"qcore.legacy-c.certificate-scope.v1\0".to_vec();
    for row in rows {
        digest_field(&mut bytes, row.name.as_bytes());
        match &row.coverage {
            LegacyCertificateCoverage::AnalyzedSafe => bytes.push(0),
            LegacyCertificateCoverage::Excluded { reason } => {
                bytes.push(1);
                digest_field(&mut bytes, reason.as_bytes());
            }
            LegacyCertificateCoverage::Unknown { reason } => {
                bytes.push(2);
                digest_field(&mut bytes, reason.as_bytes());
            }
        }
    }
    qcore_schemas::sha256_hex(&bytes)
}

fn certificate_input_identity(
    dialect: CCompilerDialect,
    function_scope: LegacyFunctionScope,
    preprocessed_source: &str,
    evidence: &LegacyCertificateScopeEvidence,
    request: &LegacyCertificateRequest,
) -> LegacyCertificateInputIdentity {
    let (raw, scope_evidence) = match evidence {
        LegacyCertificateScopeEvidence::PreprocessedTranslationUnit { raw_source } => {
            (raw_source.as_str(), preprocessed_source)
        }
        LegacyCertificateScopeEvidence::MainSourceDefinitions {
            raw_source,
            line_marked_source,
        } => (raw_source.as_str(), line_marked_source.as_str()),
    };
    let mut request_bytes = serde_json::to_vec(&(
        &request.include_paths,
        &request.definitions,
        &request.preprocessor_options,
        &request.input,
    ))
    .expect("certificate request fields are serializable");
    request_bytes.push(match dialect {
        CCompilerDialect::C11 => 0,
        CCompilerDialect::Gnu99 => 1,
        CCompilerDialect::Gnu11 => 2,
        CCompilerDialect::Gnu17 => 3,
    });
    request_bytes.push(match function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => 0,
        LegacyFunctionScope::MainSourceDefinitions => 1,
    });
    LegacyCertificateInputIdentity {
        raw_source_sha256: qcore_schemas::sha256_hex(raw.as_bytes()),
        preprocessed_source_sha256: qcore_schemas::sha256_hex(preprocessed_source.as_bytes()),
        scope_evidence_sha256: qcore_schemas::sha256_hex(scope_evidence.as_bytes()),
        analysis_request_sha256: qcore_schemas::sha256_hex(&request_bytes),
        dialect,
        function_scope,
    }
}

fn certificate_request(unit: &C11TranslationUnit<'_>) -> LegacyCertificateRequest {
    LegacyCertificateRequest {
        include_paths: unit.include_paths.clone(),
        definitions: unit.definitions.clone(),
        preprocessor_options: unit.preprocessor_options.clone(),
        input: unit.input.clone(),
    }
}

fn preprocessed_definition_names(source: &str) -> Result<Vec<String>, String> {
    let scan = function_names(&tokenize(source), &[]);
    if let Some(refusal) = scan.refusals().first() {
        return Err(format!(
            "preprocessed target inventory cannot be established: {}",
            refusal.reason()
        ));
    }
    Ok(scan.definitions().to_vec())
}

fn build_certificate_scope(
    unit: &C11TranslationUnit<'_>,
    preprocessed_source: &str,
) -> Result<LegacyCertificateScope, String> {
    let marked = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(
            enumeration::preprocess_with_line_markers(unit)
                .map_err(|error| format!("certificate scope preprocessing failed: {error}"))?,
        ),
    };
    build_certificate_scope_from_evidence(
        unit.dialect,
        unit.function_scope,
        preprocessed_source,
        Some(unit.source),
        marked.as_deref(),
        certificate_request(unit),
    )
}

fn build_certificate_scope_from_evidence(
    dialect: CCompilerDialect,
    function_scope: LegacyFunctionScope,
    preprocessed_source: &str,
    raw_source: Option<&str>,
    line_marked_source: Option<&str>,
    request: LegacyCertificateRequest,
) -> Result<LegacyCertificateScope, String> {
    let (names, evidence) = match function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => {
            let raw = raw_source.ok_or_else(|| {
                "preprocessed-unit certificate scope carries no raw source evidence".to_owned()
            })?;
            (
                preprocessed_definition_names(preprocessed_source)?,
                LegacyCertificateScopeEvidence::PreprocessedTranslationUnit {
                    raw_source: raw.to_owned(),
                },
            )
        }
        LegacyFunctionScope::MainSourceDefinitions => {
            let raw = raw_source.ok_or_else(|| {
                "main-source certificate scope carries no raw source evidence".to_owned()
            })?;
            let marked = line_marked_source.ok_or_else(|| {
                "main-source certificate scope carries no line-marked evidence".to_owned()
            })?;
            let line_map = location::map_marked_preprocessing(&marked, preprocessed_source);
            if let Some(refusal) = line_map.refusals().first() {
                return Err(format!(
                    "certificate scope evidence does not match the analyzed input: {}",
                    refusal.reason()
                ));
            }
            let inventory = enumeration::source_definition_inventory_from_evidence(raw, marked);
            if !inventory.supports_unit_safe() {
                return Err(inventory
                    .withheld_safe_reason()
                    .unwrap_or_else(|| "certificate target inventory is incomplete".to_owned()));
            }
            (
                inventory.definitions().to_vec(),
                LegacyCertificateScopeEvidence::MainSourceDefinitions {
                    raw_source: raw.to_owned(),
                    line_marked_source: marked.to_owned(),
                },
            )
        }
    };
    let definitions = names
        .into_iter()
        .map(|name| LegacyCertificateDefinition {
            name,
            coverage: LegacyCertificateCoverage::AnalyzedSafe,
        })
        .collect::<Vec<_>>();
    Ok(LegacyCertificateScope {
        inventory_sha256: certificate_inventory_sha256(&definitions),
        input_identity: certificate_input_identity(
            dialect,
            function_scope,
            preprocessed_source,
            &evidence,
            &request,
        ),
        request,
        definitions,
        evidence,
    })
}

/// Independently derive the expected artifact/scope context from the intended
/// translation unit. Persist this beside the audit request (or derive it again)
/// and pass it to [`check_legacy_safety_certificate_for`] when the intended
/// scope, rather than certificate self-consistency alone, must be authenticated.
pub fn legacy_certificate_expected_context(
    unit: &C11TranslationUnit<'_>,
) -> Result<LegacyCertificateExpectedContext, LegacyCError> {
    let preprocessed_source = validate_unrestricted_c11(unit)?;
    let scope = build_certificate_scope(unit, &preprocessed_source).map_err(|reason| {
        LegacyCError::ProductionParserRejected(format!(
            "certificate target scope could not be established: {reason}"
        ))
    })?;
    Ok(LegacyCertificateExpectedContext {
        inventory_sha256: scope.inventory_sha256,
        input_identity: scope.input_identity,
    })
}

impl fmt::Display for LegacyCError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FunctionNotFound(function) => write!(f, "C function `{function}` was not found"),
            Self::MalformedFunction { function, line } => {
                write!(
                    f,
                    "could not project body for C function `{function}` at input line {line}"
                )
            }
            Self::ProductionParserUnavailable(message) => {
                write!(f, "production C parser is unavailable: {message}")
            }
            Self::ProductionParserRejected(message) => {
                write!(
                    f,
                    "production C parser rejected the translation unit: {message}"
                )
            }
            Self::UnsupportedInputIdentity(message) => {
                write!(
                    f,
                    "translation unit input identity cannot answer a \
                     filename-sensitive construct: {message}"
                )
            }
        }
    }
}

impl std::error::Error for LegacyCError {}

/// Parse, import to a resource CFG, convert assignments to SSA-like strong
/// updates, and solve the finite may-alias/resource-state equations.
pub fn analyze_legacy_c(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let tokens = tokenize(source);
    analyze_legacy_c_tokens(&tokens, function, contracts)
}

fn analyze_legacy_c_tokens(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    // Card LOADER-ABI. This single-function entry point does not pass through
    // the translation-unit pass, so it applies the withdrawal itself: a
    // declaration this unit's own C declaration contradicts is not applied
    // here either. The whole-unit routes gate before the derivation is seeded
    // (`analyze_legacy_c_translation_unit_selected_partial`), and every
    // `build_function_cfg` caller below that point is handed the tokens and
    // the environment that pass already withdrew from — which is also why the
    // gate is not repeated per function, where re-reading every declared
    // module would cost one parse per body.
    let agreement = declared_abi::AbiAgreement::read(tokens, contracts);
    let withdrawn_environment = agreement.withdraw_environment(contracts);
    let contracts = withdrawn_environment.as_ref().unwrap_or(contracts);
    let withdrawn_tokens = agreement.withdraw_call_sites(tokens);
    let tokens = withdrawn_tokens.as_deref().unwrap_or(tokens);
    // Card LOADER-ABI-MODES, applied here for the same uniformity reason: this
    // entry point does not pass through the translation-unit pass, so a
    // declaration a body in the same stream contradicts would otherwise still
    // govern this function's call sites. The definitions are read with no
    // conditional marks because this entry's own stream carries none.
    let names = function_names(tokens, &[]).definitions;
    let gate = declared_modes::ModeGate::apply(tokens, &names, contracts);
    let tokens = gate.tokens.as_deref().unwrap_or(tokens);
    Ok(analyze_cfg(build_function_cfg(
        tokens, function, contracts,
    )?))
}

/// Gate the conservative importer with Clang's production C11 parser.  The
/// typed AST accepted by Clang is still deliberately projected into the small
/// resource language below; any projection gap remains `Unknown`.
pub fn analyze_legacy_c11(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let mut child = crate::clang::command()
        .map_err(LegacyCError::ProductionParserUnavailable)?
        .args(["-x", "c", "-std=c11", "-fsyntax-only", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
        .write_all(source.as_bytes())
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    let output = child
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    analyze_legacy_c(source, function, contracts)
}

/// Preprocess a conforming C11 translation unit with Clang, then project it
/// through the conservative resource analyzer.  Projection gaps are an
/// `Unknown` verdict, not a parse failure and never a `Safe` certificate.
pub fn analyze_unrestricted_c11(
    unit: &C11TranslationUnit<'_>,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let inventory = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(enumerate_source_definitions(unit)),
    };
    analyze_unrestricted_c11_with_inventory(unit, entry, contracts, inventory.as_ref())
}

/// [`analyze_unrestricted_c11`] over an inventory the caller already
/// enumerated (unit **E01-F1**).
///
/// A driver that reports the denominator must enumerate it anyway, and the
/// enumeration costs one Clang preprocessing; this entry point lets the two
/// share it, and — more importantly — guarantees the analyzer's selection is
/// *the same inventory the report publishes*, rather than a second, possibly
/// divergent, computation of it.
///
/// `inventory` is `None` exactly when the unit's scope is
/// [`LegacyFunctionScope::PreprocessedTranslationUnit`], where there is no
/// main-source restriction to apply.
pub fn analyze_unrestricted_c11_with_inventory(
    unit: &C11TranslationUnit<'_>,
    entry: &str,
    contracts: &ResourceContracts,
    inventory: Option<&SourceDefinitionInventory>,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let source_functions = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(inventory.map_or_else(
            || enumerate_source_definitions(unit).definitions().to_vec(),
            |inventory| inventory.definitions().to_vec(),
        )),
    };
    #[cfg(feature = "indirect-census")]
    crate::indirect_census::next_unit();
    let preprocessed_source = validate_unrestricted_c11(unit)?;
    let line_marked_source = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => {
            enumeration::preprocess_with_line_markers(unit).ok()
        }
    };
    analyze_preprocessed_unrestricted_c11(
        preprocessed_source,
        unit.dialect,
        unit.function_scope,
        unit.source.to_owned(),
        line_marked_source,
        certificate_request(unit),
        entry,
        contracts,
        source_functions,
    )
}

/// Analyze one unit from the immutable preprocessing, syntax, inventory and
/// coordinate preparation already shared by the audit driver.
pub fn analyze_prepared_translation_unit(
    prepared: &PreparedTranslationUnit,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let preprocessed_source = prepared
        .preprocessed_source()
        .map_err(Clone::clone)?
        .to_owned();
    let source_functions = match prepared.function_scope() {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => {
            Some(prepared.inventory().definitions().to_vec())
        }
    };
    #[cfg(feature = "indirect-census")]
    crate::indirect_census::next_unit();
    analyze_preprocessed_unrestricted_c11(
        preprocessed_source,
        prepared.dialect(),
        prepared.function_scope(),
        prepared.raw_source().to_owned(),
        prepared.line_marked_source().map(str::to_owned),
        LegacyCertificateRequest {
            include_paths: prepared.include_paths().to_vec(),
            definitions: prepared.definitions().to_vec(),
            preprocessor_options: prepared.preprocessor_options().to_vec(),
            input: prepared.input().clone(),
        },
        entry,
        contracts,
        source_functions,
    )
}

fn analyze_preprocessed_unrestricted_c11(
    preprocessed_source: String,
    dialect: CCompilerDialect,
    function_scope: LegacyFunctionScope,
    raw_source: String,
    line_marked_source: Option<String>,
    request: LegacyCertificateRequest,
    entry: &str,
    contracts: &ResourceContracts,
    source_functions: Option<Vec<String>>,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let analysis = match analyze_legacy_c_translation_unit_selected_partial(
        &preprocessed_source,
        entry,
        contracts,
        source_functions.as_deref(),
    ) {
        Ok(summary) => summary,
        Err(error) => {
            return Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source,
                reason: error.to_string(),
                partial: None,
            });
        }
    };
    let summary = analysis.summary;
    let summaries = analysis.summaries;
    if !analysis.projection_failures.is_empty() {
        let failures = analysis
            .projection_failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason: format!("some source functions could not be projected: {failures}"),
            partial: Some(summary),
        });
    }
    match summary.status {
        AnalysisStatus::Unsafe => Ok(UnrestrictedLegacyResult::Unsafe(summary)),
        AnalysisStatus::Unknown => Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason: unresolved_projection_reason(&summary),
            partial: Some(summary),
        }),
        AnalysisStatus::Safe => {
            // Fail-closed (unit **E01-TAIL**, residual E01-F1-DENOM). A `Safe`
            // verdict here is a claim over the *selected scope* — the
            // enumerated main-source definition set. A selected definition that
            // carries no analysed row was never looked at, and hard invariant 2
            // says projection coverage is not safety coverage: publishing
            // `Safe` over it would assert safety for a function the analysis
            // never read.
            //
            // Such a definition reaches here silently, because
            // `analyze_legacy_c_translation_unit_selected_partial` intersects
            // the selection with the definitions the analyzer's own body finder
            // reports and simply drops the rest — no diagnostic, no projection
            // failure. Two measured causes: a definition the enumeration's raw
            // half kept but the preprocessor removed (a `#if 0` body — it is
            // not in this translation unit at all), and a definition present in
            // the preprocessed text that the body finder does not classify as
            // one (a K&R body is the named instance; an unnamed one would be an
            // analyzer blind spot).
            //
            // The two are indistinguishable from here, and the report's own
            // `not_projected_meaning` already conflates them, so this refuses
            // both rather than excusing either. Only the `Safe` arm is gated:
            // `Unsafe` still names its validated reachable violation and
            // `Unknown` is already the fail-closed answer, so neither moves.
            //
            // Card **E01-TAIL-CERT-SCOPE** extends this issuance gate through
            // replay: the certificate retains the exact evidence from which
            // the selected denominator is reconstructed, and the checker never
            // uses `checked_functions` as its target set.
            if let Some(selected) = source_functions.as_deref() {
                let unread = selected
                    .iter()
                    .filter(|name| !summary.functions.contains_key(*name))
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(first) = unread.first() {
                    return Ok(UnrestrictedLegacyResult::Unknown {
                        preprocessed_source,
                        reason: format!(
                            "{} of {} selected source definition(s) carry no analysed row, so no whole-unit Safe claim is published over them; first: `{first}`",
                            unread.len(),
                            selected.len(),
                        ),
                        partial: Some(summary),
                    });
                }
            }
            let target_scope = match build_certificate_scope_from_evidence(
                dialect,
                function_scope,
                &preprocessed_source,
                Some(&raw_source),
                line_marked_source.as_deref(),
                request,
            ) {
                Ok(scope) => scope,
                Err(reason) => {
                    return Ok(UnrestrictedLegacyResult::Unknown {
                        preprocessed_source,
                        reason: format!(
                            "certificate target scope could not be established: {reason}"
                        ),
                        partial: Some(summary),
                    });
                }
            };
            let checked_functions = summary.functions.keys().cloned().collect::<Vec<_>>();
            let certificate = LegacySafetyCertificate {
                preprocessed_source,
                entry: summary.entry.clone(),
                contracts: contracts.clone(),
                dialect,
                function_scope,
                summary,
                target_scope,
                checked_functions,
                summaries,
            };
            check_legacy_safety_certificate(&certificate).map_err(|reason| {
                LegacyCError::ProductionParserRejected(format!(
                    "internal legacy-C certificate check failed: {reason}"
                ))
            })?;
            Ok(UnrestrictedLegacyResult::Safe(certificate))
        }
    }
}

fn unresolved_projection_reason(summary: &LegacyProgramSummary) -> String {
    let mut unsupported = 0;
    let mut aliases = 0;
    let mut conflicts = 0;
    for diagnostic in summary
        .functions
        .values()
        .flat_map(|function| &function.diagnostics)
    {
        match diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { .. } => unsupported += 1,
            LegacyDiagnosticKind::UnknownAlias { .. } => aliases += 1,
            LegacyDiagnosticKind::SummaryConflict { .. } => conflicts += 1,
            _ => {}
        }
    }
    format!(
        "resource projection unresolved: {unsupported} unsupported operation(s), \
         {aliases} unresolved alias(es), {conflicts} custody summary conflict(s)"
    )
}

/// Preprocess and syntax-check an unrestricted C translation unit without
/// requiring it to contain an executable function body.
pub fn validate_unrestricted_c11(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    preparation::validate_plain_and_syntax(unit).map(|(source, _, _)| source)
}

/// Independent structural check for an unrestricted-C `Safe` result.
pub fn check_legacy_safety_certificate(
    certificate: &LegacySafetyCertificate,
) -> Result<(), String> {
    crate::contract_loader::check_source_assumptions(&certificate.contracts)?;
    let target_names = match (
        &certificate.function_scope,
        &certificate.target_scope.evidence,
    ) {
        (
            LegacyFunctionScope::PreprocessedTranslationUnit,
            LegacyCertificateScopeEvidence::PreprocessedTranslationUnit { .. },
        ) => preprocessed_definition_names(&certificate.preprocessed_source)?,
        (
            LegacyFunctionScope::MainSourceDefinitions,
            LegacyCertificateScopeEvidence::MainSourceDefinitions {
                raw_source,
                line_marked_source,
            },
        ) => {
            let line_map = location::map_marked_preprocessing(
                line_marked_source,
                &certificate.preprocessed_source,
            );
            if let Some(refusal) = line_map.refusals().first() {
                return Err(format!(
                    "certificate scope evidence does not match the analyzed input: {}",
                    refusal.reason()
                ));
            }
            let inventory = enumeration::source_definition_inventory_from_evidence(
                raw_source,
                line_marked_source,
            );
            if !inventory.supports_unit_safe() {
                return Err(inventory
                    .withheld_safe_reason()
                    .unwrap_or_else(|| "certificate target inventory is incomplete".to_owned()));
            }
            inventory.definitions().to_vec()
        }
        _ => return Err("certificate scope evidence does not match its scope label".into()),
    };
    let expected_rows = target_names
        .iter()
        .cloned()
        .map(|name| LegacyCertificateDefinition {
            name,
            coverage: LegacyCertificateCoverage::AnalyzedSafe,
        })
        .collect::<Vec<_>>();
    if certificate.target_scope.definitions != expected_rows {
        return Err(
            "certificate target inventory or per-definition coverage does not match its scope evidence"
                .into(),
        );
    }
    if certificate.target_scope.inventory_sha256
        != certificate_inventory_sha256(&certificate.target_scope.definitions)
    {
        return Err("certificate target inventory digest does not match its rows".into());
    }
    let selected = match certificate.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(target_names.as_slice()),
    };
    let recomputed = analyze_legacy_c_translation_unit_selected_partial(
        &certificate.preprocessed_source,
        &certificate.entry,
        &certificate.contracts,
        selected,
    )
    .map_err(|error| format!("certificate source cannot be reanalyzed: {error}"))?;
    if let Some(error) = recomputed.projection_failures.first() {
        return Err(format!("certificate source cannot be reanalyzed: {error}"));
    }
    // The recheck re-derives the interprocedural custody summaries from the
    // preprocessed source and the declared contracts alone. A certificate that
    // leans on a summary this pass cannot reproduce fails containment; a
    // summary is never taken on the certificate's word.
    if recomputed.summaries != certificate.summaries {
        return Err("inferred custody summaries do not match the preprocessed source".into());
    }
    if certificate
        .summaries
        .iter()
        .any(|record| record.trust != ContractTrust::InferredValidated)
    {
        return Err("certificate carries a custody summary that was not validated".into());
    }
    if recomputed.summary != certificate.summary {
        return Err("program summary does not match the preprocessed source".into());
    }
    let expected_identity = certificate_input_identity(
        certificate.dialect,
        certificate.function_scope,
        &certificate.preprocessed_source,
        &certificate.target_scope.evidence,
        &certificate.target_scope.request,
    );
    if certificate.target_scope.input_identity != expected_identity {
        return Err("certificate effective input identity does not match its artifacts".into());
    }
    if certificate.summary.status != AnalysisStatus::Safe {
        return Err("program summary is not Safe".into());
    }
    let names = certificate
        .summary
        .functions
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    if names != target_names {
        return Err("program summary does not cover the complete certificate target scope".into());
    }
    if certificate.checked_functions != names {
        return Err("checked function index is not canonical".into());
    }
    for (name, function) in &certificate.summary.functions {
        if function.status != AnalysisStatus::Safe || !function.diagnostics.is_empty() {
            return Err(format!("function `{name}` is not independently Safe"));
        }
        if function
            .cfg
            .nodes
            .iter()
            .any(|node| matches!(node.instruction, LegacyInstruction::Unknown { .. }))
        {
            return Err(format!("function `{name}` retains an unknown instruction"));
        }
    }
    Ok(())
}

/// Recheck a certificate and bind it to a caller-held intended audit context.
///
/// The ordinary recheck proves that the certificate is internally consistent
/// with its retained source evidence. This stronger entry point additionally
/// prevents a coherent replacement of that evidence, inventory and digest
/// from being accepted as the caller's original target.
pub fn check_legacy_safety_certificate_for(
    certificate: &LegacySafetyCertificate,
    expected: &LegacyCertificateExpectedContext,
) -> Result<(), String> {
    check_legacy_safety_certificate(certificate)?;
    if certificate.target_scope.inventory_sha256 != expected.inventory_sha256 {
        return Err("certificate target denominator differs from the expected audit scope".into());
    }
    if certificate.target_scope.input_identity != expected.input_identity {
        return Err(
            "certificate effective input identity differs from the expected audit input".into(),
        );
    }
    Ok(())
}

/// Analyze every function definition in a translation unit and combine the
/// per-function results into a bounded interprocedural status summary.
/// Internal calls are resource-neutral at the call site, while the callee's
/// own resource behavior is checked separately and contributes to the result.
pub fn analyze_legacy_c_translation_unit(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyProgramSummary, LegacyCError> {
    analyze_legacy_c_translation_unit_selected(source, entry, contracts, None)
}

fn analyze_legacy_c_translation_unit_selected(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
) -> Result<LegacyProgramSummary, LegacyCError> {
    let analysis = analyze_legacy_c_translation_unit_selected_partial(
        source,
        entry,
        contracts,
        selected_functions,
    )?;
    if let Some(error) = analysis.projection_failures.into_iter().next() {
        return Err(error);
    }
    Ok(analysis.summary)
}

/// Run the production derivation over one preprocessed translation unit and
/// return only its recursive-component census (card C03-REACH).
///
/// This shares [`derive_custody_summaries`] with the analyzer, so the census
/// reads the same components, the same environment and the same route
/// decisions the audit does. It performs no per-function analysis and produces
/// no summary, certificate or verdict.
pub(crate) fn recursive_component_census(
    source: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
) -> Vec<crate::legacy_census::NeutralComponentCensus> {
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    let agreement = declared_abi::AbiAgreement::read(&tokens, contracts);
    let withdrawn_environment = agreement.withdraw_environment(contracts);
    let contracts = withdrawn_environment.as_ref().unwrap_or(contracts);
    let withdrawn_tokens = agreement.withdraw_call_sites(&tokens);
    let tokens = withdrawn_tokens.unwrap_or(tokens);
    let mut names = function_names(&tokens, &conditionals).definitions;
    if let Some(selected) = selected_functions {
        let selected = selected.iter().collect::<BTreeSet<_>>();
        names.retain(|name| selected.contains(name));
    }
    // Card LOADER-ABI-MODES, for the same reason the type gate runs here: the
    // census must read the components the analysis reads, over the stream the
    // analysis reads.
    declared_modes::ModeGate::apply(&tokens, &names, contracts)
        .derivation
        .census()
}

pub(crate) struct PartialTranslationUnitAnalysis {
    pub(crate) summary: LegacyProgramSummary,
    pub(crate) projection_failures: Vec<LegacyCError>,
    /// Validated interprocedural custody summaries used at call sites.
    pub(crate) summaries: Vec<ResourceSummaryRecord>,
}

fn analyze_legacy_c_translation_unit_selected_partial(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
) -> Result<PartialTranslationUnitAnalysis, LegacyCError> {
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    // Card LOADER-ABI. A declared contract whose parameter/result ABI
    // projections this unit's own C declaration contradicts is withdrawn here,
    // before anything reads either: the call sites are rewritten to a marked
    // uncontracted callee, and the name leaves the environment the derivation
    // seeds, so no defined body is checked against — and no caller trusts — an
    // interface the source contradicts. Both halves are decided from this
    // token stream and the environment's own retained texts, so W4 derivation,
    // W4 validation and `check_legacy_safety_certificate` reproduce them.
    let agreement = declared_abi::AbiAgreement::read(&tokens, contracts);
    let withdrawn_environment = agreement.withdraw_environment(contracts);
    let contracts = withdrawn_environment.as_ref().unwrap_or(contracts);
    let withdrawn_tokens = agreement.withdraw_call_sites(&tokens);
    let tokens = withdrawn_tokens.unwrap_or(tokens);
    let mut names = function_names(&tokens, &conditionals).definitions;
    let requested_was_selected =
        selected_functions.is_some_and(|selected| selected.iter().any(|name| name == entry));
    if let Some(selected) = selected_functions {
        let selected = selected.iter().collect::<BTreeSet<_>>();
        names.retain(|name| selected.contains(name));
    }
    let effective_entry = if names.iter().any(|name| name == entry) {
        entry.to_owned()
    } else if requested_was_selected {
        names
            .first()
            .cloned()
            .ok_or_else(|| LegacyCError::FunctionNotFound(entry.to_owned()))?
    } else {
        return Err(LegacyCError::FunctionNotFound(entry.to_owned()));
    };
    // A retained callee summary does not prove neutral parameter/return
    // custody. Explicit trusted contracts are preserved; a call is modeled only
    // through such a contract or through an inferred summary that this pass
    // validated by re-running the intraprocedural analyzer against it (W4).
    // Everything else stays an uncontracted call and therefore `Unknown`.
    // Card LOADER-ABI-MODES. LOADER-ABI compared the declared *types* against
    // this unit's C declaration; this gate compares the declared *modes*
    // against the validated summary W4 derives from the function's own body
    // here. A measured contradiction — a declared `Borrowed` the body releases,
    // a declared `Released` the body never performs, a declared absent result
    // the body produces — withdraws the declaration's application at every call
    // site in this unit. The environment is deliberately left alone: it is what
    // makes the derivation record the `SummaryConflict` that already keeps the
    // defining function `Unknown`, so keeping it is strictly more refusing.
    // The gate hands back the derivation it settled on, so a unit with no
    // contradiction derives exactly once, as before this card.
    let gate = declared_modes::ModeGate::apply(&tokens, &names, contracts);
    let tokens = gate.tokens.unwrap_or(tokens);
    let derivation = gate.derivation;
    let mut functions = BTreeMap::new();
    let mut projection_failures = Vec::new();
    for name in names {
        match analyze_defined_function(&tokens, &name, &derivation) {
            Ok(summary) => {
                functions.insert(name, summary);
            }
            Err(error @ LegacyCError::MalformedFunction { .. }) => {
                projection_failures.push(error);
            }
            Err(error) => return Err(error),
        }
    }
    let status = if functions
        .values()
        .any(|summary| summary.status == AnalysisStatus::Unsafe)
    {
        AnalysisStatus::Unsafe
    } else if !projection_failures.is_empty()
        || functions
            .values()
            .any(|summary| summary.status == AnalysisStatus::Unknown)
    {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    };
    Ok(PartialTranslationUnitAnalysis {
        summary: LegacyProgramSummary {
            entry: effective_entry,
            status,
            functions,
        },
        projection_failures,
        summaries: derivation.records(),
    })
}

/// Analyze one defined function under the derived environment.
///
/// The entry state comes from the contract in force for the function itself —
/// explicit if one was declared, otherwise the validated inferred summary — so
/// that a callee's own body is checked against the same custody interface its
/// callers use. If the body does not validate against that interface the
/// function falls back to the contract-free analysis, which makes no custody
/// assumption about the formals at all.
fn analyze_defined_function(
    tokens: &[Token],
    function: &str,
    derivation: &CustodyDerivation,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let cfg = match derivation.contextual_cfg(function) {
        Some(cfg) => cfg.clone(),
        None => build_function_cfg(tokens, function, derivation.environment())?,
    };
    let mut summary = match derivation.entry_contract(function) {
        Some(contract) => match function_parameters(tokens, function) {
            Some(parameters) => match EntryCustody::build(&cfg, &parameters, contract) {
                Some(entry) => {
                    let (candidate, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
                    if crate::legacy_summary::observations_match_contract(
                        contract,
                        &entry,
                        &candidate,
                        &observations,
                    ) {
                        candidate
                    } else {
                        analyze_cfg(cfg)
                    }
                }
                None => analyze_cfg(cfg),
            },
            None => analyze_cfg(cfg),
        },
        None => analyze_cfg(cfg),
    };
    if let Some((inferred, declared)) = derivation.conflict(function) {
        let line = find_function_body(tokens, function).map_or(0, |(_, line)| line);
        summary.diagnostics.push(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::SummaryConflict {
                inferred: Box::new(inferred.clone()),
                declared: Box::new(declared.clone()),
            },
        });
        summary.diagnostics.sort();
        summary.diagnostics.dedup();
        summary.status = classify_diagnostics(&summary.diagnostics);
        // A summary that contradicts the declared contract invalidates every
        // per-exit claim derived under it, including the exits the conflicting
        // interface never reached.
        for exit in &mut summary.exits {
            exit.custody = ExitCustody::Unknown;
        }
    }
    Ok(summary)
}

/// Parse one function body into its resource CFG under `contracts`.
pub(crate) fn build_function_cfg(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyCfg, LegacyCError> {
    // Card HASH-CONTRACTS. A declared literal-argument premise restricts a
    // contract to the call sites whose named argument is the named literal;
    // every other call is rewritten to an undeclared callee, so the restriction
    // holds uniformly at every reader below — the out-alias normalizer, the
    // typed access facts, the C02-P and aggregate-store admissions and the
    // resource CFG — rather than at whichever contract lookups remembered it.
    // The premises come from the environment's own declared module texts, so
    // W4 derivation, W4 validation and certificate rederivation, which each
    // hold exactly that text, reproduce this rewrite identically.
    let premises = literal_premise::LiteralPremises::read(contracts);
    let restricted = literal_premise::restrict_to_declared_literals(tokens, contracts, &premises);
    let tokens = restricted.as_deref().unwrap_or(tokens);
    let normalized = out_alias::normalize(tokens, function, contracts);
    let tokens = normalized.as_deref().unwrap_or(tokens);
    let (body_start, line) = find_function_body(tokens, function)
        .ok_or_else(|| LegacyCError::FunctionNotFound(function.to_owned()))?;
    let parameters = function_parameters(tokens, function).unwrap_or_default();
    let formal_parameters = parameters.iter().flatten().cloned().collect();
    let protected_formals =
        protected_resource_formals(tokens, body_start, function, &parameters, contracts);
    let variadic_tails = declared_variadic_tails(contracts);
    let (typed_model, typed_types, typed_bases, store_bases, publications, frame_protected) =
        typed_access_facts(
            tokens,
            function,
            body_start,
            &parameters,
            &protected_formals,
            contracts,
            &variadic_tails,
        );
    let alias_classes = crate::may_alias::MayAliasClasses::read(&typed_types);
    let mut parser = Parser {
        tokens,
        cursor: body_start,
        contracts,
        next_site: 0,
        formal_parameters,
        protected_formals,
        typed_model,
        typed_types,
        typed_bases,
        store_bases,
        alias_classes,
        variadic_tails,
        frame_protected,
        #[cfg(feature = "indirect-census")]
        census_function: function.to_owned(),
    };
    let statements =
        parser
            .parse_block()
            .map_err(|failure_line| LegacyCError::MalformedFunction {
                function: function.to_owned(),
                line: failure_line.max(line),
            })?;
    let mut cfg = CfgBuilder::build(function, &statements, publications);
    if let Some(body_end) = matching(tokens, body_start, "{", "}") {
        let body = tokens[body_start + 1..body_end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let unique = crate::typed_field_access::unique_local_names(&parser.typed_model, &body);
        cfg.conditional_names = unique
            .intersection(&parser.frame_protected)
            .filter(|name| !parser.formal_parameters.contains(*name))
            .cloned()
            .collect();
    }
    Ok(cfg)
}

/// The callees `contracts` declares a variadic data tail for (card O04-TRACE).
///
/// The tail is not carried on `ResourceFunctionContract`; it is rederived from
/// the environment's own retained trusted texts, which is what makes W4 summary
/// derivation and [`check_legacy_safety_certificate`] reproduce it from the
/// same bytes. Two fail-closed properties hold by construction: an environment
/// with no declared sources has no tails, and a source set the loader rejects
/// yields the empty set rather than a partial one.
fn declared_variadic_tails(contracts: &ResourceContracts) -> BTreeSet<String> {
    // A source that does not contain the clause's spelling cannot declare it,
    // so the common case costs a scan rather than a parse.
    if !contracts
        .source_assumptions()
        .values()
        .any(|source| source.contains("resource_variadic_data_tail"))
    {
        return BTreeSet::new();
    }
    qcore_contract::contract_loader::declared_variadic_data_tails(contracts).unwrap_or_default()
}

/// Read the declaration facts the typed direct scalar-field whitelist needs.
///
/// Both the record inventory and the function's variable types come from the
/// same preprocessed token stream the importer is already projecting, so the
/// classification is reproduced exactly by W4 summary derivation and by
/// [`check_legacy_safety_certificate`], which hold only that stream. When the
/// function's declarator list cannot be read, [`FunctionTypes::default`]
/// refuses every access in it and the lexical gate governs unchanged.
fn typed_access_facts(
    tokens: &[Token],
    function: &str,
    body_start: usize,
    parameters: &[Option<String>],
    protected_formals: &BTreeSet<String>,
    contracts: &ResourceContracts,
    variadic_tails: &BTreeSet<String>,
) -> (
    TypedAccessModel,
    FunctionTypes,
    UntrackedBases,
    AggregateStoreBases,
    PublicationFacts,
    BTreeSet<String>,
) {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let model = TypedAccessModel::read(&texts);
    let body = match (
        function_parameter_range(tokens, function, body_start),
        matching(tokens, body_start, "{", "}"),
    ) {
        (Some((open, close)), Some(body_end)) => Some((open, close, body_end)),
        _ => None,
    };
    let Some((open, close, body_end)) = body else {
        return (
            model,
            FunctionTypes::default(),
            UntrackedBases::default(),
            AggregateStoreBases::default(),
            // No body was located, so no statement of it is analyzed and no
            // destination of it can be classified. An empty set here never
            // admits anything: the refusal below fires only on a name this
            // reader positively recognised.
            PublicationFacts::default(),
            // Card AOM-BINDING-POISON: no body, so no name is spared and every
            // refusal in this function keeps the full poison.
            BTreeSet::new(),
        );
    };
    let types = FunctionTypes::read(
        &model,
        &texts[open + 1..close],
        &texts[body_start + 1..body_end],
        protected_formals,
    );
    // O04/C02-P: the admissible access bases of this body, read from the same
    // token stream, so that W4 derivation, W4 validation and certificate
    // rederivation all repeat the classification from the certificate's own
    // content. See [`crate::borrowed_actual`].
    let bases = crate::borrowed_actual::untracked_bases(
        &texts[body_start + 1..body_end],
        protected_formals,
        contracts,
        variadic_tails,
        &model,
        &types,
    );
    // O00 blocker B08: the local aggregate pointers a store may target, read
    // from the same token stream for the same reason.
    let formals = parameters
        .iter()
        .flatten()
        .cloned()
        .collect::<BTreeSet<_>>();
    let stores = crate::aggregate_store::store_bases(
        &texts[body_start + 1..body_end],
        &formals,
        contracts,
        &model,
        &types,
    );
    // CTWIN-PUBLISH: the assignment destinations of this body whose object has
    // static storage duration — file-scope objects this body does not shadow,
    // plus its own `static` locals. Read from the same token stream for the
    // same reason as every set above.
    // CTWIN-PUBLISH's destinations, plus CTWIN-RESIDUALS' continued-access
    // stores into them and the scalar names no publication may fire on. Read
    // from the same token stream for the same reason as every set above.
    let lines = tokens.iter().map(|token| token.line).collect::<Vec<_>>();
    let publications = crate::publication::facts(
        &texts,
        &texts[body_start + 1..body_end],
        &lines[body_start + 1..body_end],
        &formals,
        &types,
    );
    // Card AOM-BINDING-POISON, read from the same token stream for the same
    // reason as every set above: W4 derivation, W4 validation and certificate
    // rederivation each hold exactly this stream and so reproduce it.
    let frame_protected =
        frame_protected_locals(&texts, &texts[body_start + 1..body_end], &formals);
    (model, types, bases, stores, publications, frame_protected)
}

/// The parenthesis indices of the definition whose body starts at `body_start`.
fn function_parameter_range(
    tokens: &[Token],
    function: &str,
    body_start: usize,
) -> Option<(usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let Some(close) = matching(tokens, index + 1, "(", ")") else {
            continue;
        };
        if close + 1 == body_start {
            return Some((index + 1, close));
        }
    }
    None
}

// Discover identities requiring stable bindings before parsing expressions.
// This is only a conservative candidate inventory, never an inferred effect:
// direct *formal writes, forwarded Out calls, and non-borrowed resource calls
// are still checked by CFG re-analysis. Declared modes also protect formals.
fn protected_resource_formals(
    tokens: &[Token],
    body_start: usize,
    function: &str,
    parameters: &[Option<String>],
    contracts: &ResourceContracts,
) -> BTreeSet<String> {
    let formals = parameters
        .iter()
        .flatten()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut protected = BTreeSet::new();
    if let Some(contract) = contracts.contract(function) {
        for (parameter, mode) in parameters.iter().zip(&contract.parameters) {
            if !matches!(mode, ResourceParameterMode::Borrowed)
                && let Some(name) = parameter
            {
                protected.insert(name.clone());
            }
        }
    }
    let mut depth = 0usize;
    let body_end = tokens
        .iter()
        .enumerate()
        .skip(body_start)
        .find_map(|(index, token)| {
            match token.text.as_str() {
                "{" => depth += 1,
                "}" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
            None
        })
        .unwrap_or(tokens.len());
    let body = tokens[body_start..body_end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    for (index, pair) in body.windows(2).enumerate() {
        if pair[0] == "*" && formals.contains(pair[1]) {
            protected.insert(pair[1].to_owned());
        }
        if pair[1] != "(" {
            continue;
        }
        let Some(contract) = contracts.contract(pair[0]) else {
            continue;
        };
        let Some(arguments) = call_arguments(&body, index) else {
            continue;
        };
        for (mode, argument) in contract.parameters.iter().zip(arguments) {
            if matches!(mode, ResourceParameterMode::Borrowed) {
                continue;
            }
            let Some(place) = contract_argument_place(argument, *mode) else {
                continue;
            };
            let name = place.strip_prefix('*').unwrap_or(&place);
            if is_identifier(name) {
                protected.insert(name.to_owned());
            }
        }
    }
    // Propagate protection, never custody, across direct lexical aliases.
    // The closure is deliberately flow-insensitive: mutation before an alias
    // declaration must not be hidden by the later resource call on that alias.
    let mut aliases = Vec::new();
    let mut start = 0usize;
    for (end, token) in body.iter().enumerate() {
        if !matches!(*token, ";" | "{" | "}") {
            continue;
        }
        let statement = &body[start..end];
        start = end + 1;
        let Some(equal) = statement.iter().position(|token| *token == "=") else {
            continue;
        };
        let Some(destination) = direct_assignment_destination(&statement[..equal]) else {
            continue;
        };
        let Some(source) = canonical_place(&statement[equal + 1..]) else {
            continue;
        };
        if is_identifier(&destination) && is_identifier(&source) {
            aliases.push((destination, source));
        }
    }
    loop {
        let before = protected.len();
        for (destination, source) in &aliases {
            if protected.contains(destination) || protected.contains(source) {
                protected.insert(destination.clone());
                protected.insert(source.clone());
            }
        }
        if before == protected.len() {
            break;
        }
    }
    protected
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    text: String,
    line: usize,
}

/// Width of a C11 5.1.1.2 phase-2 line splice at `cursor`, if there is one.
///
/// Phase 2 deletes each backslash immediately followed by a new-line, joining
/// the two physical lines into one *logical* line, before any preprocessing
/// token is formed. Only the two spellings a conforming source can use are
/// admitted: `\` `\n` and `\` `\r` `\n`. A backslash followed by anything else
/// — trailing blanks included — is not a splice here, so a source that relies
/// on a compiler extension keeps its existing tokenization rather than
/// silently acquiring a different one.
pub(crate) fn line_splice_width(bytes: &[u8], cursor: usize) -> Option<usize> {
    if bytes.get(cursor) != Some(&b'\\') {
        return None;
    }
    match bytes.get(cursor + 1) {
        Some(&b'\n') => Some(2),
        Some(&b'\r') if bytes.get(cursor + 2) == Some(&b'\n') => Some(3),
        _ => None,
    }
}

/// Lex C source into the gateway's coarse token stream.
///
/// # Directives (unit **E01-F1-RAW**)
///
/// C11 6.10 reads a preprocessing directive only when `#` is the **first
/// preprocessing token of a logical line** — that is, when nothing but white
/// space and comments precedes it since the last new-line that phase 2 did not
/// delete — and the directive then runs to the end of that *logical* line. Two
/// consequences are load-bearing here and were both wrong before:
///
/// * a `#` that is *not* line-initial is an ordinary punctuator: the `#`
///   stringification and `##` pasting operators of a macro replacement list,
///   which may sit on a backslash-continuation line. Reading one as a directive
///   discarded the rest of that line — a closing brace included — and
///   desynchronised the file-scope brace depth for the whole rest of the file
///   (`docs/reports/remaining-e01-f1/README.md` §2.2, measured on
///   `librdkafka/src/rdkafka.c`);
/// * a directive continued with `\`-newline carries its whole replacement list,
///   so a multi-line macro body is skipped in full, exactly as a single-line
///   one already was. Macro *bodies* therefore contribute no token and no brace
///   to this scan, uniformly; a macro's braces perturb no depth.
///
/// A `#` reached in any other position is emitted as its own punctuator token
/// rather than discarded, so unmodeled text stays visible to the callers that
/// refuse what they cannot read.
pub(crate) fn tokenize(source: &str) -> Vec<Token> {
    tokenize_with_conditionals(source).0
}

/// Which conditional-inclusion directive a line-initial `#` introduces
/// (unit **E01-RAW-COND**).
///
/// Only the four *structural* roles matter to the raw scan: a group opens, an
/// arm ends and another begins, the last arm begins, the group closes. Which
/// controlling expression a directive carries is deliberately never read: this
/// scan does not evaluate conditionals and never selects an arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionalDirective {
    /// `#if`, `#ifdef`, `#ifndef`.
    Open,
    /// `#elif`, `#elifdef`, `#elifndef`.
    Alternative,
    /// `#else`. The group's arms then cover every outcome, so there is no
    /// implicit empty arm.
    Otherwise,
    /// `#endif`.
    Close,
}

/// One conditional directive, positioned in the token stream it contributes no
/// token to: `token_index` is the number of tokens emitted before it, so the
/// directive stands immediately before `tokens[token_index]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConditionalMark {
    token_index: usize,
    directive: ConditionalDirective,
}

/// Read the directive word of one complete directive line, which begins at its
/// `#`, and classify it if it is a conditional-inclusion directive.
///
/// The word is separated from the `#` by nothing, by horizontal white space, by
/// phase-2 splices, or by block comments; a `//` comment ends the line before a
/// word can follow. Recognition is total and exact — a directive introducer is
/// never macro-expanded (C11 6.10.3p10), so `#if` has exactly these spellings —
/// and it is deliberately *not* extended to unknown words: `#define`,
/// `#include`, `#pragma` and the null directive stay what they were, opaque
/// text that contributes nothing.
fn conditional_directive(line: &str) -> Option<ConditionalDirective> {
    let bytes = line.as_bytes();
    let mut cursor = 1; // the `#` itself
    loop {
        match bytes.get(cursor) {
            Some(&b'\\') if line_splice_width(bytes, cursor).is_some() => {
                cursor += line_splice_width(bytes, cursor)?;
            }
            Some(byte) if byte.is_ascii_whitespace() => cursor += 1,
            Some(&b'/') if bytes.get(cursor + 1) == Some(&b'*') => {
                cursor += 2;
                while cursor + 1 < bytes.len()
                    && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
                {
                    cursor += 1;
                }
                cursor = (cursor + 2).min(bytes.len());
            }
            _ => break,
        }
    }
    let start = cursor;
    while cursor < bytes.len() && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_') {
        cursor += 1;
    }
    match &line[start..cursor] {
        "if" | "ifdef" | "ifndef" => Some(ConditionalDirective::Open),
        "elif" | "elifdef" | "elifndef" => Some(ConditionalDirective::Alternative),
        "else" => Some(ConditionalDirective::Otherwise),
        "endif" => Some(ConditionalDirective::Close),
        _ => None,
    }
}

/// Lex as [`tokenize`] does, additionally recording where each
/// conditional-inclusion directive stood.
///
/// The token stream is **byte-identical** to [`tokenize`]'s: a directive still
/// contributes no token, so every consumer that reads only tokens is unchanged.
/// The marks are a side channel for [`function_names`], which needs the group
/// structure to keep the arms of one `#if … #endif` from each contributing a
/// brace to the same file-scope depth (unit **E01-RAW-COND**).
pub(crate) fn tokenize_with_conditionals(source: &str) -> (Vec<Token>, Vec<ConditionalMark>) {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut conditionals = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    // True while nothing but white space and comments has been seen since the
    // last undeleted new-line. A splice does not restore it: the logical line
    // continues.
    let mut at_logical_line_start = true;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(width) = line_splice_width(bytes, cursor) {
            line += 1;
            cursor += width;
        } else if byte == b'\n' {
            line += 1;
            cursor += 1;
            at_logical_line_start = true;
        } else if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            // A `//` comment is white space that ends at the next *logical*
            // line boundary, so a spliced continuation line stays commented.
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if let Some(width) = line_splice_width(bytes, cursor) {
                    line += 1;
                    cursor += width;
                } else {
                    cursor += 1;
                }
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            // A block comment is white space too. A new-line inside it is a
            // real new-line, so a `#` after it is still line-initial.
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                if bytes[cursor] == b'\n' {
                    line += 1;
                    at_logical_line_start = true;
                }
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'#' && at_logical_line_start {
            // A directive, consumed to the end of its logical line.
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if let Some(width) = line_splice_width(bytes, cursor) {
                    line += 1;
                    cursor += width;
                } else {
                    cursor += 1;
                }
            }
            if let Some(directive) = conditional_directive(&source[start..cursor]) {
                conditionals.push(ConditionalMark {
                    token_index: tokens.len(),
                    directive,
                });
            }
        } else if byte == b'"' || byte == b'\'' {
            at_logical_line_start = false;
            let quote = byte;
            let token_line = line;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else if bytes[cursor] == quote {
                    cursor += 1;
                    break;
                } else {
                    if bytes[cursor] == b'\n' {
                        line += 1;
                    }
                    cursor += 1;
                }
            }
            tokens.push(Token {
                text: "__qcore_literal__".to_owned(),
                line: token_line,
            });
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            at_logical_line_start = false;
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else if byte.is_ascii_digit() {
            at_logical_line_start = false;
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else {
            // Every other byte is its own punctuator, `#` and `##` included:
            // a non-line-initial `#` is a stringification or pasting operator,
            // never a directive introducer.
            at_logical_line_start = false;
            tokens.push(Token {
                text: char::from(byte).to_string(),
                line,
            });
            cursor += 1;
        }
    }
    (tokens, conditionals)
}

pub(crate) fn find_function_body(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) == Some("{") {
            return Some((close + 1, token.line));
        }
    }
    None
}

/// Why the raw scan could not establish a conditional group's contribution to
/// file-scope brace depth (unit **E01-RAW-COND**).
///
/// Both grounds are mechanical, and neither is a defect claim about the
/// analysed source: they say the *scan* cannot decide, not that the source is
/// wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConditionalScanRefusal {
    /// Two arms of one `#if … #endif` group leave different file-scope brace
    /// depths, so which depth survives is decided by evaluating the controlling
    /// expressions — preprocessing, not lexing. The scan takes the greatest
    /// arm depth, which can only hide definitions, never invent one.
    ArmDepthDisagreement,
    /// A conditional group is not well formed in this token stream: an
    /// `#elif`, `#else` or `#endif` with no open group, an `#elif`/`#else`
    /// after an `#else`, or a group never closed.
    UnstructuredGroup,
}

impl ConditionalScanRefusal {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArmDepthDisagreement => "conditional-arm-depth-disagreement",
            Self::UnstructuredGroup => "unstructured-conditional-group",
        }
    }

    /// The free-text reason carried by the refusal row.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ArmDepthDisagreement => {
                "raw scan refused a conditional group: its arms leave different file-scope brace depths, and deciding which arm survives is conditional evaluation, not lexing"
            }
            Self::UnstructuredGroup => {
                "raw scan refused a conditional group: an `#elif`, `#else` or `#endif` has no open group, or a group is never closed"
            }
        }
    }
}

/// The raw token scan of one main source: the definitions it found, and the
/// grounds on which it could not establish a conditional group's contribution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LegacyRawScan {
    definitions: Vec<String>,
    refusals: Vec<ConditionalScanRefusal>,
}

impl LegacyRawScan {
    /// The definitions this scan found, sorted and deduplicated.
    #[must_use]
    pub fn definitions(&self) -> &[String] {
        &self.definitions
    }

    /// The grounds on which a conditional group's contribution was refused.
    /// Empty means every conditional group in this source was decided exactly.
    #[must_use]
    pub fn refusals(&self) -> &[ConditionalScanRefusal] {
        &self.refusals
    }

    /// Whether every conditional group was decided exactly. A scan that is not
    /// exact is still a *subset* of the definitions of every arm selection —
    /// the depth it carries dominates every selection's — but it may be a
    /// proper subset, so it cannot support a completeness claim.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.refusals.is_empty()
    }
}

/// One open `#if … #endif` group, while its arms are being scanned.
struct ConditionalFrame {
    /// File-scope brace depth where the group began: every arm is scanned from
    /// here, so a definition written in *any* arm is seen at the depth it would
    /// have if that arm survived (E01-F1 §1.4 monotonicity).
    entry_depth: usize,
    /// The depth each already-scanned arm ended at.
    arm_depths: Vec<usize>,
    /// Whether an `#else` arm has been opened.
    saw_else: bool,
}

/// Apply one conditional directive to the scan state (unit **E01-RAW-COND**).
///
/// The rule, stated once: **a group's resulting depth is the greatest depth its
/// arms reach**, where a group without `#else` has an implicit empty arm that
/// reaches the entry depth. When every arm reaches the same depth — the
/// overwhelmingly common case, and the `rd_kafka_dir_is_empty` case this unit
/// closes — that greatest depth *is* the depth of every arm, so the group is
/// decided exactly without any arm being selected. When the arms disagree, the
/// scan does not guess: it takes the greatest depth, which dominates every arm
/// selection and can therefore only hide definitions, and records
/// [`ConditionalScanRefusal::ArmDepthDisagreement`] so the enumeration's
/// completeness claim is refused rather than silently over-claimed.
fn apply_conditional(
    mark: ConditionalMark,
    frames: &mut Vec<ConditionalFrame>,
    brace_depth: &mut usize,
    refusals: &mut BTreeSet<ConditionalScanRefusal>,
) {
    match mark.directive {
        ConditionalDirective::Open => frames.push(ConditionalFrame {
            entry_depth: *brace_depth,
            arm_depths: Vec::new(),
            saw_else: false,
        }),
        ConditionalDirective::Alternative | ConditionalDirective::Otherwise => {
            let Some(frame) = frames.last_mut() else {
                refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
                return;
            };
            if frame.saw_else {
                // An `#elif` or a second `#else` after `#else` is not a group
                // this scan can read.
                refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
            }
            frame.saw_else |= mark.directive == ConditionalDirective::Otherwise;
            frame.arm_depths.push(*brace_depth);
            *brace_depth = frame.entry_depth;
        }
        ConditionalDirective::Close => {
            let Some(mut frame) = frames.pop() else {
                refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
                return;
            };
            frame.arm_depths.push(*brace_depth);
            if !frame.saw_else {
                // Every controlling expression may be false: an implicit empty
                // arm that reaches the entry depth is one of the outcomes.
                frame.arm_depths.push(frame.entry_depth);
            }
            let first = frame.arm_depths[0];
            if frame.arm_depths.iter().any(|depth| *depth != first) {
                refusals.insert(ConditionalScanRefusal::ArmDepthDisagreement);
            }
            *brace_depth = frame.arm_depths.into_iter().max().unwrap_or(first);
        }
    }
}

fn function_names(tokens: &[Token], conditionals: &[ConditionalMark]) -> LegacyRawScan {
    let mut names = BTreeSet::new();
    let mut refusals = BTreeSet::new();
    let mut frames: Vec<ConditionalFrame> = Vec::new();
    let mut next_mark = 0usize;
    let mut brace_depth = 0usize;
    let mut linkage_closes = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        while next_mark < conditionals.len() && conditionals[next_mark].token_index <= index {
            apply_conditional(
                conditionals[next_mark],
                &mut frames,
                &mut brace_depth,
                &mut refusals,
            );
            next_mark += 1;
        }
        if token.text == "{" {
            // Raw C sources commonly retain an `extern "C"` wrapper behind
            // `#ifdef __cplusplus`; it is a linkage boundary, not a function.
            if index >= 2
                && tokens[index - 2].text == "extern"
                && tokens[index - 1].text == "__qcore_literal__"
                && let Some(close) = matching(tokens, index, "{", "}")
            {
                linkage_closes.insert(close);
                continue;
            }
            brace_depth += 1;
            continue;
        }
        if token.text == "}" {
            if linkage_closes.remove(&index) {
                continue;
            }
            brace_depth = brace_depth.saturating_sub(1);
            continue;
        }
        if brace_depth != 0 {
            continue;
        }
        if !is_identifier(&token.text)
            || matches!(token.text.as_str(), "if" | "while" | "for" | "switch")
            || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
        {
            continue;
        }
        if let Some(close) = matching(tokens, index + 1, "(", ")")
            && tokens.get(close + 1).map(|next| next.text.as_str()) == Some("{")
        {
            names.insert(token.text.clone());
        }
    }
    while next_mark < conditionals.len() {
        apply_conditional(
            conditionals[next_mark],
            &mut frames,
            &mut brace_depth,
            &mut refusals,
        );
        next_mark += 1;
    }
    if !frames.is_empty() {
        refusals.insert(ConditionalScanRefusal::UnstructuredGroup);
    }
    LegacyRawScan {
        definitions: names.into_iter().collect(),
        refusals: refusals.into_iter().collect(),
    }
}

/// Return the function definitions written in a source file before headers
/// and macros are expanded. This is the auditable scope used by
/// [`LegacyFunctionScope::MainSourceDefinitions`].
#[must_use]
pub fn legacy_c_source_functions(source: &str) -> Vec<String> {
    legacy_c_source_scan(source).definitions
}

/// Scan a source file for the function definitions written in it, before
/// headers and macros are expanded, reporting whether every conditional group
/// was decided exactly (unit **E01-RAW-COND**).
///
/// This is [`legacy_c_source_functions`] with its conditional-group grounds
/// kept, so a caller that publishes a completeness claim over the result can
/// refuse it. The definition list is identical.
#[must_use]
pub fn legacy_c_source_scan(source: &str) -> LegacyRawScan {
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    function_names(&tokens, &conditionals)
}

/// Declaration specifiers that can end a parameter declarator without naming
/// it. If one of these is the last token, the parameter is unnamed and the body
/// cannot refer to it.
const DECLARATION_SPECIFIERS: &[&str] = &[
    "void",
    "char",
    "short",
    "int",
    "long",
    "float",
    "double",
    "signed",
    "unsigned",
    "_Bool",
    "_Complex",
    "_Imaginary",
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
    "struct",
    "union",
    "enum",
    "static",
    "register",
    "inline",
    "_Atomic",
];

/// Formal parameter names of a defined function, in declaration order.
///
/// `Some(vec![Some(name), None, ..])` names each formal the body can refer to;
/// an inner `None` is an unnamed formal. The whole result is `None` whenever
/// the declarator list cannot be read *unambiguously* — variadic lists,
/// function-pointer parameters, K&R-style single-identifier declarators,
/// unnamed tagged types, and duplicate names all reject the function outright.
/// Misreading which declarator carries a name would attach custody to the wrong
/// argument, so this parser refuses everything it cannot pin down.
pub(crate) fn function_parameters(tokens: &[Token], function: &str) -> Option<Vec<Option<String>>> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) != Some("{") {
            continue;
        }
        return parse_parameter_declarators(&tokens[index + 2..close]);
    }
    None
}

fn parse_parameter_declarators(tokens: &[Token]) -> Option<Vec<Option<String>>> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    // `(`/`)` mark a function-pointer or nested declarator; `.` is the first
    // token of a variadic `...`. Neither is read here.
    if texts
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "."))
    {
        return None;
    }
    if texts.is_empty() {
        return Some(Vec::new());
    }
    let groups = texts.split(|text| *text == ",").collect::<Vec<_>>();
    if groups.len() == 1 && groups[0] == ["void"] {
        return Some(Vec::new());
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for group in groups {
        let mut group = group;
        // Drop array declarator suffixes: `char buf[16]` names `buf`.
        while group.last() == Some(&"]") {
            let open = group.iter().rposition(|text| *text == "[")?;
            group = &group[..open];
        }
        if group.is_empty() {
            return None;
        }
        let last = *group.last()?;
        if !is_identifier(last) {
            // An abstract declarator such as `const char *`: unnamed.
            names.push(None);
            continue;
        }
        if DECLARATION_SPECIFIERS.contains(&last) {
            names.push(None);
            continue;
        }
        if group.len() < 2 {
            // `f(n)` (K&R) and `f(size_t)` are indistinguishable here.
            return None;
        }
        if matches!(group[group.len() - 2], "struct" | "union" | "enum") {
            // `struct foo` without a declarator: the identifier is the tag.
            names.push(None);
            continue;
        }
        if !seen.insert(last.to_owned()) {
            return None;
        }
        names.push(Some(last.to_owned()));
    }
    Some(names)
}

/// Names in `candidates` that the body of `function` calls directly.
pub(crate) fn direct_calls(
    tokens: &[Token],
    function: &str,
    candidates: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut called = BTreeSet::new();
    let Some((body_start, _)) = find_function_body(tokens, function) else {
        return called;
    };
    let Some(body_end) = matching(tokens, body_start, "{", "}") else {
        return called;
    };
    for index in body_start..body_end {
        if tokens.get(index + 1).map(|token| token.text.as_str()) != Some("(") {
            continue;
        }
        let name = &tokens[index].text;
        if candidates.contains(name) {
            called.insert(name.clone());
        }
    }
    called
}

pub(crate) fn matching(tokens: &[Token], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
enum Statement {
    Instruction(LegacyInstruction, usize),
    Block(Vec<Statement>),
    If {
        line: usize,
        condition_supported: bool,
        /// Card **B02-BRANCH-CONDITION**: the one contracted call whose
        /// custody effects are evaluated before this branch decision.  The
        /// isolated instruction is produced by the ordinary statement
        /// classifier; `Some` therefore adds no second call-effect model.
        condition_call: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        /// C02-P-ALIAS: the pointer-inequality guard this condition expresses,
        /// when it relates two names the may-alias rule relates. Consulted only
        /// when `null_condition` is `None`, so no null test changes shape.
        distinct_condition: Option<PointerDistinctCondition>,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    While {
        line: usize,
        condition_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated at
        /// every visit to this loop header, before either successor is chosen.
        condition_call: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        body: Vec<Statement>,
    },
    For {
        line: usize,
        init: Option<Box<(LegacyInstruction, usize)>>,
        condition_always_true: bool,
        condition_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated at
        /// every visit to this loop header, before either successor is chosen.
        condition_call: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        increment: Option<Box<(LegacyInstruction, usize)>>,
        body: Vec<Statement>,
    },
    DoWhile {
        line: usize,
        condition_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated
        /// after every completed body execution, before the back/exit choice.
        condition_call: Option<Box<LegacyInstruction>>,
        /// The controlling expression is the single literal token `0`, so the
        /// body runs exactly once (C11 §6.8.5.2) and the CFG carries no back
        /// edge. Keyed on the *token*, never on an evaluated expression
        /// (standing constraint 5a: an operand-class marker is never a
        /// computed value). Card `C-GATEWAY-CONSTANT-DO-WHILE`.
        condition_is_literal_zero: bool,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        body: Vec<Statement>,
    },
    Switch {
        line: usize,
        expression_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated
        /// once before control enters the selected switch arm.
        expression_call: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        body: Vec<Statement>,
    },
    Case {
        line: usize,
        expression_supported: bool,
        writes: RefusedWrites,
        is_default: bool,
    },
    Label {
        line: usize,
        name: String,
    },
    Goto {
        line: usize,
        label: String,
    },
    Break {
        line: usize,
    },
    Continue {
        line: usize,
    },
    Return {
        line: usize,
        expression_supported: bool,
        writes: RefusedWrites,
        returned_place: Option<String>,
        /// A modeled evaluation of this `return` expression. Card
        /// C-GATEWAY-RETURN-CALL-W4 supplies the exact contract-call case;
        /// B01-RETURN-EXPRESSION adds bounded scalar and opaque evaluations.
        /// A declined evaluation keeps the existing refusal byte for byte.
        returned_evaluation: Option<Box<LegacyInstruction>>,
        /// Whether the evaluated value can carry custody and therefore needs
        /// the synthetic `EscapeIfOwned` exit. B01's scalar/opaque family is
        /// resource-free and uses `NoOp` instead.
        returned_evaluation_escapes: bool,
    },
}

#[derive(Clone, Debug)]
struct PointerNullCondition {
    source: String,
    true_is_null: bool,
}

/// C02-P-ALIAS: a comparison of two plain declared names the may-alias rule
/// relates. Nothing else about the condition is read, and no other rule
/// consults it.
#[derive(Clone, Debug)]
struct PointerDistinctCondition {
    left: String,
    right: String,
    true_is_distinct: bool,
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    contracts: &'a ResourceContracts,
    next_site: usize,
    formal_parameters: BTreeSet<String>,
    protected_formals: BTreeSet<String>,
    /// Declaration facts of the enclosing translation unit and function, used
    /// only to admit the typed direct scalar-field whitelist of
    /// [`crate::typed_field_access`]. Everything outside that whitelist keeps
    /// the lexical rejection.
    typed_model: TypedAccessModel,
    typed_types: FunctionTypes,
    /// Access bases this function never binds to a resource identity, used only
    /// to admit the borrowed pointer-field actuals of
    /// [`crate::borrowed_actual`]. Everything outside that whitelist keeps the
    /// lexical rejection.
    typed_bases: UntrackedBases,
    /// Local aggregate pointers whose single-selection fields a store may
    /// target (O00 blocker B08). Everything outside that whitelist keeps the
    /// lexical rejection. See [`crate::aggregate_store`].
    store_bases: AggregateStoreBases,
    /// C02-P-ALIAS: the declared record-pointer classes of this function, used
    /// only to attach may-alias obligations to modeled reads and to recognise a
    /// pointer-inequality guard. It admits nothing.
    alias_classes: crate::may_alias::MayAliasClasses,
    /// Card O04-TRACE: the callees whose declaration carries a
    /// `resource_variadic_data_tail()`, rederived from the environment's own
    /// retained trusted sources. Empty for an environment with no declared
    /// sources, so an inferred summary never acquires a tail.
    variadic_tails: BTreeSet<String>,
    /// Card **AOM-BINDING-POISON**: the names of this body whose slot no lvalue
    /// but the name itself designates, per [`frame_protected_locals`]. It
    /// admits nothing; it only bounds what a refused statement destroys in the
    /// binding environment. Empty whenever the body could not be delimited,
    /// which restores the pre-repair blanket poison for the whole function.
    frame_protected: BTreeSet<String>,
    /// Card **C-GATEWAY-INDIRECT-CENSUS-2**: this function's name, carried only
    /// so the instrumented census can label its records. Compiled out with the
    /// feature off, and read by nothing else.
    #[cfg(feature = "indirect-census")]
    census_function: String,
}

impl Parser<'_> {
    /// Card **AOM-BINDING-POISON**: the bindings one refused statement spares.
    ///
    /// Called only where `texts` is the **complete** token text of the refused
    /// statement, which is exactly [`Self::classify_simple`]'s contract with
    /// its callers — the statement walker hands it a whole
    /// `consume_to_semicolon` run, and the `for` walker a whole init or
    /// increment clause. The delimited condition/return expression reader also
    /// uses this rule; structural control-flow failures remain unbounded.
    fn refused_writes(&self, texts: &[&str]) -> RefusedWrites {
        let written = may_write_names(texts);
        RefusedWrites::Sparing(
            self.frame_protected
                .iter()
                .filter(|name| !written.contains(name.as_str()))
                .cloned()
                .collect(),
        )
    }

    /// Card **C-GATEWAY-INDIRECT-CENSUS-2**: hand one already-decided
    /// `indirect memory access` refusal to the instrumented census.
    ///
    /// Called only after the classifier has committed to refusing, so it can
    /// neither admit nor refuse anything. Compiled out with the feature off.
    #[cfg(feature = "indirect-census")]
    fn census_indirect(
        &self,
        texts: &[&str],
        exempt: &[(usize, usize)],
        refusal: &crate::indirect_census::Refusal,
    ) {
        crate::indirect_census::record(
            &crate::indirect_census::Context {
                function: &self.census_function,
                formal_parameters: &self.formal_parameters,
                typed_model: &self.typed_model,
                typed_types: &self.typed_types,
                typed_bases: &self.typed_bases,
            },
            texts,
            exempt,
            refusal,
        );
    }

    /// The complete expression was delimited by the parser; malformed
    /// control flow never reaches this constructor.
    fn expression_writes(&self, tokens: &[Token]) -> RefusedWrites {
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
    fn scalar_effect(&self, texts: &[&str]) -> LegacyInstruction {
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
    fn supported_expression(&self, tokens: &[Token], role: ExpressionRole) -> bool {
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
        if supported_neutral_expression(tokens) {
            return true;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
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
    fn condition_call_instruction(&mut self, condition: &[Token]) -> Option<LegacyInstruction> {
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

        let restore = self.next_site;
        let instruction = self.classify_simple(&condition[start..end]);
        if matches!(instruction, LegacyInstruction::ContractCall { .. }) {
            Some(instruction)
        } else {
            self.next_site = restore;
            None
        }
    }

    /// The null test a branch condition expresses, including the one
    /// aggregate-field form card O04-B08 admits.
    ///
    /// The general refusal of `->` in a condition is unchanged: only a null
    /// test whose *entire* operand is an admitted single-selection **pointer**
    /// field of an admitted aggregate base is read here, and only when the
    /// condition carries no call, no braces and no other unmodeled access.
    fn null_condition(&self, condition: &[Token]) -> Option<PointerNullCondition> {
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

    fn aggregate_null_condition(&self, texts: &[&str]) -> Option<PointerNullCondition> {
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
            });
        }
        None
    }

    /// The place named by an operand that is exactly `BASE - > FIELD` on an
    /// admitted base whose member is a single-star pointer.
    fn admitted_field_operand(&self, texts: &[&str]) -> Option<String> {
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
    fn distinct_condition(&self, condition: &[Token]) -> Option<PointerDistinctCondition> {
        if !supported_neutral_expression(condition) {
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

    fn distinct_condition_text(&self, texts: &[&str]) -> Option<PointerDistinctCondition> {
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
    fn call_alias_reads(
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
    fn read_alias(&self, base: Option<&str>) -> Option<AliasPeers> {
        let place = base?;
        let peers = self.alias_classes.peers(place);
        (!peers.is_empty()).then(|| AliasPeers {
            place: place.to_owned(),
            peers,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, usize> {
        let opening_line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        if self.text() != Some("{") {
            return Err(opening_line);
        }
        self.cursor += 1;
        let mut statements = Vec::new();
        while self.cursor < self.tokens.len() && self.text() != Some("}") {
            statements.push(self.parse_statement()?);
        }
        if self.text() != Some("}") {
            return Err(self.tokens.last().map_or(opening_line, |token| token.line));
        }
        self.cursor += 1;
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, usize> {
        let line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        match self.text() {
            Some("if") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let condition_supported =
                    self.supported_expression(&condition, ExpressionRole::Decides);
                let condition_call = (!condition_supported)
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let then_body = self.parse_statement_body()?;
                let else_body = if self.text() == Some("else") {
                    self.cursor += 1;
                    self.parse_statement_body()?
                } else {
                    Vec::new()
                };
                Ok(Statement::If {
                    line,
                    writes: self.expression_writes(&condition),
                    condition_supported,
                    condition_call,
                    null_condition: self.null_condition(&condition),
                    distinct_condition: self.distinct_condition(&condition),
                    then_body,
                    else_body,
                })
            }
            Some("while") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let condition_supported =
                    self.supported_expression(&condition, ExpressionRole::Decides);
                let condition_call = (!condition_supported)
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let body = self.parse_statement_body()?;
                Ok(Statement::While {
                    line,
                    writes: self.expression_writes(&condition),
                    condition_supported,
                    condition_call,
                    null_condition: self.null_condition(&condition),
                    body,
                })
            }
            Some("for") => {
                self.cursor += 1;
                let clauses = self.take_parenthesized()?;
                let Some([init, _stripped_condition, increment]) = split_for_clauses(&clauses)
                else {
                    return Ok(Statement::Instruction(
                        LegacyInstruction::Unknown {
                            description: "malformed for-loop clauses".to_owned(),
                            writes: RefusedWrites::Unbounded,
                        },
                        line,
                    ));
                };
                // The condition is classified from its original spelling; the
                // other two clauses keep the stripped one they have today.
                let Some(condition) = for_condition_tokens(&clauses) else {
                    return Ok(Statement::Instruction(
                        LegacyInstruction::Unknown {
                            description: "malformed for-loop clauses".to_owned(),
                            writes: RefusedWrites::Unbounded,
                        },
                        line,
                    ));
                };
                let condition_supported =
                    self.supported_expression(&condition, ExpressionRole::Decides);
                let condition_call = (!condition_supported && !condition.is_empty())
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let body = self.parse_statement_body()?;
                let init =
                    (!init.is_empty()).then(|| Box::new((self.classify_simple(&init), line)));
                let increment = (!increment.is_empty())
                    .then(|| Box::new((self.classify_simple(&increment), line)));
                Ok(Statement::For {
                    line,
                    writes: self.expression_writes(&condition),
                    init,
                    condition_always_true: condition.is_empty(),
                    condition_supported,
                    condition_call,
                    null_condition: self.null_condition(&condition),
                    increment,
                    body,
                })
            }
            Some("do") => {
                self.cursor += 1;
                let body = self.parse_statement_body()?;
                if self.text() != Some("while") {
                    return Err(line);
                }
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                let condition_supported =
                    self.supported_expression(&condition, ExpressionRole::Decides);
                let condition_call = (!condition_supported)
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                Ok(Statement::DoWhile {
                    line,
                    writes: self.expression_writes(&condition),
                    condition_supported,
                    condition_call,
                    condition_is_literal_zero: condition_is_literal_zero(&condition),
                    null_condition: self.null_condition(&condition),
                    body,
                })
            }
            Some("switch") => {
                self.cursor += 1;
                let expression = self.take_parenthesized()?;
                let expression_supported =
                    self.supported_expression(&expression, ExpressionRole::Decides);
                let expression_call = (!expression_supported)
                    .then(|| self.condition_call_instruction(&expression))
                    .flatten()
                    .map(Box::new);
                let body = self.parse_statement_body()?;
                Ok(Statement::Switch {
                    line,
                    writes: self.expression_writes(&expression),
                    expression_supported,
                    expression_call,
                    body,
                })
            }
            Some("case") => {
                self.cursor += 1;
                let expression = self.consume_to_colon()?;
                Ok(Statement::Case {
                    line,
                    writes: self.expression_writes(&expression),
                    // A `case` label is an integer constant expression: no call and
                    // no member access can appear in one, so the typed admission
                    // would be inert here. The strict lexical predicate is kept
                    // deliberately (W4-EXPR-POS-CASE).
                    expression_supported: supported_neutral_expression(&expression),
                    is_default: false,
                })
            }
            Some("default") => {
                self.cursor += 1;
                if self.text() != Some(":") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Case {
                    line,
                    writes: RefusedWrites::Unbounded,
                    expression_supported: true,
                    is_default: true,
                })
            }
            Some("goto") => {
                self.cursor += 1;
                let Some(label) = self.text().filter(|text| is_identifier(text)) else {
                    return Err(line);
                };
                let label = label.to_owned();
                self.cursor += 1;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Goto { line, label })
            }
            Some("break") => {
                self.cursor += 1;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Break { line })
            }
            Some("continue") => {
                self.cursor += 1;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Continue { line })
            }
            Some("return") => {
                self.cursor += 1;
                let expression = self.consume_to_semicolon()?;
                let texts = expression
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<Vec<_>>();
                // Card C-GATEWAY-RETURN-POSITION. The strict `Returned` rule is
                // asked first and its answer is never weakened; only when it
                // refuses does the pointer-leaf widening get a say, and an
                // expression it admits escapes **nothing** — see
                // `neutral_position::admits_only_untracked_returned_pointer`
                // for the argument and for why `returned_place` must be `None`
                // here rather than whatever `canonical_place` would spell.
                let strict = self.supported_expression(&expression, ExpressionRole::Returned);
                let untracked_pointer =
                    !strict && self.supported_expression(&expression, ExpressionRole::Decides);
                // Card C-GATEWAY-RETURN-CALL-W4. Asked last, and only where
                // both rules above refuse, so no expression this project
                // admitted before is decided by different code. A `Some` here
                // is a *modeled* call whose result escapes only custody the
                // call itself produced; everything else keeps the refusal it
                // has, including the `Reallocated` result wave 33's soundness
                // ground was right about.
                let returned_evaluation = (!strict && !untracked_pointer)
                    .then(|| self.return_expression_instruction(&expression, line))
                    .flatten();
                Ok(Statement::Return {
                    line,
                    writes: self.expression_writes(&expression),
                    expression_supported: strict || untracked_pointer,
                    returned_place: if untracked_pointer {
                        None
                    } else {
                        canonical_place(&texts)
                    },
                    returned_evaluation: returned_evaluation
                        .as_ref()
                        .map(|(instruction, _)| Box::new(instruction.clone())),
                    returned_evaluation_escapes: returned_evaluation
                        .is_some_and(|(_, escapes)| escapes),
                })
            }
            Some("{") => Ok(Statement::Block(self.parse_block()?)),
            Some(name)
                if is_identifier(name)
                    && self
                        .tokens
                        .get(self.cursor + 1)
                        .map(|token| token.text.as_str())
                        == Some(":") =>
            {
                let name = name.to_owned();
                self.cursor += 2;
                Ok(Statement::Label { line, name })
            }
            Some(_) => {
                let tokens = self.consume_to_semicolon()?;
                Ok(Statement::Instruction(self.classify_simple(&tokens), line))
            }
            None => Err(line),
        }
    }

    fn parse_statement_body(&mut self) -> Result<Vec<Statement>, usize> {
        if self.text() == Some("{") {
            self.parse_block()
        } else {
            Ok(vec![self.parse_statement()?])
        }
    }

    fn take_parenthesized(&mut self) -> Result<Vec<Token>, usize> {
        let line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        if self.text() != Some("(") {
            return Err(line);
        }
        let close = matching(self.tokens, self.cursor, "(", ")").ok_or(line)?;
        let result = self.tokens[self.cursor + 1..close].to_vec();
        self.cursor = close + 1;
        Ok(result)
    }

    fn consume_to_semicolon(&mut self) -> Result<Vec<Token>, usize> {
        let start = self.cursor;
        let line = self.tokens.get(start).map_or(1, |token| token.line);
        let mut delimiters = Vec::new();
        while self.cursor < self.tokens.len() {
            match self.text() {
                Some("(") => delimiters.push(")"),
                Some("[") => delimiters.push("]"),
                Some("{") => delimiters.push("}"),
                Some(close @ (")" | "]" | "}")) => {
                    if delimiters.pop() != Some(close) {
                        return Err(line);
                    }
                }
                Some(";") if delimiters.is_empty() => {
                    let result = self.tokens[start..self.cursor].to_vec();
                    self.cursor += 1;
                    return Ok(result);
                }
                _ => {}
            }
            self.cursor += 1;
        }
        Err(line)
    }

    fn consume_to_colon(&mut self) -> Result<Vec<Token>, usize> {
        let start = self.cursor;
        let line = self.tokens.get(start).map_or(1, |token| token.line);
        let mut parentheses = 0usize;
        while self.cursor < self.tokens.len() {
            match self.text() {
                Some("(") => parentheses += 1,
                Some(")") => parentheses = parentheses.saturating_sub(1),
                Some(":") if parentheses == 0 => {
                    let result = self.tokens[start..self.cursor].to_vec();
                    self.cursor += 1;
                    return Ok(result);
                }
                Some("{") | Some("}") | Some(";") if parentheses == 0 => return Err(line),
                _ => {}
            }
            self.cursor += 1;
        }
        Err(line)
    }

    /// Card **C-GATEWAY-RETURN-CALL-W4**: the modeled call a `return`
    /// expression is, desugared to the assignment the analyzer already decides.
    ///
    /// The whole expression must be exactly one call, the reserved synthetic
    /// place must be unspelled in this stream, and the answer
    /// [`Self::classify_simple`] gives to `PLACE = <expression>` must be a
    /// [`LegacyInstruction::ContractCall`] into that place whose result mode
    /// escapes only custody the call produced. Anything else answers `None` and
    /// the `return` keeps the refusal it had.
    ///
    /// `classify_simple`'s only mutation of the parser is `next_site`, so a
    /// declined probe restores it and is observationally identical to never
    /// having run — which is what keeps site numbering, and therefore
    /// certificate identity, unmoved for every `return` this card refuses.
    fn return_call_instruction(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<LegacyInstruction> {
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        return_call::whole_expression_call(expression, &texts)?;
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE) {
            return None;
        }
        let mut synthetic = Vec::with_capacity(expression.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(expression);
        let restore = self.next_site;
        let instruction = self.classify_simple(&synthetic);
        let admitted = matches!(
            &instruction,
            LegacyInstruction::ContractCall {
                destination: Some(destination),
                contract,
                ..
            } if destination == return_call::RETURN_RESULT_PLACE
                && return_call::escapes_only_fresh_custody(contract.result)
        );
        if admitted {
            Some(instruction)
        } else {
            self.next_site = restore;
            None
        }
    }

    /// Card **B01-RETURN-EXPRESSION**: evaluate a refused return through the
    /// ordinary assignment classifier before falling back to the position's
    /// generic refusal.
    ///
    /// The pre-existing whole-call rule is asked first and unchanged. For the
    /// remaining expressions this rule admits two bounded shapes:
    ///
    /// * an `OpaqueValue` assigned to the reserved return place; the classifier
    ///   has established that the value carries no resource identity, so B01
    ///   emits a `NoOp` exit and performs no custody escape;
    /// * exactly one contracted call with `ResourceReturnMode::None` inside an
    ///   otherwise custody-neutral scalar expression. The call is classified
    ///   as a standalone statement so its parameter effects are applied, and
    ///   its resource-free result leaves nothing for the return to escape.
    ///
    /// Every refusal, including the classifier's own `Unknown`, keeps the old
    /// return-position wrapper and wording.
    ///
    /// A declined probe restores `next_site`, so a classification attempt
    /// cannot perturb later certificate identities.
    fn return_expression_instruction(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<(LegacyInstruction, bool)> {
        if let Some(call) = self.return_call_instruction(expression, line) {
            return Some((call, true));
        }
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE) {
            return None;
        }
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if let Some((call_index, call_end)) =
            crate::legacy_condition_call::unconditional_call_span(&texts)
        {
            // This widening is for calls backed by a declared API contract.
            // A locally defined callee may acquire a provisional summary
            // during fixed-point analysis; treating that summary as the
            // declaration premise would weaken W4's pointer-actual boundary
            // for recursive expressions such as `1 + counted(p)`.
            let externally_declared = find_function_body(self.tokens, texts[call_index]).is_none();
            let outside = texts[..call_index]
                .iter()
                .chain(texts[call_end..].iter())
                .copied()
                .collect::<Vec<_>>();
            if externally_declared
                && crate::typed_field_access::is_scalar_operand_expression(
                    &outside,
                    &self.typed_model,
                    &self.typed_types,
                )
            {
                let restore = self.next_site;
                let instruction = self.classify_simple(&expression[call_index..call_end]);
                let admitted = matches!(
                    &instruction,
                    LegacyInstruction::ContractCall {
                        destination: None,
                        contract,
                        ..
                    } if contract.result == ResourceReturnMode::None
                );
                if admitted {
                    return Some((instruction, false));
                }
                self.next_site = restore;
            }
        }
        let mut synthetic = Vec::with_capacity(expression.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(expression);
        let restore = self.next_site;
        let instruction = self.classify_simple(&synthetic);
        let direct_member_read = texts.iter().filter(|text| **text == ".").count()
            + texts.windows(2).filter(|pair| *pair == ["-", ">"]).count()
            == 1;
        let admitted = matches!(
            &instruction,
            LegacyInstruction::OpaqueValue { destination, .. }
                if destination == return_call::RETURN_RESULT_PLACE
        ) && direct_member_read;
        if admitted {
            self.next_site = restore;
            Some((instruction, false))
        } else {
            self.next_site = restore;
            None
        }
    }

    fn classify_simple(&mut self, tokens: &[Token]) -> LegacyInstruction {
        // Retain the enclosing function and subsequent cleanup, but never
        // mistake nested initializers or GNU statement expressions for a
        // single modeled call/assignment. Their custody effects are unknown.
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
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
                description: "unmodeled expression may mutate a formal resource identity".into(),
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
            return LegacyInstruction::ScalarMemberStore {
                base: base.to_owned(),
                alias_read: self.read_alias(Some(base)),
            };
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
            && crate::typed_field_access::is_scalar_field_noop(
                &texts,
                &self.typed_model,
                &self.typed_types,
            )
        {
            return self.scalar_effect(&texts);
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
        // Explicit Out stores remain a separate modeled instruction. Reading
        // through a pointer, member or subscript has no heap/liveness model and
        // must never disappear into scalar NoOp or borrowed-call argument erasure.
        let unsupported_access = if let Some(equal) = assignment {
            let destination = direct_assignment_destination(&texts[..equal]);
            (admitted_store.is_none()
                && destination.as_deref().is_some_and(|place| {
                    place.contains('.') || place.contains('[') || place.contains("->")
                }))
                || contains_unmodeled_access(&outside_spans(&texts, &borrowed_actuals, equal + 1))
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
            (texts.contains(&"[") && !constant_array_declaration(&texts)) || texts.contains(&".")
        } else {
            contains_unmodeled_access(&outside_spans(&texts, &borrowed_actuals, 0))
        };
        // Card HASH-CONTRACTS. A call the premise rewrite withdrew is refused
        // *before* the lexical access gate, so the statement reports why the
        // declaration did not cover it rather than the access the withdrawn
        // declaration would have admitted. Both are the same `Unknown`; only
        // the attribution differs, and the informative one is the premise.
        if let Some(callee) =
            find_call(&texts).and_then(|(_, function)| literal_premise::unmet_callee(function))
        {
            return LegacyInstruction::Unknown {
                description: format!(
                    "call to `{callee}` outside its declared literal-argument premise"
                ),
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
                        && direct_assignment_destination(&texts[..equal])
                            .as_deref()
                            .is_some_and(|place| {
                                place.contains('.') || place.contains('[') || place.contains("->")
                            })
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
                if let Some(store) = &admitted_store
                    && !matches!(contract.result, ResourceReturnMode::None)
                    && !store.pointer_field
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
                if !matches!(contract.result, ResourceReturnMode::None) && destination.is_none() {
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
                if let Some(arguments) = &raw_arguments
                    && arguments.len() > contract.parameters.len()
                    && !(variadic_tail
                        && crate::variadic_nested::tail_arguments_are_data_with_nested(
                            arguments,
                            contract.parameters.len(),
                            &nested,
                            &self.typed_model,
                            &self.typed_types,
                            &self.typed_bases,
                        ))
                {
                    return LegacyInstruction::Unknown {
                        description: format!(
                            "arguments beyond the declared parameters of `{function}` are outside the resource projection"
                        ),
                        writes: self.refused_writes(&texts),
                    };
                }
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
                                && (((!matches!(mode, ResourceParameterMode::Out { .. })
                                    && argument.contains(&"&"))
                                    || (matches!(mode, ResourceParameterMode::Borrowed)
                                        && canonical_place(argument).is_none()))
                                    && argument
                                        .iter()
                                        .any(|name| self.protected_formals.contains(*name)))
                        },
                    )
                }) {
                    return LegacyInstruction::Unknown {
                        description: "call has an unmodeled formal address or borrowed expression"
                            .into(),
                        writes: self.refused_writes(&texts),
                    };
                }
                let arguments = contract
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
                let produced_count =
                    usize::from(!matches!(contract.result, ResourceReturnMode::None))
                        + contract
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
                return LegacyInstruction::ContractCall {
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
            // Card HASH-CONTRACTS: a callee whose declared literal-argument
            // premise this call site does not satisfy arrives here rewritten,
            // exactly as uncontracted as any callee with no declaration at all.
            // Only the wording distinguishes the two, so that a report can tell
            // "nothing was declared" from "the declaration does not cover this
            // call".
            if let Some(callee) = literal_premise::unmet_callee(function) {
                return LegacyInstruction::Unknown {
                    description: format!(
                        "call to `{callee}` outside its declared literal-argument premise"
                    ),
                    writes: self.refused_writes(&texts),
                };
            }
            // Card LOADER-ABI: a callee whose declared contract this
            // translation unit's own C declaration contradicts arrives here
            // rewritten, exactly as uncontracted as a callee with no
            // declaration at all. The wording carries the refusal key, so a
            // report can tell a withdrawn declaration from an absent one.
            if let Some((refusal, callee)) = declared_abi::withdrawn_callee(function) {
                return LegacyInstruction::Unknown {
                    description: format!(
                        "call to `{callee}` whose declared contract was withdrawn against its C \
                         declaration in this translation unit ({})",
                        refusal.as_str()
                    ),
                    writes: self.refused_writes(&texts),
                };
            }
            // Card LOADER-ABI-MODES: likewise for a declaration the callee's
            // own body in this unit contradicts. The two withdrawals carry
            // distinct wordings so a report can tell a signature disagreement
            // from a custody-duty contradiction.
            if let Some((refusal, callee)) = declared_modes::withdrawn_callee(function) {
                return LegacyInstruction::Unknown {
                    description: format!(
                        "call to `{callee}` whose declared contract was withdrawn against the \
                         summary derived from its body in this translation unit ({})",
                        refusal.as_str()
                    ),
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

    fn text(&self) -> Option<&str> {
        self.tokens
            .get(self.cursor)
            .map(|token| token.text.as_str())
    }
}

/// Whether `tokens` is exactly one balanced parenthesized group carrying no
/// call, no assignment and no comma — the shape of a cast applied to the value
/// that follows it. An empty slice is not a cast; nor is anything with a
/// second top-level group.
fn is_leading_cast(tokens: &[&str]) -> bool {
    if tokens.first() != Some(&"(") || tokens.last() != Some(&")") || tokens.len() < 3 {
        return false;
    }
    let mut depth = 0usize;
    for (index, text) in tokens.iter().enumerate() {
        match *text {
            "(" => depth += 1,
            ")" => {
                depth = match depth.checked_sub(1) {
                    Some(depth) => depth,
                    None => return false,
                };
                if depth == 0 && index + 1 != tokens.len() {
                    return false;
                }
            }
            "=" | "," | "[" | "]" | "{" | "}" | ";" => return false,
            _ => {}
        }
    }
    depth == 0 && find_call(tokens).is_none()
}

fn find_call<'a>(tokens: &'a [&'a str]) -> Option<(usize, &'a str)> {
    tokens.windows(2).enumerate().find_map(|(index, pair)| {
        (is_identifier(pair[0]) && pair[1] == "(").then_some((index, pair[0]))
    })
}

/// C type-operator tokens, distinguished lexically from ordinary callees.
/// This list alone proves no operand unevaluated: sizeof can evaluate VLA
/// bounds. The fixed scalar query rule establishes its narrower typed premise.
pub(crate) const TYPE_OPERATORS: [&str; 6] = [
    "sizeof",
    "_Alignof",
    "alignof",
    "typeof",
    "__typeof",
    "__typeof__",
];

/// The name of a call in `texts` other than the one whose callee is at
/// `call_index`, ignoring the C type operators (card O04-B09B10).
///
/// [`find_call`] returns the *first* call of a statement and the modeling below
/// covers exactly that one, so any further call would have its acquisitions,
/// releases and transfers silently dropped. Reporting it is strictly more
/// refusing: this can only withdraw a projection, never create one.
fn nested_call<'a>(texts: &[&'a str], call_index: usize) -> Option<&'a str> {
    texts.windows(2).enumerate().find_map(|(index, pair)| {
        (index != call_index
            && is_identifier(pair[0])
            && pair[1] == "("
            && !TYPE_OPERATORS.contains(&pair[0]))
        .then_some(pair[0])
    })
}

fn contains_call(tokens: &[Token]) -> bool {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    find_call(&texts).is_some()
}

/// The tokens of `texts[from..]` that no admitted borrowed pointer-field
/// actual covers.
///
/// Every admitted span is a *whole* call argument, so deleting it leaves the
/// surrounding punctuation adjacent (`( ,` or `( )`) and can never manufacture
/// a unary `*` or a member `.` out of neighbours that did not have one.
fn outside_spans<'a>(texts: &[&'a str], spans: &[(usize, usize)], from: usize) -> Vec<&'a str> {
    texts
        .iter()
        .enumerate()
        .skip(from)
        .filter(|(index, _)| {
            !spans
                .iter()
                .any(|(start, end)| index >= start && index < end)
        })
        .map(|(_, text)| *text)
        .collect()
}

/// Whether a declaration statement's every bracket group is an integer constant
/// bound (or an empty `[]`), so that the declaration allocates automatic
/// storage and evaluates nothing. A variable-length bound, a designator or a
/// function declarator is refused.
fn constant_array_declaration(texts: &[&str]) -> bool {
    if texts
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "="))
    {
        return false;
    }
    let mut cursor = 0usize;
    while cursor < texts.len() {
        if texts[cursor] != "[" {
            cursor += 1;
            continue;
        }
        let Some(close) = texts.iter().skip(cursor).position(|text| *text == "]") else {
            return false;
        };
        let bound = &texts[cursor + 1..cursor + close];
        if !bound
            .iter()
            .all(|text| text.bytes().all(|byte| byte.is_ascii_alphanumeric()) && is_numeric(text))
        {
            return false;
        }
        cursor += close + 1;
    }
    true
}

fn is_numeric(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// This is a lexical exclusion boundary, not a proof of C memory safety.
/// Only multiplication with a plain left operand is identifiable here; a star
/// after a closing parenthesis may follow a typedef cast and remains unknown.
fn contains_unmodeled_access(texts: &[&str]) -> bool {
    texts.windows(2).any(|pair| pair == ["-", ">"])
        || texts.iter().enumerate().any(|(index, text)| match *text {
            "[" | "]" => true,
            "." => {
                !texts
                    .get(index.wrapping_sub(1))
                    .is_some_and(|left| left.bytes().all(|byte| byte.is_ascii_digit()))
                    || !texts
                        .get(index + 1)
                        .is_some_and(|right| right.bytes().all(|byte| byte.is_ascii_digit()))
            }
            "*" => !texts.get(index.wrapping_sub(1)).is_some_and(|left| {
                (is_identifier(left) || left.bytes().all(|byte| byte.is_ascii_digit()))
                    && !matches!(
                        *left,
                        "sizeof"
                            | "_Alignof"
                            | "alignof"
                            | "__extension__"
                            | "__real__"
                            | "__real"
                            | "__imag__"
                            | "__imag"
                    )
            }),
            _ => false,
        })
}

fn supported_neutral_expression(tokens: &[Token]) -> bool {
    !contains_call(tokens)
        && !contains_braces(tokens)
        && !contains_unmodeled_access(
            &tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
        )
}

fn contains_braces(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.text.as_str(), "{" | "}"))
}

/// Is a controlling expression the single literal token `0`?
///
/// This is the whole key of the constant `do … while (0)` rule (card
/// `C-GATEWAY-CONSTANT-DO-WHILE`). It is deliberately a **token** test and
/// deliberately the narrowest one:
///
/// * Standing constraint 5a forbids deciding control flow from an *evaluated*
///   expression, so nothing here folds constants, reads a binding's value or
///   consults a scalar fact. `while (x)` with `x` known zero, `while (1 - 1)`
///   and `while ((0))` all answer `false` and keep the back edge.
/// * Answering `false` is the conservative answer. Keeping the back edge
///   over-approximates the paths, which can only *add* refusals and
///   accusations; answering `true` removes paths, which is the admitting
///   direction invariant 1 governs. Every widening of this predicate
///   therefore needs its own measured re-pin.
/// * Only the spelling `0` is claimed. `0u`, `0L`, `0x0` and `'\0'` are
///   literal zeros too and are **not** claimed here
///   (`CONSTANT-DO-WHILE-ZERO-SPELLINGS`).
fn condition_is_literal_zero(tokens: &[Token]) -> bool {
    matches!(tokens, [token] if token.text == "0")
}

/// Split a `for` header into its three clauses.
///
/// The grouping tokens are consumed by the depth tracking and do **not** reach
/// the clause vectors. That is load-bearing for the `init` and `increment`
/// clauses, whose classification by [`Parser::classify_simple`] this card does
/// not touch, and it is why [`for_condition_tokens`] exists: the condition is
/// classified from the original spelling instead, so that a call, a subscript
/// or a parenthesized group in a `for` condition is visible to the predicates
/// that must refuse it. See `W4-EXPR-POS-FOR-CLAUSES` in
/// `docs/reports/remaining-w4-expr-pos/README.md`.
fn split_for_clauses(tokens: &[Token]) -> Option<[Vec<Token>; 3]> {
    let mut clauses = vec![Vec::new()];
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            ";" if parentheses == 0 && brackets == 0 => clauses.push(Vec::new()),
            _ => clauses.last_mut()?.push(token.clone()),
        }
    }
    clauses.try_into().ok()
}

/// The `for` header's condition clause, **with its grouping tokens intact**.
///
/// A measured fail-open repair (card W4-EXPR-POS). `split_for_clauses` drops
/// `(`, `)`, `[` and `]`, so `for (; external(n); )` reached
/// `supported_neutral_expression` as the two tokens `external n`, in which
/// `contains_call` finds no call and `contains_unmodeled_access` finds no
/// subscript: the position was classified `NoOp` and the unmodelled call
/// disappeared. Every other position was already classified from its original
/// spelling; this makes the `for` condition agree with them.
///
/// This is a **refusal** repair — it can only turn an accepted condition into a
/// refused one — and it is confined to the condition. The `init` and
/// `increment` clauses keep the stripped spelling they have today, byte for
/// byte.
fn for_condition_tokens(tokens: &[Token]) -> Option<Vec<Token>> {
    let mut clauses = vec![Vec::new()];
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            ";" if parentheses == 0 && brackets == 0 => {
                clauses.push(Vec::new());
                continue;
            }
            _ => {}
        }
        clauses.last_mut()?.push(token.clone());
    }
    let [_, condition, _]: [Vec<Token>; 3] = clauses.try_into().ok()?;
    Some(condition)
}

fn pointer_null_condition(tokens: &[Token]) -> Option<PointerNullCondition> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if !supported_neutral_expression(tokens) {
        return None;
    }
    null_condition_text(&texts)
}

fn null_condition_text(texts: &[&str]) -> Option<PointerNullCondition> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        let mut condition = null_condition_text(rest)?;
        condition.true_is_null = !condition.true_is_null;
        return Some(condition);
    }
    if let Some(source) = canonical_place(texts) {
        return Some(PointerNullCondition {
            source,
            true_is_null: false,
        });
    }
    let mut depth = 0usize;
    for index in 0..texts.len().saturating_sub(1) {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" | "!" if depth == 0 && texts[index + 1] == "=" => {
                let left = &texts[..index];
                let right = &texts[index + 2..];
                let source = if is_null_constant(left) {
                    canonical_place(right)?
                } else if is_null_constant(right) {
                    canonical_place(left)?
                } else {
                    return None;
                };
                return Some(PointerNullCondition {
                    source,
                    true_is_null: texts[index] == "=",
                });
            }
            _ => {}
        }
    }
    None
}

// Only erase parentheses that enclose the complete expression. In particular,
// `(p) + offset` must not become the resource place `p`.
fn strip_parentheses<'a>(mut tokens: &'a [&'a str]) -> &'a [&'a str] {
    while tokens.first() == Some(&"(") && tokens.last() == Some(&")") {
        let mut depth = 0usize;
        let mut closes_at_end = false;
        for (index, token) in tokens.iter().enumerate() {
            match *token {
                "(" => depth += 1,
                ")" => {
                    let Some(next) = depth.checked_sub(1) else {
                        return tokens;
                    };
                    depth = next;
                    if depth == 0 {
                        closes_at_end = index == tokens.len() - 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            break;
        }
        tokens = &tokens[1..tokens.len() - 1];
    }
    tokens
}

fn is_null_constant(tokens: &[&str]) -> bool {
    let tokens = strip_parentheses(tokens);
    matches!(tokens, ["NULL" | "nullptr" | "0"])
        || tokens
            .strip_prefix(&["(", "void", "*", ")"])
            .is_some_and(|rest| strip_parentheses(rest) == ["0"])
}

pub(crate) fn call_arguments<'a>(
    tokens: &'a [&'a str],
    call_index: usize,
) -> Option<Vec<&'a [&'a str]>> {
    Some(
        call_argument_spans(tokens, call_index)?
            .into_iter()
            .map(|(start, end)| &tokens[start..end])
            .collect(),
    )
}

/// The half-open token spans of one call's arguments, in order.
///
/// [`call_arguments`] is exactly this, resolved against `tokens`; card
/// O04-TRACE needs the indices themselves so an argument can be recognised as
/// *being* a nested call at a known position.
pub(crate) fn call_argument_spans(
    tokens: &[&str],
    call_index: usize,
) -> Option<Vec<(usize, usize)>> {
    let mut arguments = Vec::new();
    let mut start = call_index + 2;
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut cursor = start;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "(" => parentheses += 1,
            ")" if parentheses == 0 && brackets == 0 => {
                if cursor > start {
                    arguments.push((start, cursor));
                }
                return Some(arguments);
            }
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            "," if parentheses == 0 && brackets == 0 => {
                arguments.push((start, cursor));
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// The modeled place of one actual at a declared `Borrowed` position.
pub(crate) fn borrowed_argument_place(argument: &[&str]) -> Option<String> {
    contract_argument_place(argument, ResourceParameterMode::Borrowed)
}

fn contract_argument_place(argument: &[&str], mode: ResourceParameterMode) -> Option<String> {
    match mode {
        // Retain direct borrowed names for the enclosing Out-slot identity
        // guard; the callee's borrowed resource effect still consumes nothing.
        ResourceParameterMode::Borrowed => canonical_place(argument),
        ResourceParameterMode::Out { .. } => {
            let argument = strip_parentheses(argument);
            if argument.first() == Some(&"&") {
                canonical_place(&argument[1..])
            } else {
                // A bare pointer may forward the current function's out slot.
                // Naming *formal is not permission to write it: transfer must
                // find that exact slot sanctioned and its pointer unchanged.
                match argument {
                    [name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
                        Some(format!("*{name}"))
                    }
                    _ => None,
                }
            }
        }
        ResourceParameterMode::Owned { .. }
        | ResourceParameterMode::Consumed { .. }
        | ResourceParameterMode::Released { .. }
        | ResourceParameterMode::Stored { .. }
        | ResourceParameterMode::Removed { .. } => canonical_place(argument),
    }
}

fn direct_assignment_destination(tokens: &[&str]) -> Option<String> {
    if tokens.first() == Some(&"*") {
        // A single dereference of a plain identifier is the only indirect
        // destination this importer names: `*out = …` becomes the place
        // `*out`. It is *not* modeled here — [`transfer`] rejects it unless the
        // analysis in force sanctions `*out` as an out-parameter slot — but
        // naming it is what lets an out-parameter contract describe the store.
        // Anything deeper (`*p->field`, `**p`, `*(p + 1)`) stays unnamed.
        return match tokens {
            ["*", name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
                Some(format!("*{name}"))
            }
            _ => None,
        };
    }
    (0..tokens.len()).find_map(|start| canonical_place(&tokens[start..]))
}

/// Whether `place` denotes a store through a pointer rather than a variable.
/// Such a place is modeled only when the contract in force declares it an
/// out-parameter slot; see [`EntryCustody::out_slots`].
fn is_indirect_place(place: &str) -> bool {
    place.starts_with('*')
}

fn canonical_place(tokens: &[&str]) -> Option<String> {
    let tokens = strip_parentheses(tokens);
    let first = tokens.first().copied()?;
    if !is_identifier(first) || matches!(first, "NULL" | "nullptr") {
        return None;
    }
    let mut place = first.to_owned();
    let mut cursor = 1;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "." if tokens
                .get(cursor + 1)
                .is_some_and(|field| is_identifier(field)) =>
            {
                place.push('.');
                place.push_str(tokens[cursor + 1]);
                cursor += 2;
            }
            "-" if tokens.get(cursor + 1) == Some(&">")
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|field| is_identifier(field)) =>
            {
                place.push_str("->");
                place.push_str(tokens[cursor + 2]);
                cursor += 3;
            }
            "[" if tokens
                .get(cursor + 1)
                .is_some_and(|index| index.bytes().all(|byte| byte.is_ascii_digit()))
                && tokens.get(cursor + 2) == Some(&"]") =>
            {
                place.push('[');
                place.push_str(tokens[cursor + 1]);
                place.push(']');
                cursor += 3;
            }
            _ => return None,
        }
    }
    Some(place)
}

/// Whether a token could be spelled as an identifier. Reserved words are
/// *included*: this is a lexical shape test, and `find_call` relies on it to
/// treat `sizeof (` as call-shaped. [`crate::neutral_position`] uses the same
/// test so that the occurrences it must account for are exactly the ones
/// [`contains_call`] refuses on.
pub(crate) fn is_identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
}

struct CfgBuilder {
    nodes: Vec<LegacyCfgNode>,
    labels: BTreeMap<String, LegacyNodeId>,
    pending_gotos: Vec<(LegacyNodeId, String, LegacyNodeId)>,
    break_targets: Vec<LegacyNodeId>,
    continue_targets: Vec<LegacyNodeId>,
    switch_cases: Vec<SwitchTargets>,
}

#[derive(Default)]
struct SwitchTargets {
    entries: Vec<LegacyNodeId>,
    has_default: bool,
}

impl CfgBuilder {
    fn build(
        function: &str,
        statements: &[Statement],
        publications: PublicationFacts,
    ) -> LegacyCfg {
        let mut builder = Self {
            nodes: Vec::new(),
            labels: BTreeMap::new(),
            pending_gotos: Vec::new(),
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            switch_cases: Vec::new(),
        };
        let exit = builder.push(LegacyInstruction::NoOp, Vec::new(), 0, true);
        let entry = builder.compile_sequence(statements, exit);
        for (node, label, fallback) in std::mem::take(&mut builder.pending_gotos) {
            if let Some(target) = builder.labels.get(&label).copied() {
                builder.nodes[node].successors = vec![target];
            } else {
                builder.nodes[node].instruction = LegacyInstruction::Unknown {
                    description: format!("goto to unresolved label `{label}`"),
                    writes: RefusedWrites::Unbounded,
                };
                builder.nodes[node].successors = vec![fallback];
            }
        }
        LegacyCfg {
            function: function.to_owned(),
            entry,
            nodes: builder.nodes,
            publications,
            conditional_names: BTreeSet::new(),
        }
    }

    fn compile_sequence(
        &mut self,
        statements: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        statements
            .iter()
            .rev()
            .fold(continuation, |next, statement| {
                self.compile(statement, next)
            })
    }

    fn compile(&mut self, statement: &Statement, continuation: LegacyNodeId) -> LegacyNodeId {
        match statement {
            Statement::Instruction(instruction, line) => {
                self.push(instruction.clone(), vec![continuation], *line, false)
            }
            Statement::Block(statements) => self.compile_sequence(statements, continuation),
            Statement::Return {
                line,
                writes,
                expression_supported,
                returned_place,
                returned_evaluation,
                returned_evaluation_escapes,
            } => {
                // Card C-GATEWAY-RETURN-CALL-W4. `return f(args);` becomes the
                // two nodes `PLACE = f(args);` and `return PLACE;` — the same C,
                // and the pair the analyzer has decided since Cycle 9. The call
                // node is an ordinary interior node; the exit is an ordinary
                // `EscapeIfOwned`, so `observe_exit` needs no arm for a call
                // result.
                if let Some(call) = returned_evaluation {
                    let exit_instruction = if *returned_evaluation_escapes {
                        LegacyInstruction::EscapeIfOwned {
                            source: return_call::RETURN_RESULT_PLACE.to_owned(),
                        }
                    } else {
                        LegacyInstruction::NoOp
                    };
                    let exit = self.push(exit_instruction, Vec::new(), *line, true);
                    return self.push((**call).clone(), vec![exit], *line, false);
                }
                let instruction = if !*expression_supported {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in return expression"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                } else if let Some(source) = returned_place {
                    LegacyInstruction::EscapeIfOwned {
                        source: source.clone(),
                    }
                } else {
                    LegacyInstruction::NoOp
                };
                self.push(instruction, Vec::new(), *line, true)
            }
            Statement::If {
                line,
                writes,
                condition_supported,
                condition_call,
                null_condition,
                distinct_condition,
                then_body,
                else_body,
            } => {
                let then_entry = self.compile_sequence(then_body, continuation);
                let else_entry = self.compile_sequence(else_body, continuation);
                let instruction = if let Some(condition) = null_condition {
                    LegacyInstruction::BranchOnNull {
                        source: condition.source.clone(),
                        true_is_null: condition.true_is_null,
                    }
                } else if let Some(condition) = distinct_condition {
                    // C02-P-ALIAS. A supported neutral condition would have
                    // become `NoOp`; this carries the same absence of custody
                    // effect and additionally lets the distinct edge discharge
                    // one may-alias obligation.
                    LegacyInstruction::BranchOnDistinct {
                        left: condition.left.clone(),
                        right: condition.right.clone(),
                        true_is_distinct: condition.true_is_distinct,
                    }
                } else if *condition_supported || condition_call.is_some() {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in branch condition"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                let branch = self.push(instruction, vec![then_entry, else_entry], *line, false);
                condition_call.as_ref().map_or(branch, |call| {
                    self.push((**call).clone(), vec![branch], *line, false)
                })
            }
            Statement::While {
                line,
                writes,
                condition_supported,
                condition_call,
                null_condition,
                body,
            } => {
                let instruction = if let Some(condition) = null_condition {
                    LegacyInstruction::BranchOnNull {
                        source: condition.source.clone(),
                        true_is_null: condition.true_is_null,
                    }
                } else if let Some(call) = condition_call {
                    (**call).clone()
                } else if *condition_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in loop condition"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                let header = self.push(instruction, Vec::new(), *line, false);
                self.break_targets.push(continuation);
                self.continue_targets.push(header);
                let body_entry = self.compile_sequence(body, header);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = vec![body_entry, continuation];
                // SCALAR-ROUTE-EXIT-EDGE: index 1 is the successor that leaves
                // the loop.
                self.nodes[header].loop_exit_edge = Some(1);
                header
            }
            Statement::For {
                line,
                writes,
                init,
                condition_always_true,
                condition_supported,
                condition_call,
                null_condition,
                increment,
                body,
            } => {
                let instruction = condition_call.as_ref().map_or_else(
                    || {
                        branch_instruction(
                            *condition_supported,
                            null_condition,
                            writes,
                            "unmodeled call, aggregate or indirect access in for-loop condition",
                        )
                    },
                    |call| (**call).clone(),
                );
                let header = self.push(instruction, Vec::new(), *line, false);
                let increment_entry = increment.as_ref().map_or(header, |clause| {
                    let (instruction, line) = clause.as_ref();
                    self.push(instruction.clone(), vec![header], *line, false)
                });
                self.break_targets.push(continuation);
                self.continue_targets.push(increment_entry);
                let body_entry = self.compile_sequence(body, increment_entry);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = if *condition_always_true {
                    vec![body_entry]
                } else {
                    vec![body_entry, continuation]
                };
                // SCALAR-ROUTE-EXIT-EDGE. A `for (;;)` header has no exit
                // successor to protect — that route removes it at construction
                // and is a different, unowned neighbour
                // (`FOREVER-FOR-EXIT-EDGE-UNOWNED`), deliberately untouched.
                self.nodes[header].loop_exit_edge = (!*condition_always_true).then_some(1);
                init.as_ref().map_or(header, |clause| {
                    let (instruction, line) = clause.as_ref();
                    self.push(instruction.clone(), vec![header], *line, false)
                })
            }
            Statement::DoWhile {
                line,
                writes,
                condition_supported,
                condition_call,
                condition_is_literal_zero,
                null_condition,
                body,
            } => {
                let instruction = condition_call.as_ref().map_or_else(
                    || {
                        branch_instruction(
                            *condition_supported,
                            null_condition,
                            writes,
                            "unmodeled call, aggregate or indirect access in do-while condition",
                        )
                    },
                    |call| (**call).clone(),
                );
                let header = self.push(instruction, Vec::new(), *line, false);
                self.break_targets.push(continuation);
                self.continue_targets.push(header);
                let body_entry = self.compile_sequence(body, header);
                self.continue_targets.pop();
                self.break_targets.pop();
                // `do S while (0)` executes S exactly once (C11 §6.8.5.2), so
                // the controlling expression has the single successor
                // `continuation`: the CFG is sequential composition, and the
                // back edge to `body_entry` would be a path the C does not
                // have. Removing it is the admitting direction (invariant 1),
                // which is why the key is the literal token and the rule is
                // proved in `QCore.ConstantDoWhile`. A `continue` in the body
                // still targets `header`, which is exactly C's "jump to the
                // controlling expression", and so now exits.
                self.nodes[header].successors = if *condition_is_literal_zero {
                    vec![continuation]
                } else {
                    vec![body_entry, continuation]
                };
                // SCALAR-ROUTE-EXIT-EDGE. Under card
                // `C-GATEWAY-CONSTANT-DO-WHILE`'s rule the continuation is the
                // header's only successor, so it is index 0; otherwise index 1,
                // as for `while`. Either way it is the edge that leaves.
                self.nodes[header].loop_exit_edge = Some(usize::from(!*condition_is_literal_zero));
                body_entry
            }
            Statement::Switch {
                line,
                writes,
                expression_supported,
                expression_call,
                body,
            } => {
                self.break_targets.push(continuation);
                self.switch_cases.push(SwitchTargets::default());
                let _body_entry = self.compile_sequence(body, continuation);
                let targets = self.switch_cases.pop().expect("switch target scope");
                self.break_targets.pop();
                let mut successors = targets.entries;
                if !targets.has_default || successors.is_empty() {
                    successors.push(continuation);
                }
                successors.sort_unstable();
                successors.dedup();
                let instruction = if let Some(call) = expression_call {
                    (**call).clone()
                } else if *expression_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in switch expression"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                self.push(instruction, successors, *line, false)
            }
            Statement::Case {
                line,
                writes,
                expression_supported,
                is_default,
            } => {
                let instruction = if *expression_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in case expression"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                let entry = self.push(instruction, vec![continuation], *line, false);
                if let Some(targets) = self.switch_cases.last_mut() {
                    targets.entries.push(entry);
                    targets.has_default |= *is_default;
                } else {
                    self.nodes[entry].instruction = LegacyInstruction::Unknown {
                        description: "case label outside switch".to_owned(),
                        writes: RefusedWrites::Unbounded,
                    };
                }
                entry
            }
            Statement::Label { line, name } => {
                let entry = self.push(LegacyInstruction::NoOp, vec![continuation], *line, false);
                if self.labels.insert(name.clone(), entry).is_some() {
                    self.nodes[entry].instruction = LegacyInstruction::Unknown {
                        description: format!("duplicate label `{name}`"),
                        writes: RefusedWrites::Unbounded,
                    };
                }
                entry
            }
            Statement::Goto { line, label } => {
                let node = self.push(LegacyInstruction::NoOp, Vec::new(), *line, false);
                if let Some(target) = self.labels.get(label).copied() {
                    self.nodes[node].successors = vec![target];
                } else {
                    self.pending_gotos.push((node, label.clone(), continuation));
                }
                node
            }
            Statement::Break { line } => {
                if let Some(target) = self.break_targets.last().copied() {
                    self.push(LegacyInstruction::NoOp, vec![target], *line, false)
                } else {
                    self.push(
                        LegacyInstruction::Unknown {
                            description: "break outside loop or switch".to_owned(),
                            writes: RefusedWrites::Unbounded,
                        },
                        vec![continuation],
                        *line,
                        false,
                    )
                }
            }
            Statement::Continue { line } => {
                if let Some(target) = self.continue_targets.last().copied() {
                    self.push(LegacyInstruction::NoOp, vec![target], *line, false)
                } else {
                    self.push(
                        LegacyInstruction::Unknown {
                            description: "continue outside loop".to_owned(),
                            writes: RefusedWrites::Unbounded,
                        },
                        vec![continuation],
                        *line,
                        false,
                    )
                }
            }
        }
    }

    fn push(
        &mut self,
        instruction: LegacyInstruction,
        successors: Vec<LegacyNodeId>,
        line: usize,
        is_exit: bool,
    ) -> LegacyNodeId {
        let id = self.nodes.len();
        self.nodes.push(LegacyCfgNode {
            instruction,
            successors,
            line,
            is_exit,
            // Set only by the three loop headers, where the successors are
            // assigned after the body is compiled.
            loop_exit_edge: None,
        });
        id
    }
}

fn branch_instruction(
    condition_supported: bool,
    null_condition: &Option<PointerNullCondition>,
    writes: &RefusedWrites,
    unsupported_description: &str,
) -> LegacyInstruction {
    if let Some(condition) = null_condition {
        LegacyInstruction::BranchOnNull {
            source: condition.source.clone(),
            true_is_null: condition.true_is_null,
        }
    } else if condition_supported {
        LegacyInstruction::NoOp
    } else {
        LegacyInstruction::Unknown {
            description: unsupported_description.to_owned(),
            writes: writes.clone(),
        }
    }
}

// `pub(crate)` since card HYGIENE-14 so `borrow_liveness` classifies the
// analyzer's own status bits rather than restating their numeric values. The
// values, the join and every use here are unchanged.
pub(crate) const LIVE: u8 = 1;
pub(crate) const RELEASED: u8 = 2;
pub(crate) const ESCAPED: u8 = 4;
pub(crate) const CONTAINED: u8 = 8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AbstractPointer {
    targets: BTreeSet<usize>,
    unknown: bool,
    may_be_null: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AbstractState {
    pointers: BTreeMap<String, AbstractPointer>,
    resources: BTreeMap<usize, u8>,
    nullable_sites: BTreeSet<usize>,
    conditional_replacements: BTreeMap<usize, BTreeSet<usize>>,
    contained: BTreeMap<usize, BTreeSet<usize>>,
    /// O00 blocker B08: `owner site -> field name -> child sites` recorded by
    /// an admitted aggregate field store. Deliberately separate from
    /// `contained`: escaping the owner escapes its live linked children, while
    /// *releasing* the owner does not cascade, because a plain C field store
    /// carries no declared destructor obligation (premise P-B08-SHALLOW).
    field_links: BTreeMap<usize, BTreeMap<String, BTreeSet<usize>>>,
    /// Custody retained after a rejected effect; its residual is not a proven
    /// leak because the unmodeled action may have discharged it.
    unresolved_effect_sites: BTreeSet<usize>,
    widened: bool,
    /// Whether some instruction on *this* path was refused or reported an
    /// unresolved effect (O04). It is the per-path counterpart of the
    /// function-wide `Unknown` status: a path carrying it can support no
    /// custody claim at the exits it reaches. Joined by disjunction, so it is
    /// monotone and the work list still terminates.
    unmodeled: bool,
}

const MAX_ALIAS_TARGETS: usize = 8;

impl AbstractState {
    fn join(&mut self, incoming: &Self) -> bool {
        let before = self.clone();
        let variables = self
            .pointers
            .keys()
            .chain(incoming.pointers.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for variable in variables {
            match (
                self.pointers.get(&variable).cloned(),
                incoming.pointers.get(&variable),
            ) {
                (Some(mut current), Some(other)) => {
                    current.targets.extend(other.targets.iter().copied());
                    current.unknown |= other.unknown;
                    current.may_be_null |= other.may_be_null;
                    if current.targets.len() > MAX_ALIAS_TARGETS {
                        current.targets.clear();
                        current.unknown = true;
                        self.widened = true;
                    }
                    self.pointers.insert(variable, current);
                }
                (Some(mut current), None) => {
                    current.unknown = true;
                    current.may_be_null = true;
                    self.pointers.insert(variable, current);
                }
                (None, Some(other)) => {
                    let mut joined = other.clone();
                    joined.unknown = true;
                    joined.may_be_null = true;
                    self.pointers.insert(variable, joined);
                }
                (None, None) => {}
            }
        }
        for (&site, &status) in &incoming.resources {
            self.resources
                .entry(site)
                .and_modify(|current| *current |= status)
                .or_insert(status);
        }
        self.nullable_sites
            .extend(incoming.nullable_sites.iter().copied());
        self.unresolved_effect_sites
            .extend(incoming.unresolved_effect_sites.iter().copied());
        for pointer in self.pointers.values() {
            if pointer.may_be_null {
                self.nullable_sites.extend(pointer.targets.iter().copied());
            }
        }
        for (&site, replaced) in &incoming.conditional_replacements {
            self.conditional_replacements
                .entry(site)
                .or_default()
                .extend(replaced.iter().copied());
        }
        for (&owner, children) in &incoming.contained {
            self.contained
                .entry(owner)
                .or_default()
                .extend(children.iter().copied());
        }
        self.join_field_links(incoming);
        self.widened |= incoming.widened;
        self.unmodeled |= incoming.unmodeled;
        *self != before
    }
}

/// C02-P-ALIAS: the may-alias facts in force on entry to one node.
///
/// They are computed by [`alias_facts`], a **separate** fixed point over the
/// same CFG, and never live in [`AbstractState`]. That separation is deliberate
/// and was forced by measurement: carrying them in the custody state changed
/// how often `AbstractState::join` reported a change, which reordered the
/// custody work list, and `report_leaks` runs on *intermediate* iterates of an
/// exit node rather than on its settled state. Two leak diagnostics appeared in
/// one corpus function from that reordering alone, with the rule's own refusal
/// disabled. Keeping the facts out of the custody state makes this card
/// scheduling-neutral; the underlying order-sensitivity of in-loop leak
/// reporting is recorded as follow-on `C02-P-ALIAS-LEAK-ORDER` and deliberately
/// not repaired here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NodeAliasFacts {
    /// Names released on some path reaching this node. A *may* fact, joined by
    /// union. Deliberately never cleared by a rebinding: reassigning the name
    /// that released an object does not resurrect the object.
    released: BTreeSet<String>,
    /// Name pairs a pointer-inequality guard proved distinct on *every* path
    /// reaching this node, each stored with its two names in ascending order.
    /// A *must* fact, joined by intersection and dropped when either of its
    /// names is rebound.
    distinct: BTreeSet<(String, String)>,
}

impl NodeAliasFacts {
    /// Join an incoming edge's facts. Returns whether anything changed.
    fn absorb(&mut self, incoming: &Self) -> bool {
        let before = self.clone();
        self.released.extend(incoming.released.iter().cloned());
        self.distinct
            .retain(|pair| incoming.distinct.contains(pair));
        *self != before
    }

    /// Whether a pointer-inequality guard proved these two names distinct here.
    fn proved_distinct(&self, one: &str, other: &str) -> bool {
        let pair = if one <= other {
            (one.to_owned(), other.to_owned())
        } else {
            (other.to_owned(), one.to_owned())
        };
        self.distinct.contains(&pair)
    }
}

/// The plain name one instruction *releases*, if any.
///
/// Exactly the sites at which [`apply_resource_action`] runs with `escape`
/// false: an explicit `Release`, and every declared `Released` position of a
/// contracted call. Consuming a value is deliberately outside v1
/// (`C02-P-ALIAS-CONSUME`).
fn released_names(instruction: &LegacyInstruction) -> Vec<&str> {
    // Only a *plain name* is related by this rule: releasing `p->field`
    // releases a member, not the name `p`.
    let plain = |place: &str| crate::may_alias::place_root(place) == Some(place);
    match instruction {
        LegacyInstruction::Release { source } if plain(source) => vec![source.as_str()],
        LegacyInstruction::ContractCall {
            arguments,
            contract,
            ..
        } => contract
            .parameters
            .iter()
            .zip(arguments)
            .filter_map(|(mode, argument)| {
                matches!(mode, ResourceParameterMode::Released { .. })
                    .then_some(argument.as_deref()?)
                    .filter(|place| plain(place))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The names one instruction binds, whose guard facts it therefore invalidates.
///
/// Exactly the destinations [`transfer`] and [`create_resource`] write.
fn rebound_names(instruction: &LegacyInstruction) -> Vec<&str> {
    let mut names = match instruction {
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::OpaqueValue { destination, .. }
        | LegacyInstruction::ForgetScalar { destination }
        | LegacyInstruction::Clear { destination } => vec![destination.as_str()],
        LegacyInstruction::ContractCall {
            destination,
            arguments,
            contract,
            ..
        } => {
            let mut names = destination.as_deref().into_iter().collect::<Vec<_>>();
            names.extend(contract.parameters.iter().zip(arguments).filter_map(
                |(mode, argument)| {
                    matches!(mode, ResourceParameterMode::Out { .. })
                        .then_some(argument.as_deref()?)
                },
            ));
            names
        }
        _ => Vec::new(),
    };
    // A guard is about two plain names, so only a plain destination can
    // invalidate one; `*out` and `p->field` name no such value.
    names.retain(|name| crate::may_alias::place_root(name) == Some(*name));
    names
}

/// C02-P-ALIAS: the may-alias facts on entry to every node of one CFG.
///
/// A separate forward fixed point, independent of custody: which releases have
/// run and which pointer inequalities a branch proved depend only on the CFG's
/// shape and its instructions, never on the abstract state. Releases only grow
/// and guards only shrink, both over finite sets, so it terminates.
fn alias_facts(cfg: &LegacyCfg) -> Vec<NodeAliasFacts> {
    let mut entry = vec![None::<NodeAliasFacts>; cfg.nodes.len()];
    entry[cfg.entry] = Some(NodeAliasFacts::default());
    let mut work = VecDeque::from([cfg.entry]);
    while let Some(node_id) = work.pop_front() {
        let Some(facts) = entry[node_id].clone() else {
            continue;
        };
        let node = &cfg.nodes[node_id];
        let mut out = facts;
        for name in released_names(&node.instruction) {
            out.released.insert(name.to_owned());
        }
        for name in rebound_names(&node.instruction) {
            out.distinct
                .retain(|(left, right)| left != name && right != name);
        }
        for (edge, &successor) in node.successors.iter().enumerate() {
            let mut edge_out = out.clone();
            if let LegacyInstruction::BranchOnDistinct {
                left,
                right,
                true_is_distinct,
            } = &node.instruction
            {
                let distinct = if edge == 0 {
                    *true_is_distinct
                } else {
                    !*true_is_distinct
                };
                if distinct {
                    let pair = if left <= right {
                        (left.clone(), right.clone())
                    } else {
                        (right.clone(), left.clone())
                    };
                    edge_out.distinct.insert(pair);
                }
            }
            let changed = match &mut entry[successor] {
                Some(current) => current.absorb(&edge_out),
                slot @ None => {
                    *slot = Some(edge_out);
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }
    entry.into_iter().map(Option::unwrap_or_default).collect()
}

/// C02-P-ALIAS: the provenance discharge.
///
/// `place` is bound here to exactly one live identity **this body acquired**.
/// A body-made acquisition returns an object distinct from every object live
/// when it ran, so the projection's own identity for `place` answers the
/// question and no peer's release can reach it. A caller-held entry site is
/// deliberately *not* provenance: two formals carrying declared entry custody
/// are exactly the pair this card exists for.
fn distinct_by_provenance(
    state: &AbstractState,
    caller_sites: &BTreeSet<usize>,
    place: &str,
) -> bool {
    let Some(pointer) = state.pointers.get(place) else {
        return false;
    };
    if pointer.unknown || pointer.may_be_null || pointer.targets.len() != 1 {
        return false;
    }
    pointer.targets.iter().all(|site| {
        !caller_sites.contains(site)
            && state.resources.get(site) == Some(&LIVE)
            && !state.unresolved_effect_sites.contains(site)
    })
}

/// ALIAS-LOCALS: whether the projection's own bindings show these two names
/// denote one object here. See [`crate::may_alias::definitely_aliased`].
///
/// Read-only against the custody state, exactly like [`distinct_by_provenance`],
/// so it cannot perturb the custody work list.
fn definitely_aliased(state: &AbstractState, one: &str, other: &str) -> bool {
    let (Some(left), Some(right)) = (state.pointers.get(one), state.pointers.get(other)) else {
        return false;
    };
    fn denote(pointer: &AbstractPointer) -> crate::may_alias::Denotation<'_> {
        crate::may_alias::Denotation {
            targets: &pointer.targets,
            unknown: pointer.unknown,
            may_be_null: pointer.may_be_null,
        }
    }
    crate::may_alias::definitely_aliased(denote(left), denote(right))
}

/// ALIAS-LOCALS: the pointer-inequality discharge, with the definite-alias
/// exception.
///
/// A `BranchOnDistinct` edge discharges the pair only while the projection has
/// not itself decided that the two names denote one object. When it has, the
/// branch and the identity model contradict each other; refusing is strictly
/// the more refusing of the two answers, and it is the only one that does not
/// let a source-level comparison overrule the projection's own bindings.
fn distinct_by_guard(
    state: &AbstractState,
    alias: &NodeAliasFacts,
    place: &str,
    peer: &str,
) -> bool {
    alias.proved_distinct(place, peer) && !definitely_aliased(state, place, peer)
}

/// C02-P-ALIAS: refuse each modeled read whose base may denote an object some
/// peer's release already destroyed.
///
/// Reported as the existing `UnknownAlias` — no diagnostic code is added,
/// changed or downgraded — naming the read place and the peer that compromises
/// it, in the `owner/source` composite spelling this module already uses. The
/// diagnostic poisons the path, so the exit it reaches supports no custody
/// claim, and the call's own effects still apply exactly as before: this rule
/// can turn `Safe` into `Unknown` and can never hide a leak.
fn require_unaliased_reads(
    function: &str,
    line: usize,
    reads: &[AliasPeers],
    state: &AbstractState,
    caller_sites: &BTreeSet<usize>,
    alias: &NodeAliasFacts,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for read in reads {
        if distinct_by_provenance(state, caller_sites, &read.place) {
            continue;
        }
        for peer in &read.peers {
            if alias.released.contains(peer) && !distinct_by_guard(state, alias, &read.place, peer)
            {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: format!("{}/{peer}", read.place),
                    },
                });
            }
        }
    }
}

impl AbstractState {
    /// Join the B08 field-link relation.
    ///
    /// The union alone would be unsound in the escaping direction: a child
    /// linked on one path only would be carried away by an escape of the
    /// aggregate on a path that never stored it, hiding a leak. Every child in
    /// the symmetric difference is therefore recorded as an unresolved effect,
    /// which makes its disposition `Mixed` and the exit `Unknown`. The union
    /// and `unresolved_effect_sites` both only grow, so the work list still
    /// terminates.
    fn join_field_links(&mut self, incoming: &Self) {
        let owners = self
            .field_links
            .keys()
            .chain(incoming.field_links.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        for owner in owners {
            let mine = self.field_links.get(&owner).cloned().unwrap_or_default();
            let theirs = incoming
                .field_links
                .get(&owner)
                .cloned()
                .unwrap_or_default();
            let fields = mine
                .keys()
                .chain(theirs.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut joined: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
            for field in fields {
                let left = mine.get(&field).cloned().unwrap_or_default();
                let right = theirs.get(&field).cloned().unwrap_or_default();
                if left != right {
                    self.unresolved_effect_sites
                        .extend(left.symmetric_difference(&right).copied());
                }
                let union = left.union(&right).copied().collect::<BTreeSet<_>>();
                if !union.is_empty() {
                    joined.insert(field, union);
                }
            }
            if joined.is_empty() {
                self.field_links.remove(&owner);
            } else {
                self.field_links.insert(owner, joined);
            }
        }
    }
}

/// Custody the caller hands to a function's formal parameters, derived from the
/// contract in force for that function.
///
/// Parameter resources are *caller-owned*: they are seeded live at entry and
/// exempt from exit-leak reporting, because discharging them is the caller's
/// obligation, not the callee's. Their exit disposition is what the summary
/// claims about the call, and [`crate::legacy_summary`] checks that claim.
#[derive(Clone, Debug, Default)]
pub(crate) struct EntryCustody {
    state: AbstractState,
    /// Abstract site bound to each formal parameter, when the contract gives
    /// that parameter observable custody. `None` for `Borrowed`/`Out` modes.
    pub(crate) parameter_sites: Vec<Option<usize>>,
    /// Place a formal declared `Out` writes through, as `*name`. Nothing is
    /// bound at entry — the callee *initializes* the slot — but naming it is
    /// what makes `*name = …` a modeled store instead of an unmodeled one.
    pub(crate) out_slots: Vec<Option<String>>,
    /// Whether the contract's result may be null. A body may return a
    /// possibly-null place only when its own contract says the caller has to
    /// null-check the result; otherwise the escape reports an unresolved alias.
    pub(crate) result_nullable: bool,
}

impl EntryCustody {
    /// Build the entry state a contract prescribes, or `None` when the
    /// parameter list and the contract cannot be lined up exactly.
    pub(crate) fn build(
        cfg: &LegacyCfg,
        parameters: &[Option<String>],
        contract: &ResourceFunctionContract,
    ) -> Option<Self> {
        if contract.parameters.len() != parameters.len() {
            return None;
        }
        let base = first_free_site(cfg);
        let mut custody = Self {
            state: AbstractState::default(),
            parameter_sites: vec![None; parameters.len()],
            out_slots: vec![None; parameters.len()],
            result_nullable: match contract.result {
                ResourceReturnMode::None => false,
                ResourceReturnMode::Owned { nullable }
                | ResourceReturnMode::Reallocated { nullable, .. } => nullable,
            },
        };
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            let nullable = match mode {
                // Nothing is bound: the body may not treat the formal as a
                // resource it already owns.
                ResourceParameterMode::Borrowed => continue,
                // An `Out` formal is *written* by the callee. Nothing is bound
                // at entry either, but the slot it writes through is named so
                // the store can be modeled and observed at every exit.
                ResourceParameterMode::Out { .. } => {
                    let name = parameters[index].as_ref()?;
                    custody.out_slots[index] = Some(format!("*{name}"));
                    continue;
                }
                ResourceParameterMode::Owned { nullable }
                | ResourceParameterMode::Consumed { nullable }
                | ResourceParameterMode::Released { nullable }
                | ResourceParameterMode::Stored { nullable, .. }
                | ResourceParameterMode::Removed { nullable, .. } => nullable,
            };
            let name = parameters[index].as_ref()?;
            let site = base.checked_add(index)?;
            custody.state.resources.insert(site, LIVE);
            custody.state.pointers.insert(
                name.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([site]),
                    unknown: false,
                    may_be_null: nullable,
                },
            );
            if nullable {
                custody.state.nullable_sites.insert(site);
            }
            custody.parameter_sites[index] = Some(site);
        }
        // A Removed formal is supplied inside its owner's aggregate, not in
        // direct custody. Bind every formal first so argument order cannot
        // change the owner identity. Unsupported owner shapes fail closed.
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            if let ResourceParameterMode::Removed {
                owner_parameter, ..
            } = mode
            {
                if index == owner_parameter
                    || !matches!(
                        contract.parameters.get(owner_parameter),
                        Some(ResourceParameterMode::Owned { nullable: false })
                    )
                {
                    return None;
                }
                let child = custody.parameter_sites[index]?;
                let owner = custody
                    .parameter_sites
                    .get(owner_parameter)
                    .copied()
                    .flatten()?;
                custody.state.resources.insert(child, CONTAINED);
                custody
                    .state
                    .contained
                    .entry(owner)
                    .or_default()
                    .insert(child);
            }
        }
        Some(custody)
    }

    /// Sites the caller — not this body — is responsible for discharging at a
    /// given exit: every formal the contract bound, plus whatever each out
    /// slot holds there, because an `Out` contract hands that resource back.
    ///
    /// The out part is read from the exit state rather than fixed up front, so
    /// a resource overwritten in the slot (`*out = p; *out = q;`) loses its
    /// exemption and is reported as the leak it is.
    fn exempt_sites(&self, state: &AbstractState) -> BTreeSet<usize> {
        let mut sites = self
            .parameter_sites
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        for place in self.out_slots.iter().flatten() {
            if let Some(pointer) = state.pointers.get(place) {
                sites.extend(pointer.targets.iter().copied());
            }
        }
        sites
    }

    /// Places a store may legitimately target in this analysis.
    fn sanctioned_places(&self) -> BTreeSet<&str> {
        self.out_slots
            .iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }
}

/// The first abstract site that no acquisition in `cfg` already uses.
fn first_free_site(cfg: &LegacyCfg) -> usize {
    cfg.nodes
        .iter()
        .flat_map(|node| match &node.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![*site],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        })
        .max()
        .map_or(0, |site| site.saturating_add(1))
}

/// What one reachable exit of a function does to the custody it was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExitObservation {
    /// Disposition of each bound formal parameter site at this exit.
    pub(crate) parameters: Vec<Option<SiteDisposition>>,
    pub(crate) returned: ReturnedValue,
    /// What each declared out slot holds at this exit.
    pub(crate) out_slots: Vec<Option<OutSlotValue>>,
    /// Containment whose *owner* is a site the caller holds — a formal, or a
    /// resource handed back through an out slot. A summary may only claim such
    /// an attachment through an explicit `Stored` mode; anything else here is
    /// a transfer invisible at the call site, and refutes the summary.
    pub(crate) caller_containment: BTreeMap<usize, BTreeSet<usize>>,
    /// Reallocation replacements that this exit left unresolved.
    pub(crate) pending_replacements: BTreeMap<usize, BTreeSet<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteDisposition {
    Live,
    Released,
    Escaped,
    Contained,
    /// The site left the state on this path (a refuted nullable acquisition).
    Absent,
    /// Several dispositions joined; no single custody claim describes it.
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReturnedValue {
    /// The exit returns no modeled resource place (including a null constant).
    None,
    Site {
        site: usize,
        nullable: bool,
        disposition: SiteDisposition,
    },
    /// The returned place denotes more than one site, or an unknown one.
    Ambiguous,
}

/// What an out slot holds at one exit. `Unwritten` is deliberately distinct
/// from `Null`: a slot the callee never stored to leaves the caller's variable
/// as it was, which no `Out` mode describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutSlotValue {
    /// The slot was never stored to on this path.
    Unwritten,
    /// The slot definitely holds a null pointer.
    Null,
    Site {
        site: usize,
        nullable: bool,
        disposition: SiteDisposition,
    },
    /// The slot denotes more than one site, or an unknown one.
    Ambiguous,
}

fn observe_out_slot(state: &AbstractState, place: &str) -> OutSlotValue {
    match state.pointers.get(place) {
        None => OutSlotValue::Unwritten,
        Some(pointer) if pointer.unknown || pointer.targets.len() > 1 => OutSlotValue::Ambiguous,
        Some(pointer) => match pointer.targets.first().copied() {
            None => OutSlotValue::Null,
            Some(site) => OutSlotValue::Site {
                site,
                nullable: pointer.may_be_null || state.nullable_sites.contains(&site),
                disposition: site_disposition(state, site),
            },
        },
    }
}

fn site_disposition(state: &AbstractState, site: usize) -> SiteDisposition {
    if state.unresolved_effect_sites.contains(&site) {
        return SiteDisposition::Mixed;
    }
    match state.resources.get(&site).copied() {
        None => SiteDisposition::Absent,
        Some(LIVE) => SiteDisposition::Live,
        Some(RELEASED) => SiteDisposition::Released,
        Some(ESCAPED) => SiteDisposition::Escaped,
        Some(CONTAINED) => SiteDisposition::Contained,
        Some(_) => SiteDisposition::Mixed,
    }
}

/// Containment a caller could observe: the declared `Stored` relation, plus
/// every B08 field link. A field link whose owner the caller holds is a
/// transfer invisible at the call site, so it must reach
/// [`crate::legacy_summary`]'s observation check and refute any summary that
/// does not declare it. The store admission already refuses a caller-held
/// aggregate, so this is defence in depth rather than a reachable case.
fn caller_visible_containment(state: &AbstractState) -> BTreeMap<usize, BTreeSet<usize>> {
    let mut containment = state.contained.clone();
    for (owner, fields) in &state.field_links {
        let children = containment.entry(*owner).or_default();
        for linked in fields.values() {
            children.extend(linked.iter().copied());
        }
    }
    containment
}

fn observe_exit(
    node: &LegacyCfgNode,
    state: &AbstractState,
    entry: &EntryCustody,
) -> ExitObservation {
    let returned = match &node.instruction {
        LegacyInstruction::EscapeIfOwned { source } | LegacyInstruction::Escape { source } => {
            match state.pointers.get(source) {
                None => ReturnedValue::None,
                Some(pointer) if pointer.unknown || pointer.targets.len() > 1 => {
                    ReturnedValue::Ambiguous
                }
                Some(pointer) => match pointer.targets.first().copied() {
                    // A definitely-null place returns no resource.
                    None => ReturnedValue::None,
                    Some(site) => ReturnedValue::Site {
                        site,
                        nullable: pointer.may_be_null || state.nullable_sites.contains(&site),
                        disposition: site_disposition(state, site),
                    },
                },
            }
        }
        _ => ReturnedValue::None,
    };
    let exempt = entry.exempt_sites(state);
    ExitObservation {
        parameters: entry
            .parameter_sites
            .iter()
            .map(|site| site.map(|site| site_disposition(state, site)))
            .collect(),
        returned,
        out_slots: entry
            .out_slots
            .iter()
            .map(|place| place.as_deref().map(|place| observe_out_slot(state, place)))
            .collect(),
        caller_containment: caller_visible_containment(state)
            .iter()
            .filter(|(owner, children)| exempt.contains(owner) && !children.is_empty())
            .map(|(owner, children)| (*owner, children.clone()))
            .collect(),
        pending_replacements: state.conditional_replacements.clone(),
    }
}

/// `ALIAS-LOCALS-BORROW-LIVENESS`: refuse every `Borrowed` actual the
/// projection's own bindings show is a **definitely released** identity.
///
/// Card ALIAS-LOCALS recorded that a `Borrowed` position had no implemented
/// precondition at all, so a body could hand a callee an identity this
/// projection records `RELEASED` and be certified `Safe`. The decision is
/// [`crate::borrow_liveness::borrowed_actual_liveness`]; this function only
/// locates the positions and publishes the refusal.
///
/// Three deliberate bounds, each recorded in
/// `docs/reports/remaining-borrow-liveness/README.md`:
///
/// * **Settled states only.** The pass runs after the work list is stable and
///   reads `incoming[node]` — the state *entering* the call, before any of this
///   call's own parameter effects. A precondition is evaluated in the
///   pre-state, so `f(p /*released*/, p /*borrowed*/)` is not refused here; the
///   release side already has its own diagnostics.
/// * **Plain names only.** A `BASE->FIELD` bound read and a member-address
///   actual are governed by O04-READ-BOUND / O04-B09B10, whose admissions
///   already require the identity to be `LIVE` (`bound_read_resolves`,
///   `resolved_aggregate_owner`). Restating it here would double-report against
///   another card's channel without adding a refusal.
/// * **Read-only.** Nothing is written to any `AbstractState`, so the custody
///   fixed point, the leak inventory and every per-exit residual are unchanged
///   by construction (`C02-P-ALIAS-LEAK-ORDER` undisturbed). The refusal
///   therefore does not set `unmodeled`, exactly as ALIAS-LOCALS's obligations
///   do not — the same recorded `ALIAS-LOCALS-EXIT-CUSTODY` boundary, since a
///   read changes no site's disposition.
///
/// The published claim is the existing `UnknownAlias` naming the actual. No
/// diagnostic code is added, changed or downgraded, and the rule can only turn
/// `Safe` into `Unknown`.
/// Card **SCALAR-ROUTE-EXIT-EDGE**: report each loop header whose exit edge the
/// value route called infeasible and the work list retained anyway.
///
/// Read from the **settled** incoming states, exactly as
/// [`require_live_borrowed_actuals`] is and for the same reason: the arm is
/// reached on intermediate iterates of a header whose fixed point later widens
/// the deciding binding to `unknown` (a body that writes the scalar takes
/// `ForgetScalar`, and `AbstractState::join`'s `(Some, None)` arm then makes it
/// `unknown`), so a diagnostic emitted inside the work list would depend on the
/// work-list schedule — the hazard constraint 6 records.
///
/// A `BranchOnNull` node's transfer function is empty and it is never a release
/// site, so its settled incoming state *is* the state the edge was refined
/// from; the probe below therefore asks exactly the question the work list
/// asked, on the state the fixed point settled at.
fn report_retained_loop_exit_edges(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        // An unreached header supports no claim in either direction.
        let (Some(edge), Some(state)) = (node.loop_exit_edge, incoming[node_id].as_ref()) else {
            continue;
        };
        let LegacyInstruction::BranchOnNull {
            source,
            true_is_null,
        } = &node.instruction
        else {
            continue;
        };
        let assume_null = if edge == 0 {
            *true_is_null
        } else {
            !*true_is_null
        };
        let mut probe = state.clone();
        if matches!(
            refine_pointer_null(&mut probe, source, assume_null),
            PointerRefinement::Infeasible
        ) {
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: LOOP_EXIT_EDGE_RETAINED.to_owned(),
                },
            });
        }
    }
}

fn require_live_borrowed_actuals(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        // An unreached node supports no claim in either direction.
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let LegacyInstruction::ContractCall {
            arguments,
            contract,
            nested,
            ..
        } = &node.instruction
        else {
            continue;
        };
        // Card CONSTRUCTOR-RESIDUALS, a fail-closed repair of a **pre-existing**
        // hole this card's own subject made reachable, and measured at the base
        // tree before any of this card's rules existed.
        //
        // Card O04-TRACE's admitted nested call carries its own modeled
        // actuals, and card BORROW-LIVENESS's precondition only ever looked at
        // the enclosing `ContractCall`'s. So
        //
        // ```c
        // s_close(h); s_note(s_label(h));      /* base: Safe, no diagnostic  */
        // s_close(h); s_label(h);              /* base: Unknown, UnknownAlias */
        // ```
        //
        // answered differently for the same released identity at the same
        // declared `Borrowed` mode, purely because one of them was nested. The
        // nested call's declared effects are the identity on the abstract state
        // (premises N6/N7), which is exactly why its *reads* need the same
        // precondition and get no other channel to raise it: `transfer` applies
        // its effects and produces nothing.
        //
        // Both are examined against the **same settled pre-state**, which is
        // sound in the ordering sense for the same reason O04-TRACE composes
        // them at all: an admitted nested call changes nothing, so the state
        // entering the nested call and the state entering the outer call are
        // the same state.
        let calls = std::iter::once((arguments, contract)).chain(
            nested
                .iter()
                .map(|nested| (&nested.arguments, &nested.contract)),
        );
        for (arguments, contract) in calls {
            // Card BORROW-RESIDUALS (`BORROW-LIVENESS-PRESTATE`): what this call's
            // own declared effects destroy, resolved once against the same settled
            // pre-state. Only `Released` destroys, exactly as only the `RELEASED`
            // bit does in `borrow_liveness::custody_definitely_ended`; `Consumed`,
            // `Stored` and `Removed` move or detach custody, and `Out` writes a
            // slot.
            let destroying = contract
                .parameters
                .iter()
                .enumerate()
                .map(|(index, mode)| {
                    matches!(mode, ResourceParameterMode::Released { .. })
                        .then(|| arguments.get(index).and_then(Option::as_deref))
                        .flatten()
                        .and_then(|place| state.pointers.get(place))
                        .map(denote_pointer)
                })
                .collect::<Vec<_>>();
            for (index, mode) in contract.parameters.iter().enumerate() {
                if !matches!(mode, ResourceParameterMode::Borrowed) {
                    continue;
                }
                let Some(source) = arguments.get(index).and_then(Option::as_deref) else {
                    continue;
                };
                let denotation = state.pointers.get(source).map(denote_pointer);
                // The path rule, on plain names only: a composite place carries
                // O04-READ-BOUND's and O04-B09B10's strictly stronger `LIVE`
                // requirement in this same pre-state (`BORROW-LIVENESS-PLACES`).
                let path_refuses = crate::may_alias::place_root(source) == Some(source)
                    && crate::borrow_liveness::borrowed_actual_liveness(
                        crate::borrow_liveness::BorrowSite {
                            denotation,
                            resources: &state.resources,
                            unresolved: &state.unresolved_effect_sites,
                        },
                    ) == crate::borrow_liveness::BorrowLiveness::DefinitelyNotLive;
                // The same-call rule, on every place the projection settled: no
                // other channel asks this call's own effects about its own reads,
                // so there is nothing here to duplicate.
                let call_refuses = crate::borrow_liveness::borrow_destroyed_by_this_call(
                    crate::borrow_liveness::SameCallSite {
                        borrowed: denotation,
                        destroying: &destroying,
                    },
                ) == crate::borrow_liveness::SameCallBorrow::DestroyedByThisCall;
                if path_refuses || call_refuses {
                    diagnostics.insert(LegacyDiagnostic {
                        function: cfg.function.clone(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::UnknownAlias {
                            variable: source.to_owned(),
                        },
                    });
                }
            }
        }
    }
}

/// One tracked pointer's binding, as [`crate::may_alias::Denotation`] reads it.
///
/// Borrowed rather than copied, so that neither of the two read-only decisions
/// above can reach a mutable projection state.
fn denote_pointer(pointer: &AbstractPointer) -> crate::may_alias::Denotation<'_> {
    crate::may_alias::Denotation {
        targets: &pointer.targets,
        unknown: pointer.unknown,
        may_be_null: pointer.may_be_null,
    }
}

fn classify_diagnostics(diagnostics: &[LegacyDiagnostic]) -> AnalysisStatus {
    let unsafe_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::Leak { .. }
                | LegacyDiagnosticKind::DoubleRelease { .. }
                | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
                | LegacyDiagnosticKind::CustodyConflict { .. }
        )
    });
    let unknown_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { .. }
                | LegacyDiagnosticKind::Unsupported { .. }
                | LegacyDiagnosticKind::SummaryConflict { .. }
        )
    });
    if unsafe_found {
        AnalysisStatus::Unsafe
    } else if unknown_found {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    }
}

/// The parts of the contract in force that the transfer function consults.
///
/// Both fields default to the most conservative setting, which is what the
/// contract-free analysis uses: no store through a pointer is modeled, and a
/// returned place may not be null. A contract only ever *adds* permissions
/// here, and every permission it adds is re-checked at the exits.
#[derive(Default)]
struct TransferContext<'a> {
    /// Out-parameter slots the contract in force declares. A store to any
    /// other indirect place stays unmodeled.
    sanctioned: BTreeSet<&'a str>,
    /// Whether the contract's result mode obliges callers to null-check.
    result_nullable: bool,
    /// Sites the *caller* holds at entry. An aggregate store never targets one
    /// of these: a transfer into a caller-owned aggregate is invisible at the
    /// call site and is refused rather than modeled (O00 blocker B08).
    caller_sites: BTreeSet<usize>,
    /// Card **CTWIN-PUBLISH**: the destinations whose object outlives the call.
    /// Storing an identity *this body* holds into one of them is a publication
    /// the projection does not model, and is refused; see
    /// [`reject_publication`].
    publications: PublicationFacts,
}

fn analyze_cfg(cfg: LegacyCfg) -> LegacyFunctionSummary {
    analyze_cfg_with_entry(cfg, &EntryCustody::default()).0
}

pub(crate) fn analyze_cfg_with_entry(
    cfg: LegacyCfg,
    entry_custody: &EntryCustody,
) -> (LegacyFunctionSummary, Vec<ExitObservation>) {
    let context = TransferContext {
        sanctioned: entry_custody.sanctioned_places(),
        result_nullable: entry_custody.result_nullable,
        caller_sites: entry_custody
            .parameter_sites
            .iter()
            .flatten()
            .copied()
            .collect(),
        publications: cfg.publications.clone(),
    };
    // C02-P-ALIAS: computed once, before the custody work list starts, and only
    // read below. See [`alias_facts`].
    let alias = alias_facts(&cfg);
    let joined = joined_provenance::derive(&cfg);
    let mut incoming = vec![None::<AbstractState>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(entry_custody.state.clone());
    let mut work = VecDeque::from([cfg.entry]);
    let mut diagnostics = BTreeSet::new();
    let mut steps = 0usize;
    let acquisition_lines = cfg
        .nodes
        .iter()
        .flat_map(|node| match &node.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![(*site, node.line)],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites
                .iter()
                .map(|site| (*site, node.line))
                .collect(),
            _ => Vec::new(),
        })
        .collect::<BTreeMap<_, _>>();

    while let Some(node_id) = work.pop_front() {
        steps += 1;
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let node = &cfg.nodes[node_id];
        let mut local = BTreeSet::new();
        let conditional_release = joined_provenance::release_site(node, &state, &joined[node_id]);
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &context,
            &alias[node_id],
            &mut local,
        );
        // A path that reached a refusal supports no per-exit custody claim.
        // Recorded before the local diagnostics are merged, so the flag tracks
        // *this* node rather than anything reported elsewhere in the function.
        joined_provenance::finish_release(conditional_release, &mut output, &local);
        output.unmodeled |= local.iter().any(is_path_poisoning);
        diagnostics.append(&mut local);
        if node.is_exit {
            let exempt = entry_custody.exempt_sites(&output);
            report_leaks(
                &cfg.function,
                node.line,
                &output,
                &acquisition_lines,
                &exempt,
                &mut diagnostics,
            );
        }
        for (edge, &successor) in node.successors.iter().enumerate() {
            let mut edge_output = output.clone();
            if let LegacyInstruction::BranchOnNull {
                source,
                true_is_null,
            } = &node.instruction
            {
                let assume_null = if edge == 0 {
                    *true_is_null
                } else {
                    !*true_is_null
                };
                match refine_pointer_null(&mut edge_output, source, assume_null) {
                    // Card **SCALAR-ROUTE-EXIT-EDGE**. A loop header's exit
                    // edge is never removed, whatever the value route decides.
                    //
                    // `Infeasible` elsewhere is ordinary path sensitivity: an
                    // `if`'s two arms rejoin at the same continuation, so
                    // pruning one orphans nothing. A *loop header*'s two
                    // successors do not rejoin — everything after the loop
                    // hangs off this one edge — so deleting it leaves the exit
                    // node with no incoming state at all, `report_leaks` never
                    // runs there, and `classify_diagnostics` can return `Safe`
                    // for a body that leaks, double-releases or releases after
                    // escape *after* the loop. That is the direction invariant
                    // 1 forbids, and it was measured: see
                    // `crates/qcore-c-gateway/tests/constant_do_while.rs`.
                    //
                    // The deletion would be exact only if the loop really
                    // cannot terminate, and this route cannot know that. Its
                    // must-fact is a *modelled value*, not a token, so it rests
                    // on two premises the analyzer does not verify: that every
                    // write the body performs to the deciding name reaches the
                    // header as a forget, an opaque value or a poison
                    // (constraint 10a's `Sparing`, in a role 10a does not
                    // record), and that the object is not modified
                    // asynchronously — which is **false for a `volatile`
                    // object**, where C11 §5.1.2.3 lets `while (!flag) { }`
                    // terminate. Constraint 5a already forbids deciding
                    // control flow from a computed value; this is that rule
                    // applied to the one place that still did.
                    //
                    // The edge is therefore retained carrying the *unrefined*
                    // state, and the header reports `Unsupported`, so the
                    // function is `Unknown` rather than either a false `Safe`
                    // or a fabricated `Unsafe` about a path the model says it
                    // does not have. The sibling *back* edge is deliberately
                    // untouched (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`): removing
                    // it deletes a loop body, which is what card
                    // `C-GATEWAY-CONSTANT-DO-WHILE` proved for a token, and it
                    // needs its own measured re-pin rather than being folded in
                    // here.
                    //
                    // The refusal itself is **not** reported here. This arm is
                    // reached on intermediate iterates of a header whose fixed
                    // point later widens the deciding binding to `unknown`, so a
                    // diagnostic emitted inside the work list would depend on
                    // the work-list schedule — constraint 6's recorded hazard.
                    // It is reported from the settled states by
                    // [`report_retained_loop_exit_edges`] instead, exactly as
                    // `require_live_borrowed_actuals` reads its must-fact.
                    // Retaining the edge is itself monotone, so the fixed point
                    // stays order-independent.
                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        edge_output = output.clone();
                        edge_output.unmodeled = true;
                    }
                    PointerRefinement::Infeasible => continue,
                    PointerRefinement::Unknown => {
                        edge_output.unmodeled = true;
                        diagnostics.insert(LegacyDiagnostic {
                            function: cfg.function.clone(),
                            line: node.line,
                            kind: LegacyDiagnosticKind::UnknownAlias {
                                variable: source.clone(),
                            },
                        });
                    }
                    PointerRefinement::Refined => {}
                }
            }
            let changed = match &mut incoming[successor] {
                Some(current) => current.join(&edge_output),
                slot @ None => {
                    *slot = Some(edge_output);
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }

    if incoming.iter().flatten().any(|state| state.widened) {
        diagnostics.insert(LegacyDiagnostic {
            function: cfg.function.clone(),
            line: 0,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "alias set exceeded {MAX_ALIAS_TARGETS} targets and was widened"
                ),
            },
        });
    }

    // ALIAS-LOCALS-BORROW-LIVENESS: the `Borrowed` liveness precondition, read
    // from the settled incoming states for the same reason the exit pass below
    // is: it is a *must* fact, and a must fact read on an intermediate iterate
    // would depend on the work-list schedule. Read-only — see
    // [`crate::borrow_liveness`].
    require_live_borrowed_actuals(&cfg, &incoming, &mut diagnostics);
    report_retained_loop_exit_edges(&cfg, &incoming, &mut diagnostics);

    // Read the settled exit states once the work list is stable, so that a
    // summary describes the joined behavior of every reachable return rather
    // than an intermediate iterate. Diagnostics are already recorded above;
    // this pass only observes.
    let mut observations = Vec::new();
    let mut exits = Vec::new();
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        if !node.is_exit {
            continue;
        }
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let mut ignored = BTreeSet::new();
        let conditional_release = joined_provenance::release_site(node, &state, &joined[node_id]);
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &context,
            &alias[node_id],
            &mut ignored,
        );
        joined_provenance::finish_release(conditional_release, &mut output, &ignored);
        output.unmodeled |= ignored.iter().any(is_path_poisoning);
        observations.push(observe_exit(node, &output, entry_custody));
        exits.push(observe_residual(
            node_id,
            node,
            &output,
            &acquisition_lines,
            entry_custody,
        ));
    }

    let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    let status = classify_diagnostics(&diagnostics);
    (
        LegacyFunctionSummary {
            function: cfg.function.clone(),
            status,
            diagnostics,
            fixed_point_steps: steps,
            cfg,
            exits,
        },
        observations,
    )
}

/// Whether a diagnostic means the path that produced it can support no custody
/// claim. `Leak` is deliberately absent: it is reported *at* an exit by
/// [`report_leaks`], and it is exactly the residual this ledger describes
/// rather than a reason to refuse describing it.
fn is_path_poisoning(diagnostic: &LegacyDiagnostic) -> bool {
    matches!(
        diagnostic.kind,
        LegacyDiagnosticKind::Unsupported { .. }
            | LegacyDiagnosticKind::UnknownAlias { .. }
            | LegacyDiagnosticKind::SummaryConflict { .. }
            | LegacyDiagnosticKind::DoubleRelease { .. }
            | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
            | LegacyDiagnosticKind::CustodyConflict { .. }
    )
}

/// Read one exit's custody residual out of its settled state.
fn observe_residual(
    node_id: LegacyNodeId,
    node: &LegacyCfgNode,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    entry: &EntryCustody,
) -> LegacyExitResidual {
    let exempt = entry.exempt_sites(state);
    let sites = state
        .resources
        .keys()
        .copied()
        .filter(|site| !exempt.contains(site))
        .map(|site| LegacyResidualSite {
            site,
            acquisition_line: acquisition_line(acquisition_lines, site),
            disposition: site_disposition(state, site),
        })
        .collect::<Vec<_>>();
    let indeterminate = state.unmodeled
        || state.widened
        || sites
            .iter()
            .any(|site| site.disposition == SiteDisposition::Mixed);
    let custody = if indeterminate {
        ExitCustody::Unknown
    } else if sites
        .iter()
        .any(|site| site.disposition == SiteDisposition::Live)
    {
        ExitCustody::Residual
    } else {
        ExitCustody::Discharged
    };
    LegacyExitResidual {
        node: node_id,
        line: node.line,
        custody,
        sites,
    }
}

/// Card **CTWIN-PUBLISH**: report a store of a tracked identity into an object
/// that outlives the call, and poison the path exactly as any other unmodeled
/// construct does.
///
/// **Why refusal rather than an escape.** Modeling the store as an
/// [`LegacyInstruction::Escape`] would be fail-closed in one direction only.
/// A published-then-released body would become a `ReleaseAfterEscape`, but a
/// published-and-*not*-released body — measured `Unsafe` with a `Leak` before
/// this card — would become **`Safe`**, because the escape discharges the
/// residual. That is a promotion, and it asserts an accounting no contract
/// states: unlike a return or a declared consuming position, a store into a
/// file-scope object hands custody to no named party. Both directions are
/// therefore withdrawn: this projection can say nothing about a resource whose
/// custody left through an unmodeled publication protocol.
fn reject_publication(
    function: &str,
    line: usize,
    place: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "publication of a tracked resource into the static-storage object `{place}` is outside the resource projection"
            ),
        },
    });
    for pointer in state.pointers.values_mut() {
        pointer.unknown = true;
        pointer.may_be_null = true;
    }
}

/// Report an indirect store the contract in force does not sanction, and
/// poison the state exactly as an unmodeled construct would. The importer
/// *names* `*out` so an out-parameter contract can describe it; without such a
/// contract the store is as opaque as it was before out slots existed.
fn reject_unsanctioned_store(
    function: &str,
    line: usize,
    place: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("store through `{place}` without an out-parameter contract"),
        },
    });
    for pointer in state.pointers.values_mut() {
        pointer.unknown = true;
        pointer.may_be_null = true;
    }
}

fn transfer(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    // C02-P-ALIAS: the may-alias facts on entry to this node, precomputed by
    // [`alias_facts`]. Read only; nothing here writes them, which is what keeps
    // the custody work list scheduled exactly as it was before this card.
    alias: &NodeAliasFacts,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    // A slot name denotes the caller's original pointer only while that
    // formal is unchanged. Tracking its reassignment would need pointer-to-
    // pointer alias analysis; reject it rather than writing a fictitious slot.
    let destination = match &node.instruction {
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::OpaqueValue { destination, .. }
        | LegacyInstruction::ForgetScalar { destination }
        | LegacyInstruction::Clear { destination } => Some(destination.as_str()),
        LegacyInstruction::ContractCall { destination, .. } => destination.as_deref(),
        _ => None,
    };
    let changes_slot_identity = |place: &str| {
        context
            .sanctioned
            .iter()
            .any(|slot| slot.strip_prefix('*') == Some(place))
    };
    let overwritten = destination.is_some_and(changes_slot_identity)
        || match &node.instruction {
            LegacyInstruction::ContractCall {
                arguments,
                contract,
                ..
            } => contract
                .parameters
                .iter()
                .zip(arguments)
                .any(|(mode, argument)| {
                    matches!(mode, ResourceParameterMode::Out { .. })
                        && argument.as_deref().is_some_and(changes_slot_identity)
                }),
            _ => false,
        };
    if overwritten {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: "reassignment of an out-parameter pointer".to_owned(),
            },
        });
        for pointer in state.pointers.values_mut() {
            pointer.unknown = true;
            pointer.may_be_null = true;
        }
        return state;
    }
    // CTWIN-RESIDUALS, shape `CTWIN-PUBLISH-OUT`: a declared `Out` slot whose
    // actual *is* one of those objects acquires the resource directly into it.
    // That is the `Acquire` arm below seen from the callee's side — the
    // identity is published at the moment it is created — and it was measured
    // invisible: `y_fill(&g_published); y_release(g_published);` was `Safe`
    // with no diagnostic, the publish-then-free hazard in the out shape, and
    // the same body without the release reported a `Leak` the projection is in
    // no position to claim. Refused *before* the call's effects apply, so no
    // site exists to carry forward.
    if let LegacyInstruction::ContractCall {
        arguments,
        contract,
        ..
    } = &node.instruction
        && let Some(place) = contract
            .parameters
            .iter()
            .zip(arguments)
            .filter(|(mode, _)| matches!(mode, ResourceParameterMode::Out { .. }))
            .filter_map(|(_, argument)| argument.as_deref())
            .find(|argument| context.publications.destinations.contains(*argument))
    {
        let place = place.to_owned();
        // The call's declared effects never apply, so no identity it names may
        // keep a residual claim: a `Released` argument would otherwise stay
        // live to the exit and be reported as a leak the call was going to
        // discharge. Marking exactly the arguments' own identities keeps an
        // unrelated resource's residual intact.
        let named = arguments
            .iter()
            .flatten()
            .filter_map(|argument| state.pointers.get(argument).cloned())
            .collect::<Vec<_>>();
        for pointer in &named {
            mark_unresolved_effect(&mut state, pointer);
        }
        reject_publication(function, node.line, &place, &mut state, diagnostics);
        return state;
    }
    // CTWIN-PUBLISH: a store of an identity *this body* holds into an object
    // that outlives the call is a publication this projection does not model.
    // Checked here, before any effect of the instruction applies, and
    // flow-sensitively: naming such a destination is not the refusal, storing
    // a tracked identity into it is. See [`reject_publication`] for why both
    // directions are withdrawn rather than an escape recorded, and
    // [`crate::publication`] for how the destination set is read.
    if let Some(place) =
        destination.filter(|place| context.publications.destinations.contains(*place))
    {
        let published = match &node.instruction {
            // The value is whatever `source` denotes here — **any** identity
            // this projection tracks, caller-held or body-acquired.
            //
            // Card CTWIN-RESIDUALS withdrew the caller-held exclusion card
            // CTWIN-PUBLISH left open as `CTWIN-PUBLISH-BORROWED`, on a
            // measured ground rather than a wider notion of safety: a body that
            // publishes a *borrowed* formal was `Safe` with no diagnostic, so
            // W4 published `Borrowed` as its summary for that formal, and a
            // caller that acquired, called it and released was `Safe` too —
            // which is the wave-16 headline body with one level of indirection
            // through a helper, and the rule that refuses it inlined must
            // refuse it composed. `Borrowed` asserts that the caller's
            // obligations are unchanged across the call, and a publication
            // falsifies exactly that. The faithful alternative modes —
            // `Escaped`, `Consumed` — are the escape model CTWIN-PUBLISH §3
            // proved promotes, so the only fail-closed answer is withdrawal.
            //
            // The rule therefore reads no provenance, exactly as it reads no
            // per-site status: a formal the projection tracks no identity for
            // still publishes nothing, so a body analyzed with no entry custody
            // keeps its pre-existing answer.
            LegacyInstruction::Alias { source, .. }
                if !context.publications.scalar_sources.contains(source) =>
            {
                state
                    .pointers
                    .get(source)
                    .filter(|pointer| !pointer.targets.is_empty())
                    .cloned()
            }
            // An acquisition bound directly to such an object publishes it at
            // the moment it is created. Refused *before* the site exists, so
            // there is nothing to carry forward.
            LegacyInstruction::Acquire { .. } => Some(AbstractPointer::default()),
            LegacyInstruction::ContractCall { contract, .. } => {
                (!matches!(contract.result, ResourceReturnMode::None))
                    .then(AbstractPointer::default)
            }
            // `Clear` stores the null constant and `OpaqueValue` a value this
            // function never tracks; neither publishes a resource identity.
            _ => None,
        };
        if let Some(pointer) = published {
            // The published identities carry an unresolved effect from here on,
            // exactly as an unresolved callee effect does: the projection can
            // no longer say whether they are released, so a residual leak that
            // *depends on* this publication stays `Unknown` instead of being
            // reported as a validated violation (the w4-nullable-removal rule).
            mark_unresolved_effect(&mut state, &pointer);
            reject_publication(function, node.line, place, &mut state, diagnostics);
            return state;
        }
    }
    // A store through a pointer is modeled only where the contract in force
    // declares that slot an out parameter. `ContractCall` handles its own
    // destination below, because a call whose result carries no custody never
    // touched the slot in the first place.
    if let LegacyInstruction::Alias {
        destination,
        source: _,
    }
    | LegacyInstruction::Clear { destination } = &node.instruction
        && is_indirect_place(destination)
        && !context.sanctioned.contains(destination.as_str())
    {
        reject_unsanctioned_store(function, node.line, destination, &mut state, diagnostics);
        return state;
    }
    // Forwarded Out arguments denote indirect writes just like *out = value.
    // A local alias or an unbound pointer must never gain a fabricated slot.
    if let LegacyInstruction::ContractCall {
        arguments,
        contract,
        ..
    } = &node.instruction
    {
        for (mode, argument) in contract.parameters.iter().zip(arguments) {
            if !matches!(mode, ResourceParameterMode::Out { .. })
                && argument.as_deref().is_some_and(changes_slot_identity)
            {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: "out-slot pointer passed without a modeled Out effect".into(),
                    },
                });
                for pointer in state.pointers.values_mut() {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
                return state;
            }
            if matches!(mode, ResourceParameterMode::Out { .. })
                && let Some(place) = argument
                && is_indirect_place(place)
                && !context.sanctioned.contains(place.as_str())
            {
                reject_unsanctioned_store(function, node.line, place, &mut state, diagnostics);
                return state;
            }
        }
    }
    // O04-READ-BOUND: every admitted bound *read* of an aggregate member
    // requires the same flow-sensitive premise as a store, plus the member
    // actually carrying the field link an admitted store recorded. A read
    // through a released, escaped, refuted, ambiguous, nullable or caller-held
    // aggregate refuses and poisons the path, and so does a member this body
    // never linked or whose binding no longer agrees with its link. This is the
    // C00 premise in its read direction.
    if let LegacyInstruction::ContractCall {
        aggregate_reads,
        aggregate_addresses,
        ..
    } = &node.instruction
        && let Some(owner) = aggregate_reads
            .iter()
            .find(|place| !bound_read_resolves(&state, place, &context.caller_sites))
            .and_then(|place| crate::aggregate_store::place_owner(place))
            // O04-B09B10: the same C00 premise for a member *address*. The
            // member itself carries no link — it is declared with no `*` — so
            // only the aggregate's own liveness is required, not a field link.
            .or_else(|| {
                aggregate_addresses
                    .iter()
                    .map(String::as_str)
                    .find(|owner| {
                        resolved_aggregate_owner(&state, owner, &context.caller_sites).is_none()
                    })
            })
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: owner.to_owned(),
            },
        });
        for pointer in state.pointers.values_mut() {
            pointer.unknown = true;
            pointer.may_be_null = true;
        }
        return state;
    }
    // C02-P-ALIAS: every modeled read of a pointer this rule relates to another
    // declared name refuses when that peer was released on this path and
    // neither discharge applies. Reported before the instruction's own effects,
    // which still apply unchanged: this can only add an `UnknownAlias`, never
    // suppress a leak or a double release.
    match &node.instruction {
        LegacyInstruction::ContractCall { alias_reads, .. } => require_unaliased_reads(
            function,
            node.line,
            alias_reads,
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        LegacyInstruction::OpaqueValue {
            alias_read: Some(read),
            ..
        } => require_unaliased_reads(
            function,
            node.line,
            std::slice::from_ref(read),
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        LegacyInstruction::ScalarMemberStore {
            alias_read: Some(read),
            ..
        } => require_unaliased_reads(
            function,
            node.line,
            std::slice::from_ref(read),
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        _ => {}
    }
    // W42-INDIRECT-ACCESS: a scalar destination store dereferences its base.
    // Require one settled live identity after the may-alias obligation above,
    // without mutating any projection map.  Every declined shape has already
    // retained the lexical refusal; this is the flow-sensitive refusal for the
    // one typed shape the classifier lowered explicitly.
    if let LegacyInstruction::ScalarMemberStore { base, .. } = &node.instruction
        && !scalar_member_store_base_resolves(&state, base)
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: base.clone(),
            },
        });
        // The unresolved dereference refuses this function, but the typed
        // statement still writes only a scalar field from a scalar-only RHS.
        // It cannot rebind any pointer or resource identity, so retain the
        // incoming frame and keep later residual/double-release evidence.
        return state;
    }
    // O00 blocker B08: every store into an admitted aggregate field requires
    // its aggregate to resolve, here and now, to exactly one live identity
    // *this* body acquired. A released, escaped, refuted, ambiguous, nullable
    // or caller-held aggregate refuses and poisons the path; this is the C00
    // premise in its store direction, checked flow-sensitively rather than
    // syntactically.
    let stored_field = match written_field_place(&node.instruction) {
        None => None,
        Some(place) => {
            let owner = crate::aggregate_store::place_owner(place).unwrap_or_default();
            match resolved_aggregate_owner(&state, owner, &context.caller_sites) {
                Some(site) => {
                    let field = crate::aggregate_store::place_field(place).unwrap_or_default();
                    Some((site, field.to_owned(), place.to_owned()))
                }
                None => {
                    diagnostics.insert(LegacyDiagnostic {
                        function: function.to_owned(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::UnknownAlias {
                            variable: owner.to_owned(),
                        },
                    });
                    for pointer in state.pointers.values_mut() {
                        pointer.unknown = true;
                        pointer.may_be_null = true;
                    }
                    return state;
                }
            }
        }
    };
    match &node.instruction {
        LegacyInstruction::Acquire { destination, site } => {
            state.resources.insert(*site, LIVE);
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([*site]),
                    unknown: false,
                    may_be_null: false,
                },
            );
        }
        LegacyInstruction::Alias {
            destination,
            source,
        } => {
            let pointer = state.pointers.get(source).cloned().unwrap_or_else(|| {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.clone(),
                    },
                });
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                    may_be_null: true,
                }
            });
            state.pointers.insert(destination.clone(), pointer);
        }
        LegacyInstruction::Clear { destination } => {
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    may_be_null: true,
                    ..AbstractPointer::default()
                },
            );
        }
        LegacyInstruction::Release { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                false,
                false,
            );
        }
        LegacyInstruction::Escape { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                true,
                false,
            );
        }
        LegacyInstruction::EscapeIfOwned { source } => {
            if state.pointers.contains_key(source) {
                apply_resource_action(
                    function,
                    node.line,
                    source,
                    &mut state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    // Returning a possibly-null place is an unresolved alias
                    // unless this function's own result mode already obliges
                    // its callers to null-check what they get back.
                    context.result_nullable,
                );
            }
        }
        LegacyInstruction::ContractCall {
            function: called,
            destination,
            arguments,
            contract,
            produced_sites,
            // All revalidated above, before any effect of this call applies.
            aggregate_reads: _,
            aggregate_consumed: _,
            aggregate_addresses: _,
            alias_reads: _,
            nested,
        } => {
            // Card O04-TRACE: the admitted nested call's declared effects, in
            // order, before the enclosing call's. It produces nothing, so its
            // produced-site vector is empty by construction.
            for nested in nested {
                apply_contract_call(
                    function,
                    node.line,
                    &nested.function,
                    None,
                    &nested.arguments,
                    &nested.contract,
                    &[],
                    &mut state,
                    acquisition_lines,
                    diagnostics,
                );
            }
            let unsanctioned = destination
                .as_deref()
                .filter(|place| is_indirect_place(place) && !context.sanctioned.contains(place));
            match unsanctioned {
                // A result that carries no custody never wrote the slot, so
                // the call's parameter effects still apply and the unmodeled
                // destination is simply ignored — the pre-out-slot behavior.
                Some(_) if matches!(contract.result, ResourceReturnMode::None) => {
                    apply_contract_call(
                        function,
                        node.line,
                        called,
                        None,
                        arguments,
                        contract,
                        produced_sites,
                        &mut state,
                        acquisition_lines,
                        diagnostics,
                    );
                }
                // Custody would be created in a place this analysis cannot
                // account for: refuse the whole call.
                Some(_) => {
                    diagnostics.insert(LegacyDiagnostic {
                        function: function.to_owned(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::Unsupported {
                            description: format!(
                                "discarded owned result of resource API `{called}`"
                            ),
                        },
                    });
                    for pointer in state.pointers.values_mut() {
                        pointer.unknown = true;
                        pointer.may_be_null = true;
                    }
                }
                None => apply_contract_call(
                    function,
                    node.line,
                    called,
                    destination.as_deref(),
                    arguments,
                    contract,
                    produced_sites,
                    &mut state,
                    acquisition_lines,
                    diagnostics,
                ),
            }
        }
        LegacyInstruction::OpaqueValue {
            destination,
            // Revalidated above, before the binding takes effect.
            alias_read: _,
        } => {
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                    may_be_null: true,
                },
            );
        }
        LegacyInstruction::ForgetScalar { destination } => {
            state.pointers.remove(destination);
        }
        LegacyInstruction::ScalarMemberStore { .. } => {}
        LegacyInstruction::AggregateStore { place } => {
            // The stored value carries no tracked identity, so the member binds
            // nothing. Whatever it previously linked is unlinked below, which
            // is what makes an overwritten resource visible as a residual.
            state.pointers.remove(place);
        }
        LegacyInstruction::BranchOnNull { .. } | LegacyInstruction::BranchOnDistinct { .. } => {}
        LegacyInstruction::Unknown {
            description,
            writes,
        } => {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: description.clone(),
                },
            });
            // CTWIN-RESIDUALS, shape `CTWIN-PUBLISH-INDIRECT`. The statement
            // keeps the refusal it already had — the attribution stays with the
            // gate that read it — and additionally withdraws the residual claim
            // on whatever identity it stored, because the destination is a
            // continued access on an object that outlives the call. Only the
            // sites the statement's own operands denote are marked, so an
            // unrelated resource's residual is untouched; a statement that
            // names no tracked identity marks nothing, and nothing here can
            // turn a refusal into an admission.
            //
            // The value test is the direct arm's, exactly: a source must denote
            // at least one site and must not be a declared scalar. Presence in
            // the binding map is *not* enough — an ordinary scalar assignment
            // puts a name there with no targets — and refusing on that would be
            // refusal without a property. Measured: `crc32c_table[k][n] = crc;`
            // in `librdkafka/crc32c.c` is the corpus's own witness, and it
            // publishes nothing.
            if let Some(publication) = context.publications.indirect.get(&node.line) {
                let published = publication
                    .sources
                    .iter()
                    .filter(|source| !context.publications.scalar_sources.contains(*source))
                    .filter_map(|source| state.pointers.get(source))
                    .filter(|pointer| !pointer.targets.is_empty())
                    .cloned()
                    .collect::<Vec<_>>();
                if !published.is_empty() {
                    for pointer in &published {
                        mark_unresolved_effect(&mut state, pointer);
                    }
                    reject_publication(
                        function,
                        node.line,
                        &publication.place,
                        &mut state,
                        diagnostics,
                    );
                }
            }
            // Card AOM-BINDING-POISON. The refusal itself is unchanged — the
            // `Unsupported` diagnostic above still stands, still poisons the
            // path and still makes this function `Unknown` — and so is every
            // resource state: `state.resources` is not touched here and never
            // was. What is bounded is the collateral damage to *bindings*.
            //
            // `Unbounded` is the pre-repair behaviour and the answer for every
            // refusal that has no complete expression or statement text. A
            // `Sparing` refusal preserves only plain automatic names of this
            // body, never address-taken, that the refused text cannot write; see [`RefusedWrites`].
            for (place, pointer) in state.pointers.iter_mut() {
                if writes.poisons(place) {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
            }
        }
        LegacyInstruction::NoOp => {}
    }
    // O04-B09B10: a custody-taking member read handed the linked child to the
    // callee, whose contract effect above released or consumed it. The member
    // now holds a value this projection must not denote again, so drop both the
    // place binding and the field link. A second read of the same member then
    // fails `bound_read_resolves` — conjunct 2 for the binding, conjunct 3 for
    // the link — instead of releasing the same identity twice, and an escape of
    // the aggregate no longer cascades to a child it no longer holds.
    if let LegacyInstruction::ContractCall {
        aggregate_consumed, ..
    } = &node.instruction
    {
        for place in aggregate_consumed {
            state.pointers.remove(place);
            let (Some(owner), Some(field)) = (
                crate::aggregate_store::place_owner(place),
                crate::aggregate_store::place_field(place),
            ) else {
                continue;
            };
            let Some(owner_site) = resolved_aggregate_owner(&state, owner, &context.caller_sites)
            else {
                continue;
            };
            if let Some(fields) = state.field_links.get_mut(&owner_site) {
                fields.remove(field);
                if fields.is_empty() {
                    state.field_links.remove(&owner_site);
                }
            }
        }
    }
    if let Some((owner_site, field, place)) = stored_field {
        let children = state
            .pointers
            .get(&place)
            .map(|pointer| pointer.targets.clone())
            .unwrap_or_default();
        let entry = state.field_links.entry(owner_site).or_default();
        if children.is_empty() {
            entry.remove(&field);
        } else {
            entry.insert(field, children);
        }
        if entry.is_empty() {
            state.field_links.remove(&owner_site);
        }
    }
    state
}

/// The admitted aggregate field place an instruction stores into, if any.
fn written_field_place(instruction: &LegacyInstruction) -> Option<&str> {
    let place = match instruction {
        LegacyInstruction::AggregateStore { place } => place.as_str(),
        LegacyInstruction::Alias { destination, .. } | LegacyInstruction::Clear { destination } => {
            destination.as_str()
        }
        LegacyInstruction::ContractCall { destination, .. } => destination.as_deref()?,
        _ => return None,
    };
    crate::aggregate_store::is_field_place(place).then_some(place)
}

/// Whether an admitted bound read of `place` denotes, here and now, exactly the
/// identity an admitted store linked into that member (card O04-READ-BOUND).
///
/// Three conjuncts, all flow-sensitive and all fail-closed:
///
/// 1. the **owner** resolves through [`resolved_aggregate_owner`] — one live
///    identity this body acquired, never a caller-held, nullable or
///    unresolved-effect one. A read through a released or escaped aggregate can
///    therefore never be admitted, which is card C00's lesson in this direction;
/// 2. the **place** is bound to a definite, non-empty target set. A member this
///    body never stored into carries no identity to denote;
/// 3. the place's targets are **exactly** the field link the store recorded
///    under that owner. A binding that has drifted from its link — through a
///    path join, a refutation or any other route — is refused rather than
///    believed;
/// 4. every linked identity is itself **live** and carries no unresolved
///    effect. A member left pointing at an identity this body already released
///    or published is a dangling read, and admitting it is exactly the mistake
///    card C00 records: the whole point of premise 4 is that a read through a
///    released pointer must never reach `Safe`.
fn bound_read_resolves(state: &AbstractState, place: &str, caller_sites: &BTreeSet<usize>) -> bool {
    let (Some(owner), Some(field)) = (
        crate::aggregate_store::place_owner(place),
        crate::aggregate_store::place_field(place),
    ) else {
        return false;
    };
    let Some(owner_site) = resolved_aggregate_owner(state, owner, caller_sites) else {
        return false;
    };
    let Some(pointer) = state.pointers.get(place) else {
        return false;
    };
    if pointer.unknown || pointer.targets.is_empty() {
        return false;
    }
    if state
        .field_links
        .get(&owner_site)
        .and_then(|fields| fields.get(field))
        != Some(&pointer.targets)
    {
        return false;
    }
    pointer.targets.iter().all(|site| {
        state.resources.get(site) == Some(&LIVE) && !state.unresolved_effect_sites.contains(site)
    })
}

/// The single live identity an admitted aggregate base denotes here, when the
/// projection can name exactly one and this body acquired it.
fn resolved_aggregate_owner(
    state: &AbstractState,
    owner: &str,
    caller_sites: &BTreeSet<usize>,
) -> Option<usize> {
    let pointer = state.pointers.get(owner)?;
    if pointer.unknown || pointer.may_be_null || pointer.targets.len() != 1 {
        return None;
    }
    let site = pointer.targets.first().copied()?;
    if caller_sites.contains(&site)
        || state.nullable_sites.contains(&site)
        || state.unresolved_effect_sites.contains(&site)
    {
        return None;
    }
    (state.resources.get(&site) == Some(&LIVE)).then_some(site)
}

/// Whether a scalar member-store base denotes one identity that is safe to
/// dereference at this point.  Caller provenance is immaterial: unlike an
/// aggregate link store, this instruction neither creates nor transfers an
/// identity.  Nullability, ambiguity, release/escape and unresolved effects
/// all refuse.
fn scalar_member_store_base_resolves(state: &AbstractState, base: &str) -> bool {
    let Some(pointer) = state.pointers.get(base) else {
        return false;
    };
    if pointer.unknown || pointer.may_be_null || pointer.targets.len() != 1 {
        return false;
    }
    let Some(site) = pointer.targets.first() else {
        return false;
    };
    !state.nullable_sites.contains(site)
        && !state.unresolved_effect_sites.contains(site)
        && state.resources.get(site) == Some(&LIVE)
}

#[allow(clippy::too_many_arguments)]
fn apply_resource_action(
    function: &str,
    line: usize,
    source: &str,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    escape: bool,
    nullable: bool,
) {
    let pointer = state
        .pointers
        .get(source)
        .cloned()
        .unwrap_or(AbstractPointer {
            targets: BTreeSet::new(),
            unknown: true,
            may_be_null: true,
        });
    let unresolved = pointer.unknown
        || pointer.targets.len() > 1
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null));
    if unresolved {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    let unresolved_nullability = !nullable
        && pointer
            .targets
            .iter()
            .any(|site| state.nullable_sites.contains(site));
    if unresolved_nullability {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!("nullable resource `{source}` used by a non-null contract"),
            },
        });
    }
    if unresolved || unresolved_nullability {
        // One source action cannot discharge every possible selected target.
        // Preserve the residual obligations when target identity is unknown.
        mark_unresolved_effect(state, &pointer);
        return;
    }
    if !cascade_is_unambiguous(state, &pointer.targets) {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        mark_unresolved_effect(state, &pointer);
        return;
    }
    for site in pointer.targets {
        let status = state.resources.get(&site).copied().unwrap_or(0);
        if escape && status != LIVE {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
            continue;
        }
        if status & CONTAINED != 0 {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
        if escape {
            state
                .resources
                .insert(site, (status & !(LIVE | CONTAINED)) | ESCAPED);
        } else {
            if status & RELEASED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::DoubleRelease {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            if status & ESCAPED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::ReleaseAfterEscape {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            state
                .resources
                .insert(site, (status & !(LIVE | CONTAINED)) | RELEASED);
        }
        cascade_resource_action(state, site, escape);
    }
}

fn mark_unresolved_effect(state: &mut AbstractState, pointer: &AbstractPointer) {
    let mut pending = if pointer.unknown {
        state.resources.keys().copied().collect::<BTreeSet<_>>()
    } else {
        pointer.targets.clone()
    };
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if visited.insert(site) {
            state.unresolved_effect_sites.insert(site);
            if let Some(children) = state.contained.get(&site) {
                pending.extend(children);
            }
            if let Some(fields) = state.field_links.get(&site) {
                pending.extend(fields.values().flatten().copied());
            }
        }
    }
}

fn cascade_is_unambiguous(state: &AbstractState, roots: &BTreeSet<usize>) -> bool {
    let mut pending = roots.clone();
    let mut visited = BTreeSet::new();
    while let Some(owner) = pending.pop_first() {
        if !visited.insert(owner) {
            return false;
        }
        for child in state.contained.get(&owner).into_iter().flatten() {
            if state.resources.get(child) != Some(&CONTAINED)
                || state
                    .contained
                    .iter()
                    .any(|(other, children)| *other != owner && children.contains(child))
            {
                return false;
            }
            pending.insert(*child);
        }
    }
    true
}

fn cascade_resource_action(state: &mut AbstractState, owner: usize, escape: bool) {
    let mut pending = state.contained.remove(&owner).unwrap_or_default();
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if !visited.insert(site) {
            continue;
        }
        if let Some(children) = state.contained.remove(&site) {
            pending.extend(children);
        }
        let status = state.resources.get(&site).copied().unwrap_or(0);
        let terminal = if escape { ESCAPED } else { RELEASED };
        state
            .resources
            .insert(site, (status & !(LIVE | CONTAINED)) | terminal);
    }
    if escape {
        cascade_field_links_on_escape(state, owner);
    }
}

/// B08: publishing an aggregate publishes the resources its admitted members
/// link, so a live linked child escapes with its owner rather than being
/// reported as a leak. A child in any other state is left exactly as it is and
/// recorded as an unresolved effect: a member still pointing at a released
/// identity when its aggregate escapes is a hazard this projection will not
/// decide in either direction.
///
/// **Releasing** an owner deliberately does not reach this function
/// (premise P-B08-SHALLOW): a declared `resource_released` release names one
/// identity, and a plain C field store carries no declared destructor
/// obligation, so a child still live there stays live and is reported.
fn cascade_field_links_on_escape(state: &mut AbstractState, owner: usize) {
    let mut pending = state
        .field_links
        .remove(&owner)
        .into_iter()
        .flat_map(|fields| fields.into_values().flatten())
        .collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if !visited.insert(site) {
            continue;
        }
        if let Some(fields) = state.field_links.remove(&site) {
            pending.extend(fields.into_values().flatten());
        }
        if let Some(children) = state.contained.remove(&site) {
            pending.extend(children);
        }
        match state.resources.get(&site).copied() {
            Some(LIVE) => {
                state.resources.insert(site, ESCAPED);
            }
            Some(ESCAPED) | None => {}
            Some(_) => {
                unresolved.insert(site);
            }
        }
    }
    state.unresolved_effect_sites.extend(unresolved);
}

#[allow(clippy::too_many_arguments)]
fn apply_contract_call(
    function: &str,
    line: usize,
    called: &str,
    destination: Option<&str>,
    arguments: &[Option<String>],
    contract: &ResourceFunctionContract,
    produced_sites: &[usize],
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    // Reallocation consumes at most one proven live original on success.
    // A null old value carries no custody only when its input contract permits it.
    // Validate it before any other parameter effect or result allocation.
    if let ResourceReturnMode::Reallocated { parameter, .. } = contract.result {
        let input_nullable = matches!(
            contract.parameters.get(parameter),
            Some(ResourceParameterMode::Owned { nullable: true })
        );
        let source = arguments.get(parameter).and_then(Option::as_deref);
        let old = source
            .and_then(|source| state.pointers.get(source))
            .cloned();
        let valid = old.as_ref().is_some_and(|old| {
            !old.unknown
                && old.targets.len() <= 1
                && (input_nullable || (old.targets.len() == 1 && !old.may_be_null))
                && old.targets.iter().all(|site| {
                    state.resources.get(site) == Some(&LIVE)
                        && (input_nullable || !state.nullable_sites.contains(site))
                        && !state.unresolved_effect_sites.contains(site)
                })
        });
        if !valid {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::UnknownAlias {
                    variable: source.unwrap_or(called).to_owned(),
                },
            });
            mark_unresolved_effect(
                state,
                &old.unwrap_or(AbstractPointer {
                    unknown: true,
                    ..Default::default()
                }),
            );
            return;
        }
    }
    let mut sites = produced_sites.iter().copied();
    // Apply input custody effects before assigning a return value. This is
    // essential when the destination aliases a consumed/reallocated argument.
    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        let source = arguments.get(index).and_then(Option::as_deref);
        match mode {
            ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => {}
            ResourceParameterMode::Owned { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                require_live_resource(function, line, source, nullable, state, diagnostics);
            }
            ResourceParameterMode::Consumed { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                apply_resource_action(
                    function,
                    line,
                    source,
                    state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    nullable,
                );
            }
            ResourceParameterMode::Released { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                apply_resource_action(
                    function,
                    line,
                    source,
                    state,
                    acquisition_lines,
                    diagnostics,
                    false,
                    nullable,
                );
            }
            ResourceParameterMode::Stored {
                owner_parameter,
                nullable,
            } => {
                let owner = arguments.get(owner_parameter).and_then(Option::as_deref);
                let (Some(source), Some(owner)) = (source, owner) else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                attach_contained_resource(
                    function,
                    line,
                    owner,
                    source,
                    nullable,
                    state,
                    acquisition_lines,
                    diagnostics,
                );
            }
            ResourceParameterMode::Removed {
                owner_parameter,
                nullable,
            } => {
                let owner = arguments.get(owner_parameter).and_then(Option::as_deref);
                let (Some(source), Some(owner)) = (source, owner) else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                detach_contained_resource(
                    function,
                    line,
                    owner,
                    source,
                    nullable,
                    state,
                    acquisition_lines,
                    diagnostics,
                );
            }
        }
    }

    match contract.result {
        ResourceReturnMode::None => {
            // The result carries no custody, so the destination holds no
            // resource afterwards. Dropping the binding — rather than leaving
            // the previous one in place — keeps a stale alias from covering for
            // a resource the assignment actually dropped.
            if let Some(destination) = destination {
                state.pointers.remove(destination);
            }
        }
        ResourceReturnMode::Owned { nullable } => {
            let (Some(destination), Some(site)) = (destination, sites.next()) else {
                report_unsupported_contract(function, line, called, diagnostics);
                return;
            };
            create_resource(state, destination, site, nullable);
        }
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => {
            let source = arguments.get(parameter).and_then(Option::as_deref);
            let (Some(destination), Some(source), Some(site)) = (destination, source, sites.next())
            else {
                report_unsupported_contract(function, line, called, diagnostics);
                return;
            };
            let Some(old) = state.pointers.get(source).cloned() else {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                return;
            };
            let input_nullable = matches!(
                contract.parameters.get(parameter),
                Some(ResourceParameterMode::Owned { nullable: true })
            );
            if old.unknown || old.targets.len() > 1 || (old.targets.is_empty() && !input_nullable) {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                mark_unresolved_effect(state, &old);
                return;
            }
            create_resource(state, destination, site, nullable);
            state
                .conditional_replacements
                .insert(site, old.targets.clone());
            if !nullable {
                finalize_replacement_success(state, site);
            }
        }
    }

    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        if let ResourceParameterMode::Out { nullable } = mode {
            let source = arguments.get(index).and_then(Option::as_deref);
            let (Some(destination), Some(site)) = (source, sites.next()) else {
                report_unsupported_contract(function, line, called, diagnostics);
                continue;
            };
            create_resource(state, destination, site, nullable);
        }
    }
}

fn create_resource(state: &mut AbstractState, destination: &str, site: usize, nullable: bool) {
    // Acquisition sites may repeat in loops; retain prior uncertainty rather
    // than assuming this abstract site is a fresh runtime generation.
    state.resources.insert(site, LIVE);
    if nullable {
        state.nullable_sites.insert(site);
    } else {
        // Re-executing a non-null acquisition site in a loop is a strong
        // update. A predecessor on which the destination did not yet exist
        // must not permanently poison the fresh result as nullable.
        state.nullable_sites.remove(&site);
    }
    state.pointers.insert(
        destination.to_owned(),
        AbstractPointer {
            targets: BTreeSet::from([site]),
            unknown: false,
            may_be_null: nullable,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn attach_contained_resource(
    function: &str,
    line: usize,
    owner: &str,
    source: &str,
    nullable: bool,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let owner_live = require_live_resource(function, line, owner, false, state, diagnostics);
    let child_live = require_live_resource(function, line, source, nullable, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        return;
    };
    if !owner_live || !child_live {
        mark_unresolved_effect(state, &children);
        return;
    }
    if owners.unknown || children.unknown || owners.targets.len() != 1 || children.targets.len() > 1
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        mark_unresolved_effect(state, &children);
        return;
    }
    let owner_site = *owners.targets.first().expect("one owner target");
    for child in children.targets {
        let status = state.resources.get(&child).copied().unwrap_or(0);
        if status & LIVE == 0 || status & CONTAINED != 0 || child == owner_site {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, child),
                },
            });
            continue;
        }
        state.resources.insert(child, (status & !LIVE) | CONTAINED);
        state.contained.entry(owner_site).or_default().insert(child);
    }
}

#[allow(clippy::too_many_arguments)]
fn detach_contained_resource(
    function: &str,
    line: usize,
    owner: &str,
    source: &str,
    nullable: bool,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let owner_live = require_live_resource(function, line, owner, false, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return;
    };
    if !owner_live
        || owners.unknown
        || children.unknown
        || owners.targets.len() != 1
        // A call detaches one child, not every possible target of an alias
        // joined from different paths. Their correlation is not represented.
        || children.targets.len() > 1
        || (!nullable && (children.targets.is_empty() || children.may_be_null))
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        return;
    }
    let owner_site = *owners.targets.first().expect("one owner target");
    for child in children.targets {
        let containing_owners = state
            .contained
            .iter()
            .filter_map(|(owner, children)| children.contains(&child).then_some(*owner))
            .collect::<BTreeSet<_>>();
        let status = state.resources.get(&child).copied().unwrap_or(0);
        // Every represented path must still own this exact child through the
        // owner. A joined CONTAINED|LIVE state includes an already-detached
        // path; removing the CONTAINED bit would erase its repeated removal.
        // The union of two different possible owners is not proof that the
        // requested owner contains the child on every represented path.
        if containing_owners != BTreeSet::from([owner_site]) || status != CONTAINED {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, child),
                },
            });
            continue;
        }
        if let Some(contained) = state.contained.get_mut(&owner_site) {
            contained.remove(&child);
            if contained.is_empty() {
                state.contained.remove(&owner_site);
            }
        }
        state.resources.insert(child, (status & !CONTAINED) | LIVE);
    }
}

fn require_live_resource(
    function: &str,
    line: usize,
    source: &str,
    nullable: bool,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    let Some(pointer) = state.pointers.get(source) else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return false;
    };
    let invalid = pointer.unknown
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null))
        || (!nullable
            && pointer
                .targets
                .iter()
                .any(|site| state.nullable_sites.contains(site)))
        || pointer.targets.iter().any(|site| {
            state.resources.get(site) != Some(&LIVE) || state.unresolved_effect_sites.contains(site)
        });
    if invalid {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    !invalid
}

fn report_unsupported_contract(
    function: &str,
    line: usize,
    called: &str,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("arguments do not match resource contract for `{called}`"),
        },
    });
}

fn finalize_replacement_success(state: &mut AbstractState, site: usize) {
    state.nullable_sites.remove(&site);
    if let Some(replaced) = state.conditional_replacements.remove(&site) {
        for old in replaced {
            let status = state.resources.get(&old).copied().unwrap_or(0);
            state.resources.insert(old, (status & !LIVE) | RELEASED);
        }
    }
}

enum PointerRefinement {
    Refined,
    Infeasible,
    Unknown,
}

/// Card **SCALAR-ROUTE-EXIT-EDGE**: the refusal a loop header reports when the
/// value route called its exit edge infeasible and the edge was retained
/// anyway. It is `Unsupported`, so [`classify_diagnostics`] reads `Unknown` —
/// the fail-closed answer for a path whose absence the analyzer cannot prove.
pub(crate) const LOOP_EXIT_EDGE_RETAINED: &str =
    "loop exit edge retained: a modeled value cannot remove it";

fn refine_pointer_null(
    state: &mut AbstractState,
    source: &str,
    assume_null: bool,
) -> PointerRefinement {
    let Some(pointer) = state.pointers.get(source).cloned() else {
        // The reduced importer has no scalar type environment. An untracked
        // identifier may be an ordinary integer condition, so leave it alone.
        return PointerRefinement::Refined;
    };
    if pointer.unknown || pointer.targets.len() > 1 {
        // Nullness is independent of identity. A guard narrows its own
        // binding, never another alias or a resource's existence. Keep the
        // identity refusal and every obligation on both edges.
        let narrowed = state.pointers.get_mut(source).expect("existing binding");
        if assume_null {
            narrowed.targets.clear();
            narrowed.unknown = false;
            narrowed.may_be_null = true;
        } else {
            narrowed.may_be_null = false;
        }
        return PointerRefinement::Unknown;
    }
    let Some(site) = pointer.targets.iter().next().copied() else {
        return if assume_null {
            PointerRefinement::Refined
        } else {
            PointerRefinement::Infeasible
        };
    };
    if !pointer.may_be_null && !state.nullable_sites.contains(&site) {
        return if assume_null {
            PointerRefinement::Infeasible
        } else {
            PointerRefinement::Refined
        };
    }
    if assume_null {
        state.resources.remove(&site);
        state.nullable_sites.remove(&site);
        state.conditional_replacements.remove(&site);
        // The null alternative has no child at this site. Remove precisely
        // its incoming aggregate edges, retaining every sibling obligation.
        state.contained.retain(|_, children| {
            children.remove(&site);
            !children.is_empty()
        });
        state.field_links.retain(|_, fields| {
            fields.retain(|_, children| {
                children.remove(&site);
                !children.is_empty()
            });
            !fields.is_empty()
        });
        for alias in state.pointers.values_mut() {
            if alias.targets.remove(&site) {
                alias.may_be_null = true;
            }
        }
    } else {
        finalize_replacement_success(state, site);
        for alias in state.pointers.values_mut() {
            if alias.targets.contains(&site) {
                alias.may_be_null = false;
            }
        }
    }
    PointerRefinement::Refined
}

fn acquisition_line(acquisition_lines: &BTreeMap<usize, usize>, site: usize) -> usize {
    acquisition_lines.get(&site).copied().unwrap_or(0)
}

fn report_leaks(
    function: &str,
    line: usize,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    // Caller-owned parameter resources are discharged by the caller; a live
    // formal at exit is the interface, not a leak.
    exempt: &BTreeSet<usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (&site, &status) in &state.resources {
        if status & LIVE != 0
            && !exempt.contains(&site)
            && !state.unresolved_effect_sites.contains(&site)
        {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Leak {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_c.rs"]
mod tests;
