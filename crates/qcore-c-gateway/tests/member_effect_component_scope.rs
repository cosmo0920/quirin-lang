//! Card MEMBER-EFFECT-COMPONENT-SCOPE
//! (`docs/reports/member-effect-component-scope/`).
//!
//! Diagnosis. `cmt_opts.c::cmt_opts_exit` stayed `Unknown` under rule 38j
//! because its members are spelled through a pointer typedef (`cfl_sds_t ns;`,
//! `typedef char *cfl_sds_t;`): the lexical recogniser of a call actual
//! (`legacy_call_argument_access.rs::pointer_read`, `pointer_member`) refuses
//! `opts->ns` as an untyped member, and rule 1's admission of a member handed
//! to a nullable release needs that recogniser's pointer read. The row's scope
//! (the unit's function list, which every row of a unit shares) and the
//! sibling `cmt_opts_init` play no part: the same body with `char *` members is
//! `Safe`, alone or beside its initialiser, and the typedef-spelled body is
//! refused alone as beside it.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

fn contracts() -> ResourceContracts {
    let files: BTreeMap<String, String> = [
        (
            "libc/memory.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv")
                .to_owned(),
        ),
        (
            "cfl/sds_lifetime.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv")
                .to_owned(),
        ),
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
            eprintln!("skipping Clang-dependent MEMBER-EFFECT-COMPONENT-SCOPE test: {reason}");
            false
        }
    }
}

/// The entry's status and its diagnostics' descriptions.
fn leaves(entry: &str, body: &str) -> (AnalysisStatus, Vec<String>) {
    let text = format!("#include <stdlib.h>\n#include <string.h>\n{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error:?}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
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

/// The record of `cmt_opts.h`, its members spelled `MEMBER`.
fn opts_record(member: &str) -> String {
    format!(
        "typedef char *cfl_sds_t;\nvoid cfl_sds_destroy(cfl_sds_t s);\ncfl_sds_t cfl_sds_create(const char *str);\n\
         struct cmt_opts {{\n  {member} ns;\n  {member} subsystem;\n  {member} name;\n  {member} description;\n  {member} fqname;\n  int resource_index;\n}};\n"
    )
}

/// `cmt_opts_init`, shortened to its stores and early returns.
const OPTS_INIT: &str = "int cmt_opts_init(struct cmt_opts *opts, char *ns, char *name) {\n  if (!name) {\n    return -1;\n  }\n  if (ns) {\n    opts->ns = cfl_sds_create(ns);\n    if (!opts->ns) {\n      return -1;\n    }\n  }\n  opts->name = cfl_sds_create(name);\n  if (!opts->name) {\n    return -1;\n  }\n  return 0;\n}\n";

/// `cmt_opts_exit`, verbatim but for whitespace.
const OPTS_EXIT: &str = "void cmt_opts_exit(struct cmt_opts *opts) {\n  if (opts->ns) {\n    cfl_sds_destroy(opts->ns);\n  }\n  if (opts->subsystem) {\n    cfl_sds_destroy(opts->subsystem);\n  }\n  if (opts->name) {\n    cfl_sds_destroy(opts->name);\n  }\n  if (opts->description) {\n    cfl_sds_destroy(opts->description);\n  }\n  if (opts->fqname) {\n    cfl_sds_destroy(opts->fqname);\n  }\n}\n";

const MEMBERS: [&str; 5] = ["ns", "subsystem", "name", "description", "fqname"];

#[test]
fn the_deciding_premise_is_the_member_spelling_not_the_unit() {
    if !clang_available() {
        return;
    }
    let refused = MEMBERS
        .iter()
        .map(|member| named("opts", member))
        .collect::<Vec<_>>();
    // The pair as the unit holds it: refused, each release named by rule 38i.
    let (status, found) = leaves(
        "cmt_opts_exit",
        &format!("{}{OPTS_INIT}{OPTS_EXIT}", opts_record("cfl_sds_t")),
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{found:?}");
    assert_eq!(found, refused);
    // Alone: the same refusal, so the sibling is not the premise.
    let (status, found) = leaves(
        "cmt_opts_exit",
        &format!("{}{OPTS_EXIT}", opts_record("cfl_sds_t")),
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{found:?}");
    assert_eq!(found, refused);
    // The members spelled `char *`: Safe, alone and beside the initialiser.
    for unit in [
        format!("{}{OPTS_EXIT}", opts_record("char *")),
        format!("{}{OPTS_INIT}{OPTS_EXIT}", opts_record("char *")),
    ] {
        let (status, found) = leaves("cmt_opts_exit", &unit);
        assert_eq!(status, AnalysisStatus::Safe, "{found:?}");
    }
}
