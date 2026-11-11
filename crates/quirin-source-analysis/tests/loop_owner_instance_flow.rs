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

//! W47-LOOP-OWNER-INSTANCE-FLOW — the instance a runtime plugin copy is
//! installed on, over compilable fixtures of the Fluent Bit shapes at
//! `90978ac6`: `flb_input` → `flb_input_new` appending to `config->inputs`
//! (`src/flb_lib.c:268`, `src/flb_input.c:326`, `:677`), `flb_create`'s fresh
//! config (`src/flb_lib.c:145`, `src/flb_config.c:507`), `collector_create`
//! (`src/flb_input.c:2222`), the pre-run dispatch (`:2046`) and the fixed
//! test's list-head read and holder (`tests/runtime/in_tail.c:2551-2557`).
//! Each fixture is compiled by Clang (the typed reading) and read by the
//! lexical census. Negative controls first.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::predefined_macros::{
    PredefinedMacros, fixture_command, fixture_database, fixture_facts,
};
use qcore_c_gateway::thread_entry_census::{
    BuildConfiguration, CensusFile, FLUENT_BIT_INSTANCE_FLOW,
};
use qcore_c_gateway::typed_dispatch_census::{TypedFacts, TypedUnit, member_key, read_units};
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::{Mode, OwnerVerdict};
use quirin_source_analysis::loop_owner_census::{Census, FLUENT_BIT_HOPS, census_typed};

const CONTRACTS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
);

/// The input header: Monkey's list, the plugin descriptor, the instance on a
/// config's input list, the config, the library context, the collector, the
/// coroutine constructor and hand-off.
const INPUT_HEADER: &str = r#"
#ifndef FLB_INPUT_H
#define FLB_INPUT_H
#define NULL ((void *) 0)
#define FLB_INPUT_NET  4
#define FLB_INPUT_CORO 128
struct mk_list { struct mk_list *prev; struct mk_list *next; };
static inline void mk_list_init(struct mk_list *list)
{
    list->next = list;
    list->prev = list;
}
static inline void mk_list_add(struct mk_list *entry, struct mk_list *head)
{
    entry->next = head;
    entry->prev = head->prev;
    head->prev->next = entry;
    head->prev = entry;
}
#define mk_list_entry(ptr, type, member) ((type *) ((char *) (ptr) - __builtin_offsetof(type, member)))
struct flb_coro { void *caller; void *callee; };
struct flb_input_instance;
struct flb_input_plugin {
    int flags;
    const char *name;
    int (*cb_init)(struct flb_input_instance *, void *, void *);
    int (*cb_pre_run)(struct flb_input_instance *, void *, void *);
    void (*cb_pause)(void *, void *);
    struct mk_list _head;
};
struct flb_input_instance {
    struct flb_input_plugin *p;
    void *context;
    void *data;
    int threaded;
    int runs_in_coroutine;
    struct mk_list _head;
};
struct flb_config { struct mk_list inputs; struct mk_list in_plugins; };
struct flb_lib_ctx { struct flb_config *config; };
struct flb_input_collector { int (*cb_collect)(struct flb_input_instance *, void *, void *); struct flb_input_instance *instance; };
struct flb_input_coro { struct flb_coro *coro; };
struct test_entry { const char *name; void (*func)(void); };
#define TEST_LIST struct test_entry test_list[]
typedef int (*collect_fn)(struct flb_input_instance *, void *, void *);
void *flb_calloc(unsigned long n, unsigned long size);
void flb_free(void *p);
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
int flb_start(struct flb_lib_ctx *ctx);
struct flb_lib_ctx *flb_create(void);
int flb_input(struct flb_lib_ctx *ctx, const char *input, void *data);
int flb_service_set(struct flb_lib_ctx *ctx, const char *key);
void service_hook(struct flb_lib_ctx *ctx);
int flb_input_set_collector_time(struct flb_input_instance *ins, collect_fn cb, long s, long ns, void *config);
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in);
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in);
int flb_input_collector_running(int coll_id, struct flb_input_instance *in);
struct flb_input_instance *flb_input_new(struct flb_config *config, const char *input, void *data, int public_only);
void register_hook(void (*hook)(struct flb_config *));
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
#endif
"#;

/// The library: the engine and worker loops, the public pause, the pre-run
/// dispatch `ins->p->cb_pre_run(ins, …)`, the collector registration, the
/// fresh config and context, the constructor appending each instance to its
/// config's input list, and the `flb_input` wrapper.
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
int flb_start(struct flb_lib_ctx *ctx)
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
struct flb_config *flb_config_init(void)
{
    struct flb_config *config = flb_calloc(1, sizeof(struct flb_config));
    mk_list_init(&config->inputs);
    mk_list_init(&config->in_plugins);
    return config;
}
struct flb_lib_ctx *flb_create(void)
{
    struct flb_lib_ctx *ctx = flb_calloc(1, sizeof(struct flb_lib_ctx));
    ctx->config = flb_config_init();
    return ctx;
}
struct flb_input_instance *flb_input_new(struct flb_config *config, const char *input, void *data, int public_only)
{
    struct flb_input_plugin *plugin = 0;
    struct flb_input_instance *instance = flb_calloc(1, sizeof(struct flb_input_instance));
    instance->p = plugin;
    instance->data = data;
    instance->runs_in_coroutine = 0;
    if (plugin->flags & FLB_INPUT_CORO) {
        instance->runs_in_coroutine = 1;
    }
    mk_list_add(&instance->_head, &config->inputs);
    return instance;
}
int flb_input(struct flb_lib_ctx *ctx, const char *input, void *data)
{
    struct flb_input_instance *ins;
    ins = flb_input_new(ctx->config, input, data, 1);
    if (!ins) {
        return -1;
    }
    return 0;
}
int flb_service_set(struct flb_lib_ctx *ctx, const char *key)
{
    service_hook(ctx);
    return 0;
}
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_running(int coll_id, struct flb_input_instance *in) { return 0; }
"#;

/// The service hook: nothing, by default.
const SERVICE: &str = r#"
#include "fluent-bit/flb_input.h"
void service_hook(struct flb_lib_ctx *ctx)
{
}
"#;

const TAIL_PLUGIN: &str = r#"
#include "fluent-bit/flb_input.h"
static void in_tail_pause(void *data, void *config)
{
    flb_input_collector_pause(1, 0);
}
struct flb_input_plugin in_tail_plugin = {
    .name = "tail",
    .cb_pause = in_tail_pause,
    .flags = 0
};
"#;

/// A coroutine input whose pause may yield: a callback that calls the public
/// pause may yield while its route is kept.
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

/// The fixed test's callbacks and holder; `MAKER` is the context maker's
/// body after the context and before its return, `SETUP` the test body after
/// the maker's call and before the list-head read, `AFTER` after the
/// installation.
fn test_file(maker: &str, setup: &str, after: &str, extra: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
struct tail_pause_test {{ struct flb_input_plugin plugin; struct flb_input_plugin *original_plugin; int collector_id; }};
struct test_ctx {{ struct flb_lib_ctx *flb; int i_ffd; }};
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
static struct test_ctx *test_ctx_create(const char *name)
{{
    int i;
    struct test_ctx *ctx = flb_calloc(1, sizeof(struct test_ctx));
    if (!ctx) {{
        return NULL;
    }}
    ctx->flb = flb_create();
    flb_service_set(ctx->flb, "Flush");
{maker}
    return ctx;
}}
void flb_test_inotify_pause_collectors(void)
{{
    struct mk_list *head;
    struct flb_input_instance *ins;
    struct test_ctx *ctx;
    struct tail_pause_test test = {{0}};
    ctx = test_ctx_create("tail");
    if (!ctx) {{
        return;
    }}
{setup}
    head = ctx->flb->config->inputs.next;
    ins = mk_list_entry(head, struct flb_input_instance, _head);
    test.original_plugin = ins->p;
    test.plugin = *ins->p;
    test.plugin.cb_pre_run = cb_tail_pause_test_pre_run;
    ins->p = &test.plugin;
    ins->data = &test;
{after}
    flb_start(ctx->flb);
}}
{extra}
TEST_LIST = {{
    {{ "inotify_pause_collectors", flb_test_inotify_pause_collectors }},
    {{ 0, 0 }}
}};
"#
    )
}

const ONE_TAIL: &str = r#"    ctx->i_ffd = flb_input(ctx->flb, (char *) "tail", NULL);"#;

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
    let root =
        std::env::temp_dir().join(format!("qcore-instance-flow-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mut sources = vec![
        ("include/fluent-bit/flb_input.h", INPUT_HEADER.to_owned()),
        ("src/flb_lib.c", LIBRARY.to_owned()),
        ("plugins/in_tail/tail.c", TAIL_PLUGIN.to_owned()),
        ("plugins/in_coro/coro.c", CORO_PLUGIN.to_owned()),
        ("tests/runtime/in_tail.c", test.to_owned()),
    ];
    if !extra.iter().any(|(path, _)| *path == "src/service.c") {
        sources.push(("src/service.c", SERVICE.to_owned()));
    }
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

fn facts(fixture: &Fixture) -> TypedFacts {
    let units = fixture
        .files
        .iter()
        .filter(|file| Path::new(&file.path).extension().is_some_and(|e| e == "c"))
        .map(|file| TypedUnit {
            file: fixture.root.join(&file.path),
            directory: fixture.root.clone(),
            arguments: vec![
                "-std=gnu17".to_owned(),
                // This fixture deliberately leaves both platform arms untyped;
                // separate supplied macro facts exercise their decision gate.
                "-U_WIN32".to_owned(),
                "-U__linux__".to_owned(),
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

fn run(fixture: &Fixture) -> Census {
    let typed = facts(fixture);
    let obligations = loop_owner_obligations(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads");
    census_typed(
        &fixture.files,
        &obligations,
        &FLUENT_BIT_HOPS,
        None,
        Some(&typed),
    )
}

fn flag(census: &Census) -> (Option<bool>, String) {
    let id = census
        .reading
        .index("tests/runtime/in_tail.c::cb_tail_pause_test")
        .expect("id");
    let flag = &census.reading.instance_flags[&id];
    (flag.derived, flag.reason.clone())
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

/// The flag is underived for `needle` and the callback's sites stay
/// unresolved on the kept coroutine route.
fn route_kept(census: &Census, needle: &str) {
    let (derived, reason) = flag(census);
    assert_eq!(derived, None, "{reason}");
    assert!(reason.contains(needle), "{needle}: {reason}");
    unresolved_with(&verdicts(census, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_two_inputs_created_before_the_read_leave_the_head_ambiguous() {
    let maker = format!("{ONE_TAIL}\n    flb_input(ctx->flb, (char *) \"tail\", NULL);");
    let fixture = make_fixture("two-inputs", &test_file(&maker, "", "", ""), &[]);
    route_kept(&run(&fixture), "more than one call reaches a hazard");
}

#[test]
fn nc_an_input_created_by_a_dynamic_name_is_unresolved() {
    let maker = "    ctx->i_ffd = flb_input(ctx->flb, name, NULL);";
    let fixture = make_fixture("dynamic", &test_file(maker, "", "", ""), &[]);
    route_kept(&run(&fixture), "is not one literal the typed reading reads");
}

#[test]
fn nc_a_creation_inside_a_loop_is_unresolved() {
    let maker = "    for (i = 0; i < 2; i++) {\n        flb_input(ctx->flb, (char *) \"tail\", NULL);\n    }";
    let fixture = make_fixture("loop", &test_file(maker, "", "", ""), &[]);
    route_kept(&run(&fixture), "reaches a hazard inside a loop");
}

#[test]
fn nc_a_list_mutation_on_the_path_is_unresolved() {
    let service = r#"
#include "fluent-bit/flb_input.h"
static struct flb_input_instance other;
void service_hook(struct flb_lib_ctx *ctx)
{
    mk_list_add(&other._head, &ctx->config->inputs);
}
"#;
    let fixture = make_fixture(
        "mutation",
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/service.c", service)],
    );
    route_kept(&run(&fixture), "more than one call reaches a hazard");
}

/// A creator whose address escapes into a function pointer.
const WILD: &str = r#"
#include "fluent-bit/flb_input.h"
void (*saved_hook)(struct flb_config *);
static void make_more(struct flb_config *config)
{
    flb_input_new(config, "coro", 0, 1);
}
void install_hook(void)
{
    saved_hook = make_more;
}
"#;

#[test]
fn nc_a_creator_reachable_through_an_unmodelled_call_on_the_path_is_unresolved() {
    let service = r#"
#include "fluent-bit/flb_input.h"
extern void (*saved_hook)(struct flb_config *);
void service_hook(struct flb_lib_ctx *ctx)
{
    if (saved_hook) {
        saved_hook(ctx->config);
    }
}
"#;
    let fixture = make_fixture(
        "wild",
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/wild.c", WILD), ("src/service.c", service)],
    );
    route_kept(
        &run(&fixture),
        "may be called through a pointer from the path",
    );
}

#[test]
fn an_escaped_creator_no_unmodelled_call_on_the_path_can_reach_does_not_block() {
    let fixture = make_fixture(
        "wild-unreached",
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/wild.c", WILD)],
    );
    let census = run(&fixture);
    assert_eq!(flag(&census).0, Some(false), "{}", flag(&census).1);
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
}

#[test]
fn nc_a_call_into_a_file_the_reader_cannot_structure_is_unresolved() {
    let service = r#"
#include "fluent-bit/flb_input.h"
void hidden_impl(struct flb_lib_ctx *ctx);
void service_hook(struct flb_lib_ctx *ctx)
{
    hidden_impl(ctx);
}
"#;
    // The arms end at different depths, so the census cannot structure the
    // file; Clang compiles the second arm.
    let hidden = r#"
#include "fluent-bit/flb_input.h"
static struct flb_input_instance other;
#ifdef NEVER_DEFINED
void hidden_impl(struct flb_lib_ctx *ctx) { {
#else
void hidden_impl(struct flb_lib_ctx *ctx) {
#endif
    mk_list_add(&other._head, &ctx->config->inputs);
}
"#;
    let fixture = make_fixture(
        "unstructured",
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/service.c", service), ("src/hidden.c", hidden)],
    );
    route_kept(&run(&fixture), "cannot");
}

#[test]
fn nc_a_list_mutation_in_a_function_read_in_place_is_unresolved() {
    // The maker itself links another instance into the context's list.
    let maker = format!(
        "{ONE_TAIL}\n    static struct flb_input_instance other_ins;\n    \
         mk_list_add(&other_ins._head, &ctx->flb->config->inputs);"
    );
    let fixture = make_fixture("maker-mutation", &test_file(&maker, "", "", ""), &[]);
    route_kept(&run(&fixture), "read in place");
}

#[test]
fn nc_a_holder_that_writes_the_instance_flag_is_unresolved() {
    let extra = r#"
static int cb_force(struct flb_input_instance *ins, void *config, void *data)
{
    ins->runs_in_coroutine = 1;
    return 0;
}
"#;
    let fixture = make_fixture("flag-writer", &test_file(ONE_TAIL, "", "", extra), &[]);
    route_kept(
        &run(&fixture),
        "written other than by `flb_input_new` alone",
    );
}

#[test]
fn nc_a_holder_that_re_points_the_instance_plugin_is_unresolved() {
    let extra = r#"
static int cb_repoint(struct flb_input_instance *ins, void *config, void *data)
{
    struct tail_pause_test *test = ins->data;
    ins->p = test->original_plugin;
    return 0;
}
"#;
    let fixture = make_fixture("repoint", &test_file(ONE_TAIL, "", "", extra), &[]);
    route_kept(&run(&fixture), "an instance's `p` is written");
}

#[test]
fn nc_a_copy_member_written_outside_the_test_is_unresolved() {
    let extra = r#"
static int cb_rewrite(struct flb_input_instance *ins, void *config, void *data)
{
    struct tail_pause_test *test = ins->data;
    test->plugin.flags = FLB_INPUT_CORO;
    return 0;
}
"#;
    let fixture = make_fixture("copy-writer", &test_file(ONE_TAIL, "", "", extra), &[]);
    route_kept(&run(&fixture), "the copy member `plugin` is written");
}

#[test]
fn nc_a_dispatch_on_another_instance_leaves_the_callback_instance_underived() {
    let other = r#"
#include "fluent-bit/flb_input.h"
void other_pre_run(struct flb_input_instance *a, struct flb_input_instance *b)
{
    a->p->cb_pre_run(b, 0, 0);
}
"#;
    let fixture = make_fixture(
        "callback-instance",
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/other.c", other)],
    );
    route_kept(&run(&fixture), "P-CALLBACK-INSTANCE is not derived");
}

#[test]
fn nc_a_coroutine_descriptor_at_the_head_keeps_the_route() {
    let maker = r#"    ctx->i_ffd = flb_input(ctx->flb, (char *) "coro", NULL);"#;
    let fixture = make_fixture("coro-head", &test_file(maker, "", "", ""), &[]);
    let census = run(&fixture);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(true), "{reason}");
    unresolved_with(&verdicts(&census, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_a_descriptor_matching_only_ignoring_case_counts() {
    let shout = r#"
#include "fluent-bit/flb_input.h"
struct flb_input_plugin in_shout_plugin = {
    .name = "TAIL",
    .flags = FLB_INPUT_CORO
};
"#;
    let fixture = make_fixture(
        "case",
        &test_file(ONE_TAIL, "", "", ""),
        &[("plugins/in_shout/shout.c", shout)],
    );
    let census = run(&fixture);
    assert_eq!(flag(&census).0, Some(true), "{}", flag(&census).1);
    unresolved_with(&verdicts(&census, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_the_old_test_shape_is_not_discharged() {
    // The previous test (`90978ac6~1`) reads the head after the start and
    // operates the collectors on the test thread itself.
    let old = r#"
#include "fluent-bit/flb_input.h"
struct test_ctx { struct flb_lib_ctx *flb; int i_ffd; };
static struct test_ctx *test_ctx_create(void)
{
    struct test_ctx *ctx = flb_calloc(1, sizeof(struct test_ctx));
    if (!ctx) {
        return NULL;
    }
    ctx->flb = flb_create();
    ctx->i_ffd = flb_input(ctx->flb, (char *) "tail", NULL);
    return ctx;
}
void flb_test_inotify_pause_collectors(void)
{
    struct mk_list *head;
    struct flb_input_instance *ins;
    struct test_ctx *ctx;
    ctx = test_ctx_create();
    if (!ctx) {
        return;
    }
    flb_start(ctx->flb);
    head = ctx->flb->config->inputs.next;
    ins = mk_list_entry(head, struct flb_input_instance, _head);
    flb_input_collector_running(1, ins);
    flb_input_pause(ins);
    flb_input_collector_running(1, ins);
}
TEST_LIST = {
    { "inotify_pause_collectors", flb_test_inotify_pause_collectors },
    { 0, 0 }
};
"#;
    let fixture = make_fixture("old", old, &[]);
    let census = run(&fixture);
    for verdict in verdicts(&census, "flb_test_inotify_pause_collectors") {
        assert!(
            matches!(verdict, OwnerVerdict::NotDischarged { context, .. } if context.thread == "T"),
            "{verdict:?}"
        );
    }
}

#[test]
fn the_fixed_shape_discharges_only_when_every_derivation_holds() {
    let fixture = make_fixture("fixed", &test_file(ONE_TAIL, "", "", ""), &[]);
    let typed = facts(&fixture);
    // The typed reading records the members the derivation reads.
    assert!(
        typed
            .member_accesses
            .get(&member_key("struct flb_config", "inputs"))
            .is_some_and(|sites| sites
                .iter()
                .any(|s| s.contains("|flb_input_new|arg:mk_list_add")))
    );
    assert!(
        typed
            .member_accesses
            .get(&member_key("struct flb_input_instance", "p"))
            .is_some_and(|sites| sites
                .iter()
                .any(|s| s.contains("|flb_test_inotify_pause_collectors|write")))
    );
    let census = run(&fixture);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(reason.contains("list-head instance"), "{reason}");
    assert!(reason.contains("holder escape"), "{reason}");
    assert!(reason.contains("P-CALLBACK-INSTANCE derived"), "{reason}");
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
}

// Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: the fixed-test shape whose path
// reaches definitions only Windows arms hold — `localtime_r` under
// `#if defined (__linux__) … #elif defined (_WIN32)`
// (`lib/monkey/mk_core/mk_utils.c:41`) and `dirname` under
// `#ifndef _WIN32 … #else` (`src/config_format/flb_cf_yaml.c:550`) — beside an
// escaped creator. Clang compiles neither arm's definition, so without a
// decision both have no typed reading and hold the list-head derivation.

/// The service hook reaches both Windows-only definitions.
const WINDOWS_ARMS: &str = r#"
#include "fluent-bit/flb_input.h"
int platform_localtime(int t);
char *platform_dirname(char *path);
#if defined (__linux__)
static int linux_only;
#elif defined (_WIN32)
int platform_localtime(int t)
{
    return t;
}
#endif
#ifndef _WIN32
static int posix_only;
#else
char *platform_dirname(char *path)
{
    return path;
}
#endif
void service_hook(struct flb_lib_ctx *ctx)
{
    platform_localtime(0);
    platform_dirname(0);
}
"#;

/// The derived facts of a fixture: each `.c` file one command whose start
/// of unit defines `defined`; `failed` names a file whose derivation failed.
fn predefined(
    fixture: &Fixture,
    defined: &[&str],
    file_defined: &[&str],
    failed: Option<&str>,
) -> PredefinedMacros {
    let root = fixture.root.display().to_string();
    let units = fixture
        .files
        .iter()
        .map(|file| file.path.as_str())
        .filter(|path| path.ends_with(".c"))
        .collect::<Vec<_>>();
    let database = fixture_database(&root, &units);
    let commands = units
        .iter()
        .map(|path| {
            let mut command = fixture_command(path, defined, file_defined, &[]);
            if failed == Some(*path) {
                command.failure = Some("`-E -dD`: exit 1".to_owned());
            }
            command
        })
        .collect();
    let covered = fixture
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    let facts = fixture_facts(&database, commands, &["_WIN32", "__linux__"], &covered);
    PredefinedMacros::derive(&facts, &database, &fixture.root).expect("the facts match")
}

fn run_configured(fixture: &Fixture, predefined: Option<PredefinedMacros>) -> Census {
    let typed = facts(fixture);
    let obligations = loop_owner_obligations(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads");
    let configuration = BuildConfiguration {
        units: fixture
            .files
            .iter()
            .filter(|file| file.path.ends_with(".c"))
            .map(|file| (file.path.clone(), BTreeSet::new()))
            .collect(),
        owned_prefixes: vec!["FLB_".to_owned()],
        defined_elsewhere: BTreeSet::new(),
        predefined,
        region_macros: None,
    };
    census_typed(
        &fixture.files,
        &obligations,
        &FLUENT_BIT_HOPS,
        Some(&configuration),
        Some(&typed),
    )
}

fn windows_arms_fixture(name: &str) -> Fixture {
    make_fixture(
        name,
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/wild.c", WILD), ("src/service.c", WINDOWS_ARMS)],
    )
}

#[test]
fn nc_the_fixed_shape_with_windows_arms_undecided_is_unresolved() {
    let fixture = windows_arms_fixture("win-undecided");
    let census = run_configured(&fixture, None);
    route_kept(&census, "have no typed reading");
    let (_, reason) = flag(&census);
    assert!(reason.contains("platform_localtime"), "{reason}");
    assert!(reason.contains("platform_dirname"), "{reason}");
}

#[test]
fn nc_the_fixed_shape_is_unresolved_where_the_compiler_defines_win32() {
    // A derived set that defines `_WIN32` keeps both Windows arms.
    let fixture = windows_arms_fixture("win-defined");
    let view = predefined(&fixture, &["_WIN32"], &[], None);
    route_kept(
        &run_configured(&fixture, Some(view)),
        "have no typed reading",
    );
}

#[test]
fn nc_the_fixed_shape_is_unresolved_where_a_file_defines_win32() {
    // Some preprocessed file `#define`s `_WIN32`: nothing is decided.
    let fixture = windows_arms_fixture("win-file-defined");
    let view = predefined(&fixture, &["__linux__"], &["_WIN32"], None);
    route_kept(
        &run_configured(&fixture, Some(view)),
        "have no typed reading",
    );
}

#[test]
fn nc_the_fixed_shape_is_unresolved_where_the_derivation_failed() {
    let fixture = windows_arms_fixture("win-failed");
    let view = predefined(&fixture, &["__linux__"], &[], Some("src/service.c"));
    route_kept(
        &run_configured(&fixture, Some(view)),
        "have no typed reading",
    );
}

#[test]
fn the_fixed_shape_discharges_when_the_derived_set_decides_the_windows_arms_out() {
    let fixture = windows_arms_fixture("win-decided");
    let view = predefined(&fixture, &["__linux__"], &[], None);
    let census = run_configured(&fixture, Some(view));
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(reason.contains("list-head instance"), "{reason}");
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
    let decided = &census.reading.predefined_decisions["src/service.c"];
    assert!(decided.contains("__linux__=defined"), "{decided:?}");
    assert!(decided.contains("_WIN32=undefined"), "{decided:?}");
}

#[test]
fn nc_the_old_test_shape_stays_refused_under_the_derived_set() {
    let old = r#"
#include "fluent-bit/flb_input.h"
struct test_ctx { struct flb_lib_ctx *flb; int i_ffd; };
static struct test_ctx *test_ctx_create(void)
{
    struct test_ctx *ctx = flb_calloc(1, sizeof(struct test_ctx));
    if (!ctx) {
        return NULL;
    }
    ctx->flb = flb_create();
    ctx->i_ffd = flb_input(ctx->flb, (char *) "tail", NULL);
    return ctx;
}
void flb_test_inotify_pause_collectors(void)
{
    struct mk_list *head;
    struct flb_input_instance *ins;
    struct test_ctx *ctx;
    ctx = test_ctx_create();
    if (!ctx) {
        return;
    }
    flb_start(ctx->flb);
    head = ctx->flb->config->inputs.next;
    ins = mk_list_entry(head, struct flb_input_instance, _head);
    flb_input_collector_running(1, ins);
    flb_input_pause(ins);
    flb_input_collector_running(1, ins);
}
TEST_LIST = {
    { "inotify_pause_collectors", flb_test_inotify_pause_collectors },
    { 0, 0 }
};
"#;
    let fixture = make_fixture(
        "old-win",
        old,
        &[("src/wild.c", WILD), ("src/service.c", WINDOWS_ARMS)],
    );
    let view = predefined(&fixture, &["__linux__"], &[], None);
    let census = run_configured(&fixture, Some(view));
    for verdict in verdicts(&census, "flb_test_inotify_pause_collectors") {
        assert!(
            matches!(verdict, OwnerVerdict::NotDischarged { context, .. } if context.thread == "T"),
            "{verdict:?}"
        );
    }
}
