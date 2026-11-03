//! Card GAPO-RETURN-CORRECTNESS (gateway rule 15x): the probe state's field
//! list is pinned against the parser's declaration, and a declined probe
//! leaves every probe-state field as it found it.

use std::collections::BTreeSet;

use super::super::{LegacyInstruction, Parser, ResourceContracts, Token, list_iteration};
use super::{FieldClass, PARSER_FIELDS, PROBE_STATE_FIELDS, Probe};

/// The field names of `struct NAME … {` in `source`, in declaration order.
fn declared_fields(source: &str, header: &str) -> Vec<String> {
    let source = source.replace("\r\n", "\n");
    let start = source.find(header).expect("the struct is declared");
    let end = start + source[start..].find("\n}\n").expect("the struct closes");
    source[start..end]
        .lines()
        .filter_map(|line| {
            let field = line.strip_prefix("    ")?;
            let (name, _) = field.split_once(':')?;
            (!name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
                .then(|| name.to_owned())
        })
        .collect()
}

#[test]
fn every_parser_field_is_classified_in_declaration_order() {
    let declared = declared_fields(include_str!("../../src/legacy_c.rs"), "struct Parser<'a> {");
    let classified = PARSER_FIELDS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect::<Vec<_>>();
    assert_eq!(declared, classified);
}

#[test]
fn the_snapshot_carries_exactly_the_probe_state_class() {
    let snapshot = declared_fields(
        include_str!("../../src/legacy_probe_state.rs"),
        "pub(super) struct ParserProbeState {",
    );
    assert_eq!(
        snapshot,
        PROBE_STATE_FIELDS
            .iter()
            .map(|name| (*name).to_owned())
            .chain([
                "list_frame_count".to_owned(),
                "consultations".to_owned(),
                "reading_site_checkpoint".to_owned()
            ])
            .collect::<Vec<_>>()
    );
    let class = PARSER_FIELDS
        .iter()
        .filter(|(_, class)| *class == FieldClass::ProbeState)
        .map(|(name, _)| *name)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        class,
        PROBE_STATE_FIELDS.iter().copied().collect::<BTreeSet<_>>()
    );
    // The pinned list itself (a change here is a reviewed decision).
    assert_eq!(
        PROBE_STATE_FIELDS,
        [
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
        ]
    );
    let monotone = PARSER_FIELDS
        .iter()
        .filter(|(_, class)| *class == FieldClass::MonotoneFact)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    assert_eq!(monotone, ["list_frames"]);
}

fn contracts() -> ResourceContracts {
    let sources = [
        (
            "q_base.qcore",
            include_str!("../fixtures/contracts/q_base.qv"),
        ),
        (
            "q_realloc.qcore",
            include_str!("../fixtures/contracts/q_realloc.qv"),
        ),
    ]
    .into_iter()
    .map(|(name, text)| (name.to_owned(), text.to_owned()))
    .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).expect("fixtures load")
}

fn tokens(text: &str) -> Vec<Token> {
    text.split_whitespace()
        .map(|text| Token {
            text: text.to_owned(),
            line: 1,
        })
        .collect()
}

/// A parser with no declaration facts and no Clang index (the typed index is
/// already answered `None`, so no route launches Clang).
pub(in crate::legacy_c) fn bare<'a>(
    stream: &'a [Token],
    contracts: &'a ResourceContracts,
) -> Parser<'a> {
    Parser {
        tokens: stream,
        typed_path_index: std::cell::OnceCell::from(None),
        typed_path_function: "subject".to_owned(),
        typed_position_uses: std::cell::Cell::new(1),
        condition_widening: std::cell::Cell::new(0),
        variadic_tail_widening: std::cell::Cell::new(false),
        variadic_tail_readings: std::cell::RefCell::new(None),
        condition_cycles: Default::default(),
        typed_place_origin: std::cell::Cell::new(None),
        exact_formal_returns: std::cell::OnceCell::new(),
        typed_root_declarations: std::cell::OnceCell::new(),
        readonly_list_count_certified: std::cell::OnceCell::new(),
        declared_no_return: std::cell::OnceCell::new(),
        call_graph_components: std::cell::OnceCell::new(),
        cursor: 0,
        contracts,
        next_site: 3,
        fixed_formals: Default::default(),
        formal_parameters: BTreeSet::new(),
        protected_formals: BTreeSet::new(),
        typed_model: Default::default(),
        typed_types: Default::default(),
        typed_bases: Default::default(),
        store_bases: Default::default(),
        alias_classes: Default::default(),
        variadic_tails: BTreeSet::new(),
        variadic_borrowed_tails: BTreeSet::new(),
        frame_protected: BTreeSet::new(),
        collection_types: Default::default(),
        sizeof_operands: Default::default(),
        element_lowerings: 2,
        descriptor_names: Default::default(),
        cell_witness_names: Default::default(),
        result_witness_names: Default::default(),
        declared_callees: BTreeSet::new(),
        in_place_names: Default::default(),
        #[cfg(feature = "indirect-census")]
        census_function: "subject".to_owned(),
        #[cfg(feature = "indirect-census")]
        census_bindings: Default::default(),
        admit_derived_returns: false,
        argument_probe: Default::default(),
        member_releases: Default::default(),
        desugar: super::super::condition_desugar::DesugarState::new("subject"),
        return_facts: Default::default(),
        sub_object_roots: Default::default(),
        callee_types: Default::default(),
        list_facts: Default::default(),
        list_frames: Vec::new(),
        caller_formals: Default::default(),
        cell_protocols: Default::default(),
        element_origins: Default::default(),
    }
}

/// The W4, B01 and B01-opaque probes of a refused `return q_realloc(p, n)`
/// (a `Reallocated` result W4 never admits) all decline; the classification
/// they ask moves the state and consults `q_realloc`, and every declined
/// probe hands all of it back (the consultation log included).
#[test]
fn declined_return_probes_hand_back_the_whole_state() {
    let contracts = contracts();
    let stream = tokens("int subject ( void * p , int n ) { return q_realloc ( p , n ) ; }");
    let expression = tokens("q_realloc ( p , n )");
    let mut parser = bare(&stream, &contracts);
    let enclosing = qcore_schemas::contract_consultation::open();
    let before = parser.probe_state();
    assert_eq!(parser.return_expression_instruction(&expression, 1), None);
    assert_eq!(parser.probe_state(), before);
    let after_declined = qcore_schemas::contract_consultation::close(enclosing);
    assert!(after_declined.is_empty(), "{after_declined:?}");
    // Non-vacuous: the probe's own question mints a site and consults `q_realloc`.
    let enclosing = qcore_schemas::contract_consultation::open();
    let mut synthetic = super::super::return_call::assignment_prefix(1).to_vec();
    synthetic.extend(expression.iter().cloned());
    let asked = parser.classify_simple(&synthetic);
    let kept = qcore_schemas::contract_consultation::close(enclosing);
    assert!(
        matches!(asked, LegacyInstruction::ContractCall { .. }),
        "{asked:?}"
    );
    assert!(kept.found.contains("q_realloc"), "{kept:?}");
    assert_ne!(parser.probe_state(), before);
}

/// The scoped form: a declined probe restores every probe-state field and the
/// consultation log whatever it changed; a kept one keeps them; a monotone
/// fact (a list frame's protected names) is kept either way.
#[test]
fn the_scoped_probe_restores_only_on_decline_and_keeps_monotone_facts() {
    let contracts = contracts();
    let stream = tokens("int subject ( void ) { }");
    let mut parser = bare(&stream, &contracts);
    parser.list_frames.push(list_iteration::Frame {
        kind: list_iteration::IterationKind::Plain,
        cursor: "c".to_owned(),
        successor: None,
        protected: BTreeSet::from(["c".to_owned()]),
        links: BTreeSet::new(),
        head: vec!["h".to_owned()],
    });
    let enclosing = qcore_schemas::contract_consultation::open();
    let before = parser.probe_state();
    let disturb = |parser: &mut Parser<'_>| {
        parser.cursor += 7;
        parser.next_site += 1;
        parser.element_lowerings += 1;
        parser.element_origins.propose("e", None);
        parser.typed_position_uses.set(9);
        parser.condition_widening.set(3);
        parser.variadic_tail_widening.set(true);
        *parser.variadic_tail_readings.borrow_mut() = Some(Vec::new());
        parser.typed_place_origin.set(Some((4, 2)));
        parser.list_frames[0].protected.insert("e".to_owned());
        qcore_schemas::contract_consultation::note("probed_callee", true);
    };
    let answer = parser.probe(|parser| {
        disturb(parser);
        Probe::Decline(17)
    });
    assert_eq!(answer, 17);
    assert_eq!(parser.probe_state(), before);
    assert!(parser.list_frames[0].protected.contains("e"));
    parser.probe(|parser| {
        disturb(parser);
        Probe::Keep(())
    });
    assert_ne!(parser.probe_state(), before);
    let logged = qcore_schemas::contract_consultation::close(enclosing);
    assert!(logged.found.contains("probed_callee"));
}

#[test]
fn actual_nested_capture_is_removed_by_declined_probe_and_late_instantiation_refusal() {
    use crate::contract_dependencies::{Recording, primary_source};
    use qcore_schemas::contract::{
        ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    };
    let mut contracts = crate::contract_loader::load_resource_contract_sources(
        &std::collections::BTreeMap::from([(
            "errno.qv".to_owned(),
            include_str!("../../../../providers/quirin-provider-libc/contracts/libc/errno.qv")
                .to_owned(),
        )]),
    )
    .unwrap();
    contracts.insert_contract(
        "consume",
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None),
    );
    let source = "char *strerror(int n);\nvoid consume(char *p);\nvoid subject(void) { consume(strerror(1)); }\n";
    let stream = crate::legacy_c::tokenize(source);
    let expression = crate::legacy_c::tokenize("consume(strerror(1))");
    // Retain the actual primary statement's line identity.
    let start = stream
        .windows(expression.len())
        .position(|w| w.iter().zip(&expression).all(|(a, b)| a.text == b.text))
        .unwrap();
    let statement = &stream[start..start + expression.len()];
    let texts = stream.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let setup = || {
        let mut parser = bare(&stream, &contracts);
        parser.typed_model = crate::typed_field_access::TypedAccessModel::read(&texts);
        parser.typed_types = crate::typed_field_access::FunctionTypes::read(
            &parser.typed_model,
            &["void"],
            &[],
            &BTreeSet::new(),
        );
        parser
            .declared_callees
            .extend(["consume".to_owned(), "strerror".to_owned()]);
        parser
    };
    let mut parser = setup();
    let record = Recording::start();
    primary_source(source);
    let accepted = parser.classify_simple(statement);
    assert!(
        matches!(accepted, LegacyInstruction::ContractCall { .. }),
        "{accepted:?}"
    );
    assert_eq!(
        record.finish().reading_sites.len(),
        1,
        "non-vacuous accepted nested capture"
    );

    let mut parser = setup();
    let record = Recording::start();
    primary_source(source);
    parser.probe(|p| {
        p.classify_simple(statement);
        Probe::Decline(())
    });
    assert!(record.finish().reading_sites.is_empty());

    let mut parser = setup();
    let record = Recording::start();
    primary_source(source);
    let refused = parser.instantiate_call(
        "consume",
        statement,
        Some("wrong_destination"),
        crate::call_instantiation::ResultUse::Discarded,
    );
    assert!(matches!(
        refused,
        Err(super::super::call_instantiation_route::CallRefusal::UnexpectedDestination)
    ));
    assert!(record.finish().reading_sites.is_empty());
}
