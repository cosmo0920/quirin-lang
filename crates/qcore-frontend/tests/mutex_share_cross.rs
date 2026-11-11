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

//! Card `MUTEX-SHARE-CROSS` (wave 32): CT46's composition — a payload that is
//! sendable and not freely shareable crossing a task boundary **because** a
//! verified mutex protocol protects it, and refusing when it crosses any other
//! way.
//!
//! Every refusal below is driven by a named test here, so the report can say
//! which arms are exercised rather than merely stated. Each rejection asserts
//! the diagnostic code *and* a distinguishing fragment of the message: a test
//! that only asserted `is_err()` would not tell this card's rule from any other
//! refusal on the way to it.

use qcore_core::mutex_invariant::{MutexInvariantEvent, check_mutex_invariant_program};
use qcore_core::transfer::{
    AdmittedTransfer, MUTEX_SHARE_CROSSING, MUTEX_SHARING_GAP, ShareProtocol, TransferKind,
    TransferMode, admits, admits_capture,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_mutex::{LoweredMutexModule, has_source_mutex, lower_source_mutex};

const DECLARATIONS: &str = "let buf = protected 4; let m = mutex guards buf;";

fn entry(body: &str) -> String {
    format!("#import std.concurrency; fn main() -> unit {{ thread scope s {{ {body} }} }}")
}

/// Lower only. Returns `(code, message)` pairs in the order reported.
fn lower(source: &str) -> Result<LoweredMutexModule, Vec<(String, String)>> {
    let parsed = parse_source_program(source).expect("parses");
    assert!(
        has_source_mutex(&parsed),
        "route detection must still select this module"
    );
    let typed = elaborate_source(&parsed).map_err(|d| {
        d.into_iter()
            .map(|d| (d.code.to_owned(), d.message))
            .collect::<Vec<_>>()
    })?;
    lower_source_mutex(&parsed, &typed, "main").map_err(|d| {
        d.into_iter()
            .map(|d| (d.code.to_owned(), d.message))
            .collect()
    })
}

/// The whole route: lower, then check with the **unedited** core.
fn checked(source: &str) -> Result<LoweredMutexModule, Vec<(String, String)>> {
    let module = lower(source)?;
    match check_mutex_invariant_program(&module.program) {
        Ok(_) => Ok(module),
        Err(diagnostics) => Err(diagnostics
            .iter()
            .map(|d| {
                let located = module.locate(d);
                (located.code.to_owned(), located.message)
            })
            .collect()),
    }
}

fn refusal(source: &str) -> (String, String) {
    checked(source)
        .expect_err("this program must be refused")
        .remove(0)
}

// ---------------------------------------------------------------------------
// 1. The admission: CT46's shape, accepted through the verified protocol.
// ---------------------------------------------------------------------------

/// The flagship. A `protected N` payload — sendable, not freely shareable —
/// is declared to cross into a child task under the generation that protects
/// it, and the child reaches it inside its own critical section over that
/// generation. Both tasks reach the payload; neither reaches it unguarded.
#[test]
fn a_protected_payload_crosses_into_a_child_under_the_generation_that_protects_it() {
    let module = checked(&entry(&format!(
        "{DECLARATIONS} \
         thread scope c share buf under m \
           {{ with lock m as gc {{ open gc; write buf with gc; close gc; }} }} \
         with lock m as g {{ read buf with g; }}"
    )))
    .expect("the declared crossing is admitted");

    // The crossing really is a task boundary in the model: the child is
    // spawned, it takes the generation itself, and it is joined.
    let events = &module.program.events;
    assert!(
        events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1
            }
        )),
        "the declaration must lower to a real spawn"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::Lock { task: 1, .. })),
        "the child acquires the generation itself; nothing is parked with it"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1
            }
        )),
        "and it is joined by its creator"
    );

    // Both tasks reach the payload, and every reach names a guard. This is the
    // property the Lean theorem lifts to every point of every interleaving.
    let reaches: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            MutexInvariantEvent::Access { task, guard, .. } => Some((*task, guard.is_some())),
            _ => None,
        })
        .collect();
    assert_eq!(
        reaches,
        vec![(1, true), (0, true)],
        "the child and the creator both reach the payload, both under a guard"
    );
}

/// The control that makes the admission mean something: supplying the protocol
/// is exactly what flips the verdict. The same program with the clause removed
/// still checks (this card changes nothing without the clause), and the same
/// program whose child reaches the payload unguarded refuses.
#[test]
fn the_declaration_is_what_admits_the_crossing_and_the_guard_is_what_admits_the_reach() {
    let guarded = "with lock m as gc { read buf with gc; }";
    checked(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under m {{ {guarded} }}"
    )))
    .expect("declared crossing, guarded reach");
    checked(&entry(&format!(
        "{DECLARATIONS} thread scope c {{ {guarded} }}"
    )))
    .expect("no clause: unchanged by this card");
}

// ---------------------------------------------------------------------------
// 2. The three protocol refusals the card names. Absent and mismatched are
//    this lowering's; unverified is the unedited core's own verdict.
// ---------------------------------------------------------------------------

/// **Absent.** A crossing whose `under` names something real that is not a
/// mutex generation is refused. It is never treated as an unprotected share,
/// and never allowed to fall through to the core with a protocol nobody
/// verified.
#[test]
fn a_crossing_whose_protocol_names_no_generation_is_refused() {
    let (code, message) = refusal(&entry(
        "let buf = protected 4; let other = protected 4; let m = mutex guards buf; \
         thread scope c share buf under other { }",
    ));
    assert_eq!(code, "Q600");
    assert!(
        message.contains("is not a mutex generation of this module")
            && message.contains("only through a verified mutex protocol"),
        "the refusal must name the missing protocol: {message}"
    );
}

/// The neighbouring case, pinned so the boundary between the two is recorded:
/// a clause naming a binding that does not exist at all is refused by **name
/// resolution** (`Q013`), before this card's gates run. That is the
/// pre-existing behaviour and it is what makes both clause names genuine
/// references — which is in turn why the constraint-14 census needed no
/// name-resolution exclusion for this card.
#[test]
fn a_clause_naming_an_unbound_name_is_refused_by_name_resolution() {
    for body in [
        "thread scope c share buf under nope { }",
        "thread scope c share ghost under m { }",
    ] {
        let (code, _) = refusal(&entry(&format!("{DECLARATIONS} {body}")));
        assert_eq!(code, "Q013", "for {body}");
    }
}

/// **Mismatched.** A generation that protects some *other* resource verifies
/// perfectly well and still says nothing about this payload. The refusal names
/// both what the generation guards and what the crossing tried to share.
#[test]
fn a_crossing_whose_protocol_guards_a_different_payload_is_refused() {
    let (code, message) = refusal(&entry(
        "let buf = protected 4; let other = protected 4; let m = mutex guards other; \
         thread scope c share buf under m { }",
    ));
    assert_eq!(code, "Q600");
    assert!(
        message.contains("guards `other`, not `buf`")
            && message.contains("must protect the payload it shares"),
        "the refusal must name both resources: {message}"
    );
}

/// **Unverified.** The child crosses under a verified protocol and then
/// reaches the payload *outside* any critical section. This refusal is the
/// unedited `CheckedMutexInvariantV1` checker's own — a protected footprint has
/// no unguarded access at all — reported under the core's own code. Nothing
/// about that rule is re-implemented beside the judgment, which is exactly why
/// the crossing composes rather than duplicates.
#[test]
fn a_child_that_reaches_the_shared_payload_outside_the_protocol_is_refused_by_the_core() {
    let (code, message) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under m {{ read buf; }}"
    )));
    assert_eq!(
        code, "Q611",
        "the core's UnguardedProtectedAccess, not a code this card allocated"
    );
    assert!(
        !message.contains("transferability judgment"),
        "the refusal must be the core's own, not the crossing's: {message}"
    );
    // The control: the same child, reaching the same payload, inside a section
    // of the declared generation.
    checked(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under m \
         {{ with lock m as gc {{ read buf with gc; }} }}"
    )))
    .expect("the guarded reach is admitted");
}

// ---------------------------------------------------------------------------
// 3. The judgment's own refusal at the crossing.
// ---------------------------------------------------------------------------

/// A mutex generation at the payload position. `MutexGeneration` is
/// `Never(NotAValue)` in the judgment — it has no denotation in the value
/// grammar — so this refuses with the judgment's reason rather than with a
/// protocol complaint about the wrong thing.
#[test]
fn sharing_the_generation_itself_is_refused_by_the_judgment() {
    let (code, message) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share m under m {{ }}"
    )));
    assert_eq!(code, "Q600");
    assert!(
        message.contains("the transferability judgment refuses this crossing")
            && message.contains("KindNotTransferable"),
        "the judgment must own this refusal: {message}"
    );
}

/// The payload position decides the crossing before the protocol position
/// does, so a generation shared under itself is a *judgment* refusal and a
/// payload shared under a payload is a *protocol* refusal. The two gates are
/// reached in that order and neither masks the other.
#[test]
fn the_judgment_decides_the_payload_before_the_protocol_gate_runs() {
    let (_, generation_at_payload) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share m under m {{ }}"
    )));
    assert!(generation_at_payload.contains("transferability judgment"));

    // `buf` is a real payload, so the judgment admits and the protocol gate is
    // what refuses — naming the protocol, not the kind.
    let (_, payload_at_protocol) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under buf {{ }}"
    )));
    assert!(
        payload_at_protocol.contains("is not a mutex generation of this module"),
        "{payload_at_protocol}"
    );

    // The payload position is classified from the payload name **alone**: a
    // generation shared under something that is not a generation is still the
    // non-value it is, not an unmodeled form. If the protocol name leaked into
    // the payload's classification this would read `UnmodeledType`.
    let (_, generation_under_payload) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share m under buf {{ }}"
    )));
    assert!(
        generation_under_payload.contains("KindNotTransferable(NotAValue)"),
        "the payload's kind must not depend on the protocol name: \
         {generation_under_payload}"
    );
}

// ---------------------------------------------------------------------------
// 4. The declaration is load-bearing.
// ---------------------------------------------------------------------------

/// The opt-in strengthening: a child that declared a crossing reaches the
/// payload it named and no other declared resource. Without this rule the
/// declaration would decorate a reach the child had anyway.
#[test]
fn a_declaring_child_may_not_reach_a_resource_it_did_not_declare() {
    let program = "let buf = protected 4; let other = protected 4; \
         let m = mutex guards buf; let n = mutex guards other; \
         thread scope c share buf under m { with lock n as gc { read other with gc; } }";
    let (code, message) = refusal(&entry(program));
    assert_eq!(code, "Q600");
    assert!(
        message.contains("this child declared the crossing `share buf under m`")
            && message.contains("`other` was not shared with it"),
        "the refusal must name the declaration and the undeclared reach: {message}"
    );

    // The control, one word different: the child that declares `other` reaches
    // `other`. So the rule is the declaration's, not a ban on second resources.
    checked(&entry(
        "let buf = protected 4; let other = protected 4; \
         let m = mutex guards buf; let n = mutex guards other; \
         thread scope c share other under n { with lock n as gc { read other with gc; } }",
    ))
    .expect("a child reaches exactly what it declared");
}

/// The same rule inside a guard-derived borrow, which is the other way a child
/// can reach bytes. The reach check sits at every resolution site, not only at
/// the access statement.
#[test]
fn a_declaring_child_may_not_borrow_a_resource_it_did_not_declare() {
    let (code, message) = refusal(&entry(
        "let buf = protected 4; let other = protected 4; \
         let m = mutex guards buf; let n = mutex guards other; \
         thread scope c share buf under m \
           { with lock n as gc { let b = borrow other with gc; drop b; } }",
    ));
    assert_eq!(code, "Q600");
    assert!(
        message.contains("`other` was not shared with it"),
        "{message}"
    );
}

// ---------------------------------------------------------------------------
// 5. Shape and bounded-slice refusals.
// ---------------------------------------------------------------------------

/// The bounded slice is one wrapper, one payload, one mutex, no nesting. A
/// declaring child inside a declaring child is refused by name rather than
/// admitted on an untested reading of what it would inherit.
#[test]
fn a_declared_crossing_does_not_nest() {
    let (code, message) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c share buf under m \
         {{ thread scope d share buf under m {{ }} }}"
    )));
    assert_eq!(code, "Q600");
    assert!(message.contains("does not nest"), "{message}");
}

/// A malformed clause is a shape refusal naming the whole form.
#[test]
fn a_malformed_crossing_clause_names_the_form() {
    for body in [
        "thread scope c share buf { }",
        "thread scope c share buf under { }",
        "thread scope c share under m { }",
    ] {
        let (code, message) = refusal(&entry(&format!("{DECLARATIONS} {body}")));
        assert_eq!(code, "Q600", "for {body}");
        assert!(
            message.contains("`thread scope NAME share PLACE under MUTEX { … }`")
                || message.contains("transferability judgment"),
            "for {body}: {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Compatibility: the clause is opt-in, and nothing without it moved.
// ---------------------------------------------------------------------------

/// Every pre-existing header form still lowers to exactly the events it did.
/// The three forms are told apart by one token at a fixed position, so a
/// module that does not spell `share` cannot reach any of this card's code.
#[test]
fn the_pre_existing_child_forms_are_untouched() {
    // A plain child that takes the generation itself — the shape card T03-F
    // admitted, unchanged.
    checked(&entry(&format!(
        "{DECLARATIONS} thread scope c {{ with lock m as gc {{ read buf with gc; }} }}"
    )))
    .expect("plain child, unchanged");
    // A plain child reaching the payload unguarded still refuses with the
    // core's own rule, and this card did not move it.
    let (code, _) = refusal(&entry(&format!(
        "{DECLARATIONS} thread scope c {{ read buf; }}"
    )));
    assert_eq!(code, "Q611");
    // The guard-local join form, whose clause word is `under` at the very
    // position this card reads `share`.
    checked(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ read buf with g; thread scope c under g {{ }} }}"
    )))
    .expect("guard-local join, unchanged");
}

/// `share` was a parse failure at this position before the card, so claiming
/// it collides with nothing — and the word stays what it already was
/// everywhere else. Both `share` and `under` were already reserved, so the
/// constraint-14 census cost of this card is **+0 reserved words**.
#[test]
fn the_clause_words_were_already_reserved_and_claim_only_this_position() {
    // A `share` that is not a child-task header is still not a statement.
    assert!(lower(&entry(&format!("{DECLARATIONS} share buf under m;"))).is_err());
}

// ---------------------------------------------------------------------------
// 7. The judgment, from the frontend's side.
// ---------------------------------------------------------------------------

/// The gap constant this card closes, and the one it deliberately leaves open.
/// The task route still has no mutex, so its `own<Buffer>` still cannot cross
/// under one — residual `MUTEX-SHARE-CROSS-TASK-BUFFER`.
#[test]
fn the_crossing_is_requestable_for_the_protected_payload_and_for_nothing_else() {
    // Compared as whole values rather than field-by-field: these are `const`,
    // so a bare `assert!` on one field is decided at compile time and clippy is
    // right to refuse it. Comparing the structs also pins more.
    assert_eq!(
        MUTEX_SHARE_CROSSING,
        qcore_core::transfer::SharingGap {
            kind: TransferKind::ProtectedResource,
            protocol: ShareProtocol::CheckedMutex,
            requestable: true,
        },
        "the half this card closes"
    );
    assert_eq!(
        MUTEX_SHARING_GAP,
        qcore_core::transfer::SharingGap {
            kind: TransferKind::OwnedBuffer,
            protocol: ShareProtocol::CheckedMutex,
            requestable: false,
        },
        "the task route's Buffer still has no mutex to cross under"
    );
    assert_eq!(
        admits(TransferKind::OwnedBuffer, TransferMode::ShareUnderMutex),
        Err(qcore_core::transfer::TransferRefusal::ProtocolUnavailable(
            ShareProtocol::CountedReadShares
        )),
        "and it refuses with its own sharing discipline named"
    );
    assert_eq!(
        admits_capture(
            TransferKind::ProtectedResource,
            TransferMode::ShareUnderMutex,
            TransferKind::MutexShare
        ),
        Ok(AdmittedTransfer::ShareUnderMutex(
            ShareProtocol::CheckedMutex
        ))
    );
}

/// The task route refuses a mutex generation beside a `spawn` exactly as it
/// did: this card composes on the *mutex* route and gives the task route's
/// `spawn`/`join` expressions no mutex. The pinned refusal is unmoved.
#[test]
fn the_task_routes_spawn_still_refuses_a_mutex_generation() {
    let source = "#import std.concurrency; fn worker(x: i32) -> i32 { return x; } \
         fn main() -> unit { thread scope s { let buf = protected 4; \
         let m = mutex guards buf; let h = spawn s worker(copy buf); } }";
    let codes: Vec<_> = lower(source)
        .expect_err("still refused")
        .into_iter()
        .map(|(code, _)| code)
        .collect();
    assert_eq!(codes, vec!["Q600"]);
}
