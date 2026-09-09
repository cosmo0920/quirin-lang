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
            include_str!("fixtures/contracts/q_base.qcore").to_owned(),
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
#[test]
fn a_dereference_in_a_return_expression_stays_unknown() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long dereferenced(unsigned long *slot) {{
            return digits10(*slot);
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("dereferenced"), AnalysisStatus::Unknown);
}

/// Pointer arithmetic inside an otherwise admitted call actual. Measured
/// boundary, recorded rather than assumed: a *bare* `(unsigned long)(p + n)`
/// return carries no token the existing lexical predicate refuses, so it was
/// admitted before this card and still is. What this card decides is the
/// actual, where the C02 operand walker refuses `p` because it is not a
/// declared scalar.
#[test]
fn pointer_arithmetic_in_a_call_actual_stays_unknown() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long offset(char *p, unsigned long n) {{
            return digits10((unsigned long) (p + n));
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("offset"), AnalysisStatus::Unknown);
}

/// A subscript in the same position.
#[test]
fn a_subscript_in_a_return_expression_stays_unknown() {
    if !clang_available() {
        return;
    }
    let program = format!(
        "{DIGITS10}
        unsigned long indexed(unsigned long *table, unsigned long n) {{
            return digits10(table[n]);
        }}"
    );
    let run = run(&program);
    assert_eq!(run.status("indexed"), AnalysisStatus::Unknown);
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

/// `sizeof` is call-shaped to the lexical scan and is never a declared callee.
#[test]
fn a_sizeof_in_a_return_expression_keeps_its_existing_answer() {
    if !clang_available() {
        return;
    }
    let run = run("unsigned long sized(void) {
            return sizeof(unsigned long);
        }");
    // The existing lexical predicate already refuses it; this pins that the new
    // rule does not admit it either.
    assert_eq!(run.status("sized"), AnalysisStatus::Unknown);
}
