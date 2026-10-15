//! Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Only
//! nonnull_member* need it"; gateway rules 14x, 14z): a named premise beside
//! a stable member or a scope entry, the loader's placement rule, and every
//! null row resting on the premised fact listing it, on a real Clang AST.
//! The premise changes no verdict, check or reason.
//!
//! The fixture declarations are trusted test assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, NullUnitReport, analyze_null_c11};
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_schemas::contract::ResourceContracts;

const DECLARATIONS: &str = "\
struct S { int a; struct S *next; };
void nd_use(void *p); struct S *nd_make(int k);
";

const PREMISE: &str =
    "requires resource_premise(no_plugin, \"no code outside the run stores into S.next\");";

/// The stable member and a scope entry, each premised (or not).
fn clauses(premised: bool) -> String {
    let premise = if premised { PREMISE } else { "" };
    format!(
        "module contract;
extern fn nd_make(k: i32) -> ptr<mut, u8>
contract {{
    requires bounds_contract_v1;
    requires nonnull_member_stable(S, next);
    {premise}
}};
extern fn scoped(s: ptr<mut, u8>) -> i32
contract {{
    requires bounds_contract_v1;
    requires nonnull_member_scope(S, next, s);
    {premise}
}};
extern fn deref(p: ptr<mut, u8>) -> unit
contract {{
    requires bounds_contract_v1;
    quant null-deref #p;
}};
"
    )
}

fn bounds(text: &str) -> BoundsContracts {
    load_bounds_contract_sources(&BTreeMap::from([(
        "nd_bounds.qv".to_owned(),
        text.to_owned(),
    )]))
    .expect("the fixture bounds contracts load")
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

const SOURCE: &str = "\
void deref(struct S *p);
int scoped(struct S *s) { if (!s) return 0; nd_use(s); return s->next->a; }
int keeper(struct S *s) { if (!s || !s->next) return 0; nd_use(s); return s->next->a; }
int caller(struct S *s) { if (!s || !s->next) return 0; return scoped(s); }
int plain(struct S *s) { deref(s); return 0; }
";

const FUNCTIONS: [&str; 4] = ["scoped", "keeper", "caller", "plain"];

fn report(premised: bool) -> NullUnitReport {
    let text = format!("{DECLARATIONS}\n{SOURCE}\n");
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    let functions = FUNCTIONS
        .iter()
        .map(|f| (*f).to_owned())
        .collect::<Vec<_>>();
    analyze_null_c11(
        &unit,
        &functions,
        &ResourceContracts::default(),
        &bounds(&clauses(premised)),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error}\n{text}"))
}

fn row<'a>(unit: &'a NullUnitReport, name: &str) -> &'a NullFunctionRow {
    unit.row(name).expect("the row")
}

/// The row without the fields the premise adds.
fn stripped(row: &NullFunctionRow) -> NullFunctionRow {
    let mut row = row.clone();
    row.premises.clear();
    row.contract_dependencies
        .retain(|d| !d.starts_with("premise:"));
    row
}

#[test]
fn every_row_resting_on_a_premised_member_fact_lists_the_premise() {
    if !clang_available() {
        return;
    }
    let unit = report(true);
    // The scope entry's own body and a caller checked against it, through the
    // function's name; the kept fact, through `stable:S.next`.
    for (name, through) in [
        ("scoped", "scoped"),
        ("keeper", "stable:S.next"),
        ("caller", "scoped"),
    ] {
        let row = row(&unit, name);
        assert_eq!(row.premises.len(), 1, "{name}: {row:#?}");
        let premise = &row.premises[0];
        assert_eq!(premise.name, "no_plugin");
        assert_eq!(
            premise.statement,
            "no code outside the run stores into S.next"
        );
        assert!(
            premise.through.iter().any(|d| d == through),
            "{name}: {premise:?}"
        );
        assert!(
            row.contract_dependencies
                .contains(&"premise:no_plugin".to_owned()),
            "{name}: {:?}",
            row.contract_dependencies
        );
    }
    // A `quant null-deref` fact carries no premise (decision 1).
    let plain = row(&unit, "plain");
    assert!(plain.premises.is_empty(), "{plain:#?}");
    assert!(
        !plain
            .contract_dependencies
            .iter()
            .any(|d| d.starts_with("premise:")),
        "{plain:#?}"
    );
}

#[test]
fn listing_the_premise_changes_no_verdict_check_or_reason() {
    if !clang_available() {
        return;
    }
    let with = report(true);
    let without = report(false);
    for name in FUNCTIONS {
        let (a, b) = (row(&with, name), row(&without, name));
        assert!(b.premises.is_empty(), "{name}");
        assert_eq!(stripped(a), *b, "{name}");
    }
}

#[test]
fn the_premise_stands_only_beside_a_scoped_member_form() {
    let text = |body: &str| {
        format!(
            "module contract;\nextern fn f(s: ptr<mut, u8>, k: i32) -> unit\ncontract {{\n    requires bounds_contract_v1;\n    {body}\n}};\n"
        )
    };
    let load = |body: &str| {
        load_bounds_contract_sources(&BTreeMap::from([("m.qv".to_owned(), text(body))]))
    };
    let ok = load(&format!(
        "requires nonnull_member_stable(S, next);\n    {PREMISE}"
    ))
    .expect("beside a stable member");
    assert_eq!(
        ok.functions["f"].premises(),
        vec![("no_plugin", "no code outside the run stores into S.next")]
    );
    // The bounds consumer never reads it, nor the member clause.
    assert!(ok.for_bounds().functions.is_empty());
    assert!(
        load(&format!(
            "requires nonnull_member_scope(S, next, s);\n    {PREMISE}"
        ))
        .is_ok()
    );
    // Beside no scoped member form: alone, beside a `quant null-deref`,
    // beside rule 14t's plain member, beside a bounds clause.
    assert!(load(PREMISE).is_err());
    assert!(load(&format!("quant null-deref #s;\n    {PREMISE}")).is_err());
    assert!(load(&format!("requires nonnull_member(S, next);\n    {PREMISE}")).is_err());
    assert!(load(&format!("requires bounds_index(k, 4);\n    {PREMISE}")).is_err());
    // Malformed, repeated or a postcondition: the module is rejected.
    for bad in [
        "requires resource_premise(No_plugin, \"x\");",
        "requires resource_premise(no_plugin, x);",
        "requires resource_premise(no_plugin, \"\");",
        "requires resource_premise(no_plugin, \"x\", \"y\");",
        "ensures normal resource_premise(no_plugin, \"x\");",
        "requires resource_premise(no_plugin, \"x\");\n    requires resource_premise(no_plugin, \"y\");",
        "requires bounds_review_pending(resource_premise(no_plugin, \"x\"), \"why\");",
    ] {
        assert!(
            load(&format!(
                "requires nonnull_member_stable(S, next);\n    {bad}"
            ))
            .is_err(),
            "{bad} must reject the module"
        );
    }
}
