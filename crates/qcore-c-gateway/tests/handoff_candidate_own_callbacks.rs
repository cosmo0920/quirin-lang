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

//! Card HANDOFF-CANDIDATE-OWN-CALLBACKS (gateway rule 13u amended, with 12m
//! and 12n): a callback candidate whose own declaration carries member
//! invocations is decided at the invocation as the composition of its own
//! clauses, each own invocation over the run's set, by least fixed point;
//! and rule 12m's caller branch check follows an exact copy of a member into
//! a variable before its call.
//!
//! The brief's six cases: a decided own invocation (contracted), an
//! undecided one (refused), a self-cycle and a two-candidate cycle (refused),
//! an exact copy (admitted) and an inexact one (refused).

use std::collections::BTreeMap;
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, read_unit};
use qcore_c_gateway::handoff_branch::{Handed, Invocation, Query, UnitBodies, admit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{RunMemberFacts, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::handoff::{GuardOp, HandOffGuard};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent own-callback test: {reason}");
            false
        }
    }
}

/// An op whose callback member `op_cb` a destroyer invokes, and a handle
/// whose embedded conf holds a refresh callback (as `rd_kafka_op_destroy`,
/// `rd_kafka_oauthbearer_refresh_op` and `rk_conf.sasl.oauthbearer`).
const RECORDS: &str = "\
struct handle_s;
struct op_s;
typedef void refresh_fn(struct handle_s *);
typedef int op_fn(struct handle_s *, struct op_s *);
struct conf_s { refresh_fn *refresh_cb; };
struct handle_s { int kind; struct conf_s conf; };
struct op_s { op_fn *op_cb; op_fn *aux_cb; int type; };
struct op_s *op_new(void);
void destroy(struct op_s *op);
void quiet_refresh(struct handle_s *);
void freeing_refresh(struct handle_s *);
int refresh_op(struct handle_s *, struct op_s *);
int loop_op(struct handle_s *, struct op_s *);
int a_op(struct handle_s *, struct op_s *);
int b_op(struct handle_s *, struct op_s *);
int plain_op(struct handle_s *, struct op_s *);
";

const CONTRACTS: &str = "module contract;

extern fn op_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};

extern fn destroy(op: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(op, nonnull);
    ensures normal resource_callback_member_invoked(op, op_s.op_cb, _, op);
    ensures normal resource_return_none();
};

extern fn quiet_refresh(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_return_none();
};

extern fn freeing_refresh(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(rk, nonnull);
    ensures normal resource_return_none();
};

extern fn plain_op(rk: ptr<mut, u8>, op: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(op);
    ensures normal resource_return_none();
};

extern fn refresh_op(rk: ptr<mut, u8>, op: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(op);
    ensures normal resource_callback_member_invoked(rk, handle_s.conf.refresh_cb, rk);
    ensures normal resource_return_none();
};

extern fn loop_op(rk: ptr<mut, u8>, op: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(op);
    ensures normal resource_callback_member_invoked(op, op_s.op_cb, _, op);
    ensures normal resource_return_none();
};

extern fn a_op(rk: ptr<mut, u8>, op: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(op);
    ensures normal resource_callback_member_invoked(op, op_s.aux_cb, _, op);
    ensures normal resource_return_none();
};

extern fn b_op(rk: ptr<mut, u8>, op: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(op);
    ensures normal resource_callback_member_invoked(op, op_s.op_cb, _, op);
    ensures normal resource_return_none();
};
";

const CALLER: &str = "void use(void) { struct op_s *o = op_new(); destroy(o); }\n";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "own_callbacks.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load")
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

fn run_of(stores: &str) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(
        vec![facts(stores), facts(CALLER)],
        BTreeMap::new(),
    ))
}

fn row_of(run: Option<Arc<RunMemberFacts>>) -> (AnalysisStatus, Vec<String>) {
    let contracts = contracts();
    let text = format!("{RECORDS}{CALLER}");
    let outcome = scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            "use",
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
    let row = &summary.functions["use"];
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

/// A candidate with no own invocation, as before the card: decided.
#[test]
fn a_plain_candidate_decides_as_before() {
    if !clang_available() {
        return;
    }
    let (status, leaves) = row_of(Some(run_of(
        "void reg(struct op_s *o) { o->op_cb = plain_op; }\n",
    )));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A candidate whose own invocation decides over the run's set is
/// contracted: `destroy` decides, and the caller is `Safe`.
#[test]
fn a_candidate_with_a_decided_own_invocation_is_contracted() {
    if !clang_available() {
        return;
    }
    let (status, leaves) = row_of(Some(run_of(
        "void reg(struct handle_s *h, struct op_s *o) {\n\
           h->conf.refresh_cb = quiet_refresh; o->op_cb = refresh_op; }\n",
    )));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    // A run-closed empty set invokes nothing (E1): still contracted.
    let (status, leaves) = row_of(Some(run_of(
        "void reg(struct op_s *o) { o->op_cb = refresh_op; }\n",
    )));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// One own invocation that does not decide refuses the candidate, naming
/// the invocation and its inner refusal: an effectful inner candidate, an
/// open set, and no run facts at all (a unit's own set is never used).
#[test]
fn a_candidate_with_an_undecided_own_invocation_refuses() {
    if !clang_available() {
        return;
    }
    let (status, leaves) = row_of(Some(run_of(
        "void reg(struct handle_s *h, struct op_s *o) {\n\
           h->conf.refresh_cb = freeing_refresh; o->op_cb = refresh_op; }\n",
    )));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-own-invocation-refused")
                && leaf.contains("refresh_op")
                && leaf.contains("callback-effectful")),
        "{leaves:?}"
    );
    let (status, leaves) = row_of(Some(run_of(
        "refresh_fn *chosen;\n\
         void reg(struct handle_s *h, struct op_s *o) {\n\
           h->conf.refresh_cb = chosen; o->op_cb = refresh_op; }\n",
    )));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-own-invocation-refused")
                && leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
    let (status, leaves) = row_of(None);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A self-cycle: `loop_op` invokes `op_cb`, whose run set is `{loop_op}`.
#[test]
fn a_self_cycle_refuses() {
    if !clang_available() {
        return;
    }
    let (status, leaves) = row_of(Some(run_of(
        "void reg(struct op_s *o) { o->op_cb = loop_op; }\n",
    )));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-own-invocation-refused")
                && leaf.contains("loop_op -> loop_op")
                && leaf.contains("a cycle never supports itself")),
        "{leaves:?}"
    );
}

/// A two-candidate cycle: `a_op` invokes `aux_cb` (`{b_op}`), `b_op` invokes
/// `op_cb` (`{a_op}`).
#[test]
fn a_two_candidate_cycle_refuses() {
    if !clang_available() {
        return;
    }
    let (status, leaves) = row_of(Some(run_of(
        "void reg(struct op_s *o) { o->op_cb = a_op; o->aux_cb = b_op; }\n",
    )));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-own-invocation-refused")
                && leaf.contains("a_op -> b_op -> a_op")),
        "{leaves:?}"
    );
}

/// A served op's own callback member, called through a variable (as
/// `rd_kafka_op_handle` calls `rko_serve`).
const SERVE: &str = "\
typedef enum { RES_PASS, RES_HANDLED, RES_KEEP } res_t;
struct sop_s;
typedef res_t serve_fn(struct sop_s *);
struct sop_s { serve_fn *serve; serve_fn *other; };
struct sop_s *sop_new(void);
void sop_destroy(struct sop_s *);
";

/// The caller branch check for a `!= RES_PASS` hand-off of the op at the
/// `serve` member's invocation, from `loop`, whose body destroys the op only
/// on `RES_PASS`.
fn admitted(handle: &str) -> Result<(), String> {
    let text = format!(
        "{SERVE}{handle}\n\
         void loop(void) {{ struct sop_s *op = sop_new(); res_t r = handle(op, 0);\n\
           if (r == RES_PASS) sop_destroy(op); }}\n"
    );
    let unit = read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"));
    let guard = HandOffGuard {
        op: GuardOp::Ne,
        constant: "RES_PASS".into(),
    };
    admit(
        &UnitBodies(vec![&unit]),
        &Query {
            invoker: "loop",
            invocation: Invocation::Member {
                object: None,
                path: "serve".into(),
            },
            position: 0,
            handed: Handed::Own,
            guard: &guard,
            value: 0,
        },
    )
}

/// An exact copy is followed: a formal assigned once from the member (as
/// `rd_kafka_op_handle`), and a local initialized from it.
#[test]
fn an_exact_copy_is_followed() {
    if !clang_available() {
        return;
    }
    let formal = "res_t handle(struct sop_s *op, serve_fn *callback) {\n\
                    if (op->serve) { callback = op->serve; op->serve = 0; }\n\
                    return callback(op); }";
    assert_eq!(admitted(formal), Ok(()));
    let local = "res_t handle(struct sop_s *op, serve_fn *unused) {\n\
                   serve_fn *cb = op->serve; (void)unused; return cb(op); }";
    assert_eq!(admitted(local), Ok(()));
}

/// Any other flow is not followed, and the check refuses: a written cast, a
/// second write, another member, a variable whose address is taken, an
/// object that is reassigned.
#[test]
fn an_inexact_copy_refuses() {
    if !clang_available() {
        return;
    }
    let cases = [
        "res_t handle(struct sop_s *op, serve_fn *callback) {\n\
           callback = (serve_fn *)op->serve; return callback(op); }",
        "res_t handle(struct sop_s *op, serve_fn *callback) {\n\
           callback = op->serve; if (!callback) callback = op->other; return callback(op); }",
        "res_t handle(struct sop_s *op, serve_fn *callback) {\n\
           callback = op->other; return callback(op); }",
        "void keep(serve_fn **);\n\
         res_t handle(struct sop_s *op, serve_fn *callback) {\n\
           callback = op->serve; keep(&callback); return callback(op); }",
        "res_t handle(struct sop_s *op, serve_fn *callback) {\n\
           callback = op->serve; op = sop_new(); return callback(op); }",
    ];
    for case in cases {
        let refused = admitted(case);
        assert!(refused.is_err(), "{case}");
    }
    // Every refusal names why; the two-write case reads no invocation.
    let refused = admitted(cases[1]).expect_err("refused");
    assert!(refused.contains("no invocation"), "{refused}");
}

/// A row resting on a decided invocation lists every declaration the
/// decision read: the invoking declaration, its candidates (not spelled in
/// the body), and a composed candidate's inner candidates.
#[test]
fn a_row_resting_on_a_composed_candidate_lists_what_it_read() {
    use qcore_c_gateway::contract_dependencies::{Recording, row_dependencies};
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    let text = format!("{RECORDS}{CALLER}");
    let run = run_of(
        "void reg(struct handle_s *h, struct op_s *o) {\n\
           h->conf.refresh_cb = quiet_refresh; o->op_cb = refresh_op; }\n",
    );
    let recording = Recording::start();
    let outcome = scoped(Some(run), || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            "use",
            &contracts,
        )
    })
    .expect("analysed");
    let record = recording.finish();
    let UnrestrictedLegacyResult::Safe(certificate) = &outcome else {
        panic!("not Safe");
    };
    let dependencies = row_dependencies(&record, &certificate.summary, &contracts);
    let row = &dependencies["use"];
    for name in ["op_new", "destroy", "refresh_op", "quiet_refresh"] {
        assert!(row.declarations.contains(name), "{name}: {row:?}");
    }
}
