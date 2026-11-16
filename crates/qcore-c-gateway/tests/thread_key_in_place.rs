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
//! * `&k`, `k` one automatic object: refused. An automatic key would be a
//!   tracked by-value object, and the card's gate found a statement route
//!   that reads one as data, so no key is tracked.
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

/// An automatic key is never tracked: its creation refuses (rule 11v (5)).
/// A tracked by-value key could reach a statement route that reads it as
/// data: the card's gate found `(void) pthread_getspecific(key)` admitted
/// after `pthread_key_delete(key)`. Every form refuses, so no read after a
/// delete can be certified.
#[test]
fn an_automatic_key_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "pthread_key_t key; int r; r = pthread_key_create(&key, 0); if (r) return; \
         pthread_key_delete(key);",
        "pthread_key_t key; int r; r = pthread_key_create(&key, 0); if (r) return;",
        "pthread_key_t key; pthread_key_create(&key, 0);",
        "pthread_key_t key; int r; r = pthread_key_create(&key, 0); if (r) return; \
         pthread_key_delete(key); (void) pthread_getspecific(key);",
        "pthread_key_t key; int r; r = pthread_key_create(&key, 0); if (r) return; \
         (void) pthread_getspecific(key); pthread_key_delete(key);",
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}: {run:?}");
        assert!(
            run.refused("thread key created into an object that is not of static storage duration"),
            "{body}: {run:?}"
        );
    }
}

/// A local shadowing a file-scope key is automatic, so it is refused, never
/// read as the static holder.
#[test]
fn a_local_shadowing_a_file_scope_key_is_refused() {
    if !clang_available() {
        return;
    }
    let run = run("pthread_key_t key;\n\
         void subject(void) { pthread_key_t key; int r; \
         r = pthread_key_create(&key, 0); if (r) return; }");
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    assert!(
        run.refused("thread key created into an object that is not of static storage duration"),
        "{run:?}"
    );
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
