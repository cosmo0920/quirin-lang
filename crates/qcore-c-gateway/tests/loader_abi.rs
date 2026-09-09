//! LOADER-ABI — the declared contract's C ABI projections are compared against
//! the analysed translation unit's own C declaration
//! (`docs/reports/remaining-loader-abi/README.md`).
//!
//! Card NEIGHBOURS recorded the gap this closes: the loader checks a
//! declaration's parameter and result spellings against its own closed table
//! and against nothing else, so an unfaithful spelling passed silently. The
//! spellings are not decoration — the loader's admissibility rules are keyed on
//! them, so an unfaithful type is exactly what buys a custody mode the real C
//! type could not carry.
//!
//! **The family is held out.** A `k_*` API unrelated to Fluent Bit, ChunkIO,
//! Monkey and every earlier card's fixtures.
//!
//! Nothing here promotes anything: every `Safe` below is `Safe` for a synthetic
//! declared custody projection under the premises the fixture names.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AbiOutcome, AbiRefusal, AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind,
    LegacyFunctionSummary, LegacyProgramSummary, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, UnrestrictedLegacyResult,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11, declared_abi_census,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

fn load(source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "k_api.qcore".to_owned(),
        format!("module contract;\n\n{source}"),
    )]))
    .expect("the fixture module loads")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .expect("the function is analysed")
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

/// Analyse one whole translation unit through the pure-token entry point.
///
/// The ABI comparison reads the same token stream the importer projects, so it
/// needs no Clang; the two Clang-gated tests below check the same decision on
/// the production entry point and at certificate issuance.
fn analyse(source: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(source, "w", contracts).expect("the fixture tokenizes")
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

/// The refusal a withdrawn callee's call carries, by key.
fn withdrawal(callee: &str, refusal: AbiRefusal) -> String {
    format!(
        "call to `{callee}` whose declared contract was withdrawn against its C declaration in \
         this translation unit ({})",
        refusal.as_str()
    )
}

/// The contract every reproduction below shares: an allocator declared to
/// return **nothing**, so its acquisition is invisible to the projection.
const UNIT_RESULT: &str = "\
extern fn k_alloc(size: usize) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};
";

/// The body: one call whose result is discarded. Under `UNIT_RESULT` there is
/// nothing to account for; under the real C return type there is.
const DISCARDING_BODY: &str = "void w(void) { k_alloc(8); }";

// ---------------------------------------------------------------------------
// 1. The fail-open, reproduced — and the boundary that still carries it
// ---------------------------------------------------------------------------

/// **The reproduction.** With no C declaration of the callee in the unit there
/// is nothing to compare, the unfaithful `-> unit` declaration applies, and the
/// allocation it hides never enters the projection: the unit is `Safe`.
///
/// This is deliberately still true after this card: it is the recorded boundary
/// `LOADER-ABI-UNDECLARED`, and pinning it here is what keeps the boundary
/// honest instead of implied.
#[test]
fn without_a_c_declaration_the_unfaithful_result_type_still_applies() {
    let summary = analyse(DISCARDING_BODY, &load(UNIT_RESULT));
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Safe);
    assert!(unsupported(function(&summary, "w")).is_empty());
    assert_eq!(
        declared_abi_census(DISCARDING_BODY, &load(UNIT_RESULT))
            .into_iter()
            .map(|row| (row.function, row.outcome))
            .collect::<Vec<_>>(),
        vec![("k_alloc".to_owned(), AbiOutcome::Undeclared)]
    );
}

/// **The repair.** Declare the callee as C really declares it — returning a
/// pointer — and the same unfaithful contract is withdrawn: the call refuses
/// with the named key and the unit is `Unknown`.
#[test]
fn a_result_type_the_c_declaration_contradicts_withdraws_the_contract() {
    let source = format!("void *k_alloc(unsigned long size);\n{DISCARDING_BODY}");
    let contracts = load(UNIT_RESULT);
    let summary = analyse(&source, &contracts);
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Unknown);
    assert_eq!(
        unsupported(function(&summary, "w")),
        vec![withdrawal("k_alloc", AbiRefusal::ReturnTypeDisagreement)]
    );
    assert_eq!(
        declared_abi_census(&source, &contracts)
            .into_iter()
            .map(|row| (row.function, row.outcome))
            .collect::<Vec<_>>(),
        vec![(
            "k_alloc".to_owned(),
            AbiOutcome::Withdrawn(AbiRefusal::ReturnTypeDisagreement)
        )]
    );
}

/// The faithful declaration of the same callee is admitted, and then the
/// projection sees the acquisition it was hiding: the discarded owned result is
/// reported. Without this row the test above could be passing for the wrong
/// reason — a comparison that refuses everything.
#[test]
fn the_faithful_result_type_is_admitted_and_the_acquisition_becomes_visible() {
    let contracts = load(
        "\
extern fn k_alloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};
",
    );
    let source = format!("void *k_alloc(unsigned long size);\n{DISCARDING_BODY}");
    let summary = analyse(&source, &contracts);
    let w = function(&summary, "w");
    assert_eq!(w.status, AnalysisStatus::Unknown);
    assert!(
        unsupported(w)
            .iter()
            .all(|text| !text.contains("withdrawn against its C declaration")),
        "the faithful declaration must be admitted: {:?}",
        unsupported(w)
    );
    assert_eq!(
        declared_abi_census(&source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Agreed]
    );
}

/// The second direction the loader's type-keyed rules make reachable: a
/// declaration that names its parameters in the *other* order applies a
/// `Released` mode to the argument at the wrong position. The C declaration
/// contradicts it at both positions, so it is withdrawn.
#[test]
fn a_swapped_parameter_vector_is_withdrawn() {
    let contracts = load(
        "\
extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(handle: ptr<mut, u8>, flags: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(handle, nonnull);
    ensures normal resource_borrowed(flags);
    ensures normal resource_return_none();
};
",
    );
    let source = "void k_close(int flags, void *handle);\n\
                  void *k_open(void);\n\
                  void w(void) { void *h; h = k_open(); k_close(0, h); }";
    let summary = analyse(source, &contracts);
    assert_eq!(
        unsupported(function(&summary, "w")),
        vec![withdrawal("k_close", AbiRefusal::ParameterTypeDisagreement)]
    );
}

// ---------------------------------------------------------------------------
// 2. The admission and refusal shapes, one row each
// ---------------------------------------------------------------------------

/// One contracted callee, one C declaration, one expected outcome.
struct Row {
    label: &'static str,
    declared: &'static str,
    c: &'static str,
    outcome: Option<AbiRefusal>,
}

const ROWS: &[Row] = &[
    // -- exact agreement, the base case ------------------------------------
    Row {
        label: "the exact shape agrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n);",
        outcome: None,
    },
    // -- integer widths -----------------------------------------------------
    Row {
        label: "`unsigned long` admits `usize` on the LP64 targets audited here",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, unsigned long n);",
        outcome: None,
    },
    Row {
        label: "`unsigned long` admits `u64` too: one reading, two spellings",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, unsigned long n);",
        outcome: None,
    },
    Row {
        label: "specifier order is a multiset, so `long unsigned int` is the same type",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, long unsigned int n);",
        outcome: None,
    },
    Row {
        label: "signedness is not admitted across",
        declared: "p: ptr<mut, u8>, n: i64",
        c: "void k_use(void *p, unsigned long n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "width is not admitted across",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "plain `char` has an implementation-defined signedness: both byte spellings",
        declared: "p: ptr<mut, u8>, n: u8",
        c: "void k_use(void *p, char n);",
        outcome: None,
    },
    Row {
        label: "`signed char` fixes it, and refuses the other spelling",
        declared: "p: ptr<mut, u8>, n: u8",
        c: "void k_use(void *p, signed char n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    // -- pointers and qualifiers -------------------------------------------
    Row {
        label: "a const pointee is `ptr<const, u8>`, never `ptr<mut, u8>`",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(const char *p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and the faithful const spelling agrees",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const char *p, int n);",
        outcome: None,
    },
    Row {
        label: "a mutable pointee refuses the const spelling",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(char *p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "an opaque `struct` pointer is the same opaque projection",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "struct k_opaque;\nvoid k_use(struct k_opaque *p, int n);",
        outcome: None,
    },
    Row {
        label: "`volatile` on the pointee does not change the pointer's projection",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const volatile void *p, int n);",
        outcome: None,
    },
    Row {
        label: "a pointer to pointer is the out-slot projection",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(struct k_item **p, int n);",
        outcome: None,
    },
    Row {
        label: "a qualifier between two pointer levels has no spelling here",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(const char **p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a three-level pointer has no spelling here",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(char ***p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- C's own declarator adjustments ------------------------------------
    Row {
        label: "an array parameter is adjusted to a pointer (C11 6.7.6.3p7)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[], int n);",
        outcome: None,
    },
    Row {
        label: "a sized array parameter is the same adjustment",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[8], int n);",
        outcome: None,
    },
    // -- typedefs ----------------------------------------------------------
    Row {
        label: "a typedef is transparent, so it is resolved rather than refused",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef unsigned long k_size;\nvoid k_use(void *p, k_size n);",
        outcome: None,
    },
    Row {
        label: "a typedef chain resolves too",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef unsigned long k_word;\ntypedef k_word k_size;\nvoid k_use(void *p, k_size n);",
        outcome: None,
    },
    Row {
        label: "a typedef to a pointer resolves to the pointer projection",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef char *k_string;\nvoid k_use(k_string p, int n);",
        outcome: None,
    },
    Row {
        label: "and resolving it does not lose the disagreement underneath",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef int k_count;\nvoid k_use(void *p, k_count n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "a typedef declared twice to different targets is not resolved",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef unsigned long k_size;\ntypedef int k_size;\nvoid k_use(void *p, k_size n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- types with no established spelling --------------------------------
    Row {
        label: "an enumeration's compatible integer type is implementation-defined",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "enum k_kind { K_A, K_B };\nvoid k_use(void *p, enum k_kind n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "`long double` has no spelling in this vocabulary",
        declared: "p: ptr<mut, u8>, n: f64",
        c: "void k_use(void *p, long double n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a function-pointer parameter is not read",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void (*p)(int), int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- arity -------------------------------------------------------------
    Row {
        label: "a C declaration with more parameters disagrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n, int extra);",
        outcome: Some(AbiRefusal::ArityDisagreement),
    },
    Row {
        label: "a C declaration with fewer parameters disagrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p);",
        outcome: Some(AbiRefusal::ArityDisagreement),
    },
    // -- two declarations of one name --------------------------------------
    Row {
        label: "two agreeing declarations of the same name are one declaration",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n);\nvoid k_use(void *p, int n);",
        outcome: None,
    },
    Row {
        label: "two declarations that project differently are ambiguous",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n);\nvoid k_use(void *p, unsigned long n);",
        outcome: Some(AbiRefusal::AmbiguousDeclaration),
    },
    Row {
        label: "an unspecified list composes with a specified one (C11 6.2.7p3)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use();\nvoid k_use(void *p, int n);",
        outcome: None,
    },
    Row {
        label: "an unspecified list alone cannot check a declared parameter vector",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use();",
        outcome: Some(AbiRefusal::UnspecifiedParameters),
    },
];

/// Every row of the table, decided through the production analyzer.
#[test]
fn the_admission_and_refusal_table_decides_row_by_row() {
    for row in ROWS {
        let contracts = load(&format!(
            "extern fn k_use({}) -> unit\n\
             contract {{\n    \
             requires resource_contract_v1;\n    \
             ensures normal resource_borrowed(p);\n    \
             ensures normal resource_borrowed(n);\n    \
             ensures normal resource_return_none();\n\
             }};\n",
            row.declared
        ));
        let source = format!("{}\nvoid w(void) {{ }}", row.c);
        let measured = declared_abi_census(&source, &contracts);
        let expected = match row.outcome {
            None => AbiOutcome::Agreed,
            Some(refusal) => AbiOutcome::Withdrawn(refusal),
        };
        assert_eq!(
            measured
                .iter()
                .map(|row| row.outcome)
                .collect::<Vec<_>>()
                .as_slice(),
            [expected],
            "{}: {}",
            row.label,
            row.c
        );
    }
}

/// A zero-parameter contract against an unspecified C list is admitted, on a
/// stated ground rather than by exception: it claims nothing about parameters,
/// and the analyzer's existing arity gate refuses any call that passes one.
/// The measured instance is Fluent Bit's `double flb_time_now();`.
#[test]
fn a_zero_parameter_contract_survives_an_unspecified_c_list() {
    let contracts = load(
        "\
extern fn k_now() -> f64
contract {
    requires resource_contract_v1;
    ensures normal resource_return_none();
};
",
    );
    let source = "double k_now();\nvoid w(void) { k_now(); }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Agreed]
    );
    // The gate that makes it safe: a call that passes an actual is refused by
    // the existing arity rule, not admitted by this declaration.
    let calling = "double k_now();\nvoid w(void) { k_now(1); }";
    assert_eq!(
        unsupported(function(&analyse(calling, &contracts), "w")),
        vec![
            "arguments beyond the declared parameters of `k_now` are outside the resource \
             projection"
                .to_owned()
        ]
    );
}

/// The variadic rows, in both directions.
#[test]
fn the_variadic_tail_agrees_only_where_c_declares_one() {
    let with_tail = load(
        "\
extern fn k_log(format: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(format);
    ensures normal resource_variadic_data_tail();
    ensures normal resource_return_none();
};
",
    );
    let without_tail = load(
        "\
extern fn k_log(format: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(format);
    ensures normal resource_return_none();
};
",
    );
    let variadic = "void k_log(const char *format, ...);\nvoid w(void) { }";
    let plain = "void k_log(const char *format);\nvoid w(void) { }";
    let outcome = |source: &str, contracts: &ResourceContracts| {
        declared_abi_census(source, contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>()
    };
    assert_eq!(outcome(variadic, &with_tail), vec![AbiOutcome::Agreed]);
    // A declared tail for a function C does not declare variadic would admit
    // actuals C cannot accept at all.
    assert_eq!(
        outcome(plain, &with_tail),
        vec![AbiOutcome::Withdrawn(AbiRefusal::VariadicDisagreement)]
    );
    // The other direction only ever *narrows* what the arity gate admits, so
    // it is not a disagreement.
    assert_eq!(outcome(variadic, &without_tail), vec![AbiOutcome::Agreed]);
    assert_eq!(outcome(plain, &without_tail), vec![AbiOutcome::Agreed]);
}

// ---------------------------------------------------------------------------
// 3. What the comparison rests on, and what it refuses to guess
// ---------------------------------------------------------------------------

/// An environment with no retained declared texts declares no spellings, so
/// nothing is compared. That is the honest reading — an inferred W4 summary or
/// a directly built table never had a declared type — and it is pinned so the
/// direction cannot drift silently.
#[test]
fn an_environment_without_declared_sources_compares_nothing() {
    let mut contracts = ResourceContracts::default();
    contracts.insert_contract(
        "k_use",
        ResourceFunctionContract::new(
            vec![
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::None,
        ),
    );
    let source = "void k_use(int n, int m);\nvoid w(void) { }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Undeclared]
    );
}

/// A function **defined** in the unit whose declared contract its own
/// definition contradicts is withdrawn too: its callers stop trusting the
/// interface, and the body is no longer checked against it.
///
/// The second half is deliberately silent — a body checked contract-free
/// produces no diagnostic of its own — which is the refusing direction (no
/// entry custody is assumed) but is recorded as `LOADER-ABI-SILENT-DEFINITION`
/// rather than implied. The caller-side refusal below is what makes the
/// withdrawal visible in a report.
#[test]
fn a_defined_function_is_withdrawn_from_its_own_callers() {
    let contracts = load(
        "\
extern fn k_dup(s: ptr<const, u8>, n: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nullable);
};
",
    );
    // The definition in this unit takes `(char *, unsigned long)`, so neither
    // parameter position agrees with the declaration.
    let source = "void *k_dup(char *s, unsigned long n) { return s; }\n\
                  void w(void) { k_dup(0, 1); }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)]
    );
    let summary = analyse(source, &contracts);
    assert_eq!(
        unsupported(function(&summary, "w")),
        vec![withdrawal("k_dup", AbiRefusal::ParameterTypeDisagreement)]
    );
    // The definition itself is analysed, contract-free, and carries no
    // withdrawal diagnostic of its own.
    assert!(
        unsupported(function(&summary, "k_dup")).is_empty(),
        "the definition is checked contract-free, silently: {:?}",
        unsupported(function(&summary, "k_dup"))
    );
}

/// A retained source set the loader cannot re-read poisons the whole
/// comparison: **every** contracted callee is withdrawn, rather than none of
/// them. An empty map would apply every declaration, which is the fail-open
/// direction.
#[test]
fn unreadable_declared_sources_withdraw_every_contract() {
    let mut contracts = load(
        "\
extern fn k_use(p: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
",
    );
    contracts.set_source_assumptions(BTreeMap::from([(
        "k_api.qcore".to_owned(),
        "module contract; this is not a contract module".to_owned(),
    )]));
    let source = "void k_use(void *p, int n);\nvoid w(void) { k_use(0, 0); }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaredSources)]
    );
    assert_eq!(
        unsupported(function(&analyse(source, &contracts), "w")),
        vec![withdrawal("k_use", AbiRefusal::UnreadableDeclaredSources)]
    );
}

/// A withdrawal never moves a verdict towards `Safe`: it can only ever turn a
/// modelled call into an unmodelled one.
///
/// **What it can do, measured and recorded rather than implied
/// (`LOADER-ABI-WITHDRAWAL-UNSAFE`):** withdrawing a callee whose declared
/// `Released` mode was discharging an acquisition leaves that acquisition
/// unaccounted, and the analyzer then reports a **provisional** `Unsafe` — the
/// existing behaviour for *any* uncontracted release, and exactly why hard
/// invariant 2 forbids presenting a provisional `Unsafe` as an upstream defect.
/// No withdrawal occurs anywhere in the pinned corpus, so this direction is not
/// reachable there; it is pinned here so a future one is not a surprise.
#[test]
fn a_withdrawal_never_moves_a_verdict_towards_safe() {
    let contracts = load(
        "\
extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
",
    );
    let faithful = "void *k_open(void);\nvoid k_close(void *h, int n);\n\
                    void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
    let unfaithful = faithful.replace("void k_close(void *h, int n);", "void k_close(void *h);");
    let before = function(&analyse(faithful, &contracts), "w").status;
    let withdrawn = function(&analyse(&unfaithful, &contracts), "w").clone();
    assert_eq!(before, AnalysisStatus::Safe, "the faithful control decides");
    assert_ne!(
        withdrawn.status,
        AnalysisStatus::Safe,
        "the withdrawal must never certify"
    );
    assert_eq!(
        withdrawn.status,
        AnalysisStatus::Unsafe,
        "the measured direction: the release becomes invisible and the leak is alleged \
         provisionally"
    );
    assert!(
        unsupported(&withdrawn).contains(&withdrawal("k_close", AbiRefusal::ArityDisagreement)),
        "the withdrawal is reported, so the allegation is never silent: {:?}",
        unsupported(&withdrawn)
    );
}

/// The keyed refusal vocabulary is closed and round-trips through the token a
/// withdrawn call site carries, and no refusal is typed as a statement about
/// the environment.
#[test]
fn the_refusal_vocabulary_is_closed_and_round_trips() {
    for refusal in AbiRefusal::ALL {
        assert_eq!(AbiRefusal::from_token(refusal.token()), Some(refusal));
        assert!(!refusal.as_str().is_empty());
        assert!(!refusal.reason().is_empty());
        assert_ne!(
            refusal.category().subject(),
            ReasonSubject::Environment,
            "{}: an ABI disagreement is never an environment failure",
            refusal.as_str()
        );
    }
    // A disagreement is evidence bound to another declaration, not a construct
    // the analysed source got wrong.
    assert_eq!(
        AbiRefusal::ArityDisagreement.category(),
        ReasonCategory::StaleEvidence
    );
    assert_eq!(
        AbiRefusal::UnreadableDeclaration.category(),
        ReasonCategory::UnsupportedSyntax
    );
    assert_eq!(
        AbiRefusal::UnreadableDeclaredSources.category(),
        ReasonCategory::MissingPremise
    );
    assert_eq!(
        AbiRefusal::ALL.len(),
        AbiRefusal::ALL
            .iter()
            .map(|refusal| refusal.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "every key is distinct"
    );
}

/// The same decision on the production entry point, and at certificate
/// issuance: a unit whose declaration was withdrawn cannot be certified, and
/// the faithful control is.
#[test]
fn the_production_entry_point_and_the_certificate_agree_with_the_census() {
    if !clang_available() {
        eprintln!("skipping Clang-dependent LOADER-ABI test");
        return;
    }
    let contracts = load(UNIT_RESULT);
    let withdrawn = format!("void *k_alloc(unsigned long size);\n{DISCARDING_BODY}");
    match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &withdrawn,
            ..Default::default()
        },
        "w",
        &contracts,
    )
    .expect("Clang accepts the fixture")
    {
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            let summary = partial.expect("a partial summary is published");
            assert_eq!(
                unsupported(function(&summary, "w")),
                vec![withdrawal("k_alloc", AbiRefusal::ReturnTypeDisagreement)]
            );
        }
        other => panic!("a withdrawn declaration must not certify: {other:?}"),
    }
    // The control: the same body with the callee's own faithful contract, which
    // does certify, so the row above is not passing because nothing certifies.
    let faithful = load(
        "\
extern fn k_alloc(size: usize) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};
",
    );
    let agreeing = format!("void k_alloc(unsigned long size);\n{DISCARDING_BODY}");
    assert!(
        matches!(
            analyze_unrestricted_c11(
                &C11TranslationUnit {
                    source: &agreeing,
                    ..Default::default()
                },
                "w",
                &faithful,
            )
            .expect("Clang accepts the control"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the agreeing control must still certify"
    );
}
