//! Conservative resource analysis for the Cycle 9 annotated legacy-C subset.
//!
//! The parser intentionally accepts more C than the analyzer models.  Every
//! construct outside the supported structured subset becomes an `Unknown`
//! instruction. `Safe` concerns the supported custody projection under its
//! declared resource contracts, not arbitrary C memory safety. Indirect reads,
//! aggregate/indexed accesses and ambiguous expression forms without a model
//! remain unsupported; symbolic resource place spelling is not access proof.

#[path = "legacy_joined_provenance.rs"]
pub(crate) mod joined_provenance;

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
// Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): an enumeration's integer
// projection, read from Clang over the analysed unit.
#[path = "legacy_enum_projection.rs"]
pub(crate) mod enum_projection;
// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): route G's dialect and extension hooks.
#[path = "legacy_dialect_hook.rs"]
mod dialect_hook;

#[path = "legacy_scalar_precedence.rs"]
mod scalar_precedence;

pub use declared_abi::{AbiAgreementRow, AbiOutcome, AbiRefusal, declared_abi_census};

#[path = "legacy_declared_modes.rs"]
pub mod declared_modes;

// Card W45-CONTRACT-DEFERRED-REVIEW: a declared contract awaiting human review
// is withheld and its call sites are held. Everything the card adds lives
// there; this file only carries the hooks marked with the card's name.
#[path = "legacy_review_hold.rs"]
pub mod review_hold;

pub use review_hold::{AWAITING_HUMAN_REVIEW, held_call, hold_description};

// Card W45-CONTRACT-REVISION-MODE: a pinned contract the audit withheld for a
// pin mismatch; its call sites are held exactly as a pending review's are.
#[path = "legacy_pin_hold.rs"]
pub mod pin_hold;

// Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: a `Pending` row is joined
// with every reason its function would carry with the held declarations in
// force, so a held call never masks another reason.
#[path = "legacy_review_standing.rs"]
pub mod review_standing;

// Card W45-REFUTED-CONTRACT-WITHDRAWAL: a declared contract its own analysed
// body refutes is withdrawn from every caller, in its unit and, through the
// audit's refutation table, in every other unit.
#[path = "legacy_refuted_contract.rs"]
pub mod refuted_contract;

// Card W48-RK-LOG-CALLER-FRAME (rule 32a): a caller-frame declaration serves
// only callers outside its frame; a member's call to a member is held.
#[path = "legacy_caller_frame_hold.rs"]
pub mod caller_frame_hold;

// Card CONTRACT-SUPERSESSION (contract spec C06.6): a function whose
// contract bundles' pin stamps do not decide which declaration is in force;
// its call sites are held (`contract-conflict`, never `Pending`).
#[path = "legacy_conflict_hold.rs"]
pub mod conflict_hold;

// Card W45-SR5-SOUNDNESS: the reserved rewrite prefixes, escaped where the
// source spells them, so a held or withdrawn call is only ever tool-minted.
#[path = "legacy_reserved_spelling.rs"]
pub mod reserved_spelling;

// Card W45-USER-CONTRACT-PLUGINS: the four-value verdict. Function and unit
// statuses are folds of the verdict semigroup; a held call is `Pending`.
#[path = "legacy_verdict.rs"]
pub mod verdict;

// Card W45-PARENTHESISED-CALLEE: `(f)(x)` is `f(x)`, and any other callee
// that is not a name refuses. Everything the card adds lives there; this file
// only carries the hooks marked with the card's name.
#[path = "legacy_parenthesised_callee.rs"]
pub(crate) mod parenthesised_callee;

// Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p): a call through a
// function pointer whose candidate set the unit closes is rewritten to the
// calls C may make, right after rule 10c's normalisation.
#[path = "legacy_indirect_calls.rs"]
mod indirect_call_route;

#[path = "legacy_descriptor_custody.rs"]
pub(crate) mod descriptor_custody;

// Card CG25-RESOURCE-KINDS (rule 11v): in-place resource kinds — the storage
// a `&NAME` actual designates carries the obligation, the initialiser's status
// is its sentinel. Hooks below naming the card are thin.
#[path = "legacy_in_place_custody.rs"]
pub(crate) mod in_place_custody;

// Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: typed indirect reads in the actuals
// of a contracted call. Everything the card adds lives there; this file only
// carries the hooks marked with the card's name.
#[path = "legacy_call_argument_access.rs"]
pub(crate) mod call_argument_access;
// Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12i): a refused `return`'s one
// call read with rule W44's call-argument access, asked last.
#[path = "legacy_return_call_access.rs"]
mod return_call_access;
// Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule 12k): the gate's refusal of a
// call whose callee is uncontracted names that callee when it decides; asked
// last, a re-attribution that never admits.
#[path = "legacy_uncontracted_access.rs"]
mod uncontracted_access;
// Card MEMBER-EFFECT-SUMMARY step 1 (rule 38i): the gate's refusal of member
// custody through a formal parent names its premise; asked last, a
// re-attribution that never admits.
#[path = "legacy_member_custody_attribution.rs"]
mod member_custody_attribution;
// Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (rules 14a, 14b): one
// contracted scalar call inside a typed scalar statement, and an interior
// pointer into an untracked object; asked after the statement fallbacks.
#[path = "legacy_residual_access.rs"]
mod residual_access;

// Card W45-CALLEE-SUMMARY-CASCADE: a pointer member handed to a nullable
// release, a pointer member null test, and the call-site sub-custody guard
// before every call to a function defined in this unit. Hooks name the card.
#[path = "legacy_member_release.rs"]
mod member_release;

// Card DECLARED-ASSUMPTION-BODY-CHECKS (rules 13m, 13n): a declaration whose
// W4 summary only refines it, checked evidence-only against its body.
#[path = "legacy_declared_entry_check.rs"]
pub(crate) mod declared_entry_check;

pub use descriptor_custody::ValidityFact;

// Card W44-BRANCH-CONDITION-CALLS: the desugaring of a refused call-bearing
// controlling expression into calls and branch edges. Every hook below that
// names `condition_desugar` is consulted only where the position's existing
// lowering is its refusal.
#[path = "legacy_condition_desugar.rs"]
mod condition_desugar;

// Card W47-ACCESS-PATH-POSITIONS (rule 10k): the Clang-typed access-path layer
// in condition operands, returns and call actuals, each a fallback after the
// position's existing routes. The hooks below naming the card are thin.
#[path = "legacy_typed_positions.rs"]
mod typed_positions;

// Card MEMBER-ACCESS-PROJECTION (rule 38h): an uninitialized declaration the
// lexical gate refuses, read from the Clang AST. The gate's hook is thin.
#[path = "legacy_typed_declaration.rs"]
mod typed_declaration_gate;

// Card W47-TYPED-ROOT-DECLARATION-READER (rule 10l): the typed positions'
// never-bound, `may_bind` and declared-local premises, from the Clang AST
// where the lexical declaration reader answers nothing. Hooks are thin.
#[path = "legacy_typed_roots.rs"]
mod typed_roots;

// Card CG02-ELABORATION-CORE (rule 10s): the elaborator's route-G adapter, a
// fallback after every existing route of a statement, a `return` and a
// call-free controlling expression. The hooks below naming the card are thin.
// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): an atomic builtin the
// classifier refused as an uncontracted call, routed to the weak-memory
// model's vocabulary. Hooks name the card.
#[path = "legacy_atomic_route.rs"]
mod atomic_route;

// Card CG26-VARIADICS (gateway rule 11w): the `va_list` builtins' dedicated
// recognised form, a fallback after every existing route of a statement.
#[path = "legacy_variadics.rs"]
mod variadics_route;

// Card VA-LOOP-AND-POINTER-ARG (gateway rule 12c): a `va_arg` in a controlling
// expression, asked after every older route of the position (the one hook is
// in `legacy_condition_desugar.rs`'s `planned_condition`).
#[path = "legacy_va_condition.rs"]
mod va_condition_route;

#[path = "legacy_elaboration.rs"]
mod elaboration_route;

// Card CG03-CALL-SUMMARY-INSTANTIATION (rule 10u): the one call-contract
// application point and its new consumers (a scalar `return` operand, a call
// whose actuals carry effects, a typed substitution), each a fallback after
// every existing route of its position. The hooks below naming the card are
// thin.
#[path = "legacy_call_instantiation.rs"]
mod call_instantiation_route;
// Card CG05-NESTED-CALL-EFFECTS (rule 10y): nested and sibling calls with
// non-identity effects, through the one application point.
#[path = "legacy_nested_call.rs"]
mod nested_call_route;
// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the elaborator's own call
// events with their contract footprints, lowered through the one application
// point inside CG02's statement route.
#[path = "legacy_elaborator_calls.rs"]
mod elaborator_calls_route;

// Card CG04-CONDITION-ACQUISITION (rule 10x): ownership-producing calls in
// controlling expressions and complex condition leaves, decided inside the
// condition desugaring (rule 5h). The hooks naming the card are thin.
#[path = "legacy_condition_acquisition.rs"]
mod condition_acquisition_route;

// Card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): value `&&`, `||`
// and `?:` in statements and returns, lowered by branching, a fallback after
// every existing route of the position. The hooks below naming the card are
// thin.
#[path = "legacy_value_connective.rs"]
mod value_connective_route;

// Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): GNU statement
// expressions in a discarded, initializer, assigned or returned position,
// lowered from the stream in C's order, a fallback after every existing route
// of the position. The hooks below naming the card are thin.
#[path = "legacy_statement_expression.rs"]
mod statement_expression_route;

// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): a declared no-return call
// the Clang declaration establishes ends its path; inline assembly, SEH and
// `cleanup` attributes are named boundaries asked first. The hooks below
// naming the card are thin.
#[path = "legacy_call_exits.rs"]
mod call_exits_route;

// Card CG04-LOOP-SITE-GENERATIONS: a re-acquired site on a CFG cycle keeps
// its earlier generation apart. The hooks below naming the card are thin.
#[path = "legacy_loop_generations.rs"]
mod loop_generations_route;

// Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (rule 11c): a field link that
// holds on some paths only never leaves an obligation unresolved without a
// diagnostic. The hooks below naming the card are thin.
#[path = "legacy_field_link_join.rs"]
mod field_link_join_route;

// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): the null test is no escape or
// binding, pointer-to-pointer casts at a pointer destination, the cast return
// the reader does not commit to, and Clang-confirmed unevaluated `sizeof`
// operands at the gate. The hooks below naming the card are thin.
#[path = "legacy_cast_representation.rs"]
mod cast_representation_route;

// Card CAST-FORMALIZATION (rule 12h): the one conversion table's route-G
// consultations and the explicit-cast census. The hooks naming the card are
// thin.
#[path = "legacy_cast_semantics.rs"]
mod cast_semantics_route;

// Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): a static-duration
// declaration is no event, and `volatile` objects carry no value-derived
// fact. The hooks below naming the card are thin.
#[path = "legacy_storage_duration.rs"]
mod storage_duration_route;

// Card W47-ACCESS-PATH-POSITION-RESIDUALS (rule 10n): the P3 residual shapes
// C's evaluation order admits. Hooks are thin.
#[path = "legacy_typed_residuals.rs"]
mod typed_residuals;

// Card W44-LEAK-PIN-AUDIT: the refused-escape rule, the cast return
// (`RETURN-CAST-PLACE-NOT-ESCAPED`) and the leak-witness census.
#[path = "legacy_refused_operands.rs"]
pub(crate) mod refused_operands;

#[path = "legacy_cast_return.rs"]
pub(crate) mod cast_return;

// Card SUP07-REFUSED-TEST-ACCUSATION (rule 12e): the null edge of a refused
// test. Everything the card adds lives there; this file only carries the hook
// marked with the card's name.
#[path = "legacy_refused_test.rs"]
pub(crate) mod refused_test;

// Card W45-RETURN-EXPRESSION-FAMILIES: the scalar-result read and the cast of
// a whole call in `return` position. Everything the card adds lives there;
// this file only carries the hooks marked with the card's name.
#[path = "legacy_return_family.rs"]
pub(crate) mod return_family;
// Card W45-SUB-OBJECT-STORES: a store into a sub-object through a resolved,
// typed base. Everything the card adds lives there; this file only carries the
// hooks marked with the card's name.
#[path = "legacy_sub_object_store.rs"]
pub(crate) mod sub_object_store;
// Card CG11-POINTER-VALUED-STORES (rule 10v): the flow of a pointer store.
#[path = "legacy_pointer_store.rs"]
mod pointer_store_flow;
// Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the per-path bound-untracked
// root premise, read off one state, and its census.
#[path = "legacy_bound_untracked.rs"]
mod bound_untracked_route;
// Card NULL-SAFETY-PROPERTY (rule 14n): decision 1's non-null reading of a
// dereferenced target-free alias, under a named premise.
#[path = "legacy_null_premise.rs"]
mod null_premise_route;
// Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f): record copies,
// braced initializers, compound literals and string initializers, asked last.
#[path = "legacy_aggregate_value.rs"]
mod aggregate_value_route;
// Card CG12-VOID-VIEW-READBACK (rule 11j): the readback of a `void *`
// member, its static premises on the parser and its flow decision.
#[path = "legacy_void_view.rs"]
mod void_view_route;
// Card CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): VLA declarations and
// `container_of` values, asked last, and the layout census.
#[path = "legacy_layout_extents.rs"]
mod layout_extents_route;
// Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): return-code dispositions
// at call sites (the class environment, the stored-return rewrite, the `if`
// plan and the untested refusal).
#[path = "legacy_relational_post.rs"]
mod relational_route;
// Card HANDOFF-DIRECT-CALL-READING (rule 12m amended): a direct call of a
// hand-off declaration, split at the caller's own test of its result (the
// stored-return rewrite, the `if` plan and the refusal of every other call).
#[path = "legacy_handoff_direct.rs"]
mod handoff_direct_route;
// Card RELATIONAL-REWRITE-LINE-ORDER (rule 11r amended): the line order of
// the stream the parser reads.
#[path = "legacy_stream_line_order.rs"]
mod stream_line_order;
// Card CALLEE-RETENTION-CLASS1 (rule 12t): retained pointers (the minted
// custody environment, the call-site decision, the records the transfer
// keeps, the release and exit refusals).
#[path = "legacy_retained_by.rs"]
pub(crate) mod retained_by_route;
// Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): descriptor cells (the operands,
// the cell table's transfer, the witnesses, the exit and release refusals).
#[path = "legacy_descriptor_cells.rs"]
pub(crate) mod descriptor_cells_route;
// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): callee frames over a
// record's declared cell protocol, read in its release window.
#[path = "legacy_cell_frames.rs"]
pub(crate) mod cell_frames_route;
// Card W43-ABI-ENUM-AND-CONST-PP (rule 12w): guarded retention clauses.
#[path = "legacy_retention_guard.rs"]
pub(crate) mod retention_guard;
// Card SLOT-MOVE-AND-INTERIOR-BORROW (rules 12x, 12y, 12z, 13a).
#[path = "legacy_detach.rs"]
mod detach_route;
#[path = "legacy_lookup.rs"]
mod lookup_route;
// Card DETACH-RESULT-WRAPPER-COMPOSITION (rule 33): checked removed-selection
// frames, their build-stream expansion and their selection entry.
#[path = "legacy_selection.rs"]
pub(crate) mod selection_route;
#[path = "legacy_slot_move_borrow.rs"]
pub(crate) mod slot_move_route;
// Card REFCOUNT-RESOURCE-KIND (rules 13b, 13c): reference counts (the
// operands, the keep and drop transfer, the exit balance, the body checks).
#[path = "legacy_reference_count.rs"]
pub(crate) mod reference_route;
// Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule 13d): the body check of
// a returned-reference declaration, called from `reference_route::settle`.
#[path = "legacy_reference_result.rs"]
pub(crate) mod reference_result_route;
// Card CG22-CALLBACK-REGISTRY-CONTRACT (rules 13u-13y): callback contracts
// (the call-site decision of invocations and tables, the registry records
// the transfer keeps, the release refusal, the body checks).
#[path = "legacy_callback_route.rs"]
pub(crate) mod callback_route;
// Card SERVE-FORWARDED-CALLBACK-FORMALS (rule 15w): a callback formal handed
// on unchanged to the same clause position of a callee in force, read at the
// body check (`callback_route::settle`) and at the forwarding call.
#[path = "legacy_callback_forward.rs"]
pub(crate) mod callback_forward;
// Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): a constructor's
// stated result state on route G.
#[path = "legacy_constructed_route.rs"]
pub(crate) mod constructed_route;
// Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): a callback candidate's
// stated member-path states, composed with the arms at an invocation.
#[path = "legacy_member_path_route.rs"]
pub(crate) mod member_path_route;

// Card W44-C11-GENERIC-SELECTION: a C11 generic selection whose associations
// are one expression up to pointer casts is lowered to that expression before
// a body is parsed; every other selection is left as it was.
#[path = "legacy_generic_selection.rs"]
mod generic_selection;

// Card W45-FB-BACKLOG-CUSTODY: intrusive circular-list iteration and element
// identity (gateway constraint 5l). Everything the card adds lives there; this
// file only carries the hooks marked with the card's name.
#[path = "legacy_list_iteration.rs"]
pub(crate) mod list_iteration;
// Card W45-FB-BACKLOG-CUSTODY: a declaration marked `resource_body_checked` is
// in force only where its body establishes it (contract spec §C06.2).
#[path = "legacy_checked_spec.rs"]
pub(crate) mod checked_spec;

// Card W48-BOTTOM-UP-CALLEE-BODIES (rule 12f): a scalar `return` value read
// as the value of a statement.
#[path = "legacy_scalar_return_value.rs"]
mod scalar_return_value;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the statement and return fallback
// orders as explicit tables, asked by one dispatcher each.
#[path = "legacy_route_table.rs"]
mod route_table;

// Card GAPO-RETURN-CORRECTNESS (rule 15x): the parser state a classification
// probe may move, saved as one value and handed back by a declined probe.
#[path = "legacy_probe_state.rs"]
mod probe_state;

// Card GAPO-RETURN-CORRECTNESS (rule 15y): a declared no-return call ends its
// path at every lowered position, settled on the built CFG.
#[path = "legacy_no_return_paths.rs"]
mod no_return_paths;

#[cfg(feature = "indirect-census")]
#[path = "legacy_leak_witness.rs"]
pub(crate) mod leak_witness;

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
    ContractTrust, DerivedExtent, ResourceApiKind, ResourceContracts, ResourceFunctionContract,
    ResourceKind, ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NestedContractCall {
    pub function: String,
    pub contract: ResourceFunctionContract,
    pub arguments: Vec<Option<String>>,
}

// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the kinds on
// `ResourceFunctionContract` grew the `ContractCall` variant past clippy's
// size-difference threshold. Instructions are built once per statement and
// cloned only with their CFG, so boxing every contract would add an
// allocation per call statement to buy nothing; the crate's precedent at
// `UnrestrictedLegacyResult` makes the same decision.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        /// Card **W45-FB-RETRY-LIFETIME**: the operands of the call's declared
        /// container effects, applied after its borrowed reading
        /// ([`container_publication`]). `None` for every call whose contract
        /// in force has none.
        container: Option<Box<container_publication::Operands>>,
        /// Card **CALLEE-RETENTION-CLASS1** (rule 12t): the decided
        /// retentions and holder resets of a retaining callee, applied after
        /// its borrowed reading ([`retained_by_route`]). `None` for every
        /// other call.
        retention: Option<Box<retained_by_route::Operands>>,
    },
    BranchOnNull {
        source: String,
        true_is_null: bool,
    },
    /// Card **MK-EVENT-CELL-CROSS-FUNCTION** (rule 38c): a store to, or a
    /// close of, a member of a record with a declared cell protocol, read
    /// after the statement's own lowering (a separate node: the statement's
    /// instruction is unchanged). Emitted only in an environment declaring
    /// a `quant cell` protocol.
    CellProtocolStep {
        step: Box<descriptor_cells_route::ProtocolStep>,
    },
    /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: a branch on the validity of
    /// a name that may hold an integer-carried identity (`fd < 0`,
    /// `fd == -1`, `fd >= 0`, …). It is the integer counterpart of
    /// [`Self::BranchOnNull`]: an `Invalid` edge carries no resource at the
    /// tested identity, a `Valid` edge carries a valid one, and an `Unrefined`
    /// edge carries the state unchanged. A null-shaped test of such a name
    /// (`if (fd)`, `fd == 0`) is one of these with both edges `Unrefined`,
    /// because descriptor `0` is valid. Emitted only for a name
    /// [`descriptor_custody::DescriptorNames`] lists.
    BranchOnValidity {
        source: String,
        when_true: ValidityFact,
        when_false: ValidityFact,
    },
    /// Card **CELL-GUARD-RESULT-RANGE-CHECKED** (gateway rule 38e): a branch
    /// on `source OP literal` where `source` is a cell-creating call's status
    /// witness whose callee's pin-derived result set is in force. It refines
    /// nothing but the creation's class, and only where the test reads that
    /// class exactly within the set; every other edge carries the state
    /// unchanged, as a scalar condition does.
    BranchOnResult {
        source: String,
        comparison: ResultComparison,
        literal: i128,
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
    /// Card **VA-LOOP-AND-POINTER-ARG** (gateway rule 12c): bind
    /// `destination` to a caller's pointer read by `va_arg` — opaque, possibly
    /// null and **never owned**.
    ///
    /// The binding is the one a declared derived result over an unbound base
    /// gets (`derive_pointer(None, Exact, …)`): no target, unknown, possibly
    /// null, `Derived`. So, by the derived-pointer rules already in force
    /// (card W43-LIBC-RESULT-ALIAS), a null test narrows the binding alone, a
    /// `Borrowed` position reads it, and a release, consumption, store,
    /// escape or reallocation through it is never applied and refuses
    /// (`UnknownAlias`, never accused: it has no target). It creates,
    /// releases and transfers no custody.
    VaArgPointer {
        destination: String,
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
        /// Independently checked scalar plan, selected only after its flow obligations discharge.
        typed_alternative: Option<Box<LegacyInstruction>>,
        base: String,
        /// The same C02-P-ALIAS obligation as every other modeled read through
        /// a declared record pointer.
        alias_read: Option<AliasPeers>,
    },
    /// Card **W43-INDIRECT-ACCESS-GENERAL**: one complete statement whose
    /// every memory access is a typed scalar access (`*B`, `B[I]`, `B->F`,
    /// `S.F`) and whose destination and value are scalar; see
    /// [`crate::typed_field_access::typed_scalar_statement`].
    ///
    /// It is custody-neutral: it acquires, releases, transfers and rebinds no
    /// tracked identity, so every projection map is preserved, admitted or
    /// refused. What it adds is the dereference obligation of each base,
    /// checked by [`transfer`]: a base must be unbound on every reaching path,
    /// a non-null binding with no tracked target, or exactly one known,
    /// non-null, live identity with no nullable or unresolved-effect marker.
    /// Anything else records `UnknownAlias`. A base settled on one released
    /// identity is also a `use_after_release` candidate, reported from the
    /// settled states. Bounds are not decided: a subscript's index is read as
    /// a scalar and nothing is claimed about the object's extent.
    IndirectScalarAccess {
        /// Bounded scalar alternative with exactly this destination and write frame.
        typed_alternative: Option<Box<LegacyInstruction>>,
        bases: Vec<String>,
        /// The C02-P-ALIAS obligations of the bases read through `->`.
        alias_reads: Vec<AliasPeers>,
        /// The declared scalar variable written, whose old scalar fact is
        /// forgotten exactly as [`Self::ForgetScalar`] forgets it. Card
        /// W45-FB-BACKLOG-CUSTODY also names here an admitted list cursor or
        /// element (`list_iteration`): its value is list memory this function
        /// does not track, so it too is left unbound.
        destination: Option<String>,
        /// For a write through `*B` or `B[I]` only: the bindings it may have
        /// rewritten, bounded exactly as the refusal it replaces bounded them
        /// ([`RefusedWrites`]). A character-typed write may rewrite a pointer
        /// object's representation, so every binding outside this frame is
        /// made unknown and possibly null, as before; the resource, custody
        /// and field-link maps are untouched. `None` for a read or a member
        /// store, which writes one scalar and nothing else.
        frame: Option<RefusedWrites>,
        /// Card **W44-INDIRECT-CHAIN-ACCESS**: the record-pointer member
        /// chains the statement or condition reads or writes through, each
        /// with its per-link obligation
        /// ([`crate::indirect_chain_access::chain_resolves`]). A dereferenced
        /// chain root is also in `bases`. Empty for every access the
        /// single-selection route admits.
        chains: Vec<crate::indirect_chain_access::AccessChain>,
        /// W46: a pointer-valued path read binds a target-free borrowed alias.
        /// It carries no custody and any later dereference/release must resolve
        /// under its own rules. Older scalar routes always set this false.
        borrowed_destination: bool,
        /// Card **CG12-VOID-VIEW-READBACK** (rule 11j): the `void *` member
        /// place (`R->F`) a borrowed destination reads back, when every
        /// static premise of the readback holds. The flow then binds the
        /// destination to the member's identity where
        /// `bound_read_resolves` holds at the node, and leaves it the
        /// target-free borrowed alias otherwise. `None` on every other
        /// access, which is then exactly as before.
        readback: Option<String>,
    },
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: `BASE[INDEX] = CALL(...)`
    /// or `BASE[INDEX] = NULL`, where `BASE` is declared `T **BASE` and the
    /// existing classifier refused the statement; see
    /// [`crate::indexed_collection`].
    ///
    /// `call` is the acquiring contract call (every parameter `Borrowed`, an
    /// owned result) with its destination renamed to the element place, or
    /// `None` for the null constant. When `BASE` does not resolve to exactly
    /// one live, non-null identity this body acquired, the node is exactly
    /// `fallback`, the refusal the classifier gave it: no effect of the call
    /// applies. Otherwise the call's result moves into the collection at
    /// `INDEX`, and a `name++` index is stepped afterwards.
    ElementStore {
        base: String,
        index: crate::indexed_collection::IndexSpelling,
        call: Option<Box<LegacyInstruction>>,
        fallback: Box<LegacyInstruction>,
    },
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: `CALLEE(BASE[INDEX]);` with a
    /// one-parameter `resource_released` contract and no result. Releases the
    /// element through the collection; `fallback` as for [`Self::ElementStore`].
    ElementRelease {
        base: String,
        index: crate::indexed_collection::IndexSpelling,
        callee: String,
        /// The contract's `nullable` flag: whether a null element may be passed.
        nullable: bool,
        fallback: Box<LegacyInstruction>,
    },
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: an `if` testing one element
    /// against the null constant. The first successor is the true edge.
    BranchOnElementNull {
        base: String,
        index: crate::indexed_collection::IndexSpelling,
        true_is_null: bool,
        fallback: Box<LegacyInstruction>,
    },
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the header of a canonical
    /// counted loop `for (INDEX = 0; INDEX < BOUND; INDEX++)` whose body
    /// neither writes `INDEX` or `BOUND` nor leaves or enters except through
    /// the header. Its body edge carries `INDEX < BOUND`; its exit edge
    /// rewrites a collection bound `INDEX` to `BOUND`. Otherwise a `NoOp`.
    CountedLoopHeader {
        index: String,
        bound: crate::indexed_collection::Term,
    },
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: that loop's `INDEX++`. It
    /// forgets `INDEX`'s scalar fact exactly as [`Self::ForgetScalar`] does and
    /// renames every collection bound `INDEX + k` to `INDEX + (k - 1)`.
    CountedLoopStep {
        index: String,
    },
    /// Card **W43-INDEX-TRAVERSAL**: `destination = source ± offset` over two
    /// declared single-level pointer objects (`P++`, `P += E`, `P = Q + E`,
    /// and the step half of `*P++`); see [`crate::index_traversal`].
    ///
    /// The result points into the object `source` points into (C11 6.5.6p8
    /// under the no-undefined-behaviour reading), so it keeps `source`'s
    /// targets, nullness and unknownness, and is **derived** from them with an
    /// interior extent: it owns nothing, a release through it is never a
    /// release of the base, and a use after the base's release is a candidate.
    /// An unbound `source` gives an unbound `destination`: arithmetic on a
    /// pointer into an object this projection does not track stays there.
    /// Stepping in place the only identity binding of a live tracked object
    /// refuses, because the base would then be named by nothing but
    /// arithmetic the projection does not evaluate. Bounds are not decided.
    PointerStep {
        destination: String,
        source: String,
        offset: StepOffset,
    },
    /// Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS**: the second half of the
    /// obligation of the call actuals the call argument rule admitted, checked
    /// before the call at the same line, after the typed read
    /// ([`Self::IndirectScalarAccess`]) of every base they dereference.
    ///
    /// It is custody-neutral and changes no map. It records `UnknownAlias` for
    /// every name in `names` bound to an identity carried by an integer (a
    /// tracked descriptor is custody, never data), every base in
    /// `pointer_bases` bound to a target that owns sub-custody (a field link, a
    /// contained child or an indexed collection: the pointer read out of it may
    /// be a tracked object the read does not name), and every place in
    /// `pointer_places` (`p->f`, `s.f`) that is itself bound.
    ArgumentReads {
        /// Every name the admitted actuals read.
        names: Vec<String>,
        /// The bases a pointer-valued actual is read through.
        pointer_bases: Vec<String>,
        /// The places a pointer-valued actual spells, when it spells one.
        pointer_places: Vec<String>,
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
    /// Card **W45-SUB-OBJECT-STORES**: a store into one member through a
    /// resolved, typed base that the lexical gate refused and no older route
    /// admits; see [`sub_object_store`]. Every flow premise that fails makes
    /// the node exactly `fallback`, the gate's refusal, so a store that stays
    /// refused still hands on (card W44-LEAK-PIN-AUDIT).
    SubObjectStore {
        destination: sub_object_store::Destination,
        value: sub_object_store::StoredValue,
        /// The bases a scalar value's typed accesses dereference.
        value_bases: Vec<String>,
        /// The C02-P-ALIAS obligation of the dereferenced base.
        alias_read: Option<AliasPeers>,
        fallback: Box<LegacyInstruction>,
    },
    /// Card **CG11-POINTER-VALUED-STORES** (rule 10v): a pointer-valued store
    /// into a typed place whose value carries no identity; see
    /// [`crate::pointer_store`]. `access` is the destination's (and an
    /// address value's) typed access with its per-hop obligations; every flow
    /// premise that fails makes the node exactly `fallback`, the gate's
    /// refusal.
    PointerStore {
        store: crate::pointer_store::PointerStore,
        access: Box<LegacyInstruction>,
        fallback: Box<LegacyInstruction>,
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
    /// Rule 36: source-confirmed original borrowed-formal identity. Appended
    /// to preserve every existing serialized instruction discriminant.
    ExactFormalReturn {
        source: String,
        parameter: usize,
    },
    /// Rule 36a: a source-checked initializer leaf, conditional on the
    /// private row's empty custody frame. Never an ordinary call summary.
    SelfInitCall {
        function: String,
        base: String,
        fallback: Box<LegacyInstruction>,
    },
    /// Rule 36b: explicit checked self-writes into live owned storage.
    /// `path` selects the embedded object; `writes` names its self fields.
    /// Appended; never exported as an ordinary custody summary.
    TrackedSelfInit {
        function: String,
        base: String,
        path: Vec<String>,
        writes: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        protected_fields: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        compatible_roots: Vec<String>,
        access: Box<LegacyInstruction>,
        fallback: Box<LegacyInstruction>,
    },
}

/// Card **W43-INDEX-TRAVERSAL**: what a pointer step does to the pointer's
/// position in its object, as far as the rule commits to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StepOffset {
    /// `P++`, `++P`, `P += N`, `Q + N` with `N` a positive decimal integer
    /// literal: the result lies strictly after the pointer it was computed
    /// from.
    Forward,
    /// Any other amount — a decrement, a subtraction, zero or a variable: the
    /// result may lie anywhere in the object, its start included.
    Unknown,
}

/// Card **W43-INDEX-TRAVERSAL**: the declared names of one function the
/// traversal transfer rules key on, read from the same preprocessed token
/// stream as every other typed premise (so W4 derivation, W4 validation and
/// certificate rederivation reproduce them).
///
/// * `scalars`: declared non-pointer arithmetic or enumeration variables. Their
///   bindings carry only a branch fact (the null constant's "zero"), never a
///   custody claim, so a path on which one has no fact and a path on which it
///   has a target-free one join to no fact, and copying one with no fact gives
///   no fact.
/// * `pointers`: declared `SCALAR *`, `SCALAR [N]` and `struct T *` objects
///   other than the formal parameters.
///   Copying one that no reaching path bound gives a copy no path binds.
///
/// Both are empty when the declaration reader refused the function, which
/// restores every pre-card answer there.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TraversalNames {
    pub scalars: BTreeSet<String>,
    pub pointers: BTreeSet<String>,
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AliasPeers {
    pub place: String,
    pub peers: Vec<String>,
}

/// Source-derived indexed lookup call descriptor. The ordinary ContractCall
/// remains visible to every contract, condition and lifetime consumer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckedLookupCall {
    pub callee: String,
    pub parent: String,
    pub index: usize,
    /// Canonical complete defining record, shared by the actual and frame.
    pub record: String,
    /// Caller declarations rederived from the retained source, never sites.
    pub pointer_records: BTreeMap<String, String>,
}

/// A read point in the exact retained indexed lookup frame. The ordinary
/// IndirectScalarAccess and every original obligation remain intact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckedLookupRead {
    pub parent: String,
    pub cursor: String,
    pub guarded_cursor: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LegacyCfgNode {
    /// Exact retained assigned-call/callee frame, rederived during replay.
    /// Absence never supplies a child identity or membership fact.
    pub lookup: Option<CheckedLookupCall>,
    pub lookup_read: Option<CheckedLookupRead>,
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Card **SUP07-REFUSED-TEST-ACCUSATION** (rule 12e): `conditional_names`
    /// with C's reading of `&NAME->F`, the address of a member of the object
    /// `NAME` points to and never of `NAME` itself. A superset of
    /// `conditional_names`, read only by rule 12e's provenance
    /// (`refused_test::tie_facts`); every other reader keeps
    /// `conditional_names`. Derived from source.
    pub tie_names: BTreeSet<String>,
    /// Card **W43-INDEX-TRAVERSAL**: the declared scalar and single-level
    /// pointer names the traversal transfer rules key on. Derived from source.
    pub traversal: TraversalNames,
    /// Card **MK-EVENT-CELL-CROSS-FUNCTION** (rule 38c): the branch nodes
    /// whose condition is a declared cell protocol's guard test. Empty (and
    /// omitted) in every environment declaring no `quant cell` protocol.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cell_tests: BTreeMap<LegacyNodeId, descriptor_cells_route::ProtocolStep>,
    /// Card **CELL-PROTOCOL-RELEASE-WINDOW-FRAMES** (rule 38g): the standing
    /// in this unit of every declared frame of a callee this body calls.
    /// Empty (and omitted) in every environment declaring no frame.
    #[serde(
        default,
        skip_serializing_if = "cell_frames_route::CellFrames::is_empty"
    )]
    pub cell_frames: cell_frames_route::CellFrames,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisStatus {
    Safe,
    Unsafe,
    Unknown,
    /// Card W45-USER-CONTRACT-PLUGINS: not decided, awaiting a named human
    /// act — every unresolved diagnostic is a call held for a withheld
    /// contract ([`verdict`]). Never `Safe`; ordered below `Unknown`.
    Pending,
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
    /// Card W43-LIBC-RESULT-ALIAS: a pointer **into** the object acquired at
    /// `acquisition_line` — an interior pointer a declared derived result
    /// produced — was handed to a release. Freeing it is not freeing the
    /// object; the object's own obligation stays open. A violation
    /// *candidate*: the projection does not know the pointer's offset, and an
    /// interior pointer that happens to sit at offset zero would be a valid
    /// release.
    InvalidRelease {
        acquisition_line: usize,
    },
    /// Card W43-LIBC-RESULT-ALIAS: a pointer derived from the object acquired
    /// at `acquisition_line` was used after that object was released on some
    /// path reaching the use. The value of a pointer to an object whose
    /// lifetime has ended is indeterminate (C11 6.2.4p2). A violation
    /// candidate, reported only for a derived binding that resolves to one
    /// identity.
    UseAfterRelease {
        acquisition_line: usize,
    },
    /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the identity acquired at
    /// `acquisition_line`, of kind `acquired`, was handed to a release of
    /// another kind (`free(fopen(…))`, `fclose(malloc(…))`, `close(fp)`). The
    /// release discharges nothing. A violation *candidate*, published in the
    /// `invalid_release` category.
    KindMismatchedRelease {
        acquisition_line: usize,
        acquired: ResourceKind,
        released: ResourceKind,
    },
    /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: a non-heap identity (a
    /// descriptor, a stream, …) acquired at `acquisition_line` was used at a
    /// declared position after it was released on some path reaching the use
    /// (`read(fd, …)` after `close(fd)`). A violation *candidate*, published in
    /// the `use_after_release` category; the heap kind keeps its pre-existing
    /// refusal.
    UseAfterClose {
        acquisition_line: usize,
        kind: ResourceKind,
    },
    /// Card W45-FB-RETRY-LIFETIME: a counted-user obligation a path refutes —
    /// a declared exit clause contradicted on an exit that returns its value,
    /// an uncovered exit that has not restored a count, or a user dropped
    /// without a release check that reaches an exit with none re-added
    /// ([`counted_users`]). A violation *candidate*, published in the
    /// `counted_user` category.
    CountedUser {
        description: String,
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
    /// Card W45-FB-RETRY-LIFETIME: the verdict of every counted-user clause
    /// checked against this body. Empty for every function the pass leaves
    /// alone.
    pub counted_users: Vec<counted_users::ClauseRecord>,
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the refutations of this
    /// function's declaration by its own body. Empty for every undeclared
    /// function and every declaration its body does not refute.
    pub refutations: Vec<refuted_contract::BodyRefutation>,
    /// Card NULL-SAFETY-PROPERTY (gateway rule 14n): every dereference this
    /// analysis read non-null under decision 1's moved question (a
    /// target-free, possibly-null borrowed alias), each a named premise
    /// (`null-deref:FUNCTION:LINE:VARIABLE`) the verdict rests on until the
    /// null-dereference property proves it. Empty for every function that
    /// made no such reading.
    pub null_premises: Vec<crate::null_premise::NullPremise>,
    /// Exact outgoing registrations; Safe is conditional on their attributed
    /// protocol and every dependent caller's lifetime obligations.
    pub lifetime_effects: BTreeSet<crate::loop_lifetime_summary::ForwardedLifetime>,
    /// Card ARM-OBLIGATIONS: the obligations of the rule packages' admissions
    /// this row's unit analysis used ([`crate::arm_obligations`]): named
    /// premises among the row's assumptions, and open obligations, each of
    /// which also left an `Unsupported` diagnostic. Empty for every row whose
    /// unit used no admission with an obligation.
    pub rule_obligations: crate::arm_obligations::Carried,
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
    /// Transport the native observer's section layout flags unchanged.
    FunctionSections,
    DataSections,
    /// Exact bounded sanitizer selection. This affects `__has_feature`, so
    /// dropping it would analyze a different preprocessor configuration.
    AddressUndefinedSanitizers,
    /// `-O0` … `-Og`: transported because the level is visible to the
    /// preprocessor (see [`COptimizationLevel`]).
    Optimization(COptimizationLevel),
    // --- Target identity (card CG30-DIALECTS-FLAGS-TYPES, rule 11y) --------
    /// A structured `--target=` triple from the closed vocabulary, admitted
    /// only under the readers' LP64 premise. Serialised into the request
    /// digest and the compile-task identity, so a changed target is a
    /// different evidence identity.
    Target(crate::c_frontend_identity::CTargetTriple),
    /// A `-mabi=` name from the closed table, under the same admission.
    Abi(crate::c_frontend_identity::CAbiName),
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
            Self::FunctionSections => command.arg("-ffunction-sections"),
            Self::DataSections => command.arg("-fdata-sections"),
            Self::AddressUndefinedSanitizers => command.arg("-fsanitize=address,undefined"),
            Self::Optimization(level) => command.arg(level.spelling()),
            Self::Target(triple) => command.arg(format!("--target={}", triple.spelled())),
            Self::Abi(abi) => command.arg(format!("-mabi={}", abi.name())),
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
    /// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): explicit C17, C23 and C2y.
    /// Each carries its registered AST differences
    /// (`c_frontend_identity::dialect_differences`) into the evidence
    /// identity; none is adopted, so each is refused by name.
    C17,
    C23,
    C2y,
}

impl CCompilerDialect {
    pub(crate) fn clang_argument(self) -> &'static str {
        match self {
            Self::C11 => "-std=c11",
            Self::Gnu99 => "-std=gnu99",
            Self::Gnu11 => "-std=gnu11",
            Self::Gnu17 => "-std=gnu17",
            Self::C17 => "-std=c17",
            Self::C23 => "-std=c23",
            Self::C2y => "-std=c2y",
        }
    }

    /// The driver spelling accepted by the selected Clang release.
    ///
    /// The semantic identity remains [`Self::clang_argument`]: `c2x` is only
    /// Clang 17's spelling for the finalized C23 mode. C2y has no equivalent
    /// older spelling and therefore fails closed on a compiler that lacks it.
    pub(crate) fn clang_invocation_argument(self) -> Result<&'static str, String> {
        let version = crate::clang::detected_version()?;
        match self {
            Self::C23 => Ok(version.c23_argument()),
            Self::C2y if !version.supports_c2y() => Err(format!(
                "Clang {}.{}.{} does not support the C2y frontend mode required by this unit",
                version.major, version.minor, version.patch
            )),
            _ => Ok(self.clang_argument()),
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
    /// Home bodies and explicit link-map identities for imported W4 summaries.
    /// Replay reconstructs their contracts before reanalysing this caller.
    pub cross_unit_homes: Vec<CrossUnitHomeEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
// The certificate deliberately carries its complete replay and scope evidence
// by value in the stable public result API. Boxing only this variant would add
// an unrelated compatibility break to a certificate-containment change.
#[allow(clippy::large_enum_variant)]
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
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the four baseline dialects
    // keep their one historical byte; C17, C23 and C2y add theirs and, with
    // registered differences, the registry.
    request_bytes.extend(crate::c_frontend_identity::dialect_identity_bytes(dialect));
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
            let line_map = location::map_marked_preprocessing(marked, preprocessed_source);
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

/// Derive the caller-held target context from the exact retained preparation
/// products used by an audit run. This is the prepared-input counterpart of
/// [`legacy_certificate_expected_context`]: it starts from the caller's
/// prepared request and evidence, never from a presented certificate, and does
/// not invoke Clang a second time.
pub fn legacy_certificate_expected_context_from_prepared(
    prepared: &PreparedTranslationUnit,
) -> Result<LegacyCertificateExpectedContext, LegacyCError> {
    let preprocessed_source = prepared
        .preprocessed_source()
        .map_err(Clone::clone)?
        .to_owned();
    let request = LegacyCertificateRequest {
        include_paths: prepared.include_paths().to_vec(),
        definitions: prepared.definitions().to_vec(),
        preprocessor_options: prepared.preprocessor_options().to_vec(),
        input: prepared.input().clone(),
    };
    let scope = build_certificate_scope_from_evidence(
        prepared.dialect(),
        prepared.function_scope(),
        &preprocessed_source,
        Some(prepared.raw_source()),
        prepared.line_marked_source(),
        request,
    )
    .map_err(|reason| {
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
    let _typed_path_source =
        crate::typed_access_path::enter_source(source, CCompilerDialect::C11, None);
    let tokens = tokenize(source);
    analyze_legacy_c_tokens(&tokens, function, contracts)
}

fn analyze_legacy_c_tokens(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let summary = analyze_legacy_c_tokens_under_holds(tokens, function, contracts)?;
    // Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: the same settlement as
    // the translation-unit route, over this entry's one row.
    let mut rows = BTreeMap::from([(summary.function.clone(), summary)]);
    review_standing::settle_with(&mut rows, contracts, |resolved| {
        analyze_legacy_c_tokens_under_holds(tokens, function, resolved)
            .map(|resolved| BTreeMap::from([(resolved.function.clone(), resolved)]))
    });
    Ok(rows
        .into_values()
        .next()
        .expect("the entry's one row is kept"))
}

fn analyze_legacy_c_tokens_under_holds(
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
    // Card W45-PARENTHESISED-CALLEE: `(f)(x)` is `f(x)` before anything,
    // the hold included, reads a call.
    let normalised = parenthesised_callee::normalise_tokens(tokens);
    let tokens = normalised.as_deref().unwrap_or(tokens);
    // Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p).
    let rewritten = indirect_call_route::rewrite_tokens(tokens, contracts);
    let tokens = rewritten.as_deref().unwrap_or(tokens);
    // Card W45-SR5-SOUNDNESS: a reserved prefix the source spells is escaped
    // before any rewrite mints one.
    let escaped = reserved_spelling::escape(tokens);
    let tokens = escaped.as_deref().unwrap_or(tokens);
    // Card W45-CONTRACT-DEFERRED-REVIEW: held before anything reads a call.
    let held_tokens = review_hold::hold_call_sites(tokens, contracts);
    let tokens = held_tokens.as_deref().unwrap_or(tokens);
    // Card W45-CONTRACT-REVISION-MODE: likewise for a pin mismatch.
    let pin_held_tokens = pin_hold::hold_call_sites(tokens, contracts);
    let tokens = pin_held_tokens.as_deref().unwrap_or(tokens);
    // Card W48-RK-LOG-CALLER-FRAME (rule 32a): a caller frame's own calls.
    let frame_held_tokens = caller_frame_hold::hold_call_sites(tokens, contracts);
    let tokens = frame_held_tokens.as_deref().unwrap_or(tokens);
    // Card CONTRACT-SUPERSESSION: a function whose bundles conflict.
    let conflict_held_tokens = conflict_hold::hold_call_sites(tokens, contracts);
    let tokens = conflict_held_tokens.as_deref().unwrap_or(tokens);
    let agreement = declared_abi::AbiAgreement::read(tokens, contracts);
    let withdrawn_environment = agreement.withdraw_environment(contracts);
    let mut settled_environment = withdrawn_environment.map_or(
        std::borrow::Cow::Borrowed(contracts),
        std::borrow::Cow::Owned,
    );
    let withdrawn_tokens = agreement.withdraw_call_sites(tokens);
    let tokens = withdrawn_tokens.as_deref().unwrap_or(tokens);
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12w): a guarded retention whose
    // body this unit defines stands only when the body validates it.
    if let Some(next) = retention_guard::settle(tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 13a): a slot or borrowed-result
    // declaration whose body this unit defines stands only when it validates.
    if let Some(next) = slot_move_route::settle(tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card REFCOUNT-RESOURCE-KIND (rule 13c): a reference declaration whose
    // body this unit defines stands only when the body validates it.
    if let Some(next) = detach_route::settle(tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    if let Some(next) = reference_route::settle(tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13y): a callback declaration
    // whose body this unit defines stands only when the body validates it.
    if let Some(next) = callback_route::settle(tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    let contracts = settled_environment.as_ref();
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

/// The unread-definition gate's refusal (unit **E01-TAIL**): the selected
/// source definitions that carry no analysed row, if there are any. Shared by
/// the `Safe` arm and, since card W45-SR5-SOUNDNESS, the `Pending` arm.
fn unread_selected_definitions(
    selected: Option<&[String]>,
    summary: &LegacyProgramSummary,
) -> Option<String> {
    let selected = selected?;
    let unread = selected
        .iter()
        .filter(|name| !summary.functions.contains_key(*name))
        .collect::<Vec<_>>();
    let first = unread.first()?;
    Some(format!(
        "{} of {} selected source definition(s) carry no analysed row, so no whole-unit Safe claim is published over them; first: `{first}`",
        unread.len(),
        selected.len(),
    ))
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
    analyze_unrestricted_c11_with_inventory_and_cross_unit(unit, entry, contracts, inventory, &[])
}

/// Inventory-preserving analysis with opt-in, replayable cross-unit homes.
pub fn analyze_unrestricted_c11_with_inventory_and_cross_unit(
    unit: &C11TranslationUnit<'_>,
    entry: &str,
    contracts: &ResourceContracts,
    inventory: Option<&SourceDefinitionInventory>,
    homes: &[CrossUnitHomeEvidence],
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
    #[cfg(feature = "indirect-census")]
    crate::access_path_census::record_unit(
        &preprocessed_source,
        unit.dialect.clang_argument(),
        &census_frontend_flags(&unit.preprocessor_options),
    );
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
        homes,
    )
}

/// Analyze one unit from the immutable preprocessing, syntax, inventory and
/// coordinate preparation already shared by the audit driver.
pub fn analyze_prepared_translation_unit(
    prepared: &PreparedTranslationUnit,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    analyze_prepared_translation_unit_with_cross_unit(prepared, entry, contracts, &[])
}

/// Analyse a prepared caller with opt-in, independently replayable home
/// evidence. The imported contracts must already be in `contracts`' separate
/// inferred-import map and must match the evidence exactly.
pub fn analyze_prepared_translation_unit_with_cross_unit(
    prepared: &PreparedTranslationUnit,
    entry: &str,
    contracts: &ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
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
    #[cfg(feature = "indirect-census")]
    crate::access_path_census::record_unit(
        &preprocessed_source,
        prepared.dialect().clang_argument(),
        &census_frontend_flags(prepared.preprocessor_options()),
    );
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
        homes,
    )
}

/// Card W46-ACCESS-PATH-CENSUS: the frontend flags
/// the typed-AST transport appends ([`C11TranslationUnit::append_frontend_flags`]),
/// as argument strings.
#[cfg(feature = "indirect-census")]
fn census_frontend_flags(options: &[CPreprocessorOption]) -> Vec<String> {
    let mut command = std::process::Command::new("clang");
    for option in options {
        if let CPreprocessorOption::FrontendFlag(flag) = option {
            flag.append_to(&mut command);
        }
    }
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

// This private boundary spells out the exact retained PreparedTranslationUnit
// evidence used by certificate issuance. Keeping the fields explicit avoids a
// second, subtly different snapshot type at the trust boundary.
#[allow(clippy::too_many_arguments)]
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
    homes: &[CrossUnitHomeEvidence],
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    crate::contract_dependencies::primary_source(&preprocessed_source);
    cross_unit::check_imports(&preprocessed_source, contracts, homes).map_err(|reason| {
        LegacyCError::ProductionParserRejected(format!("cross-unit import refused: {reason}"))
    })?;
    let analysis = match analyze_legacy_c_translation_unit_selected_partial(
        &preprocessed_source,
        entry,
        contracts,
        source_functions.as_deref(),
        (dialect, Some(&request)),
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
        // Card W45-USER-CONTRACT-PLUGINS: a `Pending` unit is published
        // through the same variant; `UnrestrictedLegacyResult::verdict` reads
        // it back from the summary's status.
        //
        // Card W45-SR5-SOUNDNESS (`PENDING-BYPASSES-UNIT-SAFE-GATES`): a
        // `Pending` unit says only a named human act stands between it and
        // `Safe`, so it passes the `Safe` arm's scope gates as well: a unit
        // either gate withholds from `Safe` is `Unknown`, whatever it holds
        // (spec §L21.1). Before, an unread selected definition beside a held
        // call published `Pending` where the same unit without the hold was
        // `Unknown`.
        AnalysisStatus::Pending => {
            let mut summary = summary;
            let reason = unresolved_projection_reason(&summary);
            let withheld = unread_selected_definitions(source_functions.as_deref(), &summary)
                .or_else(|| {
                    build_certificate_scope_from_evidence(
                        dialect,
                        function_scope,
                        &preprocessed_source,
                        Some(&raw_source),
                        line_marked_source.as_deref(),
                        request,
                    )
                    .err()
                    .map(|scope| {
                        format!("certificate target scope could not be established: {scope}")
                    })
                });
            if let Some(withheld) = withheld {
                summary.status = AnalysisStatus::Unknown;
                return Ok(UnrestrictedLegacyResult::Unknown {
                    preprocessed_source,
                    reason: format!("{reason}; {withheld}"),
                    partial: Some(summary),
                });
            }
            Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source,
                reason,
                partial: Some(summary),
            })
        }
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
            if let Some(reason) = unread_selected_definitions(source_functions.as_deref(), &summary)
            {
                return Ok(UnrestrictedLegacyResult::Unknown {
                    preprocessed_source,
                    reason,
                    partial: Some(summary),
                });
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
                cross_unit_homes: homes.to_vec(),
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
    // Card W45-CONTRACT-DEFERRED-REVIEW: a held call is not an unsupported
    // operation of the source, so it is counted apart and named only when
    // present; every summary without one is spelled exactly as before.
    let mut held = 0;
    // Card W45-CONTRACT-REVISION-MODE: calls held for a pin mismatch, counted
    // and named in the same way, after the review component.
    let mut pin_held = 0;
    for diagnostic in summary
        .functions
        .values()
        .flat_map(|function| &function.diagnostics)
    {
        match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description }
                if review_hold::held_call(description).is_some() =>
            {
                held += 1;
            }
            LegacyDiagnosticKind::Unsupported { description }
                if pin_hold::held_call(description).is_some() =>
            {
                pin_held += 1;
            }
            LegacyDiagnosticKind::Unsupported { .. } => unsupported += 1,
            LegacyDiagnosticKind::UnknownAlias { .. } => aliases += 1,
            LegacyDiagnosticKind::SummaryConflict { .. } => conflicts += 1,
            _ => {}
        }
    }
    let mut reason = format!(
        "resource projection unresolved: {unsupported} unsupported operation(s), \
         {aliases} unresolved alias(es), {conflicts} custody summary conflict(s)"
    );
    if held > 0 {
        reason.push_str(&format!(", {held} call(s) held awaiting human review"));
    }
    if pin_held > 0 {
        reason.push_str(&format!(
            ", {pin_held} call(s) held for a contract pin mismatch"
        ));
    }
    reason
}

/// Preprocess and syntax-check an unrestricted C translation unit without
/// requiring it to contain an executable function body.
pub fn validate_unrestricted_c11(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    preparation::validate_plain_and_syntax(unit).map(|(source, _, _)| source)
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (rule 12n, F1): the unit's plain
/// `-E -P` preprocessing alone, without the syntax check, for the
/// member-facts pre-pass.
///
/// # Errors
/// Clang's refusal or absence.
pub fn preprocess_unrestricted_c11(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    preparation::preprocess_plain_only(unit)
}

/// Independent structural check for an unrestricted-C `Safe` result.
pub fn check_legacy_safety_certificate(
    certificate: &LegacySafetyCertificate,
) -> Result<(), String> {
    cross_unit::check_imports(
        &certificate.preprocessed_source,
        &certificate.contracts,
        &certificate.cross_unit_homes,
    )?;
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
        (certificate.dialect, Some(&certificate.target_scope.request)),
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
    // The complete independently reconstructed record has now matched. Its
    // CFGs have no remaining consumer; later checks read the presented record.
    drop(recomputed);
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
        (CCompilerDialect::C11, None),
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
    // Card W45-PARENTHESISED-CALLEE: the same stream the analysis reads.
    let (tokens, conditionals) = parenthesised_callee::normalise_owned(tokens, conditionals);
    // Card W45-SR5-SOUNDNESS: the same escape the analysis applies.
    let tokens = reserved_spelling::escape_owned(tokens);
    // Card W45-CONTRACT-DEFERRED-REVIEW: held before anything reads a call.
    let tokens = review_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card W45-CONTRACT-REVISION-MODE: likewise for a pin mismatch.
    let tokens = pin_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card W48-RK-LOG-CALLER-FRAME (rule 32a): a caller frame's own calls.
    let tokens = caller_frame_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card CONTRACT-SUPERSESSION: a function whose bundles conflict.
    let tokens = conflict_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    let agreement = declared_abi::AbiAgreement::read(&tokens, contracts);
    let withdrawn_environment = agreement.withdraw_environment(contracts);
    let mut settled_environment = withdrawn_environment.map_or(
        std::borrow::Cow::Borrowed(contracts),
        std::borrow::Cow::Owned,
    );
    let withdrawn_tokens = agreement.withdraw_call_sites(&tokens);
    let tokens = withdrawn_tokens.unwrap_or(tokens);
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12w): a guarded retention whose
    // body this unit defines stands only when the body validates it.
    if let Some(next) = retention_guard::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 13a).
    if let Some(next) = slot_move_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card REFCOUNT-RESOURCE-KIND (rule 13c): a reference declaration whose
    // body this unit defines stands only when the body validates it.
    if let Some(next) = detach_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    if let Some(next) = reference_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13y): a callback declaration
    // whose body this unit defines stands only when the body validates it.
    if let Some(next) = callback_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    let contracts = settled_environment.as_ref();
    let mut names = function_names(&tokens, &conditionals).definitions;
    // Card W44-CORPUS-CALLEE-SUMMARIES: the same derivation scope as the analysis.
    let definitions = names.clone();
    if let Some(selected) = selected_functions {
        let selected = selected.iter().collect::<BTreeSet<_>>();
        names.retain(|name| selected.contains(name));
    }
    let names =
        callee_scope::derivation_scope(&tokens, &names, &definitions, selected_functions.is_some());
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

// Card W44-CORPUS-CALLEE-SUMMARIES: which definitions W4 derives summaries for.
#[path = "legacy_callee_scope.rs"]
pub(crate) mod callee_scope;

#[path = "legacy_cross_unit.rs"]
mod cross_unit;

pub use cross_unit::{
    CrossUnitExport, CrossUnitHomeEvidence, export_homes as export_cross_unit_homes,
    for_caller as cross_unit_for_caller,
};

// Card W45-FB-RETRY-LIFETIME: intrusive-container publication.
#[path = "legacy_container_publication.rs"]
pub(crate) mod container_publication;

// Card W45-FB-RETRY-LIFETIME: counted-user clauses.
#[path = "legacy_counted_users.rs"]
pub mod counted_users;

/// Recollect local reader observations with independent caller-held inputs.
/// This exposes no certificate and does not require the whole CFG to succeed.
pub(crate) fn replay_reading_observations(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected: Option<&[String]>,
    dialect: CCompilerDialect,
) -> Result<(), LegacyCError> {
    analyze_legacy_c_translation_unit_selected_partial(
        source,
        entry,
        contracts,
        selected,
        (dialect, None),
    )
    .map(|_| ())
}

fn analyze_legacy_c_translation_unit_selected_partial(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
    typed_context: (CCompilerDialect, Option<&LegacyCertificateRequest>),
) -> Result<PartialTranslationUnitAnalysis, LegacyCError> {
    crate::contract_dependencies::primary_source(source);
    let mut analysis = analyze_translation_unit_under_holds(
        source,
        entry,
        contracts,
        selected_functions,
        typed_context,
    )?;
    // Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: a `Pending` row stands
    // only if its function carries no other reason with every held
    // declaration in force; a unit without a `Pending` row is untouched and
    // analysed once.
    let moved =
        review_standing::settle_with(&mut analysis.summary.functions, contracts, |resolved| {
            analyze_translation_unit_under_holds(
                source,
                entry,
                resolved,
                selected_functions,
                typed_context,
            )
            .map(|resolved| resolved.summary.functions)
        });
    if moved {
        analysis.summary.status = verdict::unit_status(
            analysis.summary.functions.values(),
            !analysis.projection_failures.is_empty(),
        );
    }
    Ok(analysis)
}

fn analyze_translation_unit_under_holds(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
    typed_context: (CCompilerDialect, Option<&LegacyCertificateRequest>),
) -> Result<PartialTranslationUnitAnalysis, LegacyCError> {
    let (dialect, request) = typed_context;
    // Card ARM-OBLIGATIONS: every rule package admission this analysis uses
    // carries its obligations here, and every row below carries them.
    let obligations = crate::arm_obligations::Frame::open();
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g): the environment the unit
    // is analysed under, before any withdrawal, for the body evidence.
    let input = contracts;
    let _typed_path_source = crate::typed_access_path::enter_source(source, dialect, request);
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    // Card W45-PARENTHESISED-CALLEE: every body's `(f)(x)` is rewritten to
    // `f(x)` first, so the hold, both withdrawals, W4 and every classifier
    // position below read the call C evaluates.
    let (tokens, conditionals) = parenthesised_callee::normalise_owned(tokens, conditionals);
    // Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p): a closed candidate
    // set is the calls C may make, before any hold or withdrawal reads one.
    let (tokens, conditionals) =
        indirect_call_route::rewrite_owned(tokens, conditionals, contracts);
    // Card W45-SR5-SOUNDNESS: every identifier the source spells with a prefix
    // a rewrite below reserves is escaped first, so every marked name any
    // reader sees was minted from the environment (`reserved_spelling`).
    let tokens = reserved_spelling::escape_owned(tokens);
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): a registered, unadopted
    // difference of the dialect withdraws its function's body before any
    // reader, the derivation or replay reads it; a digit separator, which this
    // token reader cannot lex, refuses the unit.
    let (tokens, dialect_refusals) =
        dialect_hook::refuse(dialect, request, source, tokens, &conditionals)?;
    // Card W45-CONTRACT-DEFERRED-REVIEW: a declaration awaiting human review
    // is already absent from the environment (the loader withholds it); its
    // call sites are rewritten here, before the ABI gate and everything else
    // reads a call, so every consumer of the stream sees the hold.
    let tokens = review_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card W45-CONTRACT-REVISION-MODE: likewise for a pin mismatch.
    let tokens = pin_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card W48-RK-LOG-CALLER-FRAME (rule 32a): a caller frame's own calls.
    let tokens = caller_frame_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card CONTRACT-SUPERSESSION: a function whose bundles conflict.
    let tokens = conflict_hold::hold_call_sites(&tokens, contracts).unwrap_or(tokens);
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the audit's refutation table, read
    // here because the ABI gate below rebuilds the environment without it.
    let refuted_table = contracts
        .refuted_by_body()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
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
    if let Some(after) = &withdrawn_environment {
        crate::contract_dependencies::withdrawn_between(input, after, "abi", true);
    }
    let mut settled_environment = withdrawn_environment.map_or(
        std::borrow::Cow::Borrowed(contracts),
        std::borrow::Cow::Owned,
    );
    let withdrawn_tokens = agreement.withdraw_call_sites(&tokens);
    let tokens = withdrawn_tokens.unwrap_or(tokens);
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12w): a guarded retention whose
    // body this unit defines stands only when the body validates it.
    if let Some(next) = retention_guard::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 13a).
    if let Some(next) = slot_move_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card REFCOUNT-RESOURCE-KIND (rule 13c): a reference declaration whose
    // body this unit defines stands only when the body validates it.
    if let Some(next) = detach_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    if let Some(next) = reference_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    // Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13y): a callback declaration
    // whose body this unit defines stands only when the body validates it.
    if let Some(next) = callback_route::settle(&tokens, &settled_environment) {
        settled_environment = std::borrow::Cow::Owned(next);
    }
    let contracts = settled_environment.as_ref();
    let mut names = function_names(&tokens, &conditionals).definitions;
    // Card W44-CORPUS-CALLEE-SUMMARIES: every definition, kept before the
    // selection narrows `names`, for the derivation scope below.
    let definitions = names.clone();
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
    // Card W44-CORPUS-CALLEE-SUMMARIES census (feature-gated, verdict-neutral).
    #[cfg(feature = "indirect-census")]
    if selected_functions.is_some() {
        callee_scope::census::record(&tokens, &names, &definitions, contracts);
    }
    // Card W44-CORPUS-CALLEE-SUMMARIES: W4 derives over the selection plus the
    // unselected internal-linkage definitions it reaches (`callee_scope`); the
    // rows analysed and reported below stay `names` alone.
    let scope =
        callee_scope::derivation_scope(&tokens, &names, &definitions, selected_functions.is_some());
    // Card W45-FB-BACKLOG-CUSTODY: a checked specification stays in the
    // environment the gate derives over only while its body establishes it.
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: then every call site of a refuted
    // declaration is held, and the unit is settled over the held stream.
    let checked_spec::Settled {
        gate,
        withdrawn: withdrawn_specifications,
        carried: specification_obligations,
    } = refuted_contract::settle(&tokens, &scope, contracts, &refuted_table);
    // Card ARM-OBLIGATIONS: the standing specifications' admissions, each
    // attributed to its specification (card ARM-OBLIGATIONS-UNIT-GRAIN).
    obligations.carry_declarations(specification_obligations);
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g): what the final
    // derivation's bodies made of the declarations they define.
    let mode_withdrawn = gate
        .agreement
        .withdrawn_names()
        .cloned()
        .collect::<Vec<_>>();
    let tokens = gate.tokens.unwrap_or(tokens);
    let mut derivation = gate.derivation;
    // The settled caller environment has handed all later call-site duties to
    // the derivation. Retain the original input for body evidence, not another
    // obsolete owned copy of its settled environment.
    drop(settled_environment);
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): extension types are refused
    // by name, read from each definition's typed AST where the unit spells one.
    let extensions = dialect_hook::Extensions::of(&tokens);
    // Card DECLARED-ASSUMPTION-BODY-CHECKS (rules 13m, 13n): evidence only.
    let declared_entry = declared_entry_check::check(
        &declared_entry_check::Unit {
            input,
            scope: &scope,
            tokens: &tokens,
            derivation: &derivation,
            mode: &mode_withdrawn,
            specifications: &withdrawn_specifications,
        },
        |summary, name| {
            dialect_hook::name_refusals(summary, &tokens, name, &dialect_refusals, &extensions);
        },
    );
    crate::contract_dependencies::unit_evidence(
        input,
        &scope,
        &derivation,
        &mode_withdrawn,
        &withdrawn_specifications,
        &declared_entry,
    );
    let mut functions = BTreeMap::new();
    let mut projection_failures = Vec::new();
    // Global declared-body and dependency evidence above are the final readers
    // of unselected contextual bodies. Callee summaries remain in force.
    derivation.retain_contextual_cfgs(&names);
    for name in names {
        match analyze_defined_function(&tokens, &name, &mut derivation) {
            Ok(mut summary) => {
                // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y).
                dialect_hook::name_refusals(
                    &mut summary,
                    &tokens,
                    &name,
                    &dialect_refusals,
                    &extensions,
                );
                // Card W45-FB-BACKLOG-CUSTODY: a withdrawn specification keeps
                // its defining function out of `Safe`.
                if let Some(description) =
                    checked_spec::diagnostic(&withdrawn_specifications, &name)
                {
                    let line = find_function_body(&tokens, &name).map_or(0, |(_, line)| line);
                    summary.diagnostics.push(LegacyDiagnostic {
                        function: name.clone(),
                        line,
                        kind: LegacyDiagnosticKind::Unsupported { description },
                    });
                    summary.diagnostics.sort();
                    summary.diagnostics.dedup();
                    summary.status = classify_diagnostics(&summary.diagnostics);
                }
                // Card W45-REFUTED-CONTRACT-WITHDRAWAL: what the row records
                // against its own declaration, for the audit's table.
                summary.refutations =
                    refuted_contract::refutations(&summary, derivation.environment());
                // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g).
                if !summary.refutations.is_empty() {
                    crate::contract_dependencies::failed(&name, "C06.4-own");
                }
                functions.insert(name, summary);
            }
            Err(error @ LegacyCError::MalformedFunction { .. }) => {
                projection_failures.push(error);
            }
            Err(error) => return Err(error),
        }
    }
    // Card CMT-GETTER-OWNERSHIP-CLOSURE: source-validated conditional
    // embedded storage. This can only append a witnessed violation to the
    // public root; every ordinary diagnostic remains evidence.
    legacy_conditional_embedded_storage::diagnose(&tokens, &mut functions);
    // Card ARM-OBLIGATIONS: every row carries what the unit's admissions it
    // rests on rest on: premises into its assumptions, an open obligation out
    // of `Safe`. Card ARM-OBLIGATIONS-UNIT-GRAIN: an admission attributed to
    // a declaration only on the rows whose derivation reaches it, every
    // other admission on every row.
    let (carried, attribution) = obligations.close();
    if !carried.is_empty() {
        let reaching =
            crate::arm_obligations::reaching_rows(&carried, &attribution, &functions, input);
        for (name, summary) in &mut functions {
            let line = find_function_body(&tokens, name).map_or(0, |(_, line)| line);
            let row = carried.row(|declaration| {
                reaching
                    .get(declaration)
                    .is_none_or(|rows| rows.contains(name))
            });
            crate::arm_obligations::apply(summary, &row, line);
        }
    }
    // Card W45-USER-CONTRACT-PLUGINS: the fold of the verdict semigroup,
    // which is the three-way decision this replaced on every status it had.
    let status = verdict::unit_status(functions.values(), !projection_failures.is_empty());
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
    derivation: &mut CustodyDerivation,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    // Move the final contextual body into its consumer instead of cloning it
    // and retaining the original until every selected row has finished.
    let contextual = derivation.take_contextual_cfg(function);
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f).
    crate::contract_dependencies::within(function, || {
        analyze_defined_function_unscoped(tokens, function, derivation, contextual)
    })
}

fn analyze_defined_function_unscoped(
    tokens: &[Token],
    function: &str,
    derivation: &CustodyDerivation,
    contextual: Option<LegacyCfg>,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let has_contextual = contextual.is_some();
    let cfg = match contextual {
        Some(cfg) => cfg,
        None => build_function_cfg(tokens, function, derivation.environment())?,
    };
    let double_releases = member_release::callee_double_releases(&cfg);
    let mut summary = match derivation.entry_contract(function) {
        Some(contract) => match function_parameters(tokens, function) {
            Some(parameters) => {
                // Card W43-RESULT-ALIAS-RETURNED: a derived result is checked on
                // the CFG that desugars a derived `return f(…)`; a body that does
                // not validate falls back to the ordinary CFG's contract-free
                // analysis, exactly as any other failed entry does.
                let (entry_cfg, fallback) =
                    if matches!(contract.result, ResourceReturnMode::Derived { .. })
                        && !has_contextual
                    {
                        (
                            build_derived_return_cfg(tokens, function, derivation.environment())
                                .ok(),
                            Some(cfg),
                        )
                    } else {
                        (Some(cfg), None)
                    };
                (|| match entry_cfg {
                    Some(entry_cfg) => {
                        let Some(entry) = EntryCustody::build(&entry_cfg, &parameters, contract)
                        else {
                            return analyze_cfg(fallback.unwrap_or(entry_cfg));
                        };
                        let (candidate, observations) = analyze_cfg_with_entry(entry_cfg, &entry);
                        if crate::legacy_summary::observations_match_contract(
                            contract,
                            &entry,
                            &candidate,
                            &observations,
                        ) {
                            // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g):
                            // the body, under its contract's entry custody,
                            // matches every exit and is fully modelled.
                            if candidate.status == AnalysisStatus::Safe
                                && candidate.diagnostics.is_empty()
                                && derivation.conflict(function).is_none()
                                && *contract == contract.custody_projection()
                            {
                                crate::contract_dependencies::validated(function, "entry");
                            }
                            candidate
                        } else {
                            analyze_cfg(fallback.unwrap_or(candidate.cfg))
                        }
                    }
                    None => analyze_cfg(fallback.expect("derived CFG retains ordinary fallback")),
                })()
            }
            None => analyze_cfg(cfg),
        },
        // Card DETACH-RESULT-WRAPPER-COMPOSITION (rule 33): a validated
        // selection frame is decided under its selection entry; a run that
        // does not validate falls back to the contract-free analysis.
        None if derivation.self_init_frame(function) => {
            self_link_init::row(tokens, function, derivation.environment(), cfg.clone())
                .unwrap_or_else(|| analyze_cfg(cfg))
        }
        None if let Some(frame) = derivation.selection(function) => {
            match function_parameters(tokens, function)
                .and_then(|parameters| selection_route::row(cfg.clone(), &parameters, frame))
            {
                Some(summary) => summary,
                None => analyze_cfg(cfg),
            }
        }
        // Card CALLEE-RETENTION-CLASS1 (rule 12s(2)): a function whose own
        // return-code disposition is in force is decided per class.
        None => derivation
            .environment()
            .relational_contract(function)
            .and_then(|contract| {
                crate::legacy_summary::relational_forwarding::relational_row(
                    tokens,
                    function,
                    derivation.environment(),
                    contract,
                )
            })
            .unwrap_or_else(|| analyze_cfg(cfg)),
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
    // Card W45-FB-RETRY-LIFETIME: the counted-user clauses, checked against
    // this body after its custody analysis. The identity in every environment
    // that declares none.
    counted_users::check(tokens, function, derivation.environment(), &mut summary);
    // Card W45-SR5-SOUNDNESS: a member release a later callee repeats is a
    // double release candidate, beside the refusal that withheld it.
    if !double_releases.is_empty() {
        summary.diagnostics.extend(double_releases);
        summary.diagnostics.sort();
        summary.diagnostics.dedup();
        summary.status = classify_diagnostics(&summary.diagnostics);
    }
    Ok(summary)
}

/// Parse one function body into its resource CFG under `contracts`.
pub(crate) fn build_function_cfg(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyCfg, LegacyCError> {
    build_function_cfg_with(tokens, function, contracts, false, None)
}

/// Card **W43-RESULT-ALIAS-RETURNED**: does the body name a callee whose contract in force has a derived result,
/// and does it `return` anything? A cheap token pre-filter, so the derived CFG
/// is built only for bodies that could possibly qualify.
pub(crate) fn body_calls_derived_result(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> bool {
    let Some((start, _)) = find_function_body(tokens, function) else {
        return false;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return false;
    };
    let body = &tokens[start + 1..end];
    let derived_call = body.windows(2).any(|pair| {
        pair[1].text == "("
            && environment.contract(&pair[0].text).is_some_and(|contract| {
                matches!(contract.result, ResourceReturnMode::Derived { .. })
            })
    });
    ((derived_call || lookup_route::read(tokens, function).is_some())
        && body.iter().any(|token| token.text == "return"))
        || exact_formal_return::candidate(tokens, function)
}

/// Card **W43-RESULT-ALIAS-RETURNED**: the CFG a derived-result summary is
/// inferred and validated on. It differs from [`build_function_cfg`]'s only at
/// `return` statements: a whole-call return of a derived result is desugared
/// like any admitted return call, and a null-constant return names its exit.
/// Nothing else reads it, so every other function's CFG is unchanged.
pub(crate) fn build_derived_return_cfg(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyCfg, LegacyCError> {
    build_function_cfg_with(tokens, function, contracts, true, None)
}

/// Card **W47-SUMMARY-VALIDATION-PROTECTED-FORMALS**: the CFG the reported
/// run builds for `function` once `own` is its contract in force, for a W4
/// candidate the environment does not yet hold. `own` is read exactly where
/// the function's own contract is read — the protected formals and the
/// descriptor names — and nowhere else: a call to `function` inside its own
/// body still resolves through `contracts` alone, so no recursive
/// assumption is introduced (rule 4). A declaration in `contracts` wins.
pub(crate) fn build_function_cfg_under_own_contract(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    own: &ResourceFunctionContract,
    admit_derived_returns: bool,
) -> Result<LegacyCfg, LegacyCError> {
    build_function_cfg_with(
        tokens,
        function,
        contracts,
        admit_derived_returns,
        Some(own),
    )
}

fn build_function_cfg_with(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    admit_derived_returns: bool,
    own: Option<&ResourceFunctionContract>,
) -> Result<LegacyCfg, LegacyCError> {
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f): every lookup this build
    // makes is `function`'s consultation; the identity without a recording.
    let result = crate::contract_dependencies::within(function, || {
        build_function_cfg_unscoped(tokens, function, contracts, admit_derived_returns, own)
    });
    crate::audit_ir_capture::record_build(
        function,
        tokens,
        contracts,
        admit_derived_returns,
        own,
        &result,
    );
    result
}

fn build_function_cfg_unscoped(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    admit_derived_returns: bool,
    own: Option<&ResourceFunctionContract>,
) -> Result<LegacyCfg, LegacyCError> {
    // Card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS: the contract in force for
    // this function's own body, which its declaration decides when it has one.
    // Its non-Borrowed modes also fix formal slots against the read-only
    // formal-step exception (W48 bottom-up batch 2).
    let own_contract = contracts.contract(function).or(own);
    // Card RELATIONAL-REWRITE-LINE-ORDER: parity runs only.
    let census_stage = |stage: &str, before: &[Token], after: Option<&Vec<Token>>| {
        if let Some(after) = after {
            stream_line_order::record_stage(function, stage, before, after);
        }
    };
    if crate::typed_access_path::parity_enabled()
        && stream_line_order::first_disorder(tokens).is_some()
    {
        stream_line_order::record_stage(function, "input", tokens, tokens);
    }
    // Card W44-C11-GENERIC-SELECTION: lower every selection-invariant
    // `_Generic` in this body to its common expression first, so every reader
    // below (the literal premises included) sees the call C evaluates. Only
    // this body's tokens change; a refused selection is untouched.
    let resolved = generic_selection::resolve_in_body(tokens, function);
    census_stage("generic-selection", tokens, resolved.as_ref());
    let tokens = resolved.as_deref().unwrap_or(tokens);
    // Card DETACH-RESULT-WRAPPER-COMPOSITION (rule 33): a checked selection
    // frame's own body, and its decimal-constant caller statements, read as
    // their constituent calls. The identity on every other body.
    let expanded = selection_route::expand(tokens, function, contracts);
    census_stage("removed-selection", tokens, expanded.as_ref());
    let tokens = expanded.as_deref().unwrap_or(tokens);
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
    census_stage("literal-premise", tokens, restricted.as_ref());
    let tokens = restricted.as_deref().unwrap_or(tokens);
    // Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): a stored return value
    // tested by the next statement is the same C as the test of the
    // assignment, and the parser reads every class of a return-code
    // disposition under its minted name. Both are the identity on an
    // environment with no disposition.
    let stored_returns = relational_route::tested_stored_returns(tokens, contracts);
    census_stage("relational-11r", tokens, stored_returns.as_ref());
    let tokens = stored_returns.as_deref().unwrap_or(tokens);
    // Card HANDOFF-DIRECT-CALL-READING (rule 12m amended): the same rewrite
    // for a direct call of a hand-off declaration (the identity on an
    // environment without one).
    let handoff_returns = handoff_direct_route::stored_returns(tokens, contracts);
    census_stage("handoff-direct", tokens, handoff_returns.as_ref());
    let tokens = handoff_returns.as_deref().unwrap_or(tokens);
    let class_environment = relational_route::class_environment(tokens, contracts);
    let contracts = class_environment.as_ref().unwrap_or(contracts);
    // Card CALLEE-RETENTION-CLASS1 (rule 12t): the custody contract of every
    // retaining declaration the stream names, under its minted name. The
    // identity on an environment with none.
    let retaining_environment = retained_by_route::retaining_environment(tokens, contracts);
    let contracts = retaining_environment.as_ref().unwrap_or(contracts);
    let normalized = out_alias::normalize(tokens, function, contracts);
    census_stage("out-alias", tokens, normalized.as_ref());
    let tokens = normalized.as_deref().unwrap_or(tokens);
    let (body_start, line) = find_function_body(tokens, function)
        .ok_or_else(|| LegacyCError::FunctionNotFound(function.to_owned()))?;
    // Card RELATIONAL-REWRITE-LINE-ORDER (rule 11r amended): the stream the
    // parser reads keeps its lines non-decreasing; one that does not is a
    // hard error in debug and test builds and refuses the function in
    // release builds (its first instruction below).
    let line_order_refusal = stream_line_order::guard(function, tokens);
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f): the names this body
    // spells in the stream the build reads, which bound its consultations.
    crate::contract_dependencies::body(function, tokens, body_start);
    let parameters = function_parameters(tokens, function).unwrap_or_default();
    let formal_parameters = parameters.iter().flatten().cloned().collect();
    // W48 bottom-up batch 2: an entry custody/out-slot identity is immobile.
    let fixed_formals = own_contract.map_or_else(BTreeSet::new, |contract| {
        parameters
            .iter()
            .zip(&contract.parameters)
            .filter_map(|(name, mode)| {
                (!matches!(mode, ResourceParameterMode::Borrowed))
                    .then(|| name.clone())
                    .flatten()
            })
            .collect()
    });
    let protected_formals =
        protected_resource_formals_under(tokens, body_start, &parameters, contracts, own_contract);
    let variadic_tails = declared_variadic_tails(contracts);
    // Card CG26-VARIADICS (rule 11w): the borrowed tails, a subset.
    let variadic_borrowed_tails = if variadic_tails.is_empty() {
        BTreeSet::new()
    } else {
        qcore_contract::contract_loader::declared_variadic_borrowed_tails(contracts)
            .unwrap_or_default()
    };
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
    // Card W45-PARENTHESISED-CALLEE: the type names a cast may spell, over
    // this function's parameters and body.
    let callee_types = {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let start = function_parameter_range(tokens, function, body_start)
            .map_or(body_start, |(open, _)| open);
        let end = matching(tokens, body_start, "{", "}").map_or(tokens.len(), |end| end + 1);
        parenthesised_callee::TypeNames::read(&texts, start..end)
    };
    let collection_types = collection_types(tokens, function, body_start, &typed_model);
    // Card W44-COLLECTION-ALLOCATION: names whose `sizeof` dereference operand
    // is unevaluated, from the same declarator reader.
    let sizeof_operands = sizeof_operand_types(tokens, function, body_start, &typed_model);
    let descriptor_names = matching(tokens, body_start, "{", "}").map_or_else(
        descriptor_custody::DescriptorNames::default,
        |body_end| {
            let body = tokens[body_start + 1..body_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            descriptor_custody::DescriptorNames::read(&body, contracts, own_contract, &parameters)
        },
    );
    // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38).
    let cell_witness_names = matching(tokens, body_start, "{", "}").map_or_else(
        descriptor_custody::DescriptorNames::default,
        |body_end| {
            let body = tokens[body_start + 1..body_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            descriptor_custody::cell_witness_names(&body, contracts)
        },
    );
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e).
    let result_witness_names = matching(tokens, body_start, "{", "}").map_or_else(
        descriptor_custody::DescriptorNames::default,
        |body_end| {
            let body = tokens[body_start + 1..body_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            descriptor_custody::result_witness_names(&body, contracts)
        },
    );
    // Card CG25-RESOURCE-KINDS (rule 11v).
    let in_place_names = matching(tokens, body_start, "{", "}").map_or_else(
        in_place_custody::InPlaceNames::default,
        |body_end| {
            let body = tokens[body_start + 1..body_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            in_place_custody::InPlaceNames::read(&body, contracts)
        },
    );
    let declared_callees = if descriptor_names.is_empty() && in_place_names.is_empty() {
        BTreeSet::new()
    } else {
        descriptor_custody::declared_callees(contracts)
    };
    // Card W45-RETURN-EXPRESSION-FAMILIES: read before the model moves.
    let return_facts = return_family::FunctionFacts::read(tokens, function, &typed_model);
    // Card W45-CALLEE-SUMMARY-CASCADE: rule 1's lexical premises, read once.
    let member_releases =
        member_release::AdmissibleReleases::read(tokens, body_start, &formal_parameters, contracts);
    // Card W45-SUB-OBJECT-STORES: the record-typed roots.
    let sub_object_roots = match (
        function_parameter_range(tokens, function, body_start),
        matching(tokens, body_start, "{", "}"),
    ) {
        (Some((open, close)), Some(body_end)) => {
            let texts = tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            sub_object_store::RecordRoots::read(
                &typed_model,
                &texts[open + 1..close],
                &texts[body_start + 1..body_end],
                &formal_parameters,
            )
        }
        _ => sub_object_store::RecordRoots::default(),
    };
    // Card W45-FB-BACKLOG-CUSTODY: P-ILI-UNTRACKED's lexical guard, over the
    // same body the typed bases were read from.
    let list_facts = matching(tokens, body_start, "{", "}").map_or_else(
        list_iteration::FunctionFacts::default,
        |body_end| {
            let body = tokens[body_start + 1..body_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            list_iteration::function_facts(&body, &typed_bases)
        },
    );
    // Card W45-FB-RETRY-LIFETIME: the unrebound formals, from the body's own
    // tokens.
    let caller_formals = matching(tokens, body_start, "{", "}").map_or_else(
        container_publication::CallerFormals::default,
        |body_end| {
            let body = tokens[body_start + 1..body_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            container_publication::CallerFormals::read(&body, &formal_parameters)
                // Card W45-CONTAINER-ROOT-REACH: static containers.
                .with_statics(&publications.destinations)
                // Card W45-SR5-SOUNDNESS: a root the body releases is not one.
                .with_released_roots(&body, contracts)
        },
    );
    let mut parser = Parser {
        tokens,
        cursor: body_start,
        contracts,
        next_site: 0,
        formal_parameters,
        fixed_formals,
        protected_formals,
        typed_model,
        typed_types,
        typed_bases,
        store_bases,
        alias_classes,
        variadic_tails,
        variadic_borrowed_tails,
        frame_protected,
        collection_types,
        sizeof_operands,
        element_lowerings: 0,
        descriptor_names,
        cell_witness_names,
        result_witness_names,
        declared_callees,
        in_place_names,
        #[cfg(feature = "indirect-census")]
        census_function: function.to_owned(),
        #[cfg(feature = "indirect-census")]
        census_bindings: census_body_bindings(tokens, body_start, contracts),
        admit_derived_returns,
        argument_probe: call_argument_access::ArgumentProbe::default(),
        member_releases,
        desugar: condition_desugar::DesugarState::new(function),
        return_facts,
        sub_object_roots,
        callee_types,
        list_facts,
        list_frames: Vec::new(),
        caller_formals,
        cell_protocols: descriptor_cells_route::protocol_specs(contracts),
        element_origins: container_publication::ForeignOrigins::default(),
        typed_path_index: std::cell::OnceCell::new(),
        typed_path_function: function.to_owned(),
        typed_position_uses: std::cell::Cell::new(0),
        condition_widening: std::cell::Cell::new(0),
        variadic_tail_widening: std::cell::Cell::new(false),
        variadic_tail_readings: std::cell::RefCell::new(None),
        condition_cycles: condition_acquisition_route::CycleRanges::read(tokens, body_start),
        typed_place_origin: std::cell::Cell::new(None),
        exact_formal_returns: std::cell::OnceCell::new(),
        typed_root_declarations: std::cell::OnceCell::new(),
        readonly_list_count_certified: std::cell::OnceCell::new(),
        declared_no_return: std::cell::OnceCell::new(),
        call_graph_components: std::cell::OnceCell::new(),
    };
    // Card W47-TYPED-ROOT-DECLARATION-READER: parity runs only.
    parser.record_typed_root_parity();
    // Card CG08-CASTS-AND-TYPE-QUERIES (`W45-CALLEE-CAST-LEXICAL-PREMISE`):
    // parity runs only.
    parser.record_callee_cast_census(body_start);
    // Card CAST-FORMALIZATION (rule 12h): parity runs only.
    parser.record_cast_census();
    // Card CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): parity runs only.
    parser.record_layout_census();
    #[cfg(feature = "indirect-census")]
    crate::traversal_census::record_function(function, &parser.typed_types);
    let mut statements =
        parser
            .parse_block()
            .map_err(|failure_line| LegacyCError::MalformedFunction {
                function: function.to_owned(),
                line: failure_line.max(line),
            })?;
    if let Some(description) = line_order_refusal {
        statements.insert(
            0,
            Statement::Instruction(
                LegacyInstruction::Unknown {
                    description,
                    writes: RefusedWrites::Unbounded,
                },
                line,
            ),
        );
    }
    let mut cfg = CfgBuilder::build(function, &statements, publications);
    // Card CG25-RESOURCE-KINDS (rule 11v): no in-place effect is dropped by a
    // route that lowered its call without the object's place.
    in_place_custody::verify_cfg(&mut cfg);
    // Card GAPO-RETURN-CORRECTNESS (rule 15y): a no-return call a later route
    // lowered (or any position but rule 11o's statement hook) ends its path
    // where rule 11o's premises hold and is refused otherwise.
    parser.settle_no_return_nodes(&mut cfg);
    // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): `volatile` objects
    // carry no value-derived fact; a custody decision on a volatile read
    // refuses. Asked only where the unit spells `volatile` before the body's
    // end, so a unit with none asks Clang for nothing more.
    if storage_duration_route::mentions_volatile(tokens, body_start)
        && let Some((index, facts)) = parser.storage_facts()
    {
        storage_duration_route::settle_volatile(&mut cfg, &facts, function);
        parser.record_volatile_traces(&index);
    }
    // Card W45-CONTAINER-ROOT-REACH: the foreign unlinks, over the whole body.
    if let Some(body_end) = matching(tokens, body_start, "{", "}") {
        container_publication::settle_foreign_unlinks(
            &mut cfg,
            &parser.element_origins,
            &tokens[body_start + 1..body_end],
            &parser.caller_formals,
        );
    }
    let (scalars, mut pointers) = parser.typed_types.traversal_names();
    // A copy of an unbound *formal* pointer keeps its unresolved-alias answer:
    // W4 summary derivation reads exactly that answer as the signal to bind
    // the formal (`legacy_summary::bind_unresolved_formals`), and a formal it
    // binds is then seeded, copied and walked as a tracked identity. Only a
    // local's copy (an automatic array, a never-bound pointer) is unbound.
    pointers.retain(|name| !parser.formal_parameters.contains(name));
    cfg.traversal = TraversalNames { scalars, pointers };
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
        // Card SUP07-REFUSED-TEST-ACCUSATION (rule 12e): the tie names.
        cfg.tie_names = refused_test::tie_names(
            tokens,
            &body,
            &parser.formal_parameters,
            &unique,
            &cfg.conditional_names,
        );
    }
    // Card W45-CALLEE-SUMMARY-CASCADE: the sub-custody guard before every call
    // to a function this unit defines (`member_release`, rule 3).
    member_release::guard_unit_callees(&mut cfg, tokens, contracts);
    // Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): the standing of
    // every declared frame of a callee this body calls.
    cell_frames_route::settle_frames(&mut cfg, contracts);
    // Card W45-CALLEE-SUMMARY-CASCADE (`AGGREGATE-NULL-TEST-AFTER-RELEASE`):
    // a null test of a member place keeps the dereference obligation of
    // every pointer it reads through.
    member_release::oblige_member_null_tests(&mut cfg);
    lookup_route::settle(&mut cfg, tokens, function);
    lookup_route::settle_calls(&mut cfg, tokens, contracts);
    self_link_init::restore_unreachable(&mut cfg);
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
    if !contracts.source_assumptions().values().any(|source| {
        source.contains("resource_variadic_data_tail")
                // Card CG26-VARIADICS: the borrowed tail implies the data tail.
                || source.contains("resource_variadic_borrowed_tail")
    }) {
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
    // Card CG10-BOUND-UNTRACKED-ROOTS: why the closure binds each name it
    // binds (parity only).
    bound_untracked_route::record_closure(
        function,
        &texts[body_start + 1..body_end],
        protected_formals,
        contracts,
        variadic_tails,
        &bases,
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
        &texts[open + 1..close],
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

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the declaration facts of the
/// collection rules, read from the same token stream as every typed fact.
fn collection_types(
    tokens: &[Token],
    function: &str,
    body_start: usize,
    model: &TypedAccessModel,
) -> crate::indexed_collection::CollectionTypes {
    let (Some((open, close)), Some(body_end)) = (
        function_parameter_range(tokens, function, body_start),
        matching(tokens, body_start, "{", "}"),
    ) else {
        return crate::indexed_collection::CollectionTypes::default();
    };
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    crate::indexed_collection::CollectionTypes::read(
        model,
        &texts[open + 1..close],
        &texts[body_start + 1..body_end],
    )
}

/// Card **W44-COLLECTION-ALLOCATION**: the `sizeof` operand facts, read from
/// the same token stream and ranges as [`collection_types`].
fn sizeof_operand_types(
    tokens: &[Token],
    function: &str,
    body_start: usize,
    model: &TypedAccessModel,
) -> crate::sizeof_operand::OperandTypes {
    let (Some((open, close)), Some(body_end)) = (
        function_parameter_range(tokens, function, body_start),
        matching(tokens, body_start, "{", "}"),
    ) else {
        return crate::sizeof_operand::OperandTypes::default();
    };
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    crate::sizeof_operand::OperandTypes::read(
        model,
        &texts[open + 1..close],
        &texts[body_start + 1..body_end],
    )
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
// Production reads [`protected_resource_formals_under`]; this form, which
// reads the function's contract from `contracts`, is kept for the unit tests.
#[cfg(test)]
fn protected_resource_formals(
    tokens: &[Token],
    body_start: usize,
    function: &str,
    parameters: &[Option<String>],
    contracts: &ResourceContracts,
) -> BTreeSet<String> {
    protected_resource_formals_under(
        tokens,
        body_start,
        parameters,
        contracts,
        contracts.contract(function),
    )
}

/// [`protected_resource_formals`] with the function's own contract in force
/// given explicitly (card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS), so W4
/// validation reads the protection its reported run reads.
fn protected_resource_formals_under(
    tokens: &[Token],
    body_start: usize,
    parameters: &[Option<String>],
    contracts: &ResourceContracts,
    own_contract: Option<&ResourceFunctionContract>,
) -> BTreeSet<String> {
    let formals = parameters
        .iter()
        .flatten()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut protected = BTreeSet::new();
    if let Some(contract) = own_contract {
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
        // Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: this closure is a
        // flow-insensitive candidate inventory in which a larger set only
        // protects more, so it keeps the pre-card suffix reader: a sub-object
        // store `recs [ i ] . name = src` still relates `name` and `src`,
        // over-approximating as it always did, and no name loses protection.
        let left = &statement[..equal];
        let destination = if left.first() == Some(&"*") {
            direct_assignment_destination(left)
        } else {
            first_suffix_place(left).map(|(_, place)| place)
        };
        let Some(destination) = destination else {
            continue;
        };
        let Some(source) = canonical_place(&statement[equal + 1..]) else {
            continue;
        };
        if is_identifier(&destination) && is_identifier(&source) {
            aliases.push((destination, source));
        }
    }
    // Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (rule 10o): the aliases the
    // statement reading above skips (nested, later-declarator, parenthesised);
    // the run's first `=`, which it read, keeps its decision.
    let first = crate::lexical_nested_assignment::first_equals_of_runs(&body);
    for assignment in crate::lexical_nested_assignment::assignments(&body) {
        if first.contains(&assignment.operator) {
            continue;
        }
        if let Some(source) = canonical_place(&body[assignment.value.clone()])
            && is_identifier(&source)
        {
            aliases.push((assignment.destination.to_owned(), source));
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
enum Statement {
    Instruction(LegacyInstruction, usize),
    /// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): a call that ends its
    /// path. Compiled to a node with no successor that is not an exit, so
    /// nothing after it is reached and it contributes no exit state; its own
    /// instruction (the call's preconditions) is still transferred.
    PathEnd(LegacyInstruction, usize),
    Block(Vec<Statement>),
    If {
        line: usize,
        condition_supported: bool,
        /// Card **B02-BRANCH-CONDITION**: the one contracted call whose
        /// custody effects are evaluated before this branch decision.  The
        /// isolated instruction is produced by the ordinary statement
        /// classifier; `Some` therefore adds no second call-effect model.
        condition_call: Option<Box<LegacyInstruction>>,
        /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
        /// lowers to when its every memory access is a typed scalar access
        /// (see [`Parser::condition_access`]). Consulted only where the
        /// existing lowering would have been the refusal.
        condition_access: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        /// C02-P-ALIAS: the pointer-inequality guard this condition expresses,
        /// when it relates two names the may-alias rule relates. Consulted only
        /// when `null_condition` is `None`, so no null test changes shape.
        distinct_condition: Option<PointerDistinctCondition>,
        /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the element null test this
        /// refused condition expresses, when its premises hold. Consulted only
        /// when the condition is otherwise refused.
        element_condition: Option<(String, crate::indexed_collection::IndexSpelling, bool)>,
        /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
        /// branch edges of a call-bearing condition every route above refused.
        condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
        /// Card **MK-EVENT-CELL-CROSS-FUNCTION** (rule 38c): the condition is
        /// a declared cell protocol's guard test; its edges read it.
        cell_test: Option<Box<descriptor_cells_route::ProtocolStep>>,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    While {
        line: usize,
        condition_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated at
        /// every visit to this loop header, before either successor is chosen.
        condition_call: Option<Box<LegacyInstruction>>,
        /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
        /// lowers to when its every memory access is a typed scalar access
        /// (see [`Parser::condition_access`]). Consulted only where the
        /// existing lowering would have been the refusal.
        condition_access: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
        /// branch edges of a call-bearing condition every route above refused.
        condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
        body: Vec<Statement>,
    },
    For {
        line: usize,
        /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the index and bound of a
        /// canonical counted loop around at least one element operation.
        counted: Option<(String, crate::indexed_collection::Term)>,
        init: Option<Box<(LegacyInstruction, usize)>>,
        condition_always_true: bool,
        condition_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated at
        /// every visit to this loop header, before either successor is chosen.
        condition_call: Option<Box<LegacyInstruction>>,
        /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
        /// lowers to when its every memory access is a typed scalar access
        /// (see [`Parser::condition_access`]). Consulted only where the
        /// existing lowering would have been the refusal.
        condition_access: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
        /// branch edges of a call-bearing condition every route above refused.
        condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
        increment: Option<Box<(LegacyInstruction, usize)>>,
        /// Card **W45-FB-BACKLOG-CUSTODY**: a second increment node, run after
        /// `increment` and before the header (a safe list iteration's
        /// `N = C->next`). `None` for every other loop.
        list_step: Option<Box<(LegacyInstruction, usize)>>,
        body: Vec<Statement>,
    },
    DoWhile {
        line: usize,
        condition_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated
        /// after every completed body execution, before the back/exit choice.
        condition_call: Option<Box<LegacyInstruction>>,
        /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused condition
        /// lowers to when its every memory access is a typed scalar access
        /// (see [`Parser::condition_access`]). Consulted only where the
        /// existing lowering would have been the refusal.
        condition_access: Option<Box<LegacyInstruction>>,
        /// The controlling expression is the single literal token `0`, so the
        /// body runs exactly once (C11 §6.8.5.2) and the CFG carries no back
        /// edge. Keyed on the *token*, never on an evaluated expression
        /// (standing constraint 5a: an operand-class marker is never a
        /// computed value). Card `C-GATEWAY-CONSTANT-DO-WHILE`.
        condition_is_literal_zero: bool,
        writes: RefusedWrites,
        null_condition: Option<PointerNullCondition>,
        /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls and
        /// branch edges of a call-bearing condition every route above refused.
        condition_plan: Option<Box<condition_desugar::ConditionPlan>>,
        body: Vec<Statement>,
    },
    Switch {
        line: usize,
        expression_supported: bool,
        /// Card **B03-LOOP-AND-SWITCH**: the one contracted call evaluated
        /// once before control enters the selected switch arm.
        expression_call: Option<Box<LegacyInstruction>>,
        /// Card **W43-INDEX-TRAVERSAL**: the typed read a refused switch expression
        /// lowers to when its every memory access is a typed scalar access
        /// (see [`Parser::expression_access`]). Consulted only where the
        /// existing lowering would have been the refusal.
        expression_access: Option<Box<LegacyInstruction>>,
        writes: RefusedWrites,
        /// Card **W44-BRANCH-CONDITION-CALLS**: the desugared calls of a
        /// call-bearing switch expression every route above refused.
        expression_plan: Option<Box<condition_desugar::ConditionPlan>>,
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
        /// Card W43-RESULT-ALIAS-RETURNED: the expression is a null pointer
        /// constant. Set only by a parser built for derived-result validation
        /// ([`Parser::admit_derived_returns`]); always `false` otherwise, so
        /// every other CFG is unchanged.
        returns_null_constant: bool,
    },
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED: a comparison operator of
/// [`LegacyInstruction::BranchOnResult`] (serialisable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResultComparison {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl ResultComparison {
    fn of(cmp: qcore_core::result_set::Cmp) -> Self {
        use qcore_core::result_set::Cmp;
        match cmp {
            Cmp::Eq => Self::Eq,
            Cmp::Ne => Self::Ne,
            Cmp::Lt => Self::Lt,
            Cmp::Le => Self::Le,
            Cmp::Gt => Self::Gt,
            Cmp::Ge => Self::Ge,
        }
    }

    /// The comparison of the set algebra.
    #[must_use]
    pub fn cmp(self) -> qcore_core::result_set::Cmp {
        use qcore_core::result_set::Cmp;
        match self {
            Self::Eq => Cmp::Eq,
            Self::Ne => Cmp::Ne,
            Self::Lt => Cmp::Lt,
            Self::Le => Cmp::Le,
            Self::Gt => Cmp::Gt,
            Self::Ge => Cmp::Ge,
        }
    }
}

#[derive(Clone, Debug)]
struct PointerNullCondition {
    source: String,
    true_is_null: bool,
    /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: `Some((when_true, when_false))`
    /// when the condition is a validity test of a descriptor name, which
    /// compiles to [`LegacyInstruction::BranchOnValidity`] instead.
    validity: Option<(ValidityFact, ValidityFact)>,
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): `Some((OP, K))` when
    /// the condition is exactly `NAME OP K` of a status witness whose callee's
    /// derived result set is in force, which compiles to
    /// [`LegacyInstruction::BranchOnResult`] instead.
    result_test: Option<(qcore_core::result_set::Cmp, i128)>,
}

impl PointerNullCondition {
    /// The branch instruction this condition compiles to.
    fn branch(&self) -> LegacyInstruction {
        if let Some((comparison, literal)) = self.result_test {
            return LegacyInstruction::BranchOnResult {
                source: self.source.clone(),
                comparison: ResultComparison::of(comparison),
                literal,
            };
        }
        match self.validity {
            Some((when_true, when_false)) => LegacyInstruction::BranchOnValidity {
                source: self.source.clone(),
                when_true,
                when_false,
            },
            None => LegacyInstruction::BranchOnNull {
                source: self.source.clone(),
                true_is_null: self.true_is_null,
            },
        }
    }
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
    /// Lazily parsed Clang AST of the token stream this CFG reads.
    typed_path_index: std::cell::OnceCell<Option<crate::typed_access_path::StatementIndex>>,
    typed_path_function: String,
    /// Card W47-ACCESS-PATH-POSITIONS: how many condition operands the typed
    /// reader decided so far; a call-free plan is kept only if it grew.
    typed_position_uses: std::cell::Cell<usize>,
    /// Card CG04-CONDITION-ACQUISITION: the widenings of rule 10x a plan
    /// being built took (`condition_acquisition_route::WIDENED_*`).
    condition_widening: std::cell::Cell<u8>,
    /// Card CG26-VARIADICS (rule 11w): set only around the statement
    /// fallback's re-classification, which reads trailing actuals per position.
    variadic_tail_widening: std::cell::Cell<bool>,
    /// Card CG26-TAIL-RESIDUAL-ACTUALS (rule 12g): the Clang readings of the
    /// trailing actuals of the statement the fallback re-classifies (one per
    /// trailing position), set only around that re-classification.
    variadic_tail_readings:
        std::cell::RefCell<Option<Vec<crate::variadic_tail_actuals::TailReading>>>,
    /// Card CG04-CONDITION-ACQUISITION: the body's token ranges that may lie
    /// on a CFG cycle (a split acquisition there is refused).
    condition_cycles: condition_acquisition_route::CycleRanges,
    /// Card CG10-COMMON-TYPED-PLACE (I04): the stream index and length of the
    /// statement being classified, set only around its `classify_simple`.
    typed_place_origin: std::cell::Cell<Option<(usize, usize)>>,
    /// Card W47-TYPED-ROOT-DECLARATION-READER: the AST reader's answers.
    exact_formal_returns: std::cell::OnceCell<BTreeMap<(usize, String), usize>>,
    typed_root_declarations: std::cell::OnceCell<crate::typed_root_declarations::RootDeclarations>,
    /// One exact header-body check per analysed function's translation unit.
    readonly_list_count_certified: std::cell::OnceCell<bool>,
    /// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): the callees the
    /// environment declares `resource_no_return()` for, read on first use.
    declared_no_return: std::cell::OnceCell<BTreeSet<String>>,
    /// Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): the direct-call
    /// graph's components of `tokens`, read on first use by the parity
    /// stream's call-instantiation rows. It replaces a thread-local cache
    /// keyed by the token slice's address, which a later stream allocated at
    /// the same address with the same length could have answered.
    call_graph_components: std::cell::OnceCell<BTreeMap<String, usize>>,
    cursor: usize,
    contracts: &'a ResourceContracts,
    next_site: usize,
    formal_parameters: BTreeSet<String>,
    fixed_formals: BTreeSet<String>,
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
    /// Card CG26-VARIADICS (rule 11w): the subset of `variadic_tails` whose
    /// declaration is `resource_variadic_borrowed_tail()`; only these admit a
    /// trailing name as a `Borrowed` place (T8).
    variadic_borrowed_tails: BTreeSet<String>,
    /// Card **AOM-BINDING-POISON**: the names of this body whose slot no lvalue
    /// but the name itself designates, per [`frame_protected_locals`]. It
    /// admits nothing; it only bounds what a refused statement destroys in the
    /// binding environment. Empty whenever the body could not be delimited,
    /// which restores the pre-repair blanket poison for the whole function.
    frame_protected: BTreeSet<String>,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: pointer-to-pointer bases and
    /// integer scalars of this function, from the same declaration reader.
    collection_types: crate::indexed_collection::CollectionTypes,
    /// Card **W44-COLLECTION-ALLOCATION**: the names whose `sizeof`
    /// dereference operand is never evaluated (see [`crate::sizeof_operand`]).
    sizeof_operands: crate::sizeof_operand::OperandTypes,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: how many element operations
    /// the classifier has lowered so far. A counted loop is recognised only
    /// around a body that lowered at least one, so no other loop's CFG changes.
    element_lowerings: usize,
    /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: the names of this body that
    /// may hold a tracked integer-carried identity. Empty in every environment
    /// that declares no integer-kinded result, which makes every rule reading
    /// it the identity there.
    descriptor_names: descriptor_custody::DescriptorNames,
    /// Card **MK-EVENT-DESCRIPTOR-QUANT** (rule 38): the names holding a
    /// cell-creating call's descriptor-valued witness; read only by the
    /// condition reader. Empty in every environment with no such declaration.
    cell_witness_names: descriptor_custody::DescriptorNames,
    /// Card **CELL-GUARD-RESULT-RANGE-CHECKED** (rule 38e): the names holding
    /// a status witness whose callee's derived result set is in force; read
    /// only by the condition reader. Empty wherever none is in force.
    result_witness_names: descriptor_custody::DescriptorNames,
    /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: the callees a retained
    /// trusted text declares, as opposed to ones only a W4 summary covers.
    declared_callees: BTreeSet<String>,
    /// Card **CG25-RESOURCE-KINDS** (rule 11v): the names of this body passed
    /// as `&NAME` to a declared in-place initialiser. Empty in every
    /// environment that declares no in-place kind.
    in_place_names: in_place_custody::InPlaceNames,
    /// Card **C-GATEWAY-INDIRECT-CENSUS-2**: this function's name, carried only
    /// so the instrumented census can label its records. Compiled out with the
    /// feature off, and read by nothing else.
    #[cfg(feature = "indirect-census")]
    census_function: String,
    /// Card **W43-INDIRECT-ACCESS-GENERAL**: the names this body assigns the
    /// result of a contracted call whose declared result is derived
    /// (`resource_return_alias`/`resource_return_interior`) or owned, read
    /// once from the body's own tokens and the classifier's own contracts.
    /// Census labels only; compiled out with the feature off.
    #[cfg(feature = "indirect-census")]
    census_bindings: crate::indirect_census::BodyBindings,
    /// Card **W43-RESULT-ALIAS-RETURNED**: build the CFG a derived-result
    /// summary is validated on. A whole-call `return f(…)` whose contract in
    /// force has a [`ResourceReturnMode::Derived`] result is desugared like any
    /// other admitted return call, and a null-constant `return` names its exit
    /// (see [`return_call::NULL_RETURN_PLACE`]). `false` for every other CFG,
    /// which therefore keeps the W43-LIBC-RESULT-ALIAS refusal byte for byte.
    admit_derived_returns: bool,
    /// Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS**: the actuals the call
    /// argument rule admitted while it classifies one statement again. Empty
    /// otherwise, which makes every hook reading it the identity.
    argument_probe: call_argument_access::ArgumentProbe,
    /// Card W45-CALLEE-SUMMARY-CASCADE: the member places rule 1 of
    /// `member_release` may admit in this body (its lexical premises).
    member_releases: member_release::AdmissibleReleases,
    /// Card **W44-BRANCH-CONDITION-CALLS**: the enclosing function and the
    /// unit call graph the condition desugaring reads lazily.
    desugar: condition_desugar::DesugarState,
    /// Card **W45-RETURN-EXPRESSION-FAMILIES**: the enclosing function's
    /// declared result carrier, read once from its definition.
    return_facts: return_family::FunctionFacts,
    /// Card **W45-SUB-OBJECT-STORES**: the record-typed roots
    /// ([`sub_object_store::RecordRoots`]).
    sub_object_roots: sub_object_store::RecordRoots,
    /// Card **W45-PARENTHESISED-CALLEE**: the typedef names this function
    /// trusts as types ([`parenthesised_callee::TypeNames`]).
    callee_types: parenthesised_callee::TypeNames,
    /// Card **W45-FB-BACKLOG-CUSTODY**: the function-level premises of the
    /// list-iteration rule ([`list_iteration::FunctionFacts`]).
    list_facts: list_iteration::FunctionFacts,
    /// Card **W45-FB-BACKLOG-CUSTODY**: the admitted iterations whose bodies
    /// are being classified, innermost last. Empty outside every one of them,
    /// which makes rule 4 the identity.
    list_frames: Vec<list_iteration::Frame>,
    /// Card **W45-FB-RETRY-LIFETIME**: the formals no statement rebinds, the
    /// only roots a publication's container may be reached from
    /// ([`container_publication::CallerFormals`]).
    caller_formals: container_publication::CallerFormals,
    /// Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): every record's declared
    /// cell protocol; empty in every environment declaring none.
    cell_protocols: BTreeMap<String, Vec<descriptor_cells_route::ProtocolSpec>>,
    /// Card **W45-CONTAINER-ROOT-REACH**: the element bindings rule 5l
    /// admitted, for the foreign unlink ([`container_publication::ForeignOrigins`]).
    element_origins: container_publication::ForeignOrigins,
}

impl Parser<'_> {
    /// Consult the Clang statement index only after the old classifier has
    /// named an access refusal. Missing or ambiguous AST facts refuse.
    fn typed_path_plan(&self, tokens: &[Token]) -> Option<crate::typed_access_path::StatementPlan> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let function = &self.typed_path_function;
        let index = self
            .typed_path_index
            .get_or_init(|| crate::typed_access_path::StatementIndex::read(function))
            .as_ref()?;
        // Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (rule 10o): the join of
        // the lexical and AST closures. Card CG10 (rule 10t): several
        // candidates are told apart by this run's own occurrence.
        let line = tokens.first()?.line;
        // Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): this
        // consultation's own parity decisions, recorded below and dropped
        // with the frame.
        let decisions = crate::consultation_decisions::Decisions::new();
        let plan = index.statement_with(
            &texts,
            line,
            &|| crate::typed_place::occurrence(self.tokens, self.typed_place_origin.get(), tokens),
            |name| self.joined_never_bound(name),
            Some(&decisions),
        );
        decisions.record_places(function, line, &texts, "statement", plan.is_some());
        decisions.record_classifications(function, line, &texts);
        plan
    }

    /// `fallback` is the refusal the statement had: card CG11 (rule 10v)
    /// carries it on a pointer store, whose flow premises may fail.
    fn typed_path_statement(
        &self,
        tokens: &[Token],
        fallback: &LegacyInstruction,
    ) -> Option<LegacyInstruction> {
        let plan = self.typed_path_plan(tokens)?;
        if crate::typed_access_path::parity_enabled() {
            let texts = tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            crate::typed_access_path::record_parity(
                &self.typed_path_function,
                tokens.first()?.line,
                &texts,
                "old_refusal",
                None,
                Some(&plan),
            );
        }
        if plan.accesses == 0 {
            if plan.pointer_store.is_some() {
                return None;
            }
            return Some(match plan.scalar.destination {
                Some(destination) => LegacyInstruction::ForgetScalar { destination },
                None => LegacyInstruction::NoOp,
            });
        }
        // Card CG12-VOID-VIEW-READBACK (rule 11j): a borrowed destination
        // reading a `void *` member back; a type pun refuses the statement.
        let readback = match plan.readback.as_ref().filter(|_| plan.borrowed_destination) {
            None => None,
            Some(candidate) => match void_view_route::decide(self, tokens, candidate) {
                crate::void_view::ReadbackDecision::Identity => Some(candidate.place()),
                crate::void_view::ReadbackDecision::Pun => {
                    let texts = tokens
                        .iter()
                        .map(|token| token.text.as_str())
                        .collect::<Vec<_>>();
                    return Some(LegacyInstruction::Unknown {
                        description: void_view_route::PUN_REFUSAL.to_owned(),
                        writes: self.refused_writes(&texts),
                    });
                }
                _ => None,
            },
        };
        let access = LegacyInstruction::IndirectScalarAccess {
            alias_reads: plan
                .scalar
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.scalar.bases,
            destination: plan.scalar.destination,
            frame: plan
                .scalar
                .writes_through_pointer
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: plan.chains,
            borrowed_destination: plan.borrowed_destination,
            readback,
            typed_alternative: None,
        };
        // Card CG11-POINTER-VALUED-STORES (rule 10v).
        Some(match plan.pointer_store {
            Some(store) => LegacyInstruction::PointerStore {
                store,
                access: Box::new(access),
                fallback: Box::new(fallback.clone()),
            },
            None => access,
        })
    }

    fn record_legacy_typed_parity(
        &self,
        tokens: &[Token],
        route: &str,
        instruction: &LegacyInstruction,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let new = self.typed_path_plan(tokens);
        crate::typed_access_path::record_legacy_route(
            &self.typed_path_function,
            tokens.first().map_or(0, |token| token.line),
            &texts,
            route,
            &format!("{instruction:?}"),
            new.as_ref(),
        );
    }

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
                // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the null and
                // distinct tests read only fields no statement changes, so
                // each is asked once and its answer used at every place
                // below that asked it again.
                let null_condition = self.null_condition(&condition);
                let distinct_condition = self.distinct_condition(&condition);
                // Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): a test of a
                // call's returned value against a constant, for a callee with
                // a return-code disposition, asked before every other route.
                // Card HANDOFF-DIRECT-CALL-READING (rule 12m amended): a test of
                // a direct call of a hand-off declaration, asked next.
                let relational_plan = (!condition_supported)
                    .then(|| {
                        self.relational_condition_plan(&condition)
                            .or_else(|| self.handoff_condition_plan(&condition))
                    })
                    .flatten();
                let condition_call = (!condition_supported && relational_plan.is_none())
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let condition_access =
                    (!condition_supported && condition_call.is_none() && relational_plan.is_none())
                        .then(|| self.condition_access(&condition))
                        .flatten();
                #[cfg(feature = "indirect-census")]
                self.census_traversal_condition(
                    line,
                    "if",
                    &condition,
                    !condition_supported && condition_call.is_none() && null_condition.is_none(),
                );
                let element_condition = (!condition_supported
                    && relational_plan.is_none()
                    && condition_call.is_none()
                    && condition_access.is_none()
                    && null_condition.is_none())
                .then(|| self.element_condition(&condition))
                .flatten();
                if let Some((_base, _, _)) = &element_condition {
                    self.element_lowerings += 1;
                    #[cfg(feature = "indirect-census")]
                    self.census_element_condition(line, &condition, _base);
                }
                // Card W44-BRANCH-CONDITION-CALLS: asked only where every
                // route above refused the condition.
                let condition_plan = match relational_plan {
                    Some(plan) => Some(plan),
                    None => self.planned_condition(
                        line,
                        &condition,
                        condition_desugar::Position::Branch,
                        !condition_supported
                            && condition_call.is_none()
                            && condition_access.is_none()
                            && element_condition.is_none()
                            && null_condition.is_none()
                            && distinct_condition.is_none(),
                    ),
                };
                // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): a guard test
                // of a record with a declared cell protocol; a desugared plan
                // reads it on its one deciding node.
                let cell_test = self.cell_test_step(&condition, line);
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
                    condition_access,
                    null_condition,
                    distinct_condition,
                    element_condition,
                    cell_test,
                    condition_plan,
                    then_body,
                    else_body,
                })
            }
            Some("while") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let condition_supported =
                    self.supported_expression(&condition, ExpressionRole::Decides);
                // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: asked once (see
                // the `if` arm).
                let null_condition = self.null_condition(&condition);
                let condition_call = (!condition_supported)
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let condition_access = (!condition_supported && condition_call.is_none())
                    .then(|| self.condition_access(&condition))
                    .flatten();
                #[cfg(feature = "indirect-census")]
                self.census_traversal_condition(
                    line,
                    "while",
                    &condition,
                    !condition_supported && condition_call.is_none() && null_condition.is_none(),
                );
                // Card W44-BRANCH-CONDITION-CALLS.
                let condition_plan = self.planned_condition(
                    line,
                    &condition,
                    condition_desugar::Position::Loop,
                    !condition_supported
                        && condition_call.is_none()
                        && condition_access.is_none()
                        && null_condition.is_none(),
                );
                let body = self.parse_statement_body()?;
                Ok(Statement::While {
                    line,
                    writes: self.expression_writes(&condition),
                    condition_supported,
                    condition_call,
                    condition_access,
                    null_condition,
                    condition_plan,
                    body,
                })
            }
            Some("for") => {
                self.cursor += 1;
                let clauses = self.take_parenthesized()?;
                // Card W45-FB-BACKLOG-CUSTODY: an intrusive-list header the
                // premises admit is the whole statement; anything else keeps
                // the lowering below.
                if let Some(statement) = self.list_iteration_statement(&clauses, line)? {
                    return Ok(statement);
                }
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
                // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: asked once (see
                // the `if` arm).
                let null_condition = self.null_condition(&condition);
                let condition_call = (!condition_supported && !condition.is_empty())
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let lowered_before = self.element_lowerings;
                let body_start = self.cursor;
                let condition_access = (!condition_supported && condition_call.is_none())
                    .then(|| self.condition_access(&condition))
                    .flatten();
                #[cfg(feature = "indirect-census")]
                self.census_traversal_condition(
                    line,
                    "for",
                    &condition,
                    !condition_supported && condition_call.is_none() && null_condition.is_none(),
                );
                // Card W44-BRANCH-CONDITION-CALLS.
                let condition_plan = self.planned_condition(
                    line,
                    &condition,
                    condition_desugar::Position::ForLoop,
                    !condition_supported
                        && condition_call.is_none()
                        && condition_access.is_none()
                        && null_condition.is_none(),
                );
                let body = self.parse_statement_body()?;
                let body_end = self.cursor;
                // Card W45-PARENTHESISED-CALLEE: a clause whose stripped
                // spelling lost its groups is never admitted from it.
                let original = parenthesised_callee::for_clauses(&clauses);
                let init = (!init.is_empty()).then(|| {
                    let original = original.as_ref().map(|clauses| clauses[0].as_slice());
                    Box::new((self.classify_for_clause(&init, original), line))
                });
                let increment_clause = increment.clone();
                let increment = (!increment.is_empty()).then(|| {
                    let original = original.as_ref().map(|clauses| clauses[2].as_slice());
                    Box::new((self.classify_for_clause(&increment, original), line))
                });
                let counted = (self.element_lowerings > lowered_before
                    && condition_supported
                    && condition_call.is_none()
                    && null_condition.is_none())
                .then(|| {
                    self.counted_loop(
                        init.as_deref(),
                        &condition,
                        &increment_clause,
                        increment.as_deref(),
                        &self.tokens[body_start..body_end],
                    )
                })
                .flatten();
                #[cfg(feature = "indirect-census")]
                {
                    let clauses = split_for_clauses(&clauses).unwrap_or_default();
                    if let Some(clause) = &init {
                        self.census_traversal_statement(line, "for-init", &clauses[0], &clause.0);
                    }
                    if let Some(clause) = &increment {
                        self.census_traversal_statement(
                            line,
                            "for-increment",
                            &clauses[2],
                            &clause.0,
                        );
                    }
                }
                Ok(Statement::For {
                    line,
                    counted,
                    writes: self.expression_writes(&condition),
                    init,
                    condition_always_true: condition.is_empty(),
                    condition_supported,
                    condition_call,
                    condition_access,
                    null_condition,
                    condition_plan,
                    increment,
                    list_step: None,
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
                // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: asked once (see
                // the `if` arm).
                let null_condition = self.null_condition(&condition);
                let condition_call = (!condition_supported)
                    .then(|| self.condition_call_instruction(&condition))
                    .flatten()
                    .map(Box::new);
                let condition_access = (!condition_supported && condition_call.is_none())
                    .then(|| self.condition_access(&condition))
                    .flatten();
                #[cfg(feature = "indirect-census")]
                self.census_traversal_condition(
                    line,
                    "do-while",
                    &condition,
                    !condition_supported && condition_call.is_none() && null_condition.is_none(),
                );
                // Card W44-BRANCH-CONDITION-CALLS. The body was parsed first,
                // as C writes it; its sites precede the condition's.
                let condition_plan = self.planned_condition(
                    line,
                    &condition,
                    condition_desugar::Position::DoWhile,
                    !condition_supported
                        && condition_call.is_none()
                        && condition_access.is_none()
                        && null_condition.is_none(),
                );
                Ok(Statement::DoWhile {
                    line,
                    writes: self.expression_writes(&condition),
                    condition_supported,
                    condition_call,
                    condition_access,
                    condition_is_literal_zero: condition_is_literal_zero(&condition),
                    null_condition,
                    condition_plan,
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
                let expression_access = (!expression_supported && expression_call.is_none())
                    .then(|| self.condition_access(&expression))
                    .flatten();
                #[cfg(feature = "indirect-census")]
                self.census_traversal_condition(
                    line,
                    "switch",
                    &expression,
                    !expression_supported && expression_call.is_none(),
                );
                // Card W44-BRANCH-CONDITION-CALLS.
                let expression_plan = self.planned_condition(
                    line,
                    &expression,
                    condition_desugar::Position::Switch,
                    !expression_supported
                        && expression_call.is_none()
                        && expression_access.is_none(),
                );
                let body = self.parse_statement_body()?;
                Ok(Statement::Switch {
                    line,
                    writes: self.expression_writes(&expression),
                    expression_supported,
                    expression_call,
                    expression_access,
                    expression_plan,
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
                // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): a computed
                // `goto *E;` is a named boundary, refused in its function
                // rather than failing the whole parse.
                if self.text() == Some("*") {
                    let tokens = self.consume_to_semicolon()?;
                    return Ok(self.computed_goto(&tokens, line));
                }
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
                // Card CG07 (rule 11g): the expression's place in the stream.
                let expression_origin = self.cursor;
                let expression = self.consume_to_semicolon()?;
                // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b), a soundness
                // repair asked first: a pointer C converts to a truth value
                // is no escape.
                if let Some(tested) = self.pointer_test_return(&expression, line) {
                    return Ok(tested);
                }
                let statement = self.return_statement(expression.clone(), line);
                // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the return
                // fallbacks (10s, 10u, 10y, 10z, 11g, 11i, 11u, 12f), in their
                // table's order, each with its own entry condition.
                let lowered = self
                    .ask_return_fallbacks(&route_table::ReturnQuery {
                        expression: &expression,
                        line,
                        origin: expression_origin,
                        statement: &statement,
                    })
                    .unwrap_or(statement);
                // Card GAPO-RETURN-CORRECTNESS (rule 15z), parity runs only:
                // the statement-only rules this return's lowering would need.
                self.record_return_route_gaps(&expression, line, &lowered);
                Ok(lowered)
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
                let origin = self.cursor;
                let tokens = self.consume_to_semicolon()?;
                Ok(self.expression_statement(&tokens, line, Some(origin)))
            }
            None => Err(line),
        }
    }

    /// The lowering of `return EXPRESSION;`, `expression` without `return`
    /// and `;`. Card CG02-ELABORATION-CORE: also the lowering of the value
    /// operand the elaborator hands back after sequencing a comma (rule 10s).
    fn return_statement(&mut self, expression: Vec<Token>, line: usize) -> Statement {
        // Rule 36 (card RULE36-PILOT: package `rule-36-exact-formal-return`),
        // the first return route: in the derived-result CFG only, a confirmed
        // bare return of the original borrowed formal. Off or refused, the
        // return keeps the lowering below.
        if let Some(exact) = self.exact_formal_return_route(&expression, line) {
            return exact;
        }
        {
            {
                let texts = expression
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<Vec<_>>();
                // Card W44-LEAK-PIN-AUDIT (`RETURN-CAST-PLACE-NOT-ESCAPED`): a
                // carrier-preserving cast of one place is decided below as the
                // return of that place, and a carrier-crossing one is refused;
                // see `cast_return`.
                let expression = match cast_return::classify(
                    &texts,
                    &self.descriptor_names,
                    &self.typed_types,
                ) {
                    cast_return::CastReturn::Operand(start) => expression[start..].to_vec(),
                    cast_return::CastReturn::Refuse => {
                        return cast_return::refusal(line, self.expression_writes(&expression));
                    }
                    // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b,
                    // `CAST-RETURN-UNCOMMITTED-OPERAND`): Clang's
                    // `PointerToIntegral` of a possibly tracked pointer.
                    cast_return::CastReturn::PreCard
                        if self.pointer_integer_return(&expression) =>
                    {
                        return cast_return::refusal(line, self.expression_writes(&expression));
                    }
                    cast_return::CastReturn::PreCard => expression,
                };
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
                // Card W47-ACCESS-PATH-POSITION-RESIDUALS (rule 10n). Asked last.
                if !strict
                    && !untracked_pointer
                    && returned_evaluation.is_none()
                    && let Some(desugared) = self.typed_connective_return(&expression, line)
                {
                    return desugared;
                }
                // Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12i). Asked
                // last, where every route above declined or only re-attributed
                // the gate's refusal.
                if !strict
                    && !untracked_pointer
                    && returned_evaluation.as_ref().is_none_or(|(instruction, _)| {
                        return_call_access::is_gate_refusal(instruction)
                    })
                    && let Some(lowered) = self.return_call_argument_access(&expression, line)
                {
                    return lowered;
                }
                #[cfg(feature = "indirect-census")]
                if !strict && !untracked_pointer && returned_evaluation.is_none() {
                    self.census_return(line, &texts);
                }
                // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule 12k). Asked
                // last: the gate's refusal of a whole-call return names the
                // uncontracted callee where it decides; never an admission.
                let returned_evaluation =
                    self.uncontracted_return_attribution(&expression, line, returned_evaluation);
                Statement::Return {
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
                    returns_null_constant: self.admit_derived_returns
                        && return_call::is_null_pointer_constant(&texts),
                }
            }
        }
    }

    /// One expression or declaration statement, `tokens` without its `;`.
    /// Card CG02-ELABORATION-CORE: the statement walker's lowering, also the
    /// lowering of every sequenced piece the elaborator hands back (rule 10s).
    /// `origin` is the run's first token in the stream, `None` for a piece
    /// (card CG10's occurrence identity is then not asked).
    fn expression_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        origin: Option<usize>,
    ) -> Statement {
        let statement = self.expression_statement_lowered(tokens, line, origin);
        // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): a store to, or a
        // close of, a member of a record with a declared cell protocol is
        // read after the statement's own lowering, which is unchanged.
        let step = self.cell_statement_step(tokens);
        // Card CELL-PROTOCOL-CLOSE-ACTUAL (rule 38d): a close the gate
        // refused, whose actual it admits for rule 38c's close.
        if let Some(step) = &step
            && let Some(admitted) = self.cell_close_actual(tokens, line, &statement, step)
        {
            return admitted;
        }
        // Card MEMBER-EFFECT-SUMMARY step 1 (rule 38i). Asked last, after rule
        // 38d: the gate's refusal of member custody through a formal parent
        // names its premise; the node and its writes are unchanged.
        let statement = self.member_custody_attribution(tokens, line, statement);
        match step {
            Some(step) => Statement::Block(vec![
                statement,
                Statement::Instruction(
                    LegacyInstruction::CellProtocolStep {
                        step: Box::new(step),
                    },
                    line,
                ),
            ]),
            None => statement,
        }
    }

    fn expression_statement_lowered(
        &mut self,
        tokens: &[Token],
        line: usize,
        origin: Option<usize>,
    ) -> Statement {
        // Card W45-CONTAINER-ROOT-REACH: keep an element binding only
        // when it is the statement's final instruction.
        self.element_origins.clear();
        // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o), a soundness repair
        // asked first: assembly, SEH and `cleanup` are named boundaries.
        if let Some(boundary) = self.call_exit_boundary(tokens, line) {
            return boundary;
        }
        // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h), a soundness
        // repair asked first: a static-duration declaration runs nothing.
        if let Some(declaration) = self.static_duration_declaration(tokens, line) {
            return declaration;
        }
        // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b), a soundness repair
        // asked first: a pointer C converts to a truth value binds nothing.
        if let Some(tested) = self.pointer_test_store(tokens, line) {
            self.element_origins.clear();
            return tested;
        }
        // Card CG10 (I04): this run's own place in the stream.
        self.typed_place_origin
            .set(origin.map(|origin| (origin, tokens.len())));
        // Card GAPO-RETURN-CORRECTNESS (rule 15x): rule 12h's second
        // classification replaces the first, so it starts from the state the
        // first started from (no gap in site numbering).
        let before = self.probe_state();
        let instruction = self.classify_simple(tokens);
        // Card CAST-FORMALIZATION (rule 12h), a soundness repair asked
        // first: an identity-preserving cast of a place at a call actual is
        // that place, so the call is classified as its uncast statement.
        let instruction = self
            .uncast_call_actuals(tokens, &instruction, &before)
            .unwrap_or(instruction);
        self.typed_place_origin.set(None);
        self.element_origins.settle(tokens, &instruction);
        // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): a declared
        // no-return call (new with the card, so no older lowering is
        // replaced) ends its path where its declaration establishes it.
        if let Some(exit) = self.call_exit_statement(tokens, line, &instruction) {
            return exit;
        }
        #[cfg(feature = "indirect-census")]
        self.census_statement(line, tokens, &instruction);
        #[cfg(feature = "indirect-census")]
        self.census_traversal_statement(line, "statement", tokens, &instruction);
        // Card W45-SUB-OBJECT-STORES: the sub-object store census.
        #[cfg(feature = "indirect-census")]
        self.census_sub_object_store(line, tokens, &instruction);
        // Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the statement fallbacks
        // (W43 stepped access, W44 call actuals, 10s, 11b, 10u, 10y, 10z, 11g,
        // 11f, 11i, 11u, 11w, 12h), in their table's order.
        if let Some(statement) = self.ask_statement_fallbacks(&route_table::StatementQuery {
            tokens,
            line,
            origin,
            instruction: &instruction,
        }) {
            return statement;
        }
        if let LegacyInstruction::ContractCall { destination, .. } = &instruction {
            let use_ = if destination.is_some() {
                crate::call_instantiation::ResultUse::Stored
            } else {
                crate::call_instantiation::ResultUse::Discarded
            };
            self.record_instantiation_parity(
                crate::call_instantiation::Consumer::Statement,
                line,
                &instruction,
                use_,
            );
        }
        // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (rules 14a, 14b),
        // asked after every statement fallback, only for the gate's refusal.
        self.typed_place_origin
            .set(origin.map(|origin| (origin, tokens.len())));
        let residual = self.residual_access_statement(tokens, line, &instruction);
        self.typed_place_origin.set(None);
        if let Some(statement) = residual {
            return statement;
        }
        // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule 12k). Asked last,
        // only for the gate's refusal: it names the uncontracted callee where
        // the callee decides; the node stays the same refusal with its writes.
        self.typed_place_origin
            .set(origin.map(|origin| (origin, tokens.len())));
        let instruction = self.uncontracted_access_attribution(tokens, line, instruction);
        self.typed_place_origin.set(None);
        Statement::Instruction(instruction, line)
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

    fn text(&self) -> Option<&str> {
        self.tokens
            .get(self.cursor)
            .map(|token| token.text.as_str())
    }
}

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the token stream: the tokenizer,
// conditional-directive scan, raw definition scan, body and parameter
// readers and bracket matching.
#[path = "legacy_tokens.rs"]
mod tokens;
pub use self::tokens::*;

#[path = "legacy_conditional_embedded_storage.rs"]
mod legacy_conditional_embedded_storage;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the parser's measurement-only
// census hooks (feature `indirect-census`).
#[cfg(feature = "indirect-census")]
#[path = "legacy_parser_census.rs"]
mod parser_census;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the expression and condition
// readers the statement and condition routes ask (ROUTE-MAP sections 3 and
// 6): writes, scalar effects, the supported test, the condition call,
// access, null and distinct routes, alias reads.
#[path = "legacy_condition_routes.rs"]
mod condition_routes;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the return-expression routes
// (ROUTE-MAP section 4, step 2): W4 return call, B01, W43 and W45 families,
// 10k, 11h.
#[path = "legacy_return_routes.rs"]
mod return_routes;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the statement classifier
// (ROUTE-MAP section 2.2): `classify_simple` and its guards, the typed
// fallbacks of `classify_simple_unguarded`, the element, sub-object and
// stepped routes and `classify_simple_base`.
#[path = "legacy_classify.rs"]
mod classify;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: lexical shape readers over token
// texts: calls, arguments, conditions, places and declarators.
#[path = "legacy_token_shapes.rs"]
mod token_shapes;
pub(crate) use self::token_shapes::*;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the CFG builder: parsed
// statements compiled to `LegacyCfg` nodes, with each condition's compile
// precedence (ROUTE-MAP section 3).
#[path = "legacy_cfg_builder.rs"]
mod cfg_builder;
pub(crate) use self::cfg_builder::*;

// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the abstract state and the
// transfer functions: pointers, derivations, entry custody, exit
// observations and the flow analysis over a CFG.
#[path = "legacy_exact_formal_return.rs"]
pub(crate) mod exact_formal_return;
#[path = "legacy_self_link_init.rs"]
pub(crate) mod self_link_init;
#[path = "legacy_tracked_self_init.rs"]
pub(crate) mod tracked_self_init;

#[path = "legacy_transfer.rs"]
mod transfer;
pub use self::transfer::*;

#[cfg(test)]
#[path = "../tests/unit/legacy_c.rs"]
mod tests;
