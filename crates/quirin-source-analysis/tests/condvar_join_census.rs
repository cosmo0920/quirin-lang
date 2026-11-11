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

//! W47-CONDVAR-JOIN-PUBLICATION — the condvar/join census over fixtures built
//! from the bound shapes of `docs/reports/w47-tail-pause-bindings/bindings.json`
//! (B41–B58), negative controls first, then the production contract module:
//! its clauses load, a malformed clause rejects the module, and the per-unit
//! resource route refuses every declared call.
//!
//! The fixture is a miniature of the in_tail runtime test binary: `start`
//! spawns the engine worker through `mk_utils_worker_spawn` and stores its
//! identifier in `e->worker`, the worker runs the test's callback `publish`
//! (defined in the test file, so the worker retains the test's storage), and
//! `stop` joins it. Each test is one NC case.

use std::collections::BTreeMap;

use qcore_c_gateway::thread_entry_census::CensusFile;
use qcore_contract::contract_loader::{
    ThreadSyncOperation, load_resource_contract_sources, thread_sync_obligations,
};
use qcore_core::condvar_join::{KeptRefusal, SyncVerdict};
use quirin_source_analysis::condvar_join_census::{
    Census, FLUENT_BIT_SYNC, ReleaseVerdict, SiteVerdict, census,
};
use quirin_source_analysis::loop_owner_census::FLUENT_BIT_HOPS;

const CONTRACTS: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/pthread_sync.qv");

/// The engine: a checked join on every path (the NC2 control's stop).
const LIBRARY: &str = r#"
struct engine { pthread_t worker; void *data; };

static void engine_worker(void *data)
{
    publish(data);
}

int start(struct engine *e, void *arg)
{
    pthread_t tid;
    int ret;

    ret = mk_utils_worker_spawn(engine_worker, arg, &tid);
    if (ret == -1) {
        return -1;
    }
    e->worker = tid;
    return 0;
}

int stop(struct engine *e)
{
    int ret;
    pthread_t tid;

    tid = e->worker;
    ret = pthread_join(tid, NULL);
    return ret;
}

int stop_r2(struct engine *e, int status)
{
    int ret;
    pthread_t tid;

    tid = e->worker;
    if (status == 0) {
        pthread_join(tid, NULL);
        return 0;
    }
    ret = pthread_join(tid, NULL);
    return ret;
}
"#;

/// The test file's shared part: the storage and the publishing callback.
const COMMON: &str = r#"
struct box {
    pthread_mutex_t mutex;
    pthread_cond_t cond;
    int done;
    int result;
};

void publish(void *data)
{
    struct box *b = data;

    pthread_mutex_lock(&b->mutex);
    b->result = 42;
    b->done = 1;
    pthread_cond_signal(&b->cond);
    pthread_mutex_unlock(&b->mutex);
}
"#;

const NC2: &str = r#"
void test_nc2(void)
{
    int ret;
    struct engine e;
    struct box b = {0};

    pthread_mutex_init(&b.mutex, NULL);
    pthread_cond_init(&b.cond, NULL);
    e.data = &b;
    ret = start(&e, &b);
    pthread_mutex_lock(&b.mutex);
    while (!b.done) {
        pthread_cond_wait(&b.cond, &b.mutex);
    }
    use(b.result);
    pthread_mutex_unlock(&b.mutex);
    ret = stop(&e);
    if (ret != 0) {
        exit(1);
    }
    pthread_cond_destroy(&b.cond);
    pthread_mutex_destroy(&b.mutex);
}
"#;

fn program(tests: &[&str], table: &str, extra: &[(&str, &str)]) -> Census {
    let mut files = vec![
        CensusFile {
            path: "src/flb_lib.c".into(),
            text: LIBRARY.into(),
        },
        CensusFile {
            path: "tests/runtime/in_tail.c".into(),
            text: format!(
                "{COMMON}\n{}\nTEST_LIST = {{\n{table}\n    {{ NULL, NULL }}\n}};\n",
                tests.join("\n")
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

fn sites<'a>(census: &'a Census, function: &str, kind: &str) -> Vec<&'a SiteVerdict> {
    let found = census
        .sites
        .iter()
        .filter(|s| s.function.ends_with(function) && s.kind == kind)
        .collect::<Vec<_>>();
    assert!(
        !found.is_empty(),
        "no {kind} site in {function}: {:#?}",
        census.sites
    );
    found
}

fn releases<'a>(census: &'a Census, root: &str) -> Vec<&'a ReleaseVerdict> {
    let found = census
        .releases
        .iter()
        .filter(|r| r.reading.root.ends_with(root))
        .collect::<Vec<_>>();
    assert!(
        !found.is_empty(),
        "no release under {root}: {:#?}",
        census.releases
    );
    found
}

fn not_discharged_with(verdict: &SyncVerdict, words: &str) {
    match verdict {
        SyncVerdict::NotDischarged { reason } => {
            assert!(reason.contains(words), "{reason}");
        }
        other => panic!("expected a refusal naming `{words}`, got {other:?}"),
    }
}

#[test]
fn nc4_publication_is_not_a_join() {
    // E publishes `done`; T observes it and releases before any join.
    let test = r#"
void test_nc4(void)
{
    int ret;
    struct engine e;
    struct box b = {0};

    pthread_mutex_init(&b.mutex, NULL);
    pthread_cond_init(&b.cond, NULL);
    e.data = &b;
    ret = start(&e, &b);
    pthread_mutex_lock(&b.mutex);
    while (!b.done) {
        pthread_cond_wait(&b.cond, &b.mutex);
    }
    pthread_mutex_unlock(&b.mutex);
    pthread_cond_destroy(&b.cond);
    ret = stop(&e);
}
"#;
    let census = program(&[test], r#"    { "nc4", test_nc4 },"#, &[]);
    let found = releases(&census, "test_nc4");
    let destroy = found
        .iter()
        .find(|r| r.reading.what == "`pthread_cond_destroy`")
        .expect("the destroy is a release");
    not_discharged_with(&destroy.verdict, "publication is not a join");
    // The condvar's own C4 quiescence is a different obligation: the only
    // signaller signals once per thread (derived) and the waiter observed the
    // predicate under the mutex, so no signal follows and no waiter blocks.
    // That admits the `destroy` as a condvar operation; it grants nothing to
    // the storage the worker may still touch, which C5 refuses above.
    let lifecycle = sites(&census, "test_nc4", "destroy");
    assert!(
        lifecycle.iter().all(|s| s.verdict.admits()),
        "{lifecycle:#?}"
    );
}

#[test]
fn nc5_a_timeout_is_not_cancellation() {
    let test = r#"
void test_nc5(void)
{
    int ret;
    struct engine e;
    struct box b = {0};
    struct timespec deadline;

    pthread_mutex_init(&b.mutex, NULL);
    pthread_cond_init(&b.cond, NULL);
    e.data = &b;
    ret = start(&e, &b);
    pthread_mutex_lock(&b.mutex);
    while (!b.done) {
        ret = pthread_cond_timedwait(&b.cond, &b.mutex, &deadline);
        if (ret != 0) {
            break;
        }
    }
    pthread_mutex_unlock(&b.mutex);
    if (ret != 0) {
        pthread_cond_destroy(&b.cond);
        return;
    }
    ret = stop(&e);
    if (ret != 0) {
        exit(1);
    }
}
"#;
    let census = program(&[test], r#"    { "nc5", test_nc5 },"#, &[]);
    let found = releases(&census, "test_nc5");
    let destroy = found
        .iter()
        .find(|r| r.reading.what == "`pthread_cond_destroy`")
        .expect("the destroy is a release");
    not_discharged_with(&destroy.verdict, "timeout is not cancellation");
    // The timed wait itself, in its predicate loop, is admitted.
    assert!(sites(&census, "test_nc5", "wait")[0].verdict.admits());
}

#[test]
fn nc9_fixture_removal_before_the_join_is_rejected() {
    let test = r#"
static void teardown(struct engine *e)
{
    unlink("fixture.log");
    stop(e);
}

void test_nc9(void)
{
    int ret;
    struct engine e;

    ret = start(&e, 0);
    teardown(&e);
}
"#;
    let census = program(&[test], r#"    { "nc9", test_nc9 },"#, &[]);
    let found = releases(&census, "test_nc9");
    let unlink = found
        .iter()
        .find(|r| r.reading.what == "`unlink`")
        .expect("the unlink is a release");
    not_discharged_with(&unlink.verdict, "released before any join");
}

#[test]
fn an_unchecked_join_branch_cannot_discharge_r2() {
    // `flb_stop`'s shape: one branch joins and returns 0 without checking.
    let test = r#"
void test_r2(void)
{
    int ret;
    struct engine e;

    ret = start(&e, 0);
    ret = stop_r2(&e, ret);
    if (ret != 0) {
        exit(1);
    }
    unlink("fixture.log");
}
"#;
    let census = program(&[test], r#"    { "r2", test_r2 },"#, &[]);
    let unlink = releases(&census, "test_r2")
        .into_iter()
        .find(|r| r.reading.what == "`unlink`")
        .expect("the unlink is a release");
    not_discharged_with(&unlink.verdict, "is not checked");
}

#[test]
fn nc8_a_wait_without_its_predicate_loop_makes_no_result_claim() {
    let test = r#"
void test_nc8(void)
{
    int ret;
    struct engine e;
    struct box b = {0};

    pthread_mutex_init(&b.mutex, NULL);
    pthread_cond_init(&b.cond, NULL);
    e.data = &b;
    ret = start(&e, &b);
    pthread_mutex_lock(&b.mutex);
    pthread_cond_wait(&b.cond, &b.mutex);
    use(b.result);
    pthread_mutex_unlock(&b.mutex);
    ret = stop(&e);
    if (ret != 0) {
        exit(1);
    }
}
"#;
    // NC2 in the same program supplies the condvar's predicate.
    let census = program(
        &[NC2, test],
        r#"    { "nc2", test_nc2 },
    { "nc8", test_nc8 },"#,
        &[],
    );
    assert_eq!(
        sites(&census, "test_nc8", "wait")[0].verdict,
        SyncVerdict::Refused(KeptRefusal::WaitOutsidePredicateLoop)
    );
    let read = sites(&census, "test_nc8", "read");
    assert!(read.iter().all(|s| !s.verdict.admits()), "{read:#?}");
}

#[test]
fn nc2_the_publish_await_join_path_is_admitted() {
    let census = program(&[NC2], r#"    { "nc2", test_nc2 },"#, &[]);
    assert!(
        sites(&census, "publish", "signal")[0].verdict.admits(),
        "{:#?}",
        census.sites
    );
    assert!(sites(&census, "test_nc2", "wait")[0].verdict.admits());
    assert!(sites(&census, "test_nc2", "init")[0].verdict.admits());
    for read in sites(&census, "test_nc2", "read") {
        assert!(read.verdict.admits(), "{read:#?}");
    }
    assert_eq!(
        census.results.get("box.cond").cloned().unwrap_or_default(),
        ["box.result".to_owned()].into_iter().collect()
    );
    for release in releases(&census, "test_nc2") {
        assert!(release.verdict.admits(), "{release:#?}");
    }
    for destroy in sites(&census, "test_nc2", "destroy") {
        assert!(destroy.verdict.admits(), "{destroy:#?}");
    }
}

#[test]
fn a_result_written_after_the_predicate_is_not_a_publication() {
    let late = COMMON.replace(
        "    b->result = 42;\n    b->done = 1;\n",
        "    b->done = 1;\n    b->result = 42;\n",
    );
    let mut census_files = vec![
        CensusFile {
            path: "src/flb_lib.c".into(),
            text: LIBRARY.into(),
        },
        CensusFile {
            path: "tests/runtime/in_tail.c".into(),
            text: format!(
                "{late}\n{NC2}\nTEST_LIST = {{ {{ \"nc2\", test_nc2 }}, {{ NULL, NULL }} }};\n"
            ),
        },
    ];
    let census = census(
        &std::mem::take(&mut census_files),
        &FLUENT_BIT_HOPS,
        &FLUENT_BIT_SYNC,
    );
    // `done` is written first, so `result` is not a published result at all:
    // the section writes it after the predicate.
    let signal = sites(&census, "publish", "signal");
    assert!(
        !signal[0].verdict.admits() || !census.results.contains_key("box.cond"),
        "{signal:#?}"
    );
}

#[test]
fn kept_refusals_refuse() {
    let extra = r#"
struct other { pthread_mutex_t a; pthread_mutex_t b; pthread_cond_t cond; int ready; };
static pthread_cond_t global_cond = PTHREAD_COND_INITIALIZER;

void broadcast_all(struct box *b)
{
    pthread_mutex_lock(&b->mutex);
    b->done = 1;
    pthread_cond_broadcast(&b->cond);
    pthread_mutex_unlock(&b->mutex);
}

void signal_unlocked(struct box *b)
{
    b->done = 1;
    pthread_cond_signal(&b->cond);
}

void two_mutexes(struct other *o)
{
    pthread_mutex_lock(&o->a);
    while (!o->ready) {
        pthread_cond_wait(&o->cond, &o->a);
    }
    pthread_mutex_unlock(&o->a);
    pthread_mutex_lock(&o->b);
    while (!o->ready) {
        pthread_cond_wait(&o->cond, &o->b);
    }
    pthread_mutex_unlock(&o->b);
}

void c11_and_rwlock(cnd_t *c, mtx_t *m, pthread_rwlock_t *rw, pthread_mutex_t *mu)
{
    cnd_wait(c, m);
    pthread_rwlock_rdlock(rw);
    pthread_mutex_trylock(mu);
}
"#;
    let census = program(
        &[NC2],
        r#"    { "nc2", test_nc2 },"#,
        &[("src/extra.c", extra)],
    );
    assert_eq!(
        sites(&census, "broadcast_all", "broadcast")[0].verdict,
        SyncVerdict::Refused(KeptRefusal::Broadcast)
    );
    assert_eq!(
        sites(&census, "signal_unlocked", "signal")[0].verdict,
        SyncVerdict::Refused(KeptRefusal::SignalWithoutMutex)
    );
    for wait in sites(&census, "two_mutexes", "wait") {
        assert_eq!(
            wait.verdict,
            SyncVerdict::Refused(KeptRefusal::CondvarWithTwoMutexes)
        );
    }
    let refused = census
        .sites
        .iter()
        .filter(|s| s.kind == "refused")
        .map(|s| s.verdict.clone())
        .collect::<Vec<_>>();
    for kind in [
        KeptRefusal::C11Condvar,
        KeptRefusal::Rwlock,
        KeptRefusal::TryOrTimedLock,
        KeptRefusal::StaticInitializer,
    ] {
        assert!(
            refused.contains(&SyncVerdict::Refused(kind)),
            "{kind:?}: {refused:?}"
        );
    }
}

/// The worker status handshake (B52–B54): one signal per worker lifetime.
const HANDSHAKE: &str = r#"
struct instance { pthread_mutex_t init_mutex; pthread_cond_t init_condition; int init_status; };

static void set_status(struct instance *thi, int status)
{
    pthread_mutex_lock(&thi->init_mutex);
    thi->init_status = status;
    pthread_cond_signal(&thi->init_condition);
    pthread_mutex_unlock(&thi->init_mutex);
}

static int get_status(struct instance *thi)
{
    int status;

    pthread_mutex_lock(&thi->init_mutex);
    while (thi->init_status == 0) {
        pthread_cond_wait(&thi->init_condition, &thi->init_mutex);
    }
    pthread_mutex_unlock(&thi->init_mutex);
    pthread_cond_destroy(&thi->init_condition);
    pthread_cond_init(&thi->init_condition, NULL);
    status = thi->init_status;
    return status;
}

static struct instance *create(void)
{
    struct instance *thi;

    thi = flb_calloc(1, sizeof(struct instance));
    if (!thi) {
        return NULL;
    }
    thi->init_status = 0;
    pthread_mutex_init(&thi->init_mutex, NULL);
    pthread_cond_init(&thi->init_condition, NULL);
    return thi;
}

static void worker(void *data)
{
    int ret;
    struct instance *thi = data;

    ret = setup(thi);
    if (ret == -1) {
        set_status(thi, 2);
        return;
    }
    set_status(thi, 1);
    run(thi);
}

int instance_init(struct instance *thi)
{
    pthread_t tid;

    mk_utils_worker_spawn(worker, thi, &tid);
    return get_status(thi);
}
"#;

#[test]
fn the_worker_status_handshake_is_admitted_under_the_derived_single_signal() {
    let census = program(
        &[NC2],
        r#"    { "nc2", test_nc2 },"#,
        &[("src/instance.c", HANDSHAKE)],
    );
    for kind in ["wait", "destroy", "init", "read"] {
        for site in sites(&census, "get_status", kind) {
            assert!(site.verdict.admits(), "{kind}: {site:#?}");
        }
    }
    assert!(sites(&census, "set_status", "signal")[0].verdict.admits());
    assert!(sites(&census, "create", "init")[0].verdict.admits());
}

#[test]
fn a_second_signal_on_a_path_withholds_the_reinit() {
    let twice = HANDSHAKE.replace(
        "        set_status(thi, 2);\n        return;\n",
        "        set_status(thi, 2);\n",
    );
    let census = program(
        &[NC2],
        r#"    { "nc2", test_nc2 },"#,
        &[("src/instance.c", &twice)],
    );
    // The wait itself stays admitted; the re-initialisation after the unlock
    // and the unlocked read are not.
    assert!(sites(&census, "get_status", "wait")[0].verdict.admits());
    for kind in ["destroy", "read"] {
        for site in sites(&census, "get_status", kind) {
            assert!(!site.verdict.admits(), "{kind}: {site:#?}");
        }
    }
}

#[test]
fn an_unreadable_body_leaves_its_sites_unresolved() {
    let goto = r#"
struct gate { pthread_mutex_t m; pthread_cond_t c; int ok; };
void jumpy(struct gate *g)
{
    pthread_mutex_lock(&g->m);
again:
    while (!g->ok) {
        pthread_cond_wait(&g->c, &g->m);
    }
    goto again;
}
"#;
    let census = program(
        &[NC2],
        r#"    { "nc2", test_nc2 },"#,
        &[("src/goto.c", goto)],
    );
    for site in sites(&census, "jumpy", "wait") {
        assert!(
            matches!(site.verdict, SyncVerdict::Unresolved { .. }),
            "{site:#?}"
        );
    }
}

#[test]
fn the_production_module_declares_six_obligations() {
    let declared = thread_sync_obligations(&BTreeMap::from([(
        "libc/pthread_sync.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the production module loads");
    assert_eq!(declared.len(), 6);
    assert_eq!(
        declared["pthread_cond_wait"].operation,
        ThreadSyncOperation::CondWait
    );
    assert_eq!(declared["pthread_cond_wait"].mutex, Some(1));
    assert_eq!(
        declared["pthread_cond_timedwait"].operation,
        ThreadSyncOperation::CondTimedWait
    );
    assert_eq!(
        declared["pthread_join"].operation,
        ThreadSyncOperation::Join
    );
    assert!(!declared.contains_key("pthread_create"));
    assert!(!declared.contains_key("pthread_cond_broadcast"));
}

#[test]
fn a_malformed_or_duplicate_obligation_rejects_the_module() {
    for clause in [
        "requires resource_thread_sync(cond, cond_wait);",
        "requires resource_thread_sync(cond, mutex, cond_signal);",
        "requires resource_thread_sync(cond, cond_broadcast);",
        "requires resource_thread_sync(cond, cond, cond_wait);",
        "requires resource_thread_sync(n, join);",
        "requires resource_thread_sync(cond, cond_init); requires resource_thread_sync(cond, cond_init);",
        "requires resource_thread_sync(nobody, cond_signal);",
        "requires resource_thread_sync(n, cond_signal);",
    ] {
        let module = format!(
            "module contract;\nextern fn g(cond: ptr<mut, u8>, mutex: ptr<mut, u8>, n: i32) -> i32\n\
             contract {{\n    requires resource_contract_v1;\n    {clause}\n    ensures normal \
             resource_borrowed(cond);\n    ensures normal resource_borrowed(mutex);\n    ensures \
             normal resource_borrowed(n);\n    ensures normal resource_return_none();\n}};\n"
        );
        let sources = BTreeMap::from([("m.qv".to_owned(), module)]);
        assert!(thread_sync_obligations(&sources).is_err(), "{clause}");
        assert!(
            load_resource_contract_sources(&sources).is_err(),
            "{clause}"
        );
    }
}

#[test]
fn every_resource_route_refuses_a_declared_call() {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "libc/pthread_sync.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("resource-v1 loads the module");
    assert!(contracts.contract("pthread_cond_signal").is_some());
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let source = "typedef union { char s[48]; } pthread_cond_t; \
        int pthread_cond_signal(pthread_cond_t *cond); \
        void f(pthread_cond_t *c) { pthread_cond_signal(c); }";
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
            "call to `pthread_cond_signal` outside its declared thread-synchronization obligation"
        ),
        "{rendered}"
    );
    assert!(!matches!(
        result,
        qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Safe(_)
    ));
}
