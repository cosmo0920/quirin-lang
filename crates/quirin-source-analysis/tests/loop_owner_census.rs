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

//! W47-LOOP-OWNER-CONTRACTS — the thread-entry census and the loop-owner
//! obligation over fixtures built from the bound shapes of
//! `docs/reports/w47-tail-pause-bindings/bindings.json` (negative controls
//! first), then the production contract module through the gateway's
//! per-unit route, where the obligation is never discharged.
//!
//! The fixtures are a miniature of the in_tail runtime test binary: the
//! engine thread E is spawned by `do_start`, the input worker I by the thread
//! pool, collectors are dispatched on each loop, the public pause forks on the
//! threaded flag and the plugin callbacks sit in a designated-initializer
//! vtable. The two test files are the previous (`90978ac6~1`) and the fixed
//! (`90978ac6`) non-threaded test, each beside the unchanged threaded test.

use std::collections::BTreeMap;

use qcore_c_gateway::thread_entry_census::CensusFile;
use qcore_contract::contract_loader::{
    LoopOwnerOperation, load_resource_contract_sources, loop_owner_obligations,
};
use qcore_core::loop_owner::{
    CollectorResult, ControlClaim, Ingestion, Mode, OwnerVerdict, PairDirection, PairingClaim,
    claim_discharged, control_completion, pairing,
};
use quirin_source_analysis::loop_owner_census::{FLUENT_BIT_HOPS, OwnerSite, census};

const CONTRACTS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
);

const LIBRARY: &str = r#"
struct flb_input_plugin;
struct flb_input_instance { struct flb_input_plugin *p; void *context; void *data; int threaded; };
struct flb_input_collector { int (*cb_collect)(struct flb_input_instance *, void *, void *); struct flb_input_instance *instance; };
static void flb_lib_worker(void *data)
{
    flb_engine_start(data);
}
static int do_start(void *ctx)
{
    int tid;
    return mk_utils_worker_spawn(flb_lib_worker, ctx, &tid);
}
int flb_start(void *ctx)
{
    return do_start(ctx);
}
int flb_input_is_threaded(struct flb_input_instance *ins)
{
    return ins->threaded;
}
int flb_input_collector_fd(int fd, void *config)
{
    struct flb_input_collector *collector = 0;
    collector->cb_collect(collector->instance, config, collector->instance->context);
    return 0;
}
static int flb_engine_handle_event(int fd, void *config)
{
    return flb_input_collector_fd(fd, config);
}
int flb_input_pre_run_all(struct flb_input_instance *ins, void *config)
{
    if (!flb_input_is_threaded(ins)) {
        ins->p->cb_pre_run(ins, config, ins->context);
    }
    return 0;
}
static void input_thread(void *data)
{
    input_collector_fd(0, data);
    handle_input_event(data);
}
int flb_input_thread_instance_init(struct flb_input_instance *ins, void *config)
{
    return flb_tp_thread_create(0, input_thread, ins, config);
}
int flb_input_instance_init(struct flb_input_instance *ins, void *config)
{
    if (flb_input_is_threaded(ins)) {
        return flb_input_thread_instance_init(ins, config);
    }
    return 0;
}
int flb_engine_start(void *config)
{
    struct flb_input_instance *ins = 0;
    flb_input_instance_init(ins, config);
    flb_input_pre_run_all(ins, config);
    while (1) {
        flb_engine_handle_event(0, config);
    }
    return 0;
}
int input_collector_fd(int fd, struct flb_input_instance *ins)
{
    struct flb_input_collector *collector = 0;
    collector->cb_collect(collector->instance, 0, collector->instance->context);
    return 0;
}
int flb_input_plugin_pause(struct flb_input_instance *ins)
{
    if (ins->p->cb_pause != 0) {
        ins->p->cb_pause(ins->context, 0);
    }
    return 0;
}
int handle_input_event(struct flb_input_instance *ins)
{
    return flb_input_plugin_pause(ins);
}
int flb_input_thread_instance_pause(struct flb_input_instance *ins)
{
    return 0;
}
int flb_input_pause(struct flb_input_instance *ins)
{
    if (flb_input_is_threaded(ins)) {
        return flb_input_thread_instance_pause(ins);
    }
    else {
        return flb_input_plugin_pause(ins);
    }
}
int flb_input_set_collector_time(struct flb_input_instance *ins, void *cb, long s, long ns, void *config)
{
    return 0;
}
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_running(int coll_id, struct flb_input_instance *in) { return 0; }
"#;

const PLUGIN: &str = r#"
struct flb_tail_config { int coll_fd_static; int coll_fd_fs1; struct flb_input_instance *ins; };
static void flb_tail_fs_inotify_pause(struct flb_tail_config *ctx)
{
    flb_input_collector_pause(ctx->coll_fd_static, ctx->ins);
    flb_input_collector_pause(ctx->coll_fd_fs1, ctx->ins);
}
static void in_tail_pause(void *data, void *config)
{
    flb_tail_fs_inotify_pause(data);
}
static int in_tail_pre_run(struct flb_input_instance *ins, void *config, void *data)
{
    return 0;
}
struct flb_input_plugin in_tail_plugin = {
    .cb_pre_run = in_tail_pre_run,
    .cb_pause = in_tail_pause,
};
"#;

/// The unchanged threaded test (NC3), present at both revisions.
const THREADED_TEST: &str = r#"
static int wait_tail_collectors_state(struct flb_tail_config *tail_ctx, struct flb_input_instance *ins)
{
    return flb_input_collector_running(tail_ctx->coll_fd_fs1, ins);
}
void flb_test_inotify_threaded_pause_collectors()
{
    struct flb_input_instance *ins = 0;
    flb_start(0);
    wait_tail_collectors_state(ins->context, ins);
    flb_input_pause(ins);
    wait_tail_collectors_state(ins->context, ins);
}
"#;

/// `90978ac6~1`: the non-threaded test pauses from the test thread (NC1).
const PREVIOUS_TEST: &str = r#"
void flb_test_inotify_pause_collectors()
{
    struct flb_input_instance *ins = 0;
    struct flb_tail_config *tail_ctx = 0;
    flb_start(0);
    flb_input_pause(ins);
    flb_input_collector_running(tail_ctx->coll_fd_fs1, ins);
}
TEST_LIST = {
    { "inotify_pause_collectors", flb_test_inotify_pause_collectors },
    { "inotify_threaded_pause_collectors", flb_test_inotify_threaded_pause_collectors },
    { 0, 0 }
};
"#;

/// `90978ac6`: pause, snapshot and resume run in a collector callback on the
/// engine thread, registered from a copied vtable's `cb_pre_run` (V*).
const FIXED_TEST: &str = r#"
struct tail_pause_test { struct flb_input_plugin plugin; struct flb_input_plugin *original_plugin; int collector_id; };
static int cb_tail_pause_test(struct flb_input_instance *ins, void *config, void *data)
{
    struct tail_pause_test *test = ins->data;
    struct flb_tail_config *tail_ctx = data;
    flb_input_collector_pause(test->collector_id, ins);
    flb_input_pause(ins);
    flb_input_collector_running(tail_ctx->coll_fd_fs1, ins);
    return 0;
}
static int cb_tail_pause_test_pre_run(struct flb_input_instance *ins, void *config, void *data)
{
    struct tail_pause_test *test = ins->data;
    test->original_plugin->cb_pre_run(ins, config, data);
    test->collector_id = flb_input_set_collector_time(ins, cb_tail_pause_test, 0, 10000000, config);
    return 0;
}
void flb_test_inotify_pause_collectors()
{
    struct flb_input_instance *ins = 0;
    struct tail_pause_test test;
    test.original_plugin = ins->p;
    test.plugin = *ins->p;
    test.plugin.cb_pre_run = cb_tail_pause_test_pre_run;
    ins->p = &test.plugin;
    flb_start(0);
}
TEST_LIST = {
    { "inotify_pause_collectors", flb_test_inotify_pause_collectors },
    { "inotify_threaded_pause_collectors", flb_test_inotify_threaded_pause_collectors },
    { 0, 0 }
};
"#;

fn obligations() -> BTreeMap<String, qcore_contract::contract_loader::DeclaredLoopOwner> {
    loop_owner_obligations(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads")
}

fn run(test: &str, extra: &[(&str, &str)]) -> Vec<OwnerSite> {
    let mut files = vec![
        CensusFile {
            path: "src/flb_lib.c".into(),
            text: LIBRARY.into(),
        },
        CensusFile {
            path: "plugins/in_tail/tail.c".into(),
            text: PLUGIN.into(),
        },
        CensusFile {
            path: "tests/runtime/in_tail.c".into(),
            text: format!("{THREADED_TEST}\n{test}"),
        },
    ];
    for (path, text) in extra {
        files.push(CensusFile {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    census(&files, &obligations(), &FLUENT_BIT_HOPS).sites
}

fn site<'a>(sites: &'a [OwnerSite], function: &str, callee: &str) -> Vec<&'a OwnerVerdict> {
    let found = sites
        .iter()
        .filter(|site| site.function.ends_with(function) && site.callee == callee)
        .map(|site| &site.verdict)
        .collect::<Vec<_>>();
    assert!(
        !found.is_empty(),
        "no site {function} -> {callee}: {sites:#?}"
    );
    found
}

fn not_discharged(verdicts: &[&OwnerVerdict], thread: &str) {
    for verdict in verdicts {
        let OwnerVerdict::NotDischarged { context, .. } = verdict else {
            panic!("expected a refusal, got {verdict:?}");
        };
        assert_eq!(context.thread, thread);
    }
}

#[test]
fn the_production_module_declares_four_obligations() {
    let declared = obligations();
    assert_eq!(declared.len(), 4);
    assert_eq!(
        declared["flb_input_collector_running"].operation,
        LoopOwnerOperation::Read
    );
    for name in [
        "flb_input_collector_pause",
        "flb_input_collector_resume",
        "flb_input_collector_delete",
    ] {
        assert_eq!(declared[name].operation, LoopOwnerOperation::Mutate);
        assert_eq!(declared[name].parameter, 1);
    }
}

#[test]
fn nc1_the_previous_test_pauses_off_the_engine_thread() {
    let sites = run(PREVIOUS_TEST, &[]);
    // T → flb_input_pause (non-threaded fork) → plugin pause → in_tail_pause
    // → the collector pause.
    not_discharged(
        &site(
            &sites,
            "flb_tail_fs_inotify_pause",
            "flb_input_collector_pause",
        ),
        "T",
    );
    // The previous test's own running read on T.
    not_discharged(
        &site(
            &sites,
            "flb_test_inotify_pause_collectors",
            "flb_input_collector_running",
        ),
        "T",
    );
}

#[test]
fn nc2_the_fixed_callback_discharges_on_the_engine_loop() {
    let sites = run(FIXED_TEST, &[]);
    for callee in ["flb_input_collector_pause", "flb_input_collector_running"] {
        for verdict in site(&sites, "cb_tail_pause_test", callee) {
            let OwnerVerdict::Discharged { contexts } = verdict else {
                panic!("expected the owner obligation discharged, got {verdict:?}");
            };
            // E in the non-threaded mode (the engine loop's dispatch, reached
            // through the copied vtable's cb_pre_run registration), and I in
            // the threaded mode (the field-keyed registrant is attributed the
            // worker's dispatch too, where I owns its loop).
            assert!(contexts.iter().any(
                |c| c.thread == "src/flb_lib.c::flb_lib_worker" && c.mode == Mode::NonThreaded
            ));
            assert!(contexts.iter().all(|c| match c.mode {
                Mode::NonThreaded => c.thread == "src/flb_lib.c::flb_lib_worker",
                Mode::Threaded => c.thread == "src/flb_lib.c::input_thread",
                Mode::Either => false,
            }));
        }
    }
}

#[test]
fn nc1_after_the_fix_the_plugin_path_is_still_refused_through_the_threaded_test() {
    // The threaded test calls the public pause from T, and the census cannot
    // fix its threaded flag from configuration (F), so the non-threaded fork
    // reaches in_tail_pause on T: the plugin's collector pause is not
    // discharged at `90978ac6` either. This is the prediction's NC1 row.
    let sites = run(FIXED_TEST, &[]);
    not_discharged(
        &site(
            &sites,
            "flb_tail_fs_inotify_pause",
            "flb_input_collector_pause",
        ),
        "T",
    );
}

#[test]
fn nc3_the_threaded_poll_from_the_test_thread_is_refused_at_both_revisions() {
    for test in [PREVIOUS_TEST, FIXED_TEST] {
        let sites = run(test, &[]);
        not_discharged(
            &site(
                &sites,
                "wait_tail_collectors_state",
                "flb_input_collector_running",
            ),
            "T",
        );
    }
    // And the enqueue it waits on never discharges "applied".
    assert!(!claim_discharged(
        ControlClaim::OperationApplied,
        control_completion(Mode::Threaded)
    ));
}

#[test]
fn nc6_and_nc7_are_the_kernel_pairing_claims() {
    let sites = run(FIXED_TEST, &[]);
    let owner = site(&sites, "cb_tail_pause_test", "flb_input_collector_pause")[0].clone();
    // NC6: a resume while ingestion is inactive promises no running state.
    assert!(matches!(
        pairing(
            PairDirection::Resume,
            [CollectorResult::Succeeded; 2],
            Ingestion::Inactive,
            &owner
        ),
        PairingClaim::Refused(_)
    ));
    // NC7: tail_fs_inotify.c discards both results — no conclusion.
    assert!(matches!(
        pairing(
            PairDirection::Pause,
            [CollectorResult::Ignored; 2],
            Ingestion::Active,
            &owner
        ),
        PairingClaim::NoConclusion(_)
    ));
}

#[test]
fn an_escaped_address_leaves_the_obligation_unresolved() {
    // A second file takes the address of the plugin callback outside every
    // recognised hop: its callers are no longer closed.
    let escape = r#"
void *hook(void)
{
    return (void *) cb_tail_pause_test;
}
"#;
    let mut test = FIXED_TEST.to_owned();
    test.push_str(escape);
    let sites = run(&test, &[]);
    for verdict in site(&sites, "cb_tail_pause_test", "flb_input_collector_pause") {
        let OwnerVerdict::Unresolved { reasons } = verdict else {
            panic!("expected unresolved, got {verdict:?}");
        };
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("address of `cb_tail_pause_test`"))
        );
    }
}

#[test]
fn a_macro_or_a_field_read_as_a_value_leaves_the_obligation_unresolved() {
    let macro_file = "#define TAIL_CB cb_tail_pause_test\n";
    let mut test = FIXED_TEST.to_owned();
    test.push_str(macro_file);
    let sites = run(&test, &[]);
    assert!(
        site(&sites, "cb_tail_pause_test", "flb_input_collector_pause")
            .iter()
            .all(|v| matches!(v, OwnerVerdict::Unresolved { .. }))
    );
    // Reading the collector field into a variable lets its registrants run
    // wherever that value goes.
    let leak = r#"
void *leak(struct flb_input_collector *c)
{
    void *f = c->cb_collect;
    return f;
}
"#;
    let sites = run(FIXED_TEST, &[("src/leak.c", leak)]);
    assert!(
        site(&sites, "cb_tail_pause_test", "flb_input_collector_pause")
            .iter()
            .all(|v| matches!(v, OwnerVerdict::Unresolved { .. }))
    );
}

#[test]
fn a_second_engine_dispatcher_thread_withholds_the_engine_owner() {
    // T itself runs the engine dispatcher: the engine loop has no single
    // derived owner, so nothing on it is discharged.
    let extra = r#"
void flb_test_drive_engine()
{
    flb_input_collector_fd(0, 0);
}
TEST_LIST = {
    { "drive", flb_test_drive_engine },
    { 0, 0 }
};
"#;
    let sites = run(FIXED_TEST, &[("tests/runtime/other.c", extra)]);
    assert!(
        site(&sites, "cb_tail_pause_test", "flb_input_collector_pause")
            .iter()
            .all(|v| !matches!(v, OwnerVerdict::Discharged { .. }))
    );
}

#[test]
fn every_resource_route_refuses_a_declared_operation() {
    // The per-unit route has no derived thread: the declaration is loaded
    // but a call to it is refused with the loop-owner wording.
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("resource-v1 loads the module");
    assert!(contracts.contract("flb_input_collector_pause").is_some());
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let source = "struct flb_input_instance; \
        int flb_input_collector_pause(int coll_id, struct flb_input_instance *in); \
        void f(struct flb_input_instance *in) { flb_input_collector_pause(1, in); }";
    let result = qcore_c_gateway::legacy_c::analyze_unrestricted_c11(
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source,
            ..Default::default()
        },
        "f",
        &contracts,
    )
    .expect("clang accepts the fixture");
    let rendered = format!("{result:?}");
    assert!(
        rendered.contains(
            "call to `flb_input_collector_pause` outside its declared loop-owner obligation"
        ),
        "{rendered}"
    );
    assert!(!matches!(
        result,
        qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Safe(_)
    ));
}

#[test]
fn a_malformed_or_duplicate_obligation_rejects_the_module() {
    for clause in [
        "requires resource_loop_owner(in, write);",
        "requires resource_loop_owner(in);",
        "requires resource_loop_owner(coll_id, read);",
        "requires resource_loop_owner(in, read); requires resource_loop_owner(in, read);",
        "requires resource_loop_owner(nobody, read);",
    ] {
        let module = format!(
            "module contract;\nextern fn g(coll_id: i32, in: ptr<mut, u8>) -> i32\ncontract {{\n    \
             requires resource_contract_v1;\n    {clause}\n    ensures normal \
             resource_borrowed(coll_id);\n    ensures normal resource_borrowed(in);\n    ensures \
             normal resource_return_none();\n}};\n"
        );
        let sources = BTreeMap::from([("m.qv".to_owned(), module)]);
        assert!(loop_owner_obligations(&sources).is_err(), "{clause}");
        assert!(
            load_resource_contract_sources(&sources).is_err(),
            "{clause}"
        );
    }
}
