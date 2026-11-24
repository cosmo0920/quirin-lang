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

//! Card **FRAME-HOLDER-SHADOWED-NAME** (gateway rule 74o): a frame holder is
//! one declaration.
//!
//! Rule 12t's retention frame holder (`legacy_retained_by.rs`, `holder`) and
//! the callback registry's frame holder (`legacy_callback_route.rs`,
//! `frame_registry_holder`) read `&NAME` as storage of this frame when the
//! token census admits every mention and `one_automatic_object` holds. That
//! counts only the function's own `VarDecl`s, so a block-scope holder
//! shadowing a file-scope object the body also names was read as one frame
//! object: the file-scope object's `&NAME` became the frame holder. Probes
//! F1-F3 (retention) and F4 (registry) were `Safe` at base; F5 refused.
//! Both readers now need `every_reference_is` too, and every probe refuses.
//! The controls (a non-shadowing frame holder, and a shadowing local every
//! reference of which is its own) keep `Safe`.
//!
//! The `r_*` and `c_*` declarations are synthetic trusted test assumptions
//! (`fixtures/contracts/retained_by.qv`, `fixtures/contracts/callbacks.qv`):
//! a `Safe` here is `Safe` over them and promotes nothing.
//! Lean: `QCore.FrameHolderOneDeclaration`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const RETAINED: &str = "\
struct st { const char *base; int n; };
void *r_alloc(__SIZE_TYPE__ n); void r_free(void *p);
int r_keep(struct st *state, const char *src); void r_reset(struct st *state);
void r_global(const char *value);
struct st g;
";

const REGISTRY: &str = "\
#include <stddef.h>
struct list { void **elems; int cnt; void (*free_cb)(void *); };
void *c_alloc(__SIZE_TYPE__ n); void c_free(void *p);
void c_list_init(struct list *rl, int size, void (*free_cb)(void *));
void c_list_add(struct list *rl, void *elem);
void c_list_destroy(struct list *rl);
struct list gl;
";

fn load(name: &str, text: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), text.to_owned())]))
        .expect("the fixture contracts load")
}

fn retained() -> ResourceContracts {
    load(
        "retained_by.qcore",
        include_str!("fixtures/contracts/retained_by.qv"),
    )
}

fn callbacks() -> ResourceContracts {
    load(
        "callbacks.qv",
        include_str!("fixtures/contracts/callbacks.qv"),
    )
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent FRAME-HOLDER-SHADOWED-NAME test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Run {
    fn refused_with(&self, fragment: &str) -> bool {
        self.status != AnalysisStatus::Safe
            && self
                .unsupported
                .iter()
                .any(|description| description.contains(fragment))
    }
}

/// The row of `subject` in the unit `prelude + source`.
fn run(prelude: &str, source: &str, environment: &ResourceContracts) -> Run {
    let text = format!("{prelude}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        environment,
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
    let row = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: row.status,
        unsupported: row
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// Retention controls: a frame holder no other object shares a name with,
/// and a shadowing local whose every reference is its own (the file-scope `g`
/// is declared but the body never names it). Both `Safe` at base and after.
#[test]
fn a_frame_holder_that_is_one_declaration_stays_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(const char *src) { struct st s; return r_keep(&s, src); }",
        "int subject(const char *src) { struct st g; return r_keep(&g, src); }",
        "int subject(const char *src) { int r; { struct st g; r = r_keep(&g, src); } return r; }",
    ] {
        let run = run(RETAINED, source, &retained());
        assert_eq!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
    }
}

/// Probes F1 and F2: a caller's object retained by the file-scope `g`, after
/// the shadowing local's block closes (F1) or before its declarator (F2).
/// The file-scope `&g` was read as the frame holder, so a caller's object
/// "outlived" it and the row was `Safe`. Now the name is not one
/// declaration, the holder is undecided, and the row refuses.
#[test]
fn a_file_scope_holder_behind_a_shadowing_local_is_not_a_frame_holder() {
    if !clang_available() {
        return;
    }
    let mut admitted = Vec::new();
    for source in [
        "int subject(const char *src) { { struct st g; r_keep(&g, src); } return r_keep(&g, src); }",
        "int subject(const char *src) { r_keep(&g, src); { struct st g; return r_keep(&g, src); } }",
    ] {
        let run = run(RETAINED, source, &retained());
        if !run.refused_with("retention-undecided") {
            admitted.push(format!("{source}: {run:?}"));
        }
    }
    assert!(admitted.is_empty(), "{admitted:#?}");
}

/// Probe F3: a tracked object retained by the file-scope `g` and released
/// while that program-lifetime holder still holds it. The shadowing local's
/// reset dropped the file-scope record (one frame key `g`), so the release
/// was `Safe`. Now the retention refuses where it stands.
#[test]
fn a_shadowing_reset_never_ends_a_file_scope_record() {
    if !clang_available() {
        return;
    }
    let source = "void subject(void) { char *p = r_alloc(8); if (!p) return; r_keep(&g, p); { struct st g; r_reset(&g); } r_free(p); }";
    let run = run(RETAINED, source, &retained());
    assert!(run.refused_with("retention-undecided"), "{run:?}");
}

/// Registry control: a frame registry that is one declaration, beside a
/// file-scope `gl` the body never names. `Safe` at base and after.
#[test]
fn a_frame_registry_that_is_one_declaration_stays_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void) { struct list l; c_list_init(&l, 0, NULL); c_list_add(&l, \"x\"); c_list_destroy(&l); }",
        "void subject(void) { struct list gl; c_list_init(&gl, 0, NULL); c_list_add(&gl, \"x\"); c_list_destroy(&gl); }",
    ] {
        let run = run(REGISTRY, source, &callbacks());
        assert_eq!(run.status, AnalysisStatus::Safe, "{source}: {run:?}");
    }
}

/// Probes F4 and F5: the shadowing local registers no callback, and the file-scope
/// `gl`, whose callback this body never saw, is destroyed. The destroy was
/// read against the local's registration (one frame key `gl`), invoked
/// nothing and was `Safe` (F4). With the file-scope mention first (F5) the
/// add precedes any registration of the frame key, so F5 already refused at
/// base: the direction there is a refusal only, pinned so it stays one. Now
/// the file-scope `&gl` is no frame registry in either: the add and the
/// destroy refuse (`callback-registry-undecided`).
#[test]
fn a_file_scope_registry_behind_a_shadowing_local_is_not_a_frame_registry() {
    if !clang_available() {
        return;
    }
    let mut admitted = Vec::new();
    for source in [
        "void subject(void) { { struct list gl; c_list_init(&gl, 0, NULL); } c_list_add(&gl, \"x\"); c_list_destroy(&gl); }",
        "void subject(void) { c_list_add(&gl, \"x\"); { struct list gl; c_list_init(&gl, 0, NULL); c_list_destroy(&gl); } }",
    ] {
        let run = run(REGISTRY, source, &callbacks());
        if !run.refused_with("callback-registry-undecided") {
            admitted.push(format!("{source}: {run:?}"));
        }
    }
    assert!(admitted.is_empty(), "{admitted:#?}");
}
