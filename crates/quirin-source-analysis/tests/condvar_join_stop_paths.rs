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

//! W47-CONDVAR-JOIN-STOP-PATHS — the stop paths of the C5 census over
//! fixtures built from `flb_stop`, `do_start`, `flb_create` and
//! `mk_utils_worker_spawn` (`src/flb_lib.c`, `lib/monkey/mk_core/mk_utils.c`
//! at the `90978ac6` export), negative controls first:
//!
//! * a stop whose config can be null on the path, or whose config field is
//!   stored after publication somewhere, keeps the null arm: not discharged;
//! * a detached thread joined, a thread joined twice and a join from the
//!   thread itself are not counted: not discharged;
//! * a descendant never joined: not discharged;
//! * the fixed shape (the config dereferenced before its test, stored once
//!   before publication; a status branch whose unchecked first join has every
//!   POSIX case excluded) discharges, under P-POSIX-JOIN.

use qcore_c_gateway::thread_entry_census::CensusFile;
use qcore_core::condvar_join::{SyncVerdict, join_succeeds};
use quirin_source_analysis::condvar_join_census::{
    Census, FLUENT_BIT_SYNC, ReleaseVerdict, census,
};
use quirin_source_analysis::loop_owner_census::FLUENT_BIT_HOPS;

/// `mk_utils_worker_spawn` as at `mk_utils.c:297-328`, with the detach state
/// it sets as a parameter of the fixture.
fn wrapper(detach_state: &str) -> String {
    format!(
        r#"
struct mk_worker_start {{ void (*callback)(void *); void *argument; }};

static void *mk_utils_worker_entry(void *data)
{{
    struct mk_worker_start *start;

    start = data;
    run_start(start);
    return NULL;
}}

int mk_utils_worker_spawn(void (*func) (void *), void *arg, pthread_t *tid)
{{
    int ret;
    struct mk_worker_start *start;
    pthread_attr_t thread_attr;

    start = malloc(sizeof(*start));
    if (start == NULL) {{
        return -1;
    }}
    ret = pthread_attr_init(&thread_attr);
    if (ret != 0) {{
        free(start);
        return -1;
    }}
    ret = pthread_attr_setdetachstate(&thread_attr, {detach_state});
    if (ret == 0) {{
        ret = pthread_create(tid, &thread_attr, mk_utils_worker_entry, start);
    }}
    pthread_attr_destroy(&thread_attr);
    if (ret != 0) {{
        free(start);
        return -1;
    }}
    return 0;
}}
"#
    )
}

/// `flb_create`/`do_start`/`flb_stop` in miniature.
const LIBRARY: &str = r#"
struct engine_config { pthread_t worker; int cf_main; };
struct engine { int status; struct engine_config *config; };
typedef struct engine engine_t;

static void engine_worker(void *data)
{
    publish(data);
}

engine_t *create(void)
{
    engine_t *e;
    struct engine_config *config;

    e = flb_calloc(1, sizeof(engine_t));
    if (!e) {
        return NULL;
    }
    config = flb_calloc(1, sizeof(struct engine_config));
    if (!config) {
        flb_free(e);
        return NULL;
    }
    e->config = config;
    e->status = 0;
    return e;
}

int start(engine_t *e, void *arg)
{
    pthread_t tid;
    int ret;

    ret = mk_utils_worker_spawn(engine_worker, arg, &tid);
    if (ret == -1) {
        return -1;
    }
    e->config->worker = tid;
    e->status = 1;
    return 0;
}

int start_failing(engine_t *e, void *arg, int bad)
{
    pthread_t tid;
    int ret;

    ret = mk_utils_worker_spawn(engine_worker, arg, &tid);
    if (ret == -1) {
        return -1;
    }
    e->config->worker = tid;
    if (bad) {
        pthread_join(tid, NULL);
        e->status = -1;
        return -1;
    }
    e->status = 1;
    return 0;
}

int stop(engine_t *e)
{
    int ret;
    pthread_t tid;

    tid = e->config->worker;
    if (e->status == 0) {
        pthread_join(tid, NULL);
        return 0;
    }
    if (!e->config) {
        return 0;
    }
    ret = pthread_join(tid, NULL);
    if (ret != 0) {
        report();
    }
    return ret;
}

int stop_nullable(engine_t *e)
{
    int ret;
    pthread_t tid;

    if (!e->config) {
        return 0;
    }
    tid = e->config->worker;
    ret = pthread_join(tid, NULL);
    return ret;
}
"#;

const COMMON: &str = r#"
struct box { int done; };

void publish(void *data)
{
    struct box *b = data;

    use(b);
}
"#;

fn program(library: &str, detach_state: &str, test: &str, extra: &[(&str, &str)]) -> Census {
    let mut files = vec![
        CensusFile {
            path: "lib/monkey/mk_core/mk_utils.c".into(),
            text: wrapper(detach_state),
        },
        CensusFile {
            path: "src/flb_lib.c".into(),
            text: library.into(),
        },
        CensusFile {
            path: "tests/runtime/in_tail.c".into(),
            text: format!(
                "{COMMON}\n{test}\nTEST_LIST = {{\n    {{ \"t\", test_case }},\n    {{ NULL, NULL }}\n}};\n"
            ),
        },
    ];
    for (path, text) in extra {
        files.push(CensusFile {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    census(&files, &FLUENT_BIT_HOPS, &FLUENT_BIT_SYNC)
}

fn release<'a>(census: &'a Census, what: &str) -> &'a ReleaseVerdict {
    census
        .releases
        .iter()
        .find(|r| r.reading.root.ends_with("test_case") && r.reading.what == what)
        .unwrap_or_else(|| panic!("no {what} release: {:#?}", census.releases))
}

fn not_discharged_with(verdict: &SyncVerdict, words: &str) {
    match verdict {
        SyncVerdict::NotDischarged { reason } => {
            assert!(reason.contains(words), "{reason}");
        }
        other => panic!("expected a refusal naming `{words}`, got {other:?}"),
    }
}

const JOINABLE: &str = "PTHREAD_CREATE_JOINABLE";

/// The fixed test's shape: start, then stop whatever start returned, check
/// stop's result, then release.
fn fixed_shape(stop: &str) -> String {
    format!(
        r#"
void test_case(void)
{{
    int ret;
    engine_t *e;
    struct box b = {{0}};

    e = create();
    if (!e) {{
        return;
    }}
    ret = start(e, &b);
    ret = {stop}(e);
    if (ret != 0) {{
        exit(1);
    }}
    unlink("fixture.log");
}}
"#
    )
}

#[test]
fn nc_a_stop_whose_config_can_be_null_is_not_discharged() {
    // The null test precedes every dereference: nothing excludes its arm.
    let census = program(LIBRARY, JOINABLE, &fixed_shape("stop_nullable"), &[]);
    let test = census
        .reading
        .null_tests
        .iter()
        .find(|t| t.function == "stop_nullable" && t.field == "config")
        .expect("the null test is read");
    assert!(test.excluded.is_err(), "{test:#?}");
    not_discharged_with(
        &release(&census, "`unlink`").verdict,
        "returned from `stop_nullable`",
    );
}

#[test]
fn nc_a_config_stored_after_publication_keeps_the_null_arm() {
    // Somewhere else, a published engine's config is cleared.
    let library = format!("{LIBRARY}\nvoid reset(engine_t *e)\n{{\n    e->config = NULL;\n}}\n");
    let census = program(&library, JOINABLE, &fixed_shape("stop"), &[]);
    let test = census
        .reading
        .null_tests
        .iter()
        .find(|t| t.function == "stop" && t.field == "config")
        .expect("the null test is read");
    let why = test.excluded.clone().unwrap_err();
    assert!(why.contains("not a pre-publication store"), "{why}");
    not_discharged_with(
        &release(&census, "`unlink`").verdict,
        "returned from `stop`",
    );
}

#[test]
fn nc_a_detached_thread_joined_is_not_discharged() {
    let census = program(
        LIBRARY,
        "PTHREAD_CREATE_DETACHED",
        &fixed_shape("stop"),
        &[],
    );
    not_discharged_with(&release(&census, "`unlink`").verdict, "not joinable");
}

#[test]
fn nc_a_thread_joined_twice_is_not_discharged() {
    // `do_start`'s failure path joins the engine thread; the test stops it
    // anyway, and the status branch joins it again.
    let test = r#"
void test_case(void)
{
    int ret;
    engine_t *e;
    struct box b = {0};

    e = create();
    if (!e) {
        return;
    }
    ret = start_failing(e, &b, 1);
    ret = stop(e);
    if (ret != 0) {
        exit(1);
    }
    unlink("fixture.log");
}
"#;
    let census = program(LIBRARY, JOINABLE, test, &[]);
    not_discharged_with(
        &release(&census, "`unlink`").verdict,
        "joined a second time",
    );
}

#[test]
fn nc_a_join_from_the_thread_itself_is_not_counted() {
    let census = program(LIBRARY, JOINABLE, &fixed_shape("stop"), &[]);
    let own = census
        .facts
        .exclusions("engine_worker", "mk_utils_worker_spawn", "engine_worker");
    assert!(own.not_self.is_err(), "{own:#?}");
    assert!(join_succeeds(&own).is_err());
}

#[test]
fn nc_a_descendant_never_joined_is_not_discharged() {
    // The engine thread starts a child it never joins; `flb_destroy` releases
    // the storage every live thread retains.
    let library = LIBRARY.replace(
        "static void engine_worker(void *data)\n{\n    publish(data);\n}",
        "static void *child(void *data)\n{\n    return data;\n}\n\n\
         static void engine_worker(void *data)\n{\n    pthread_t t;\n\n    \
         pthread_create(&t, NULL, child, data);\n    publish(data);\n}",
    );
    let test = fixed_shape("stop").replace("unlink(\"fixture.log\");", "flb_destroy(e);");
    let census = program(&library, JOINABLE, &test, &[]);
    not_discharged_with(&release(&census, "`flb_destroy`").verdict, "never joined");
}

#[test]
fn the_fixed_shape_excludes_the_null_arm_and_counts_the_first_join() {
    let census = program(LIBRARY, JOINABLE, &fixed_shape("stop"), &[]);
    let test = census
        .reading
        .null_tests
        .iter()
        .find(|t| t.function == "stop" && t.field == "config")
        .expect("the null test is read");
    assert_eq!(test.excluded, Ok(()), "{test:#?}");
    let unlink = release(&census, "`unlink`");
    assert_eq!(unlink.verdict, SyncVerdict::Discharged, "{unlink:#?}");
    assert!(unlink.premises.contains("P-POSIX-JOIN"), "{unlink:#?}");
}

#[test]
fn a_joined_descendant_lets_the_release_discharge() {
    let library = LIBRARY.replace(
        "static void engine_worker(void *data)\n{\n    publish(data);\n}",
        "static void *child(void *data)\n{\n    return data;\n}\n\n\
         static void engine_worker(void *data)\n{\n    pthread_t t;\n    int ret;\n\n    \
         pthread_create(&t, NULL, child, data);\n    publish(data);\n    \
         ret = pthread_join(t, NULL);\n    if (ret != 0) {\n        abort();\n    }\n}",
    );
    let test = fixed_shape("stop").replace("unlink(\"fixture.log\");", "flb_destroy(e);");
    let census = program(&library, JOINABLE, &test, &[]);
    let destroy = release(&census, "`flb_destroy`");
    assert_eq!(destroy.verdict, SyncVerdict::Discharged, "{destroy:#?}");
}
