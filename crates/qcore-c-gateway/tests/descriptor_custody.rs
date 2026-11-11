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

//! Card W43-DESCRIPTOR-RESOURCE-TRACKING — kind-indexed custody and
//! integer-carried identities.
//!
//! Every body below exercises one rule of contract spec §C04.6, and asserts
//! the **complete** diagnostic vector of the function it names together with
//! its status, so a rule that silently dropped a pre-existing leak or double
//! release while adding its own answer would fail.
//!
//! The fixtures use a generic `k_*` surface (`fixtures/contracts/
//! descriptor_generic.qcore`), deliberately unrelated to libc: `Safe` here is
//! `Safe` over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, ResourceKind, ResourceReturnMode, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/descriptor_generic.qv");

/// The C declarations the fixture contracts describe.
const DECLARATIONS: &str = "\
int k_open(const char *path); \
int k_close(int fd); \
__INTPTR_TYPE__ k_read(int fd, void *buffer, __SIZE_TYPE__ size); \
void *k_fopen(const char *path); \
int k_fclose(void *stream); \
int k_fflush(void *stream); \
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
struct k_conn { int fd; int flags; }; \
struct k_conn *k_conn_new(void); \
int g_fd;";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the descriptor fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent descriptor test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
        LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
        LegacyDiagnosticKind::CustodyConflict { .. } => "custody_conflict".to_owned(),
        LegacyDiagnosticKind::InvalidRelease { .. } => "invalid_release".to_owned(),
        LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
        LegacyDiagnosticKind::KindMismatchedRelease {
            acquired, released, ..
        } => format!(
            "kind_mismatched_release:{}<-{}",
            acquired.spelling(),
            released.spelling()
        ),
        LegacyDiagnosticKind::UseAfterClose { kind, .. } => {
            format!("use_after_close:{}", kind.spelling())
        }
        LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
    }
}

/// The status and the complete, sorted, de-duplicated diagnostic vector of
/// function `name`. The entry function `w` reports the program result, which
/// also rechecks a `Safe` certificate; any other function reports its own.
fn outcome_of(body: &str, name: &str) -> (&'static str, Vec<String>) {
    let result = analyze(body);
    let program_status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let function = program(&result)
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"));
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    let status = if name == "w" {
        program_status
    } else {
        match function.status {
            AnalysisStatus::Safe => "Safe",
            AnalysisStatus::Unsafe => "Unsafe",
            AnalysisStatus::Unknown => "Unknown",
            // Card W45-USER-CONTRACT-PLUGINS: no fixture here holds a call.
            AnalysisStatus::Pending => "Pending",
        }
    };
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    expect_function(label, body, "w", status, vector);
}

fn expect_function(label: &str, body: &str, name: &str, status: &str, vector: &[&str]) {
    let (measured, keys) = outcome_of(body, name);
    assert_eq!(
        (
            measured,
            keys.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

fn refused_route(name: &str) -> String {
    format!(
        "unsupported:tracked descriptor `{name}` read by a custody-neutral scalar route is \
         outside the resource projection"
    )
}

// -- The loader ---------------------------------------------------------------

#[test]
fn kinds_load_onto_their_positions_and_the_heap_is_the_default() {
    let contracts = contracts();
    let open = contracts.contract("k_open").expect("declared");
    assert_eq!(open.result, ResourceReturnMode::Owned { nullable: true });
    assert_eq!(open.result_kind, ResourceKind::Descriptor);
    let close = contracts.contract("k_close").expect("declared");
    assert_eq!(close.parameter_kind(0), ResourceKind::Descriptor);
    assert_eq!(
        contracts.contract("k_fopen").expect("declared").result_kind,
        ResourceKind::Stream
    );
    assert_eq!(
        contracts
            .contract("k_fclose")
            .expect("declared")
            .parameter_kind(0),
        ResourceKind::Stream
    );
    let free = contracts.contract("k_free").expect("declared");
    assert!(!free.has_non_heap_kind());
    assert_eq!(free.parameter_kind(0), ResourceKind::Heap);
}

/// `CT-CTWIN-DESCRIPTOR`: the C-twin shape the loader used to refuse with
/// "owned result requires a pointer return type" now loads, as a descriptor.
#[test]
fn the_ct_twin_descriptor_shape_is_expressible() {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "contracts.qcore".to_owned(),
        "module contract;\n\
         extern fn q_open_fd() -> i32\n\
         contract {\n    requires resource_contract_v1;\n    \
         ensures normal resource_return_owned(sentinel);\n    \
         ensures normal resource_return_kind(descriptor);\n};\n"
            .to_owned(),
    )]))
    .expect("an int-typed descriptor result loads");
    assert_eq!(
        contracts
            .contract("q_open_fd")
            .expect("declared")
            .result_kind,
        ResourceKind::Descriptor
    );
    // Without the kind, the pre-card refusal stands verbatim.
    let refusal = load_resource_contract_sources(&BTreeMap::from([(
        "contracts.qcore".to_owned(),
        "module contract;\n\
         extern fn q_open_fd() -> i32\n\
         contract {\n    requires resource_contract_v1;\n    \
         ensures normal resource_return_owned(nullable);\n};\n"
            .to_owned(),
    )]))
    .expect_err("an unkinded int result stays refused");
    assert!(
        refusal.contains("owned result requires a pointer return type"),
        "{refusal}"
    );
}

// -- Validity and the lifecycle --------------------------------------------------

#[test]
fn a_guarded_descriptor_closed_once_certifies() {
    if !clang_available() {
        return;
    }
    for guard in [
        "if (fd < 0) return;",
        "if (fd == -1) return;",
        "if (-1 == fd) return;",
        "if (fd <= -1) return;",
        "if (0 > fd) return;",
        "if (!(fd >= 0)) return;",
    ] {
        expect(
            guard,
            &format!("void w(void) {{ int fd = k_open(\"p\"); {guard} k_close(fd); }}"),
            "Safe",
            &[],
        );
    }
    for guard in ["if (fd >= 0)", "if (fd != -1)", "if (fd > -1)"] {
        expect(
            guard,
            &format!("void w(void) {{ int fd = k_open(\"p\"); {guard} {{ k_close(fd); }} }}"),
            "Safe",
            &[],
        );
    }
}

#[test]
fn an_unguarded_descriptor_is_not_valid_for_a_valid_position() {
    if !clang_available() {
        return;
    }
    expect(
        "no guard",
        "void w(void) { int fd = k_open(\"p\"); k_close(fd); }",
        "Unknown",
        &[
            "unknown_alias:fd",
            "unsupported:nullable resource `fd` used by a non-null contract",
        ],
    );
}

/// Descriptor `0` is valid, so a null-shaped test of a descriptor refines
/// nothing: after `if (!fd) return;` the failure value is still possible on
/// the fall-through, and the early return abandons a descriptor `0` — the
/// classic `!fd` bug, reported as the leak candidate it is.
#[test]
fn a_null_shaped_test_of_a_descriptor_refines_nothing() {
    if !clang_available() {
        return;
    }
    for guard in [
        "if (!fd) return;",
        "if (fd == 0) return;",
        "if (fd) {} else return;",
    ] {
        expect(
            guard,
            &format!("void w(void) {{ int fd = k_open(\"p\"); {guard} k_close(fd); }}"),
            "Unsafe",
            &[
                "leak",
                "unknown_alias:fd",
                "unsupported:nullable resource `fd` used by a non-null contract",
            ],
        );
    }
}

/// `fd > 0` proves validity on its true edge only: its false edge includes
/// descriptor `0`, which is never closed.
#[test]
fn a_positive_test_leaks_descriptor_zero() {
    if !clang_available() {
        return;
    }
    expect(
        "fd > 0",
        "void w(void) { int fd = k_open(\"p\"); if (fd > 0) { k_close(fd); } }",
        "Unsafe",
        &["leak"],
    );
}

#[test]
fn the_three_lifecycle_violations_are_candidates() {
    if !clang_available() {
        return;
    }
    expect(
        "leak",
        "void w(void) { int fd = k_open(\"p\"); if (fd < 0) return; }",
        "Unsafe",
        &["leak"],
    );
    expect(
        "double close",
        "void w(void) { int fd = k_open(\"p\"); if (fd < 0) return; k_close(fd); k_close(fd); }",
        "Unsafe",
        &["double_release"],
    );
    expect(
        "use after close",
        "void w(char *b) { int fd = k_open(\"p\"); if (fd < 0) return; \
         k_close(fd); k_read(fd, b, 1); }",
        "Unsafe",
        &["unknown_alias:fd", "use_after_close:descriptor"],
    );
    expect(
        "use before close",
        "void w(char *b) { int fd = k_open(\"p\"); if (fd < 0) return; \
         k_read(fd, b, 1); k_close(fd); }",
        "Safe",
        &[],
    );
    expect(
        "stream use after close",
        "void w(void) { void *s = k_fopen(\"p\"); if (!s) return; k_fclose(s); k_fflush(s); }",
        "Unsafe",
        &["unknown_alias:s", "use_after_close:stream"],
    );
}

/// A release discharges only an obligation of its own kind.
#[test]
fn a_release_of_another_kind_is_accused_and_discharges_nothing() {
    if !clang_available() {
        return;
    }
    expect(
        "free(fopen(...))",
        "void w(void) { void *s = k_fopen(\"p\"); if (!s) return; k_free(s); }",
        "Unsafe",
        &["kind_mismatched_release:stream<-heap", "leak"],
    );
    expect(
        "fclose(malloc(...))",
        "void w(void) { void *p = k_alloc(1); if (!p) return; k_fclose(p); }",
        "Unsafe",
        &["kind_mismatched_release:heap<-stream", "leak"],
    );
    expect(
        "the matching pairs certify",
        "void w(void) { void *s = k_fopen(\"p\"); void *p = k_alloc(1); \
         if (s) k_fclose(s); k_free(p); }",
        "Safe",
        &[],
    );
}

// -- The scalar routes: a tracked descriptor is custody, not data ---------------

/// Every custody-neutral scalar-store route refuses a tracked descriptor on
/// its right-hand side: the typed scalar-field no-op on a base this body never
/// binds (`p->f = fd`), W42's `ScalarMemberStore` on a base it does bind, a
/// local struct member (`s.f = fd`), a store through a pointer (`*p = fd`)
/// and a subscript store (`p[i] = fd`) — the four shapes
/// W43-INDIRECT-ACCESS-GENERAL admits for a *declared scalar* value.
#[test]
fn a_tracked_descriptor_on_every_scalar_store_route_is_refused() {
    if !clang_available() {
        return;
    }
    let open = "int fd = k_open(\"p\"); if (fd < 0) return;";
    let shapes = [
        (
            "unbound member (`p->f = fd`)",
            format!("void w(struct k_conn *c) {{ {open} c->fd = fd; k_close(fd); }}"),
        ),
        (
            "bound member (ScalarMemberStore)",
            "void w(void) { struct k_conn *c = k_conn_new(); if (!c) return; \
             int fd = k_open(\"p\"); if (fd < 0) { k_free(c); return; } \
             c->fd = fd; k_close(fd); k_free(c); }"
                .to_owned(),
        ),
        (
            "local struct member (`s.f = fd`)",
            format!("void w(void) {{ struct k_conn s; {open} s.fd = fd; k_close(fd); }}"),
        ),
        (
            "through a pointer (`*p = fd`)",
            format!("void w(int *p) {{ {open} *p = fd; k_close(fd); }}"),
        ),
        (
            "subscript (`p[i] = fd`)",
            format!("void w(int *p, int i) {{ {open} p[i] = fd; k_close(fd); }}"),
        ),
    ];
    for (label, body) in shapes {
        let (status, keys) = outcome_of(&body, "w");
        assert_eq!(status, "Unknown", "{label}: {keys:?}\n{body}");
        assert!(
            !keys.iter().any(|key| key == "leak"),
            "{label} must not become an accusation: {keys:?}"
        );
        assert!(
            keys.iter().any(|key| key.starts_with("unsupported:")),
            "{label} must refuse: {keys:?}"
        );
    }
    // Every one of these shapes is admitted as custody-neutral for an ordinary
    // scalar — by W42's `ScalarMemberStore`, the typed scalar-field no-op or
    // W43-INDIRECT-ACCESS-GENERAL's `IndirectScalarAccess` — so each reaches
    // the descriptor guard itself, which refuses it by name.
    for body in [
        format!("void w(struct k_conn *c) {{ {open} c->fd = fd; k_close(fd); }}"),
        format!(
            "void w(void) {{ struct k_conn *c = k_conn_new(); if (!c) return; {open} \
             c->fd = fd; k_close(fd); k_free(c); }}"
        ),
        format!("void w(void) {{ struct k_conn s; {open} s.fd = fd; k_close(fd); }}"),
        format!("void w(int *p) {{ {open} *p = fd; k_close(fd); }}"),
        format!("void w(int *p, int i) {{ {open} p[i] = fd; k_close(fd); }}"),
    ] {
        let (_, keys) = outcome_of(&body, "w");
        assert!(keys.contains(&refused_route("fd")), "{keys:?}\n{body}");
    }
    // The same stores of an ordinary integer stay admitted, through each route:
    // the guard refuses the descriptor, not the shape.
    for body in [
        "void w(struct k_conn *c, int n) { c->flags = n; }",
        "void w(void) { struct k_conn s; int n = 1; s.fd = n; }",
        "void w(int *p, int n) { *p = n; }",
        "void w(int *p, int i, int n) { p[i] = n; }",
    ] {
        expect("an ordinary int through a scalar route", body, "Safe", &[]);
    }
}

/// Publication into a file-scope object: the declared-scalar source exemption
/// does not cover a scalar that holds a tracked descriptor.
#[test]
fn publishing_a_tracked_descriptor_is_refused_not_admitted_as_a_scalar() {
    if !clang_available() {
        return;
    }
    let (status, keys) = outcome_of(
        "void w(void) { int fd = k_open(\"p\"); if (fd < 0) return; g_fd = fd; }",
        "w",
    );
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(!keys.iter().any(|key| key == "leak"), "{keys:?}");
    // An ordinary scalar keeps the exemption.
    expect(
        "an untracked scalar still publishes nothing",
        "void w(int n) { g_fd = n; }",
        "Safe",
        &[],
    );
}

/// Arithmetic on a descriptor creates and aliases no identity; a copy aliases
/// the same one; a return escapes it.
#[test]
fn arithmetic_copies_and_returns_state_their_effect_exactly() {
    if !clang_available() {
        return;
    }
    expect(
        "arithmetic",
        "int w(void) { int fd = k_open(\"p\"); int n; if (fd < 0) return 0; n = fd + 1; \
         k_close(fd); return n; }",
        "Safe",
        &[],
    );
    expect(
        "a copy is the same identity",
        "void w(void) { int fd = k_open(\"p\"); int g; if (fd < 0) return; g = fd; k_close(g); }",
        "Safe",
        &[],
    );
    expect(
        "a copy closed twice",
        "void w(void) { int fd = k_open(\"p\"); int g; if (fd < 0) return; g = fd; \
         k_close(g); k_close(fd); }",
        "Unsafe",
        &["double_release"],
    );
    expect(
        "a validated descriptor is returned",
        "int w(void) { int fd = k_open(\"p\"); if (fd < 0) return -1; return fd; }",
        "Safe",
        &[],
    );
    expect(
        "overwriting a live descriptor loses it",
        "void w(void) { int fd = k_open(\"p\"); if (fd < 0) return; fd = -1; }",
        "Unsafe",
        &["leak"],
    );
}

/// A validated W4 summary was derived with its integer formals untracked, so a
/// helper that stores its formal still earned `Borrowed`; handing it a tracked
/// descriptor is refused, not read as a use.
#[test]
fn a_descriptor_passed_where_no_declaration_states_the_effect_is_refused() {
    if !clang_available() {
        return;
    }
    let body = "void keep(struct k_conn *c, int x) { c->fd = x; } \
                void w(struct k_conn *c) { int fd = k_open(\"p\"); if (fd < 0) return; \
                keep(c, fd); k_close(fd); }";
    let (status, keys) = outcome_of(body, "w");
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(
        keys.iter().any(|key| key.contains("passed to `keep`")),
        "{keys:?}"
    );
}

/// W4 never publishes a heap summary for a body that releases a formal with a
/// descriptor release, or returns a descriptor: the wrapper and its caller stay
/// `Unknown` instead of being credited or accused.
#[test]
fn no_heap_summary_is_inferred_for_a_descriptor_wrapper() {
    if !clang_available() {
        return;
    }
    let body = "void my_close(int x) { k_close(x); } \
                int my_open(void) { int fd = k_open(\"p\"); if (fd < 0) return -1; return fd; } \
                void w(void) { int fd = my_open(); if (fd < 0) return; my_close(fd); }";
    let (status, keys) = outcome_of(body, "w");
    assert_eq!(status, "Unknown", "{keys:?}");
    assert!(
        keys.iter()
            .any(|key| key == "unsupported:call to uncontracted function `my_open`"),
        "{keys:?}"
    );
    let (wrapper, wrapper_keys) = outcome_of(body, "my_close");
    assert_eq!(wrapper, "Unknown", "{wrapper_keys:?}");
    // The opener certifies on its own, but publishes no summary.
    expect_function("my_open", body, "my_open", "Safe", &[]);
}

/// This card measured a boundary here: the return route read `return (T *)p;`
/// as returning no place, so an acquisition returned through a cast was
/// reported as a leak — for the heap exactly as for a stream — and the card
/// withheld `dlopen`/`dlclose` on it (`RETURN-CAST-PLACE-NOT-ESCAPED`). Card
/// W44-LEAK-PIN-AUDIT discharged that residual: a pointer cast of one place
/// now escapes it exactly as the uncast return does, for every pointer kind
/// (`legacy_cast_return.rs`, `QCore.CastReturnEscape`). The pin was flipped
/// deliberately; `tests/leak_pin_audit.rs` carries the carrier-crossing cases.
#[test]
fn a_cast_return_escapes_for_every_pointer_kind() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *w(void) { void *p = k_alloc(1); if (!p) return 0; return (char *)p; }",
        "void *w(void) { void *p = k_fopen(\"p\"); if (!p) return 0; return (char *)p; }",
    ] {
        expect("cast return", body, "Safe", &[]);
    }
    // Without the cast the same bodies escape their identity and certify.
    for body in [
        "void *w(void) { void *p = k_alloc(1); if (!p) return 0; return p; }",
        "void *w(void) { void *p = k_fopen(\"p\"); if (!p) return 0; return p; }",
    ] {
        expect("plain return", body, "Safe", &[]);
    }
}

/// A heap-only body is analysed exactly as before this card.
#[test]
fn a_heap_only_body_is_unchanged() {
    if !clang_available() {
        return;
    }
    expect(
        "heap",
        "void w(void) { void *p = k_alloc(1); if (!p) return; k_free(p); }",
        "Safe",
        &[],
    );
    expect(
        "heap leak",
        "void w(void) { void *p = k_alloc(1); if (!p) return; }",
        "Unsafe",
        &["leak"],
    );
}
