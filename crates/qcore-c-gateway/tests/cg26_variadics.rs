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

//! Card CG26-VARIADICS (gateway rule 11w): the `va_list` protocol and the
//! trailing actuals, through route G (`analyze_unrestricted_c11`).
//!
//! The `va_*` half runs over the CG00 fixture contracts (`support/clang_gap.rs`)
//! plus one synthetic `cg_vlog(const char *, va_list)` borrower; the tail
//! half also over the production `fcntl` declaration of
//! `quirin-provider-libc` (`libc/unistd.qv`). Every `Safe` is over those
//! trusted assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/clang_gap.rs"]
mod clang_gap;
#[path = "support/text.rs"]
mod text;

use clang_gap::{G, clang_available, g};
use text::normalize_newlines;

const UNISTD: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/unistd.qv");

const VLOG: &str = "module contract;\n\
extern fn cg_vlog(format: ptr<const, u8>, list: va_list) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_borrowed(format);\n\
    ensures normal resource_borrowed(list);\n\
    ensures normal resource_return_none();\n\
};\n\
extern fn cg_blog(format: ptr<const, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_borrowed(format);\n\
    ensures normal resource_return_none();\n\
    ensures normal resource_variadic_borrowed_tail();\n\
};\n";

const STDARG: &str = "#include <stdarg.h>\nvoid cg_vlog(const char *format, va_list list);\n\
void cg_blog(const char *format, ...);\n";

fn protocol(reason: &str) -> String {
    format!("va_list protocol refused: {reason}")
}

const UNCONTRACTED_START: &str = "call to uncontracted function `__builtin_va_start`";
const UNCONTRACTED_END: &str = "call to uncontracted function `__builtin_va_end`";

/// The CG00 harness's run over `STDARG` + `source`.
fn va(source: &str) -> G {
    g(&format!("{STDARG}{source}"))
}

// ---------------------------------------------------------------------------
// The `va_list` protocol (K09, W44-VA-START-END-BUILTINS)
// ---------------------------------------------------------------------------

/// The CG00 BEFORE control's refusals are replaced only when the protocol is
/// established; without `cg_vlog` declared, the pass stays uncontracted.
#[test]
fn a_started_and_ended_list_is_custody_neutral() {
    if !clang_available() {
        return;
    }
    let run = va(
        "void subject(const char *fmt, ...) { va_list ap; void *p = q_alloc(); \
         va_start(ap, fmt); va_end(ap); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    assert!(!run.refused(UNCONTRACTED_START) && !run.refused(UNCONTRACTED_END));
}

#[test]
fn scalar_va_args_in_a_loop_are_established() {
    if !clang_available() {
        return;
    }
    let run = va(
        "int subject(int n, ...) { va_list ap; int total = 0; int x; int i; va_start(ap, n); \
         for (i = 0; i < n; i++) { x = va_arg(ap, int); total = total + x; } \
         va_end(ap); return total; }",
    );
    assert!(run.safe(), "{run:?}");
}

#[test]
fn a_leak_beside_an_established_protocol_is_still_reported() {
    if !clang_available() {
        return;
    }
    let run = va(
        "void subject(const char *fmt, ...) { va_list ap; void *p = q_alloc(); \
         va_start(ap, fmt); va_end(ap); }",
    );
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("Leak"), "{run:?}");
}

#[test]
fn a_missing_va_end_is_never_safe() {
    if !clang_available() {
        return;
    }
    let run = va("void subject(const char *fmt, ...) { va_list ap; va_start(ap, fmt); }");
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(&protocol("`ap` may reach a function exit without `va_end`")),
        "{run:?}"
    );
}

#[test]
fn a_return_between_start_and_end_is_a_missing_end() {
    if !clang_available() {
        return;
    }
    let run =
        va("void subject(int n, ...) { va_list ap; va_start(ap, n); if (n) return; va_end(ap); }");
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(&protocol("`ap` may reach a function exit without `va_end`")),
        "{run:?}"
    );
}

#[test]
fn a_copy_without_its_own_end_is_never_safe() {
    if !clang_available() {
        return;
    }
    let run = va(
        "void subject(const char *fmt, ...) { va_list ap; va_list aq; va_start(ap, fmt); \
         va_copy(aq, ap); va_end(ap); }",
    );
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(&protocol("`aq` may reach a function exit without `va_end`")),
        "{run:?}"
    );
}

#[test]
fn a_copy_with_its_own_end_is_established() {
    if !clang_available() {
        return;
    }
    let run = va(
        "int subject(const char *fmt, ...) { va_list ap; va_list aq; int x; va_start(ap, fmt); \
         va_copy(aq, ap); x = va_arg(aq, int); va_end(aq); va_end(ap); return x; }",
    );
    assert!(run.safe(), "{run:?}");
}

#[test]
fn a_use_after_va_end_is_never_safe() {
    if !clang_available() {
        return;
    }
    let run = va(
        "int subject(int n, ...) { va_list ap; int x; va_start(ap, n); va_end(ap); \
         x = va_arg(ap, int); return x; }",
    );
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(&protocol(
            "`ap` may be read after `va_end`, after being passed to a callee, or before `va_start`"
        )),
        "{run:?}"
    );
}

#[test]
fn a_second_va_start_before_end_is_never_safe() {
    if !clang_available() {
        return;
    }
    let run = va(
        "void subject(int n, ...) { va_list ap; va_start(ap, n); va_start(ap, n); va_end(ap); }",
    );
    assert!(
        run.refused(&protocol(
            "`va_start`/`va_copy` into `ap`, which may already be started"
        )),
        "{run:?}"
    );
}

/// The early-exit cleanup shape: every `goto` reaches the one `va_end`.
#[test]
fn a_goto_to_the_ending_label_is_established() {
    if !clang_available() {
        return;
    }
    let run = va(
        "int subject(int n, ...) { va_list ap; int x = 0; va_start(ap, n); \
         if (n < 0) goto out; x = va_arg(ap, int); out: va_end(ap); return x; }",
    );
    assert!(run.safe(), "{run:?}");
}

/// A list handed whole to a callee is indeterminate (7.16p3): the second pass
/// and a `va_arg` after it are refused.
#[test]
fn a_second_pass_is_never_safe() {
    if !clang_available() {
        return;
    }
    let contracts = with_vlog();
    let run = g_with(
        &contracts,
        "void subject(const char *fmt, ...) { va_list ap; va_start(ap, fmt); cg_vlog(fmt, ap); \
         cg_vlog(fmt, ap); va_end(ap); }",
    );
    assert_eq!(run.status, AnalysisStatus::Unknown, "{run:?}");
    let once = g_with(
        &contracts,
        "void subject(const char *fmt, ...) { va_list ap; va_start(ap, fmt); cg_vlog(fmt, ap); \
         va_end(ap); }",
    );
    assert_eq!(once.status, AnalysisStatus::Safe, "{once:?}");
}

/// A parameter list is read and passed, never ended here.
#[test]
fn a_parameter_list_owes_no_end_and_takes_none() {
    if !clang_available() {
        return;
    }
    let read =
        va("int subject(const char *fmt, va_list ap) { int x; x = va_arg(ap, int); return x; }");
    assert!(read.safe(), "{read:?}");
    let ended = va("void subject(const char *fmt, va_list ap) { va_end(ap); }");
    assert!(
        ended.refused(&protocol(
            "`va_start`, `va_copy` into or `va_end` on the parameter list `ap`"
        )),
        "{ended:?}"
    );
}

/// A pointer `va_arg` is a caller pointer with no identity premise (CG00's
/// K09 control). **Flipped by VA-LOOP-AND-POINTER-ARG (rule 12c)**: the
/// value is a never-owned opaque binding, so its named refusal is gone and
/// `q_free(p)` stays an unresolved alias: never `Safe`.
#[test]
fn a_pointer_va_arg_is_a_never_owned_binding() {
    if !clang_available() {
        return;
    }
    let run = va("void subject(int n, ...) { va_list ap; va_start(ap, n); \
         void *p = va_arg(ap, void *); va_end(ap); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
    assert!(
        !run.refused_prefix("va_list operation refused: the `va_arg` value stored into `p`"),
        "{run:?}"
    );
    assert!(run.has("UnknownAlias { variable: \"p\" }"), "{run:?}");
}

/// A list event under a conditional operator, and the address of a list
/// outside a whole call argument, are refused by name.
#[test]
fn unread_list_uses_refuse_by_name() {
    if !clang_available() {
        return;
    }
    let conditional = va(
        "int subject(int n, ...) { va_list ap; int x; va_start(ap, n); \
         x = n ? va_arg(ap, int) : 0; va_end(ap); return x; }",
    );
    assert!(
        conditional.refused(&protocol(
            "a `va_list` operation under a conditional operator or an unevaluated operand"
        )),
        "{conditional:?}"
    );
    let escaped = va(
        "va_list *keep; void subject(int n, ...) { va_list ap; va_start(ap, n); keep = &ap; va_end(ap); }",
    );
    assert!(
        escaped.refused(&protocol(
            "`ap` is used outside `va_start`, `va_arg`, `va_copy`, `va_end` or a whole call argument"
        )),
        "{escaped:?}"
    );
}

/// `va_start` must name the last parameter.
#[test]
fn va_start_names_the_last_parameter() {
    if !clang_available() {
        return;
    }
    let run = va("void subject(int a, int b, ...) { va_list ap; va_start(ap, a); va_end(ap); }");
    assert!(
        run.refused(&protocol(
            "`va_start` does not name the last parameter of a variadic function"
        )),
        "{run:?}"
    );
}

// ---------------------------------------------------------------------------
// Trailing actuals (K08, W46-FCNTL-SETFD-ADDRESS-TAIL)
// ---------------------------------------------------------------------------

/// A variadic logger declaring `resource_variadic_borrowed_tail()`: a tracked,
/// live identity in its tail is a borrowed read, the call is admitted and the
/// identity's own obligation stays. Under the data tail (`cg_log`) the same
/// actual keeps the O04-TRACE refusal.
#[test]
fn a_live_identity_in_a_borrowed_tail_is_borrowed() {
    if !clang_available() {
        return;
    }
    let contracts = with_vlog();
    let run = g_with(
        &contracts,
        "void subject(void) { void *p = q_alloc(); cg_blog(\"%p %d\", p, 3); q_free(p); }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{:?}", run.diagnostics);
    let leaking = g_with(
        &contracts,
        "void subject(void) { void *p = q_alloc(); cg_blog(\"%p\", p); }",
    );
    assert_ne!(
        leaking.status,
        AnalysisStatus::Safe,
        "{:?}",
        leaking.diagnostics
    );
    assert!(
        leaking
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "{:?}",
        leaking.diagnostics
    );
    let data_tail = g("void subject(void) { void *p = q_alloc(); cg_log(\"%p\", p); q_free(p); }");
    assert!(
        data_tail.refused(
            "arguments beyond the declared parameters of `cg_log` are outside the resource projection"
        ),
        "{data_tail:?}"
    );
}

/// A released identity in the borrowed tail is a use-after-release
/// candidate, never `Safe`; the address of a pointer keeps the refusal (a
/// record by value is decided by the promoted-type unit test).
#[test]
fn the_borrowed_tail_keeps_its_obligations_and_refusals() {
    if !clang_available() {
        return;
    }
    let contracts = with_vlog();
    for body in [
        "void subject(void) { void *p = q_alloc(); q_free(p); cg_blog(\"%p\", p); }",
        "void subject(void) { void *p = q_alloc(); cg_blog(\"%p\", &p); q_free(p); }",
    ] {
        let run = g_with(&contracts, body);
        assert_ne!(
            run.status,
            AnalysisStatus::Safe,
            "{body}: {:?}",
            run.diagnostics
        );
    }
}

fn fcntl_contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "libc/unistd.qv".to_owned(),
        normalize_newlines(UNISTD),
    )]))
    .expect("libc/unistd.qv loads")
}

const FCNTL: &str = "int open(const char *file, int oflag, ...); int close(int fd); \
int fcntl(int fd, int cmd, ...);\n#define F_SETFD 2\n#define FD_CLOEXEC 1\n";

fn fcntl_run(body: &str) -> LegacyFunctionSummary {
    let source = format!("{FCNTL}{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "k",
        &fcntl_contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary: &LegacyProgramSummary = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no summary: {other:?}"),
    };
    summary.functions["k"].clone()
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

/// `fcntl(fd, F_SETFD, FD_CLOEXEC)` with the production declaration.
#[test]
fn fcntl_at_f_setfd_with_its_contract_is_safe() {
    if !clang_available() {
        return;
    }
    let summary = fcntl_run(
        "void k(const char *path) { int fd; fd = open(path, 0); if (fd == -1) { return; } \
         fcntl(fd, F_SETFD, FD_CLOEXEC); close(fd); }",
    );
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
}

/// W46-FCNTL-SETFD-ADDRESS-TAIL: `rd_kafka_open_cb_generic`'s extra `&on`.
#[test]
fn fcntl_with_the_address_of_a_scalar_local_in_its_tail_is_admitted() {
    if !clang_available() {
        return;
    }
    let summary = fcntl_run(
        "void k(const char *path) { int fd; int on = 1; fd = open(path, 0); \
         if (fd == -1) { return; } fcntl(fd, F_SETFD, FD_CLOEXEC, &on); close(fd); }",
    );
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
    // The descriptor's own obligation is unchanged: dropping `close` leaks.
    let leaking = fcntl_run(
        "void k(const char *path) { int fd; int on = 1; fd = open(path, 0); \
         if (fd == -1) { return; } fcntl(fd, F_SETFD, FD_CLOEXEC, &on); }",
    );
    assert_ne!(
        leaking.status,
        AnalysisStatus::Safe,
        "{:?}",
        leaking.diagnostics
    );
}

/// The address of a tracked descriptor, or of a pointer, keeps the refusal.
#[test]
fn the_address_of_a_descriptor_or_a_pointer_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let descriptor = fcntl_run(
        "void k(const char *path) { int fd; fd = open(path, 0); if (fd == -1) { return; } \
         fcntl(fd, F_SETFD, FD_CLOEXEC, &fd); close(fd); }",
    );
    assert_ne!(
        descriptor.status,
        AnalysisStatus::Safe,
        "{:?}",
        descriptor.diagnostics
    );
    let pointer = fcntl_run(
        "void k(const char *path, char *q) { int fd; fd = open(path, 0); if (fd == -1) { return; } \
         fcntl(fd, F_SETFD, FD_CLOEXEC, &q); close(fd); }",
    );
    assert_ne!(
        pointer.status,
        AnalysisStatus::Safe,
        "{:?}",
        pointer.diagnostics
    );
    assert!(
        unsupported(&pointer)
            .iter()
            .any(|d| d == "arguments beyond the declared parameters of `fcntl` are outside the resource projection"),
        "{:?}",
        pointer.diagnostics
    );
}

/// The scalar written through `&on` loses its fact: a later test of it is not
/// refined by its earlier zero.
#[test]
fn a_scalar_lent_to_the_tail_is_forgotten() {
    if !clang_available() {
        return;
    }
    let summary = fcntl_run(
        "void k(const char *path) { int fd; int on = 0; fd = open(path, 0); \
         if (fd == -1) { return; } fcntl(fd, F_SETFD, FD_CLOEXEC, &on); \
         if (on) { return; } close(fd); }",
    );
    // With `on`'s zero kept, the `return` arm would be pruned and the body
    // Safe; forgotten, that arm leaks the descriptor, and nothing is refused.
    assert_ne!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
    assert!(
        unsupported(&summary).is_empty(),
        "{:?}",
        summary.diagnostics
    );
    assert!(
        summary
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "{:?}",
        summary.diagnostics
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn with_vlog() -> ResourceContracts {
    let mut sources = BTreeMap::new();
    sources.insert("cg_vlog.qcore".to_owned(), VLOG.to_owned());
    for (name, text) in clang_gap::contract_sources() {
        sources.insert(name, text);
    }
    load_resource_contract_sources(&sources).expect("fixture contracts load")
}

fn g_with(contracts: &ResourceContracts, source: &str) -> LegacyFunctionSummary {
    let text = format!("{}\n{STDARG}{source}\n", clang_gap::DECLARATIONS);
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            certificate.summary.functions["subject"].clone()
        }
        UnrestrictedLegacyResult::Unsafe(summary) => summary.functions["subject"].clone(),
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary.functions["subject"].clone(),
        other => panic!("no summary: {other:?}"),
    }
}

/// The borrowed tail is the data tail strengthened: it is rederived from the
/// retained texts, it implies the data tail, and the two never stand together.
#[test]
fn the_borrowed_tail_clause_loads_as_a_strengthened_data_tail() {
    use qcore_contract::contract_loader::{variadic_borrowed_tails, variadic_data_tails};
    let sources = BTreeMap::from([("cg_vlog.qcore".to_owned(), VLOG.to_owned())]);
    assert_eq!(
        variadic_borrowed_tails(&sources).expect("loads"),
        ["cg_blog".to_owned()].into_iter().collect()
    );
    assert!(
        variadic_data_tails(&sources)
            .expect("loads")
            .contains("cg_blog")
    );
    assert!(VLOG.contains("ensures normal resource_variadic_borrowed_tail();\n"));
    let both = VLOG.replace(
        "ensures normal resource_variadic_borrowed_tail();\n",
        "ensures normal resource_variadic_borrowed_tail();\nensures normal resource_variadic_data_tail();\n",
    );
    let error = load_resource_contract_sources(&BTreeMap::from([("both.qcore".to_owned(), both)]))
        .expect_err("the data and borrowed tails never stand together");
    assert!(error.contains("duplicate variadic data tail"), "{error}");
}
