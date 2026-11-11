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

//! W47-PREMISES-CORO-PLUGIN — P-CORO-HANDLE, P-PLUGIN-DIR and the immutable
//! threaded flag derived from the program, over compilable fixtures of the
//! Fluent Bit shapes at `90978ac6`: the input coroutine's constructor, hand-off,
//! entry and resume wrapper (`include/fluent-bit/flb_input.h:597-735`), the
//! coroutine primitives (`include/fluent-bit/flb_coro.h:93-135`), the
//! instance constructor and the property setter's `threaded` arm
//! (`src/flb_input.c:326`, `:796`, `:1002`), a plugin whose `cb_init`
//! registers its collector. Each fixture is compiled by Clang (the typed
//! reading) and read by the lexical census. Negative controls first.

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

const INPUT_HEADER: &str = r#"
#ifndef FLB_INPUT_H
#define FLB_INPUT_H
#define NULL ((void *) 0)
#define FLB_INPUT_CORO 128
#define FLB_INPUT_THREADED 256
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
struct flb_coro { void *caller; void *callee; void *data; };
struct flb_input_instance;
struct flb_input_plugin {
    int flags;
    const char *name;
    int (*cb_init)(struct flb_input_instance *, void *, void *);
    struct mk_list _head;
};
struct flb_input_instance {
    struct flb_input_plugin *p;
    void *context;
    void *data;
    int is_threaded;
    int runs_in_coroutine;
    struct mk_list input_coro_list;
    struct mk_list _head;
};
struct flb_config { struct mk_list inputs; struct mk_list in_plugins; };
struct flb_input_collector { int (*cb_collect)(struct flb_input_instance *, void *, void *); struct flb_input_instance *instance; };
struct flb_input_coro { struct flb_coro *coro; struct mk_list _head; };
struct flb_libco_in_params { struct flb_input_collector *coll; struct flb_coro *coro; };
struct test_entry { const char *name; void (*func)(void); };
#define TEST_LIST struct test_entry test_list[]
typedef int (*collect_fn)(struct flb_input_instance *, void *, void *);
void *flb_calloc(unsigned long n, unsigned long size);
void flb_free(void *p);
void co_switch(void *to);
void *co_active(void);
void *co_create(int size, void (*entry)(void), int *stack);
struct flb_coro *flb_coro_get(void);
struct flb_libco_in_params *in_params(void);
int mk_utils_worker_spawn(void (*func)(void *), void *data, int *tid);
int flb_tp_thread_create(int id, void (*func)(void *), void *data, void *config);
int flb_engine_start(void *config);
int input_collector_fd(int fd, void *config);
int flb_io_net_write(int fd);
int flb_start(void *ctx);
int flb_input_set_collector_time(struct flb_input_instance *ins, collect_fn cb, long s, long ns, void *config);
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in);
int flb_input_collector_running(int coll_id, struct flb_input_instance *in);
int flb_input_set_property(struct flb_input_instance *ins, const char *k, const char *v);
static inline struct flb_coro *flb_coro_create(void *data)
{
    struct flb_coro *coro;
    coro = (struct flb_coro *) flb_calloc(1, sizeof(struct flb_coro));
    if (!coro) {
        return NULL;
    }
    coro->data = data;
    return coro;
}
static inline void flb_coro_yield(struct flb_coro *coro, int ended)
{
    co_switch(coro->caller);
}
static inline void flb_coro_resume(struct flb_coro *coro)
{
    coro->caller = co_active();
    co_switch(coro->callee);
}
static inline struct flb_input_coro *flb_input_coro_create(struct flb_input_instance *ins)
{
    struct flb_input_coro *input_coro;
    struct flb_coro *coro;
    input_coro = (struct flb_input_coro *) flb_calloc(1, sizeof(struct flb_input_coro));
    if (!input_coro) {
        return NULL;
    }
    coro = flb_coro_create(input_coro);
    if (!coro) {
        flb_free(input_coro);
        return NULL;
    }
    input_coro->coro = coro;
    mk_list_add(&input_coro->_head, &ins->input_coro_list);
    return input_coro;
}
static inline void input_params_set(struct flb_coro *coro, struct flb_input_collector *coll)
{
    struct flb_libco_in_params *params = in_params();
    params->coll = coll;
    params->coro = coro;
    co_switch(coro->callee);
}
static inline void input_pre_cb_collect(void)
{
    struct flb_input_collector *coll;
    struct flb_coro *coro;
    struct flb_libco_in_params *params = in_params();
    coll = params->coll;
    coro = params->coro;
    co_switch(coro->caller);
    coll->cb_collect(coll->instance, 0, coll->instance->context);
}
static inline void flb_input_coro_resume(struct flb_input_coro *input_coro)
{
    flb_coro_resume(input_coro->coro);
}
static inline struct flb_input_coro *flb_input_coro_collect(struct flb_input_collector *coll, void *config)
{
    struct flb_coro *coro;
    struct flb_input_coro *input_coro;
    input_coro = flb_input_coro_create(coll->instance);
    if (!input_coro) {
        return NULL;
    }
    coro = input_coro->coro;
    coro->caller = co_active();
    coro->callee = co_create(64, input_pre_cb_collect, 0);
    input_params_set(coro, coll);
    return input_coro;
}
static inline int flb_input_is_threaded(struct flb_input_instance *ins)
{
    return ins->is_threaded;
}
#endif
"#;

/// The engine and worker loops (each tests the coroutine flag before
/// constructing a coroutine), the instance's init dispatching `cb_init`
/// through `p = ins->p`, the registration, the constructor and the property
/// setter.
const LIBRARY: &str = r#"
#include "fluent-bit/flb_input.h"
int strlen_(const char *s);
int strncasecmp_(const char *a, const char *b, int n);
int atoi_(const char *s);
static void flb_lib_worker(void *data)
{
    flb_engine_start(data);
}
int flb_start(void *ctx)
{
    int tid;
    return mk_utils_worker_spawn(flb_lib_worker, ctx, &tid);
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
static void input_thread(void *data)
{
    input_collector_fd(0, data);
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
int flb_input_instance_init(struct flb_input_instance *ins, void *config)
{
    struct flb_input_plugin *p = ins->p;
    p->cb_init(ins, config, ins->data);
    if (flb_input_is_threaded(ins)) {
        return flb_tp_thread_create(0, input_thread, ins, config);
    }
    return 0;
}
int flb_engine_start(void *config)
{
    struct flb_input_instance *ins = 0;
    flb_input_instance_init(ins, config);
    while (1) {
        flb_input_collector_fd(0, config);
    }
    return 0;
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
struct flb_input_instance *flb_input_new(struct flb_config *config, struct flb_input_plugin *plugin, void *data)
{
    struct flb_input_instance *instance = NULL;
    instance = flb_calloc(1, sizeof(struct flb_input_instance));
    if (!instance) {
        return NULL;
    }
    instance->p = plugin;
    instance->data = data;
    instance->runs_in_coroutine = 0;
    if (plugin->flags & FLB_INPUT_CORO) {
        instance->runs_in_coroutine = 1;
    }
    if (plugin->flags & FLB_INPUT_THREADED) {
        instance->is_threaded = 1;
    }
    mk_list_init(&instance->input_coro_list);
    mk_list_add(&instance->_head, &config->inputs);
    return instance;
}
static int prop_key_check(const char *key, const char *kv, int k_len)
{
    return strncasecmp_(key, kv, k_len);
}
int flb_input_set_property(struct flb_input_instance *ins, const char *k, const char *v)
{
    int len = strlen_(k);
    if (prop_key_check("tag", k, len) == 0) {
        ins->context = 0;
    }
    else if (prop_key_check("threaded", k, len) == 0) {
        ins->is_threaded = atoi_(v);
    }
    return 0;
}
int flb_input_collector_pause(int coll_id, struct flb_input_instance *in) { return 0; }
int flb_input_collector_running(int coll_id, struct flb_input_instance *in) { return 0; }
"#;

/// A plugin that runs no coroutine; its collector pauses itself (the site)
/// and may yield (so a kept coroutine route leaves the site unresolved).
fn dummy_plugin(collect_extra: &str, init_extra: &str, extra: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
static int in_dummy_collect(struct flb_input_instance *ins, void *config, void *context)
{{
    flb_input_collector_pause(1, ins);
    flb_io_net_write(0);
{collect_extra}
    return 0;
}}
static int in_dummy_init(struct flb_input_instance *ins, void *config, void *data)
{{
    flb_input_set_collector_time(ins, in_dummy_collect, 1, 0, config);
    flb_input_set_property(ins, "tag", "dummy");
{init_extra}
    return 0;
}}
{extra}
struct flb_input_plugin in_dummy_plugin = {{
    .name = "dummy",
    .cb_init = in_dummy_init,
    .flags = 0
}};
"#
    )
}

fn test_file(body: &str) -> String {
    format!(
        r#"
#include "fluent-bit/flb_input.h"
void flb_test_dummy(void)
{{
{body}
    flb_start(0);
}}
TEST_LIST = {{
    {{ "dummy", flb_test_dummy }},
    {{ 0, 0 }}
}};
"#
    )
}

struct Fixture {
    root: PathBuf,
    files: Vec<CensusFile>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_fixture(name: &str, plugin: &str, test: &str, extra: &[(&str, &str)]) -> Fixture {
    make_fixture_with(name, INPUT_HEADER, LIBRARY, plugin, test, extra)
}

fn make_fixture_with(
    name: &str,
    header: &str,
    library: &str,
    plugin: &str,
    test: &str,
    extra: &[(&str, &str)],
) -> Fixture {
    let root =
        std::env::temp_dir().join(format!("qcore-coro-plugin-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mut sources = vec![
        ("include/fluent-bit/flb_input.h", header.to_owned()),
        ("src/flb_lib.c", library.to_owned()),
        ("plugins/in_dummy/dummy.c", plugin.to_owned()),
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

fn obligations() -> BTreeMap<String, qcore_contract::contract_loader::DeclaredLoopOwner> {
    loop_owner_obligations(&BTreeMap::from([(
        "fluent_bit/loop_owner.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads")
}

fn run(fixture: &Fixture) -> Census {
    let typed = facts(fixture);
    census_typed(
        &fixture.files,
        &obligations(),
        &FLUENT_BIT_HOPS,
        None,
        Some(&typed),
    )
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

fn premise(census: &Census, name: &str) -> Vec<String> {
    census
        .reading
        .coro_plugin_premises
        .get(name)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "no premise {name}: {:#?}",
                census.reading.coro_plugin_premises
            )
        })
}

fn not_derived(census: &Census, name: &str, needle: &str) {
    let lines = premise(census, name);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("not derived") && l.contains(needle)),
        "{name} / {needle}: {lines:#?}"
    );
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

/// A foreign resume: a function outside the coroutine's own reads the
/// handle member and resumes it.
#[test]
fn nc_a_foreign_resume_of_the_handle_is_not_derived() {
    let extra =
        "void resume_again(struct flb_input_coro *c)\n{\n    flb_coro_resume(c->coro);\n}\n";
    let fixture = make_fixture("foreign", &dummy_plugin("", "", extra), &test_file(""), &[]);
    let census = run(&fixture);
    not_derived(
        &census,
        "P-CORO-HANDLE",
        "outside the coroutine's own functions",
    );
    unresolved_with(
        &verdicts(&census, "in_dummy_collect"),
        "P-CORO-HANDLE is not derived",
    );
}

/// The handle copied out of the set by one of its own functions.
#[test]
fn nc_a_handle_leaving_the_set_is_not_derived() {
    let header = INPUT_HEADER
        .replace(
            "    input_params_set(coro, coll);\n    return input_coro;",
            "    input_params_set(coro, coll);\n    keep_handle(coro);\n    return input_coro;",
        )
        .replace(
            "struct flb_libco_in_params *in_params(void);",
            "struct flb_libco_in_params *in_params(void);\nvoid keep_handle(struct flb_coro *coro);",
        );
    assert_ne!(header, INPUT_HEADER);
    let fixture = make_fixture_with(
        "leaves",
        &header,
        LIBRARY,
        &dummy_plugin("", "", ""),
        &test_file(""),
        &[],
    );
    let census = run(&fixture);
    not_derived(&census, "P-CORO-HANDLE", "leaves `flb_input_coro_collect`");
    unresolved_with(
        &verdicts(&census, "in_dummy_collect"),
        "P-CORO-HANDLE is not derived",
    );
}

/// A coroutine taken from the instance's coroutine list.
#[test]
fn nc_a_list_read_through_is_not_derived() {
    let extra = "void walk(struct flb_input_instance *ins)\n{\n    struct mk_list *head = ins->input_coro_list.next;\n    (void) head;\n}\n";
    let fixture = make_fixture("list", &dummy_plugin("", "", extra), &test_file(""), &[]);
    let census = run(&fixture);
    not_derived(&census, "P-CORO-HANDLE", "a coroutine may be taken from it");
    unresolved_with(
        &verdicts(&census, "in_dummy_collect"),
        "P-CORO-HANDLE is not derived",
    );
}

/// The hand-off called from outside the set.
#[test]
fn nc_the_hand_off_called_outside_the_set_is_not_derived() {
    let extra = "void switch_in(struct flb_coro *c, struct flb_input_collector *k)\n{\n    input_params_set(c, k);\n}\n";
    let fixture = make_fixture("handoff", &dummy_plugin("", "", extra), &test_file(""), &[]);
    let census = run(&fixture);
    not_derived(&census, "P-CORO-HANDLE", "`input_params_set` is called at");
}

/// A descriptor's flags written at run time.
#[test]
fn nc_a_flags_write_at_run_time_keeps_the_route() {
    let fixture = make_fixture(
        "flags",
        &dummy_plugin("    ins->p->flags |= FLB_INPUT_CORO;", "", ""),
        &test_file(""),
        &[],
    );
    let census = run(&fixture);
    not_derived(&census, "P-PLUGIN-DIR descriptor flags", "is written at");
    unresolved_with(&verdicts(&census, "in_dummy_collect"), "may yield");
}

/// A coroutine constructed without testing the instance's flag.
#[test]
fn nc_an_unguarded_coroutine_construction_keeps_the_route() {
    let library = LIBRARY.replacen(
        "    if (collector->instance->runs_in_coroutine) {\n        input_coro = flb_input_coro_collect(collector, config);\n        flb_input_coro_resume(input_coro);\n    }\n    else {\n        collector->cb_collect(collector->instance, config, collector->instance->context);\n    }",
        "    input_coro = flb_input_coro_collect(collector, config);\n    flb_input_coro_resume(input_coro);\n    collector->cb_collect(collector->instance, config, collector->instance->context);",
        1,
    );
    assert_ne!(library, LIBRARY);
    let fixture = make_fixture_with(
        "unguarded",
        INPUT_HEADER,
        &library,
        &dummy_plugin("", "", ""),
        &test_file(""),
        &[],
    );
    let census = run(&fixture);
    not_derived(
        &census,
        "P-PLUGIN-DIR descriptor flags",
        "outside a test of",
    );
    unresolved_with(&verdicts(&census, "in_dummy_collect"), "may yield");
}

/// The threaded flag toggled after start, in the collector itself.
#[test]
fn nc_a_threaded_flag_toggled_after_start_fixes_no_mode() {
    let fixture = make_fixture(
        "toggle",
        &dummy_plugin("    ins->is_threaded = 1;", "", ""),
        &test_file(""),
        &[],
    );
    let census = run(&fixture);
    not_derived(&census, "immutable threaded flag", "a run-time write");
    unresolved_with(
        &verdicts(&census, "in_dummy_collect"),
        "the threaded flag is not derived immutable after start",
    );
}

/// A plugin registering its collector on another plugin's instance.
#[test]
fn nc_a_registration_on_another_plugins_instance_keeps_the_route() {
    let extra = "static int in_dummy_collect2(struct flb_input_instance *ins, void *config, void *context)\n{\n    flb_input_collector_running(2, ins);\n    flb_io_net_write(0);\n    return 0;\n}\nvoid dummy_register(struct flb_input_instance *ins)\n{\n    flb_input_set_collector_time(ins, in_dummy_collect2, 1, 0, 0);\n}\n";
    let other = r#"
#include "fluent-bit/flb_input.h"
void dummy_register(struct flb_input_instance *ins);
static int in_other_init(struct flb_input_instance *ins, void *config, void *data)
{
    dummy_register(ins);
    return 0;
}
struct flb_input_plugin in_other_plugin = {
    .name = "other",
    .cb_init = in_other_init,
    .flags = FLB_INPUT_CORO
};
"#;
    let fixture = make_fixture(
        "other",
        &dummy_plugin("", "", extra),
        &test_file(""),
        &[("plugins/in_other/other.c", other)],
    );
    let census = run(&fixture);
    let registrations = premise(&census, "P-PLUGIN-DIR registrations");
    assert!(
        registrations
            .iter()
            .any(|l| l.contains("in_dummy_collect2") && l.contains("not derived")),
        "{registrations:#?}"
    );
    unresolved_with(&verdicts(&census, "in_dummy_collect2"), "may yield");
    // The plugin's own registration still derives, and its site discharges.
    discharged_on_the_owners(&verdicts(&census, "in_dummy_collect"));
}

/// The same helper called from the plugin's own `cb_init` derives.
#[test]
fn a_registration_through_the_plugins_own_helper_derives() {
    let extra = "static int in_dummy_collect2(struct flb_input_instance *ins, void *config, void *context)\n{\n    flb_input_collector_running(2, ins);\n    flb_io_net_write(0);\n    return 0;\n}\nstatic void dummy_register(struct flb_input_instance *ins)\n{\n    flb_input_set_collector_time(ins, in_dummy_collect2, 1, 0, 0);\n}\n";
    let plugin = dummy_plugin("", "    dummy_register(ins);", "").replace(
        "static int in_dummy_init",
        &format!("{extra}static int in_dummy_init"),
    );
    let fixture = make_fixture("helper", &plugin, &test_file(""), &[]);
    let census = run(&fixture);
    discharged_on_the_owners(&verdicts(&census, "in_dummy_collect2"));
}

/// The test thread's own call is refused whatever the premises (the old
/// test's shape).
#[test]
fn nc_the_test_threads_call_stays_not_discharged() {
    let fixture = make_fixture(
        "old",
        &dummy_plugin("", "", ""),
        &test_file("    flb_input_collector_running(1, 0);"),
        &[],
    );
    let census = run(&fixture);
    let test_sites = verdicts(&census, "flb_test_dummy");
    for verdict in test_sites {
        assert!(
            matches!(verdict, OwnerVerdict::NotDischarged { context, .. } if context.thread == "T"),
            "{verdict:?}"
        );
    }
}

/// The setter's calls: a non-literal key and the threaded literal are the
/// named residual; another literal cannot write the flag.
#[test]
fn the_setter_calls_that_may_write_the_flag_are_the_residual() {
    let extra = "void set_any(struct flb_input_instance *ins, const char *key)\n{\n    flb_input_set_property(ins, key, \"x\");\n}\nvoid set_threaded(struct flb_input_instance *ins)\n{\n    flb_input_set_property(ins, \"Threaded\", \"on\");\n}\n";
    let fixture = make_fixture("setter", &dummy_plugin("", "", extra), &test_file(""), &[]);
    let census = run(&fixture);
    let residual = premise(&census, "residual P-THREADED-SET-BEFORE-START");
    assert!(
        residual
            .iter()
            .any(|l| l.contains("set_any") && l.contains("not one literal")),
        "{residual:#?}"
    );
    assert!(
        residual
            .iter()
            .any(|l| l.contains("set_threaded") && l.contains("\"Threaded\"")),
        "{residual:#?}"
    );
    assert!(
        residual
            .iter()
            .any(|l| l.contains("flb_input_new") && l.contains("fresh instance")),
        "{residual:#?}"
    );
    assert!(
        !residual.iter().any(|l| l.contains("in_dummy_init")),
        "{residual:#?}"
    );
    let flag = premise(&census, "immutable threaded flag");
    assert_eq!(flag[0], "derived", "{flag:#?}");
    discharged_on_the_owners(&verdicts(&census, "in_dummy_collect"));
}

/// Everything derived: the plugin's collector site discharges on the owners
/// and each premise reports what it rests on.
#[test]
fn every_premise_derives_and_the_plugin_site_discharges() {
    let fixture = make_fixture("derived", &dummy_plugin("", "", ""), &test_file(""), &[]);
    let census = run(&fixture);
    for name in [
        "P-CORO-HANDLE",
        "P-PLUGIN-DIR descriptor flags",
        "immutable threaded flag",
    ] {
        assert_eq!(
            premise(&census, name)[0],
            "derived",
            "{name}: {:#?}",
            census.reading.coro_plugin_premises
        );
    }
    let registrations = premise(&census, "P-PLUGIN-DIR registrations");
    assert!(
        registrations
            .iter()
            .any(|l| l.contains("in_dummy_collect: derived")),
        "{registrations:#?}"
    );
    let current = premise(&census, "residual P-CURRENT-HANDLE");
    assert!(
        current.iter().any(|l| l.contains("flb_coro_get")),
        "{current:#?}"
    );
    discharged_on_the_owners(&verdicts(&census, "in_dummy_collect"));
}

/// Without a typed reading nothing is derived and the premises stay stated
/// (the lexical-only reading of the earlier cards; no measured reading is
/// untyped).
#[test]
fn an_untyped_reading_derives_nothing_and_reports_nothing() {
    let fixture = make_fixture("untyped", &dummy_plugin("", "", ""), &test_file(""), &[]);
    let census = census_typed(&fixture.files, &obligations(), &FLUENT_BIT_HOPS, None, None);
    assert!(census.reading.coro_plugin_premises.is_empty());
}
