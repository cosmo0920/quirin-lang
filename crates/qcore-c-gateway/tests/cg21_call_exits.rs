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

//! Card CG21-CALL-EXITS-AND-BOUNDARIES (gateway rule 11o): route-G controls.
//!
//! The harness is CG00's (`support/clang_gap.rs`) with two more contract
//! sources: the production `quirin-provider-libc` `<stdlib.h>` module, byte
//! for byte (`abort`, `exit`, `__assert_fail` with `resource_no_return()`),
//! and the synthetic `cg21_call_exits.qv` (`cg_die`, `cg_die_with`,
//! `cg_die_plain`). A `Safe` here is `Safe` over those trusted assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::{load_resource_contract_sources, no_return_callees};

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::{DECLARATIONS, clang_available};

const STDLIB: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdlib.qv");
const FIXTURE: &str = include_str!("fixtures/contracts/cg21_call_exits.qv");

const PRELUDE: &str = "\
__attribute__((noreturn)) void cg_die(void);
__attribute__((noreturn)) void cg_die_with(void *p);
void cg_die_plain(void);
__attribute__((noreturn)) void cg_fatal(void);
void cg_abort_named(void);
";

fn sources(with_exits: bool) -> BTreeMap<String, String> {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    let mut sources = BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qcore",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_nullable.qcore",
            include_str!("fixtures/contracts/q_nullable.qv"),
        ),
        file(
            "clang_gap.qcore",
            include_str!("fixtures/contracts/clang_gap.qv"),
        ),
    ]);
    if with_exits {
        sources.insert("libc/stdlib.qv".to_owned(), STDLIB.to_owned());
        sources.insert("cg21_call_exits.qcore".to_owned(), FIXTURE.to_owned());
    }
    sources
}

fn contracts(with_exits: bool) -> ResourceContracts {
    load_resource_contract_sources(&sources(with_exits)).expect("the CG21 contracts load")
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    unsupported: Vec<String>,
    diagnostics: Vec<String>,
}

impl Run {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused(&self, description: &str) -> bool {
        self.unsupported.iter().any(|d| d == description)
    }
    fn refused_prefix(&self, prefix: &str) -> bool {
        self.unsupported.iter().any(|d| d.starts_with(prefix))
    }
}

fn run_with(source: &str, with_exits: bool) -> Run {
    let text = format!("{DECLARATIONS}\n{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(with_exits),
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
        .unwrap_or_else(|| panic!("no summary for `subject`: {outcome:?}"));
    Run {
        status: result.status,
        unsupported: result
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
        diagnostics: result
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d.kind))
            .collect(),
    }
}

fn run(source: &str) -> Run {
    run_with(source, true)
}

const ASSERT_FAIL: &str = "call to uncontracted function `__assert_fail`";

#[test]
fn the_production_declarations_are_no_return() {
    let callees = no_return_callees(&BTreeMap::from([(
        "libc/stdlib.qv".to_owned(),
        STDLIB.to_owned(),
    )]))
    .expect("stdlib.qv loads");
    assert_eq!(
        callees.into_iter().collect::<Vec<_>>(),
        ["__assert_fail", "abort", "exit"]
    );
}

/// `assert(p); use(p)`: glibc's `assert` (a statement expression, rule 11g)
/// whose `__assert_fail` arm is a path end. Without the declarations the
/// same body keeps CG07's inner leaf.
#[test]
fn assert_then_use_is_admitted() {
    if !clang_available() {
        return;
    }
    // glibc 2.43's expansion of `assert (E)` (the analysed units are
    // preprocessed; `__FILE__` needs an input file, so it is spelled here).
    let assert = |e: &str| {
        format!(
            "((void) sizeof (__assert_single_arg ({e})), __extension__ ({{ if ({e}) ; else \
             __assert_fail (\"{e}\", \"f.c\", 1, __extension__ __PRETTY_FUNCTION__); }}));"
        )
    };
    let source = format!(
        "extern _Bool __assert_single_arg (_Bool); \
         extern void __assert_fail (const char *a, const char *f, unsigned int l, const char *fn) \
         __attribute__ ((__noreturn__));\n\
         void subject(void *p) {{ void *q = q_alloc(); {} q_observe(q); {} q_free(q); }}",
        assert("q"),
        assert("p")
    );
    let source = source.as_str();
    let before = run_with(source, false);
    assert!(before.refused(ASSERT_FAIL), "{before:?}");
    assert!(!before.safe());
    let after = run(source);
    assert!(after.safe(), "{after:?}");
}

/// The path end discharges nothing: a resource live at `abort`/`exit` is not
/// a leak (the program ends), and the other arm keeps its obligation.
#[test]
fn a_terminating_exit_ends_its_path() {
    if !clang_available() {
        return;
    }
    let live = run("#include <stdlib.h>\n\
                    void subject(int c) { void *p = q_alloc(); if (c) exit(1); q_free(p); }");
    assert!(live.safe(), "{live:?}");
    let leaking = run("#include <stdlib.h>\n\
             void subject(int c) { void *p = q_alloc(); if (c) abort(); if (c) return; q_free(p); }");
    assert!(
        leaking.status == AnalysisStatus::Unsafe
            && leaking.diagnostics.iter().any(|d| d.contains("Leak")),
        "{leaking:?}"
    );
    // Every path ends: no exit state, nothing to leak.
    assert!(
        run("#include <stdlib.h>\nvoid subject(void) { void *p = q_alloc(); abort(); }").safe()
    );
}

/// Nothing after a path end is checked: the second release is unreachable.
#[test]
fn obligations_after_a_path_end_are_unreachable() {
    if !clang_available() {
        return;
    }
    let after = run("void subject(void) { void *p = q_alloc(); q_free(p); cg_die(); q_free(p); }");
    assert!(after.safe(), "{after:?}");
    // The same body with an ordinary call keeps the double release.
    let control =
        run("void subject(void) { void *p = q_alloc(); q_free(p); cg_sink(0); q_free(p); }");
    assert!(
        control
            .diagnostics
            .iter()
            .any(|d| d.contains("DoubleRelease")),
        "{control:?}"
    );
}

/// The callee's own preconditions are checked at the path end.
#[test]
fn a_no_return_call_still_checks_its_actuals() {
    if !clang_available() {
        return;
    }
    let released = run("void subject(void) { void *p = q_alloc(); q_free(p); cg_die_with(p); }");
    assert!(!released.safe(), "{released:?}");
    let live = run("void subject(void) { void *p = q_alloc(); cg_die_with(p); }");
    assert!(live.safe(), "{live:?}");
}

/// Never from a name alone: a no-return declaration the Clang declaration
/// does not establish is refused, and so is a `noreturn` function no
/// declaration states (it keeps the uncontracted refusal).
#[test]
fn a_no_return_needs_both_the_declaration_and_the_attribute() {
    if !clang_available() {
        return;
    }
    let plain = run("void subject(void) { void *p = q_alloc(); cg_die_plain(); q_free(p); }");
    assert!(
        plain.refused(
            "no-return contract of `cg_die_plain` not established at this call: its declaration \
             here is not `noreturn`"
        ),
        "{plain:?}"
    );
    assert!(!plain.safe());
    let attribute_only = run("void subject(void) { void *p = q_alloc(); cg_fatal(); q_free(p); }");
    assert!(
        attribute_only.refused("call to uncontracted function `cg_fatal`"),
        "{attribute_only:?}"
    );
    let named = run("void subject(void) { void *p = q_alloc(); cg_abort_named(); q_free(p); }");
    assert!(
        named.refused("call to uncontracted function `cg_abort_named`"),
        "{named:?}"
    );
    // A local object shadowing the callee's name: unresolved.
    let shadowed = run(
        "void subject(void) { void (*cg_die)(void) = 0; void *p = q_alloc(); cg_die(); q_free(p); }",
    );
    assert!(!shadowed.safe(), "{shadowed:?}");
}

/// A non-local exit is never a path end: `longjmp` with a live resource is
/// refused by name, and no declaration of it loads.
#[test]
fn longjmp_across_a_live_resource_is_refused() {
    if !clang_available() {
        return;
    }
    let jump = run("#include <setjmp.h>\n\
                    void subject(jmp_buf b) { void *p = q_alloc(); longjmp(b, 1); }");
    assert!(
        jump.refused("`longjmp` is a non-local exit, which no resource contract or route models"),
        "{jump:?}"
    );
    assert!(!jump.safe());
    let returns_twice = run("#include <setjmp.h>\n\
                             void subject(jmp_buf b) { void *p = q_alloc(); int r = setjmp(b); \
                             q_free(p); }");
    // glibc's header redirects setjmp to _setjmp; Darwin keeps setjmp.
    // Both must retain the complete named refusal and the Unknown verdict.
    assert_eq!(returns_twice.status, AnalysisStatus::Unknown);
    assert!(
        ["setjmp", "_setjmp"]
            .iter()
            .any(|name| returns_twice.refused(&format!(
                "`{name}` is a returns-twice call, which no resource contract or route models"
            ))),
        "{returns_twice:?}"
    );
    for name in [
        "longjmp",
        "_setjmp",
        "pthread_exit",
        "__builtin_choose_expr",
    ] {
        let module = format!(
            "module contract;\n\nextern fn {name}() -> unit\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_return_none();\n}};\n"
        );
        let error =
            load_resource_contract_sources(&BTreeMap::from([("probe.qcore".to_owned(), module)]))
                .expect_err(name);
        assert!(format!("{error:?}").contains("which no resource v1 declaration states"));
    }
}

/// `cleanup` is a named boundary asked first: the attribute's scope-exit
/// call is not modelled, so neither a body it would make Safe nor one it
/// would leave leaking is admitted.
#[test]
fn cleanup_attributes_are_a_named_boundary() {
    if !clang_available() {
        return;
    }
    const CLEANUP: &str = "a `cleanup` attribute's call at every exit of its scope is not modelled";
    let releasing = run("static void q_free_ref(void **p) { q_free(*p); }\n\
         void subject(int c) { __attribute__((cleanup(q_free_ref))) void *p = q_alloc(); \
         if (c) return; q_observe(p); }");
    assert!(releasing.refused(CLEANUP), "{releasing:?}");
    assert!(!releasing.safe());
    let scalar =
        run("void g(int *); void subject(void) { __attribute__((cleanup(g))) int x = 0; }");
    assert!(scalar.refused(CLEANUP), "{scalar:?}");
}

/// Assembly and SEH are named boundaries; an extension node is never an
/// uncontracted call.
#[test]
fn assembly_and_extension_nodes_are_named() {
    if !clang_available() {
        return;
    }
    let assembly = run(
        "void subject(void) { void *p = q_alloc(); __asm__ volatile(\"\" ::: \"memory\"); q_free(p); }",
    );
    assert!(
        assembly.refused(
            "inline assembly is an opaque boundary: its clobbers and memory effects are not \
             modelled"
        ),
        "{assembly:?}"
    );
    let goto = run(
        "void subject(void) { void *p = q_alloc(); __asm__ goto(\"\" :::: out); out: q_free(p); }",
    );
    assert!(
        goto.refused_prefix("`asm goto` is an opaque boundary"),
        "{goto:?}"
    );
    let choose = run(
        "void subject(void) { void *p = q_alloc(); int x = __builtin_choose_expr(1, 1, 0); q_free(p); }",
    );
    assert!(
        choose.refused_prefix("`__builtin_choose_expr` is the Clang extension node ChooseExpr"),
        "{choose:?}"
    );
}

/// Outside a statement position the declared call is not refused by rule
/// 11o's statement hook. Card GAPO-RETURN-CORRECTNESS (rule 15y) amended the
/// rest of this control's original reading ("its continuation is kept"):
/// wherever a route lowers the call, its node ends its path where the
/// declaration is established and is refused otherwise; here `exit` is
/// `noreturn`, so no refusal names it.
#[test]
fn an_operand_position_keeps_its_continuation() {
    if !clang_available() {
        return;
    }
    let arm = run("#include <stdlib.h>\n\
         void subject(int a) { void *p = q_alloc(); int x = a ? (exit(1), 0) : 1; q_free(p); }");
    assert!(!arm.refused_prefix("no-return contract"), "{arm:?}");
}
