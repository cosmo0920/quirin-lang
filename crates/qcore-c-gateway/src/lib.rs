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

//! Conservative existing-C analysis and Clang transport (L2 gateway).
//!
//! The gateway validates preprocessing, derives and checks custody summaries,
//! and rederives certificate evidence. Contracts remain explicit assumptions.
//! No provider or compiler crate is a dependency. The abstract interpreter,
//! summary inference machinery and entry-state constructors remain private.
//!
//! [`typed_c_ast`] provides raw Clang JSON transport for source-profile
//! interpreters. Its output alone is never a safety certificate.

// Card W46-ACCESS-PATH-CENSUS: the analysed text of every unit, for the
// typed access-path census.
#[cfg(feature = "indirect-census")]
pub mod access_path_census;
mod aggregate_store;
pub mod atomic_publication_source;
pub mod audit_ir;
pub mod audit_ir_capture;
pub mod audit_ir_store;
// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): C atomics routed to the
// core weak-memory model.
pub mod atomic_source_route;
pub mod boolean_source;
mod borrow_liveness;
mod borrowed_actual;
pub mod call_use_source;
// Card CG20-NUMERIC-DOMAIN (rule 11q): the C integer domain per target
// identity, and the integer arithmetic property beside custody.
pub mod c_numeric;
pub mod c_numeric_obligations;
mod clang;
mod clang_ast_input;
// Card VERDICT-CONTRACT-DEPENDENCIES (rules 13f-13h): the declarations each
// row's derivation consulted, and the unit's body evidence; evidence only.
pub mod contract_dependencies;
// Bounded, evidence-only libc reader captures and independent replay.
pub mod reading_site_evidence;
// Card VERDICT-BODY-STANDING-CLAUSE-SCOPE (rules 13i-13k): the clauses of a
// declaration and which of them each piece of body evidence covers.
pub mod contract_clause_standing;
// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: one unit's Clang results reused
// across the analysis scopes of a worker thread.
mod clang_reuse;
pub use clang_reuse::with_audit_unit_lifetimes;
pub mod cleanup_replay_source;
pub mod composition_source;
#[cfg(feature = "indirect-census")]
pub mod condition_call_census;
pub mod counter_source;
#[cfg(feature = "indirect-census")]
pub mod destination_suffix_census;
pub mod ffi_layout;
// Card W48-QRLIB-LINKAGE: the exported symbol table of a C-ABI artifact on
// every platform, and the generated `.qrlib` manifest.
pub mod linkage;
// Card LINK-STANDING (rule 40): a cross-unit callee's type in one unit.
pub mod link_call_type;
// Card CG03-CALL-SUMMARY-INSTANTIATION (rule 10u): the call-contract
// acceptance and the typed per-formal substitution; the G hooks are in
// `legacy_call_instantiation.rs`.
mod call_instantiation;
mod scalar_accessor_condition;
mod scalar_read_identity;
// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): casts as representations and
// type-query operands by evaluation class; the G hooks are in
// `legacy_cast_representation.rs`.
mod cast_representation;
// Card CAST-FORMALIZATION (rule 12h): one conversion table, by class, every
// route-G position consults; the G hooks are in `legacy_cast_semantics.rs`.
mod cast_semantics;
// Card CG04-CONDITION-ACQUISITION (rule 10x): acquisitions in conditions and
// complex condition leaves; the G hooks are in `legacy_condition_acquisition.rs`.
mod condition_acquisition;
// Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): a consultation's
// parity decisions, owned by the consumer that records them.
mod consultation_decisions;
// Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): return-code dispositions;
// the G hooks are in `legacy_relational_post.rs` and `legacy_summary.rs`.
mod relational_post;
// Card CALLEE-RETENTION-CLASS1 (gateway rule 12t): retained pointers; the
// decisions are here, the G hooks in `legacy_retained_by.rs`.
pub mod retained_by;
// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (rule 12n): the body check
// of a registration forwarded through an id.
pub mod registration_by_id;
// Card CG22-CALLBACK-REGISTRY-CONTRACT (rules 13u-13y): callback contracts.
pub mod arm_release_check;
pub mod callback_contracts;
pub mod callback_member_closure;
/// Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 13u amended): a candidate's own
/// member invocations, composed at the invocation.
pub mod candidate_composition;
pub mod constructed_state;
pub mod handoff_branch;
/// Card HANDOFF-DIRECT-CALL-READING (rule 12m amended): a direct call of a
/// hand-off declaration, split at the caller's own test of its result.
pub mod handoff_direct;
/// Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): the body check of a
/// declaration's member-path states.
pub mod member_path_check;
/// Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS (rule 30, amended): the
/// bounded path reading behind it, through the unit's static helpers.
pub mod member_path_reading;
// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n): the run's member facts.
pub mod lookup_record;
pub mod loop_lifetime_source;
pub mod loop_lifetime_summary;
pub mod member_run_facts;
// Card SLOT-MOVE-AND-INTERIOR-BORROW (rules 12x, 12y, 12z): the pure
// decisions are here, the G hooks in `legacy_slot_move_borrow.rs`.
pub mod detach_same_item;
pub mod list_lookup;
// Card DETACH-RESULT-WRAPPER-COMPOSITION (rule 33): the pure removed-selection
// decisions moved to `crates/gateway-rules/rule-33-removed-selection` (card
// RULE33-PILOT); the G hooks are in `legacy_selection.rs`.
pub mod slot_move_borrow;
// Card REFCOUNT-RESOURCE-KIND (gateway rules 13b, 13c): the reference-count
// kind's pure decisions are here, the G hooks in `legacy_reference_count.rs`.
pub mod reference_count;
// Card CG02-ELABORATION-CORE (rule 10s): the common expression elaborator and
// its order model; the G hooks are in `legacy_elaboration.rs`.
mod elaboration;
// Card CG16-INDEX-AND-OFFSET-EXTENTS (rule 11s): the separately selected
// bounds property; no custody route reads it.
pub mod index_extent;
mod index_traversal;
// Card NULL-SAFETY-PROPERTY (rules 14k-14p): the separately selected
// null-dereference property; no custody route reads it.
pub mod null_deref;
// Card NULL-SAFETY-PROPERTY (rule 14n): the named null premises a custody
// verdict rests on; the hook is `legacy_null_premise.rs`.
pub mod null_premise;
// Card CG04-LOOP-SITE-GENERATIONS: generation-indexed acquisition sites; the G
// hooks are in `legacy_loop_generations.rs`.
pub mod indexed_collection;
#[cfg(feature = "indirect-census")]
pub mod indirect_census;
mod indirect_chain_access;
#[cfg(feature = "indirect-census")]
pub mod indirect_chain_census;
pub mod legacy_c;
pub mod legacy_census;
#[cfg(feature = "scalar-form-census")]
pub mod scalar_form_census;
// Card SCALAR-STATEMENT-FORMS (rule 41): Clang-typed scalar statement forms;
// the G hooks are in `legacy_scalar_statement_forms.rs`.
mod scalar_statement_forms;
// Card GATEWAY-RULE-API: the rule registry and its run-time switches.
pub mod gateway_rules;
// Card RULE34-PILOT: the gateway as host of the transfer position (rule 34).
mod legacy_condition_call;
mod loop_generations;
mod transfer_rules;
// Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (rule 11c): partial field links
// and the settled-exit floor; the G hooks are in `legacy_field_link_join.rs`.
mod field_link_join;
// Card MONKEY-VHOST-PROPERTY-CUSTODY (rule 12d): a publication carries the
// owner's field-linked children; the G hooks are in
// `legacy_container_publication.rs`.
mod carried_publication;
// Card CG18-FOOTPRINT-CONNECTION (rule 11k): source places connected to the
// core footprint model; the elaborator hook is in `legacy_elaboration.rs`.
mod footprint_connection;
mod nested_call_effects; // CG05-NESTED-CALL-EFFECTS (rule 10y); G hook in legacy_nested_call.rs
// CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the event footprints live in
// `elaboration.rs`, the G lowering in legacy_elaborator_calls.rs.
#[cfg(test)]
#[path = "../tests/unit/elaborator_call_footprints.rs"]
mod elaborator_call_footprints_tests;
// Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (rule 10o).
mod legacy_summary;
mod lexical_nested_assignment;
mod may_alias;
mod neutral_position;
mod publication;
// Card W44-COLLECTION-ALLOCATION: unevaluated `sizeof` dereference operands.
// Card RULE37-PILOT: the host side of rule 37's package.
mod result_constructor_rule;
mod sizeof_operand;
// Card W45-SUB-OBJECT-STORES: the sub-object store census.
pub mod source_call_use;
#[cfg(feature = "indirect-census")]
pub mod sub_object_store_census;
pub mod teardown_source;
// Card W47-CONDVAR-JOIN-PUBLICATION (rule 10q): condvar publication and
// join-before-release, read lexically.
pub mod condvar_census;
// Card W47-LOOP-OWNER-CONTRACTS (rule 10p): derived thread identity.
pub mod thread_entry_census;
#[cfg(feature = "indirect-census")]
pub mod traversal_census;
mod type_query;
mod typed_access_path;
// Card RDHTTP-HEADER-SUMMARY-GAP (rules 13o, 13p): the unit index of a unit
// nesting beyond serde_json's default recursion limit.
mod deep_unit_ast;
// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rules 13q, 13r): the Clang JSON
// transports and the nesting each reads.
mod deep_ast_transports;
// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rules 13s, 13t): one function's
// typed readings computed once per unit index.
mod function_reading_reuse;
// Card W47-ACCESS-PATH-POSITIONS: the typed reader in condition operands,
// returns and call actuals.
mod typed_access_positions;
// Card W47-ACCESS-PATH-POSITION-RESIDUALS (rule 10n).
mod typed_position_shapes;
// Card MEMBER-ACCESS-PROJECTION (rule 38h): an uninitialized declaration the
// lexical gate refuses, read from the Clang AST.
mod typed_declaration;
// Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (rules 14a-14d).
mod residual_access_reading;
// Card RULE35-PILOT: the host of the rule packages under
// `crates/gateway-rules/` (rule 35's two arms).
mod rule_hosts;
// Card ARM-OBLIGATIONS: every package consultation, carrying an admission's
// obligations into the unit's rows.
pub mod arm_obligations;
mod scalar_element_address;
// Card CG10-COMMON-TYPED-PLACE (rule 10t): the common typed place.
mod typed_place;
// Card CG11-POINTER-VALUED-STORES (rule 10v): pointer-valued stores.
mod pointer_store;
// Card CG12-VOID-VIEW-READBACK (rule 11j): `void *` views and readbacks; the
// G hook is `legacy_void_view.rs`.
mod void_view;
// Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the per-path bound-untracked
// root premise; the flow hook is `legacy_bound_untracked.rs`.
mod bound_untracked;
// Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12j): `.`-only scalar places
// on a record object of the function's automatic storage; the hook is
// `typed_access_path::Reader::path`.
mod record_object_place;
// Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f): record copies,
// braced initializers, compound literals and string initializers; the G hook
// is `legacy_aggregate_value.rs`.
mod aggregate_value;
// Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): union-member and bit-field
// place steps.
mod union_bitfield;
// Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): storage duration and
// `volatile`; the G hooks are in `legacy_storage_duration.rs`.
mod storage_duration;
// Card CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): VLA, flexible-array,
// alignment and `container_of` layout facts from Clang; the G hooks are in
// `legacy_layout_extents.rs`, the published report is `layout_report`.
pub mod layout_extents;
// Card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): value regions and
// their order premise; the G hooks are in `legacy_value_connective.rs`.
mod value_connective;
// Card CG06-VALUE-CONNECTIVE-RESIDUALS, sibling-effect part (rule 11m): a
// region composed with its siblings by CG05's footprints.
mod connective_footprints;
// Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p): candidate callee sets
// and the callback relation; the G pass is `legacy_indirect_calls.rs`.
mod indirect_calls;
// Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): Clang's selected
// `_Generic` association and GNU statement expressions; the G hooks are in
// `legacy_statement_expression.rs` and `legacy_generic_selection.rs`.
mod generic_association;
mod statement_expression;
// Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): no-return path ends and the
// explicit boundaries; the G hooks are in `legacy_call_exits.rs`.
mod call_exits;
// Card W47-TYPED-ROOT-DECLARATION-READER (rule 10l).
// CG30-DIALECTS-FLAGS-TYPES (rule 11y): dialect, target and type identities.
pub mod c_frontend_identity;
pub mod typed_c_ast;
// Card OUTER-CONTRACT (rule 31): the record graph a runtime projection reads.
pub mod outer_record_graph;
// Card W47-LOOP-OWNER-TYPED-DISPATCH (rule 10p amendment): the record type
// of each field dispatch and store, for the thread-entry census.
pub mod typed_dispatch_census;
// Card W47-TYPED-CONST-POINTER-VALUE (rule 10m).
mod readonly_actual_frame;
mod typed_const_pointer_value;
mod typed_field_access;
mod typed_root_declarations;
pub mod typed_source;
pub mod typed_source_match;
// Card CG01-T-OPERATOR-CLOSURE (rule 10r): route T's operator and literal
// closure and the per-consumer admission that gates it.
pub mod typed_source_nodes;
mod variadic_nested;
// Card CG26-VARIADICS (rule 11w): the `va_list` protocol; the G hook is
// `legacy_variadics.rs`.
mod va_list_protocol;
// Card CG26-VARIADICS (rule 11w): trailing actuals read per position.
mod variadic_tail;
// Card CG26-TAIL-RESIDUAL-ACTUALS (rule 12g): a trailing actual read from its
// Clang node where rule 11w's token premises refuse it.
mod variadic_tail_actuals;

use qcore_contract::contract_loader;

/// Construct a validated Clang command using the configured discovery policy.
/// An invalid or empty `QCORE_CLANG` override returns an error without fallback.
/// Callers select arguments and remain responsible for subprocess execution.
pub use clang::command as clang_command;
pub use clang::{ClangIdentity, ClangVersion, detected_version as clang_version};

/// The census streams compiled into this build whose environment variable is
/// set, by variable name (card **W44-AUDIT-PARALLEL-UNITS**).
///
/// Every census sink is a process-global stream written at the moment the
/// analyzer decides, and two of them number their records from process-global
/// counters, so record order is analysis order. A caller that could analyse
/// translation units concurrently must run them one at a time whenever this is
/// non-empty; that is what keeps a census stream byte-identical to a
/// sequential run's. Empty in a build without the census features, where no
/// sink exists.
pub fn active_census_sinks() -> Vec<&'static str> {
    let mut variables = census_sink_variables();
    variables.retain(|variable| std::env::var_os(variable).is_some());
    variables
}

/// Every census stream variable compiled into this build, set or not; the
/// population [`active_census_sinks`] filters. A guard test checks it against
/// the sinks declared in this crate's sources.
// With only the `scalar-form-census` feature on, the list is one `push`.
#[cfg_attr(feature = "scalar-form-census", allow(clippy::vec_init_then_push))]
pub fn census_sink_variables() -> Vec<&'static str> {
    #[allow(unused_mut)]
    // The W46 parity stream carries stable source/function/line/route keys
    // and is sorted before archival; it does not number records by arrival.
    let mut variables: Vec<&'static str> = Vec::new();
    #[cfg(feature = "indirect-census")]
    variables.extend([
        indirect_census::CENSUS_PATH_VARIABLE,
        indirect_census::RECORD_CENSUS_PATH_VARIABLE,
        indirect_census::RETURN_CENSUS_PATH_VARIABLE,
        indirect_census::STATEMENT_CENSUS_PATH_VARIABLE,
        indirect_census::DERIVED_CENSUS_PATH_VARIABLE,
        traversal_census::TRAVERSAL_CENSUS_PATH,
        indirect_chain_census::CHAIN_CENSUS_PATH,
        condition_call_census::CONDITION_CALL_CENSUS_PATH,
        legacy_c::callee_scope::census::CALLEE_SCOPE_CENSUS_PATH_VARIABLE,
        // Added at the W44-AUDIT-PARALLEL-UNITS integration: the leak-witness
        // stream (W44-LEAK-PIN-AUDIT) numbers invocations from a global
        // counter and the destination-suffix stream
        // (W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE) is process-global, so
        // either forces one job like the others.
        legacy_c::leak_witness::LEAK_WITNESS_PATH_VARIABLE,
        destination_suffix_census::DESTINATION_SUFFIX_CENSUS_PATH,
        // Card W45-RETURN-EXPRESSION-FAMILIES: the return-family decisions,
        // and W44-INDIRECT-CALL-ARGUMENT-ACCESS's stream, which the guard
        // below found unlisted on the card's base.
        legacy_c::return_family::census::RETURN_FAMILY_CENSUS_PATH,
        legacy_c::call_argument_access::census::CALL_ARGUMENT_CENSUS_PATH,
        // Card W45-SUB-OBJECT-STORES.
        sub_object_store_census::SUB_OBJECT_STORE_CENSUS_PATH,
        // Card W45-PARENTHESISED-CALLEE.
        legacy_c::parenthesised_callee::census::PARENTHESISED_CALLEE_CENSUS_PATH,
        // Card W46-ACCESS-PATH-CENSUS: files are named by the global unit
        // ordinal, so the stream forces one job like the others.
        access_path_census::ACCESS_PATH_CENSUS_PATH,
    ]);
    #[cfg(feature = "provenance-rounds-census")]
    variables.push(legacy_c::joined_provenance::provenance_rounds::CENSUS_PATH_VARIABLE);
    // Card SCALAR-STATEMENT-FORMS: one process-global stream.
    #[cfg(feature = "scalar-form-census")]
    variables.push(scalar_form_census::SCALAR_FORM_CENSUS_PATH);
    variables
}

#[cfg(all(test, feature = "indirect-census"))]
#[path = "../../../tests/support/tracked_files.rs"]
mod census_tracked_files;

#[cfg(all(test, feature = "indirect-census"))]
mod census_sink_inventory {
    use super::census_tracked_files as tracked_files;

    /// Every source file that declares a process-global stream sink must have
    /// its path variable in [`super::census_sink_variables`], or a parallel
    /// audit would interleave that stream (integration of
    /// W44-AUDIT-PARALLEL-UNITS found two sinks missing).
    #[test]
    fn every_declared_sink_forces_one_job() {
        let listed = super::census_sink_variables();
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        // Card W45-TEST-HYGIENE: the tracked files directly in `src/`; an
        // untracked or ignored file there is not a source.
        for path in tracked_files::tracked_files(&source) {
            if path.parent() != Some(source.as_path())
                || path.extension().is_none_or(|extension| extension != "rs")
            {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            if !text.contains("BufWriter<File>>>> = Mutex::new(None)") {
                continue;
            }
            if path.ends_with("provenance_rounds.rs") && !cfg!(feature = "provenance-rounds-census")
            {
                continue;
            }
            for line in text.lines() {
                let line = line.trim();
                let Some(rest) = line.strip_prefix("pub const ") else {
                    continue;
                };
                let Some((_, literal)) = rest.split_once("&str = \"") else {
                    continue;
                };
                let variable = literal.trim_end_matches("\";");
                if variable.starts_with("QCORE_")
                    && !variable.ends_with("_CAP")
                    && !listed.contains(&variable)
                {
                    missing.push(format!("{}: {variable}", path.display()));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "census sinks that do not force one job: {missing:?}"
        );
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::io::Write;
    use std::sync::{Once, OnceLock};

    static CLANG_AVAILABILITY: OnceLock<Result<(), String>> = OnceLock::new();
    static CLANG_WARNING: Once = Once::new();

    /// Report whether the Clang-backed tests can run, emitting one visible
    /// warning when the executable is absent or unusable.
    pub(crate) fn clang_available() -> bool {
        let availability = CLANG_AVAILABILITY.get_or_init(|| crate::clang_command().map(|_| ()));
        match availability {
            Ok(()) => true,
            Err(reason) => {
                CLANG_WARNING.call_once(|| {
                    // Write directly so libtest's successful-test capture does
                    // not hide the reason these tests did not execute.
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "warning: skipping Clang-dependent tests: {reason}"
                    );
                });
                false
            }
        }
    }
}

mod call_free_condition_reads;

/// Bounded complete recorded-site source replay; no verdict consumer.
pub mod reading_site_domain;

// Source-checked collector selection and constructor-local conditional identity.
pub mod collector_identity;
