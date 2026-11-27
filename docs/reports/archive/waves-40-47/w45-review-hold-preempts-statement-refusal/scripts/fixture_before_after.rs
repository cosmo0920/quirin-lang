//! Fixture before/after probe of card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL.
//!
//! Copied by `fixture_before_after.sh` into a tree's
//! `crates/qcore-c-gateway/tests/` and run there with `--nocapture`; it prints
//! one `ROW` line per (shape, hold) with the row status and diagnostics, and
//! asserts nothing, so the same file runs at the base (`fefb9199`, before the
//! card) and at the card's tip. The shapes are those of
//! `crates/qcore-c-gateway/tests/review_hold_standing.rs`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{ResourceContracts, analyze_legacy_c_translation_unit};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract_revision::PinMismatchReason;

const BARE: &str = "module contract;

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

extern fn k_input_new(config: ptr<mut, u8>, input: ptr<const, u8>, data: ptr<mut, u8>, public_only: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(config);
    ensures normal resource_borrowed(input);
    ensures normal resource_borrowed(data);
    ensures normal resource_borrowed(public_only);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "void *k_open(void);\nvoid k_close(void *h, int n);\n\
     struct k_config;\nstruct k_ctx { struct k_config *config; struct k_ctx *next; };\n\
     void *k_input_new(struct k_config *config, const char *input, void *data, int public_only);\n";

const SHAPES: &[(&str, &str, &str)] = &[
    ("argument-access", "k_input_new", "int w(struct k_ctx **ctx, const char *input, void *data) \
     { void *i; i = k_input_new((*ctx)->config, input, data, 1); if (!i) { return -1; } return 0; }"),
    ("plain-actual", "k_input_new", "int w(struct k_config *config, const char *input, void *data) \
     { void *i; i = k_input_new(config, input, data, 1); if (!i) { return -1; } return 0; }"),
    ("twice", "k_close", "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); k_close(h, 0); }"),
    ("leaked", "k_open", "void w(void) { void *h; h = k_open(); if (!h) { return; } }"),
    ("balanced", "k_close", "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }"),
    ("later-refusal", "k_input_new", "int w(struct k_config *config, const char *input, void *data, struct k_ctx *x) \
     { void *i; i = k_input_new(config, input, data, 1); x->next->next = x; if (!i) { return -1; } return 0; }"),
];

fn load(text: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("k.qv".to_owned(), text.to_owned())])).unwrap()
}

fn deferred(callee: &str) -> ResourceContracts {
    let start = BARE.find(&format!("extern fn {callee}(")).unwrap();
    let opening = "    requires resource_contract_v1;\n";
    let at = start + BARE[start..].find(opening).unwrap() + opening.len();
    let mut text = BARE.to_owned();
    text.insert_str(at, "    requires resource_review_pending(\"under review\");\n");
    load(&text)
}

fn pin_withheld(callee: &str) -> ResourceContracts {
    let mut contracts = load(BARE);
    contracts
        .withhold_for_pin_mismatch(
            callee,
            "k.qv".into(),
            PinMismatchReason::SourceChanged,
            Some("contracts.lock".into()),
            "differs".into(),
        )
        .unwrap();
    contracts
}

#[test]
fn fixture_before_after() {
    for (name, callee, body) in SHAPES {
        for (hold, contracts) in [
            ("in-force", load(BARE)),
            ("review", deferred(callee)),
            ("pin", pin_withheld(callee)),
        ] {
            let summary =
                analyze_legacy_c_translation_unit(&format!("{PRELUDE}{body}"), "w", &contracts)
                    .unwrap();
            let row = &summary.functions["w"];
            println!("ROW {name} {hold} {:?} {:?}", row.status, row.diagnostics);
        }
    }
}
