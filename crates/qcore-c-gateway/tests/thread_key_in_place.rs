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

//! Cards CURRENT-CONTEXT-NEW-TRIAGE, D3 (gateway rule 11v (5)), and
//! CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)): the in-place
//! `thread_key` kind, read through the production libc declarations of
//! `pthread_key_create` / `pthread_key_delete` / `pthread_getspecific`
//! (`threads.qv`).
//!
//! * `&k`, `k` the one automatic key object of the function: tracked as
//!   CG25's in-place object. Create and delete is `Safe`, and so is a getter
//!   between them where the creation status certifies the key. A getter
//!   after the delete is a `use_after_close` candidate. A missing delete is a
//!   `leak` candidate and a second delete a `double_release` candidate. These
//!   candidates are provisional, never upstream findings. Every copy or
//!   escape refuses (rule 11v (6)), and so does every uncertified getter
//!   (rule 11v (7)).
//! * `&G`, `G` of static storage duration: the key is held by static storage
//!   (P-THREAD-KEY-STATIC), so no frame obligation and no witness.
//! * Refused: a static key's delete, a thread-local key, a key reached
//!   through a pointer or held by a formal, a local whose name also
//!   designates a file-scope key the body names, an untracked delete. A
//!   non-NULL destructor leaves the call undeclared.
//!
//! A `Safe` here is `Safe` over the libc assumptions and P-THREAD-KEY-STATIC,
//! and promotes nothing else. Lean: `QCore.ThreadKeyStatic`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const THREADS: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/threads.qv");

const DECLARATIONS: &str = "\
typedef unsigned int pthread_key_t;
extern int pthread_key_create (pthread_key_t *__key, void (*__destr_function) (void *));
extern int pthread_key_delete (pthread_key_t __key);
extern void *pthread_getspecific (pthread_key_t __key);
extern int pthread_setspecific (pthread_key_t __key, const void *__pointer);
void destroy_value(void *value);
void use_key(pthread_key_t key);
struct holder { pthread_key_t *key; };
struct record { pthread_key_t key; };
pthread_key_t file_key;
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "libc/threads.qv".to_owned(),
        THREADS.to_owned(),
    )]))
    .expect("the libc threads module loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
    fn refused(&self, fragment: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description.contains(fragment))
        })
    }

    fn has(&self, wanted: fn(&LegacyDiagnosticKind) -> bool) -> bool {
        self.diagnostics.iter().any(wanted)
    }
}

/// `r = pthread_key_create(&key, 0); if (r) return;`, in a `void` body.
const CREATED: &str = "pthread_key_t key; int r; void *v; \
     r = pthread_key_create(&key, 0); if (r) return;";

const STRAY: &str = "tracked in-place object `key` named outside its declarator";
const UNCERTIFIED: &str =
    "use of thread key `key` not certified by its creation status (rule 11v (7))";
const STORAGE: &str = "thread key created into an object that is not of static storage duration";

fn run(source: &str) -> Run {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

/// `mk_http_thread_initialize_tls_once`'s shape with the destructor spelled
/// `0`: two keys created into file-scope objects, statuses discarded.
#[test]
fn keys_created_into_file_scope_objects_owe_nothing() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t first_key; pthread_key_t second_key;\n\
         void subject(void) { pthread_key_create(&first_key, 0); \
         pthread_key_create(&second_key, (0)); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// Monkey's real spelling, `NULL` = `((void *) 0)` on glibc: the C11
/// 6.3.2.3p3 null pointer constant, admitted at the pointer premise
/// (§C04.5.1, user decision 2026-11-16 "Admit (void *) 0 (Recommended)").
#[test]
fn a_void_pointer_null_destructor_meets_the_premise() {
    if !clang_available() {
        return;
    }
    for destructor in ["((void *) 0)", "(void *)0"] {
        let run = run(&format!(
            "pthread_key_t key;\nvoid subject(void) {{ pthread_key_create(&key, {destructor}); }}"
        ));
        assert_eq!(run.status, AnalysisStatus::Safe, "{destructor}: {run:?}");
    }
}

/// Another pointer type's null is not the null pointer constant `(void *) 0`
/// as §C04.5.1 reads it structurally: the declaration does not apply.
#[test]
fn a_differently_typed_null_destructor_is_outside_the_premise() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t key;\n\
         void subject(void) { pthread_key_create(&key, (void (*)(void *)) 0); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        run.refused("outside its declared literal-argument premise"),
        "{run:?}"
    );
}

/// A destructor is a registration the declaration does not state: the call
/// stays undeclared (C04.5's literal premise).
#[test]
fn a_non_null_destructor_leaves_the_call_undeclared() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t key;\n\
         void subject(void) { pthread_key_create(&key, destroy_value); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        run.refused("call to `pthread_key_create` outside its declared literal-argument premise"),
        "{run:?}"
    );
}

/// `mk_plugin_preworker_calls`' shape: the key is reached through a pointer.
#[test]
fn a_key_reached_through_a_pointer_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(struct holder *h) { pthread_key_create(h->key, 0); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    // Refused before rule 11v reads it: the access gate refuses the read
    // `h->key` handed to a custody position.
    assert!(
        run.refused("indirect memory access") || run.refused("in-place resource object"),
        "{run:?}"
    );
}

/// A thread-local key is per thread, not held by static storage.
#[test]
fn a_thread_local_key_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run("__thread pthread_key_t key;\n\
         void subject(void) { pthread_key_create(&key, 0); }");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// `mk_tls_exit`'s shape: a static key's delete is refused, never accused.
#[test]
fn a_static_keys_delete_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t key;\nvoid subject(void) { pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused("thread key held by static storage"), "{run:?}");
}

/// Rule 11v (7): the one automatic key object is tracked. Create and delete,
/// with the status certifying the key, is `Safe`, and so is a getter between
/// them (README §1 rows 1, 2, 15, 25, 26).
#[test]
fn an_automatic_key_created_and_deleted_is_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        format!("{CREATED} pthread_key_delete(key);"),
        format!("{CREATED} v = pthread_getspecific(key); (void) v; pthread_key_delete(key);"),
        format!("{CREATED} v = pthread_getspecific((key)); (void) v; pthread_key_delete((key));"),
        format!("{CREATED} r = 5; pthread_key_delete(key);"),
        "pthread_key_t key; int r; void *v; r = pthread_key_create(&key, 0); \
         if (r == 0) { v = pthread_getspecific(key); (void) v; pthread_key_delete(key); }"
            .to_owned(),
        "pthread_key_t key; int r; int i; for (i = 0; i < 4; i++) { \
         r = pthread_key_create(&key, 0); if (r) return; pthread_key_delete(key); }"
            .to_owned(),
        "pthread_key_t a; pthread_key_t b; int r; int s; \
         r = pthread_key_create(&a, 0); if (r) return; \
         s = pthread_key_create(&b, 0); if (s) { pthread_key_delete(a); return; } \
         pthread_key_delete(b); pthread_key_delete(a);"
            .to_owned(),
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Safe, "{body}: {run:?}");
    }
}

/// A getter after the delete is a `use_after_close` candidate (README §1
/// row 3), provisional and never an upstream finding. The spellings a
/// statement route lowers without the key's place refuse (row 4).
#[test]
fn a_getter_after_the_delete_is_a_candidate_or_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        format!("{CREATED} pthread_key_delete(key); v = pthread_getspecific(key); (void) v;"),
        format!("{CREATED} pthread_key_delete(key); pthread_getspecific(key);"),
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{body}: {run:?}");
        assert!(
            run.has(|kind| matches!(
                kind,
                LegacyDiagnosticKind::UseAfterClose {
                    kind: qcore_schemas::contract::ResourceKind::ThreadKey,
                    ..
                }
            )),
            "{body}: {run:?}"
        );
    }
    for body in [
        format!("{CREATED} pthread_key_delete(key); (void) pthread_getspecific(key);"),
        format!("{CREATED} (void) pthread_getspecific(key); pthread_key_delete(key);"),
        format!("{CREATED} pthread_key_delete(key); if (pthread_getspecific(key)) return;"),
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}: {run:?}");
        assert!(
            run.refused(
                "in-place actual `key` of `pthread_getspecific` is carried by no call node"
            ),
            "{body}: {run:?}"
        );
    }
}

/// Rule 11v (7): a getter the creation status has not certified refuses
/// (README §1 rows 31-33): before the test, on the failure edge, before any
/// creation. POSIX leaves such a use undefined.
#[test]
fn an_uncertified_getter_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "pthread_key_t key; int r; void *v; r = pthread_key_create(&key, 0); \
         v = pthread_getspecific(key); (void) v; if (r) return; pthread_key_delete(key);",
        "pthread_key_t key; int r; void *v; r = pthread_key_create(&key, 0); \
         if (r) { v = pthread_getspecific(key); (void) v; return; } pthread_key_delete(key);",
        "pthread_key_t key; int r; void *v; v = pthread_getspecific(key); (void) v; \
         r = pthread_key_create(&key, 0); if (r) return; pthread_key_delete(key);",
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}: {run:?}");
        assert!(run.refused(UNCERTIFIED), "{body}: {run:?}");
    }
}

/// A certified key left undeleted at an exit is a `leak` candidate (README
/// §1 rows 14, 16, 17, 22, 23): provisional, never an upstream finding.
#[test]
fn a_missing_delete_is_a_leak_candidate() {
    if !clang_available() {
        return;
    }
    for body in [
        CREATED.to_owned(),
        format!("{CREATED} if (c) return; pthread_key_delete(key);"),
        format!("{CREATED} if (c) pthread_key_delete(key);"),
        format!(
            "{CREATED} r = pthread_key_create(&key, 0); if (r) return; pthread_key_delete(key);"
        ),
        "pthread_key_t key; int r; int i; for (i = 0; i < c; i++) { \
         r = pthread_key_create(&key, 0); if (r) return; if (i) continue; \
         pthread_key_delete(key); }"
            .to_owned(),
    ] {
        let run = run(&format!("void subject(int c) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{body}: {run:?}");
        assert!(
            run.has(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
            "{body}: {run:?}"
        );
    }
}

/// A second delete is a `double_release` candidate (README §1 row 18).
#[test]
fn a_double_delete_is_a_candidate() {
    if !clang_available() {
        return;
    }
    let run = run(&format!(
        "void subject(void) {{ {CREATED} pthread_key_delete(key); pthread_key_delete(key); }}"
    ));
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(
        run.has(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. })),
        "{run:?}"
    );
}

/// Rule 11v (6): every copy or escape of a tracked key refuses (README §1
/// rows 5-12): a copy, an uncontracted callee, a member or global store, a
/// return, an address, a comparison, an expression actual, `&key` at the
/// by-value delete, and the setter (its declaration is withheld from the
/// function map, so its key is no custody actual).
#[test]
fn copy_and_escape_forms_are_refused() {
    if !clang_available() {
        return;
    }
    for (body, refusal) in [
        ("pthread_key_t k2 = key; (void) k2;", STRAY),
        ("pthread_key_t k2; k2 = key; (void) k2;", STRAY),
        ("use_key(key);", STRAY),
        ("file_key = key;", STRAY),
        ("{ struct record rec; rec.key = key; (void) rec; }", STRAY),
        ("{ pthread_key_t *p = &key; (void) p; }", STRAY),
        ("if (key == 0) return;", STRAY),
        ("v = pthread_getspecific(key + 0); (void) v;", STRAY),
        ("pthread_setspecific(key, \"value\");", STRAY),
    ] {
        let source = format!("void subject(void) {{ {CREATED} {body} pthread_key_delete(key); }}");
        let run = run(&source);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}: {run:?}");
        assert!(run.refused(refusal), "{body}: {run:?}");
    }
    let run = run("pthread_key_t subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return 0; return key; }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused(STRAY), "{run:?}");
}

/// The status is the sentinel (rule 11v (3)): a discarded or consumed status,
/// a delete before the test and a delete after a create on one path only
/// refuse (README §1 rows 20, 21, 24).
#[test]
fn an_unwitnessed_or_undecided_key_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "pthread_key_t key; pthread_key_create(&key, 0); pthread_key_delete(key);",
        "pthread_key_t key; if (pthread_key_create(&key, 0) != 0) return; pthread_key_delete(key);",
        "pthread_key_t key; int r; r = pthread_key_create(&key, 0); pthread_key_delete(key);",
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}: {run:?}");
    }
    let run = run("void subject(int c) { pthread_key_t key; int r; \
         if (c) { r = pthread_key_create(&key, 0); if (r) return; } pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
}

/// Rule 11v (7) tracks only the one automatic key object every reference
/// names. A formal's address, or a local whose name the body also uses for
/// the file-scope key it shadows, keeps D3's refusal; a local whose every
/// reference is its own is tracked, and its missing delete is a leak
/// candidate (README §1 rows 27-29).
#[test]
fn only_the_one_automatic_key_object_is_tracked() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(pthread_key_t key) { int r; r = pthread_key_create(&key, 0); \
         if (r) return; pthread_key_delete(key); }",
        "void subject(void) { { pthread_key_t file_key; int r; \
         r = pthread_key_create(&file_key, 0); if (r) return; } pthread_key_delete(file_key); }",
        "void subject(void) { void *v; v = pthread_getspecific(file_key); (void) v; \
         { pthread_key_t file_key; int r; r = pthread_key_create(&file_key, 0); if (r) return; \
         pthread_key_delete(file_key); } }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Unknown, "{source}: {run:?}");
        assert!(run.refused(STORAGE), "{source}: {run:?}");
    }
    let run = run("void subject(void) { pthread_key_t file_key; int r; \
         r = pthread_key_create(&file_key, 0); if (r) return; }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(
        run.has(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "{run:?}"
    );
}

/// An automatic key's non-NULL destructor leaves the call undeclared: no
/// destructor is ever registered on a tracked key, so no callback is left
/// for the run to decide.
#[test]
fn an_automatic_key_with_a_destructor_is_undeclared() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, destroy_value); if (r) return; pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        run.refused("call to `pthread_key_create` outside its declared literal-argument premise"),
        "{run:?}"
    );
}

/// The kinded getter changes nothing for a key no body tracks: a static
/// key's getter keeps the plain borrowed reading, as does a formal's.
#[test]
fn an_untracked_getter_keeps_the_borrowed_reading() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { void *v; v = pthread_getspecific(file_key); (void) v; }",
        "void *subject(pthread_key_t key) { return pthread_getspecific(key); }",
    ] {
        let run = run(source);
        assert_eq!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
    }
}

/// Deleting a key this body never created, held by no static name, refuses.
#[test]
fn an_untracked_delete_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(pthread_key_t key) { pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
}
