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

//! Card CURRENT-CONTEXT-NEW-TRIAGE, D3 (gateway rule 11v (5)): the in-place
//! `thread_key` kind, read through the production libc declarations of
//! `pthread_key_create` / `pthread_key_delete` (`threads.qv`).
//!
//! * `&k`, `k` one automatic object: CG25's in-place protocol (status witness,
//!   by-value delete discharges, a missing delete leaks).
//! * `&G`, `G` of static storage duration: the key is held by static storage
//!   (P-THREAD-KEY-STATIC), so no frame obligation and no witness.
//! * Refused: a static key's delete, a thread-local key, a key reached
//!   through a pointer, an untracked delete. A non-NULL destructor leaves the
//!   call undeclared.
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
void destroy_value(void *value);
struct holder { pthread_key_t *key; };
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

    fn count(&self, predicate: impl Fn(&LegacyDiagnosticKind) -> bool) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| predicate(kind))
            .count()
    }
}

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

/// Monkey's real spelling, `NULL` = `((void *) 0)` on glibc, is outside the
/// literal-argument premise as §C04.5.1 states it (a cast's operand is not
/// read): the declaration does not apply there. Pinned as measured; see the
/// report's D3 finding.
#[test]
fn a_void_pointer_cast_null_destructor_is_outside_the_premise() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t key;\n\
         void subject(void) { pthread_key_create(&key, ((void *) 0)); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        run.refused("outside its declared literal-argument premise"),
        "{run:?}"
    );
}

#[test]
fn a_key_created_into_a_block_static_owes_nothing() {
    if !clang_available() {
        return;
    }
    let run = run("int subject(void) { static pthread_key_t key; \
         return pthread_key_create(&key, 0); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
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
    assert!(run.refused("in-place resource object"), "{run:?}");
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

/// An automatic key: created, tested, deleted by value.
#[test]
fn an_automatic_key_created_and_deleted_is_safe() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return; pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// The getter is not kinded in D3: naming a tracked key there is a data
/// route over tracked storage, refused (rule 11v (4)).
#[test]
fn a_getter_naming_a_tracked_key_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return; \
         (void) pthread_getspecific(key); pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
}

#[test]
fn an_automatic_key_never_deleted_leaks() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return; }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(
        run.count(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        1,
        "{run:?}"
    );
}

#[test]
fn an_automatic_key_deleted_twice_is_a_double_release() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return; \
         pthread_key_delete(key); pthread_key_delete(key); }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(
        run.count(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. })),
        1,
        "{run:?}"
    );
}

/// An automatic key's status must be witnessed (rule 11v (3), unchanged).
#[test]
fn an_automatic_key_with_a_discarded_status_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { pthread_key_t key; pthread_key_create(&key, 0); \
         pthread_key_delete(key); }",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(run.refused("in-place initialiser status"), "{run:?}");
}

/// A local shadowing a file-scope key is automatic, so it is tracked.
#[test]
fn a_local_shadowing_a_file_scope_key_is_tracked() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t key;\n\
         void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return; }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
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
