//! HASH-CONTRACTS — declared contract text for the four `flb_hash_table_add`
//! publications, and the declared literal-argument premise that keeps it inside
//! the branch it was read from
//! (`docs/reports/remaining-hash-contracts/README.md`).
//!
//! Card O04-B09B10 declared `flb_hash_table_add` as borrowing every parameter
//! and recorded `O04-B09B10-VALUE-DEPENDENT`: the declaration is faithful at
//! the four call sites, which pass the literal `0`, and the v1 vocabulary has no
//! value-dependent mode, so it also covers call sites that pass a positive size.
//!
//! This card lands the production declaration and closes that hole. Three legs,
//! all through the public gateway entry points only.
//!
//! **Held-out generic family.** A `p_*` API deliberately unrelated to Fluent
//! Bit, ChunkIO and every earlier card's family.
//!
//! **Real source.** The byte-verbatim upstream regions the five earlier cards
//! pinned, under O04-B09B10's four non-hash premises and the *production*
//! contract module for the hash publication.
//!
//! **Adversarial.** The stored value's own cleanup obligation survives the
//! publication; the table's teardown discharges nothing; every non-literal and
//! non-zero call site is refused; a malformed premise refuses everything; and
//! the certificate recheck rests on the premise, not on the statement's shape.
//!
//! Nothing here promotes a verdict. `Safe` in the generic family is `Safe` for a
//! synthetic declared custody projection under premises this card names; both
//! real functions keep function-level status `Unknown`, and no raw unit is
//! promoted.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    LegacySafetyCertificate, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const PRELUDE: &str = include_str!("fixtures/o04/prelude.c");
const STORAGE_INPUT: &str = include_str!("fixtures/o04/flb_storage_input.h.txt");
const CALLER: &str = include_str!("fixtures/o04/flb_input_chunk_create.c.txt");
const CONSTRUCTOR: &str = include_str!("fixtures/o04/cio_chunk_open.c.txt");
const PATH_CONTRACTS: &str = include_str!("fixtures/contracts/o04_path.qcore");
const ARMS_CALLEES: &str = include_str!("fixtures/contracts/o04_arms_callees.qcore");
const B08_CALLEES: &str = include_str!("fixtures/contracts/o04_b08_callees.qcore");
/// O04-B09B10's four non-hash premises, reproduced; see the fixture header.
const OTHER_CALLEES: &str = include_str!("fixtures/contracts/hash_contracts_callees.qcore");
/// The declaration O04-B09B10 measured under, kept for the byte comparison.
const B09B10_CALLEES: &str = include_str!("fixtures/contracts/o04_b09b10_callees.qcore");
/// **The production contract module this card lands.**
const PUBLICATION: &str =
    include_str!("../../../contract/qcore/fluent_bit/hash_table_publication.qcore");
const GENERIC_CONTRACTS: &str = include_str!("fixtures/contracts/hash_contracts_generic.qcore");

/// The refusal a call outside its declared premise carries.
const OUTSIDE: &str = "call to `p_publish` outside its declared literal-argument premise";
/// The same refusal for the real callee.
const OUTSIDE_HASH: &str =
    "call to `flb_hash_table_add` outside its declared literal-argument premise";
/// The existing rejection every refused member access keeps.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn load(name: &str, source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(name.to_owned(), source.to_owned())])).unwrap()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent HASH-CONTRACTS test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("expected an analyzable partial summary: {other:?}"),
    }
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<&str> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.as_str()),
            _ => None,
        })
        .collect()
}

fn leaks(summary: &LegacyFunctionSummary) -> Vec<usize> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic.kind {
            LegacyDiagnosticKind::Leak { acquisition_line } => Some(acquisition_line),
            _ => None,
        })
        .collect()
}

// --------------------------------------------------------------------------
// Held-out generic family
// --------------------------------------------------------------------------

const DECLARATIONS: &str = "\
struct p_table { int size; }; \
struct p_value { int weight; }; \
struct p_table *p_open(int n); \
void p_close(struct p_table *t); \
struct p_value *p_new(int n); \
void p_release(struct p_value *v); \
int p_publish(struct p_table *t, void *v, long size);";

fn analyze_with(contracts: &str, body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &load("hash_contracts_generic.qcore", contracts),
    )
    .unwrap_or_else(|error| panic!("generic fixture rejected by Clang: {error:?}\n{source}"))
}

fn analyze_generic(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    analyze_with(GENERIC_CONTRACTS, body, entry)
}

fn certified(body: &str, entry: &str) -> LegacySafetyCertificate {
    match analyze_generic(body, entry) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
            certificate
        }
        other => panic!("expected a validated custody projection: {other:?}"),
    }
}

fn summary_of(body: &str, entry: &str) -> LegacyFunctionSummary {
    let result = analyze_generic(body, entry);
    match &result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate
            .summary
            .functions
            .get(entry)
            .expect("entry present")
            .clone(),
        _ => function(program(&result), entry).clone(),
    }
}

/// The prologue every generic body shares: open the table, make the value.
const OPENED: &str = "\
    struct p_table *t; struct p_value *v; \
    t = p_open(1); if (!t) { return; } \
    v = p_new(2); if (!v) { p_close(t); return; }";

/// The headline: a publication at the declared literal is admitted, and the
/// caller's own obligations for both resources still settle every exit.
#[test]
fn a_publication_at_the_declared_literal_is_admitted() {
    if !clang_available() {
        return;
    }
    let body =
        format!("void p_g1(void) {{ {OPENED} p_publish(t, v, 0); p_release(v); p_close(t); }}");
    let certificate = certified(&body, "p_g1");
    assert!(
        check_legacy_safety_certificate(&certificate).is_ok(),
        "the admitted publication rechecks"
    );
}

/// **The obligation is not deleted.** A published value whose release is
/// omitted is still reported as a leak: `resource_borrowed` moves nothing, so
/// the caller keeps exactly the obligation it had before the call.
#[test]
fn publishing_a_value_does_not_discharge_the_callers_obligation() {
    if !clang_available() {
        return;
    }
    let body = format!("void p_g2(void) {{ {OPENED} p_publish(t, v, 0); p_close(t); }}");
    let summary = summary_of(&body, "p_g2");
    assert_eq!(
        leaks(&summary).len(),
        1,
        "the published value must still be accounted for: {summary:?}"
    );
}

/// **No table-teardown claim.** Closing the table releases the table and
/// nothing else; the published value's obligation survives it. This is the
/// direction a `resource_stored` reading would have destroyed.
#[test]
fn closing_the_table_never_discharges_a_published_value() {
    if !clang_available() {
        return;
    }
    let closed = format!("void p_g3(void) {{ {OPENED} p_publish(t, v, 0); p_close(t); }}");
    let open = format!("void p_g3(void) {{ {OPENED} p_publish(t, v, 0); p_release(v); }}");
    assert_eq!(
        leaks(&summary_of(&closed, "p_g3")).len(),
        1,
        "closing the table leaves the value outstanding"
    );
    assert_eq!(
        leaks(&summary_of(&open, "p_g3")).len(),
        1,
        "releasing the value leaves the table outstanding"
    );
}

/// A double release through the published value is still caught.
#[test]
fn a_published_value_released_twice_is_still_reported() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void p_g4(void) {{ {OPENED} p_publish(t, v, 0); p_release(v); p_release(v); \
         p_close(t); }}"
    );
    let result = analyze_generic(&body, "p_g4");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a double release must not be projected as safe: {result:?}"
    );
}

/// A positive literal takes the other arm of the upstream body, which this
/// declaration never modelled, so the contract is not applied there.
#[test]
fn a_positive_literal_is_outside_the_declared_premise() {
    if !clang_available() {
        return;
    }
    let body =
        format!("void p_g5(void) {{ {OPENED} p_publish(t, v, 8); p_release(v); p_close(t); }}");
    let summary = summary_of(&body, "p_g5");
    assert!(
        unsupported(&summary).contains(&OUTSIDE),
        "expected the premise refusal: {:?}",
        unsupported(&summary)
    );
    assert!(
        !unsupported(&summary)
            .iter()
            .any(|text| text.contains("uncontracted function")),
        "a declared callee is not reported as undeclared: {:?}",
        unsupported(&summary)
    );
}

/// A size the projection cannot evaluate is refused for the same reason: the
/// premise is satisfied by the literal, never by an argument that might be it.
#[test]
fn a_non_literal_size_is_outside_the_declared_premise() {
    if !clang_available() {
        return;
    }
    for actual in [
        "n",
        "sizeof(struct p_value)",
        "(long) n",
        "0 + 0",
        "-0",
        "0L",
    ] {
        let body = format!(
            "void p_g6(int n) {{ {OPENED} p_publish(t, v, {actual}); p_release(v); p_close(t); }}"
        );
        let summary = summary_of(&body, "p_g6");
        assert!(
            unsupported(&summary).contains(&OUTSIDE),
            "`{actual}` must be refused: {:?}",
            unsupported(&summary)
        );
    }
}

/// The premise is what refuses: without the directive the same positive-size
/// call is admitted, which is the state this card found and repaired.
#[test]
fn without_the_declared_premise_the_positive_size_call_is_admitted() {
    if !clang_available() {
        return;
    }
    let generic = normalize_newlines(GENERIC_CONTRACTS);
    let unrestricted = generic.replace("    requires resource_literal_argument(size, 0);\n", "");
    assert_ne!(unrestricted, generic, "the substitution applied");
    let body =
        format!("void p_g7(void) {{ {OPENED} p_publish(t, v, 8); p_release(v); p_close(t); }}");
    assert!(
        matches!(
            analyze_with(&unrestricted, &body, "p_g7"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the unrestricted declaration covers the positive-size call"
    );
    assert!(
        !matches!(
            analyze_generic(&body, "p_g7"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the declared premise withdraws it"
    );
}

// --------------------------------------------------------------------------
// Card HASH-CLAUSE: migration parity, measured
// --------------------------------------------------------------------------

/// Every call-site shape whose treatment the premise decides: the declared
/// literal, spellings that are not it, and a call that does not reach the
/// declared parameter at all.
const PARITY_ACTUALS: &[&str] = &[
    "0",
    "8",
    "1",
    "-1",
    "n",
    "sizeof(struct p_value)",
    "(long) n",
    "0 + 0",
    "-0",
    "0L",
    "(0)",
];

/// **The refusal set the directive produced**, measured with both carriers
/// present and pinned here so the claim survives the directive's retirement.
/// `true` is "the declaration was withdrawn at this call site".
const PINNED_REFUSALS: &[bool] = &[
    false, // 0                       — the declared literal
    true,  // 8
    true,  // 1
    true,  // -1
    true,  // n
    true,  // sizeof(struct p_value)
    true,  // (long) n
    true,  // 0 + 0
    true,  // -0
    true,  // 0L
    true,  // (0)                      — parenthesised, not the literal token
];

/// `flb_input_chunk_create`'s complete ordered unsupported-diagnostic vector on
/// the real upstream body, measured under the comment directive and reproduced
/// under the clause. The four publications decide — none of them appears here —
/// and the two residues are rows 0 and 9 of O04-B09B10's census, owned by that
/// card's named follow-ons.
const PINNED_REAL_DIAGNOSTICS: &[&str] = &[
    "call to uncontracted function `flb_trace`",
    "nullable resource `ic` used by a non-null contract",
];

/// Whether the declaration was withdrawn at a publication passing `actual`.
fn withdrawn_under(contracts: &str, actual: &str) -> bool {
    let body = format!(
        "void p_gp(int n) {{ {OPENED} p_publish(t, v, {actual}); p_release(v); p_close(t); }}"
    );
    let result = analyze_with(contracts, &body, "p_gp");
    match &result {
        UnrestrictedLegacyResult::Safe(_) => false,
        _ => unsupported(function(program(&result), "p_gp")).contains(&OUTSIDE),
    }
}

/// **The migration parity measurement, pinned.** At every call-site shape the
/// premise decides, the clause withdraws exactly the set card HASH-CONTRACTS'
/// comment directive withdrew. Both carriers were run against this vector while
/// both existed (`docs/reports/remaining-hash-clause/logs/carrier-parity.txt`);
/// the directive is now retired, and this pin is what keeps the claim checkable.
#[test]
fn the_clause_reproduces_the_directives_refusal_set() {
    if !clang_available() {
        return;
    }
    let measured: Vec<bool> = PARITY_ACTUALS
        .iter()
        .map(|actual| withdrawn_under(GENERIC_CONTRACTS, actual))
        .collect();
    assert_eq!(
        measured, PINNED_REFUSALS,
        "the measured refusal set moved: {PARITY_ACTUALS:?}"
    );
}

/// The same parity on the real upstream body: the production module's clause
/// leaves `flb_input_chunk_create`'s diagnostics exactly as the directive did.
/// The vector is pinned in `PINNED_REAL_REFUSALS`.
#[test]
fn the_clause_reproduces_the_directives_real_source_diagnostics() {
    if !clang_available() {
        return;
    }
    let source = real_unit();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("hash_contracts_path.qcore", &real_contracts()),
    )
    .expect("real fixture accepted by Clang");
    let measured = unsupported(function(program(&result), "flb_input_chunk_create"));
    assert_eq!(
        measured, PINNED_REAL_DIAGNOSTICS,
        "the real-source diagnostic vector moved"
    );
    assert!(
        !measured.contains(&OUTSIDE_HASH),
        "the four publications decide under the clause"
    );
}

/// The parity table archived by card HASH-CLAUSE, printed rather than asserted.
/// The directive column is gone with the directive; the transcript taken while
/// both carriers existed is
/// `docs/reports/remaining-hash-clause/logs/carrier-parity.txt`.
#[test]
#[ignore = "reporting aid, not an assertion"]
fn print_the_carrier_parity_table() {
    if !clang_available() {
        return;
    }
    println!("{:<26} {:>8} {:>10}", "val_size actual", "clause", "pinned");
    for (index, actual) in PARITY_ACTUALS.iter().enumerate() {
        println!(
            "{:<26} {:>8} {:>10}",
            actual,
            withdrawal(withdrawn_under(GENERIC_CONTRACTS, actual)),
            withdrawal(PINNED_REFUSALS[index]),
        );
    }
}

fn withdrawal(refused: bool) -> &'static str {
    if refused { "withdrawn" } else { "applied" }
}

/// A premise text the loader cannot re-read refuses **every** contracted call
/// rather than silently applying the contracts a typo left unrestricted.
///
/// A malformed clause rejects the module outright, so the only way to reach the
/// analyzer with one is to substitute the retained text after loading — which is
/// exactly what a tampered certificate would do. The premise set poisons; it
/// does not become empty, which would apply every declaration everywhere.
#[test]
fn a_malformed_premise_refuses_every_contracted_call() {
    if !clang_available() {
        return;
    }
    let broken = GENERIC_CONTRACTS.replace("(size, 0)", "(absent, 0)");
    assert_ne!(broken, GENERIC_CONTRACTS, "the substitution applied");
    assert!(
        load_resource_contract_sources(&BTreeMap::from([(
            "hash_contracts_generic.qcore".to_owned(),
            broken.clone(),
        )]))
        .is_err(),
        "a malformed clause must reject the module"
    );
    let body =
        format!("void p_g8(void) {{ {OPENED} p_publish(t, v, 0); p_release(v); p_close(t); }}");
    let mut contracts = load("hash_contracts_generic.qcore", GENERIC_CONTRACTS);
    contracts.set_source_assumptions(BTreeMap::from([(
        "hash_contracts_generic.qcore".to_owned(),
        broken,
    )]));
    let source = format!("{DECLARATIONS} {body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "p_g8",
        &contracts,
    )
    .expect("fixture accepted by Clang");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a poisoned premise set must not certify: {result:?}"
    );
    let summary = function(program(&result), "p_g8");
    for callee in ["p_open", "p_new", "p_publish", "p_release", "p_close"] {
        assert!(
            unsupported(summary)
                .iter()
                .any(|text| text.contains(callee)),
            "`{callee}` must be refused under a poisoned premise set: {:?}",
            unsupported(summary)
        );
    }
}

/// The certificate rests on the declared premise: a certificate derived without
/// it is refused by a recheck that has it, in the direction that matters —
/// the recheck rederives the restriction from the certificate's own text.
#[test]
fn a_certificate_rests_on_the_declared_premise() {
    if !clang_available() {
        return;
    }
    let body =
        format!("void p_g9(void) {{ {OPENED} p_publish(t, v, 0); p_release(v); p_close(t); }}");
    let certificate = certified(&body, "p_g9");
    check_legacy_safety_certificate(&certificate).expect("the unmodified certificate rechecks");
    // Move the admitted call outside the premise in the certificate's own
    // source. The recheck must refuse it.
    let mut forged = certificate.clone();
    forged.preprocessed_source = forged
        .preprocessed_source
        .replace("p_publish(t, v, 0)", "p_publish(t, v, 8)");
    assert_ne!(
        forged.preprocessed_source, certificate.preprocessed_source,
        "the substitution applied"
    );
    assert!(
        check_legacy_safety_certificate(&forged).is_err(),
        "a call outside the premise must be refused by the recheck"
    );
}

/// A declaration position is never rewritten: a unit that *defines* the callee
/// keeps its definition analyzable.
#[test]
fn the_callees_own_definition_is_never_rewritten() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{DECLARATIONS} int p_publish(struct p_table *t, void *v, long size) \
         {{ (void) t; (void) v; return (int) size; }}"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "p_publish",
        &load("hash_contracts_generic.qcore", GENERIC_CONTRACTS),
    )
    .expect("fixture accepted by Clang");
    assert!(
        !matches!(
            result,
            UnrestrictedLegacyResult::Unknown { partial: None, .. }
        ),
        "the definition must remain analyzable: {result:?}"
    );
}

// --------------------------------------------------------------------------
// Real source
// --------------------------------------------------------------------------

fn real_unit() -> String {
    format!("{PRELUDE}\n{STORAGE_INPUT}\n{CONSTRUCTOR}\n{CALLER}\n")
}

/// The production module is a standalone corpus module, so it opens with its
/// own `module contract;`. Concatenating it onto the path fixture keeps its
/// declaration and its premise clause byte-for-byte; only that one module
/// header, which the fixture already supplies, is dropped.
fn publication_fragment() -> String {
    let normalized = normalize_newlines(PUBLICATION);
    let body = normalized
        .strip_prefix("module contract;\n")
        .expect("the production module opens with its module header");
    assert!(
        body.contains("    requires resource_literal_argument(val_size, 0);"),
        "the production module carries its declared premise clause"
    );
    assert!(
        !body.contains("qcore-premise:"),
        "the retired comment directive is gone from the production module"
    );
    body.to_owned()
}

fn real_contracts() -> String {
    normalize_newlines(&format!(
        "{PATH_CONTRACTS}\n{ARMS_CALLEES}\n{B08_CALLEES}\n{OTHER_CALLEES}\n{}",
        publication_fragment()
    ))
}

fn analyze_real(source: &str) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "flb_input_chunk_create",
        &load("hash_contracts_path.qcore", &real_contracts()),
    )
    .expect("real fixture accepted by Clang")
}

/// The four publications decide exactly as O04-B09B10 measured them, now under
/// the production declaration and its premise: no `INDIRECT` refusal remains,
/// and no publication is refused as outside its premise.
#[test]
fn the_four_publications_decide_under_the_production_declaration() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    let summary = program(&result);
    let caller = function(summary, "flb_input_chunk_create");
    assert_eq!(
        unsupported(caller)
            .iter()
            .filter(|text| **text == INDIRECT)
            .count(),
        0,
        "no member access remains refused: {:?}",
        unsupported(caller)
    );
    assert!(
        !unsupported(caller).contains(&OUTSIDE_HASH),
        "every publication passes the literal `0`: {:?}",
        unsupported(caller)
    );
    assert!(
        !unsupported(caller)
            .iter()
            .any(|text| text.contains("flb_hash_table_add")),
        "the publication is fully declared: {:?}",
        unsupported(caller)
    );
}

/// One publication moved off the literal is refused, and only that one. This is
/// the control that shows the four admissions rest on the premise.
#[test]
fn one_publication_moved_off_the_literal_is_refused() {
    if !clang_available() {
        return;
    }
    let moved = real_unit().replace(
        "flb_hash_table_add(in->ht_metric_chunks, tag, tag_len, ic, 0)",
        "flb_hash_table_add(in->ht_metric_chunks, tag, tag_len, ic, 1)",
    );
    assert_ne!(moved, real_unit(), "the substitution applied");
    let result = analyze_real(&moved);
    let caller = function(program(&result), "flb_input_chunk_create");
    assert_eq!(
        unsupported(caller)
            .iter()
            .filter(|text| **text == OUTSIDE_HASH)
            .count(),
        1,
        "exactly the moved publication is refused: {:?}",
        unsupported(caller)
    );
}

/// Both real functions keep function-level status `Unknown`. Nothing here
/// promotes a verdict.
#[test]
fn neither_real_function_is_promoted() {
    if !clang_available() {
        return;
    }
    let result = analyze_real(&real_unit());
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "the real unit is never certified by this card: {result:?}"
    );
}

/// The four non-hash premises are character-for-character O04-B09B10's, so the
/// only difference between that card's measurement and this one is the hash
/// declaration itself.
#[test]
fn the_other_four_premises_are_o04_b09b10s_verbatim() {
    for declaration in [
        "msgpack_packer_init",
        "mk_list_add",
        "pthread_mutex_lock",
        "pthread_mutex_unlock",
    ] {
        let mine = declaration_text(OTHER_CALLEES, declaration);
        let theirs = declaration_text(B09B10_CALLEES, declaration);
        assert_eq!(mine, theirs, "`{declaration}` diverged from O04-B09B10");
    }
    assert!(
        !OTHER_CALLEES.contains("extern fn flb_hash_table_add"),
        "the hash declaration comes from the production module"
    );
}

/// The declaration text of `name`, from its `extern fn` line to its closing
/// `};`, with line endings normalized.
fn declaration_text(module: &str, name: &str) -> String {
    let normalized = normalize_newlines(module);
    let start = normalized
        .find(&format!("extern fn {name}("))
        .unwrap_or_else(|| panic!("`{name}` not declared"));
    let end = normalized[start..]
        .find("};")
        .unwrap_or_else(|| panic!("`{name}` unterminated"));
    normalized[start..start + end + 2].to_owned()
}

/// **The stored mode is not merely false here, it is unreachable.** §C04.2
/// requires the owner parameter to be declared `Owned`, and no caller of this
/// function supplies live custody of the table, so a `resource_stored` reading
/// cannot even be loaded against the measured signature.
#[test]
fn the_stored_mode_is_rejected_for_this_callee() {
    let stored = publication_fragment().replace(
        "ensures normal resource_borrowed(val);",
        "ensures normal resource_stored(val, ht, nonnull);",
    );
    assert_ne!(stored, publication_fragment(), "the substitution applied");
    let module = format!("module contract;\n{stored}");
    let error =
        load_resource_contract_sources(&BTreeMap::from([("stored.qcore".to_owned(), module)]))
            .expect_err("a stored child under a borrowed owner must not load");
    assert!(
        error.contains("flb_hash_table_add"),
        "the refusal names the declaration: {error}"
    );
}

/// The measured per-row ledger for the report: each publication's refusal
/// before this card's declaration, and after it.
#[test]
#[ignore = "reporting aid, not an assertion"]
fn print_the_measured_publication_ledger() {
    if !clang_available() {
        return;
    }
    let without = normalize_newlines(&format!(
        "{PATH_CONTRACTS}\n{ARMS_CALLEES}\n{B08_CALLEES}\n{OTHER_CALLEES}"
    ));
    for (label, contracts) in [("BEFORE", without), ("AFTER", real_contracts())] {
        let unit = real_unit();
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &unit,
                ..Default::default()
            },
            "flb_input_chunk_create",
            &load("hash_contracts_path.qcore", &contracts),
        )
        .expect("real fixture accepted by Clang");
        let summary = program(&result);
        let caller = function(summary, "flb_input_chunk_create");
        println!("== {label}: status {:?}", caller.status);
        let source = match &result {
            UnrestrictedLegacyResult::Unknown {
                preprocessed_source,
                ..
            } => preprocessed_source.clone(),
            _ => String::new(),
        };
        let lines = source.lines().collect::<Vec<_>>();
        for diagnostic in &caller.diagnostics {
            if let LegacyDiagnosticKind::Unsupported { description } = &diagnostic.kind {
                let text = lines
                    .get(diagnostic.line.saturating_sub(1))
                    .unwrap_or(&"")
                    .trim();
                println!("  L{} {text} | {description}", diagnostic.line);
            }
        }
    }
}

/// No corpus symbol may begin with the rewrite prefix, or a real call could be
/// mistaken for a rewritten one.
#[test]
fn the_rewrite_prefix_collides_with_no_upstream_symbol() {
    for text in [
        PRELUDE,
        STORAGE_INPUT,
        CALLER,
        CONSTRUCTOR,
        PATH_CONTRACTS,
        ARMS_CALLEES,
        B08_CALLEES,
        OTHER_CALLEES,
        PUBLICATION,
        GENERIC_CONTRACTS,
        DECLARATIONS,
    ] {
        assert!(
            !text.contains("qcore_premise_unmet__"),
            "the rewrite prefix must not occur in any input"
        );
    }
}
