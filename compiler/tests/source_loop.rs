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

//! End-to-end admission and refusal for `source.loop.v1` — the checked bounded
//! counted loop (card `LOOP-V1`).
//!
//! The construct is admitted on `source.buffer.v1` in v1, so every fixture here
//! is a byte-buffer module. Every other route's refusal is **measured and
//! pinned** rather than widened (`LOOP-ROUTE-WIDENING`).
//!
//! The native run compares the emitted program's **stdout** with the reference
//! observation's stdout projection and its **stderr** with the stderr
//! projection — this route's inherited discipline, and the reason the checking
//! route has to be exact rather than approximate.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::ownership::{self, Footprint, OwnershipOp};
use qcore_core::source_loop::{
    self, MAX_ITERATIONS, MAX_UNROLLED_EVENTS, custody_signature, iteration_is_custody_identity,
    peak_candidate_prefixes,
};
use qcore_core::std_buffer::{self, BufferEvent, BufferOperand, BufferProgram};

const COPY: &str = include_str!("fixtures/source_loop/copy.qr");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("source_loop.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn messages(source: &str) -> String {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A one-entry byte-buffer module around `body`, so each refusal test measures
/// the rule it names against the same declarations.
fn module(effects: &str, body: &[&str]) -> String {
    let mut source = vec![
        "#import std.buffer;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        "#import std.logger;".to_owned(),
        String::new(),
        format!("fn main() -> unit effects {{ {effects} }} {{"),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

// --- The census, pinned. -------------------------------------------------

/// Constraint 14's census result, as a test rather than as a claim.
///
/// `for` is reserved and `in` is **not**: the census found `in` as a genuine
/// reference inside the pinned contract corpus
/// (`ensures normal resource_borrowed(in);`), exactly the shape that kept `len`
/// out of the global list, so it is skipped contextually at the one position a
/// `for` header puts it and is an ordinary word everywhere else.
#[test]
fn in_is_an_ordinary_name_outside_a_for_header() {
    // A binding named `in` still resolves. Were `in` globally reserved this
    // would refuse at `Q013` with `in` reported as an unknown name.
    let source = module(
        "alloc, mut",
        &[
            "let in = 2;",
            "let b = bytes(4);",
            "b[0] = in;",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).expect("`in` is not a reserved word");
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.scalars.get("in"), Some(&2));
}

/// The other half, stated for what it is: this card claims the **statement-head
/// position** `for`, not every occurrence of the word.
///
/// `for` is in `is_reserved_source_word` — measured load-bearing, because
/// without it the header's own first token is collected as a referenced name
/// and every loop refuses at `Q013` — and that reservation governs *reference*
/// collection only. A `let` still binds the name, exactly as it does for
/// `drop`, `close` and every other reserved statement head, and that is
/// pre-existing behaviour rather than anything this card decided. The census is
/// what makes both facts free: zero occurrences of `for` outside `//` comments
/// across all 418 `.qcore`/`.qc` files.
#[test]
fn for_is_claimed_at_a_statement_head_and_not_everywhere() {
    let source = module(
        "alloc, mut",
        &[
            "let for = 2;",
            "let b = bytes(4);",
            "for i in 0..2 { b[i] = for; }",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).expect("both uses are admitted");
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.scalars.get("for"), Some(&2));
    assert_eq!(checked.certificate.accesses.len(), 2);
}

// --- The accepted construct. ---------------------------------------------

#[test]
fn the_looping_module_selects_the_buffer_profile() {
    let module = compile(COPY).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    let checked = module.checked.as_standard_buffer().unwrap();
    assert_eq!(
        checked.certificate.acquisitions,
        [("src".to_owned(), 4u32), ("dst".to_owned(), 4u32)]
    );
    assert_eq!(checked.certificate.releases, ["src", "dst"]);
}

/// **The payoff.** `STD-BUFFER-COPY` was recorded as needing "the loop or the
/// bulk operation the flat route has no statement for". This is that loop, and
/// the copy is written with no new byte statement at all.
#[test]
fn a_bounded_copy_between_two_buffers_is_admitted_and_exact() {
    let module = compile(COPY).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    // Four reads of `src` and four writes of `dst`, each at its own index, each
    // decided at check time. The loop ran; nothing was approximated.
    let copied: Vec<(u32, u8, bool)> = checked
        .certificate
        .accesses
        .iter()
        .filter(|access| access.name == "dst" && access.write)
        .map(|access| (access.index, access.byte, access.write))
        .collect();
    assert_eq!(
        copied,
        [(0, 81, true), (1, 67, true), (2, 111, true), (3, 114, true)]
    );
}

#[test]
fn the_index_takes_every_value_in_order_and_the_lines_prove_it() {
    let module = compile(COPY).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    let out: Vec<&str> = checked
        .certificate
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    // `for k in 0..3 { print dec k; }` writes 0, 1, 2 — the index rendered as
    // an ordinary integer scalar, needing no new operand form.
    assert_eq!(out, ["0", "1", "2", "81", "67", "111", "114", "[INFO] 4"]);
}

#[test]
fn a_bound_that_is_a_name_resolves_through_the_decidable_tier() {
    // `for i in 0..n` where `n = extent(src)`. The bound is a number the
    // checker reads out of the ownership ledger's own footprint.
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(3);",
            "let n = extent(b);",
            "for i in 0..n { b[i] = 9; }",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.accesses.len(), 3);
}

#[test]
fn nesting_is_admitted_and_the_inner_index_is_its_own() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(2);",
            "for i in 0..2 { for j in 0..2 { b[j] = 7; } }",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    let checked = compiled.checked.as_standard_buffer().unwrap();
    // Two passes of the outer loop times two of the inner: four writes.
    assert_eq!(checked.certificate.accesses.len(), 4);
}

/// The identity discipline's **positive** control: a body that acquires and
/// releases is the identity on custody and is admitted, which is the shape the
/// rule exists to allow rather than a case it happens not to refuse.
#[test]
fn a_body_that_acquires_and_releases_is_the_identity_and_is_admitted() {
    let source = module(
        "alloc, mut",
        &[
            "for i in 0..3 { let c = bytes(2); c[0] = 1; drop c; }",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    let checked = compiled.checked.as_standard_buffer().unwrap();
    // Three acquisitions and three releases — the unrolled body, exactly.
    assert_eq!(checked.certificate.acquisitions.len(), 3);
    assert_eq!(checked.certificate.releases.len(), 3);
    // And only two bytes are ever live at once, because each iteration's buffer
    // is released before the next one's is acquired.
    assert_eq!(checked.certificate.peak_live_bytes, 2);
}

/// Three shapes that had to be measured rather than assumed, because each is a
/// place the emitted C could have failed the `-Wall -Wextra -Werror` gate the
/// generated program is built under, or the two environments could have
/// disagreed about scope.
#[test]
fn an_empty_body_a_nest_reading_the_outer_index_and_a_reused_index_name_all_hold() {
    let source = module(
        "io, alloc, mut",
        &[
            "let b = bytes(4);",
            // An empty body. The C index is still used by the condition and the
            // increment, so it is not an unused variable.
            "for i in 0..3 { }",
            // A nest reading the outer index as an operand of the inner body.
            "for i in 0..2 { for j in 0..2 { b[i] = j; } }",
            // The same index name again, after the first loop scoped it out.
            "for i in 0..2 { print dec i; }",
            "let a = b[0];",
            "let c = b[1];",
            "print dec a;",
            "print dec c;",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap_or_else(|_| panic!("{:?}", codes(&source)));
    let checked = compiled.checked.as_standard_buffer().unwrap();
    // The inner loop writes `j` at `i` twice per outer pass, so each of the two
    // bytes ends at 1 — a fact only an exact unrolling can state.
    let lines: Vec<&str> = checked
        .certificate
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(lines, ["0", "1", "1", "1"]);
    let (out, err) = run_native(compiled.c_source.as_deref().unwrap(), "edge");
    assert_eq!(out, ["0", "1", "1", "1"]);
    assert!(err.is_empty(), "{err:?}");
}

// --- The bound: `Q098`. ---------------------------------------------------

#[test]
fn a_zero_bound_refuses_because_its_body_would_never_be_checked() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..0 { b[9] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    // And the refusal is the loop's own rather than the out-of-range store's:
    // the point is that the body is never checked, not that this body happens
    // to be wrong.
    assert!(messages(&source).contains("never be checked"));
}

#[test]
fn a_bound_the_lowering_cannot_compute_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "let x = b[0];",
            "for i in 0..x { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    assert!(messages(&source).contains("check-time-constant bound"));
}

#[test]
fn a_bound_above_the_iteration_maximum_refuses_rather_than_truncating() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..99999 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    assert!(messages(&source).contains(&MAX_ITERATIONS.to_string()));
}

/// **Amended by card `LOOP-UNROLL-BUDGET`**, which is the one test of this
/// suite whose fixture that card changed: the bound is above the default
/// unrolling budget, so it now carries the attribute that licenses it. The
/// claim is unchanged — the largest bound the construct models is admitted —
/// and what moved is that reaching it is now something a program **declares**.
#[test]
fn the_largest_admitted_bound_is_admitted() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(1);",
            &format!("[| #unroll, where #bound < {} |]", MAX_ITERATIONS + 1),
            &format!("for i in 0..{MAX_ITERATIONS} {{ b[0] = 1; }}"),
            "drop b;",
            "return;",
        ],
    );
    // `MAX_ITERATIONS` single-event iterations is inside `MAX_UNROLLED_EVENTS`,
    // so the maximum bound itself is admitted — which is what makes the refusal
    // above a boundary rather than a ceiling.
    let compiled = compile(&source).unwrap_or_else(|_| panic!("{:?}", codes(&source)));
    let checked = compiled.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.accesses.len(), MAX_ITERATIONS as usize);
}

#[test]
fn a_negative_bound_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..-1 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
}

#[test]
fn a_range_with_another_low_end_refuses_rather_than_being_re_based() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 1..3 { b[0] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
}

/// The unrolled budget, measured through nesting — which is where it bites, and
/// why the limit is a **product** rather than a depth.
#[test]
fn the_unrolled_budget_refuses_a_nest_the_route_would_check_slowly() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..100 { for j in 0..100 { b[0] = 1; } }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q098"]);
    assert!(messages(&source).contains(&MAX_UNROLLED_EVENTS.to_string()));
}

// --- The shape: `Q097`. ---------------------------------------------------

#[test]
fn break_inside_a_body_refuses_by_name() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..2 { break; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("LOOP-BREAK"));
}

#[test]
fn continue_inside_a_body_refuses_by_name() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..2 { continue; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("LOOP-BREAK"));
}

#[test]
fn a_return_inside_a_body_refuses_as_an_early_exit() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..2 { return; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("LOOP-BODY-RETURN"));
}

/// One refusal per loop, not one per failing iteration. The program is refused
/// either way; this is about what the report *says*. Every iteration before the
/// failing one still ran and was checked, so the refusal names the first index
/// that is actually wrong.
#[test]
fn a_failing_body_is_reported_once_and_names_the_first_failing_iteration() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..100 { b[i] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q053"]);
    assert!(
        messages(&source).contains("index 4 is outside `b`"),
        "{}",
        messages(&source)
    );
}

#[test]
fn a_body_binding_that_shadows_an_outer_one_refuses() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "let v = 1;",
            "for i in 0..2 { let v = 2; b[0] = v; }",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q097"]);
    assert!(messages(&source).contains("shadowing"));
}

/// The other half of the scoping rule: a body binding is **body-local**, the
/// `IF-STATEMENT` precedent for arm bindings. Naming the index after the loop
/// refuses at this route's own `Q052`, which says the true thing.
#[test]
fn the_index_is_not_visible_after_the_loop() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..2 { b[i] = 1; }",
            "b[i] = 0;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q052"]);
}

/// The same rule, measured at a **different place** — and the difference is the
/// asymmetry `STD-BUFFER` §7.3 recorded, not a second rule.
///
/// Name resolution drains every token before an `Assignment`'s `=`, so a
/// body-local name used as an *index* after the loop is never collected and
/// this route's `Q052` catches it (above); the same name used as the *stored
/// byte* sits after the `=`, is collected, and refuses earlier at `Q013`. Both
/// halves are pinned so a change in either direction says so.
#[test]
fn a_body_binding_is_not_visible_after_the_loop_either() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..2 { let v = 5; b[i] = v; }",
            "b[0] = v;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q013"]);
}

// --- Per-iteration custody identity: `Q099`. -----------------------------

#[test]
fn a_body_that_acquires_without_releasing_is_not_the_identity() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..1 { let c = bytes(2); }",
            "drop b;",
            "return;",
        ],
    );
    let codes = codes(&source);
    // `Q099` first, and the inherited checker's own leak refusal beside it. The
    // bound is **one**, so unrolling alone would have exposed nothing: this is
    // the case the explicit gate exists for.
    assert_eq!(codes.first().map(String::as_str), Some("Q099"));
    assert!(messages(&source).contains("does not release it"));
}

#[test]
fn a_body_that_releases_an_outer_buffer_is_not_the_identity() {
    let source = module(
        "alloc, mut",
        &["let b = bytes(4);", "for i in 0..1 { drop b; }", "return;"],
    );
    assert_eq!(codes(&source), ["Q099"]);
    assert!(messages(&source).contains("does not replace it"));
}

/// Without the explicit gate the two refusals above would need a bound of two
/// or more to appear at all. This pins that the **inherited** checker also
/// refuses them once the unrolling exposes them, so the gate sharpens a
/// discipline rather than replacing one.
#[test]
fn the_inherited_checker_refuses_a_repeated_release_on_its_own() {
    let program = BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: qcore_core::typed_errors::EffectRow::default(),
        records: Vec::new(),
        events: vec![
            BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 2,
            },
            BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 2,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![BufferEvent::Release {
                    name: "b".to_owned(),
                }],
            },
            BufferEvent::Return,
        ],
    };
    let operations = std_buffer::custody_operations(&program);
    assert_eq!(
        ownership::check_ownership(&operations),
        Err(ownership::OwnershipError::DoubleDrop("b".to_owned())),
        "the unrolled body presents the inherited checker with two releases"
    );
}

/// The identity gate, exercised directly over the cycle-3 IR, so the rule is
/// visible as the route-independent statement it is.
#[test]
fn the_identity_gate_is_written_over_the_ownership_ir_alone() {
    let acquire = OwnershipOp::Acquire {
        id: 1,
        owner: "r".to_owned(),
        resource_type: "T".to_owned(),
        footprint: Footprint::single("bytes", 4),
    };
    // Acquire-and-release is the identity; acquire alone is not.
    assert_eq!(
        iteration_is_custody_identity(&[], &[acquire.clone(), OwnershipOp::Drop("r".to_owned())]),
        Ok(Ok(()))
    );
    let moved = iteration_is_custody_identity(&[], &[acquire]).unwrap();
    assert!(moved.is_err());
    // A body that only reads is the identity too.
    assert_eq!(
        iteration_is_custody_identity(
            &[OwnershipOp::Acquire {
                id: 1,
                owner: "r".to_owned(),
                resource_type: "T".to_owned(),
                footprint: Footprint::single("bytes", 4),
            }],
            &[OwnershipOp::Read("r".to_owned())]
        ),
        Ok(Ok(()))
    );
}

/// The signature deliberately does **not** carry the resource identity: a body
/// that releases one buffer and acquires an equivalent one under the same name
/// *is* the identity on custody, and calling that a change would refuse the
/// shape the discipline exists to admit.
#[test]
fn the_signature_reads_the_footprint_and_the_owners_and_not_the_identity() {
    let first = ownership::execute_ownership(&[OwnershipOp::Acquire {
        id: 1,
        owner: "r".to_owned(),
        resource_type: "T".to_owned(),
        footprint: Footprint::single("bytes", 4),
    }])
    .unwrap();
    let second = ownership::execute_ownership(&[OwnershipOp::Acquire {
        id: 99,
        owner: "r".to_owned(),
        resource_type: "T".to_owned(),
        footprint: Footprint::single("bytes", 4),
    }])
    .unwrap();
    assert_ne!(first, second, "the states differ in the resource identity");
    assert_eq!(
        custody_signature(&first),
        custody_signature(&second),
        "the custody signature does not read it"
    );
}

// --- The checking route. --------------------------------------------------

/// The unrolled operation list is what the inherited checker sees, and it is
/// **exact**: the body's operations appear `count` times, with each iteration's
/// body-local binding carrying its own scope.
#[test]
fn the_body_is_unrolled_exactly_and_its_bindings_are_per_iteration() {
    let program = BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: qcore_core::typed_errors::EffectRow::default(),
        records: Vec::new(),
        events: vec![
            BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 2,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![
                    BufferEvent::Acquire {
                        name: "c".to_owned(),
                        extent: 2,
                    },
                    BufferEvent::Release {
                        name: "c".to_owned(),
                    },
                ],
            },
            BufferEvent::Return,
        ],
    };
    let operations = std_buffer::custody_operations(&program);
    let owners: Vec<String> = operations
        .iter()
        .map(|operation| match operation {
            OwnershipOp::Acquire { owner, .. } => format!("acquire {owner}"),
            OwnershipOp::Drop(name) => format!("drop {name}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        owners,
        ["acquire c#0", "drop c#0", "acquire c#1", "drop c#1"],
        "each iteration's `c` is its own binding, exactly as the emitted C's is"
    );
    // And the source name is what a reader is shown.
    assert_eq!(source_loop::display_owner("c#1"), "c");
    // The scoping is what keeps this admitted: the inherited checker refuses
    // re-binding a name it has seen dropped, and that gate is **not** weakened.
    assert!(ownership::check_ownership(&operations).is_ok());
}

#[test]
fn the_unrolled_event_count_is_the_cost_the_budget_bounds() {
    let program = vec![
        BufferEvent::Repeat {
            variable: "i".to_owned(),
            count: 4,
            index: qcore_core::std_buffer::LoopIndex::I32,
            body: vec![
                BufferEvent::Store {
                    name: "b".to_owned(),
                    index: BufferOperand::Literal(0),
                    value: BufferOperand::Literal(1),
                },
                BufferEvent::Repeat {
                    variable: "j".to_owned(),
                    count: 3,
                    index: qcore_core::std_buffer::LoopIndex::I32,
                    body: vec![BufferEvent::Store {
                        name: "b".to_owned(),
                        index: BufferOperand::Literal(0),
                        value: BufferOperand::Literal(2),
                    }],
                },
            ],
        },
        BufferEvent::Return,
    ];
    // 1 (the outer loop) + 4 * (1 store + 1 inner loop + 3 * 1 store) = 1 + 20.
    assert_eq!(std_buffer::unrolled_event_count(&program), 22);
}

/// The peak-live narrowing is **the same number** a scan of every prefix gives.
/// It is an optimisation the unrolling forced, and it changes no published
/// value — which is exactly what a test has to say about it.
#[test]
fn the_peak_is_the_same_number_a_full_prefix_scan_gives() {
    let operations = vec![
        OwnershipOp::Acquire {
            id: 1,
            owner: "a".to_owned(),
            resource_type: "T".to_owned(),
            footprint: Footprint::single("bytes", 3),
        },
        OwnershipOp::Acquire {
            id: 2,
            owner: "b".to_owned(),
            resource_type: "T".to_owned(),
            footprint: Footprint::single("bytes", 5),
        },
        OwnershipOp::Read("b".to_owned()),
        OwnershipOp::Drop("b".to_owned()),
        OwnershipOp::Write("a".to_owned()),
        OwnershipOp::Drop("a".to_owned()),
    ];
    let full = (0..=operations.len())
        .filter_map(|length| ownership::execute_ownership(&operations[..length]).ok())
        .map(|state| state.live().get("bytes"))
        .max()
        .unwrap();
    let narrowed = peak_candidate_prefixes(&operations)
        .into_iter()
        .filter_map(|length| ownership::execute_ownership(&operations[..length]).ok())
        .map(|state| state.live().get("bytes"))
        .max()
        .unwrap();
    assert_eq!(full, 8);
    assert_eq!(narrowed, full);
}

// --- Emission. -------------------------------------------------------------

/// The emitted C is a **real** `for` loop, not an unrolled body. The C computes
/// the index itself, so the native comparison is a check of the unrolling
/// rather than a restatement of it.
#[test]
fn the_emitted_c_is_a_real_for_loop() {
    let module = compile(COPY).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(
        source.contains("for (int32_t s_i = INT32_C(0); s_i < INT32_C(4); s_i += INT32_C(1)) {"),
        "{source}"
    );
    assert!(
        source.contains("qcore_bytes_store(b_dst, (size_t) 4u, (size_t) (s_i), s_v);"),
        "the index expression is written in terms of the C loop variable:\n{source}"
    );
    // Not unrolled: the copy's store is written exactly once.
    assert_eq!(
        source.matches("qcore_bytes_store(b_dst,").count(),
        1,
        "the loop body is emitted once:\n{source}"
    );
}

#[test]
fn the_accessor_prelude_reaches_a_write_that_only_a_loop_body_performs() {
    let source = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "for i in 0..4 { b[i] = 1; }",
            "drop b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    let c = compiled.c_source.as_deref().unwrap();
    assert!(
        c.contains("static void qcore_bytes_store("),
        "a program whose only write is inside a loop still writes one:\n{c}"
    );
    // And it carries no reader, because it reads nothing — the selective
    // prelude `STD-BUFFER` established, applied through the nesting.
    assert!(!c.contains("static int32_t qcore_bytes_load("));
}

/// Byte identity for a program that uses none of this card's construct.
#[test]
fn a_module_without_a_loop_emits_byte_identically() {
    const BYTES: &str = include_str!("fixtures/std_buffer/bytes.qr");
    let before = include_str!("fixtures/source_loop/bytes_expected.c");
    let module = compile(BYTES).unwrap();
    assert_eq!(
        module.c_source.as_deref().unwrap(),
        before.replace("\r\n", "\n"),
        "a program that writes no `for` travels the identical code path"
    );
}

// --- Route admission, measured. -------------------------------------------

/// v1 admits the construct on `source.buffer.v1` only. Every other route's
/// refusal is measured and pinned rather than widened — the `IF-ROUTE-WIDENING`
/// precedent, and the follow-on is `LOOP-ROUTE-WIDENING`.
#[test]
fn the_standard_io_route_still_refuses_a_nested_block() {
    let source = "#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects { io } {\n    for i in 0..3 { print dec i; }\n    return;\n}\n";
    assert_eq!(codes(source), ["Q030"]);
}

#[test]
fn a_task_scope_still_refuses_a_loop() {
    let source = "#import std.concurrency;\nconcurrency checked;\n\nfn main() -> unit {\n    thread scope s { for i in 0..2 { } }\n    return;\n}\n";
    assert!(codes(source).contains(&"Q500".to_owned()));
}

/// A loop outside the entry is refused at the loop's own code rather than left
/// unmodelled — this route lowers the entry only, and a construct nothing checks
/// is exactly what invariant 1 forbids.
#[test]
fn a_loop_outside_the_entry_refuses_rather_than_going_unchecked() {
    let source = "#import std.buffer;\n\nfn helper() -> unit {\n    for i in 0..2 { }\n}\n\nfn main() -> unit effects { alloc, mut } {\n    let b = bytes(4);\n    b[0] = 1;\n    drop b;\n    return;\n}\n";
    assert!(
        codes(source).contains(&"Q097".to_owned()),
        "{:?}",
        codes(source)
    );
}

#[test]
fn the_sequential_route_still_refuses_a_loop() {
    // No bundled module at all: the sequential cleanup route, whose statement
    // set this card does not widen.
    let source = "fn main() -> i32 {\n    for i in 0..2 { }\n    return 0;\n}\n";
    assert!(!codes(source).is_empty());
}

// --- The native evidence. --------------------------------------------------

/// The strict byte comparison, and the point of the whole checking-route
/// argument: the reference execution **unrolled** the loop and the native run
/// **rolled** it, and the two agree byte for byte on both streams.
#[test]
fn the_emitted_program_matches_the_reference_per_stream() {
    let module = compile(COPY).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "copy");
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// A **positive** leak control: the fixture allocates two buffers and releases
/// both inside the entry, and one of the four writes into the second is
/// performed by the loop body. Had the loop written past an end, or had a
/// release been lost, ASan or LSan would have said so rather than this test
/// passing. Apple ASan provides no LSan, so leak detection stays a Linux-only
/// observation.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run(source, None, &[]);
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
            "qcore-source-loop-{_label}-{}-{}",
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
