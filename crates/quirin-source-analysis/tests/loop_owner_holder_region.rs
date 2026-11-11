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

//! W47-PREMISES-HOLDER-REGION — the list-head region's macro invocations read
//! from their units' own expansions (Clang's token dump under each recorded
//! command), and P-HOLDER-OPAQUE derived from every read of the instance's
//! user pointer. The fixtures are W47-LOOP-OWNER-INSTANCE-FLOW's Fluent Bit
//! shapes (`loop_owner_instance_flow.rs`), compiled by the real Clang (the
//! typed reading and the token dump). Negative controls first: a region macro
//! that expands to a hazard-reaching call is refused; of two headers defining
//! one macro name, the one the unit includes wins; a region file without
//! facts, or read from another text, is refused; a `->data` reader that writes
//! the flag or a holder byte keeps the route.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::predefined_macros::identify;
use qcore_c_gateway::thread_entry_census::region_macros::{
    FactsRequest, RegionMacros, derive_facts, dumped_token,
};
use qcore_c_gateway::thread_entry_census::{
    BuildConfiguration, CensusFile, FLUENT_BIT_INSTANCE_FLOW,
};
use qcore_c_gateway::typed_dispatch_census::{TypedFacts, TypedUnit, read_units};
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
        std::env::temp_dir().join(format!("qcore-holder-region-{}-{name}", std::process::id()));
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

// ---------------------------------------------------------------------------
// Card W47-PREMISES-HOLDER-REGION.

/// The gateway's Clang as an absolute path (a recorded command names its
/// compiler by path).
fn clang() -> String {
    let program = PathBuf::from(
        qcore_c_gateway::clang_command()
            .expect("Clang is available")
            .get_program(),
    );
    if program.is_absolute() {
        return program.display().to_string();
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(&program))
        .find(|candidate| candidate.is_file())
        .expect("the gateway's Clang is on PATH")
        .display()
        .to_string()
}

/// Write files the census does not read (a header outside its paths, as the
/// test's `tests/lib/acutest/acutest.h` is).
fn write_unread(fixture: &Fixture, files: &[(&str, &str)]) {
    for (path, text) in files {
        let full = fixture.root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("fixture directory");
        fs::write(&full, text).expect("fixture file");
    }
}

/// The fixture's compile database: each `.c` file once with the real Clang.
fn database(fixture: &Fixture, compiler: &str) -> String {
    let entries = fixture
        .files
        .iter()
        .filter(|file| file.path.ends_with(".c"))
        .map(|file| {
            serde_json::json!({
                "directory": fixture.root.display().to_string(),
                "file": fixture.root.join(&file.path).display().to_string(),
                "arguments": [
                    compiler,
                    "-std=gnu17",
                    format!("-I{}", fixture.root.join("include").display()),
                    "-c",
                    fixture.root.join(&file.path).display().to_string(),
                    "-o",
                    format!("{}.o", file.path),
                ],
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&entries).expect("database JSON")
}

/// The region-macro facts of `only` (every `.c` file when empty), derived by
/// the real Clang, and the database they were derived from.
fn region_facts(fixture: &Fixture, only: &[&str]) -> (RegionMacros, String) {
    let compiler = clang();
    let database = database(fixture, &compiler);
    let recorded = identify(&compiler).version;
    let only = if only.is_empty() {
        fixture
            .files
            .iter()
            .filter(|file| file.path.ends_with(".c"))
            .map(|file| file.path.clone())
            .collect()
    } else {
        only.iter().map(|path| (*path).to_owned()).collect()
    };
    let facts = derive_facts(&FactsRequest {
        database_text: database.clone(),
        root: fixture.root.clone(),
        recorded_compiler_version: recorded,
        only,
        extras: Vec::new(),
        census_files: fixture.files.iter().map(|f| f.path.clone()).collect(),
        jobs: 2,
    })
    .expect("facts");
    for unit in &facts.units {
        assert_eq!(unit.failure, None, "{unit:?}");
    }
    let view = RegionMacros::derive(&facts, &database, &fixture.root).expect("the facts match");
    (view, database)
}

fn run_region(fixture: &Fixture, region: Option<RegionMacros>) -> Census {
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
        predefined: None,
        region_macros: region,
    };
    census_typed(
        &fixture.files,
        &obligations,
        &FLUENT_BIT_HOPS,
        Some(&configuration),
        Some(&typed),
    )
}

/// The census's header with the fixtures' macros: a check that calls a
/// harmless reporter, and one that creates an input.
const MACROS: &str = r#"
#ifndef MACROS_H
#define MACROS_H
#include "fluent-bit/flb_input.h"
int report(int ok);
#define CHECK(c) report((c) != 0)
#define MAKE_CORO(c) flb_input((c), (char *) "coro", NULL)
#define TWICE(x) (x); (x)
#define BAIL(c) if (!(c)) return NULL
#define CALL_PRE(i) (i)->p->cb_pre_run((i), 0, 0)
#define HELPER(c) unread_helper_((c))
#endif
"#;

/// A header the census does not read, defining `CHECK` harmlessly and a
/// helper that creates an input (as acutest's own functions sit outside the
/// census's files).
const UNREAD: &str = r#"
#ifndef UNREAD_H
#define UNREAD_H
#include "fluent-bit/flb_input.h"
static int unread_check_(int ok) { return ok; }
static void unread_helper_(struct flb_lib_ctx *c) { flb_input(c, (char *) "coro", NULL); }
#define CHECK(c) unread_check_((c) != 0)
#endif
"#;

const REPORT: &str = r#"
int report(int ok)
{
    return ok;
}
"#;

/// The fixed shape with `#include`s and a maker line.
fn with_macros(include: &str, maker_line: &str, setup: &str) -> String {
    let maker = format!("{ONE_TAIL}\n{maker_line}");
    format!("{include}\n{}", test_file(&maker, setup, "", ""))
}

fn macro_fixture(name: &str, include: &str, maker_line: &str, setup: &str) -> Fixture {
    let fixture = make_fixture(
        &format!("region-{name}"),
        &with_macros(include, maker_line, setup),
        &[("include/macros.h", MACROS), ("src/report.c", REPORT)],
    );
    write_unread(&fixture, &[("tests/lib/unread.h", UNREAD)]);
    fixture
}

const INCLUDE_CENSUS: &str = "#include \"macros.h\"";
const INCLUDE_BOTH: &str = "#include \"macros.h\"\n#undef CHECK\n#include \"../lib/unread.h\"";

#[test]
fn a_dump_line_reads_its_expansion_and_spelling() {
    let token = dumped_token("identifier 'check_'\t\tLoc=</r/t.c:5:10 <Spelling=/r/h.h:1:18>>")
        .expect("a token");
    assert_eq!(token.kind, "identifier");
    assert_eq!(token.text, "check_");
    let plain = dumped_token("r_paren ')'\t\tLoc=</r/t.c:5:23>").expect("a token");
    assert_eq!(plain.text, ")");
    let literal = dumped_token(
        "string_literal '\"a != 0\"'\t [LeadingSpace]\tLoc=</r/t.c:5:10 <Spelling=<scratch space>:4:1>>",
    )
    .expect("a token");
    assert_eq!(literal.text, "\"a != 0\"");
    assert_eq!(dumped_token("In file included from x.c:1:"), None);
    // A raw spelling across a line splice, joined by the reader.
    let unclean = dumped_token("r_brace '}'\t [LeadingSpace] [UnClean='\\\n}']\tLoc=</r/t.c:3:1>")
        .expect("a token");
    assert_eq!(unclean.text, "}");
}

#[test]
fn a_unit_dump_marks_replacement_and_argument_tokens() {
    // Nested, pasted and multi-line invocations, read from the real Clang.
    let root =
        std::env::temp_dir().join(format!("qcore-holder-region-{}-dump", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("lib")).expect("fixture directory");
    fs::write(
        root.join("lib/h.h"),
        "#define CHECK(c) check_((c), \"%s\", #c)\n#define INNER(x) inner_(x)\n\
         #define OUTER(y) do { INNER(y); } while (0)\n#define CAT(a, b) a##b\n\
         static int check_(int c, const char *f, ...) { return report_(c); }\n",
    )
    .expect("fixture file");
    fs::write(
        root.join("t.c"),
        "#include \"lib/h.h\"\nint inner_(int);\nint report_(int);\nint g(int);\n\
         void f(int a)\n{\n    OUTER(g(a)); CAT(in, ner_)(a);\n    CHECK(\n      g(2));\n}\n",
    )
    .expect("fixture file");
    let compiler = clang();
    let database = serde_json::to_string(&vec![serde_json::json!({
        "directory": root.display().to_string(),
        "file": root.join("t.c").display().to_string(),
        "arguments": [compiler, "-c", root.join("t.c").display().to_string()],
    })])
    .expect("database JSON");
    let facts = derive_facts(&FactsRequest {
        database_text: database,
        root: root.clone(),
        recorded_compiler_version: identify(&compiler).version,
        only: BTreeSet::from(["t.c".to_owned()]),
        extras: Vec::new(),
        census_files: BTreeSet::from(["t.c".to_owned()]),
        jobs: 1,
    })
    .expect("facts");
    let _ = fs::remove_dir_all(&root);
    let unit = &facts.units[0];
    assert_eq!(unit.failure, None);
    let names = unit
        .invocations
        .iter()
        .map(|i| (i.line, i.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(names, [(7, "OUTER"), (7, "CAT"), (8, "CHECK")]);
    let body = |i: usize| {
        unit.invocations[i]
            .tokens
            .iter()
            .filter(|t| t.identifier && t.body)
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
    };
    let arguments = |i: usize| {
        unit.invocations[i]
            .tokens
            .iter()
            .filter(|t| t.identifier && !t.body)
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
    };
    // `OUTER` expands `INNER` (nested) around its argument.
    assert_eq!(body(0), ["inner_"]);
    assert_eq!(arguments(0), ["g", "a"]);
    // A pasted name is the replacement list's.
    assert_eq!(body(1), ["inner_"]);
    // The argument on the next line stays an argument.
    assert_eq!(body(2), ["check_"]);
    assert_eq!(arguments(2), ["g"]);
    // `check_` is the unit's own, outside the census's files, read through
    // its body's names.
    let check = &unit.definitions["check_"];
    assert_eq!(check.file, "lib/h.h");
    assert!(
        !check.census && check.names.contains("report_"),
        "{check:?}"
    );
}

#[test]
fn nc_a_region_macro_that_expands_to_a_creation_is_refused() {
    // Without the facts the lexical region does not see the second creation
    // the macro makes (the premise this card derives).
    let fixture = macro_fixture("creation", INCLUDE_CENSUS, "    MAKE_CORO(ctx->flb);", "");
    let without = run_region(&fixture, None);
    assert_eq!(flag(&without).0, Some(false), "{}", flag(&without).1);
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "the macro `MAKE_CORO` invoked at tests/runtime/in_tail.c",
    );
}

#[test]
fn nc_a_macro_repeating_a_creation_in_its_arguments_is_refused() {
    // The lexical region counts the one creation it sees; the expansion
    // makes two.
    let maker = "    TWICE(ctx->i_ffd = flb_input(ctx->flb, (char *) \"tail\", NULL));";
    let fixture = make_fixture(
        "region-twice",
        &format!("{INCLUDE_CENSUS}\n{}", test_file(maker, "", "", "")),
        &[("include/macros.h", MACROS), ("src/report.c", REPORT)],
    );
    let without = run_region(&fixture, None);
    assert_eq!(flag(&without).0, Some(false), "{}", flag(&without).1);
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(&run_region(&fixture, Some(view)), "carries `flb_input`");
}

#[test]
fn nc_a_macro_that_jumps_is_refused() {
    let fixture = macro_fixture("jump", INCLUDE_CENSUS, "    BAIL(ctx->flb);", "");
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "expands to a jump this reading does not model",
    );
}

#[test]
fn nc_a_macro_field_dispatch_is_refused() {
    let fixture = macro_fixture(
        "dispatch",
        INCLUDE_CENSUS,
        "",
        "    { struct flb_input_instance *first = 0; CALL_PRE(first); }",
    );
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "expands to a field dispatch",
    );
}

#[test]
fn nc_a_macro_made_call_in_the_closure_is_refused() {
    // The service hook, reached from the maker's `flb_service_set`, creates an
    // input through a macro: the census graph holds no edge for it.
    let service = r#"
#include "fluent-bit/flb_input.h"
#include "macros.h"
void service_hook(struct flb_lib_ctx *ctx)
{
    MAKE_CORO(ctx);
}
"#;
    let fixture = make_fixture(
        "region-closure",
        &with_macros(INCLUDE_CENSUS, "", ""),
        &[
            ("include/macros.h", MACROS),
            ("src/report.c", REPORT),
            ("src/service.c", service),
        ],
    );
    let without = run_region(&fixture, None);
    assert_eq!(flag(&without).0, Some(false), "{}", flag(&without).1);
    // Facts for the region's files alone: the hook's file is read by every
    // read file's definition of the macro.
    let (view, _) = region_facts(&fixture, &["tests/runtime/in_tail.c", "src/flb_lib.c"]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "through a macro-made call the census graph does not hold",
    );
    // With the hook's own unit read, its expansion says the same.
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "through a macro-made call the census graph does not hold",
    );
}

#[test]
fn nc_a_macro_made_dispatch_in_the_closure_is_refused() {
    // The hook dispatches through a field in a macro; a registrant of that
    // field creates an input.
    let service = r#"
#include "fluent-bit/flb_input.h"
#define INIT_ALL(i) (i)->p->cb_init((i), 0, 0)
void service_hook(struct flb_lib_ctx *ctx)
{
    struct flb_input_instance *first = 0;
    INIT_ALL(first);
}
"#;
    let creator = r#"
#include "fluent-bit/flb_input.h"
static int in_make_init(struct flb_input_instance *ins, void *config, void *data)
{
    flb_input((struct flb_lib_ctx *) data, (char *) "coro", NULL);
    return 0;
}
struct flb_input_plugin in_make_plugin = {
    .name = "make",
    .cb_init = in_make_init,
    .flags = 0
};
"#;
    let fixture = make_fixture(
        "region-closure-dispatch",
        &with_macros(INCLUDE_CENSUS, "", ""),
        &[
            ("include/macros.h", MACROS),
            ("src/report.c", REPORT),
            ("src/service.c", service),
            ("plugins/in_make/make.c", creator),
        ],
    );
    let without = run_region(&fixture, None);
    assert_eq!(flag(&without).0, Some(false), "{}", flag(&without).1);
    let (view, _) = region_facts(&fixture, &["tests/runtime/in_tail.c", "src/flb_lib.c"]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "(macro) plugins/in_make/make.c::in_make_init",
    );
}

#[test]
fn nc_a_unit_function_outside_the_census_that_creates_is_refused() {
    let fixture = macro_fixture("unread-helper", INCLUDE_BOTH, "    HELPER(ctx->flb);", "");
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(&run_region(&fixture, Some(view)), "names `flb_input`");
}

#[test]
fn nc_of_two_definitions_the_included_one_wins() {
    // The census reads `include/macros.h`, whose `CHECK` is harmless, and
    // not `tests/lib/unread.h`; a hazardous `CHECK` in a census header the
    // unit does not include is ignored, and the unit's own is read.
    let hazardous = r#"
#ifndef OTHER_H
#define OTHER_H
#include "fluent-bit/flb_input.h"
#define CHECK(c) flb_input((struct flb_lib_ctx *) 0, (char *) "coro", NULL)
#endif
"#;
    let fixture = make_fixture(
        "region-two-defs",
        &with_macros(INCLUDE_BOTH, "    CHECK(ctx->flb != NULL);", ""),
        &[
            ("include/macros.h", MACROS),
            ("include/other.h", hazardous),
            ("src/report.c", REPORT),
        ],
    );
    write_unread(&fixture, &[("tests/lib/unread.h", UNREAD)]);
    let (view, _) = region_facts(&fixture, &[]);
    let census = run_region(&fixture, Some(view));
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(reason.contains("region macros: "), "{reason}");
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
    // The converse: the unit includes the hazardous definition.
    let fixture = make_fixture(
        "region-two-defs-bad",
        &with_macros(
            "#include \"macros.h\"\n#undef CHECK\n#include \"other.h\"",
            "    CHECK(ctx->flb != NULL);",
            "",
        ),
        &[
            ("include/macros.h", MACROS),
            ("include/other.h", hazardous),
            ("src/report.c", REPORT),
        ],
    );
    let (view, _) = region_facts(&fixture, &[]);
    route_kept(&run_region(&fixture, Some(view)), "the macro `CHECK`");
}

#[test]
fn nc_a_region_file_without_facts_is_refused() {
    let fixture = macro_fixture("no-facts", INCLUDE_CENSUS, "    CHECK(1);", "");
    let (view, _) = region_facts(&fixture, &["src/flb_lib.c"]);
    route_kept(
        &run_region(&fixture, Some(view)),
        "no unit's expansion facts read tests/runtime/in_tail.c",
    );
}

#[test]
fn nc_facts_of_another_text_are_refused() {
    let mut fixture = macro_fixture("other-text", INCLUDE_CENSUS, "    CHECK(1);", "");
    let (view, _) = region_facts(&fixture, &[]);
    let test = fixture
        .files
        .iter_mut()
        .find(|f| f.path == "tests/runtime/in_tail.c")
        .expect("the test");
    test.text.push_str("\n/* edited */\n");
    route_kept(&run_region(&fixture, Some(view)), "read another text");
}

#[test]
fn the_fixed_shape_with_region_facts_discharges() {
    let fixture = macro_fixture(
        "fixed",
        INCLUDE_BOTH,
        "    CHECK(ctx->flb != NULL);",
        "    CHECK(1);",
    );
    let (view, _) = region_facts(&fixture, &[]);
    let census = run_region(&fixture, Some(view));
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(reason.contains("region macros: "), "{reason}");
    assert!(reason.contains("P-HOLDER-OPAQUE derived"), "{reason}");
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
}

#[test]
fn nc_the_old_test_shape_with_region_facts_is_not_discharged() {
    let old = r#"
#include "macros.h"
struct test_ctx { struct flb_lib_ctx *flb; int i_ffd; };
static struct test_ctx *test_ctx_create(void)
{
    struct test_ctx *ctx = flb_calloc(1, sizeof(struct test_ctx));
    if (!ctx) {
        return NULL;
    }
    ctx->flb = flb_create();
    ctx->i_ffd = flb_input(ctx->flb, (char *) "tail", NULL);
    CHECK(ctx->i_ffd >= 0);
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
        "region-old",
        old,
        &[("include/macros.h", MACROS), ("src/report.c", REPORT)],
    );
    let (view, _) = region_facts(&fixture, &[]);
    let census = run_region(&fixture, Some(view));
    for verdict in verdicts(&census, "flb_test_inotify_pause_collectors") {
        assert!(
            matches!(verdict, OwnerVerdict::NotDischarged { context, .. } if context.thread == "T"),
            "{verdict:?}"
        );
    }
}

// P-HOLDER-OPAQUE.

/// The library's init dispatch: the instance's user pointer handed to the
/// plugin's init callback (`src/flb_input.c:2019`).
const INIT: &str = r#"
#include "fluent-bit/flb_input.h"
int flb_input_init_all(struct flb_input_instance *ins, void *config)
{
    return ins->p->cb_init(ins, config, ins->data);
}
"#;

/// An input plugin named `name` whose init callback runs `body`.
fn init_plugin(name: &str, body: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
void *memset(void *s, int c, unsigned long n);
static int in_init_cb(struct flb_input_instance *ins, void *config, void *data)
{{
{body}
    return 0;
}}
struct flb_input_plugin in_init_plugin = {{
    .name = "{name}",
    .cb_init = in_init_cb,
    .flags = 0
}};
"#
    )
}

fn holder_fixture(name: &str, extra: &[(&str, &str)]) -> Fixture {
    make_fixture(
        &format!("holder-{name}"),
        &test_file(ONE_TAIL, "", "", ""),
        extra,
    )
}

#[test]
fn nc_a_data_reader_that_writes_the_flag_keeps_the_route() {
    let reader = r#"
#include "fluent-bit/flb_input.h"
void force(struct flb_input_instance *ins)
{
    if (ins->data != NULL) {
        ins->runs_in_coroutine = 1;
    }
}
"#;
    let fixture = holder_fixture("flag", &[("src/reader.c", reader)]);
    route_kept(
        &run_region(&fixture, None),
        "written other than by `flb_input_new` alone",
    );
}

#[test]
fn nc_a_data_reader_that_writes_a_holder_byte_keeps_the_route() {
    let reader = r#"
#include "fluent-bit/flb_input.h"
void peek(struct flb_input_instance *ins)
{
    struct flb_input_plugin *p = (struct flb_input_plugin *) ins->data;
    p->flags = FLB_INPUT_CORO;
}
"#;
    let fixture = holder_fixture("write", &[("src/reader.c", reader)]);
    route_kept(
        &run_region(&fixture, None),
        "a holder byte is written or its address taken through it",
    );
}

// The tail instance's init is its copy's: a second descriptor named "tail"
// (matched ignoring case, as the creation matches) whose init misuses its data.

#[test]
fn nc_an_init_callback_that_hands_its_data_to_a_library_function_keeps_the_route() {
    let plugin = init_plugin("tail", "    memset(data, 0, 4);");
    let fixture = holder_fixture(
        "memset",
        &[("src/init.c", INIT), ("plugins/in_init/init.c", &plugin)],
    );
    route_kept(
        &run_region(&fixture, None),
        "passed to `memset`, which this reading does not read",
    );
}

#[test]
fn nc_an_init_callback_that_stores_its_data_keeps_the_route() {
    let plugin = init_plugin("tail", "    ins->context = data;");
    let fixture = holder_fixture(
        "store",
        &[("src/init.c", INIT), ("plugins/in_init/init.c", &plugin)],
    );
    route_kept(&run_region(&fixture, None), "stored in a field");
}

#[test]
fn an_init_callback_and_a_reader_that_only_read_derive_the_holder_opaque() {
    let plugin = init_plugin(
        "tail",
        "    struct cbs { int (*pause)(void *, void *); void *data; } *c = data;\n    \
         (void) data;\n    if (c != NULL && c->pause != NULL) {\n        c->pause(c->data, config);\n    }",
    );
    let fixture = holder_fixture(
        "readers",
        &[("src/init.c", INIT), ("plugins/in_init/init.c", &plugin)],
    );
    let census = run_region(&fixture, None);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(reason.contains("P-HOLDER-OPAQUE derived"), "{reason}");
    assert!(reason.contains("flb_input_init_all"), "{reason}");
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
}

#[test]
fn another_plugins_init_is_not_the_holder_instances_callback() {
    // `ins->p->cb_init(ins, …, ins->data)` on the holder's instance runs its
    // copy's `cb_init`; another descriptor's init never receives the holder.
    let plugin = init_plugin("init", "    ins->context = data;\n    memset(data, 0, 4);");
    let fixture = holder_fixture(
        "other-init",
        &[("src/init.c", INIT), ("plugins/in_init/init.c", &plugin)],
    );
    let census = run_region(&fixture, None);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(
        reason.contains("1 read(s) passed to the instance's own callback, the copy's"),
        "{reason}"
    );
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
}
