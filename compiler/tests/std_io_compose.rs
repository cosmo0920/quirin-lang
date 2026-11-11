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

//! End-to-end route selection, admission and refusal for
//! `source.std-io-extern.v1` — the checked composition of
//! `source.extern-call.v1` and `source.std-io.v1` (card `STD-IO-COMPOSE`).
//!
//! Two things are pinned here that are easy to lose in a refactor:
//!
//! * **A module using one feature set keeps its own route.** Route selection is
//!   the conjunction of the two routes' own selectors, so a printing module and
//!   an extern-calling module fall through unchanged, and the tests below
//!   assert the profile *and* the emitted C of each.
//! * **Composing weakens nothing.** Every refusal either route gives a module
//!   is still given, under that route's own code, at that route's own site.
//!
//! The native run compares the emitted program's **stdout** with the reference
//! observation's stdout projection and its **stderr** with the stderr
//! projection. No order between the two streams is asserted anywhere, because
//! none is modeled — the boundary card `STD-MODULES` stated.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

const CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");
const CHAIN: &str = include_str!("fixtures/compose/chain.qr");
const HAND_WRITTEN: &str = include_str!("fixtures/compose/chain_hand_written.qr");
const LINES: &str = include_str!("fixtures/std_modules/lines.qr");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const BORROWED: &str = include_str!("fixtures/compose/borrowed_render.qr");

// Exercise both checkout styles regardless of the host's Git configuration.
fn with_line_endings(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}

fn selection() -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([(
        "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
}

fn compile_with(
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("compose.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: contracts,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_with(source, selection())
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// One entry body with the standard preamble around it.
fn module(body: &str) -> String {
    format!(
        "#import std.extern;\n\
         #import std.fmt;\n\
         #import std.io;\n\
         #import std.logger;\n\
         \n\
         error Failure {{ Failed; }}\n\
         \n\
         extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
         extern fn qcore_extern_fill(buffer: ptr<mut, u8>, value: u32) -> unit;\n\
         extern fn qcore_extern_sum(buffer: ptr<const, u8>, count: u32) -> i32;\n\
         extern fn qcore_extern_scale(value: i32, factor: i32) -> i32;\n\
         extern fn qcore_extern_widen(value: i32) -> i64;\n\
         extern fn qcore_extern_open(slot: ptr<mut, ptr<mut, u8>>, size: usize) -> i32;\n\
         extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\
         \n\
         fn main() -> unit effects {{ io, alloc, mut }} {{\n{body}    return;\n}}\n"
    )
}

// --- The accepted composition. ---

#[test]
fn a_module_that_calls_and_prints_selects_the_composed_profile() {
    for source in with_line_endings(CHAIN) {
        let compiled = compile(&source).unwrap();
        assert_eq!(
            compiled.execution_profile,
            SourceExecutionProfile::ComposedStdIoExternV1
        );
        assert_eq!(compiled.entry.as_deref(), Some("main"));
        // The composition is its own evidence: each certificate is one half of
        // a checked pair, so a consumer must ask for the composition and see
        // the other beside it (the `ComposedEpochTasks` precedent).
        assert!(compiled.checked.as_extern_calls().is_none());
        assert!(compiled.checked.as_standard_io().is_none());
        let checked = compiled
            .checked
            .as_composed()
            .expect("the checked artifact variant is explicit for this route");
        // Both projections' own certificates are published verbatim.
        assert_eq!(checked.certificate.externs.acquisitions, vec![1]);
        assert_eq!(checked.certificate.externs.consumptions, vec![1]);
        assert_eq!(checked.certificate.io.lines.len(), 4);
        // And the sequential passes ran over the extern half, exactly as they
        // do on the extern route.
        assert_eq!(checked.sequential.functions.len(), 1);
    }
}

#[test]
fn every_premise_of_the_extern_half_crosses_the_join_unchanged() {
    use qcore_compiler::extern_call::ExternAssumption;

    let compiled = compile(CHAIN).unwrap();
    let checked = compiled.checked.as_composed().unwrap();
    for expected in [
        ExternAssumption::NoExtentTransport,
        ExternAssumption::LinkageSuppliedByBuilder,
        ExternAssumption::DeclaredContract("qcore_extern_alloc".to_owned()),
        ExternAssumption::DeclaredContract("qcore_extern_scale".to_owned()),
    ] {
        assert!(
            checked.certificate.externs.assumptions.contains(&expected),
            "composing must not drop the premise {expected:?}"
        );
    }
    // Composing adds no premise of its own: the composed certificate carries
    // the extern projection's list and nothing beside it.
    let extern_only = compile(ROUND_TRIP).unwrap();
    let extern_only = extern_only.checked.as_extern_calls().unwrap();
    for premise in &extern_only.certificate.assumptions {
        if let ExternAssumption::DeclaredContract(name) = premise
            && name == "qcore_extern_scale"
        {
            continue;
        }
        assert!(
            checked
                .certificate
                .externs
                .assumptions
                .iter()
                .any(|carried| std::mem::discriminant(carried) == std::mem::discriminant(premise)),
            "the composed premise list must be of the same shape as the extern route's"
        );
    }
}

#[test]
fn the_composed_certificate_is_independently_rederivable() {
    use qcore_compiler::composed_std_io;

    let compiled = compile(CHAIN).unwrap();
    let checked = compiled.checked.as_composed().unwrap();
    composed_std_io::recheck_composed_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the driver publishes only evidence it rederived");

    // Each half is rechecked by its own recheck, so tampering with either is
    // caught: a composed publication that edited one sub-certificate is not a
    // composed publication.
    let mut tampered = checked.certificate.clone();
    tampered.externs.acquisitions.clear();
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered extern half must not survive the composed recheck"
    );
    let mut tampered = checked.certificate.clone();
    tampered.io.lines.clear();
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered standard-IO half must not survive the composed recheck"
    );
    let mut tampered = checked.certificate.clone();
    tampered.order.clear();
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered interleaving must not survive the composed recheck"
    );
    let mut tampered = checked.certificate.clone();
    tampered.rendered_values.clear();
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered rendered-value list must not survive the composed recheck"
    );
}

// --- The payoff: `value |> render |> log`. ---

#[test]
fn the_literal_chain_emits_its_hand_written_twins_c_byte_for_byte() {
    let chain = compile(CHAIN).unwrap();
    let hand = compile(HAND_WRITTEN).unwrap();
    assert_eq!(
        chain.c_source, hand.c_source,
        "a pipeline and its hand-written statement sequence must emit the same C11"
    );
    assert_eq!(
        chain.extern_observation, hand.extern_observation,
        "and observe identically"
    );
    // The temporary's name reaches neither the C nor the observation.
    let c = chain.c_source.as_deref().unwrap();
    assert!(
        !c.contains("qcore__pipe"),
        "the temporary is named by identity"
    );
}

#[test]
fn the_rendered_line_carries_the_number_the_extern_model_bound() {
    let compiled = compile(CHAIN).unwrap();
    let checked = compiled.checked.as_composed().unwrap();
    // `qcore_extern_fill(b, 7)` writes eight bytes of 7, so the declared sum is
    // 56 and the declared scale is 112. Both numbers are the source's, and the
    // emitted program refuses to go on if the real callee disagrees.
    assert_eq!(
        checked
            .certificate
            .rendered_values
            .iter()
            .map(|(_, number)| *number)
            .collect::<Vec<_>>(),
        vec![56, 112, 84]
    );
    let observation = compiled.extern_observation.as_deref().unwrap();
    assert!(observation.contains(&"out: 56".to_owned()));
    assert!(observation.contains(&"err: [INFO] 112".to_owned()));
    assert!(observation.contains(&"err: [WARN] 0x00000054".to_owned()));
}

#[test]
fn the_emitted_program_does_not_believe_its_declared_outcome() {
    let compiled = compile(CHAIN).unwrap();
    let c = compiled.c_source.as_deref().unwrap();
    // The same discipline the nullable-owned-return adapter already had: the
    // real result is compared and the program exits 70 rather than rendering a
    // number the run did not produce.
    assert!(c.contains("if (v1 != 56)"), "emitted C: {c}");
    assert!(c.contains("QCORE_DIAG_EXTERN_OUTCOME"));
    assert!(c.contains("return 70;"));
}

// --- Route selection: a module using one model keeps its own route. ---

#[test]
fn a_printing_module_keeps_the_standard_io_route_byte_for_byte() {
    let compiled = compile_with(LINES, None).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
    // The route's own emission is unmoved: it still declares its `s_NAME`
    // copies and still writes through `QCORE_IO_WRITE`.
    let c = compiled.c_source.as_deref().unwrap();
    assert!(c.contains("int32_t s_total = INT32_C(42);"));
    assert!(c.contains("QCORE_IO_WRITE(stdout, \"%\" PRId32 \"\\n\", s_total);"));
    assert!(!c.contains("source.std-io-extern.v1"));
}

#[test]
fn an_extern_calling_module_keeps_the_extern_route_byte_for_byte() {
    let compiled = compile(ROUND_TRIP).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
    let c = compiled.c_source.as_deref().unwrap();
    assert!(c.starts_with("/* Generated by qcore `source.extern-call.v1`. */"));
    assert!(c.contains("uint8_t *r1 = qcore_extern_alloc(8);"));
    // The composed route's prologue is not this route's: no `<inttypes.h>`, no
    // rendering, nothing moved.
    assert!(!c.contains("inttypes.h"));
}

#[test]
fn declaring_an_extern_function_without_calling_it_is_not_a_composition() {
    // Declaring is not calling, so a printing module that merely declares an
    // extern function stays on the standard-IO route.
    let source = "#import std.fmt;\n\
                  #import std.io;\n\
                  extern fn qcore_extern_scale(value: i32, factor: i32) -> i32;\n\
                  fn main() -> unit effects { io } {\n    print dec 7;\n    return;\n}\n";
    let compiled = compile_with(source, None).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
}

#[test]
fn importing_the_standard_io_modules_without_using_them_is_not_a_composition() {
    let source = ROUND_TRIP.replace(
        "#import std.extern;",
        "#import std.extern;\n#import std.fmt;\n#import std.io;",
    );
    let compiled = compile(&source).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
}

// --- The concurrency exclusions, unmoved. ---

#[test]
fn a_module_that_prints_and_spawns_keeps_the_task_routes_refusal() {
    let source = "#import std.concurrency;\n\
                  #import std.fmt;\n\
                  #import std.io;\n\
                  fn main() -> i32 {\n    thread scope s {\n        print dec 1;\n    }\n    return 0;\n}\n";
    assert_eq!(
        compile_with(source, None)
            .expect_err("a printing thread scope stays refused")
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["Q600"]
    );
}

#[test]
fn a_module_that_calls_prints_and_spawns_still_keeps_the_task_routes_refusal() {
    // All three feature sets at once. The composed selector runs *after* every
    // concurrent selector, so the task route still wins and still refuses.
    let source = "#import std.concurrency;\n\
                  #import std.extern;\n\
                  #import std.fmt;\n\
                  #import std.io;\n\
                  \n\
                  error Failure { Failed; }\n\
                  extern fn qcore_extern_scale(value: i32, factor: i32) -> i32;\n\
                  \n\
                  fn main() -> unit effects { io, alloc, mut } {\n\
                  \x20   thread scope s {\n\
                  \x20       let n = qcore_extern_scale(21, 2) succeeds 42;\n\
                  \x20       print dec n;\n\
                  \x20   }\n\
                  \x20   return;\n\
                  }\n";
    assert_eq!(codes(source), ["Q600"]);
}

// --- The extern half's refusals, unweakened by composing. ---

#[test]
fn an_acquisition_without_its_adapter_still_refuses_at_q025() {
    let source = module(
        "    qcore_extern_alloc(8);\n\
         \x20   print dec 1;\n",
    );
    assert_eq!(codes(&source), ["Q025"]);
}

#[test]
fn a_callee_with_no_selected_contract_still_refuses_at_q023() {
    let source = module(
        "    let n = qcore_extern_scale(21, 2) succeeds 42;\n\
         \x20   print dec n;\n",
    );
    assert_eq!(
        compile_with(&source, None)
            .expect_err("no contract environment is selected")
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["Q023"],
        "the `Q023` default is unmoved: composing buys no callee"
    );
}

#[test]
fn a_leaked_acquisition_still_refuses_in_the_sequential_passes() {
    let source = module(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n\
         \x20   print dec 1;\n",
    );
    assert_eq!(
        codes(&source),
        ["Q202"],
        "the established leak accounting runs over the extern half exactly as it does alone"
    );
}

#[test]
fn a_use_after_move_still_refuses() {
    let source = module(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n\
         \x20   qcore_extern_release(move b);\n\
         \x20   print dec 1;\n\
         \x20   qcore_extern_fill(b, 7);\n",
    );
    assert!(
        !codes(&source).is_empty(),
        "a use after move must stay refused across the composition"
    );
}

#[test]
fn a_unit_result_still_cannot_be_bound() {
    let source = module(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n\
         \x20   let u = qcore_extern_fill(b, 7);\n\
         \x20   print dec 1;\n\
         \x20   qcore_extern_release(move b);\n",
    );
    assert_eq!(codes(&source), ["Q024"]);
}

#[test]
fn an_acquiring_callees_result_still_cannot_be_bound_as_a_scalar() {
    let source = module(
        "    let b = qcore_extern_alloc(8);\n\
         \x20   print dec 1;\n",
    );
    assert_eq!(codes(&source), ["Q025"]);
}

// --- The standard-IO half's refusals, unweakened by composing. ---

#[test]
fn rendering_a_resource_still_refuses_at_q028() {
    let source = module(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n\
         \x20   print dec b;\n\
         \x20   qcore_extern_release(move b);\n",
    );
    assert_eq!(
        codes(&source),
        ["Q028"],
        "the effect discipline is structural: no operand form denotes a resource"
    );
}

#[test]
fn an_entry_that_writes_without_declaring_io_refuses_at_the_extern_halfs_gate() {
    // Measured, and recorded rather than designed around. `Q029` — the
    // standard-IO route's own "this entry performs Io but its declared row does
    // not admit it" — is **unreachable on the composed route**, and so is the
    // composed checker's own effect-union arm: `derived_effects` stamps `Io` on
    // *every* extern call unconditionally (the measurement card STD-MODULES
    // §1.3 recorded), so the extern half's per-call gate refuses an entry
    // missing `io` before either later gate can. Both are kept as fail-closed
    // backstops on the PIPE-SCALAR-RESULT §2.3 precedent, and both halves of
    // the boundary are pinned here: the cascade that *is* measured, and the
    // absence of `Q029` and `Q043`, so a change in reachability says so rather
    // than passing silently.
    let source = CHAIN.replace("effects { io, alloc, mut }", "effects { alloc, mut }");
    let measured = codes(&source);
    assert_eq!(measured, ["Q025", "Q025", "Q025"]);
    assert!(!measured.contains(&"Q029".to_owned()));
    assert!(!measured.contains(&"Q043".to_owned()));
}

#[test]
fn the_composed_effect_row_is_the_union_of_the_two_derived_rows() {
    use qcore_compiler::typed_errors::Effect;

    let compiled = compile(CHAIN).unwrap();
    let checked = compiled.checked.as_composed().unwrap();
    // The extern half derives `Io`, `Alloc` and `Mut` here; the standard-IO
    // half derives exactly `{Io}`. The composed row is their union and is
    // admitted by the one row the entry declares.
    for effect in [Effect::Io, Effect::Alloc, Effect::Mut] {
        assert!(
            checked.certificate.effects.contains(&effect),
            "the composed row must carry {effect:?}"
        );
    }
    assert!(checked.certificate.io.effects.contains(&Effect::Io));
    assert!(!checked.certificate.io.effects.contains(&Effect::Alloc));
    assert!(
        !checked.certificate.io.effects.contains(&Effect::Mut),
        "the rendering half's row stays exactly {{Io}}: composing gives it no allocation"
    );
}

#[test]
fn an_unknown_rendering_still_refuses() {
    let source = module(
        "    let n = 42;\n\
         \x20   print oct n;\n\
         \x20   qcore_extern_scale(n, 2);\n",
    );
    assert_eq!(
        codes(&source),
        ["Q013"],
        "an unreserved word in a rendering position is collected as a referenced name, exactly as \
         card STD-MODULES measured on the route alone"
    );
}

#[test]
fn printing_without_the_io_import_still_refuses_at_q015() {
    let source = CHAIN.replace("#import std.io;", "");
    assert_eq!(codes(&source), ["Q015"]);
}

#[test]
fn calling_without_the_extern_import_still_refuses_at_q022() {
    let source = CHAIN.replace("#import std.extern;", "");
    assert_eq!(codes(&source), ["Q022"]);
}

#[test]
fn a_sink_in_a_non_final_position_still_refuses_at_q037() {
    let source = module(
        "    let n = 42;\n\
         \x20   n |> print dec |> log info dec;\n\
         \x20   qcore_extern_scale(n, 2);\n",
    );
    assert_eq!(
        codes(&source),
        ["Q037"],
        "`print` produces no value, so it cannot feed a later stage — the narrow, permanent rule"
    );
}

// --- The join's own refusals: `Q043`. ---

#[test]
fn rendering_a_value_whose_number_no_model_determines_refuses_at_q043() {
    let source = module(
        "    let n = qcore_extern_scale(21, 2);\n\
         \x20   print dec n;\n",
    );
    assert_eq!(
        codes(&source),
        ["Q043"],
        "a bound call result with no declared outcome is not a renderable operand"
    );
}

#[test]
fn the_same_shape_with_a_declared_outcome_is_admitted() {
    let source = module(
        "    let n = qcore_extern_scale(21, 2) succeeds 42;\n\
         \x20   print dec n;\n",
    );
    let compiled = compile(&source).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::ComposedStdIoExternV1
    );
    assert!(
        compiled
            .extern_observation
            .as_deref()
            .unwrap()
            .contains(&"out: 42".to_owned())
    );
}

#[test]
fn rendering_a_wider_scalar_renders_it_at_its_own_type() {
    // `qcore_extern_widen` declares `i64`. Before card
    // `W48-QR-SCALAR-TYPES-R3` its rendering refused at `Q043`, because
    // `source.std-io.v1`'s two renderings were total on `int32_t` alone. The
    // card widened the composed rendering to every §G02t type through §G02t's
    // own conversion table — a deliberate widening — so the determined `i64`
    // renders at its own type: `dec` the number, `hex` sixteen digits. A value
    // no model determined is still refused (`Q043`, rendering an undeclared
    // result, pinned above).
    let source = module(
        "    let wide = qcore_extern_widen(21) succeeds 21;\n\
         \x20   print dec wide;\n\
         \x20   print hex wide;\n",
    );
    let rendered = compile(&source).expect("an i64 renders at its own type");
    assert_eq!(
        rendered.extern_observation.unwrap(),
        ["out: 21", "out: 0x0000000000000015"]
    );
    // The same binding, not rendered, is admitted too.
    let bound_only = module(
        "    let wide = qcore_extern_widen(21) succeeds 21;\n\
         \x20   print dec 1;\n",
    );
    assert_eq!(
        compile(&bound_only).unwrap().execution_profile,
        SourceExecutionProfile::ComposedStdIoExternV1
    );
}

#[test]
fn rendering_a_value_of_a_width_this_profile_does_not_render_refuses_at_q043() {
    // `qcore_extern_open` declares `resource_out(slot, nullable)` with an i32
    // status, so it acquires and binds no scalar at all — the case a rule
    // written over the result alone would have got wrong. Its refusal is the
    // extern route's, not the join's; this test pins that the two are told
    // apart.
    let source = module(
        "    let b = qcore_extern_open(out, 8);\n\
         \x20   print dec b;\n",
    );
    assert_eq!(codes(&source), ["Q025"]);
}

#[test]
fn rendering_a_value_bound_later_in_the_entry_refuses() {
    let source = module(
        "    print dec n;\n\
         \x20   let n = 42;\n\
         \x20   qcore_extern_scale(n, 2);\n",
    );
    // Name resolution's own gate runs over the parsed body before any lowering,
    // so a forward reference is refused there. `Q043`'s ordering arm is the
    // fail-closed backstop beneath it — see the report's §6.
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn an_unchecked_linkage_waiver_crosses_the_join_and_stays_visible() {
    use qcore_compiler::extern_call::ExternAssumption;

    // A composed module that links an uncontracted symbol is checked *under*
    // that waiver, and the surface at which a reader sees it must not be lost
    // by composing: `declared_assumptions` is the composed certificate's own
    // extern projection, which is the extern route's list verbatim.
    let source = "#import std.extern;\n\
                  #import std.extern.unsafe;\n\
                  #import std.fmt;\n\
                  #import std.io;\n\
                  unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n\
                  \x20   let n = k_unchecked_add(2, 40) succeeds 42;\n\
                  \x20   print dec n;\n\
                  \x20   return;\n}\n";
    let compiled = compile_with(source, None).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::ComposedStdIoExternV1
    );
    let waived = compiled
        .declared_assumptions()
        .iter()
        .any(|premise| matches!(premise, ExternAssumption::UncheckedLinkage { callee, .. } if callee == "k_unchecked_add"));
    assert!(
        waived,
        "composing must not hide an unchecked-linkage waiver"
    );
    // And the premise is identical to the one the *discarding* spelling
    // deposits: it is per callee, not per call site, so neither binding a
    // result nor rendering it can launder it.
    let discarding = source.replace(
        "    let n = k_unchecked_add(2, 40) succeeds 42;\n    print dec n;",
        "    k_unchecked_add(2, 40);\n    print dec 42;",
    );
    let discarding = compile_with(&discarding, None).unwrap();
    assert_eq!(
        compiled.declared_assumptions(),
        discarding.declared_assumptions()
    );
}

// --- The literal binding, `EXTERN-SCALAR-LITERAL`. ---

#[test]
fn a_scalar_literal_binds_on_the_extern_route_alone() {
    let source = "#import std.extern;\n\
                  error Failure { Failed; }\n\
                  extern fn qcore_extern_scale(value: i32, factor: i32) -> i32;\n\
                  fn main() -> unit effects { io, alloc, mut } {\n\
                  \x20   let n = 42;\n\
                  \x20   qcore_extern_scale(n, 2);\n\
                  \x20   return;\n}\n";
    let compiled = compile(source).unwrap();
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::ExternCallsV1,
        "the literal binding is the extern route's own widening; composing is not required for it"
    );
    let checked = compiled.checked.as_extern_calls().unwrap();
    assert_eq!(checked.certificate.literal_bindings, vec![(1, 42)]);
    // It opens no obligation: an unread literal is not a leak, and the ledger
    // never saw it.
    assert!(checked.certificate.acquisitions.is_empty());
    assert!(checked.certificate.consumptions.is_empty());
}

#[test]
fn a_scalar_literal_cannot_shadow_a_live_resource_binding() {
    let source = module(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n\
         \x20   let b = 42;\n\
         \x20   print dec 1;\n\
         \x20   qcore_extern_release(move b);\n",
    );
    assert!(
        !codes(&source).is_empty(),
        "a duplicate local binding stays refused"
    );
}

#[test]
fn an_out_of_range_literal_refuses_rather_than_wrapping() {
    let source = module(
        "    let n = 4294967296;\n\
         \x20   print dec n;\n",
    );
    assert!(
        !codes(&source).is_empty(),
        "a number this profile cannot carry exactly is not carried at all"
    );
}

// --- The raise path: a run that ends before its later renderings. ---

#[test]
fn a_raised_error_ends_the_composed_run_and_its_later_renderings() {
    let source = "#import std.extern;\n\
                  #import std.fmt;\n\
                  #import std.io;\n\
                  \n\
                  error Failure { Failed; }\n\
                  extern fn qcore_extern_alloc_failing(size: usize) -> ptr<mut, u8>;\n\
                  extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\
                  \n\
                  fn main() -> unit effects { io, alloc, mut } {\n\
                  \x20   print dec 1;\n\
                  \x20   let b = qcore_extern_alloc_failing(8) succeeds false else raise Failure;\n\
                  \x20   print dec 2;\n\
                  \x20   qcore_extern_release(move b);\n\
                  \x20   return;\n}\n";
    let compiled = compile(source).unwrap();
    let observation = compiled.extern_observation.as_deref().unwrap();
    // The rendering *before* the failing acquisition happens; the acquisition is
    // observed on its null arm and raises; nothing after it is observed —
    // neither the later rendering nor the `return`. The emitted C returns inside
    // the failure arm, so publishing that later line would be an observation the
    // program never writes, and the per-stream comparison below would be a
    // comparison against fiction.
    assert_eq!(observation, ["out: 1"]);
    let (out, err) = run_native(compiled.c_source.as_deref().unwrap(), "raise");
    assert_eq!(out, ["1"]);
    assert!(err.is_empty());
}

// --- Card `STD-FMT-BORROW`: the borrowed render of a live resource. ---

#[test]
fn a_borrowed_render_is_admitted_and_names_the_live_identity() {
    for source in with_line_endings(BORROWED) {
        let compiled = compile(&source).unwrap();
        assert_eq!(
            compiled.execution_profile,
            SourceExecutionProfile::ComposedStdIoExternV1
        );
        let checked = compiled.checked.as_composed().unwrap();
        // The identity rendered is the one the extern model acquired, and it is
        // published as a custody identity rather than folded into the numbers
        // that crossed the value namespace.
        assert_eq!(checked.certificate.externs.acquisitions, vec![1]);
        assert_eq!(
            checked.certificate.borrowed_renders,
            vec![("b".to_owned(), 1), ("b".to_owned(), 1)]
        );
        assert_eq!(
            checked.certificate.rendered_values,
            vec![("total".to_owned(), 56)]
        );
        // The two renderings publish the identity, in the two renderings the
        // `std.fmt` vocabulary has, on their two separate streams.
        let reference = compiled.extern_observation.as_deref().unwrap();
        assert!(reference.contains(&"out: 1".to_owned()));
        assert!(reference.contains(&"err: [INFO] 56".to_owned()));
        assert!(reference.contains(&"err: [WARN] 0x00000001".to_owned()));
    }
}

#[test]
fn a_borrowed_render_leaves_the_custody_obligations_unchanged() {
    // The custody argument's second clause, measured rather than asserted: the
    // extern projection of the module *with* two borrowed renderings is the
    // extern projection of the module without them, ledger field for ledger
    // field.
    let with_renders = compile(BORROWED).unwrap();
    let without = compile(
        &BORROWED
            .replace("    print dec borrow b;\n", "")
            .replace("    log warn hex borrow b;\n", ""),
    )
    .unwrap();
    let a = &with_renders
        .checked
        .as_composed()
        .unwrap()
        .certificate
        .externs;
    let b = &without.checked.as_composed().unwrap().certificate.externs;
    assert_eq!(a.acquisitions, b.acquisitions);
    assert_eq!(a.consumptions, b.consumptions);
    assert_eq!(a.releases, b.releases);
    assert_eq!(a.stored, b.stored);
    assert_eq!(a.reallocations, b.reallocations);
    assert_eq!(a.error_dispositions, b.error_dispositions);
    assert_eq!(a.live_after, b.live_after);
    assert_eq!(
        a.assumptions, b.assumptions,
        "a rendering carries no premise"
    );
    // And the value namespace is untouched: rendering an identity binds
    // nothing, so no `ValueId` was introduced for it.
    assert_eq!(a.scalar_bindings, b.scalar_bindings);
    assert_eq!(a.literal_bindings, b.literal_bindings);
}

#[test]
fn a_borrowed_render_of_a_released_identity_refuses() {
    // The liveness precondition, in the direction that matters: the release is
    // moved above the rendering and nothing else changes.
    assert_eq!(
        codes(&module(
            "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    \
             qcore_extern_release(move b);\n    print dec borrow b;\n"
        )),
        vec!["Q043"]
    );
}

#[test]
fn a_borrowed_render_before_its_acquisition_refuses() {
    // Measured, not designed, and the same shape cards `PIPE-SCALAR-RESULT` and
    // `STD-IO-COMPOSE` both recorded: name resolution runs over the parsed body
    // before any lowering, so a use before its binding refuses at `Q013`. The
    // join's liveness rule would refuse it too — an identity not yet acquired
    // is not live — and is kept as the fail-closed backstop. Pinning `Q013`
    // here means a change in that reachability says so rather than passing
    // silently.
    assert_eq!(
        codes(&module(
            "    print dec borrow b;\n    let b = qcore_extern_alloc(8) succeeds true else raise \
             Failure;\n    qcore_extern_release(move b);\n"
        )),
        vec!["Q013"]
    );
}

#[test]
fn a_borrowed_render_of_a_failed_acquisition_refuses() {
    // `succeeds false` is the site's declared null arm: the identity is never
    // established on this run, so it is not live and the rendering refuses
    // rather than publishing a line about a resource that does not exist.
    assert_eq!(
        codes(&module(
            "    let b = qcore_extern_alloc(8) succeeds false else raise Failure;\n    print dec \
             borrow b;\n    qcore_extern_release(move b);\n"
        )),
        vec!["Q043"]
    );
}

#[test]
fn a_borrowed_render_of_a_scalar_refuses() {
    // The namespace wall, in one direction: `borrow` reads the custody
    // namespace and never the value namespace, so a perfectly renderable
    // scalar is not borrowable. The module calls, so the composed route really
    // is the one deciding this.
    assert_eq!(
        codes(&module(
            "    let n = 42;\n    qcore_extern_scale(n, 2);\n    print dec borrow n;\n"
        )),
        vec!["Q043"]
    );
}

#[test]
fn a_bare_render_of_a_resource_still_refuses_at_the_standard_io_route() {
    // The other direction, and the pin card `STD-IO-COMPOSE` established:
    // dropping the marker does **not** render the identity. `Q028`'s
    // resource-free rule for a bare operand is unweakened.
    assert_eq!(
        codes(&module(
            "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    print dec \
             b;\n    qcore_extern_release(move b);\n"
        )),
        vec!["Q028"]
    );
}

#[test]
fn a_borrowed_render_of_an_unbound_name_refuses_at_name_resolution() {
    // Measured, not designed: the marker is reserved and the name after it is
    // an ordinary reference, so the established gate runs first.
    assert_eq!(codes(&module("    print dec borrow nope;\n")), vec!["Q013"]);
}

#[test]
fn a_borrow_marker_with_no_name_refuses_at_the_routes_shape_rule() {
    assert_eq!(
        codes(&module(
            "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    print dec \
             borrow;\n    qcore_extern_release(move b);\n"
        )),
        vec!["Q030"]
    );
}

#[test]
fn a_borrowed_render_inside_a_thread_scope_keeps_the_task_routes_refusal() {
    // The route boundary, pinned in the same shape card `STD-MODULES` pinned
    // the plain one: a borrow across a task boundary means something this card
    // does not model, and route selection keeps the concurrent model.
    let source = "#import std.concurrency;\n\
                  #import std.extern;\n\
                  #import std.fmt;\n\
                  #import std.io;\n\
                  \n\
                  error Failure { Failed; }\n\
                  \n\
                  extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;\n\
                  extern fn qcore_extern_release(buffer: ptr<mut, u8>) -> unit;\n\
                  \n\
                  fn main() -> unit effects { io, alloc, mut } {\n    \
                  let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    \
                  thread scope w {\n        print dec borrow b;\n    }\n    \
                  qcore_extern_release(move b);\n    return;\n}\n";
    assert_eq!(codes(source), vec!["Q600"]);
}

#[test]
fn a_pipeline_cannot_spell_a_borrowed_render() {
    // The pipe composes with this card deliberately and in the refusing
    // direction. A sink stage's selector words are the rendering's, so the
    // marker is one word too many; and dropping the marker desugars to a bare
    // operand, which is the `Q028` case above. Neither spelling reaches the
    // borrowed render, so `PIPE-BORROW-SINK` is a named follow-on rather than
    // an accident.
    assert_eq!(
        codes(&module(
            "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    b |> print \
             dec borrow;\n    qcore_extern_release(move b);\n"
        )),
        vec!["Q036"]
    );
    assert_eq!(
        codes(&module(
            "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n    b |> print \
             dec;\n    qcore_extern_release(move b);\n"
        )),
        vec!["Q028"]
    );
}

#[test]
fn a_tampered_borrowed_render_list_fails_the_independent_recheck() {
    // The fifth tamper direction, beside the four card `STD-IO-COMPOSE`
    // pinned: the identities a publication says it rendered are rederived and
    // compared, so a certificate that quietly dropped or edited them is not a
    // certificate.
    use qcore_core::composed_std_io;
    let compiled = compile(BORROWED).unwrap();
    let checked = compiled.checked.as_composed().unwrap();
    let mut tampered = checked.certificate.clone();
    tampered.borrowed_renders.clear();
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered borrowed-render list must not survive the composed recheck"
    );
    let mut tampered = checked.certificate.clone();
    tampered.borrowed_renders = vec![("b".to_owned(), 7)];
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "and neither must one naming an identity this module never rendered"
    );
    // The standard-IO half publishes the same fact on its own, and its own
    // recheck covers it too.
    let mut tampered = checked.certificate.clone();
    tampered.io.borrowed_renders.clear();
    assert!(
        composed_std_io::recheck_composed_certificate(&checked.lowered.program, &tampered).is_err(),
        "the standard-IO projection's own borrowed-render list is rechecked as well"
    );
}

// --- Native evidence, under ASan/UBSan/LSan. ---

#[test]
fn the_borrowed_render_program_matches_the_reference_per_stream() {
    let compiled = compile(BORROWED).unwrap();
    let reference = compiled.extern_observation.as_deref().unwrap();
    let (out, err) = run_native(compiled.c_source.as_deref().unwrap(), "borrowed");

    let expected_out = reference
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let expected_err = reference
        .iter()
        .filter_map(|line| line.strip_prefix("err: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    assert_eq!(out, expected_out, "stdout must match the reference");
    assert_eq!(err, expected_err, "stderr must match the reference");
    // A **positive** leak control for this card: the fixture allocates, renders
    // the identity twice while the obligation is open, and releases. Had a
    // borrowed render moved, duplicated or discharged custody, LSan or the
    // double-free detector would say so rather than this test passing.
    assert!(
        expected_out.contains(&"1".to_owned()),
        "the identity line is one the native run actually wrote"
    );
}

#[test]
fn the_composed_program_matches_the_reference_per_stream() {
    let compiled = compile(CHAIN).unwrap();
    let reference = compiled.extern_observation.as_deref().unwrap();
    let (out, err) = run_native(compiled.c_source.as_deref().unwrap(), "chain");

    let expected_out = reference
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let expected_err = reference
        .iter()
        .filter_map(|line| line.strip_prefix("err: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    assert_eq!(out, expected_out, "stdout must match the reference");
    assert_eq!(err, expected_err, "stderr must match the reference");
    // This fixture allocates, lends and releases one real buffer, so the leak
    // check is a **positive** control: if composing had lost the release, LSan
    // would report it.
    assert_eq!(
        compiled
            .checked
            .as_composed()
            .unwrap()
            .certificate
            .externs
            .acquisitions,
        [1]
    );
}

/// Compile and run the emitted C against the real `buffer_api.c` under
/// ASan/UBSan (plus LSan where the platform provides it), returning
/// `(stdout lines, stderr lines)`.
///
/// The sanitizer runtimes write their own findings to stderr and abort, so a
/// clean exit with the expected stderr lines is itself the evidence that no
/// finding was reported. A declared-outcome disagreement exits 70 and is
/// likewise a failure here rather than a silently different observation.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run_with_sources(
            source,
            None,
            &[],
            &[(
                "buffer_api.c",
                include_str!("fixtures/extern_calls/buffer_api.c"),
            )],
        );
        assert!(
            run.status.success(),
            "emitted program failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        (out, err)
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-compose-{_label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let callees = directory.join("buffer_api.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).unwrap();
        std::fs::write(&callees, include_str!("fixtures/extern_calls/buffer_api.c")).unwrap();
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
            .arg(&callees)
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
        assert!(
            run.status.success(),
            "emitted program failed under ASan/UBSan/LSan:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        std::fs::remove_dir_all(directory).unwrap();
        (out, err)
    }
}

// --- Card `EXTERN-SCALAR-OUTCOME` (trace half): the composed route inherits
// the extended extern observation, because there is only one emission of it. ---

/// **The composed chain's whole observation, pinned as a literal.** Before the
/// trace half this fixture's extern lines were pinned only by the native
/// comparison — which is self-consistent by construction and would have moved
/// silently with the format — so the pin is established here rather than moved.
///
/// The payoff is visible on two adjacent lines: the call's trace line and the
/// `print` that renders the same value now name the same number, which is what
/// makes the trace readable beside the rendering instead of only beside itself.
#[test]
fn the_composed_extern_trace_names_the_numbers_its_renderings_render() {
    let compiled = compile(CHAIN).unwrap();
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        [
            "out: 56",
            "err: [INFO] 112",
            "err: [WARN] 0x00000054",
            "out: -7",
        ]
    );
    // The literal binding is still observed as nothing: `let value = 42;` calls
    // nothing, so `v3` appears only where a call reads it.
    assert!(
        !compiled
            .extern_observation
            .as_deref()
            .unwrap()
            .iter()
            .any(|line| line.contains("value  v3") || line.ends_with("-> 42"))
    );
}

/// And the composed route's C is the extern route's C, so its trace lines carry
/// the same field for the same reason — there is one lowering of an extern call
/// site, not two.
#[test]
fn the_composed_emission_observes_through_the_same_single_route() {
    let compiled = compile(CHAIN).unwrap();
    let c = compiled.c_source.as_deref().unwrap();
    let check = c.find("if (v1 != 56)").expect("emitted C compares v1");
    let observed = c
        .rfind("qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, 1")
        .expect("emitted C logs v1");
    assert!(check < observed, "emitted C: {c}");
}
