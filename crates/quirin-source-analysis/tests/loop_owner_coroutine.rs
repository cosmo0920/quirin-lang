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

//! W47-LOOP-OWNER-COROUTINE-HOP — hop C (a coroutine runs on the thread that
//! resumes it) and the configured reading of conditional groups, over
//! fixtures built from the Fluent Bit shapes at `90978ac6`
//! (`include/fluent-bit/flb_input.h:663-731`, `src/flb_input.c:3313-3318`,
//! `src/flb_input_thread.c:162-167`, `src/flb_input.c:625`). Negative controls
//! first.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::thread_entry_census::{BuildConfiguration, CensusFile, read_program};
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::{Mode, OwnerVerdict};
use quirin_source_analysis::loop_owner_census::{
    FLUENT_BIT_HOPS, OwnerSite, census, census_configured,
};

const CONTRACTS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
);

/// The input header: include-guarded (its closing `#endif` after the last
/// token), the coroutine constructor naming `input_pre_cb_collect`, the
/// hand-off, the resume wrapper and the terminal return.
const INPUT_HEADER: &str = r#"
#ifndef FLB_INPUT_H
#define FLB_INPUT_H
#define FLB_INPUT_NET  4   /* input address may set host and port */
#define FLB_INPUT_CORO 128 /* plugin requires a thread on callbacks */
struct flb_coro { void *caller; void *callee; };
struct flb_input_plugin { int flags; void *cb_init; void *cb_pre_run; void *cb_pause; };
struct flb_input_instance { struct flb_input_plugin *p; void *context; void *data; int threaded; int runs_in_coroutine; };
struct flb_input_collector { int (*cb_collect)(struct flb_input_instance *, void *, void *); struct flb_input_instance *instance; };
struct flb_input_coro { struct flb_coro *coro; };
static inline void flb_coro_yield(struct flb_coro *coro, int ended)
{
    co_switch(coro->caller);
}
static inline void flb_coro_resume(struct flb_coro *coro)
{
    coro->caller = co_active();
    co_switch(coro->callee);
}
static inline void input_params_set(struct flb_coro *coro)
{
    co_switch(coro->callee);
}
static inline void input_pre_cb_collect(void)
{
    struct flb_input_collector *coll = params_collector();
    struct flb_coro *coro = params_coro();
    co_switch(coro->caller);
    coll->cb_collect(coll->instance, 0, coll->instance->context);
}
static inline void flb_input_coro_resume(struct flb_input_coro *input_coro)
{
    flb_coro_resume(input_coro->coro);
}
static inline struct flb_input_coro *flb_input_coro_collect(struct flb_input_collector *coll, void *config)
{
    struct flb_input_coro *input_coro = flb_input_coro_create(coll->instance, config);
    struct flb_coro *coro = input_coro->coro;
    coro->callee = co_create(64, input_pre_cb_collect, 0);
#ifdef FLB_HAVE_VALGRIND
    valgrind_stack_register(coro->callee);
#endif
    input_params_set(coro);
    return input_coro;
}
static inline void flb_input_return_do(int ret)
{
    struct flb_coro *coro = flb_coro_get();
    flb_coro_yield(coro, 1);
}
#define FLB_INPUT_RETURN(x) flb_input_return_do(x); return x;
#endif
"#;

/// The engine and worker loops; each dispatcher runs a coroutine input's
/// collector through the constructor and resumes the handle it returned.
const LIBRARY: &str = r#"
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
    struct flb_input_coro *input_coro;
    if (collector->instance->runs_in_coroutine) {
        input_coro = flb_input_coro_collect(collector, config);
        flb_input_coro_resume(input_coro);
    }
    else {
        collector->cb_collect(collector->instance, config, collector->instance->context);
    }
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
int input_collector_fd(int fd, void *config)
{
    struct flb_input_collector *collector = 0;
    struct flb_input_coro *input_coro;
    if (collector->instance->runs_in_coroutine) {
        input_coro = flb_input_coro_collect(collector, config);
        flb_input_coro_resume(input_coro);
    }
    else {
        collector->cb_collect(collector->instance, config, collector->instance->context);
    }
    return 0;
}
int flb_input_plugin_pause(struct flb_input_instance *ins)
{
    ins->p->cb_pause(ins->context, 0);
    return 0;
}
int flb_input_pause(struct flb_input_instance *ins)
{
    return flb_input_plugin_pause(ins);
}
int flb_io_net_write(int fd)
{
    struct flb_coro *co = flb_coro_get();
    flb_coro_yield(co, 0);
    return 0;
}
int flb_input_set_collector_time(struct flb_input_instance *ins, void *cb, long s, long ns, void *config)
{
    return 0;
}
int flb_input_set_collector_event(struct flb_input_instance *ins, void *cb, int fd, void *config)
{
    return 0;
}
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_running(int coll_id, struct flb_input_instance *in) { return 0; }
"#;

/// in_tail runs no coroutine (`.flags = 0`); its collector may reach a
/// yield, which must not matter.
const TAIL_PLUGIN: &str = r#"
static int in_tail_collect(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_collector_pause(1, ins);
    flb_io_net_write(0);
    return 0;
}
static int in_tail_init(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_set_collector_time(ins, in_tail_collect, 0, 1, config);
    return 0;
}
struct flb_input_plugin in_tail_plugin = {
    .cb_init = in_tail_init,
    .flags = 0
};
"#;

/// A coroutine input: one collector that never yields, one that may.
const CORO_PLUGIN: &str = r#"
static int cb_coro_collect(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_collector_pause(2, ins);
    FLB_INPUT_RETURN(0);
}
static int cb_coro_yielding(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_collector_resume(2, ins);
    flb_io_net_write(0);
    FLB_INPUT_RETURN(0);
}
static int in_coro_init(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_set_collector_time(ins, cb_coro_collect, 0, 1, config);
    flb_input_set_collector_event(ins, cb_coro_yielding, 3, config);
    return 0;
}
struct flb_input_plugin in_coro_plugin = {
    .cb_init = in_coro_init,
    .flags = FLB_INPUT_NET | FLB_INPUT_CORO
};
"#;

/// The fixed test's shape: the callback is registered from the test file on a
/// runtime copy of the plugin, so its coroutine flag is not derived.
const FIXED_TEST: &str = r#"
struct tail_pause_test { struct flb_input_plugin plugin; struct flb_input_plugin *original_plugin; int collector_id; };
static int cb_tail_pause_test(struct flb_input_instance *ins, void *config, void *data)
{
    struct tail_pause_test *test = ins->data;
    flb_input_collector_pause(test->collector_id, ins);
    flb_input_pause(ins);
    flb_input_collector_running(test->collector_id, ins);
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
"#;

const TEST_LIST: &str = r#"
TEST_LIST = {
    { "inotify_pause_collectors", flb_test_inotify_pause_collectors },
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

fn files(test: &str, extra: &[(&str, &str)]) -> Vec<CensusFile> {
    let mut files = vec![
        CensusFile {
            path: "include/fluent-bit/flb_input.h".into(),
            text: INPUT_HEADER.into(),
        },
        CensusFile {
            path: "src/flb_lib.c".into(),
            text: LIBRARY.into(),
        },
        CensusFile {
            path: "plugins/in_tail/tail.c".into(),
            text: TAIL_PLUGIN.into(),
        },
        CensusFile {
            path: "plugins/in_coro/coro.c".into(),
            text: CORO_PLUGIN.into(),
        },
        CensusFile {
            path: "tests/runtime/in_tail.c".into(),
            text: format!("{FIXED_TEST}\n{test}\n{TEST_LIST}"),
        },
    ];
    for (path, text) in extra {
        files.push(CensusFile {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    files
}

fn run(test: &str, extra: &[(&str, &str)]) -> Vec<OwnerSite> {
    census(&files(test, extra), &obligations(), &FLUENT_BIT_HOPS).sites
}

fn configuration(defined: &[&str]) -> BuildConfiguration {
    BuildConfiguration {
        units: BTreeMap::from([(
            "src/flb_lib.c".to_owned(),
            defined.iter().map(|name| (*name).to_owned()).collect(),
        )]),
        owned_prefixes: vec!["FLB_".to_owned()],
        defined_elsewhere: BTreeSet::new(),
        predefined: None,
        region_macros: None,
    }
}

fn verdicts<'a>(sites: &'a [OwnerSite], function: &str) -> Vec<&'a OwnerVerdict> {
    let found = sites
        .iter()
        .filter(|site| site.function.ends_with(function))
        .map(|site| &site.verdict)
        .collect::<Vec<_>>();
    assert!(!found.is_empty(), "no site in {function}: {sites:#?}");
    found
}

fn discharged_on_the_owners(verdicts: &[&OwnerVerdict]) {
    for verdict in verdicts {
        let OwnerVerdict::Discharged { contexts } = verdict else {
            panic!("expected the owner obligation discharged, got {verdict:?}");
        };
        assert!(contexts.iter().all(|c| match c.mode {
            Mode::NonThreaded => c.thread == "src/flb_lib.c::flb_lib_worker",
            Mode::Threaded => c.thread == "src/flb_lib.c::input_thread",
            Mode::Either => false,
        }));
    }
}

fn unresolved_with(verdicts: &[&OwnerVerdict], needle: &str) {
    for verdict in verdicts {
        let OwnerVerdict::Unresolved { reasons } = verdict else {
            panic!("expected unresolved, got {verdict:?}");
        };
        assert!(
            reasons.iter().any(|r| r.contains(needle)),
            "{needle}: {reasons:#?}"
        );
    }
}

fn not_discharged_on(verdicts: &[&OwnerVerdict], thread: &str) {
    for verdict in verdicts {
        let OwnerVerdict::NotDischarged { context, .. } = verdict else {
            panic!("expected a refusal, got {verdict:?}");
        };
        assert_eq!(context.thread, thread);
    }
}

/// T constructs and resumes an input coroutine itself.
const T_RESUMES: &str = r#"
void flb_test_drive_coro()
{
    struct flb_input_coro *handle;
    handle = flb_input_coro_collect(0, 0);
    flb_input_coro_resume(handle);
}
TEST_LIST = {
    { "drive", flb_test_drive_coro },
    { 0, 0 }
};
"#;

#[test]
fn nc_a_coroutine_resumed_from_a_non_owner_thread_is_not_discharged() {
    let sites = run("", &[("tests/runtime/drive.c", T_RESUMES)]);
    not_discharged_on(&verdicts(&sites, "cb_coro_collect"), "T");
}

#[test]
fn nc_an_underived_resumer_leaves_the_coroutine_unresolved() {
    // A handle that is not the constructor's result.
    let foreign = r#"
void resume_someone(struct flb_input_coro *other)
{
    flb_input_coro_resume(other);
}
"#;
    let sites = run("", &[("src/foreign.c", foreign)]);
    unresolved_with(
        &verdicts(&sites, "cb_coro_collect"),
        "resumes a handle not derived from `flb_input_coro_collect`",
    );
    // A constructed coroutine its constructor's caller does not resume.
    let dropped = r#"
void construct_only(struct flb_input_collector *c)
{
    flb_input_coro_collect(c, 0);
}
"#;
    let sites = run("", &[("src/dropped.c", dropped)]);
    unresolved_with(
        &verdicts(&sites, "cb_coro_collect"),
        "without resuming the handle it returns",
    );
}

#[test]
fn nc_a_coroutine_body_that_may_yield_has_no_derived_resumer_after_the_yield() {
    let sites = run("", &[]);
    unresolved_with(
        &verdicts(&sites, "cb_coro_yielding"),
        "its resumer after the yield is not derived",
    );
}

#[test]
fn nc_an_unreadable_hand_off_leaves_the_entry_unresolved() {
    let looped = INPUT_HEADER.replace(
        "    co_switch(coro->caller);\n    coll->cb_collect",
        "    while (1) { co_switch(coro->caller); }\n    coll->cb_collect",
    );
    let mut files = files("", &[]);
    files[0].text = looped;
    let sites = census(&files, &obligations(), &FLUENT_BIT_HOPS).sites;
    unresolved_with(
        &verdicts(&sites, "cb_coro_collect"),
        "the hand-off of coroutine entry",
    );
}

#[test]
fn nc_a_coroutine_path_under_a_branch_the_build_does_not_take_is_not_used() {
    let guarded = format!("#ifdef FLB_HAVE_NOT_BUILT\n{T_RESUMES}\n#endif\n");
    let files = files("", &[("tests/runtime/drive.c", &guarded)]);
    // Raw: every arm is read, so T's resume refuses.
    let raw = census(&files, &obligations(), &FLUENT_BIT_HOPS).sites;
    not_discharged_on(&verdicts(&raw, "cb_coro_collect"), "T");
    // Configured: no compile command defines the macro and no file does,
    // so the arm is not read.
    let configured = census_configured(
        &files,
        &obligations(),
        &FLUENT_BIT_HOPS,
        Some(&configuration(&["FLB_HAVE_VALGRIND"])),
    )
    .sites;
    discharged_on_the_owners(&verdicts(&configured, "cb_coro_collect"));
    // A branch the build takes stays; so does one it cannot decide.
    for (open, defined) in [
        ("#ifdef FLB_HAVE_NOT_BUILT", &["FLB_HAVE_NOT_BUILT"][..]),
        ("#if defined(__linux__)", &[][..]),
        ("#ifdef FLB_HAVE_WRITTEN", &[][..]),
    ] {
        let guarded = format!("#define FLB_HAVE_WRITTEN\n{open}\n{T_RESUMES}\n#endif\n");
        let files = files_with(&guarded);
        let configured = census_configured(
            &files,
            &obligations(),
            &FLUENT_BIT_HOPS,
            Some(&configuration(defined)),
        )
        .sites;
        not_discharged_on(&verdicts(&configured, "cb_coro_collect"), "T");
    }
}

fn files_with(drive: &str) -> Vec<CensusFile> {
    files("", &[("tests/runtime/drive.c", drive)])
}

#[test]
fn a_coroutine_resumed_only_by_the_loop_dispatchers_runs_on_the_owners() {
    let sites = run("", &[]);
    discharged_on_the_owners(&verdicts(&sites, "cb_coro_collect"));
}

#[test]
fn an_input_that_runs_no_coroutine_does_not_depend_on_the_hop() {
    // in_tail's collector may reach a yield, but its descriptor's flags are
    // 0: it is never dispatched from the coroutine entry.
    let sites = run("", &[]);
    discharged_on_the_owners(&verdicts(&sites, "in_tail_collect"));
    // The same collector registered in a coroutine plugin's directory is a
    // coroutine body and unresolved.
    let coro_tail = TAIL_PLUGIN.replace(".flags = 0", ".flags = FLB_INPUT_CORO");
    let mut files = files("", &[]);
    files[2].text = coro_tail;
    let sites = census(&files, &obligations(), &FLUENT_BIT_HOPS).sites;
    unresolved_with(
        &verdicts(&sites, "in_tail_collect"),
        "its resumer after the yield is not derived",
    );
    // A flags operand that is not a numeric macro leaves the flag undecided.
    let mut files = files_with("");
    files[2].text = TAIL_PLUGIN.replace(".flags = 0", ".flags = tail_flags()");
    let sites = census(&files, &obligations(), &FLUENT_BIT_HOPS).sites;
    unresolved_with(
        &verdicts(&sites, "in_tail_collect"),
        "its resumer after the yield is not derived",
    );
}

#[test]
fn the_fixed_test_path_is_discharged_only_when_the_hop_derives_it() {
    // The test's callback has no derived coroutine flag, so the coroutine
    // route stands; with no yield reachable it runs on the dispatchers only.
    let sites = run("", &[]);
    discharged_on_the_owners(&verdicts(&sites, "cb_tail_pause_test"));
    // A pause callback that may yield makes the callback's post-yield
    // resumer underived: unresolved, not discharged.
    let yielding_pause = r#"
static void in_net_pause(void *data, void *config)
{
    flb_io_net_write(0);
}
struct flb_input_plugin in_net_plugin = {
    .cb_pause = in_net_pause,
    .flags = FLB_INPUT_NET
};
"#;
    let sites = run("", &[("plugins/in_net/net.c", yielding_pause)]);
    unresolved_with(
        &verdicts(&sites, "cb_tail_pause_test"),
        "`tests/runtime/in_tail.c::cb_tail_pause_test` runs in coroutine",
    );
}

#[test]
fn the_entry_is_resolved_and_runs_on_both_owners() {
    let census = census(&files("", &[]), &obligations(), &FLUENT_BIT_HOPS);
    let entry = census
        .derived("include/fluent-bit/flb_input.h::input_pre_cb_collect")
        .expect("the entry is read");
    assert!(entry.unresolved.is_empty(), "{:?}", entry.unresolved);
    let threads = entry
        .contexts
        .iter()
        .map(|c| (c.thread.as_str(), c.mode))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        threads,
        BTreeSet::from([
            ("src/flb_lib.c::flb_lib_worker", Mode::NonThreaded),
            ("src/flb_lib.c::input_thread", Mode::Threaded),
        ])
    );
}

#[test]
fn an_include_guard_closed_after_the_last_token_is_structured() {
    let reading = read_program(&files("", &[]), &FLUENT_BIT_HOPS);
    assert!(
        reading
            .unresolved
            .values()
            .flatten()
            .all(|reason| !reason.contains("cannot structure")),
        "{:#?}",
        reading.unresolved
    );
    // A group whose arms end at different depths is still unstructured when
    // read raw, and structured once the configuration decides it.
    let split = CensusFile {
        path: "src/split.c".into(),
        text: "#ifdef FLB_SYSTEM_WINDOWS\nint split(int a) {\n    if (a) {\n#else\nint \
               split(void) {\n#endif\n    return 0;\n}\n"
            .into(),
    };
    // Raw, the arms leave the file at different depths: no definition
    // closes, so `split` is not read at all.
    let raw = read_program(std::slice::from_ref(&split), &FLUENT_BIT_HOPS);
    assert!(raw.functions.is_empty(), "{:?}", raw.functions);
    let configured = qcore_c_gateway::thread_entry_census::read_program_configured(
        std::slice::from_ref(&split),
        &FLUENT_BIT_HOPS,
        Some(&configuration(&["FLB_SYSTEM_LINUX"])),
    );
    assert!(
        configured.unresolved.is_empty(),
        "{:#?}",
        configured.unresolved
    );
    assert_eq!(configured.functions, vec!["split".to_owned()]);
}
