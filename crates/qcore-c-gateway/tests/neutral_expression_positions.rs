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

//! Card W4-EXPR-POS: the typed neutral-expression-position admission, end to
//! end through the real gateway.
//!
//! The design note `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` names the
//! permissive direction as the card's central obligation: threading the
//! contract environment into the six expression positions re-classifies branch
//! conditions and return expressions across all 185 pinned units, and it does
//! so in the permissive direction. These tests are the held-out half of the
//! evidence for that: §6's minimal validation case measured end to end, and a
//! family of expressions that *look* neutral but carry a call, an address, a
//! dereference, pointer arithmetic or a subscript, each of which must keep its
//! existing `Unknown` and its existing diagnostic.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ContractTrust, LegacyDiagnosticKind,
    LegacySafetyCertificate, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, ResourceSummaryRecord, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *); void q_transfer(void *);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[(
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        )]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

struct Run {
    functions: std::collections::BTreeMap<String, AnalysisStatus>,
    diagnostics: std::collections::BTreeMap<String, Vec<LegacyDiagnosticKind>>,
    records: std::collections::BTreeMap<String, ResourceSummaryRecord>,
    certificate: Option<LegacySafetyCertificate>,
}

impl Run {
    fn status(&self, function: &str) -> AnalysisStatus {
        *self
            .functions
            .get(function)
            .unwrap_or_else(|| panic!("no projection for `{function}`"))
    }

    fn unsupported(&self, function: &str) -> Vec<String> {
        self.diagnostics
            .get(function)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect()
    }

    fn leaks(&self, function: &str) -> usize {
        self.diagnostics
            .get(function)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }
}

fn run(program: &str) -> Run {
    // Every fixture needs the named entry the gateway analyzes from; a fixture
    // that does not care about the entry gets an empty one.
    let entry = if program.contains("subject") {
        String::new()
    } else {
        "\nvoid subject(void) { }\n".to_owned()
    };
    let source = format!("{DECLARATIONS}\n{program}{entry}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let (summary, records, certificate) = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate)
                .expect("a Safe certificate must recheck independently");
            let records = certificate
                .summaries
                .iter()
                .map(|record| (record.function.clone(), record.clone()))
                .collect();
            (certificate.summary.clone(), records, Some(certificate))
        }
        UnrestrictedLegacyResult::Unsafe(summary) => {
            (summary, std::collections::BTreeMap::new(), None)
        }
        UnrestrictedLegacyResult::Unknown { partial, .. } => (
            partial.expect("these fixtures always project partially"),
            std::collections::BTreeMap::new(),
            None,
        ),
    };
    Run {
        functions: summary
            .functions
            .iter()
            .map(|(name, function)| (name.clone(), function.status))
            .collect(),
        diagnostics: summary
            .functions
            .iter()
            .map(|(name, function)| {
                (
                    name.clone(),
                    function
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.kind.clone())
                        .collect(),
                )
            })
            .collect(),
        records,
        certificate,
    }
}

fn neutral(arity: usize) -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed; arity],
        ResourceReturnMode::None,
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
            eprintln!("skipping Clang-dependent neutral-expression-position test: {reason}");
            false
        }
    }
}

/// Fluent Bit `66910c10`'s `lib/monkey/mk_core/mk_string.c` `digits10`, verbatim
/// but for the `uint32_t`/`uint64_t` spellings this fixture does not include a
/// header for. It is the design note's §6 minimal validation case: one
/// self-recursive component whose *only* blocking instruction is the last
/// return expression.
const DIGITS10: &str = r"
unsigned int digits10(unsigned long v) {
    if (v < 10) return 1;
    if (v < 100) return 2;
    if (v < 1000) return 3;
    if (v < 1000000000000UL) {
        if (v < 100000000UL) {
            if (v < 1000000) {
                if (v < 10000) return 4;
                return 5 + (v >= 100000);
            }
            return 7 + (v >= 10000000UL);
        }
        if (v < 10000000000UL) {
            return 9 + (v >= 1000000000UL);
        }
        return 11 + (v >= 100000000000UL);
    }
    return 12 + digits10(v / 1000000000000UL);
}
";

#[test]
fn digits10_validates_end_to_end_through_the_neutral_route() {
    if !clang_available() {
        return;
    }
    let run = run(DIGITS10);
    assert_eq!(
        run.status("digits10"),
        AnalysisStatus::Safe,
        "digits10's only blocking instruction is its recursive return; \
         unsupported: {:?}",
        run.unsupported("digits10")
    );
    // V7: the contract validation reads back off the body is exactly `N(1)`,
    // published under the inference trust label, never as a declaration.
    let record = run
        .records
        .get("digits10")
        .expect("an admitted component publishes its neutral summary");
    assert_eq!(record.trust, ContractTrust::InferredValidated);
    assert_eq!(record.contract, neutral(1));
    // The certificate rechecked independently inside `run`; assert it exists so
    // a future change cannot quietly stop producing one.
    assert!(run.certificate.is_some());
}

#[test]
fn a_caller_of_the_admitted_recursion_is_modelled() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        void subject(void) {{
            void *p = q_alloc();
            unsigned int n = digits10(41);
            (void) n;
            q_free(p);
        }}"
    );
    let run = run(&program);
    assert_eq!(
        run.status("subject"),
        AnalysisStatus::Safe,
        "unsupported: {:?}",
        run.unsupported("subject")
    );
}

/// The precision direction that matters most: admitting the position must not
/// erase a leak the caller really has.
#[test]
fn a_leak_beside_an_admitted_return_is_still_reported() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned int leaky(unsigned long v) {{
            void *p = q_alloc();
            (void) p;
            return 1 + digits10(v);
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("leaky"), AnalysisStatus::Unsafe);
    assert_eq!(run.leaks("leaky"), 1);
}

/// A `q_alloc` in the return expression looks like the admitted shape and is
/// not: its contract is not neutral, so the position keeps its refusal.
#[test]
fn an_acquiring_call_in_a_return_expression_stays_unknown() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long acquiring(unsigned long v) {
            if (v == 0) return 0;
            return 1 + (unsigned long) q_alloc();
        }");
    assert_eq!(run.status("acquiring"), AnalysisStatus::Unknown);
    assert!(
        run.unsupported("acquiring").iter().any(|description| {
            description == "unmodeled call, aggregate or indirect access in return expression"
        }),
        "unsupported: {:?}",
        run.unsupported("acquiring")
    );
}

/// A releasing call, at the same arity, in the same position.
#[test]
fn a_releasing_call_in_a_return_expression_stays_unknown() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long releasing(void *p) {
            return 1 + (unsigned long) (q_free(p), 0);
        }");
    assert_eq!(run.status("releasing"), AnalysisStatus::Unknown);
}

/// An uncontracted callee in the return position: the environment has nothing
/// for it, so the whole expression refuses.
#[test]
fn an_uncontracted_call_in_a_return_expression_stays_unknown() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long external(unsigned long);
        unsigned long wrapper(unsigned long v) {
            if (v == 0) return 0;
            return 1 + external(v - 1);
        }");
    assert_eq!(run.status("wrapper"), AnalysisStatus::Unknown);
    assert!(
        run.unsupported("wrapper").iter().any(|description| {
            description == "unmodeled call, aggregate or indirect access in return expression"
        }),
        "unsupported: {:?}",
        run.unsupported("wrapper")
    );
}

/// One admitted neutral call beside one unadmitted one: all-or-nothing, so the
/// whole position refuses and the unmodelled call cannot hide.
#[test]
fn one_unadmitted_call_beside_an_admitted_one_stays_unknown() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long external(unsigned long);
        unsigned long mixed(unsigned long v) {{
            if (v == 0) return 0;
            return digits10(v) + external(v);
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("mixed"), AnalysisStatus::Unknown);
}

/// An address-of actual: the operand is not a declared scalar, so the argument
/// refuses and with it the call and the position.
#[test]
fn an_address_of_actual_stays_unknown() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long counted(unsigned long *slot);
        unsigned long addressed(unsigned long v) {{
            unsigned long local = v;
            if (v == 0) return 0;
            return digits10(local) + (unsigned long) &local;
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("addressed"), AnalysisStatus::Unknown);
}

/// A dereference in the same position.
///
/// **Deliberate pin update, card `CG10-INDIRECT-ACCESS-SOLE-BLOCKERS` (gateway
/// rule 12i, `docs/reports/cg10-indirect-access-sole-blockers/README.md`).**
/// This row asserted `Unknown`: the gate refused `*slot` and the return
/// position never asked rule W44, which reads exactly that actual at the
/// statement position. Rule 12i asks it, with the same statement the return
/// position classifies, so `return digits10(*slot);` now agrees with its
/// statement form, whose verdict neither of the card's hooks can move (no
/// `return` call, no `.`-rooted place): the second leg is that control. The
/// read keeps W44's obligation on `slot`; no bounds claim is made.
#[test]
fn a_dereference_in_a_call_actual_agrees_with_the_statement_position() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long dereferenced(unsigned long *slot) {{
            return digits10(*slot);
        }}"
    );
    assert_eq!(run(&program).status("dereferenced"), AnalysisStatus::Safe);
    let statement = format!(
        "{DIGITS10}
        unsigned long dereferenced(unsigned long *slot) {{
            unsigned int t = digits10(*slot);
            return t;
        }}"
    );
    assert_eq!(run(&statement).status("dereferenced"), AnalysisStatus::Safe);
}

/// Pointer arithmetic inside an otherwise admitted call actual.
///
/// **Deliberate pin update, card `C-GATEWAY-RETURN-CALL-W4`
/// (`docs/reports/c-gateway-return-call-w4/README.md` §6).** This row asserted
/// `Unknown`. It now asserts `Safe`, and the ground is measured rather than
/// argued: the **statement** form of this identical C —
/// `unsigned int t = digits10((unsigned long)(p + n)); return t;` — was
/// **already `Safe` before that card**, so the `Unknown` this row pinned was the
/// *return position's* extra strictness and not a custody judgment. The
/// return-call admission is a desugaring of `return f(a);` into exactly that
/// statement form, so the two positions now agree, and the second leg below
/// publishes the control that makes the claim checkable instead of asserted.
///
/// What card W4-EXPR-POS measured here is **unchanged and still true**: the C02
/// operand walker does refuse `p`, and the *neutral-expression* route still
/// refuses this actual. That route is simply no longer the only way a `return`
/// can be modeled. The neighbouring dereference and subscript rows are the
/// evidence that nothing broader moved: both kept `Unknown`, because
/// `classify_simple`'s own access gate refuses `*` and `[` where it admits `+`
/// (until rule 12i read them as the statement position does, below).
#[test]
fn pointer_arithmetic_in_a_call_actual_agrees_with_the_statement_position() {
    if !clang_available() {
        return;
    }
    let returned = format!(
        "{DIGITS10}
        unsigned long offset(char *p, unsigned long n) {{
            return digits10((unsigned long) (p + n));
        }}"
    );
    assert_eq!(run(&returned).status("offset"), AnalysisStatus::Safe);

    // The control the pin update rests on: the same C, written out. Its verdict
    // predates this card, so if it ever moved, the alignment above would be
    // resting on nothing and this assertion would say so.
    let statement = format!(
        "{DIGITS10}
        unsigned long offset(char *p, unsigned long n) {{
            unsigned int t = digits10((unsigned long) (p + n));
            return t;
        }}"
    );
    assert_eq!(run(&statement).status("offset"), AnalysisStatus::Safe);
}

/// A subscript in the same position.
///
/// **Deliberate pin update, card `CG10-INDIRECT-ACCESS-SOLE-BLOCKERS` (rule
/// 12i)**, on the dereference row's ground: the statement form below is the
/// control, and no bounds claim is made for `table[n]`.
#[test]
fn a_subscript_in_a_call_actual_agrees_with_the_statement_position() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long indexed(unsigned long *table, unsigned long n) {{
            return digits10(table[n]);
        }}"
    );
    assert_eq!(run(&program).status("indexed"), AnalysisStatus::Safe);
    let statement = format!(
        "{DIGITS10}
        unsigned long indexed(unsigned long *table, unsigned long n) {{
            unsigned int t = digits10(table[n]);
            return t;
        }}"
    );
    assert_eq!(run(&statement).status("indexed"), AnalysisStatus::Safe);
}

/// A pointer actual: the C00 hazard the conjunction exists to refuse. `p` is
/// declared a pointer, so it is never an admitted operand, and the call this
/// rule would otherwise treat as neutral keeps its refusal.
#[test]
fn a_pointer_actual_stays_unknown() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long counted(void *p) {
            if (p == 0) return 0;
            return 1 + counted(p);
        }");
    assert_eq!(run.status("counted"), AnalysisStatus::Unknown);
}

/// B02/B03's condition-call widening must not read a provisional neutral
/// summary for the local recursive callee back as its own admission premise.
/// The existing neutral-expression rule still decides its previously admitted
/// scalar-actual cases; these pointer-actual conditions keep the old refusal.
#[test]
fn recursive_pointer_calls_in_control_expressions_stay_unknown() {
    if !clang_available() {
        return;
    }
    for (name, control) in [
        ("branch_counted", "if (branch_counted(p)) return 1;"),
        ("while_counted", "while (while_counted(p)) { }"),
        ("for_counted", "for (; for_counted(p); ) { }"),
        ("do_counted", "do { } while (do_counted(p));"),
        (
            "switch_counted",
            "switch (switch_counted(p)) { default: break; }",
        ),
    ] {
        let program = format!(
            "unsigned long {name}(void *p) {{
                if (p == 0) return 0;
                {control}
                return 0;
            }}"
        );
        let run = run(&program);
        assert_eq!(
            run.status(name),
            AnalysisStatus::Unknown,
            "{name} unexpectedly admitted: {:?}",
            run.unsupported(name)
        );
        assert!(
            !run.records.contains_key(name),
            "{name} must not publish a circular neutral summary"
        );
    }
}

/// Admitting the *position* must not admit an effectful *body*. This
/// component's return expression is exactly the admitted shape, and its body
/// still leaks, so the descent demotes it and it publishes nothing.
#[test]
fn an_effectful_body_publishes_nothing_even_with_an_admitted_return() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long leaking(unsigned long v) {
            void *p = q_alloc();
            (void) p;
            if (v == 0) return 0;
            return 1 + leaking(v - 1);
        }");
    assert!(
        !run.records.contains_key("leaking"),
        "an effectful body must publish no neutral summary"
    );
    // One per reachable exit: the base return and the recursive one, which is
    // the second only because the position is now admitted.
    assert_eq!(run.leaks("leaking"), 2);
}

/// A call through a member selector shares the callee's spelling with an
/// admitted one and must not borrow its contract.
#[test]
fn a_member_call_in_a_return_expression_stays_unknown() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        struct table {{ unsigned int (*digits10)(unsigned long); }};
        unsigned int selected(struct table *t, unsigned long v) {{
            return t->digits10(v);
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("selected"), AnalysisStatus::Unknown);
}

/// Wave 23 supplies the fixed scalar query premise this earlier pin lacked.
#[test]
fn a_fixed_scalar_sizeof_in_a_return_expression_is_now_admitted() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long sized(void) {
            return sizeof(unsigned long);
        }");
    assert_eq!(run.status("sized"), AnalysisStatus::Safe);
}

// ---------------------------------------------------------------------------
// Stage 2: the other five positions the design note names.
// ---------------------------------------------------------------------------

/// A branch condition carrying an admitted neutral call. The null-test and
/// pointer-inequality refinements are checked before this, and neither applies
/// to a comparison against a literal, so the position is decided by this card.
#[test]
fn a_branch_condition_admits_a_neutral_call() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        void subject(void) {{
            void *p = q_alloc();
            if (digits10(41) > 3) {{ q_free(p); }} else {{ q_free(p); }}
        }}"
    );
    let run = run(&program);
    assert_eq!(
        run.status("subject"),
        AnalysisStatus::Safe,
        "unsupported: {:?}",
        run.unsupported("subject")
    );
}

/// A `while` condition, a `for` condition and a `do`/`while` condition, all
/// three carrying the same admitted call.
#[test]
fn the_three_loop_conditions_admit_a_neutral_call() {
    if !clang_available() {
        return;
    }
    for header in [
        "while (digits10(n) < 3) { }",
        "for (; digits10(n) < 3; ) { }",
        "do { } while (digits10(n) < 3);",
    ] {
        let program = format!(
            "{DIGITS10}
            void subject(unsigned long n) {{
                void *p = q_alloc();
                {header}
                q_free(p);
            }}"
        );
        let run = run(&program);
        assert_eq!(
            run.status("subject"),
            AnalysisStatus::Safe,
            "header `{header}` unsupported: {:?}",
            run.unsupported("subject")
        );
    }
}

/// A `switch` expression carrying the same admitted call.
#[test]
fn a_switch_expression_admits_a_neutral_call() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        void subject(unsigned long n) {{
            void *p = q_alloc();
            switch (digits10(n)) {{
                case 1: break;
                default: break;
            }}
            q_free(p);
        }}"
    );
    let run = run(&program);
    assert_eq!(
        run.status("subject"),
        AnalysisStatus::Safe,
        "unsupported: {:?}",
        run.unsupported("subject")
    );
}

/// A branch condition reading a direct scalar member of an unbound base: the
/// C02 whitelist, in the position card C02 deliberately left out of scope.
#[test]
fn a_branch_condition_admits_a_direct_scalar_field_read() {
    if !clang_available() {
        return;
    }
    let run = run("struct counters { int hits; void *slot; };
        void subject(struct counters *c) {
            void *p = q_alloc();
            if (c->hits > 2) { q_free(p); } else { q_free(p); }
        }");
    assert_eq!(
        run.status("subject"),
        AnalysisStatus::Safe,
        "unsupported: {:?}",
        run.unsupported("subject")
    );
}

/// The same read through a base this body binds: refused, which is card C02's
/// premise 8 and the reason the C00 recommendation was not implemented as
/// written.
#[test]
fn a_branch_condition_refuses_a_read_through_a_bound_base() {
    if !clang_available() {
        return;
    }
    let run = run("struct counters { int hits; void *slot; };
        void subject(struct counters *c) {
            void *p = q_alloc();
            q_transfer(c);
            if (c->hits > 2) { q_free(p); } else { q_free(p); }
        }");
    assert_eq!(run.status("subject"), AnalysisStatus::Unknown);
    // Deliberate pin update, card W43-INDEX-TRAVERSAL: the condition is now
    // the typed read of `c->hits`, and its dereference obligation refuses
    // the base `q_transfer` consumed, as an unresolved alias of `c`, rather
    // than the lexical branch refusal. Premise 8 still holds: the read is
    // never neutralized through a base this body may bind.
    assert!(
        run.diagnostics["subject"].iter().any(|kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "c"
        )),
        "diagnostics: {:?}",
        run.diagnostics
    );
}

/// **Deliberate pin update, card B02-BRANCH-CONDITION.**  A releasing call in
/// a branch condition is now an ordinary `ContractCall` node before the
/// two-successor branch, so its effect does not disappear into a `NoOp`.
/// This exact comma expression evaluates the call unconditionally; the
/// short-circuit and conditional neighbours in `branch_condition_calls.rs`
/// retain the old refusal.
#[test]
fn an_unconditional_releasing_call_in_a_branch_condition_is_modeled() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) {
            void *p = q_alloc();
            if ((q_free(p), 1)) { }
        }");
    assert_eq!(run.status("subject"), AnalysisStatus::Safe);
    assert!(run.unsupported("subject").is_empty());
    assert_eq!(run.leaks("subject"), 0);
}

/// An uncontracted call in each of the five stage-2 positions keeps its own
/// existing diagnostic, so no position silently borrows another's answer.
#[test]
fn an_uncontracted_call_keeps_each_positions_own_diagnostic() {
    if !clang_available() {
        return;
    }
    for (header, description) in [
        (
            "if (external(n)) { }",
            "unmodeled call, aggregate or indirect access in branch condition",
        ),
        (
            "while (external(n)) { n = n + 1; }",
            "unmodeled call, aggregate or indirect access in loop condition",
        ),
        (
            "for (; external(n); ) { n = n + 1; }",
            "unmodeled call, aggregate or indirect access in for-loop condition",
        ),
        (
            "do { n = n + 1; } while (external(n));",
            "unmodeled call, aggregate or indirect access in do-while condition",
        ),
        (
            "switch (external(n)) { default: break; }",
            "unmodeled call, aggregate or indirect access in switch expression",
        ),
    ] {
        let program = format!(
            "unsigned long external(unsigned long);
            void subject(unsigned long n) {{
                {header}
            }}"
        );
        let run = run(&program);
        assert_eq!(
            run.status("subject"),
            AnalysisStatus::Unknown,
            "{header}: {:?}",
            run.unsupported("subject")
        );
        assert!(
            run.unsupported("subject")
                .iter()
                .any(|found| found == description),
            "`{header}` expected `{description}`, got {:?}",
            run.unsupported("subject")
        );
    }
}

/// A `case` label keeps the strict lexical predicate (`W4-EXPR-POS-CASE`): a
/// constant expression can carry neither a call nor a member access, so the
/// typed admission would be inert. Pinned so the decision is visible.
#[test]
fn a_case_label_keeps_the_lexical_predicate() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(int k) {
            switch (k) {
                case 1: break;
                default: break;
            }
        }");
    assert_eq!(run.status("subject"), AnalysisStatus::Safe);
}

// ---------------------------------------------------------------------------
// The `for`-clause fail-open this card measured and repaired.
// ---------------------------------------------------------------------------

/// `split_for_clauses` consumes `(`, `)`, `[` and `]` while tracking depth, so
/// before this card `for (; external(n); )` reached the classification as the
/// two tokens `external n` — no call for `contains_call` to find and no
/// subscript for `contains_unmodeled_access` to find — and the position was
/// classified `NoOp`. Both spellings now refuse, from the original tokens.
#[test]
fn a_for_condition_is_classified_from_its_original_spelling() {
    if !clang_available() {
        return;
    }
    // Deliberate pin update, card W43-INDEX-TRAVERSAL: `table[n]` over a
    // declared `unsigned long *` formal is now the typed condition read, so
    // the subscript spelling certifies (see `subscript_condition_certifies`
    // below); the call spelling keeps the refusal this card measured.
    let header = "for (; external(n); ) { n = n + 1; }";
    let program = format!(
        "unsigned long external(unsigned long);
        void subject(unsigned long n, unsigned long *table) {{
            {header}
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("subject"), AnalysisStatus::Unknown, "{header}");
    assert!(
        run.unsupported("subject").iter().any(|description| {
            description == "unmodeled call, aggregate or indirect access in for-loop condition"
        }),
        "`{header}` unsupported: {:?}",
        run.unsupported("subject")
    );
}

/// Card W43-INDEX-TRAVERSAL: the subscript spelling of the header above is
/// classified from its original tokens too, and is now the typed condition
/// read through an unbound formal. Bounds are not decided.
#[test]
fn subscript_condition_certifies() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(unsigned long n, unsigned long *table) {
            for (; table[n]; ) { n = n + 1; }
        }");
    assert_eq!(
        run.status("subject"),
        AnalysisStatus::Safe,
        "{:?}",
        run.diagnostics
    );
}

/// The repair is confined to the condition: `init` and `increment` keep the
/// stripped spelling `split_for_clauses` gives them, which is recorded as the
/// named follow-on `W4-EXPR-POS-FOR-CLAUSES` rather than repaired here. Pinned
/// so the residual is visible and moves deliberately.
#[test]
fn a_for_init_and_increment_keep_the_stripped_spelling() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long external(unsigned long);
        void subject(unsigned long n) {
            for (n = external(0); n < 3; n = external(n)) { }
        }");
    // Neither clause raises the call diagnostic a statement-position call
    // would: the parentheses never reach `classify_simple`.
    assert!(
        !run.unsupported("subject")
            .iter()
            .any(|description| description.starts_with("call to uncontracted function")),
        "unsupported: {:?}",
        run.unsupported("subject")
    );
    // Measured: the residual is fail-closed, but for an unrelated reason —
    // `n = external` with the parentheses gone is an unsupported assignment
    // expression, not a call. That is the shape `W4-EXPR-POS-FOR-CLAUSES`
    // records: the clause is refused by accident of the stripped spelling
    // rather than because a call was seen.
    assert_eq!(run.status("subject"), AnalysisStatus::Unknown);
    assert_eq!(
        run.unsupported("subject"),
        ["unsupported assignment expression"]
    );
}

/// The condition repair also lets the new typed rule reach the `for` position
/// at all: with the stripped spelling the admitted call was invisible in both
/// directions.
#[test]
fn a_for_condition_admits_a_neutral_call_from_its_original_spelling() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        void subject(unsigned long n) {{
            void *p = q_alloc();
            for (; digits10(n) < 3; ) {{ n = n + 1; }}
            q_free(p);
        }}"
    );
    let run = run(&program);
    assert_eq!(
        run.status("subject"),
        AnalysisStatus::Safe,
        "unsupported: {:?}",
        run.unsupported("subject")
    );
}

// Wave 23: fixed scalar type queries, through the production importer and
// certificate replay. Sizes/alignments are never interpreted as numeric facts.
#[test]
fn fixed_scalar_queries_cover_all_six_expression_positions() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long queries(unsigned int n) {
        if (sizeof(unsigned int) == 4) { n = 0; }
        while (sizeof(unsigned int) < 1) { break; }
        for (; _Alignof(unsigned long) < 1;) { break; }
        do { n = 1; } while (sizeof n < 1);
        switch (sizeof(n)) { case 4: break; default: break; }
        return sizeof(unsigned long) + _Alignof(unsigned int);
    }");
    assert_eq!(run.status("queries"), AnalysisStatus::Safe);
}

#[test]
fn scalar_query_assignment_forgets_old_zero_without_inventing_a_size() {
    if !clang_available() {
        return;
    }
    for rhs in ["sizeof(unsigned long)", "_Alignof(unsigned long)"] {
        let run = run(&format!(
            "void subject(void) {{
            unsigned long n; n = 0; n = {rhs};
            if (n) {{ void *p; p = q_alloc(); }}
        }}"
        ));
        assert_ne!(run.status("subject"), AnalysisStatus::Safe, "{rhs}");
        assert!(run.leaks("subject") > 0, "{rhs}");
    }
}

#[test]
fn scalar_query_initializers_and_typedefs_are_replayed() {
    if !clang_available() {
        return;
    }
    let run = run("typedef unsigned long count_t;
        void subject(void) {
            count_t n = sizeof(count_t) + _Alignof(unsigned int);
            n = sizeof n;
        }");
    assert_eq!(run.status("subject"), AnalysisStatus::Safe);
    assert!(run.certificate.is_some());
}

#[test]
fn type_queries_do_not_hide_evaluated_calls_or_pointer_results() {
    if !clang_available() {
        return;
    }
    for body in [
        "return sizeof(unsigned int) + (q_alloc() != 0);",
        "return sizeof(int[(q_free(p), 3)]);",
    ] {
        let run = run(&format!("unsigned long queries(int *p) {{ {body} }}"));
        assert_eq!(run.status("queries"), AnalysisStatus::Unknown, "{body}");
    }
    // Flipped by card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): Clang's node
    // shows these operands unevaluated (C11 6.5.3.4p2), so no call, read or
    // pointer result is hidden: `q_alloc` is never called and `*p` never read.
    for body in [
        "return sizeof(q_alloc());",
        "return sizeof(*p);",
        "return sizeof(void *);",
    ] {
        let run = run(&format!("unsigned long queries(int *p) {{ {body} }}"));
        assert_eq!(run.status("queries"), AnalysisStatus::Safe, "{body}");
    }
}

#[test]
fn a_vla_typedef_never_becomes_a_fixed_scalar_query() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long queries(unsigned int n) {
        typedef int vector[n]; return sizeof(vector);
    }");
    assert_eq!(run.status("queries"), AnalysisStatus::Unknown);
}

#[test]
fn expression_queries_do_not_erase_scalar_mutations() {
    if !clang_available() {
        return;
    }
    for condition in ["(n = sizeof(int))", "sizeof(int) + n++"] {
        let run = run(&format!(
            "void subject(void) {{
            unsigned long n; n = 0;
            if ({condition}) {{ }}
            if (n) {{ void *p; p = q_alloc(); }}
        }}"
        ));
        assert_ne!(run.status("subject"), AnalysisStatus::Safe, "{condition}");
        // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): the condition is now
        // call-free to the elaborator, which forgets `n` (`ForgetScalar`)
        // rather than keeping its old zero, so the leak on `if (n)` is
        // reported; a refusal would be as good. Never the stale zero.
        assert!(
            run.unsupported("subject")
                .iter()
                .any(|text| text.contains("branch condition"))
                || run.leaks("subject") > 0,
            "{condition}"
        );
    }
}
