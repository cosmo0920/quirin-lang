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

//! Card `MUTEX-SHARE-CROSS-ARMS` (wave 35): the four arms wave 32 said it did
//! not have.
//!
//! Card `MUTEX-SHARE-CROSS`'s report states, under a section headed with the
//! `STABILITY-REEVAL` cadence, that "there are no unexercised arms in the
//! card's own rules". Card `STABILITY-REEVAL-1` (wave 34) measured four in
//! `lower_declared_share`, none of them in that card's §6 table and none driven
//! by any test — residual `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`.
//!
//! Two of the four are reachable from source and are driven here. The other two
//! are unreachable from any program and are kept as fail-closed backstops on
//! the strength of `QCore.MutexShareCrossArms`, never on the strength of this
//! file being green — the `RETURN-POSITION-ESCAPE-UNREACHABLE` standard. What
//! this file can still do for them is pin the *non-local* premise their
//! deadness rests on, so a change to the judgment that revives them lands as a
//! red test here rather than as a silent gap.
//!
//! Every refusal below asserts the code **and** the arm's own distinguishing
//! sentence, and additionally asserts the *neighbouring* arms' sentences are
//! absent: all six refusals of this one function share the code `Q600`, arms A
//! and B are adjacent and differ only in which header position they read, and a
//! test that only asserted the code would not tell any of them apart.

use qcore_core::mutex_invariant::check_mutex_invariant_program;
use qcore_core::transfer::{
    AdmittedTransfer, ShareProtocol, TransferKind, TransferMode, admits_capture,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_mutex::{LoweredMutexModule, has_source_mutex, lower_source_mutex};

const DECLARATIONS: &str = "let buf = protected 4; let m = mutex guards buf;";

/// The arms' own sentences, quoted from `lower_declared_share`. Before this
/// card each occurred exactly once in the tree — its own definition — which is
/// how card `STABILITY-REEVAL-1` measured that no test drove them; each now
/// occurs twice, there and here. No two arms share a sentence, so a match here
/// is that arm and no other.
const FORM: &str = "a declared crossing is exactly `thread scope NAME share PLACE under MUTEX";
const NESTING: &str = "a declared crossing does not nest";
const PAYLOAD_NAME: &str = "a declared crossing names the payload it shares";
const MUTEX_NAME: &str = "a declared crossing names the mutex generation that protects the payload";
const JUDGMENT: &str = "the transferability judgment refuses this crossing";
const NOT_A_PROTECTED_RESOURCE: &str = "is not a protected resource of this module";

fn entry(body: &str) -> String {
    format!("#import std.concurrency; fn main() -> unit {{ thread scope s {{ {body} }} }}")
}

/// The whole route: lower, then check with the unedited core. Mirrors the
/// wave-32 file's helper so the two read the same way.
fn refusal(source: &str) -> (String, String) {
    let parsed = parse_source_program(source).expect("parses");
    assert!(
        has_source_mutex(&parsed),
        "route detection must still select this module"
    );
    let typed = elaborate_source(&parsed).expect("elaborates");
    let module: LoweredMutexModule = match lower_source_mutex(&parsed, &typed, "main") {
        Ok(module) => module,
        Err(diagnostics) => {
            let first = &diagnostics[0];
            return (first.code.to_owned(), first.message.clone());
        }
    };
    let diagnostics =
        check_mutex_invariant_program(&module.program).expect_err("this program must be refused");
    let located = module.locate(&diagnostics[0]);
    (located.code.to_owned(), located.message)
}

/// Assert the refusal is this arm and demonstrably not one of its neighbours.
fn only(message: &str, expected: &str, neighbours: &[&str]) {
    assert!(
        message.contains(expected),
        "expected this arm's own sentence, got: {message}"
    );
    for neighbour in neighbours {
        assert!(
            !message.contains(neighbour),
            "the refusal also matched a neighbouring arm ({neighbour}): {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// 1. The two reachable arms, driven.
//
// Both need a header of exactly seven tokens with `under` at index 5, which is
// what the wave-32 shape test never supplied: its three bodies are five and six
// tokens long and all three stop at the length arm above these two. An integer
// literal is a `TokenKind::Integer`, so it is a well-formed token that `name`
// refuses, which is the shape these arms exist for.
// ---------------------------------------------------------------------------

/// **Arm A** (`source_mutex.rs`, the `name(header.get(4))` arm). The payload
/// position holds a token that is not a name.
#[test]
fn a_crossing_whose_payload_position_is_not_a_name_names_the_payload_rule() {
    let (code, message) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share 3 under m {{ }}"
    )));
    assert_eq!(code, "Q600");
    only(
        &message,
        PAYLOAD_NAME,
        &[
            FORM,
            NESTING,
            MUTEX_NAME,
            JUDGMENT,
            NOT_A_PROTECTED_RESOURCE,
        ],
    );
}

/// **Arm B** (`source_mutex.rs`, the `name(header.get(6))` arm). The payload is
/// a name and the *protocol* position is not.
///
/// The pair with arm A is the point: the two arms are three lines apart and
/// differ only in which position they read, so each test asserts the other's
/// sentence is absent. Reversing the two `header.get` indices in the source
/// makes both of these tests red.
#[test]
fn a_crossing_whose_mutex_position_is_not_a_name_names_the_generation_rule() {
    let (code, message) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under 3 {{ }}"
    )));
    assert_eq!(code, "Q600");
    only(
        &message,
        MUTEX_NAME,
        &[
            FORM,
            NESTING,
            PAYLOAD_NAME,
            JUDGMENT,
            NOT_A_PROTECTED_RESOURCE,
        ],
    );
}

/// The reason these two arms were undriven, pinned so it cannot silently come
/// back: the wave-32 shape test's bodies never reach them. Each of its three
/// spellings stops at the length-and-`under` arm above, and none produces
/// either sentence.
#[test]
fn the_wave_32_shape_bodies_stop_above_both_token_arms() {
    for body in [
        "thread scope c share buf { }",
        "thread scope c share buf under { }",
        "thread scope c share under m { }",
    ] {
        let (code, message) = refusal(&entry(&format!("{DECLARATIONS} {body}")));
        assert_eq!(code, "Q600", "for {body}");
        only(&message, FORM, &[PAYLOAD_NAME, MUTEX_NAME]);
    }
}

/// And the arms really are *below* the nesting refusal, not above it: a nested
/// crossing whose payload position is also not a name reports the nesting arm.
/// This orders the four arms against each other rather than asserting each in
/// isolation.
#[test]
fn the_nesting_arm_still_precedes_both_token_arms() {
    let (code, message) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under m \
         {{ thread scope d share 3 under m {{ }} }}"
    )));
    assert_eq!(code, "Q600");
    only(&message, NESTING, &[PAYLOAD_NAME, MUTEX_NAME]);
}

// ---------------------------------------------------------------------------
// 2. The two dead arms: the non-local premise, pinned.
//
// These tests do NOT verify the arms — nothing can, from source, which is the
// finding. They pin the facts `QCore.MutexShareCrossArms` proves, in the Rust
// the lowering actually calls, so that a judgment change reviving either arm
// arrives as a red test instead of as a live-but-untested refusal. The proof
// is the justification; this is drift detection for it.
// ---------------------------------------------------------------------------

/// **Arm C** (`source_mutex.rs`, the `Ok(other)` arm) is unreachable:
/// `QCore.MutexShareCrossArms.theProtocolResidueIsEmpty`, mirrored over the
/// Rust vocabulary. For every kind there is, a mutex capture is either refused
/// — the lowering's `Err` arm — or is exactly the value the preceding `Ok` arm
/// matches. Nothing is left to bind `other`.
#[test]
fn the_protocol_residue_is_empty_over_every_kind() {
    let mut admitting = Vec::new();
    for kind in TransferKind::ALL {
        match admits_capture(
            kind,
            TransferMode::ShareUnderMutex,
            TransferKind::MutexShare,
        ) {
            Err(_) => {}
            Ok(AdmittedTransfer::ShareUnderMutex(ShareProtocol::CheckedMutex)) => {
                admitting.push(kind);
            }
            Ok(other) => panic!(
                "{kind:?} admits a mutex capture as {other:?}, which reaches the \
                 `Ok(other)` arm of `lower_declared_share`: that arm is documented \
                 unreachable and has no test behind it"
            ),
        }
    }
    // Non-vacuity. An empty admitting set would satisfy the loop above while
    // meaning the admitting path was dead too.
    assert_eq!(
        admitting,
        vec![TransferKind::ProtectedResource],
        "the mutex capture must still admit, and for exactly the payload"
    );
}

/// **Arm D** (`source_mutex.rs`, the `state.resource_of.get(...) else` arm) is
/// unreachable: `QCore.MutexShareCrossArms.theLookupAlwaysHits` supplies the
/// non-local half, mirrored here — no kind but the payload reaches the lookup.
///
/// The local half is not testable and is not asserted here: within
/// `lower_declared_share`, `place` is `ProtectedResource` only in the branch
/// guarded by `state.resource_of.contains_key(resource_name)`, and the lookup
/// that follows reads the same map at the same key. Since this card that is one
/// binding rather than two lookups, so it is read rather than argued.
#[test]
fn only_the_protected_payload_reaches_the_lookup() {
    for kind in TransferKind::ALL {
        let admitted = admits_capture(
            kind,
            TransferMode::ShareUnderMutex,
            TransferKind::MutexShare,
        )
        .is_ok();
        assert_eq!(
            admitted,
            kind == TransferKind::ProtectedResource,
            "{kind:?} disagrees with `theLookupAlwaysHits`: arm D of \
             `lower_declared_share` is dead only while the payload is the one \
             kind a mutex capture admits"
        );
    }
}

/// Arm D's deadness leans on *uniqueness* and on nothing structural:
/// `QCore.MutexShareCrossArms.theJudgmentCannotTellTwoKindsOfOneClassApart`.
/// The judgment reads the kind's class, so a second kind of the payload's class
/// would admit exactly as the payload does while naming no protected resource
/// of the module — which is what makes the backstop worth keeping rather than
/// simplifying away, and what `FB-SYSTEM-EXTENSIONS` would introduce.
#[test]
fn exactly_one_kind_carries_the_payloads_class() {
    let payload = qcore_core::transfer::transferable(TransferKind::ProtectedResource);
    let same: Vec<_> = TransferKind::ALL
        .into_iter()
        .filter(|kind| qcore_core::transfer::transferable(*kind) == payload)
        .collect();
    assert_eq!(
        same,
        vec![TransferKind::ProtectedResource],
        "a second kind of the payload's class makes arm D reachable"
    );
}
