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
//!
//! Step 2, gateway rule 38j: the member-release half of the member-effect
//! summary. A body that releases members of a formal it keeps is `Safe`
//! under rule 5k's P1-P3 and states its effect; its summary is withheld from
//! the ordinary table, and a direct call is lowered under it only where the
//! caller's own check reads the call as the release of each member.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
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

const RECORD: &str = "struct inner { int x; };\nstruct o { char *a; char *b; struct inner in; struct o *next; int n; };\nvoid obj_destroy(void *obj);\nvoid list_destroy(void *list);\nvoid *obj_create(__SIZE_TYPE__ size);\nvoid uncontracted(void *p);\nstruct o *other(void);\n";

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
    // Twice the same member: step 2's body check refuses it, step 1 names it.
    assert_leaves(
        "twice",
        "void twice(struct o *opts) {\n  free(opts->a);\n  free(opts->a);\n}",
        &[named("opts", "a"), named("opts", "a")],
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
    // `map->unit = cfl_sds_create(unit)`: an owned result. The UCRT's
    // `<stdlib.h>` spells `malloc` with Microsoft declaration specifiers the
    // declared-ABI reader does not read, so on Windows the libc contract is
    // withdrawn against it and the call stays `Unknown` by that name; `setnew2`
    // below carries the member-store refusal on every target (its declaration
    // spells the parameter `__SIZE_TYPE__`, as the UCRT's `size_t` typedef is
    // also unread there).
    assert_leaves(
        "setnew",
        "void setnew(struct o *c) {\n  c->a = malloc(4);\n}",
        &[if cfg!(windows) {
            "call to `malloc` whose declared contract was withdrawn against its C declaration \
             in this translation unit (abi-unreadable-declaration)"
                .to_owned()
        } else {
            stored("c", "a")
        }],
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

const CLEAR1: &str = "void clear1(struct o *opts) {\n  free(opts->a);\n}\n";

fn refused_call(callee: &str) -> String {
    format!(
        "call to `{callee}`, whose summary releases members of an argument, outside the \
         caller's member-effect premises (member-effect-call-refused)"
    )
}

fn assert_refused(entry: &str, body: &str, callee: &str) {
    let (status, descriptions) = leaves(entry, body);
    assert_eq!(status, AnalysisStatus::Unknown, "{entry}: {descriptions:?}");
    assert!(
        descriptions.contains(&refused_call(callee)),
        "{entry}: {descriptions:?}"
    );
}

#[test]
fn a_body_releasing_members_of_a_kept_formal_is_safe() {
    if !clang_available() {
        return;
    }
    // `cmt_opts_exit`: each member under its own null test, the parent kept.
    assert_safe(
        "opts_exit",
        "void opts_exit(struct o *opts) {\n  if (opts->a) {\n    free(opts->a);\n  }\n  if (opts->b) {\n    free(opts->b);\n  }\n}",
    );
    // `metric_release_storage`: releases, then null stores.
    assert_safe(
        "release_storage",
        "void release_storage(struct o *m) {\n  free(m->a);\n  free(m->b);\n  m->a = NULL;\n  m->b = NULL;\n}",
    );
    // `cmt_metric_exp_hist_snapshot_destroy`: a null test of the base first.
    assert_safe(
        "snapshot_destroy",
        "void snapshot_destroy(struct o *s) {\n  if (s == NULL) {\n    return;\n  }\n  if (s->a != NULL) {\n    free(s->a);\n    s->a = NULL;\n  }\n}",
    );
}

#[test]
fn the_body_check_keeps_every_other_member_use_refused() {
    if !clang_available() {
        return;
    }
    // A use of the place after its release.
    assert_not_safe(
        "after",
        "void after(struct o *opts) {\n  free(opts->a);\n  opts->b = opts->a;\n}",
    );
    // A release in a loop.
    assert_not_safe(
        "in_loop",
        "void in_loop(struct o *opts, int n) {\n  int i;\n  for (i = 0; i < n; i++)\n    free(opts->a);\n}",
    );
    // The base copied.
    assert_not_safe(
        "copied",
        "void copied(struct o *opts) {\n  struct o *c = opts;\n  free(opts->a);\n  c->n = 1;\n}",
    );
    // A non-null release: nothing establishes an untracked member non-null.
    assert_leaves(
        "nonnull",
        "void nonnull(struct o *opts) {\n  obj_destroy(opts->a);\n}",
        &[named("opts", "a")],
    );
}

fn assert_not_safe(entry: &str, body: &str) {
    let (status, descriptions) = leaves(entry, body);
    assert_ne!(status, AnalysisStatus::Safe, "{entry}: {descriptions:?}");
}

#[test]
fn a_caller_that_releases_the_parent_afterwards_reads_the_effect() {
    if !clang_available() {
        return;
    }
    // `cmt_counter_destroy`'s shape: the helper, then the parent's release.
    assert_safe(
        "destroy",
        &format!("{CLEAR1}void destroy(struct o *x) {{\n  clear1(x);\n  free(x);\n}}"),
    );
    // The embedded options object of `cmt_counter_destroy`.
    assert_safe(
        "embedded",
        &format!(
            "struct w {{ struct o in; char *c; }};\n{CLEAR1}void embedded(struct w *w) {{\n  clear1(&w->in);\n  free(w);\n}}"
        ),
    );
    // A non-null release of the parent afterwards.
    assert_safe(
        "then_destroy",
        &format!("{CLEAR1}void then_destroy(struct o *x) {{\n  clear1(x);\n  obj_destroy(x);\n}}"),
    );
}

#[test]
fn a_caller_that_keeps_the_parent_states_the_effect_in_turn() {
    if !clang_available() {
        return;
    }
    let outer = format!("{CLEAR1}void outer(struct o *x) {{\n  clear1(x);\n}}\n");
    assert_safe("outer", &outer);
    assert_safe(
        "top",
        &format!("{outer}void top(struct o *y) {{\n  outer(y);\n  free(y);\n}}"),
    );
}

#[test]
fn a_released_member_is_never_used_or_released_again_by_a_caller() {
    if !clang_available() {
        return;
    }
    // A second release of the member the callee released.
    assert_refused(
        "again",
        &format!("{CLEAR1}void again(struct o *x) {{\n  clear1(x);\n  free(x->a);\n  free(x);\n}}"),
        "clear1",
    );
    // A read of it afterwards.
    assert_refused(
        "read_after",
        &format!("{CLEAR1}void read_after(struct o *x) {{\n  clear1(x);\n  x->b = x->a;\n}}"),
        "clear1",
    );
    // Through the cascade.
    assert_refused(
        "cascade_again",
        &format!(
            "{CLEAR1}void outer(struct o *x) {{\n  clear1(x);\n}}\nvoid cascade_again(struct o *y) {{\n  outer(y);\n  free(y->a);\n  free(y);\n}}"
        ),
        "outer",
    );
    // A member of the embedded sub-object.
    assert_refused(
        "embedded_again",
        &format!(
            "struct w {{ struct o in; char *c; }};\n{CLEAR1}void embedded_again(struct w *w) {{\n  clear1(&w->in);\n  free(w->in.a);\n  free(w);\n}}"
        ),
        "clear1",
    );
    // The same object at two positions of one call.
    assert_refused(
        "aliased",
        "void pair(struct o *p, struct o *q) {\n  free(p->a);\n  free(q->a);\n}\nvoid aliased(struct o *x) {\n  pair(x, x);\n  free(x);\n}",
        "pair",
    );
    // A callee that touches a member this body released before the call.
    assert_refused(
        "touched_after",
        "void clear_a(struct o *p) {\n  free(p->a);\n  p->b = NULL;\n}\nvoid touched_after(struct o *x) {\n  free(x->b);\n  clear_a(x);\n}",
        "clear_a",
    );
}

#[test]
fn every_other_reader_of_an_effect_function_sees_no_contract() {
    if !clang_available() {
        return;
    }
    // A local (tracked) parent: no formal, no admission.
    assert_refused(
        "local_parent",
        &format!(
            "{CLEAR1}void local_parent(void) {{\n  struct o *x = calloc(1, sizeof *x);\n  if (!x) return;\n  clear1(x);\n  free(x);\n}}"
        ),
        "clear1",
    );
    // A call through a pointer, and a call in a return.
    assert_not_safe(
        "through_pointer",
        &format!(
            "{CLEAR1}void through_pointer(struct o *x) {{\n  void (*fp)(struct o *) = clear1;\n  fp(x);\n  free(x);\n}}"
        ),
    );
    assert_not_safe(
        "in_return",
        "int clear2(struct o *opts) {\n  free(opts->a);\n  return 0;\n}\nint in_return(struct o *x) {\n  return clear2(x);\n}",
    );
}
