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

//! `source.event.v1` × the source line vocabulary — card `COMPOSE-EVENT-LINES`.
//!
//! # What these tests are about
//!
//! Three things, and they are separate.
//!
//! The **composition**: an event module may now carry `print`, `log` and
//! `flush`, and one value form — `let NAME = dispatch ready E;`, the byte that
//! turn's callback drained. The line grammar is `source.std-io.v1`'s, read by
//! its own reader and rendered by its own emitter, so a line here is that
//! route's line.
//!
//! The **boundary**, which is not weakened by any of it: `Q061`'s two-member
//! refusal, the declared-readiness rule and the `write`-interest check-only arm
//! all answer exactly as they did, with a line present or not.
//!
//! The **spelling**, which is the user's own acceptance rule: every line in
//! `examples/events` is a `|>` pipeline. A statement-form line still compiles —
//! the desugaring produces one — but no published example writes one.
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const WAKE_PAYLOAD: &str = include_str!("../../examples/events/wake_payload.qr");
const SELF_PIPE: &str = include_str!("../../examples/events/self_pipe.qr");
const README: &str = include_str!("../../examples/events/README.md");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("compose_event_lines.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn diagnostics(source: &str) -> Vec<qcore_compiler::FileDiagnostic> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
}

fn codes(source: &str) -> Vec<String> {
    diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn message(source: &str) -> String {
    diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A module with the four imports this route's composed vocabulary needs and
/// `body` as its statement sequence.
fn with_body(body: &str) -> String {
    format!(
        "#import std.event;\n#import std.logger;\n#import std.io;\n#import std.fmt;\n\n\
         fn main() -> i32 effects {{ io }} {{\n{body}\n}}\n"
    )
}

fn observation(source: &str) -> Vec<String> {
    compile(source)
        .expect("this fixture must compile")
        .extern_observation
        .expect("an executable module publishes a reference observation")
}

// --- The composition. ---

#[test]
fn a_composed_event_module_keeps_the_event_route() {
    // The composition is *inside* `source.event.v1`, not a route beside it: the
    // custody half is still the reactor model's and the profile a reader sees
    // is still this one. That is the `source.buffer.v1` shape, whose lines are
    // likewise its own route's.
    let module = compile(WAKE_PAYLOAD).unwrap();
    assert_eq!(module.execution_profile, SourceExecutionProfile::EventV1);
    assert_eq!(module.entry.as_deref(), Some("main"));
    assert!(module.c_source.is_some());
}

#[test]
fn the_rendered_value_is_the_byte_the_wake_delivered() {
    // The number rendered is the one the model computed out of the readiness
    // ledger — 72 and 105, in the order the two wakes wrote them — and the
    // emitted program compares the byte it actually read against it. This is
    // `STD-FMT-BUFFER-READ`'s own standard, applied to an endpoint.
    let records = observation(WAKE_PAYLOAD);
    assert!(
        records.contains(&"err: [INFO] 72".to_owned()),
        "{records:?}"
    );
    assert!(
        records.contains(&"err: [INFO] 0x00000069".to_owned()),
        "{records:?}"
    );
}

#[test]
fn the_piped_line_and_its_hand_written_twin_are_the_same_program() {
    // The desugaring adds no semantics: the pipeline and the statement form
    // emit the same C, byte for byte, and publish the same observation. The
    // user's rule is therefore about the *spelling* of the examples and costs
    // the route nothing.
    let piped = compile(&with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a 72;
    let value = dispatch ready reader;
    value |> log info dec;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    ))
    .unwrap();
    let written = compile(&with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a 72;
    let value = dispatch ready reader;
    log info dec value;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    ))
    .unwrap();
    assert_eq!(piped.c_source, written.c_source);
    assert_eq!(piped.extern_observation, written.extern_observation);
}

#[test]
fn a_line_writes_no_trace_record_and_a_payload_reaches_no_trace() {
    // Two halves of one boundary. A source line is program output, not a
    // compiler observation, so it contributes no diagnostic event; and a
    // `wake`'s declared payload is a *value* rather than a reactor fact, so it
    // is absent from the trace. Together they are why every transcript
    // published before this card is byte-identical after it.
    let declared = observation(&with_body(
        "    log_level trace;
    event open;
    endpoints a b;
    watch reader a read;
    wake a 72;
    let value = dispatch ready reader;
    value |> log info dec;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    ));
    let plain = observation(&with_body(
        "    log_level trace;
    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    dispatch ready reader;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    ));
    let trace = |records: Vec<String>| {
        records
            .into_iter()
            .filter(|record| record.starts_with("out: "))
            .collect::<Vec<_>>()
    };
    assert_eq!(trace(declared), trace(plain));
}

#[test]
fn the_self_pipe_example_is_byte_identical_after_this_card() {
    // The published executable example of card `STD-EVENT`, unedited, and its
    // documented transcript with it. A composition that moved a byte of it
    // would have changed what the route means for a program that composes
    // nothing.
    let module = compile(SELF_PIPE).unwrap();
    assert_eq!(
        module.extern_observation.as_ref().unwrap(),
        &vec![
            "out: QCORE_EVENT_OPEN".to_owned(),
            "out: QCORE_EVENT_ENDPOINTS 0 1".to_owned(),
            "out: QCORE_EVENT_WATCH 0 0 READ".to_owned(),
            "out: QCORE_EVENT_WAKE 0".to_owned(),
            "out: QCORE_EVENT_DISPATCH 0".to_owned(),
            "out: QCORE_EVENT_DISPATCH IDLE".to_owned(),
            "out: QCORE_EVENT_UNWATCH 0".to_owned(),
            "out: QCORE_EVENT_CLOSE 0".to_owned(),
            "out: QCORE_EVENT_CLOSE 1".to_owned(),
            "out: QCORE_EVENT_TEARDOWN".to_owned(),
            "out: QCORE_EVENT_RESULT 0".to_owned(),
        ]
    );
}

#[test]
fn a_quiet_composed_program_writes_its_lines_and_nothing_else() {
    // The quiet-emission amendment, measured on this route: without
    // `log_level` the retained trace reaches the null sink, so the observation
    // is exactly the source-authored lines — and the emitted program's one
    // stdio route is `source.std-io.v1`'s own.
    let quiet = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a 72;
    let value = dispatch ready reader;
    value |> log info dec;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert_eq!(observation(&quiet), vec!["err: [INFO] 72".to_owned()]);
    let module = compile(&quiet).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(!source.contains("qcore_diagnostic_stdout"));
    assert!(source.contains("QCORE_IO_WRITE(stderr"));
    assert!(
        qcore_core::io_emit::direct_stdio_calls(source).is_empty(),
        "a quiet composed program must write through one route only"
    );
}

#[test]
fn the_emitted_program_verifies_the_byte_it_drained() {
    // The premise is checked, never trusted: a drained byte that is not the one
    // the checked model says this turn's wake delivered exits non-zero rather
    // than rendering a line no model computed.
    let module = compile(WAKE_PAYLOAD).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(source.contains("qcore_event_drain(qcore_endpoint[0], &qcore_payload)"));
    assert!(source.contains("if (qcore_payload != (unsigned char) 72u) { return 76; }"));
    assert!(source.contains("if (qcore_payload != (unsigned char) 105u) { return 76; }"));
    assert!(source.contains("qcore_event_wake(qcore_endpoint[1], (unsigned char) 72u)"));
}

#[test]
fn the_published_example_is_the_file_and_its_transcripts_are_its_own() {
    for line in README
        .split("```qcore")
        .nth(2)
        .expect("the README quotes the composed program")
        .split("```")
        .next()
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        assert!(
            WAKE_PAYLOAD.contains(line.trim()),
            "the README quotes a line the example does not contain: {line}"
        );
    }
    for record in observation(WAKE_PAYLOAD) {
        let (_, text) = record.split_once(": ").unwrap();
        assert!(
            README.contains(text),
            "the README does not publish the transcript line {text:?}"
        );
    }
}

// --- The boundary, unweakened. ---

#[test]
fn two_ready_registrations_still_refuse_with_a_line_present() {
    // `Q061` is not a rule about the spelling of a turn, and composing lines
    // into a module does not buy one a turn the ledger cannot decide.
    let source = with_body(
        "    event open;
    endpoints a b;
    endpoints c d;
    watch first a read;
    watch second c read;
    wake a 1;
    wake c 2;
    let value = dispatch ready first;
    value |> log info dec;
    unwatch first;
    unwatch second;
    close a;
    close b;
    close c;
    close d;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q061".to_owned()));
    assert!(
        message(&source).contains("registrations are ready at this dispatch turn"),
        "{}",
        message(&source)
    );
}

#[test]
fn an_owned_buffer_does_not_cross_into_an_event_module() {
    // The card brief asked for "event + buffer + stdio". Measured: it is two
    // compositions and not one. Route selection puts a module that both
    // dispatches and buffers **here**, before `source.buffer.v1`, and this route
    // keeps its refusal for the buffer statements — exactly the principle the
    // extern and standard-IO routes state about each other. Admitting one would
    // mean one entry carrying two custody models, the reactor's and the
    // ownership ledger's, which is a card of its own
    // (follow-on `EVENT-BUFFER-PAYLOAD`). The line half of the brief is what
    // landed, and the value it renders comes off the endpoint directly.
    let source = "#import std.event;\n#import std.buffer;\n#import std.logger;\n#import std.fmt;\n\n\
                  fn main() -> i32 effects { io, alloc } {\n    event open;\n    \
                  let b = bytes(4);\n    drop b;\n    event close;\n    return 0;\n}\n";
    assert!(
        codes(source).contains(&"Q058".to_owned()),
        "{:?}",
        codes(source)
    );
    assert!(
        message(source).contains("is not a statement `source.event.v1` models"),
        "{}",
        message(source)
    );
}

#[test]
fn binding_a_turn_this_program_did_not_induce_is_still_refused() {
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    let value = dispatch ready reader;
    value |> log info dec;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q061".to_owned()));
}

#[test]
fn a_binding_naming_no_registration_is_refused_before_the_turn_is_decided() {
    // The same measurement on the other side of the binding: a registration
    // name nothing binds is an unknown name, refused at `Q013` before the
    // determinism ledger is consulted at all. `Q059` is what a *bound* name
    // that is not a live registration receives — see the test below.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    let value = dispatch ready other;
    value |> log info dec;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert_eq!(codes(&source), vec!["Q013".to_owned()]);
}

#[test]
fn a_binding_on_a_registration_this_entry_unwatched_keeps_the_routes_own_refusal() {
    // A name the general resolver accepts and this route rejects: `reader` is
    // unwatched before the turn, so it is not watched at it. The binding form
    // receives that refusal exactly as the bare form does.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    unwatch reader;
    let value = dispatch ready reader;
    value |> log info dec;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert!(
        codes(&source).contains(&"Q058".to_owned()),
        "{:?}",
        codes(&source)
    );
    assert!(
        message(&source).contains("is not watched at this turn"),
        "{}",
        message(&source)
    );
}

// --- The composition's own refusals. ---

#[test]
fn rendering_a_custody_name_refuses_at_this_passes_own_code() {
    // The namespace wall, measured in the one direction a source program can
    // reach it: `a` **is** a name this entry binds — the general resolver is
    // satisfied — and it is an endpoint of the reactor model rather than a
    // value, so the rendering refuses at this pass's second band rather than
    // rendering a custody identity as though it were a number.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    a |> log info dec;
    dispatch ready reader;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q074".to_owned()), "{source}");
    assert!(
        message(&source).contains("is not a value this entry bound"),
        "{}",
        message(&source)
    );
}

#[test]
fn rendering_a_name_nothing_binds_refuses_at_name_resolution() {
    // Measured, and pinned as the measurement rather than as the design
    // expectation: an operand naming nothing at all is refused **generically**
    // by name resolution before this route is selected, exactly as card
    // `STD-MODULES` recorded for its own route (residual
    // `STD-FMT-DIAGNOSTIC`). The refusal is correct; only its specificity is
    // the residual.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    dispatch ready reader;
    value |> log info dec;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert_eq!(codes(&source), vec!["Q013".to_owned()]);
    assert!(
        message(&source).contains("unknown name `value`"),
        "{}",
        message(&source)
    );
}

#[test]
fn a_borrowed_render_refuses_and_names_what_it_would_need() {
    // An endpoint and a registration are custody names of the reactor model.
    // Rendering one needs that model to publish a live identity at the
    // statement, which this route does not do — the named follow-on is
    // `STD-FMT-DESCRIPTOR`, and the refusal says so rather than reporting an
    // unbound value.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    print dec borrow a;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q074".to_owned()));
    assert!(
        message(&source).contains("renders the model identity of a live resource"),
        "{}",
        message(&source)
    );
}

#[test]
fn a_binding_that_shadows_an_endpoint_refuses_before_this_route_sees_it() {
    // Also measured rather than assumed. The model has its own collision gate
    // (`Q074`, exercised directly in `crates/qcore-core/tests/unit/std_event.rs`),
    // but no source program reaches it: a `let` is an ordinary binding to the
    // general resolver, and an endpoint name is an ordinary binding too, so the
    // duplicate is refused at `Q012` first. The model gate is kept for the same
    // reason `EXTERN-F1`'s `resource_stored` is — it guards a public model API
    // a composing route could drive — and the unreachability is recorded, not
    // hidden.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    let a = dispatch ready reader;
    a |> log info dec;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert_eq!(codes(&source), vec!["Q012".to_owned()]);
    assert!(
        message(&source).contains("duplicate local binding `a`"),
        "{}",
        message(&source)
    );
}

#[test]
fn an_idle_turn_binds_nothing_and_no_other_let_has_a_spelling() {
    // The value vocabulary is exactly one form. An idle turn produces no byte,
    // and there is no second initialiser: a value this route did not compute
    // has no spelling here at all.
    for body in ["    let value = dispatch idle;", "    let value = 7;"] {
        let source = with_body(&format!(
            "    event open;\n{body}\n    event close;\n    return 0;"
        ));
        assert!(codes(&source).contains(&"Q074".to_owned()), "{body}");
        assert!(
            message(&source).contains("is the one binding `source.event.v1` models"),
            "{body}: {}",
            message(&source)
        );
    }
}

#[test]
fn a_payload_that_is_not_a_byte_is_refused_rather_than_truncated() {
    let source = with_body(
        "    event open;
    endpoints a b;
    wake a 256;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q058".to_owned()));
    assert!(
        message(&source).contains("delivers one byte, 0 through 255"),
        "{}",
        message(&source)
    );
}

#[test]
fn a_line_in_a_function_this_route_never_lowers_is_refused() {
    // The route models the entry. A `print` in a helper would otherwise be
    // neither emitted nor refused, which is the one shape a composition can
    // silently drop.
    let source = "#import std.event;\n#import std.io;\n#import std.fmt;\n\n\
                  fn helper() -> i32 {\n    print dec 1;\n    return 0;\n}\n\n\
                  fn main() -> i32 effects { io } {\n    event open;\n    event close;\n    \
                  return 0;\n}\n";
    assert!(codes(source).contains(&"Q058".to_owned()));
    assert!(
        message(source).contains("models the entry only"),
        "{}",
        message(source)
    );
}

#[test]
fn a_composed_line_still_needs_its_own_module() {
    // Each capability is gated by its own module and by no other. The event
    // vocabulary never grants `std.io`, and the import gate runs before route
    // selection, so the refusal names the module rather than the route.
    let source = "#import std.event;\n\nfn main() -> i32 effects { io } {\n    event open;\n    \
                  print dec 1;\n    event close;\n    return 0;\n}\n";
    assert_eq!(codes(source), vec!["Q015".to_owned()]);
    assert!(
        message(source).contains("#import std.io;"),
        "{}",
        message(source)
    );
}

// --- The native run. ---

/// Compile and run the composed example under ASan/UBSan (plus LSan where the
/// platform provides it) and compare **both** streams with the reference
/// observation: stdout carries the explicitly selected system trace, stderr the
/// source-authored lines.
///
/// The comparison is strict, and that is this card's own measurement rather
/// than a convention — see `crates/qcore-core/tests/unit/std_event.rs`, where
/// the declared `qcore.obs-reordering.v1` relation of each stream is asserted
/// to be strict because every record of an event program is written by the
/// entry's own thread.
#[test]
fn the_composed_program_reproduces_both_streams() {
    let module = compile(WAKE_PAYLOAD).unwrap();
    let source = module.c_source.as_deref().unwrap();
    let records = module.extern_observation.as_ref().unwrap();
    let stream = |prefix: &str| {
        records
            .iter()
            .filter_map(|record| record.strip_prefix(prefix).map(str::to_owned))
            .collect::<Vec<_>>()
    };
    let expected_out = stream("out: ");
    let expected_err = stream("err: ");
    assert!(!expected_out.is_empty());
    assert_eq!(expected_err.len(), 2);

    #[cfg(target_os = "windows")]
    // The established native-C harness selects the installed MSVC toolchain and
    // runs the AF_UNIX-first arm. This card did **not** run it: it is written so
    // that a Windows gate exercises the changed shim signature and the two
    // streams together, and residual `EVENT-V1-WINDOWS-RUN` is unmoved.
    let run = windows_c::run(source, None, &[]);

    #[cfg(not(target_os = "windows"))]
    let run = {
        use std::process::Command;

        let directory = std::env::temp_dir().join(format!(
            "qcore-compose-event-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
        let compile = Command::new(&compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-g",
                "-O0",
                "-fsanitize=address,undefined",
                "-fno-omit-frame-pointer",
            ])
            .arg(&generated)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary)
            .env(
                "ASAN_OPTIONS",
                if cfg!(target_os = "macos") {
                    // Apple's AddressSanitizer provides no LeakSanitizer, so leak
                    // detection stays a Linux-only observation.
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        run
    };
    // A non-zero exit is a sanitizer finding, a ready set that disagreed with
    // the declaration, or a drained byte that disagreed with the model. All
    // three are failures, deliberately.
    assert!(
        run.status.success(),
        "the composed program failed with {}:\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .collect::<Vec<_>>(),
        expected_out.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stderr)
            .lines()
            .collect::<Vec<_>>(),
        expected_err.iter().map(String::as_str).collect::<Vec<_>>()
    );
}
