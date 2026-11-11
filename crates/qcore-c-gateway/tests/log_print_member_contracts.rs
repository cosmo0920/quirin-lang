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

//! Card LOG-PRINT-BODY-CONTRACTS: the shipped declarations of `cmt_log_print`
//! (`qcore-provider-cmetrics`, `cmetrics/logging.qv`) and `cio_log_print`
//! (`qcore-provider-chunkio`, `chunkio/logging.qv`), read through rule 12n.
//!
//! Each states the five fixed parameters borrowed and the context's log
//! callback member invoked with `(ctx, level, file, line, _)`; neither states
//! a variadic tail, because the resource loader refuses one beside a callback
//! clause. The controls, over records shaped as the pinned headers declare
//! them:
//!
//! * a member the run closes empty (cmetrics: no store) invokes nothing, so a
//!   call with no tail actual is decided; without the run's facts it is
//!   refused;
//! * a call that passes an actual after `fmt` stays refused (the declaration
//!   is the refusing direction for a variadic C function);
//! * a member stored from a setter's `void *` formal (chunkio's
//!   `cio_set_log_callback`) is open, and the call is refused;
//! * an effectful candidate is refused, so the callback is never read benign;
//! * the loader refuses the tail beside the callback clause (the missing
//!   form VARIADIC-TAIL-BESIDE-CALLBACK-CLAUSES).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, read_unit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{RunMemberFacts, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

const CMETRICS: &str =
    include_str!("../../../providers/qcore-provider-cmetrics/contracts/cmetrics/logging.qv");
const CHUNKIO: &str =
    include_str!("../../../providers/qcore-provider-chunkio/contracts/chunkio/logging.qv");

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent log printer test: {reason}");
            false
        }
    }
}

/// `struct cmt` (`lib/cmetrics/include/cmetrics/cmetrics.h:52-54`) and
/// `struct cio_options` / `struct cio_ctx` (`lib/chunkio/include/chunkio/chunkio.h:70-90`),
/// reduced to the members the bodies read, with the pinned declarations of
/// the two printers and the chunkio setter.
const RECORDS: &str = "\
struct cmt { int log_level; void (*log_cb)(void *, int, const char *, int, const char *); };
struct cio_ctx;
struct cio_options { int initialized; int log_level; int (*log_cb)(struct cio_ctx *, int, const char *, int, char *); };
struct cio_ctx { int page_size; struct cio_options options; };
void cmt_log_print(void *ctx, int level, const char *file, int line, const char *fmt, ...);
void cio_log_print(void *ctx, int level, const char *file, int line, const char *fmt, ...);
void cio_set_log_callback(struct cio_ctx *ctx, void (*log_cb));
void *malloc(unsigned long);
void free(void *);
";

/// The callers: no tail actual, a tail actual, and a tail actual that is a
/// tracked allocation.
const CALLERS: &str = "\
int no_tail(struct cmt *cmt) { cmt_log_print(cmt, 1, \"f.c\", 3, \"no tail\"); return 0; }
int with_tail(struct cmt *cmt, int n) { cmt_log_print(cmt, 1, \"f.c\", 4, \"%d\", n); return 0; }
int cio_no_tail(struct cio_ctx *cio) { cio_log_print(cio, 1, \"f.c\", 5, \"no tail\"); return 0; }
";

/// The shipped modules, with an optional fixture module of candidates.
fn contracts(extra: Option<&str>) -> ResourceContracts {
    let mut sources = BTreeMap::from([
        ("cmetrics/logging.qv".to_owned(), CMETRICS.to_owned()),
        ("chunkio/logging.qv".to_owned(), CHUNKIO.to_owned()),
    ]);
    if let Some(extra) = extra {
        sources.insert("extra.qv".to_owned(), extra.to_owned());
    }
    load_resource_contract_sources(&sources).expect("the shipped log printer declarations load")
}

fn facts(body: &str) -> MemberFacts {
    let text = format!("{RECORDS}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

fn run(units: Vec<MemberFacts>) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(units, BTreeMap::new()))
}

fn row_of(
    name: &str,
    extra: Option<&str>,
    run: Option<Arc<RunMemberFacts>>,
) -> (AnalysisStatus, Vec<String>) {
    let contracts = contracts(extra);
    let text = format!("{RECORDS}{CALLERS}");
    let outcome = scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            name,
            &contracts,
        )
    })
    .expect("analysed");
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let row = &summary.functions[name];
    let leaves = row
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect();
    (row.status, leaves)
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

/// cmetrics: no unit stores into `cmt.log_cb`, so over the run's facts the
/// set is closed and empty and a call with no tail actual is the identity.
#[test]
fn a_member_no_unit_stores_invokes_nothing_over_the_run() {
    if !clang_available() {
        return;
    }
    let run = run(vec![facts(CALLERS)]);
    assert_eq!(run.closure("cmt.log_cb"), Ok(names(&[])));
    let (status, leaves) = row_of("no_tail", None, Some(run));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    // Without the run's facts the unit's own empty set is refused.
    let (status, leaves) = row_of("no_tail", None, None);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
    // An unread unit (the routes' 36 today) opens the member: refused.
    let unread = Arc::new(RunMemberFacts::new(
        vec![facts(CALLERS)],
        BTreeMap::from([("lj_vm.S".to_owned(), "its preprocessing failed".to_owned())]),
    ));
    let (status, leaves) = row_of("no_tail", None, Some(unread));
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A tail actual is not declared, so the call stays refused.
#[test]
fn a_tail_actual_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = run(vec![facts(CALLERS)]);
    let (status, leaves) = row_of("with_tail", None, Some(run));
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves.iter().any(|leaf| leaf
            == "arguments beyond the declared parameters of `cmt_log_print` are outside the resource projection"),
        "{leaves:?}"
    );
}

/// A candidate that releases the context it is handed is refused: the
/// callback is never read as benign.
#[test]
fn an_effectful_candidate_is_refused() {
    if !clang_available() {
        return;
    }
    let releasing = "module contract;

extern fn freeing_log(ctx: ptr<mut, u8>, level: i32, file: ptr<const, u8>, line: i32, msg: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(ctx, nonnull);
    ensures normal resource_borrowed(level);
    ensures normal resource_borrowed(file);
    ensures normal resource_borrowed(line);
    ensures normal resource_borrowed(msg);
    ensures normal resource_return_none();
};
";
    let registrar = facts(
        "void freeing_log(void *, int, const char *, int, const char *);\n\
         void reg(struct cmt *c) { c->log_cb = freeing_log; }\n",
    );
    let run = run(vec![registrar, facts(CALLERS)]);
    assert_eq!(run.closure("cmt.log_cb"), Ok(names(&["freeing_log"])));
    let (status, leaves) = row_of("no_tail", Some(releasing), Some(run));
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-effectful")),
        "{leaves:?}"
    );
}

/// chunkio: the setter stores its `void *` formal into the member, which
/// rule 12n opens, so the invocation is refused.
///
/// The repair this card makes: `void (*log_cb)` is spelled `void (*)` by
/// Clang (the declarator's parentheses kept as sugar), and was read as a
/// function-pointer formal, so with no call of the setter recording a
/// function-pointer actual the member closed EMPTY (`Ok({})`, measured
/// before the repair) and a call would have read the callback as never
/// invoked. A `void *` caller (`cio_set_log_callback(ctx, any)`) contributes
/// nothing the closure sees, so the formal must open the member.
#[test]
fn a_member_stored_from_a_void_pointer_formal_is_open() {
    if !clang_available() {
        return;
    }
    let setter = facts(
        "void cio_set_log_callback(struct cio_ctx *ctx, void (*log_cb)) { ctx->options.log_cb = log_cb; }\n\
         void *any;\n\
         void reg(struct cio_ctx *ctx) { cio_set_log_callback(ctx, any); }\n",
    );
    assert!(
        setter.stores["cio_options.log_cb"]
            .iter()
            .all(|flow| format!("{flow:?}").contains("a formal that is not a function pointer")),
        "{:?}",
        setter.stores
    );
    // A formal that IS a function pointer still forwards its callers'
    // actuals: the repair opens nothing else.
    let typed = facts(
        "int quiet(struct cio_ctx *, int, const char *, int, char *);\n\
         void set(struct cio_ctx *ctx, int (*cb)(struct cio_ctx *, int, const char *, int, char *)) { ctx->options.log_cb = cb; }\n\
         void reg(struct cio_ctx *ctx) { set(ctx, quiet); }\n",
    );
    assert_eq!(
        run(vec![typed, facts(CALLERS)]).closure("cio_options.log_cb"),
        Ok(names(&["quiet"]))
    );
    let run = run(vec![setter, facts(CALLERS)]);
    let closure = run.closure("cio_options.log_cb");
    assert!(closure.is_err(), "{closure:?}");
    let (status, leaves) = row_of("cio_no_tail", None, Some(run));
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
}

/// The missing form: the loader refuses the truthful tail beside the
/// callback clause, so the shipped declarations state no tail.
#[test]
fn the_loader_refuses_a_variadic_tail_beside_a_callback_clause() {
    let with_tail = CMETRICS.replace(
        "    ensures normal resource_callback_member_invoked(",
        "    ensures normal resource_variadic_borrowed_tail();\n    ensures normal resource_callback_member_invoked(",
    );
    assert_ne!(with_tail, CMETRICS);
    let error = load_resource_contract_sources(&BTreeMap::from([(
        "cmetrics/logging.qv".to_owned(),
        with_tail,
    )]))
    .expect_err("a variadic tail beside a callback clause is refused");
    assert!(
        error.contains(
            "a retention clause stands only beside plain parameter and result predicates"
        ),
        "{error}"
    );
}
