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

//! W47-LOOP-OWNER-TYPED-DISPATCH — typed field dispatch (Step A) and the
//! instance flag of a runtime plugin copy (Step B), over compilable fixtures
//! built from the Fluent Bit shapes at `90978ac6` (`src/flb_input.c:2245`,
//! `:3044`, `:3313-3321`, `:625`; `src/proxy/go/go.c:248`;
//! `tests/runtime/in_tail.c`'s `cb_tail_pause_test`). Each fixture is compiled
//! by Clang (the typed reading) and read by the lexical census. Negative
//! controls first.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::{CensusFile, FLUENT_BIT_INSTANCE_FLOW};
use qcore_c_gateway::typed_dispatch_census::{
    TypedFacts, TypedRecord, TypedUnit, dispatch_key, read_units, store_key,
};
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::{Mode, OwnerVerdict};
use quirin_source_analysis::loop_owner_census::{Census, FLUENT_BIT_HOPS, census_typed};

const CONTRACTS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
);

/// The input header, compilable: the plugin descriptor (with a pause and a
/// name), the instance with its coroutine flag, the collector, the coroutine
/// constructor and hand-off, and every prototype the library needs.
const INPUT_HEADER: &str = r#"
#ifndef FLB_INPUT_H
#define FLB_INPUT_H
#define FLB_INPUT_NET  4
#define FLB_INPUT_CORO 128
struct flb_coro { void *caller; void *callee; };
struct flb_input_instance;
struct flb_input_plugin {
    int flags;
    const char *name;
    int (*cb_init)(struct flb_input_instance *, void *, void *);
    int (*cb_pre_run)(struct flb_input_instance *, void *, void *);
    void (*cb_pause)(void *, void *);
};
struct flb_input_instance { struct flb_input_plugin *p; void *context; void *data; int threaded; int runs_in_coroutine; };
struct flb_input_collector { int (*cb_collect)(struct flb_input_instance *, void *, void *); struct flb_input_instance *instance; };
struct flb_input_coro { struct flb_coro *coro; };
struct test_entry { const char *name; void (*func)(void); };
#define TEST_LIST struct test_entry test_list[]
typedef int (*collect_fn)(struct flb_input_instance *, void *, void *);
void co_switch(void *to);
void *co_active(void);
void *co_create(int size, void (*entry)(void), int *stack);
struct flb_coro *flb_coro_get(void);
struct flb_input_collector *params_collector(void);
struct flb_coro *params_coro(void);
struct flb_input_coro *flb_input_coro_create(struct flb_input_instance *ins, void *config);
int mk_utils_worker_spawn(void (*func)(void *), void *data, int *tid);
int flb_tp_thread_create(int id, void (*func)(void *), void *data, void *config);
int flb_engine_start(void *config);
int input_collector_fd(int fd, void *config);
int flb_io_net_write(int fd);
int flb_input_pause(struct flb_input_instance *ins);
int flb_start(void *ctx);
int flb_input_set_collector_time(struct flb_input_instance *ins, collect_fn cb, long s, long ns, void *config);
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in);
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in);
int flb_input_collector_running(int coll_id, struct flb_input_instance *in);
struct flb_input_instance *flb_input_new(void *config, const char *input, void *data, int public_only);
struct flb_input_instance *first_input(void *ctx);
void register_copy(struct flb_input_plugin *copy);
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

/// The engine and worker loops, the public pause (`ins->p->cb_pause`, typed
/// `struct flb_input_plugin`), the collector registration storing its callback
/// parameter in `struct flb_input_collector`, and the instance constructor,
/// the only writer of the coroutine flag.
const LIBRARY: &str = r#"
#include "fluent-bit/flb_input.h"
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
static struct flb_input_collector the_collector;
static int collector_create(struct flb_input_instance *ins, collect_fn cb, void *config)
{
    struct flb_input_collector *coll = &the_collector;
    coll->cb_collect = cb;
    coll->instance = ins;
    return 0;
}
int flb_input_set_collector_time(struct flb_input_instance *ins, collect_fn cb, long s, long ns, void *config)
{
    return collector_create(ins, cb, config);
}
static struct flb_input_instance the_instance;
struct flb_input_instance *flb_input_new(void *config, const char *input, void *data, int public_only)
{
    struct flb_input_plugin *plugin = 0;
    struct flb_input_instance *instance = &the_instance;
    instance->p = plugin;
    instance->runs_in_coroutine = 0;
    if (plugin->flags & FLB_INPUT_CORO) {
        instance->runs_in_coroutine = 1;
    }
    return instance;
}
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_running(int coll_id, struct flb_input_instance *in) { return 0; }
"#;

/// in_tail runs no coroutine; its pause is a registrant of
/// `struct flb_input_plugin.cb_pause`.
const TAIL_PLUGIN: &str = r#"
#include "fluent-bit/flb_input.h"
static void in_tail_pause(void *data, void *config)
{
    flb_input_collector_pause(1, 0);
}
static int in_tail_collect(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_collector_pause(1, ins);
    return 0;
}
static int in_tail_init(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input_set_collector_time(ins, in_tail_collect, 0, 1, config);
    return 0;
}
struct flb_input_plugin in_tail_plugin = {
    .name = "tail",
    .cb_init = in_tail_init,
    .cb_pause = in_tail_pause,
    .flags = 0
};
"#;

/// A coroutine input whose pause may yield: it is a registrant of the same
/// `cb_pause` type, so a callback that calls the public pause may yield.
const CORO_PLUGIN: &str = r#"
#include "fluent-bit/flb_input.h"
static void in_coro_pause(void *data, void *config)
{
    flb_io_net_write(0);
}
struct flb_input_plugin in_coro_plugin = {
    .name = "coro",
    .cb_pause = in_coro_pause,
    .flags = FLB_INPUT_NET | FLB_INPUT_CORO
};
"#;

/// A different struct type with the same field name (the Go proxy's shape,
/// `src/proxy/go/go.c:248`): its dispatch runs on the test thread.
const OTHER: &str = r#"
#include "fluent-bit/flb_input.h"
struct other_plugin { void (*cb_pause)(void *, void *); };
static void other_pause(void *data, void *config)
{
    flb_input_collector_resume(3, 0);
}
struct other_plugin other_desc = { .cb_pause = other_pause };
void other_dispatch(struct other_plugin *o)
{
    o->cb_pause(0, 0);
}
"#;

/// The test's callback and its pre-run hook, registered on a runtime copy of
/// the instance's plugin (`tests/runtime/in_tail.c`). `SETUP` is the body of
/// the test function that makes the copy.
fn test_file(setup: &str, extra: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
struct other_plugin;
extern struct other_plugin other_desc;
void other_dispatch(struct other_plugin *o);
struct tail_pause_test {{ struct flb_input_plugin plugin; struct flb_input_plugin *original_plugin; int collector_id; }};
static int cb_tail_pause_test(struct flb_input_instance *ins, void *config, void *data)
{{
    struct tail_pause_test *test = ins->data;
    flb_input_collector_pause(test->collector_id, ins);
    flb_input_pause(ins);
    flb_input_collector_running(test->collector_id, ins);
    return 0;
}}
static int cb_tail_pause_test_pre_run(struct flb_input_instance *ins, void *config, void *data)
{{
    struct tail_pause_test *test = ins->data;
    test->original_plugin->cb_pre_run(ins, config, data);
    test->collector_id = flb_input_set_collector_time(ins, cb_tail_pause_test, 0, 10000000, config);
    return 0;
}}
void flb_test_inotify_pause_collectors(void)
{{
{setup}
    flb_start(0);
}}
void flb_test_other(void)
{{
    other_dispatch(&other_desc);
}}
{extra}
TEST_LIST = {{
    {{ "inotify_pause_collectors", flb_test_inotify_pause_collectors }},
    {{ "other", flb_test_other }},
    {{ 0, 0 }}
}};
"#
    )
}

/// The fixed test's own shape: the list-head instance, not tied to its
/// creation, and the holder of the copy escaping into `ins->data`.
const FIXED_SHAPE: &str = r#"
    struct flb_input_instance *ins;
    struct tail_pause_test test = {0};
    ins = first_input(0);
    test.original_plugin = ins->p;
    test.plugin = *ins->p;
    test.plugin.cb_pre_run = cb_tail_pause_test_pre_run;
    ins->p = &test.plugin;
    ins->data = &test;
"#;

/// The same shape on an instance created from the `tail` descriptor, the
/// copy's holder kept local.
const DERIVED_SHAPE: &str = r#"
    struct flb_input_instance *ins = flb_input_new(0, "tail", 0, 1);
    struct tail_pause_test test = {0};
    test.original_plugin = ins->p;
    test.plugin = *ins->p;
    test.plugin.cb_pre_run = cb_tail_pause_test_pre_run;
    ins->p = &test.plugin;
"#;

struct Fixture {
    root: PathBuf,
    files: Vec<CensusFile>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_fixture(name: &str, test: &str, extra: &[(&str, &str)]) -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "qcore-typed-dispatch-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let mut sources = vec![
        ("include/fluent-bit/flb_input.h", INPUT_HEADER.to_owned()),
        ("src/flb_lib.c", LIBRARY.to_owned()),
        ("plugins/in_tail/tail.c", TAIL_PLUGIN.to_owned()),
        ("plugins/in_coro/coro.c", CORO_PLUGIN.to_owned()),
        ("src/other.c", OTHER.to_owned()),
        ("tests/runtime/in_tail.c", test.to_owned()),
    ];
    for (path, text) in extra {
        sources.push((path, (*text).to_owned()));
    }
    let mut files = Vec::new();
    for (path, text) in sources {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("fixture directory");
        fs::write(&full, &text).expect("fixture file");
        files.push(CensusFile {
            path: path.to_owned(),
            text,
        });
    }
    Fixture { root, files }
}

fn facts(fixture: &Fixture, untyped: &[&str]) -> TypedFacts {
    let units = fixture
        .files
        .iter()
        .filter(|file| {
            Path::new(&file.path).extension().is_some_and(|e| e == "c")
                && !untyped.contains(&file.path.as_str())
        })
        .map(|file| TypedUnit {
            file: fixture.root.join(&file.path),
            directory: fixture.root.clone(),
            arguments: vec![
                "-std=gnu17".to_owned(),
                format!("-I{}", fixture.root.join("include").display()),
                "-c".to_owned(),
                fixture.root.join(&file.path).display().to_string(),
            ],
        })
        .collect::<Vec<_>>();
    let facts = read_units(
        &units,
        &fixture.root,
        &FLUENT_BIT_HOPS.instance_flow_options(&FLUENT_BIT_INSTANCE_FLOW),
        2,
    );
    assert!(facts.failures.is_empty(), "{:#?}", facts.failures);
    facts
}

fn run(fixture: &Fixture, typed: Option<&TypedFacts>) -> Census {
    let obligations = loop_owner_obligations(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads");
    census_typed(&fixture.files, &obligations, &FLUENT_BIT_HOPS, None, typed)
}

fn verdicts<'a>(census: &'a Census, function: &str) -> Vec<&'a OwnerVerdict> {
    let found = census
        .sites
        .iter()
        .filter(|site| site.function.ends_with(function))
        .map(|site| &site.verdict)
        .collect::<Vec<_>>();
    assert!(!found.is_empty(), "no site in {function}");
    found
}

fn threads(census: &Census, function: &str) -> Vec<String> {
    census
        .derived(function)
        .expect("function")
        .contexts
        .iter()
        .map(|c| c.thread.clone())
        .collect()
}

fn edge(census: &Census, caller: &str, callee: &str) -> bool {
    let (Some(caller), Some(callee)) = (census.reading.index(caller), census.reading.index(callee))
    else {
        return false;
    };
    census
        .reading
        .edges
        .iter()
        .any(|e| e.caller == caller && e.callee == callee)
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

const TAIL_PAUSE: &str = "plugins/in_tail/tail.c::in_tail_pause";
const OTHER_PAUSE: &str = "src/other.c::other_pause";

#[test]
fn nc_a_dispatch_through_one_struct_never_reaches_another_structs_registrant() {
    let fixture = make_fixture("two-structs", &test_file(DERIVED_SHAPE, ""), &[]);
    // Field-keyed: `o->cb_pause` of `struct other_plugin` reaches in_tail's
    // pause, so the test thread T calls in_tail's collector operation.
    let untyped = run(&fixture, None);
    assert!(edge(&untyped, "other_dispatch", TAIL_PAUSE));
    assert!(threads(&untyped, TAIL_PAUSE).contains(&"T".to_owned()));
    // Typed: it reaches only `struct other_plugin`'s registrant.
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    assert!(!edge(&typed, "other_dispatch", TAIL_PAUSE));
    assert!(edge(&typed, "other_dispatch", OTHER_PAUSE));
    assert!(!edge(&typed, "flb_input_plugin_pause", OTHER_PAUSE));
    assert!(edge(&typed, "flb_input_plugin_pause", TAIL_PAUSE));
    assert!(!threads(&typed, TAIL_PAUSE).contains(&"T".to_owned()));
    // The other struct's own registrant still runs on T: not discharged.
    assert!(threads(&typed, OTHER_PAUSE).contains(&"T".to_owned()));
    assert!(matches!(
        verdicts(&typed, "other_pause")[0],
        OwnerVerdict::NotDischarged { .. }
    ));
}

#[test]
fn nc_a_cast_or_union_dispatch_is_unresolved() {
    let cast = r#"
#include "fluent-bit/flb_input.h"
void cast_dispatch(void *p)
{
    ((struct flb_input_plugin *) p)->cb_pause(0, 0);
}
"#;
    let fixture = make_fixture(
        "cast",
        &test_file(DERIVED_SHAPE, ""),
        &[("src/cast.c", cast)],
    );
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    unresolved_with(
        &verdicts(&typed, "in_tail_pause"),
        "whose record type is not matched (an explicit cast on the dispatch's object path)",
    );
    let union = r#"
#include "fluent-bit/flb_input.h"
union either { void (*cb_pause)(void *, void *); int (*cb_init)(void); };
void union_dispatch(union either *u)
{
    u->cb_pause(0, 0);
}
"#;
    let fixture = make_fixture(
        "union",
        &test_file(DERIVED_SHAPE, ""),
        &[("src/union.c", union)],
    );
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    unresolved_with(&verdicts(&typed, "in_tail_pause"), "(a union field)");
}

#[test]
fn nc_a_converted_store_is_unmatched_in_the_typed_reading() {
    let converted = r#"
#include "fluent-bit/flb_input.h"
typedef void (*pause_fn)(void *, void *);
static int not_a_pause(void) { return 0; }
void convert(struct flb_input_plugin *p)
{
    p->cb_pause = (pause_fn) not_a_pause;
}
"#;
    let fixture = make_fixture(
        "converted",
        &test_file(DERIVED_SHAPE, ""),
        &[("src/convert.c", converted)],
    );
    let typed_facts = facts(&fixture, &[]);
    let line = converted
        .lines()
        .position(|l| l.contains("(pause_fn) not_a_pause"))
        .expect("line")
        + 1;
    assert_eq!(
        typed_facts
            .stores
            .get(&store_key("src/convert.c", line, "cb_pause", "not_a_pause")),
        Some(
            &[TypedRecord::Unmatched(
                "the stored function is converted".to_owned()
            )]
            .into()
        )
    );
    // The lexical reading already refuses it: its address escapes.
    let typed = run(&fixture, Some(&typed_facts));
    assert!(
        typed
            .derived("src/convert.c::not_a_pause")
            .expect("function")
            .unresolved
            .iter()
            .any(|r| r.contains("address of `not_a_pause` taken"))
    );
}

#[test]
fn nc_a_file_without_a_typed_reading_keeps_the_field_keyed_reading() {
    let fixture = make_fixture("untyped-file", &test_file(DERIVED_SHAPE, ""), &[]);
    let typed_facts = facts(&fixture, &["src/other.c"]);
    assert!(
        !typed_facts
            .dispatches
            .keys()
            .any(|key| key.starts_with("src/other.c|"))
    );
    let typed = run(&fixture, Some(&typed_facts));
    // other.c's dispatch is untyped: it reaches every `cb_pause` registrant,
    // and its registrant (untyped) is reached by every `cb_pause` dispatch.
    assert!(edge(&typed, "other_dispatch", TAIL_PAUSE));
    assert!(edge(&typed, "flb_input_plugin_pause", OTHER_PAUSE));
    assert!(threads(&typed, TAIL_PAUSE).contains(&"T".to_owned()));
}

#[test]
fn nc_the_fixed_test_shape_keeps_the_coroutine_route_while_a_yield_is_reachable() {
    let fixture = make_fixture("fixed-shape", &test_file(FIXED_SHAPE, ""), &[]);
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    let flag = typed
        .reading
        .instance_flags
        .get(
            &typed
                .reading
                .index("tests/runtime/in_tail.c::cb_tail_pause_test")
                .expect("id"),
        )
        .expect("a Step B derivation");
    assert_eq!(flag.derived, None);
    // Card W47-LOOP-OWNER-INSTANCE-FLOW: the holder's escape into the
    // installed instance's `data` is now derived not to reach the route (the
    // holder's type is the test file's own, the copy member is written only
    // here, `p` only by the constructor and this installation); the instance
    // read from an unknown function stays untied.
    assert!(
        !flag.reason.contains("holding the copy escapes"),
        "{}",
        flag.reason
    );
    assert!(flag.reason.contains("holder escape:"), "{}", flag.reason);
    assert!(
        flag.reason.contains("not tied to its creation"),
        "{}",
        flag.reason
    );
    // The callback reaches in_coro's pause (same type) and its yield.
    unresolved_with(&verdicts(&typed, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_a_copy_whose_flags_are_written_with_the_coroutine_flag_keeps_the_route() {
    let setup = format!("{DERIVED_SHAPE}    test.plugin.flags = FLB_INPUT_CORO;\n");
    let fixture = make_fixture("flags-coro", &test_file(&setup, ""), &[]);
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    let id = typed
        .reading
        .index("tests/runtime/in_tail.c::cb_tail_pause_test")
        .expect("id");
    assert_eq!(typed.reading.instance_flags[&id].derived, Some(true));
    unresolved_with(&verdicts(&typed, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_a_copy_passed_to_an_unmodelled_function_is_underived() {
    let setup = format!("{DERIVED_SHAPE}    register_copy(&test.plugin);\n");
    let fixture = make_fixture("copy-escapes", &test_file(&setup, ""), &[]);
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    let id = typed
        .reading
        .index("tests/runtime/in_tail.c::cb_tail_pause_test")
        .expect("id");
    let flag = &typed.reading.instance_flags[&id];
    assert_eq!(flag.derived, None);
    assert!(flag.reason.contains("the copy's address is passed on"));
    unresolved_with(&verdicts(&typed, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_a_second_writer_of_the_instance_flag_leaves_it_underived() {
    let writer = r#"
#include "fluent-bit/flb_input.h"
void force_coroutine(struct flb_input_instance *ins)
{
    ins->runs_in_coroutine = 1;
}
"#;
    let fixture = make_fixture(
        "second-writer",
        &test_file(DERIVED_SHAPE, ""),
        &[("src/writer.c", writer)],
    );
    let typed_facts = facts(&fixture, &[]);
    let typed = run(&fixture, Some(&typed_facts));
    let id = typed
        .reading
        .index("tests/runtime/in_tail.c::cb_tail_pause_test")
        .expect("id");
    let flag = &typed.reading.instance_flags[&id];
    assert_eq!(flag.derived, None);
    assert!(
        flag.reason
            .contains("written other than by `flb_input_new` alone")
    );
}

#[test]
fn the_derived_shape_discharges_only_when_both_flags_derive_no_coroutine() {
    let fixture = make_fixture("derived", &test_file(DERIVED_SHAPE, ""), &[]);
    // Without the typed reading the plugin name is not read: underived.
    let untyped = run(&fixture, None);
    unresolved_with(&verdicts(&untyped, "cb_tail_pause_test"), "may yield");
    let typed_facts = facts(&fixture, &[]);
    let line = DERIVED_SHAPE.lines().count();
    assert!(line > 0);
    let typed = run(&fixture, Some(&typed_facts));
    let id = typed
        .reading
        .index("tests/runtime/in_tail.c::cb_tail_pause_test")
        .expect("id");
    assert_eq!(
        typed.reading.instance_flags[&id].derived,
        Some(false),
        "{}",
        typed.reading.instance_flags[&id].reason
    );
    discharged_on_the_owners(&verdicts(&typed, "cb_tail_pause_test"));
}

#[test]
fn the_typed_reading_names_each_record_through_typedefs() {
    let aliased = r#"
#include "fluent-bit/flb_input.h"
typedef struct flb_input_plugin plugin_t;
typedef struct { void (*cb_pause)(void *, void *); } anonymous_t;
static void anon_pause(void *d, void *c) { }
anonymous_t anon = { .cb_pause = anon_pause };
void through_alias(plugin_t *p, anonymous_t *a)
{
    p->cb_pause(0, 0);
    a->cb_pause(0, 0);
}
"#;
    let fixture = make_fixture(
        "typedefs",
        &test_file(DERIVED_SHAPE, ""),
        &[("src/alias.c", aliased)],
    );
    let typed_facts = facts(&fixture, &[]);
    let line = |needle: &str| {
        aliased
            .lines()
            .position(|l| l.contains(needle))
            .expect("line")
            + 1
    };
    assert_eq!(
        typed_facts.dispatches.get(&dispatch_key(
            "src/alias.c",
            line("p->cb_pause"),
            "cb_pause"
        )),
        Some(&[TypedRecord::Record("struct flb_input_plugin".to_owned())].into())
    );
    assert_eq!(
        typed_facts.dispatches.get(&dispatch_key(
            "src/alias.c",
            line("a->cb_pause"),
            "cb_pause"
        )),
        Some(&[TypedRecord::Record("typedef anonymous_t".to_owned())].into())
    );
    assert_eq!(
        typed_facts.stores.get(&store_key(
            "src/alias.c",
            line("anon_pause };"),
            "cb_pause",
            "anon_pause"
        )),
        Some(&[TypedRecord::Record("typedef anonymous_t".to_owned())].into())
    );
    // The test's own dispatch and store are typed through its header.
    assert_eq!(
        typed_facts.dispatches.get(&dispatch_key(
            "tests/runtime/in_tail.c",
            test_file(DERIVED_SHAPE, "")
                .lines()
                .position(|l| l.contains("original_plugin->cb_pre_run"))
                .expect("line")
                + 1,
            "cb_pre_run"
        )),
        Some(&[TypedRecord::Record("struct flb_input_plugin".to_owned())].into())
    );
}
