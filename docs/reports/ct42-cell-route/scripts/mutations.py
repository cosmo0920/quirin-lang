#!/usr/bin/env python3
"""CT42-CELL-ROUTE mutation harness (shipped; `W36-W37-MUTATION-HARNESSES-UNSHIPPED`
is why). Every arm this card added is mutated, the named test is shown RED,
the original bytes are written back (never an mtime-preserving copy:
`MUTATION-HARNESS-MTIME-RESTORE`) and the round trip is asserted sha256-equal.
Every verdict is appended to the log named on the command line.

    python3 docs/reports/ct42-cell-route/scripts/mutations.py LOG

Run from the worktree root. Each mutation runs exactly the test it names, so a
RED verdict is that test's and nobody else's.
"""
import hashlib
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path.cwd()
FOOTPRINT = ROOT / "crates/qcore-core/src/socket_footprint.rs"
T09 = ROOT / "crates/qcore-core/src/disjoint_footprint.rs"
SOCKET = ROOT / "crates/qcore-core/src/std_socket.rs"
EMIT = ROOT / "crates/qcore-core/src/std_socket_c_emit.rs"
DRIVER = ROOT / "crates/qcore-verification/src/driver.rs"
READER = ROOT / "crates/qcore-frontend/src/source_socket.rs"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def log(path, line):
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    print(line, flush=True)


def cargo(args):
    started = time.time()
    finished = subprocess.run(["cargo", "test", *args], capture_output=True, text=True)
    text = finished.stdout + finished.stderr
    failed = sorted(
        {
            line.split("test ", 1)[1].split(" ...", 1)[0]
            for line in text.splitlines()
            if line.startswith("test ") and line.rstrip().endswith("FAILED")
        }
    )
    compile_error = "error[E" in text or "error: could not compile" in text
    return {
        "returncode": finished.returncode,
        "failed": failed,
        "compile_error": compile_error,
        "seconds": round(time.time() - started, 1),
    }


def mutate(logfile, label, path, old, new, args, expect_test):
    original = path.read_text(encoding="utf-8")
    before = sha(path)
    assert original.count(old) == 1, (label, old)
    path.write_text(original.replace(old, new), encoding="utf-8")
    try:
        verdict = cargo(args)
    finally:
        path.write_text(original, encoding="utf-8")
        assert sha(path) == before, label
    red = verdict["returncode"] != 0
    named = expect_test in verdict["failed"]
    ok = red and named and not verdict["compile_error"]
    log(
        logfile,
        f"{label}: {'RED' if red else 'GREEN'} ({'as required' if ok else 'NOT AS REQUIRED'}); "
        f"named test {'failed' if named else 'did not fail'}; "
        f"compile_error={verdict['compile_error']}; failing tests: "
        f"{', '.join(verdict['failed']) or '-'}; {verdict['seconds']}s; restored sha256-equal",
    )
    return ok


def baseline(logfile):
    verdict = cargo(["-p", "qcore-core", "--lib", "socket_footprint::"])
    verdict2 = cargo(["-p", "qcore-compiler", "--test", "socket_cell_route"])
    ok = verdict["returncode"] == 0 and verdict2["returncode"] == 0
    log(
        logfile,
        f"M0 baseline (unmutated): core socket_footprint {'GREEN' if verdict['returncode'] == 0 else 'RED'}, "
        f"socket_cell_route {'GREEN' if verdict2['returncode'] == 0 else 'RED'} "
        f"({'as required' if ok else 'NOT AS REQUIRED'})",
    )
    return ok


CORE = lambda name: ["-p", "qcore-core", "--lib", f"socket_footprint::tests::{name}", "--", "--exact"]
ROUTE = lambda name: ["-p", "qcore-compiler", "--test", "socket_cell_route", name, "--", "--exact"]

MUTATIONS = [
    (
        "M1 deriver accept arm back to a singleton cell (the wave-31 admitting arm)",
        FOOTPRINT,
        """                    _ => {
                        return Err(SocketCellError::UnpairedAccept {
                            event: index,
                            binding: binding.clone(),
                            listener: listener.clone(),
                        });
                    }""",
        """                    _ => acquire(&mut entries, binding, None),""",
        CORE("an_unqueued_accept_is_refused_not_given_a_singleton_cell"),
        "socket_footprint::tests::an_unqueued_accept_is_refused_not_given_a_singleton_cell",
    ),
    (
        "M2 deriver connect arm back to `continue` (the wave-31 admitting arm)",
        FOOTPRINT,
        """                    return Err(SocketCellError::UnpairedConnect {
                        event: index,
                        client: client.clone(),
                    });""",
        """                    continue;""",
        CORE("an_unresolved_connect_is_refused_not_skipped"),
        "socket_footprint::tests::an_unresolved_connect_is_refused_not_skipped",
    ),
    (
        "M3 T09 stamps DeclaredProtocolCellIdentity for a derived alias too",
        T09,
        """                if declared {
                    assumptions.insert(FootprintAssumption::DeclaredProtocolCellIdentity);
                }""",
        """                if declared || true {
                    assumptions.insert(FootprintAssumption::DeclaredProtocolCellIdentity);
                }""",
        CORE("a_derived_projection_carries_no_declared_cell_identity_premise"),
        "socket_footprint::tests::a_derived_projection_carries_no_declared_cell_identity_premise",
    ),
    (
        "M4 the route asks no question (AssertDisjoint never pushed)",
        FOOTPRINT,
        """        footprint.events.push(FootprintEvent::AssertDisjoint {
            left: left_handle,
            right: right_handle,
        });
    }
    if !errors.is_empty() {""",
        """        let _ = (left_handle, right_handle);
    }
    if !errors.is_empty() {""",
        ROUTE("the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664"),
        "the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664",
    ),
    (
        "M5 the question reads neither custodian (no Read custody op)",
        SOCKET,
        """        SocketStatement::Disjoint { left, right } => {
            for name in [left, right] {
                if names(name) {
                    operations.push(OwnershipOp::Read(name.clone()));
                }
            }
        }""",
        """        SocketStatement::Disjoint { .. } => {}""",
        ROUTE("a_question_asked_of_a_closed_socket_is_refused_by_the_inherited_checkers"),
        "a_question_asked_of_a_closed_socket_is_refused_by_the_inherited_checkers",
    ),
    (
        "M6 the question emits a line instead of erasing",
        EMIT,
        """            SocketStatement::Disjoint { .. } => {}""",
        """            SocketStatement::Disjoint { .. } => {
                let _ = writeln!(source, "    /* disjoint */");
            }""",
        ROUTE("a_listener_and_an_end_are_admitted_and_the_question_erases_from_emission"),
        "a_listener_and_an_end_are_admitted_and_the_question_erases_from_emission",
    ),
    (
        "M7 the footprint refusal reported under the socket pass, not the band's owner",
        DRIVER,
        """                    let pass = if error.is_footprint_refusal() {
                        SemanticPass::Concurrency
                    } else {
                        SemanticPass::StandardSocket
                    };""",
        """                    let pass = SemanticPass::StandardSocket;""",
        ROUTE("the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664"),
        "the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664",
    ),
    (
        "M8 the reader admits a question with more than two names",
        READER,
        """            (Some(left), Some(right)) if ends_with_semicolon(window, 3) => {""",
        """            (Some(left), Some(right)) => {""",
        ROUTE("a_malformed_question_is_q080"),
        "a_malformed_question_is_q080",
    ),
    (
        "M9 the HTTP composition checks the cells of an empty transport (the pre-fix admitting gap)",
        DRIVER,
        """    let cells = qcore_core::socket_footprint::check_socket_cells(&transport.program).map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| {
                    let event = transport""",
        """    let cells = qcore_core::socket_footprint::check_socket_cells(
        &qcore_core::std_socket::SocketProgram {
            statements: Vec::new(),
            ..transport.program.clone()
        },
    )
    .map_err(
        |errors| {
            errors
                .into_iter()
                .map(|error| {
                    let event = transport""",
        ROUTE("an_http_exchanges_two_ends_are_refused_as_one_cell_at_q664"),
        "an_http_exchanges_two_ends_are_refused_as_one_cell_at_q664",
    ),
]


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    logfile = ROOT / sys.argv[1]
    log(logfile, f"# CT42-CELL-ROUTE mutations, {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}")
    head = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    log(logfile, f"# HEAD {head}")
    results = [baseline(logfile)]
    for label, path, old, new, args, test in MUTATIONS:
        results.append(mutate(logfile, label, path, old, new, args, test))
    log(logfile, f"# {sum(results)} of {len(results)} verdicts as required")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
