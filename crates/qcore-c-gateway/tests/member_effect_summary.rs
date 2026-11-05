//! Card MEMBER-EFFECT-SUMMARY (`docs/reports/member-effect-summary/`).
//!
//! Step 1, gateway rule 38i: the gate's refusal of member custody through a
//! formal parent is named per premise and its verdict is unchanged. Each
//! fixture is a census shape of the 046accf6 `current` run
//! (MEMBER-ACCESS-PROJECTION section 3): a clear-style helper releasing a
//! formal's members and keeping the formal (`cmt_opts_exit`), a setter
//! storing a formal or a fresh allocation into a member (`confval->name =
//! name`, `map->unit = cfl_sds_create(unit)`), and an embedded list's address
//! handed to a releasing callee.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const EXTRA: &str = r#"module contract;

extern fn obj_destroy(obj: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_released(obj, nonnull);
    ensures normal resource_return_none();
};

extern fn list_destroy(list: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_released(list, nonnull);
    ensures normal resource_return_none();
};

extern fn obj_create(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};
"#;

const RECORD: &str = "struct inner { int x; };\nstruct o { char *a; char *b; struct inner in; struct o *next; int n; };\nvoid obj_destroy(void *obj);\nvoid list_destroy(void *list);\nvoid *obj_create(size_t size);\nvoid uncontracted(void *p);\nstruct o *other(void);\n";

const GATE: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let files: BTreeMap<String, String> = [
        (
            "libc/memory.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv")
                .to_owned(),
        ),
        ("fixture/member_effect.qv".to_owned(), EXTRA.to_owned()),
    ]
    .into_iter()
    .collect();
    load_resource_contract_sources(&files).expect("the modules load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent MEMBER-EFFECT-SUMMARY test: {reason}");
            false
        }
    }
}

/// The entry's status and its diagnostics' descriptions.
fn leaves(entry: &str, body: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!("#include <stdlib.h>\n{RECORD}{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error:?}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let function = &summary.functions[entry];
    (
        function.status,
        function
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    )
}

fn named(base: &str, member: &str) -> String {
    format!(
        "release of `{base}->{member}`, a member of the formal `{base}` \
         (member-release-through-formal)"
    )
}

fn stored(base: &str, member: &str) -> String {
    format!(
        "store of a custody-carrying value into `{base}->{member}`, a member of the formal \
         `{base}` (member-store-through-formal)"
    )
}

fn assert_leaves(entry: &str, body: &str, expected: &[String]) {
    let (status, descriptions) = leaves(entry, body);
    assert_eq!(status, AnalysisStatus::Unknown, "{entry}: {descriptions:?}");
    assert_eq!(descriptions, expected, "{entry}");
}

fn assert_safe(entry: &str, body: &str) {
    let (status, descriptions) = leaves(entry, body);
    assert_eq!(status, AnalysisStatus::Safe, "{entry}: {descriptions:?}");
}

fn assert_unnamed(entry: &str, body: &str) {
    let (status, descriptions) = leaves(entry, body);
    assert_ne!(status, AnalysisStatus::Safe, "{entry}");
    assert!(
        !descriptions.iter().any(|d| d.contains("through-formal")),
        "{entry}: {descriptions:?}"
    );
}

#[test]
fn a_member_release_through_a_kept_formal_is_named_and_stays_unknown() {
    if !clang_available() {
        return;
    }
    // `cmt_opts_exit`: both members released, the parent kept.
    assert_leaves(
        "clear",
        "void clear(struct o *opts) {\n  free(opts->a);\n  free(opts->b);\n}",
        &[named("opts", "a"), named("opts", "b")],
    );
    // A non-null release.
    assert_leaves(
        "clear_nn",
        "void clear_nn(struct o *opts) {\n  obj_destroy(opts->a);\n}",
        &[named("opts", "a")],
    );
    // Under a null test of the member.
    assert_leaves(
        "clear_guard",
        "void clear_guard(struct o *opts) {\n  if (opts->a)\n    obj_destroy(opts->a);\n}",
        &[named("opts", "a")],
    );
}

#[test]
fn rule_5k_still_admits_a_member_release_before_the_parents() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "destroy",
        "void destroy(struct o *opts) {\n  free(opts->a);\n  free(opts);\n}",
    );
}

#[test]
fn a_custody_store_into_a_formal_member_is_named_and_stays_unknown() {
    if !clang_available() {
        return;
    }
    // `confval->name = name`.
    assert_leaves(
        "setname",
        "void setname(struct o *c, char *name) {\n  c->a = name;\n}",
        &[stored("c", "a")],
    );
    // `map->unit = cfl_sds_create(unit)`: an owned result.
    assert_leaves(
        "setnew",
        "void setnew(struct o *c) {\n  c->a = malloc(4);\n}",
        &[stored("c", "a")],
    );
    assert_leaves(
        "setnew2",
        "void setnew2(struct o *c) {\n  c->a = obj_create(4);\n}",
        &[stored("c", "a")],
    );
    // An identity-free value is rule 10v's admission, unchanged.
    assert_safe("setnull", "void setnull(struct o *c) {\n  c->a = NULL;\n}");
}

#[test]
fn an_embedded_member_address_at_a_release_is_named() {
    if !clang_available() {
        return;
    }
    assert_leaves(
        "addr",
        "void addr(struct o *c) {\n  list_destroy(&c->in);\n}",
        &["release of `&c->in`, an address inside the formal `c` \
           (member-address-release-through-formal)"
            .to_owned()],
    );
}

#[test]
fn every_other_shape_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    // A local parent is no formal parent.
    assert_unnamed(
        "local",
        "void local(void) {\n  struct o *c = other();\n  free(c->a);\n}",
    );
    // A rebound formal.
    assert_unnamed(
        "rebound",
        "void rebound(struct o *opts) {\n  opts = opts->next;\n  free(opts->a);\n}",
    );
    // A stored value that reads memory, and a chain below the member.
    assert_leaves(
        "copy",
        "void copy(struct o *c, struct o *d) {\n  c->a = d->b;\n}",
        &[GATE.to_owned()],
    );
    assert_leaves(
        "chain",
        "void chain(struct o *c) {\n  free(c->next->a);\n}",
        &[GATE.to_owned()],
    );
    // An uncontracted callee keeps rule 12k's name.
    assert_leaves(
        "callee",
        "void callee(struct o *c) {\n  uncontracted(c->a);\n}",
        &["call to uncontracted function `uncontracted`".to_owned()],
    );
}
