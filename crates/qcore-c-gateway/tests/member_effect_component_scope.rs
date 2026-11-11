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
//! sibling `cmt_opts_init` play no part: on the base the same body with
//! `char *` members is `Safe`, alone or beside its initialiser, and the
//! typedef-spelled body is refused alone as beside it.
//!
//! The amendment (rule 38j): where the lexical recogniser refuses such an
//! actual as an untyped member, at a nullable release of a place rule 5k or
//! rule 38j admitted, Clang's reading of it as a pointer member's value stands
//! in for the lexical one. Every other premise is unchanged, so a caller that
//! uses or releases what the callee released, a recursive pair, an array or
//! function-pointer member all keep their refusal. (A typedef-spelled base,
//! refused by this card, is admitted since card MEMBER-RELEASE-UNTYPED-BASE.)

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

fn assert_safe(entry: &str, unit: &str) {
    let (status, found) = leaves(entry, unit);
    assert_eq!(status, AnalysisStatus::Safe, "{entry}: {found:?}");
}

fn assert_not_safe(entry: &str, unit: &str) -> Vec<String> {
    let (status, found) = leaves(entry, unit);
    assert_ne!(status, AnalysisStatus::Safe, "{entry}: {found:?}");
    found
}

fn refused_call(callee: &str) -> String {
    format!(
        "call to `{callee}`, whose summary releases members of an argument, outside the \
         caller's member-effect premises (member-effect-call-refused)"
    )
}

/// The `cmt_opts.c` pair: the exit is `Safe` whatever spells its members, alone
/// or beside the initialiser, which keeps its own store refusals.
#[test]
fn the_cmt_opts_pair_is_admitted_whatever_spells_the_members() {
    if !clang_available() {
        return;
    }
    for member in ["cfl_sds_t", "char *"] {
        assert_safe(
            "cmt_opts_exit",
            &format!("{}{OPTS_EXIT}", opts_record(member)),
        );
        let pair = format!("{}{OPTS_INIT}{OPTS_EXIT}", opts_record(member));
        assert_safe("cmt_opts_exit", &pair);
        let found = assert_not_safe("cmt_opts_init", &pair);
        assert!(
            found
                .iter()
                .any(|d| d.contains("member-store-through-formal")),
            "{found:?}"
        );
    }
    // Rule 5k's shape over the same members: the parent released afterwards.
    assert_safe(
        "opts_destroy",
        &format!(
            "{}void opts_destroy(struct cmt_opts *opts) {{\n  cfl_sds_destroy(opts->ns);\n  free(opts);\n}}\n",
            opts_record("cfl_sds_t")
        ),
    );
    // `cmt_counter_destroy`'s shape in the same unit: the embedded options,
    // then the parent.
    assert_safe(
        "counter_destroy",
        &format!(
            "{}{OPTS_EXIT}struct counter {{ struct cmt_opts opts; int n; }};\nvoid counter_destroy(struct counter *c) {{\n  cmt_opts_exit(&c->opts);\n  free(c);\n}}\n",
            opts_record("cfl_sds_t")
        ),
    );
}

/// Every other premise of rules 5k and 38j is unchanged: a function of the
/// same unit that uses or releases again what the exit released stays refused.
#[test]
fn a_member_one_function_releases_and_another_still_uses_stays_refused() {
    if !clang_available() {
        return;
    }
    let unit = |caller: &str| format!("{}{OPTS_EXIT}{caller}", opts_record("cfl_sds_t"));
    // A read of a released member after the call.
    let found = assert_not_safe(
        "reuse",
        &unit(
            "int reuse(struct cmt_opts *opts) {\n  cmt_opts_exit(opts);\n  return opts->ns != 0;\n}\n",
        ),
    );
    assert!(found.contains(&refused_call("cmt_opts_exit")), "{found:?}");
    // A second release of one.
    let found = assert_not_safe(
        "again",
        &unit(
            "void again(struct cmt_opts *opts) {\n  cmt_opts_exit(opts);\n  cfl_sds_destroy(opts->name);\n}\n",
        ),
    );
    assert!(found.contains(&refused_call("cmt_opts_exit")), "{found:?}");
    // Inside the body: a use after the release, and a second release.
    assert_not_safe(
        "used_after",
        &format!(
            "{}void used_after(struct cmt_opts *opts) {{\n  cfl_sds_destroy(opts->ns);\n  opts->fqname = opts->ns;\n}}\n",
            opts_record("cfl_sds_t")
        ),
    );
    let found = assert_not_safe(
        "twice",
        &format!(
            "{}void twice(struct cmt_opts *opts) {{\n  cfl_sds_destroy(opts->ns);\n  cfl_sds_destroy(opts->ns);\n}}\n",
            opts_record("cfl_sds_t")
        ),
    );
    assert_eq!(found, [named("opts", "ns"), named("opts", "ns")]);
}

/// A recursive pair publishes no member effect (rule 38j: singleton components
/// only), so neither body admits the other's call and both stay refused.
#[test]
fn a_recursive_pair_stays_refused() {
    if !clang_available() {
        return;
    }
    let pair = format!(
        "{}void exit_b(struct cmt_opts *opts, int n);\n\
         void exit_a(struct cmt_opts *opts, int n) {{\n  if (opts->ns) {{\n    cfl_sds_destroy(opts->ns);\n  }}\n  if (n > 0) {{\n    exit_b(opts, n - 1);\n  }}\n}}\n\
         void exit_b(struct cmt_opts *opts, int n) {{\n  if (opts->name) {{\n    cfl_sds_destroy(opts->name);\n  }}\n  if (n > 0) {{\n    exit_a(opts, n - 1);\n  }}\n}}\n",
        opts_record("cfl_sds_t")
    );
    let found = assert_not_safe("exit_a", &pair);
    assert!(found.contains(&named("opts", "ns")), "{found:?}");
    let found = assert_not_safe("exit_b", &pair);
    assert!(found.contains(&named("opts", "name")), "{found:?}");
    // Self-recursion.
    assert_not_safe(
        "exit_self",
        &format!(
            "{}void exit_self(struct cmt_opts *opts, int n) {{\n  if (opts->ns) {{\n    cfl_sds_destroy(opts->ns);\n  }}\n  if (n > 0) {{\n    exit_self(opts, n - 1);\n  }}\n}}\n",
            opts_record("cfl_sds_t")
        ),
    );
}

/// Clang's reading stands in only for the value of an object-pointer member
/// of a parameter's record: an array member (an interior address), a
/// function-pointer typedef and a `union` keep their refusal; a typedef-spelled
/// base is admitted since card MEMBER-RELEASE-UNTYPED-BASE.
#[test]
fn the_typed_reading_admits_only_a_pointer_members_value() {
    if !clang_available() {
        return;
    }
    let prelude =
        "typedef char *sds_t;\ntypedef void (*hook_t)(void);\nvoid cfl_sds_destroy(sds_t s);\n";
    // Clang reads `r->buf` as `ArrayToPointerDecay`, an address, not a value.
    let found = assert_not_safe(
        "array_member",
        &format!(
            "{prelude}typedef char name_t[16];\nstruct r {{ name_t buf; }};\nvoid array_member(struct r *r) {{\n  free(r->buf);\n}}\n"
        ),
    );
    assert_eq!(found, [named("r", "buf")]);
    assert_not_safe(
        "hook_member",
        &format!(
            "{prelude}struct r {{ hook_t cb; }};\nvoid hook_member(struct r *r) {{\n  free(r->cb);\n}}\n"
        ),
    );
    // The base arm of W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS: refused by
    // this card, admitted since card MEMBER-RELEASE-UNTYPED-BASE (rule 38j
    // amended again; `tests/member_release_untyped_base.rs`), flipped
    // deliberately.
    assert_safe(
        "typedef_base",
        &format!(
            "{prelude}typedef struct r {{ sds_t s; }} r_t;\nvoid typedef_base(r_t *r) {{\n  cfl_sds_destroy(r->s);\n}}\n"
        ),
    );
    // A union's member is never read here (rule 10w's ground).
    assert_not_safe(
        "union_member",
        &format!(
            "{prelude}union u {{ sds_t s; int n; }};\nvoid union_member(union u *u) {{\n  cfl_sds_destroy(u->s);\n}}\n"
        ),
    );
    // The same body with the member `sds_t`: admitted.
    assert_safe(
        "pointer_member",
        &format!(
            "{prelude}struct r {{ sds_t s; int n; }};\nvoid pointer_member(struct r *r) {{\n  if (r->s) {{\n    cfl_sds_destroy(r->s);\n  }}\n}}\n"
        ),
    );
}
