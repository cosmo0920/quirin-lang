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

//! Card **CHAINED-ASSIGNMENT-INNER-DESTINATION** (gateway rule 5i, amended):
//! `a = b = E` assigns `E` to `b` and then that value to `a` (C11 6.5.16p3).
//!
//! The statement classifier read `d = safe = rd_malloc(n)` as
//! `d = rd_malloc(n)` and dropped the write of `safe`, so
//! `rd_kafka_sasl_safe_string`'s `return safe;` escaped nothing and a leak was
//! accused (`rdkafka_sasl_scram.c:243`, `:249`). The guard refuses that
//! admission by name; rule 10s lowers the chain as its sequential
//! assignments. `Safe` here is over the synthetic `q_*` fixture contracts
//! (`support/clang_gap.rs`) and promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyInstruction, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};

/// The guard's refusal (`legacy_chained_assignment.rs`).
const CHAINED: &str = "chained assignment writes an inner destination too; it is read only \
                       as its sequential assignments (C11 6.5.16p3)";

const S: &str = "struct S { int a; int b; void *p; void *v; };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

/// The lowered instructions of `subject`, in CFG node order.
fn lowered(source: &str) -> Vec<LegacyInstruction> {
    let text = format!("{DECLARATIONS}\n{S}{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .expect("the fixture is accepted by Clang");
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    summary
        .and_then(|summary| summary.functions.get("subject"))
        .map(|function| {
            function
                .cfg
                .nodes
                .iter()
                .map(|node| node.instruction.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn binds(instructions: &[LegacyInstruction], name: &str) -> bool {
    instructions.iter().any(|instruction| match instruction {
        LegacyInstruction::ContractCall {
            destination: Some(destination),
            ..
        } => destination == name,
        LegacyInstruction::Alias { destination, .. } => destination == name,
        _ => false,
    })
}

fn aliases(instructions: &[LegacyInstruction], destination: &str, source: &str) -> bool {
    instructions.iter().any(|instruction| {
        matches!(
            instruction,
            LegacyInstruction::Alias { destination: d, source: s } if d == destination && s == source
        )
    })
}

fn leaks(run: &G) -> bool {
    run.diagnostics.iter().any(|d| d.starts_with("Leak"))
}

/// The scram shape (`rdkafka_sasl_scram.c:214-249`): a two-pass loop that
/// allocates on the first pass through `d = safe = …`, writes through `d` on
/// the second, and returns `safe`, with the unit's own declaration
/// `char *safe = NULL, *d = NULL` (refused by the lexical gate, as at
/// `:214`). Before the card the chain bound only `d`: `return safe;` escaped
/// nothing, so the run reported a leak at the chain and `loop re-acquisition`
/// there, exactly as the current run does at `:243`. Now both names hold the
/// allocation, so the leak is withdrawn; the body's other refusals keep it
/// `Unknown`, never `Safe` from this rule alone.
const SCRAM: &str = "char *subject(const char *str) {
    char *safe = 0, *d = 0;
    int pass;
    unsigned long len = 0;
    for (pass = 0; pass < 2; pass++) {
        const char *s;
        for (s = str; *s; s++) {
            if (pass == 0) {
                len += 1;
                continue;
            }
            *(d++) = *s;
        }
        if (pass == 0)
            d = safe = q_maybe_alloc();
    }
    *d = 0;
    return safe;
}";

#[test]
fn the_scram_shape_withdraws_the_leak_and_does_not_become_safe() {
    if !clang_available() {
        return;
    }
    let run = gs(SCRAM);
    assert!(!leaks(&run), "{run:?}");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(IND), "{run:?}");
    let instructions = lowered(SCRAM);
    assert!(binds(&instructions, "safe"), "{instructions:?}");
    assert!(aliases(&instructions, "d", "safe"), "{instructions:?}");
    // The old reading, spelled out: the call bound to `d` alone leaks at the
    // chain and reports the loop re-acquisition there.
    let old = gs(&SCRAM.replace("d = safe = q_maybe_alloc();", "d = q_maybe_alloc();"));
    assert!(old.unsafe_() && leaks(&old), "{old:?}");
    assert!(
        old.refused("loop re-acquisition: an earlier generation that may be live is still held"),
        "{old:?}"
    );
}

/// The minimal chain: the inner name returned escapes the allocation.
#[test]
fn the_inner_name_holds_the_value_c_assigns_it() {
    if !clang_available() {
        return;
    }
    let source = "void *subject(void) { void *d; void *s; d = s = q_alloc(); return s; }";
    assert!(gs(source).safe(), "{:?}", gs(source));
    let instructions = lowered(source);
    assert!(binds(&instructions, "s"), "{instructions:?}");
    assert!(aliases(&instructions, "d", "s"), "{instructions:?}");
    // Longer chains, the parenthesised spelling, a declaration and a return.
    for source in [
        "void *subject(void) { void *a; void *b; void *c; a = b = c = q_alloc(); return a; }",
        "void *subject(void) { void *d; void *s; d = (s = q_alloc()); return s; }",
        "void *subject(void) { void *s; void *d = s = q_alloc(); return s; }",
        "void *subject(void) { void *s; void *d = (s = q_alloc()); return s; }",
        "void *subject(void) { void *d; void *s; return d = s = q_alloc(); }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
    // Without the chain's second piece the acquisition still leaks.
    assert!(leaks(&gs(
        "void subject(void) { void *d; void *s; d = s = q_alloc(); }"
    )));
}

/// A release through the inner name is matched against the one acquisition,
/// and a second release through the outer name is the double release it is.
#[test]
fn a_release_through_the_inner_name_is_matched() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(void) { void *d; void *s; d = s = q_alloc(); q_free(s); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { void *d; void *s; d = s = q_alloc(); q_free(d); }");
    assert!(run.safe(), "{run:?}");
    let run =
        gs("void subject(void) { void *d; void *s; d = s = q_alloc(); q_free(s); q_free(d); }");
    assert!(
        run.diagnostics
            .iter()
            .any(|d| d.starts_with("DoubleRelease")),
        "{run:?}"
    );
    let run = gs(
        "void subject(void) { void *s = q_alloc(); void *a; void *b; a = b = s; q_free(a); q_free(b); }",
    );
    assert!(
        run.diagnostics
            .iter()
            .any(|d| d.starts_with("DoubleRelease")),
        "{run:?}"
    );
}

/// A destination the route refuses alone keeps the whole chain refused by
/// name: a member store keeps the lexical gate's refusal, and the same object
/// written twice (two unsequenced stores, C11 6.5p2), which the old reading
/// admitted as one write and rule 10s does not lower, keeps the guard's.
#[test]
fn a_chain_with_a_refused_destination_stays_refused_by_name() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(struct S *o) { void *s; o->p = s = q_alloc(); q_free(s); }",
        "void subject(struct S *o) { void *s; s = o->p = q_alloc(); q_free(s); }",
    ] {
        let run = gs(source);
        assert!(run.unknown(), "{source}: {run:?}");
        assert!(run.refused(IND), "{source}: {run:?}");
    }
    let run = gs("void *subject(void) { void *s; s = s = q_alloc(); return s; }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(CHAINED), "{run:?}");
}

/// A scalar chain was already refused by the older routes ("multiple scalar
/// writes") and lowered by rule 10s; the guard leaves a refusal as it is, so
/// nothing moves.
#[test]
fn a_scalar_chain_is_unchanged() {
    if !clang_available() {
        return;
    }
    let source = "int subject(void) { int x; int y; x = y = 3; return x; }";
    assert!(gs(source).safe());
    let instructions = lowered(source);
    assert!(
        instructions.iter().any(|instruction| matches!(
            instruction,
            LegacyInstruction::ForgetScalar { destination } if destination == "y"
        )),
        "{instructions:?}"
    );
    assert!(aliases(&instructions, "x", "y"), "{instructions:?}");
    assert!(gs("int subject(int k) { int x; int y; x = y += k; return x; }").safe());
    assert!(gs("int subject(int k) { int x; int y; x = y = k; return y; }").safe());
}
