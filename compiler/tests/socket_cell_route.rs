//! Card `CT42-CELL-ROUTE` (wave 39): the socket protocol cell routed from
//! source through `disjoint NAME NAME;`, end to end through `compile_files`.
//!
//! Every refusal pins its exact ordered code vector, never `is_err()`, and the
//! shared-cell refusal is asserted to be the **unedited** `DisjointFootprintsV1`
//! checker's own `SharedProtocolCell` (`Q664`), reported in the `Concurrency`
//! pass's band by condition extension — no code was allocated.

use qcore_compiler::{
    CheckedSourceModule, CompileOptions, SemanticPass, SourceExecutionProfile, SourceFile,
    compile_files,
};
use qcore_core::disjoint_footprint::FootprintAssumption;

const TCP_ECHO: &str = include_str!("../../examples/sockets/tcp_echo.qcore");
const UDP_ECHO: &str = include_str!("../../examples/sockets/udp_echo.qcore");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("socket_cell_route.qcore", source)],
        &CompileOptions::default(),
    )
}

fn refused(source: &str) -> Vec<qcore_compiler::FileDiagnostic> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
}

fn codes(source: &str) -> Vec<String> {
    refused(source)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// `TCP_ECHO` with `marker` replaced by `line`, so a question can be placed at
/// a known statement while every other byte of the published example stays.
fn echo_with(marker: &str, line: &str) -> String {
    assert_eq!(TCP_ECHO.matches(marker).count(), 1, "{marker} must occur once");
    TCP_ECHO.replace(marker, line)
}

const AFTER_ACCEPT: &str = "    let server = accept listener;\n";

// --- The case: the two ends of one accepted connection. ---

#[test]
fn the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664() {
    let source = echo_with(
        AFTER_ACCEPT,
        "    let server = accept listener;\n    disjoint client server;\n",
    );
    let diagnostics = refused(&source);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["Q664"],
        "{diagnostics:?}"
    );
    let refusal = &diagnostics[0];
    // The code is the footprint checker's own band, so the pass is that
    // band's owner (§L21: `Q650`–`Q667` belong to `Concurrency`).
    assert_eq!(refusal.pass, SemanticPass::Concurrency);
    assert!(
        refusal.message.contains("`client`")
            && refusal.message.contains("`server`")
            && refusal.message.contains("SharedProtocolCell"),
        "{}",
        refusal.message
    );
    // Attributed to the question, not to the accept or the entry.
    assert_eq!(refusal.location.line_source.trim(), "disjoint client server;");
}

#[test]
fn the_question_is_symmetric_and_reflexive() {
    for question in ["disjoint server client;", "disjoint client client;"] {
        let source = echo_with(
            AFTER_ACCEPT,
            &format!("    let server = accept listener;\n    {question}\n"),
        );
        assert_eq!(codes(&source), ["Q664"], "{question}");
    }
}

// --- The admitted side: distinct cells. ---

#[test]
fn a_listener_and_an_end_are_admitted_and_the_question_erases_from_emission() {
    let asked = echo_with(
        AFTER_ACCEPT,
        "    let server = accept listener;\n    disjoint client listener;\n",
    );
    // The twin: the same bytes with the question's line blank, so the two
    // programs differ in exactly one statement.
    let twin = echo_with(AFTER_ACCEPT, "    let server = accept listener;\n\n");
    let asked_module = compile(&asked).unwrap_or_else(|diagnostics| {
        panic!("a listener shares no connection cell: {diagnostics:?}")
    });
    let twin_module = compile(&twin).expect("the published example is admitted");
    assert_eq!(
        asked_module.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    assert_eq!(
        asked_module.c_source, twin_module.c_source,
        "the question erases: the emitted C is byte-identical with and without it"
    );
    assert_eq!(
        asked_module.extern_observation, twin_module.extern_observation,
        "and so is the reference observation"
    );
    let CheckedSourceModule::StandardSocket(module) = &asked_module.checked else {
        panic!("the socket route publishes a socket module");
    };
    assert_eq!(module.cells.questions.len(), 1);
    assert_eq!(
        (
            module.cells.questions[0].left.as_str(),
            module.cells.questions[0].right.as_str()
        ),
        ("client", "listener")
    );
    assert_eq!(module.cells.footprint.admitted_disjointness.len(), 1);
    assert_eq!(
        module.cells.cells.connections,
        vec![("client".to_owned(), "server".to_owned())]
    );
    // The T09 edit: a derived-cell certificate carries no declared-identity
    // premise (`CT42-CELL-DERIVED-PREMISE`).
    assert!(
        !module
            .cells
            .footprint
            .assumptions
            .contains(&FootprintAssumption::DeclaredProtocolCellIdentity),
        "{:?}",
        module.cells.footprint.assumptions
    );
    // And the twin, which asks nothing, still publishes its cells.
    let CheckedSourceModule::StandardSocket(twin_checked) = &twin_module.checked else {
        panic!("the socket route publishes a socket module");
    };
    assert!(twin_checked.cells.questions.is_empty());
    assert_eq!(twin_checked.cells.cells, module.cells.cells);
}

#[test]
fn the_ends_of_two_different_connections_are_admitted() {
    let source = "\
import std.socket;

fn main() -> unit effects { io, alloc } {
    let listener = tcp;
    bind listener;
    listen listener;
    let client_one = tcp;
    connect client_one listener;
    let server_one = accept listener;
    let client_two = tcp;
    connect client_two listener;
    let server_two = accept listener;
    disjoint client_one server_two;
    disjoint server_one client_two;
    disjoint client_one client_two;
    close server_one;
    close client_one;
    close server_two;
    close client_two;
    close listener;
    return;
}
";
    let module = compile(source).unwrap_or_else(|diagnostics| {
        panic!("two connections are two cells: {diagnostics:?}")
    });
    let CheckedSourceModule::StandardSocket(module) = &module.checked else {
        panic!("the socket route publishes a socket module");
    };
    assert_eq!(module.cells.questions.len(), 3);
    assert_eq!(module.cells.footprint.admitted_disjointness.len(), 3);
    assert_eq!(module.cells.cells.connections.len(), 2);
    // The control on the same program: either connection's own pair refuses.
    let refused = source.replace(
        "    disjoint client_one server_two;\n",
        "    disjoint client_two server_two;\n",
    );
    assert_eq!(codes(&refused), ["Q664"]);
}

/// `CT42-CELL-UDP-QUEUE`, re-affirmed with its ground: a datagram pair shares
/// no connection phase — either end closes independently and no operation
/// through one name is refused on the other's protocol state — so two datagram
/// sockets really are independent custodians, and the question is admitted.
#[test]
fn a_datagram_pair_is_admitted_as_disjoint() {
    let marker = "    let server = udp;\n";
    assert_eq!(UDP_ECHO.matches(marker).count(), 1, "{UDP_ECHO}");
    let source = UDP_ECHO.replace(
        marker,
        "    let server = udp;\n    disjoint client server;\n",
    );
    let module = compile(&source).unwrap_or_else(|diagnostics| {
        panic!("a datagram pair is not a cell: {diagnostics:?}")
    });
    assert_eq!(module.execution_profile, SourceExecutionProfile::StandardSocketV1);
    let CheckedSourceModule::StandardSocket(module) = &module.checked else {
        panic!("the socket route publishes a socket module");
    };
    assert!(module.cells.cells.connections.is_empty());
    assert_eq!(module.cells.footprint.admitted_disjointness.len(), 1);
}

// --- The shape and custody refusals around the question. ---

#[test]
fn a_question_naming_no_socket_is_the_transport_checkers_q081() {
    let source = echo_with(
        AFTER_ACCEPT,
        "    let server = accept listener;\n    let count = 1;\n    disjoint count client;\n",
    );
    assert_eq!(codes(&source), ["Q081"]);
}

#[test]
fn a_question_asked_of_a_closed_socket_is_refused_by_the_inherited_checkers() {
    let marker = "    close server;\n    close client;\n";
    let source = echo_with(marker, "    close server;\n    disjoint client server;\n    close client;\n");
    let diagnostics = refused(&source);
    let codes: Vec<&str> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    // The inherited ownership checker's use-after-close (`Q083`) and the
    // transport half's own closed-socket stop (`Q082`), both from checkers
    // this card did not edit; the footprint checker is never reached.
    assert_eq!(codes, ["Q083", "Q082"], "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.pass == SemanticPass::StandardSocket)
    );
}

#[test]
fn a_malformed_question_is_q080() {
    for question in ["disjoint client;", "disjoint client server listener;", "disjoint;"] {
        let source = echo_with(
            AFTER_ACCEPT,
            &format!("    let server = accept listener;\n    {question}\n"),
        );
        assert_eq!(codes(&source), ["Q080"], "{question}");
    }
}

#[test]
fn a_question_in_a_helper_is_refused_as_a_construct_outside_the_entry() {
    // The names are bound so name resolution has nothing to say; what refuses
    // is the route's own rule that every socket construct is in the entry.
    let source = format!(
        "{TCP_ECHO}\nfn helper() -> unit {{\n    let a = 1;\n    let b = 2;\n    \
         disjoint a b;\n    return;\n}}\n"
    );
    let diagnostics = refused(&source);
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code == "Q080"
            && diagnostic.message.contains("models the entry only")),
        "{diagnostics:?}"
    );
}

#[test]
fn the_question_needs_the_socket_module() {
    // The question is a socket use, so a module spelling it without
    // `import std.socket;` takes the import gate every other socket use takes.
    let source = TCP_ECHO.replace("import std.socket;\n", "");
    let with_question = echo_with(
        AFTER_ACCEPT,
        "    let server = accept listener;\n    disjoint client listener;\n",
    )
    .replace("import std.socket;\n", "");
    assert_eq!(codes(&source), codes(&with_question));
    assert!(codes(&source).contains(&"Q015".to_owned()), "{:?}", codes(&source));
}

#[test]
fn every_published_socket_example_still_compiles_and_publishes_its_cells() {
    for (name, source) in [("tcp_echo", TCP_ECHO), ("udp_echo", UDP_ECHO)] {
        let module = compile(source).unwrap_or_else(|diagnostics| {
            panic!("{name} must be admitted: {diagnostics:?}");
        });
        let CheckedSourceModule::StandardSocket(module) = &module.checked else {
            panic!("{name}: the socket route publishes a socket module");
        };
        assert!(module.cells.questions.is_empty(), "{name}");
        assert!(
            !module
                .cells
                .footprint
                .assumptions
                .contains(&FootprintAssumption::DeclaredProtocolCellIdentity),
            "{name}"
        );
    }
}
