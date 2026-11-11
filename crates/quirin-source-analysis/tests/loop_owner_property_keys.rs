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

//! W47-SERVICE-PROPERTY-KEYS — the keys a variadic property setter carries,
//! over compilable fixtures of the Fluent Bit shapes at `90978ac6`:
//! `flb_service_set` (`src/flb_lib.c:674`), `flb_config_set_property` with
//! its key table, comparison and keyed branch chain (`src/flb_config.c:56`,
//! `:782`, `:855`), a `Plugins_File` handler registering into
//! `config->in_plugins` (`src/flb_plugin.c:251`), and the fixed test's maker,
//! list-head read and holder (`tests/runtime/in_tail.c:218`, `:2551-2557`).
//! Each fixture is compiled by Clang (the typed reading) and read by the
//! lexical census. Negative controls first.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::{
    CensusFile, FLUENT_BIT_INSTANCE_FLOW, FLUENT_BIT_PROPERTY_KEYS,
};
use qcore_c_gateway::typed_dispatch_census::{TypedFacts, TypedUnit, read_units};
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::{Mode, OwnerVerdict};
use quirin_source_analysis::loop_owner_census::{Census, FLUENT_BIT_HOPS, census_typed};

const CONTRACTS: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
);

/// The input header of `loop_owner_instance_flow.rs`, with the property
/// table's record, the config's scalar fields, the key macros and a variadic
/// `flb_service_set`.
const INPUT_HEADER: &str = r#"
#ifndef FLB_INPUT_H
#define FLB_INPUT_H
#define NULL ((void *) 0)
#define FLB_INPUT_NET  4
#define FLB_INPUT_CORO 128
#include <stdarg.h>
typedef __SIZE_TYPE__ size_t;
#define offsetof(type, member) __builtin_offsetof(type, member)
int strncasecmp(const char *a, const char *b, size_t n);
size_t strnlen(const char *s, size_t n);
int atoi(const char *s);
char *flb_strdup(const char *s);
enum {
    FLB_CONF_TYPE_INT,
    FLB_CONF_TYPE_DOUBLE,
    FLB_CONF_TYPE_BOOL,
    FLB_CONF_TYPE_STR,
    FLB_CONF_TYPE_OTHER
};
#define FLB_CONF_STR_FLUSH "Flush"
#define FLB_CONF_STR_GRACE "Grace"
#define FLB_CONF_STR_LOGLEVEL "Log_Level"
#define FLB_CONF_STR_PARSERS_FILE "Parsers_File"
#define FLB_CONF_STR_PLUGINS_FILE "Plugins_File"
struct flb_service_config {
    char *key;
    int type;
    size_t offset;
};
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
struct flb_config {
    double flush;
    unsigned int grace;
    char *log;
    char *parsers_file;
    char *plugins_file;
    int verbose;
    struct mk_list inputs;
    struct mk_list in_plugins;
};
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
int flb_service_set(struct flb_lib_ctx *ctx, ...);
int flb_config_set_property(struct flb_config *config, const char *k, const char *v);
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

/// The library of `loop_owner_instance_flow.rs`, with the variadic setter's
/// body as the export has it.
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
int flb_service_set(struct flb_lib_ctx *ctx, ...)
{
    int ret;
    char *key;
    char *value;
    va_list va;

    va_start(va, ctx);
    while ((key = va_arg(va, char *))) {
        value = va_arg(va, char *);
        if (!value) {
            va_end(va);
            return -1;
        }
        ret = flb_config_set_property(ctx->config, key, value);
        if (ret != 0) {
            va_end(va);
            return -1;
        }
    }
    va_end(va);
    return 0;
}
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_resume(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_running(int coll_id, struct flb_input_instance *in) { return 0; }
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

/// `src/flb_config.c`'s shapes: the key table, the comparison, the property
/// function and its handlers; `EXTRA_ENTRY` is spliced into the table.
fn config_file(extra_entry: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
struct flb_service_config service_configs[] = {{
    {{FLB_CONF_STR_FLUSH,
     FLB_CONF_TYPE_DOUBLE,
     offsetof(struct flb_config, flush)}},

    {{FLB_CONF_STR_GRACE,
     FLB_CONF_TYPE_INT,
     offsetof(struct flb_config, grace)}},

    {{FLB_CONF_STR_PARSERS_FILE,
     FLB_CONF_TYPE_STR,
     offsetof(struct flb_config, parsers_file)}},

    {{FLB_CONF_STR_PLUGINS_FILE,
     FLB_CONF_TYPE_STR,
     offsetof(struct flb_config, plugins_file)}},

    {{FLB_CONF_STR_LOGLEVEL,
     FLB_CONF_TYPE_STR,
     offsetof(struct flb_config, log)}},
{extra_entry}
    {{NULL, FLB_CONF_TYPE_OTHER, 0}}
}};

static inline int prop_key_check(const char *key, const char *kv, int k_len)
{{
    size_t len;

    len = strnlen(key,256);
    if (strncasecmp(key, kv, k_len) == 0 && len == k_len) {{
        return 0;
    }}
    return -1;
}}

static int set_log_level(struct flb_config *config, const char *v_str)
{{
    config->verbose = 3;
    return 0;
}}

int flb_parser_conf_file(const char *file, struct flb_config *config)
{{
    return 0;
}}

static struct flb_input_plugin loaded;

int flb_plugin_load_config_file(const char *file, struct flb_config *config)
{{
    mk_list_add(&loaded._head, &config->in_plugins);
    return 0;
}}

int flb_config_set_property(struct flb_config *config,
                            const char *k, const char *v)
{{
    int i=0;
    int ret = -1;
    int *i_val;
    double *d_val;
    char **s_val;
    size_t len = strnlen(k, 256);
    char *key = service_configs[0].key;

    while (key != NULL) {{
        if (prop_key_check(key, k,len) == 0) {{
            if (!strncasecmp(key, FLB_CONF_STR_LOGLEVEL, 256)) {{
                ret = set_log_level(config, v);
            }}
            else if (!strncasecmp(key, FLB_CONF_STR_PARSERS_FILE, 32)) {{
                ret = flb_parser_conf_file(v, config);
            }}
            else if (!strncasecmp(key, FLB_CONF_STR_PLUGINS_FILE, 32)) {{
                ret = flb_plugin_load_config_file(v, config);
            }}
            else {{
                ret = 0;
                switch(service_configs[i].type) {{
                case FLB_CONF_TYPE_INT:
                    i_val  = (int*)((char*)config + service_configs[i].offset);
                    *i_val = atoi(v);
                    break;
                case FLB_CONF_TYPE_DOUBLE:
                    d_val  = (double*)((char*)config + service_configs[i].offset);
                    *d_val = 1.0;
                    break;
                case FLB_CONF_TYPE_STR:
                    s_val = (char**)((char*)config+service_configs[i].offset);
                    *s_val = flb_strdup(v);
                    break;
                default:
                    ret = -1;
                }}
            }}

            if (ret < 0) {{
                return -1;
            }}
            return 0;
        }}
        key = service_configs[++i].key;
    }}

    return 0;
}}
"#
    )
}

/// The fixed test's shape; `SERVICE` is the maker's setter call.
fn test_file(service: &str, extra: &str) -> String {
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
    struct test_ctx *ctx = flb_calloc(1, sizeof(struct test_ctx));
    if (!ctx) {{
        return NULL;
    }}
    ctx->flb = flb_create();
{service}
    ctx->i_ffd = flb_input(ctx->flb, (char *) "tail", NULL);
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

/// The fixed test's call (`tests/runtime/in_tail.c:218`).
const FIXED_SERVICE: &str = r#"    flb_service_set(ctx->flb,
                    "Flush", "0.200000000",
                    "Grace", "1",
                    "Log_Level", "info",
                    "Parsers_File", "/data/common/parsers.conf",
                    NULL);"#;

struct Fixture {
    root: PathBuf,
    files: Vec<CensusFile>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_fixture(name: &str, test: &str, extra_entry: &str) -> Fixture {
    let root =
        std::env::temp_dir().join(format!("qcore-property-keys-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let sources = vec![
        ("include/fluent-bit/flb_input.h", INPUT_HEADER.to_owned()),
        ("src/flb_lib.c", LIBRARY.to_owned()),
        ("src/flb_config.c", config_file(extra_entry)),
        (
            "src/service.c",
            "#include \"fluent-bit/flb_input.h\"\nvoid service_hook(struct flb_lib_ctx *ctx)\n{\n}\n"
                .to_owned(),
        ),
        ("plugins/in_tail/tail.c", TAIL_PLUGIN.to_owned()),
        ("plugins/in_coro/coro.c", CORO_PLUGIN.to_owned()),
        ("tests/runtime/in_tail.c", test.to_owned()),
    ];
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
        &FLUENT_BIT_HOPS.property_key_options(&FLUENT_BIT_INSTANCE_FLOW, &FLUENT_BIT_PROPERTY_KEYS),
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

/// The setter call's reading, as the census records it.
fn keys(census: &Census) -> Vec<String> {
    census
        .reading
        .property_keys
        .iter()
        .find(|(site, _)| site.ends_with("flb_service_set"))
        .map(|(_, rows)| rows.clone())
        .expect("the setter call is read")
}

/// The flag is underived for `needle`, and the callback's sites stay
/// unresolved on the kept coroutine route.
fn route_kept(census: &Census, needle: &str) {
    let (derived, reason) = flag(census);
    assert_eq!(derived, None, "{reason}");
    assert!(reason.contains(needle), "{needle}: {reason}");
    for verdict in verdicts(census, "cb_tail_pause_test") {
        let OwnerVerdict::Unresolved { reasons } = verdict else {
            panic!("expected unresolved, got {verdict:?}");
        };
        assert!(
            reasons.iter().any(|r| r.contains("may yield")),
            "{reasons:#?}"
        );
    }
}

/// Every branch is kept: the keys are not read, and the whole setter call
/// reaches the registration hazard.
fn every_branch(census: &Census, why: &str) {
    let rows = keys(census);
    assert!(
        rows.iter()
            .any(|row| row.starts_with("keys not read") && row.contains(why)),
        "{why}: {rows:#?}"
    );
    route_kept(census, "`flb_service_set` at tests/runtime/in_tail.c");
}

fn discharged_on_the_owners(census: &Census) {
    for verdict in verdicts(census, "cb_tail_pause_test") {
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

#[test]
fn nc_a_literal_plugins_file_key_keeps_the_registration_hazard() {
    let service =
        "    flb_service_set(ctx->flb, \"Flush\", \"1\", \"Plugins_File\", \"/p.conf\", NULL);";
    let fixture = make_fixture("plugins-file", &test_file(service, ""), "");
    let census = run(&fixture);
    assert!(
        keys(&census)
            .iter()
            .any(|row| row.contains("\"Plugins_File\"") && row.contains("keyed arm 3")),
        "{:#?}",
        keys(&census)
    );
    route_kept(
        &census,
        "more than one call reaches a hazard: `flb_plugin_load_config_file`",
    );
}

#[test]
fn nc_a_case_variant_of_plugins_file_keeps_the_registration_hazard() {
    let service = "    flb_service_set(ctx->flb, \"PLUGINS_file\", \"/p.conf\", NULL);";
    let fixture = make_fixture("plugins-case", &test_file(service, ""), "");
    route_kept(
        &run(&fixture),
        "more than one call reaches a hazard: `flb_plugin_load_config_file`",
    );
}

#[test]
fn nc_a_key_from_a_variable_keeps_every_branch() {
    let service = "    flb_service_set(ctx->flb, name, \"1\", NULL);";
    let fixture = make_fixture("variable-key", &test_file(service, ""), "");
    every_branch(&run(&fixture), "the key of pair 1 is not a string literal");
}

#[test]
fn nc_an_odd_argument_list_keeps_every_branch() {
    let service = "    flb_service_set(ctx->flb, \"Flush\", \"1\", \"Grace\", NULL);";
    let fixture = make_fixture("odd", &test_file(service, ""), "");
    every_branch(&run(&fixture), "an odd number of key/value arguments");
}

#[test]
fn nc_a_list_without_its_null_keeps_every_branch() {
    let service = "    flb_service_set(ctx->flb, \"Flush\", \"1\");";
    let fixture = make_fixture("no-null", &test_file(service, ""), "");
    every_branch(&run(&fixture), "not NULL-terminated");
}

#[test]
fn nc_an_unknown_key_keeps_every_branch() {
    let service = "    flb_service_set(ctx->flb, \"No_Such_Key\", \"1\", NULL);";
    let fixture = make_fixture("unknown", &test_file(service, ""), "");
    every_branch(&run(&fixture), "matches no entry of the derived table");
}

#[test]
fn nc_a_prefix_of_a_key_is_not_that_key() {
    // The comparison is over the whole key (`len == k_len`).
    let service = "    flb_service_set(ctx->flb, \"Flus\", \"1\", NULL);";
    let fixture = make_fixture("prefix", &test_file(service, ""), "");
    every_branch(&run(&fixture), "matches no entry of the derived table");
}

#[test]
fn nc_a_scalar_entry_on_a_list_member_keeps_every_branch() {
    // A table entry whose store would write the registered-plugin list head.
    let entry = "    {\"Registry\", FLB_CONF_TYPE_STR, offsetof(struct flb_config, in_plugins)},\n";
    let service = "    flb_service_set(ctx->flb, \"Registry\", \"x\", NULL);";
    let fixture = make_fixture("list-member", &test_file(service, ""), entry);
    every_branch(
        &run(&fixture),
        "its offset names `struct flb_config::in_plugins`",
    );
}

#[test]
fn nc_a_store_wider_than_its_field_keeps_every_branch() {
    // `grace` is an `unsigned int`; a double store would write past it.
    let entry = "    {\"Wide\", FLB_CONF_TYPE_DOUBLE, offsetof(struct flb_config, grace)},\n";
    let service = "    flb_service_set(ctx->flb, \"Wide\", \"1\", NULL);";
    let fixture = make_fixture("wide", &test_file(service, ""), entry);
    every_branch(&run(&fixture), "is not `struct flb_config::grace`'s type");
}

#[test]
fn nc_the_old_test_shape_is_not_discharged() {
    // The previous test (`90978ac6~1`) operates the collectors on the test
    // thread itself: the key reading changes nothing there.
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
    flb_service_set(ctx->flb, "Flush", "1", "Grace", "1", NULL);
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
    let fixture = make_fixture("old", old, "");
    let census = run(&fixture);
    for verdict in verdicts(&census, "flb_test_inotify_pause_collectors") {
        assert!(
            matches!(verdict, OwnerVerdict::NotDischarged { context, .. } if context.thread == "T"),
            "{verdict:?}"
        );
    }
}

#[test]
fn the_fixed_call_reaches_only_its_keys_effects() {
    let fixture = make_fixture("fixed", &test_file(FIXED_SERVICE, ""), "");
    let typed = facts(&fixture);
    // The typed reading records the call's arguments, the table and the
    // keyed branches' constants.
    assert!(
        typed
            .argument_lists
            .iter()
            .any(|(key, lists)| key.ends_with("|flb_service_set")
                && lists
                    .iter()
                    .any(|list| list.first().map(String::as_str) == Some("s:Flush")
                        && list.last().map(String::as_str) == Some("null"))),
        "{:#?}",
        typed.argument_lists
    );
    assert!(
        typed
            .key_tables
            .contains_key("src/flb_config.c|service_configs")
    );
    let census = run(&fixture);
    let rows = keys(&census);
    for expected in [
        "key \"Flush\" → `Flush` (FLB_CONF_TYPE_DOUBLE): scalar store into `struct flb_config::flush` (double)",
        "key \"Grace\" → `Grace` (FLB_CONF_TYPE_INT): scalar store into `struct flb_config::grace` (int)",
        "table Plugins_File | FLB_CONF_TYPE_STR | struct flb_config::plugins_file | keyed arm 3",
    ] {
        assert!(
            rows.iter().any(|row| row.starts_with(expected)),
            "{expected}: {rows:#?}"
        );
    }
    let (derived, reason) = flag(&census);
    assert_eq!(derived, Some(false), "{reason}");
    assert!(
        reason.contains("property keys: `flb_service_set`"),
        "{reason}"
    );
    assert!(reason.contains("list-head instance"), "{reason}");
    discharged_on_the_owners(&census);
}

#[test]
fn a_lower_case_key_matches_as_the_comparison_folds_case() {
    let service = "    flb_service_set(ctx->flb, \"flush\", \"1\", \"GRACE\", \"1\", NULL);";
    let fixture = make_fixture("case", &test_file(service, ""), "");
    let census = run(&fixture);
    assert!(
        keys(&census)
            .iter()
            .any(|row| row.starts_with("key \"flush\" → `Flush`")),
        "{:#?}",
        keys(&census)
    );
    discharged_on_the_owners(&census);
}
