//! Card **GAPO-RETURN-CORRECTNESS** (gateway rule 15x): the parser state a
//! classification probe may move, saved as one value and handed back by
//! every probe that declines.
//!
//! A *probe* classifies tokens to ask a question (W4's `PLACE = f(…)`,
//! B01's scalar call, W45-K's converted call, rule 12h's uncast statement,
//! rule 11w's widened tail, a guard that replaces an admission, every
//! fallback that tries a lowering and keeps the refusal). A probe that
//! declines must be observationally identical to never having run: before
//! this card most restored `next_site` and `element_lowerings` only, some
//! only `next_site`, and the descriptor guard, rule 12h's second
//! classification and rule 11w's declined widening restored nothing
//! (ROUTE-MAP findings 1, 2 and 5).
//!
//! **Every field of [`Parser`] is classified here by construction.**
//! [`Parser::probe_state`] destructures the parser without `..`, so a new
//! field does not compile until it is named in one of the classes below, and
//! the unit test pins [`PARSER_FIELDS`] against the struct's declaration and
//! [`PROBE_STATE_FIELDS`] against the snapshot:
//!
//! * [`FieldClass::ProbeState`]: moved by a classification and restored by a
//!   declined probe (the counters, the per-statement scratch, the argument
//!   probe, the element origins' admitted and pending bindings).
//! * [`FieldClass::MonotoneFact`]: a fact about the *source* statement that a
//!   classification may only grow, kept by a declined probe because it
//!   over-approximates a hazard whichever route lowers the statement: a list
//!   iteration's protected element names and link nodes (rule 4 refuses an
//!   unlink of them), and the names rule 5l(2) bound over a formal
//!   (W45-SR5-SOUNDNESS: "wherever the binding was proposed"). Restoring
//!   either could only remove a refusal, which would be the
//!   fail-open direction.
//! * [`FieldClass::FunctionFact`]: read-only after the parser is built.
//! * [`FieldClass::LazyCache`]: a value-determined answer computed on first
//!   use; recomputing it would give the same value, so a probe that fills it
//!   changes no later answer.
//! * [`FieldClass::Census`]: compiled only with the census feature, labels.
//!
//! Along with the parser's [`FieldClass::ProbeState`] fields, [`ParserProbeState`]
//! snapshots and restores the thread's contract consultation log (rule 13f),
//! so a declined probe's contract lookups do not enter the row's dependencies.
//! Parity rows and censuses outside the parser stay recorded.

use qcore_schemas::contract_consultation::{self, Consulted};

use super::{Parser, call_argument_access::ArgumentProbe, container_publication::OriginsLowering};
use crate::variadic_tail_actuals::TailReading;

/// The class of one [`Parser`] field (module documentation). Read by the
/// unit test that pins the classification against the declaration.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FieldClass {
    ProbeState,
    MonotoneFact,
    FunctionFact,
    LazyCache,
    Census,
}

/// Every field of [`Parser`], in declaration order, with its class.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) const PARSER_FIELDS: &[(&str, FieldClass)] = &[
    ("tokens", FieldClass::FunctionFact),
    ("typed_path_index", FieldClass::LazyCache),
    ("typed_path_function", FieldClass::FunctionFact),
    ("typed_position_uses", FieldClass::ProbeState),
    ("condition_widening", FieldClass::ProbeState),
    ("variadic_tail_widening", FieldClass::ProbeState),
    ("variadic_tail_readings", FieldClass::ProbeState),
    ("condition_cycles", FieldClass::FunctionFact),
    ("typed_place_origin", FieldClass::ProbeState),
    ("exact_formal_returns", FieldClass::LazyCache),
    ("typed_root_declarations", FieldClass::LazyCache),
    ("readonly_list_count_certified", FieldClass::LazyCache),
    ("declared_no_return", FieldClass::LazyCache),
    ("call_graph_components", FieldClass::LazyCache),
    ("cursor", FieldClass::ProbeState),
    ("contracts", FieldClass::FunctionFact),
    ("next_site", FieldClass::ProbeState),
    ("formal_parameters", FieldClass::FunctionFact),
    ("fixed_formals", FieldClass::FunctionFact),
    ("protected_formals", FieldClass::FunctionFact),
    ("typed_model", FieldClass::FunctionFact),
    ("typed_types", FieldClass::FunctionFact),
    ("typed_bases", FieldClass::FunctionFact),
    ("store_bases", FieldClass::FunctionFact),
    ("alias_classes", FieldClass::FunctionFact),
    ("variadic_tails", FieldClass::FunctionFact),
    ("variadic_borrowed_tails", FieldClass::FunctionFact),
    ("frame_protected", FieldClass::FunctionFact),
    ("collection_types", FieldClass::FunctionFact),
    ("sizeof_operands", FieldClass::FunctionFact),
    ("element_lowerings", FieldClass::ProbeState),
    ("descriptor_names", FieldClass::FunctionFact),
    ("cell_witness_names", FieldClass::FunctionFact),
    ("declared_callees", FieldClass::FunctionFact),
    ("in_place_names", FieldClass::FunctionFact),
    ("census_function", FieldClass::Census),
    ("census_bindings", FieldClass::Census),
    ("admit_derived_returns", FieldClass::FunctionFact),
    ("argument_probe", FieldClass::ProbeState),
    ("member_releases", FieldClass::FunctionFact),
    // The unit's definitions and call graph, read on first use.
    ("desugar", FieldClass::LazyCache),
    ("return_facts", FieldClass::FunctionFact),
    ("sub_object_roots", FieldClass::FunctionFact),
    ("callee_types", FieldClass::FunctionFact),
    ("list_facts", FieldClass::FunctionFact),
    // Pushed and popped around an iteration's body by the statement parser
    // (balanced, never inside a classification); a classification grows a
    // frame's protected names and links only.
    ("list_frames", FieldClass::MonotoneFact),
    ("caller_formals", FieldClass::FunctionFact),
    ("cell_protocols", FieldClass::FunctionFact),
    // Admitted and pending bindings are probe state; `over_formal` is a
    // monotone fact ([`OriginsLowering`]).
    ("element_origins", FieldClass::ProbeState),
];

/// The fields [`ParserProbeState`] carries, in its declaration order.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) const PROBE_STATE_FIELDS: &[&str] = &[
    "cursor",
    "next_site",
    "element_lowerings",
    "element_origins",
    "argument_probe",
    "typed_position_uses",
    "condition_widening",
    "variadic_tail_widening",
    "variadic_tail_readings",
    "typed_place_origin",
];

/// One snapshot of every [`FieldClass::ProbeState`] field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParserProbeState {
    cursor: usize,
    next_site: usize,
    element_lowerings: usize,
    element_origins: OriginsLowering,
    argument_probe: ArgumentProbe,
    typed_position_uses: usize,
    condition_widening: u8,
    variadic_tail_widening: bool,
    variadic_tail_readings: Option<Vec<TailReading>>,
    typed_place_origin: Option<(usize, usize)>,
    /// Not restored (monotone); checked: a probe never leaves a frame pushed.
    list_frame_count: usize,
    /// The thread's contract consultation log (rule 13f), restored on decline.
    consultations: Option<Consulted>,
    reading_site_checkpoint: Option<crate::contract_dependencies::ReadingCheckpoint>,
}

/// A probe's answer: kept (its state moves stand) or declined (the state is
/// handed back before the value is returned).
pub(super) enum Probe<T> {
    Keep(T),
    Decline(T),
}

impl Parser<'_> {
    /// The probe state now. The destructuring names every field of the
    /// parser, so a field added without a class does not compile.
    pub(super) fn probe_state(&self) -> ParserProbeState {
        let Parser {
            tokens: _,
            typed_path_index: _,
            typed_path_function: _,
            typed_position_uses,
            condition_widening,
            variadic_tail_widening,
            variadic_tail_readings,
            condition_cycles: _,
            typed_place_origin,
            // Rule 36: immutable source facts, like typed_path_index.
            exact_formal_returns: _,
            typed_root_declarations: _,
            readonly_list_count_certified: _,
            declared_no_return: _,
            call_graph_components: _,
            cursor,
            contracts: _,
            next_site,
            formal_parameters: _,
            fixed_formals: _,
            protected_formals: _,
            typed_model: _,
            typed_types: _,
            typed_bases: _,
            store_bases: _,
            alias_classes: _,
            variadic_tails: _,
            variadic_borrowed_tails: _,
            frame_protected: _,
            collection_types: _,
            sizeof_operands: _,
            element_lowerings,
            descriptor_names: _,
            cell_witness_names: _,
            declared_callees: _,
            in_place_names: _,
            #[cfg(feature = "indirect-census")]
                census_function: _,
            #[cfg(feature = "indirect-census")]
                census_bindings: _,
            admit_derived_returns: _,
            argument_probe,
            member_releases: _,
            desugar: _,
            return_facts: _,
            sub_object_roots: _,
            callee_types: _,
            list_facts: _,
            list_frames,
            caller_formals: _,
            cell_protocols: _,
            element_origins,
        } = self;
        ParserProbeState {
            cursor: *cursor,
            next_site: *next_site,
            element_lowerings: *element_lowerings,
            element_origins: element_origins.lowering(),
            argument_probe: argument_probe.clone(),
            typed_position_uses: typed_position_uses.get(),
            condition_widening: condition_widening.get(),
            variadic_tail_widening: variadic_tail_widening.get(),
            variadic_tail_readings: variadic_tail_readings.borrow().clone(),
            typed_place_origin: typed_place_origin.get(),
            list_frame_count: list_frames.len(),
            consultations: contract_consultation::snapshot(),
            reading_site_checkpoint: crate::contract_dependencies::reading_checkpoint(),
        }
    }

    /// Hand back `state`: every probe-state field becomes what it was.
    pub(super) fn restore_probe_state(&mut self, state: &ParserProbeState) {
        let ParserProbeState {
            cursor,
            next_site,
            element_lowerings,
            element_origins,
            argument_probe,
            typed_position_uses,
            condition_widening,
            variadic_tail_widening,
            variadic_tail_readings,
            typed_place_origin,
            list_frame_count,
            consultations,
            reading_site_checkpoint,
        } = state;
        debug_assert_eq!(self.list_frames.len(), *list_frame_count);
        self.cursor = *cursor;
        self.next_site = *next_site;
        self.element_lowerings = *element_lowerings;
        self.element_origins.restore_lowering(element_origins);
        self.argument_probe.clone_from(argument_probe);
        self.typed_position_uses.set(*typed_position_uses);
        self.condition_widening.set(*condition_widening);
        self.variadic_tail_widening.set(*variadic_tail_widening);
        self.variadic_tail_readings
            .borrow_mut()
            .clone_from(variadic_tail_readings);
        self.typed_place_origin.set(*typed_place_origin);
        contract_consultation::restore(consultations);
        crate::contract_dependencies::restore_reading_checkpoint(reading_site_checkpoint);
    }

    /// The scoped form: run `probe`; a [`Probe::Decline`] hands the state
    /// back before its value is returned, so no declining path can forget a
    /// field.
    pub(super) fn probe<T>(&mut self, probe: impl FnOnce(&mut Self) -> Probe<T>) -> T {
        let before = self.probe_state();
        match probe(self) {
            Probe::Keep(value) => value,
            Probe::Decline(value) => {
                self.restore_probe_state(&before);
                value
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_probe_state.rs"]
pub(super) mod tests;
