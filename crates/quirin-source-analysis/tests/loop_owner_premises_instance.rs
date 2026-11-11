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

//! W47-PREMISES-INSTANCE — the list-head instance's premises derived from
//! the program, over compilable fixtures of the Fluent Bit shapes at
//! `90978ac6`: `flb_create` storing `flb_config_init()`'s fresh config
//! (`src/flb_lib.c:145`, `src/flb_config.c:281-577`), the generated
//! `flb_plugins_register` (`generated/include/fluent-bit/flb_plugins.h`),
//! `flb_input_new`'s allocation and append (`src/flb_input.c:326`, `:677`),
//! the loop dispatch `C->cb_collect(C->instance, …)` (`:3321`) and the fixed
//! test's callback (`tests/runtime/in_tail.c:2475-2501`). Each fixture is
//! compiled by Clang (the typed reading, with the premise facts) and read by
//! the premised census. Negative controls first.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::{
    CensusFile, FLUENT_BIT_INSTANCE_FLOW, FLUENT_BIT_INSTANCE_PREMISES, FLUENT_BIT_PROPERTY_KEYS,
    GeneratedFile, PremiseInputs, SitePremise,
};
use qcore_c_gateway::typed_dispatch_census::{TypedFacts, TypedUnit, read_units};
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::{Mode, OwnerVerdict};
use quirin_source_analysis::loop_owner_census::{
    Census, FLUENT_BIT_HOPS, census_premised, census_typed,
};

const CONTRACTS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
);

/// W47-LOOP-OWNER-INSTANCE-FLOW's input header, with the allocator's
/// primitive, the generated registration's calls and `mk_list_del`.
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
static inline void mk_list_del(struct mk_list *entry)
{
    entry->prev->next = entry->next;
    entry->next->prev = entry->prev;
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
void *calloc(unsigned long n, unsigned long size);
void *flb_calloc(unsigned long n, unsigned long size);
void *flb_malloc(unsigned long size);
void flb_errno(void);
void *memcpy(void *dst, const void *src, unsigned long n);
int flb_plugins_register(struct flb_config *config);
struct flb_plugins_extra;
void flb_plugins_reload(struct flb_config *config);
extern struct flb_input_instance *spare_dispatch_instance;
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

/// W47-LOOP-OWNER-INSTANCE-FLOW's library with the export's shapes of
/// `flb_calloc`, `flb_config_init`, `flb_create` and `flb_input_new`; the
/// `@…@` holes are filled by [`Shape`].
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
        collector->cb_collect(@DISPATCHED@, config, collector->instance->context);
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
void *flb_calloc(unsigned long n, unsigned long size)
{
    if (size == 0) {
        return NULL;
    }
    return calloc(n, size);
}
struct flb_config *flb_config_init(void)
{
    int ret;
    struct flb_config *config;
    config = flb_calloc(1, sizeof(struct flb_config));
    if (!config) {
        return NULL;
    }
@INIT_LISTS@
    ret = flb_plugins_register(config);
    if (ret == -1) {
        return NULL;
    }
@INIT_EXTRA@
    return config;
}
struct flb_lib_ctx *flb_create(void)
{
    struct flb_lib_ctx *ctx;
    struct flb_config *config;
    ctx = flb_calloc(1, sizeof(struct flb_lib_ctx));
    if (!ctx) {
        return NULL;
    }
    config = flb_config_init();
    if (!config) {
        flb_free(ctx);
        return NULL;
    }
    ctx->config = config;
    return ctx;
}
struct flb_input_instance *flb_input_new(struct flb_config *config, const char *input, void *data, int public_only)
{
    struct flb_input_plugin *plugin = 0;
    struct flb_input_instance *instance = NULL;
    if (!input) {
        return NULL;
    }
    instance = flb_calloc(1, sizeof(struct flb_input_instance));
    if (!instance) {
        return NULL;
    }
@NEW_EXTRA@
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

/// One registration block of the generated `flb_plugins_register`.
fn block(object: &str) -> String {
    format!(
        "    in = flb_malloc(sizeof(struct flb_input_plugin));\n    if (!in) {{\n        \
         flb_errno();\n        return -1;\n    }}\n    memcpy(in, &{object}, sizeof(struct \
         flb_input_plugin));\n    mk_list_add(&in->_head, &config->in_plugins);\n\n"
    )
}

/// The generated registration file.
fn generated(objects: &[&str], extra: &str) -> String {
    format!(
        "#ifndef FLB_PLUGINS_H\n#define FLB_PLUGINS_H\n#include \"fluent-bit/flb_input.h\"\n\
         extern struct flb_input_plugin in_tail_plugin;\n\
         extern struct flb_input_plugin in_coro_plugin;\n\n\
         int flb_plugins_register(struct flb_config *config)\n{{\n    \
         struct flb_input_plugin *in;\n\n{}    return 0;\n}}\n{extra}\n#endif\n",
        objects.iter().map(|o| block(o)).collect::<String>()
    )
}

/// What a fixture varies: the library's holes, the generated registration
/// and who resolves it.
struct Shape {
    init_lists: String,
    init_extra: String,
    new_extra: String,
    dispatched: String,
    registered: Vec<&'static str>,
    generated_extra: String,
    included_by: BTreeSet<String>,
    with_generated: bool,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            init_lists:
                "    mk_list_init(&config->in_plugins);\n    mk_list_init(&config->inputs);"
                    .to_owned(),
            init_extra: String::new(),
            new_extra: String::new(),
            dispatched: "collector->instance".to_owned(),
            registered: vec!["in_tail_plugin", "in_coro_plugin"],
            generated_extra: String::new(),
            included_by: BTreeSet::from(["src/flb_lib.c".to_owned()]),
            with_generated: true,
        }
    }
}

/// The fixed test's callbacks and holder (W47-LOOP-OWNER-INSTANCE-FLOW's
/// fixture), with `callback` inserted at the start of the collector callback.
fn test_file(maker: &str, setup: &str, callback: &str, extra: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
struct tail_pause_test {{ struct flb_input_plugin plugin; struct flb_input_plugin *original_plugin; int collector_id; }};
struct test_ctx {{ struct flb_lib_ctx *flb; int i_ffd; }};
static struct flb_input_instance *spare_instance;
static int cb_tail_pause_test(struct flb_input_instance *ins, void *config, void *data)
{{
    struct tail_pause_test *test = ins->data;
{callback}
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
    premises: PremiseInputs,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_fixture(name: &str, shape: &Shape, test: &str, extra: &[(&str, &str)]) -> Fixture {
    let root = std::env::temp_dir().join(format!("qcore-premises-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let library = LIBRARY
        .replace("@INIT_LISTS@", &shape.init_lists)
        .replace("@INIT_EXTRA@", &shape.init_extra)
        .replace("@NEW_EXTRA@", &shape.new_extra)
        .replace("@DISPATCHED@", &shape.dispatched);
    let mut sources = vec![
        ("include/fluent-bit/flb_input.h", INPUT_HEADER.to_owned()),
        ("src/flb_lib.c", library),
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
    let premises = PremiseInputs {
        generated: if shape.with_generated {
            vec![GeneratedFile {
                path: "generated/include/fluent-bit/flb_plugins.h".to_owned(),
                text: generated(&shape.registered, &shape.generated_extra),
                included_by: shape.included_by.clone(),
            }]
        } else {
            Vec::new()
        },
    };
    Fixture {
        root,
        files,
        premises,
    }
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
        &FLUENT_BIT_HOPS.premise_options(
            &FLUENT_BIT_INSTANCE_FLOW,
            &FLUENT_BIT_PROPERTY_KEYS,
            &FLUENT_BIT_INSTANCE_PREMISES,
        ),
        2,
    );
    assert!(facts.failures.is_empty(), "{:#?}", facts.failures);
    facts
}

fn obligations() -> BTreeMap<String, qcore_contract::contract_loader::DeclaredLoopOwner> {
    loop_owner_obligations(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads")
}

fn run(fixture: &Fixture) -> Census {
    let typed = facts(fixture);
    census_premised(
        &fixture.files,
        &obligations(),
        &FLUENT_BIT_HOPS,
        None,
        Some(&typed),
        Some(&fixture.premises),
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

/// P-INSTANCE at the callback's collector operations.
fn site_premises(census: &Census) -> Vec<SitePremise> {
    census
        .reading
        .premises
        .as_ref()
        .expect("the premised census reports")
        .instance_sites
        .iter()
        .filter(|(site, _)| site.starts_with("tests/runtime/in_tail.c:"))
        .map(|(_, premise)| premise.clone())
        .collect()
}

fn fixed(name: &str, shape: &Shape, extra: &[(&str, &str)]) -> Census {
    let fixture = make_fixture(name, shape, &test_file(ONE_TAIL, "", "", ""), extra);
    run(&fixture)
}

// --- P-REGISTERED-DESCRIPTORS -------------------------------------------

#[test]
fn nc_no_generated_registration_leaves_the_head_underived() {
    let shape = Shape {
        with_generated: false,
        ..Shape::default()
    };
    route_kept(
        &fixed("no-generated", &shape, &[]),
        "no generated file defines `flb_plugins_register`",
    );
}

#[test]
fn nc_a_registration_the_includer_does_not_resolve_is_underived() {
    let shape = Shape {
        included_by: BTreeSet::new(),
        ..Shape::default()
    };
    route_kept(
        &fixed("unresolved-include", &shape, &[]),
        "resolves its include to the generated",
    );
}

#[test]
fn nc_a_literal_no_registered_descriptor_matches_is_underived() {
    let shape = Shape {
        registered: vec!["in_coro_plugin"],
        ..Shape::default()
    };
    route_kept(
        &fixed("unregistered", &shape, &[]),
        "no registered descriptor matches \"tail\"",
    );
}

#[test]
fn nc_a_descriptor_added_at_run_time_on_the_path_is_unresolved() {
    // A generated function registering at run time, called on the path.
    let shape = Shape {
        generated_extra: "void flb_plugins_reload(struct flb_config *config)\n{\n    \
                          mk_list_add(&in_tail_plugin._head, &config->in_plugins);\n}\n"
            .to_owned(),
        ..Shape::default()
    };
    let service = r#"
#include "fluent-bit/flb_input.h"
void service_hook(struct flb_lib_ctx *ctx)
{
    flb_plugins_reload(ctx->config);
}
"#;
    route_kept(
        &fixed("runtime-descriptor", &shape, &[("src/service.c", service)]),
        "more than one call reaches a hazard",
    );
}

#[test]
fn nc_an_internal_input_created_before_the_tests_is_unresolved() {
    let shape = Shape {
        init_extra: "    flb_input_new(config, \"coro\", NULL, 0);".to_owned(),
        ..Shape::default()
    };
    let census = fixed("internal-input", &shape, &[]);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, None, "{reason}");
    assert!(
        reason.contains("reaches a hazard") || reason.contains("creates an input instance"),
        "{reason}"
    );
    unresolved_with(&verdicts(&census, "cb_tail_pause_test"), "may yield");
}

#[test]
fn nc_a_second_flb_input_call_is_unresolved() {
    let maker = format!("{ONE_TAIL}\n    flb_input(ctx->flb, (char *) \"tail\", NULL);");
    let fixture = make_fixture(
        "second-input",
        &Shape::default(),
        &test_file(&maker, "", "", ""),
        &[],
    );
    route_kept(&run(&fixture), "more than one call reaches a hazard");
}

// --- P-FRESH-CONTEXT and P-HEAD-INSTANCE --------------------------------

#[test]
fn nc_a_list_initialised_on_one_branch_is_not_fresh() {
    let shape = Shape {
        init_lists: "    mk_list_init(&config->in_plugins);\n    if (ret) {\n        \
                     mk_list_init(&config->inputs);\n    }"
            .to_owned(),
        ..Shape::default()
    };
    route_kept(
        &fixed("branch-init", &shape, &[]),
        "P-FRESH-CONTEXT is not derived",
    );
}

#[test]
fn nc_an_instance_unlinked_on_the_path_is_unresolved() {
    let service = r#"
#include "fluent-bit/flb_input.h"
static void drop_first(struct flb_lib_ctx *ctx)
{
    struct flb_input_instance *ins = mk_list_entry(ctx->config->inputs.next, struct flb_input_instance, _head);
    mk_list_del(&ins->_head);
}
void service_hook(struct flb_lib_ctx *ctx)
{
    drop_first(ctx);
}
"#;
    route_kept(
        &fixed("unlink", &Shape::default(), &[("src/service.c", service)]),
        "more than one call reaches a hazard",
    );
}

#[test]
fn nc_a_non_null_return_before_the_append_is_underived() {
    let shape = Shape {
        new_extra: "    if (public_only) {\n        return instance;\n    }".to_owned(),
        ..Shape::default()
    };
    route_kept(
        &fixed("early-return", &shape, &[]),
        "P-HEAD-INSTANCE's appended creation is not derived",
    );
}

// --- P-INSTANCE-FIELDS-BY-MEMBER ----------------------------------------

#[test]
fn nc_a_memcpy_over_an_instance_refutes_the_fields() {
    let clobber = r#"
#include "fluent-bit/flb_input.h"
void clobber(struct flb_input_instance *a, struct flb_input_instance *b)
{
    memcpy(a, b, sizeof(*a));
}
"#;
    route_kept(
        &fixed("memcpy", &Shape::default(), &[("src/clobber.c", clobber)]),
        "P-INSTANCE-FIELDS-BY-MEMBER is refuted",
    );
}

#[test]
fn nc_a_whole_instance_assignment_refutes_the_fields() {
    let clobber = r#"
#include "fluent-bit/flb_input.h"
void clobber(struct flb_input_instance *a, struct flb_input_instance *b)
{
    *a = *b;
}
"#;
    route_kept(
        &fixed("assign", &Shape::default(), &[("src/clobber.c", clobber)]),
        "P-INSTANCE-FIELDS-BY-MEMBER is refuted",
    );
}

#[test]
fn nc_a_character_view_of_an_instance_refutes_the_fields() {
    let clobber = r#"
#include "fluent-bit/flb_input.h"
void clobber(struct flb_input_instance *a)
{
    unsigned char *bytes = (unsigned char *) a;
    bytes[0] = 0;
}
"#;
    route_kept(
        &fixed("bytes", &Shape::default(), &[("src/clobber.c", clobber)]),
        "P-INSTANCE-FIELDS-BY-MEMBER is refuted",
    );
}

#[test]
fn a_void_escape_and_an_other_type_view_stay_stated() {
    let escape = r#"
#include "fluent-bit/flb_input.h"
struct other_ctx { int x; };
void keep(void *p);
void escape(struct flb_input_instance *a)
{
    struct other_ctx *o = (struct other_ctx *) a;
    keep(a);
    flb_free(a);
    (void) o;
}
"#;
    let census = fixed("escape", &Shape::default(), &[("src/escape.c", escape)]);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(
        reason.contains("converted to another object type"),
        "{reason}"
    );
    let fields = &census
        .reading
        .premises
        .as_ref()
        .expect("report")
        .fields_by_member;
    assert!(fields.violations.is_empty(), "{fields:#?}");
    assert!(
        fields
            .escapes
            .iter()
            .any(|e| e.contains("escape conv:void*|arg:keep")),
        "{fields:#?}"
    );
    assert!(
        fields
            .other_types
            .iter()
            .any(|e| e.contains("conv:struct other_ctx*|value")),
        "{fields:#?}"
    );
    assert!(fields.harmless >= 1, "{fields:#?}");
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
}

// --- P-INSTANCE -----------------------------------------------------------

#[test]
fn nc_a_collector_re_pointed_to_another_instance_is_refuted() {
    let repoint = r#"
#include "fluent-bit/flb_input.h"
void repoint(struct flb_input_collector *c, struct flb_input_instance *other)
{
    c->instance = other;
}
"#;
    let census = fixed("repoint", &Shape::default(), &[("src/repoint.c", repoint)]);
    for premise in site_premises(&census) {
        assert!(
            matches!(&premise, SitePremise::Refuted(why) if why.contains("is written at")),
            "{premise:?}"
        );
    }
    for verdict in verdicts(&census, "cb_tail_pause_test") {
        assert!(
            !matches!(verdict, OwnerVerdict::Discharged { .. }),
            "{verdict:?}"
        );
    }
}

#[test]
fn nc_a_dispatch_passing_another_instance_refutes_and_withholds_the_discharge() {
    // Both loop dispatchers pass another instance than the collector's.
    let shape = Shape {
        dispatched: "spare_dispatch_instance".to_owned(),
        ..Shape::default()
    };
    let spare = r#"
#include "fluent-bit/flb_input.h"
struct flb_input_instance *spare_dispatch_instance;
"#;
    let fixture = make_fixture(
        "other-instance",
        &shape,
        &test_file(ONE_TAIL, "", "", ""),
        &[("src/spare.c", spare)],
    );
    let census = run(&fixture);
    let premises = site_premises(&census);
    assert!(!premises.is_empty());
    for premise in &premises {
        assert!(
            matches!(premise, SitePremise::Refuted(why) if why.contains("does not pass `C->instance`")),
            "{premise:?}"
        );
    }
    unresolved_with(
        &verdicts(&census, "cb_tail_pause_test"),
        "P-INSTANCE is refuted",
    );
}

#[test]
fn nc_an_operation_on_a_reassigned_parameter_is_refuted() {
    let fixture = make_fixture(
        "reassigned",
        &Shape::default(),
        &test_file(ONE_TAIL, "", "    ins = spare_instance;", ""),
        &[],
    );
    let census = run(&fixture);
    for premise in site_premises(&census) {
        assert!(
            matches!(&premise, SitePremise::Refuted(why) if why.contains("reassigned")),
            "{premise:?}"
        );
    }
    for verdict in verdicts(&census, "cb_tail_pause_test") {
        assert!(
            !matches!(verdict, OwnerVerdict::Discharged { .. }),
            "{verdict:?}"
        );
    }
}

#[test]
fn nc_the_old_test_shape_is_not_discharged_under_the_premises() {
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
    let fixture = make_fixture("old", &Shape::default(), old, &[]);
    let census = run(&fixture);
    for verdict in verdicts(&census, "flb_test_inotify_pause_collectors") {
        assert!(
            matches!(verdict, OwnerVerdict::NotDischarged { context, .. } if context.thread == "T"),
            "{verdict:?}"
        );
    }
    for premise in site_premises(&census) {
        assert!(matches!(premise, SitePremise::Stated(_)), "{premise:?}");
    }
}

// --- The fixed shape -------------------------------------------------------

#[test]
fn the_fixed_shape_discharges_under_the_derived_premises() {
    let census = fixed("fixed", &Shape::default(), &[]);
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    for needle in [
        "P-FRESH-CONTEXT derived",
        "P-REGISTERED-DESCRIPTORS derived",
        "\"tail\" matches 1 of them (in_tail_plugin)",
        "P-HEAD-INSTANCE narrowed",
        "stated: P-CREATIONS-SUCCEED",
        "P-INSTANCE-FIELDS-BY-MEMBER narrowed",
        "P-CALLBACK-INSTANCE derived",
    ] {
        assert!(reason.contains(needle), "{needle}: {reason}");
    }
    discharged_on_the_owners(&verdicts(&census, "cb_tail_pause_test"));
    let premises = site_premises(&census);
    assert_eq!(premises.len(), 2, "{premises:?}");
    for premise in premises {
        assert!(
            matches!(&premise, SitePremise::Derived(why) if why.contains("P-INSTANCE derived")),
            "{premise:?}"
        );
    }
}

#[test]
fn the_unpremised_census_is_unchanged() {
    // Without the premise inputs the census states the premises, as before
    // this card, and reports none.
    let fixture = make_fixture(
        "unpremised",
        &Shape::default(),
        &test_file(ONE_TAIL, "", "", ""),
        &[],
    );
    let typed = facts(&fixture);
    let census = census_typed(
        &fixture.files,
        &obligations(),
        &FLUENT_BIT_HOPS,
        None,
        Some(&typed),
    );
    assert!(census.reading.premises.is_none());
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(
        reason.contains("(P-HEAD-INSTANCE, P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS)"),
        "{reason}"
    );
}
