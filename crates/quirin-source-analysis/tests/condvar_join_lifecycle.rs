//! W47-FLB-LIB-LIFECYCLE — the paired start/stop lifecycle over fixtures
//! built from `flb_create`, `do_start`, `flb_stop` and `mk_utils_worker_spawn`
//! in miniature (the stop-path card's library), declared through
//! `requires resource_lifecycle(ctx, start|stop|destroy)`. Negative controls
//! first:
//!
//! * a double start is refused (the path reader alone would have discharged
//!   it: the second engine thread collapses into the first);
//! * a stop that joins again after a failed start refutes L4, and the release
//!   stays not discharged;
//! * a stop of a never-started context is permitted and governed by L5, which
//!   a stop that joins anyway refutes;
//! * a destroy before the stop is refused;
//! * the fixed shape conforms, L2 is in force and the release is unchanged.

use std::collections::BTreeMap;

use qcore_c_gateway::thread_entry_census::CensusFile;
use qcore_contract::contract_loader::{
    LifecycleOperation, lifecycle_obligations, load_resource_contract_sources,
};
use qcore_core::condvar_join::SyncVerdict;
use qcore_core::paired_lifecycle::{Clause, ClauseStatus};
use quirin_source_analysis::condvar_join_census::{
    Census, FLUENT_BIT_SYNC, ReleaseVerdict, census, census_with_lifecycle,
};
use quirin_source_analysis::lifecycle_census::{LifecycleCensus, governing_at};
use quirin_source_analysis::loop_owner_census::FLUENT_BIT_HOPS;

const WRAPPER: &str = r#"
struct mk_worker_start { void (*callback)(void *); void *argument; };

static void *mk_utils_worker_entry(void *data)
{
    struct mk_worker_start *start;

    start = data;
    run_start(start);
    return NULL;
}

int mk_utils_worker_spawn(void (*func) (void *), void *arg, pthread_t *tid)
{
    int ret;
    struct mk_worker_start *start;
    pthread_attr_t thread_attr;

    start = malloc(sizeof(*start));
    if (start == NULL) {
        return -1;
    }
    ret = pthread_attr_init(&thread_attr);
    if (ret != 0) {
        free(start);
        return -1;
    }
    ret = pthread_attr_setdetachstate(&thread_attr, PTHREAD_CREATE_JOINABLE);
    if (ret == 0) {
        ret = pthread_create(tid, &thread_attr, mk_utils_worker_entry, start);
    }
    pthread_attr_destroy(&thread_attr);
    if (ret != 0) {
        free(start);
        return -1;
    }
    return 0;
}
"#;

/// `flb_create`/`do_start`/`flb_stop` in miniature; `start_failing` fails after its
/// spawn when `bad` (joining the thread, as `do_start` does), and
/// `stop_once` returns at once unless the engine is started.
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

int start(engine_t *e, void *arg, int bad)
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

int stop_once(engine_t *e)
{
    int ret;
    pthread_t tid;

    if (e->status != 1) {
        return 0;
    }
    tid = e->config->worker;
    e->status = 0;
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

/// The declared lifecycle of the miniature: `start` and `start_failing`,
/// `STOP` (the stop under test) and `flb_destroy`.
fn module(stop: &str) -> String {
    format!(
        "module contract;\n\
         extern fn start(e: ptr<mut, u8>, arg: ptr<mut, u8>, bad: i32) -> i32\n\
         contract {{\n    requires resource_contract_v1;\n    requires resource_lifecycle(e, start);\n    \
         ensures normal resource_borrowed(e);\n    ensures normal resource_borrowed(arg);\n    \
         ensures normal resource_borrowed(bad);\n    ensures normal resource_return_none();\n}};\n\
         extern fn start_failing(e: ptr<mut, u8>, arg: ptr<mut, u8>, bad: i32) -> i32\n\
         contract {{\n    requires resource_contract_v1;\n    requires resource_lifecycle(e, start);\n    \
         ensures normal resource_borrowed(e);\n    ensures normal resource_borrowed(arg);\n    \
         ensures normal resource_borrowed(bad);\n    ensures normal resource_return_none();\n}};\n\
         extern fn {stop}(e: ptr<mut, u8>) -> i32\n\
         contract {{\n    requires resource_contract_v1;\n    requires resource_lifecycle(e, stop);\n    \
         ensures normal resource_borrowed(e);\n    ensures normal resource_return_none();\n}};\n\
         extern fn flb_destroy(e: ptr<mut, u8>) -> unit\n\
         contract {{\n    requires resource_contract_v1;\n    requires resource_lifecycle(e, destroy);\n    \
         ensures normal resource_released(e, nullable);\n    ensures normal resource_return_none();\n}};\n"
    )
}

fn files(test: &str) -> Vec<CensusFile> {
    vec![
        CensusFile {
            path: "lib/monkey/mk_core/mk_utils.c".into(),
            text: WRAPPER.into(),
        },
        CensusFile {
            path: "src/flb_lib.c".into(),
            text: LIBRARY.into(),
        },
        CensusFile {
            path: "tests/runtime/in_tail.c".into(),
            text: format!(
                "{COMMON}\n{test}\nTEST_LIST = {{\n    {{ \"t\", test_case }},\n    {{ NULL, NULL }}\n}};\n"
            ),
        },
    ]
}

fn run(test: &str, stop: &str) -> Census {
    let declared = lifecycle_obligations(&BTreeMap::from([("m.qv".to_owned(), module(stop))]))
        .expect("the fixture module declares its obligations");
    census_with_lifecycle(
        &files(test),
        &FLUENT_BIT_HOPS,
        &FLUENT_BIT_SYNC,
        &declared,
        &["create"],
    )
}

fn release<'a>(census: &'a Census, what: &str) -> &'a ReleaseVerdict {
    census
        .releases
        .iter()
        .find(|r| r.reading.root.ends_with("test_case") && r.reading.what == what)
        .unwrap_or_else(|| panic!("no {what} release: {:#?}", census.releases))
}

fn lifecycle(census: &Census) -> &LifecycleCensus {
    census
        .lifecycle
        .as_ref()
        .expect("lifecycle roles were given")
}

fn status(census: &Census, clause: Clause, function: Option<&str>) -> ClauseStatus {
    lifecycle(census)
        .status
        .get(&(clause, function.map(str::to_owned)))
        .cloned()
        .unwrap_or_else(|| panic!("no status for {clause:?}: {:#?}", lifecycle(census).status))
}

fn not_discharged_with(verdict: &SyncVerdict, words: &str) {
    match verdict {
        SyncVerdict::NotDischarged { reason } => assert!(reason.contains(words), "{reason}"),
        other => panic!("expected a refusal naming `{words}`, got {other:?}"),
    }
}

/// Create, the body, then check `stop`'s result and release the fixture.
fn shape(body: &str) -> String {
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
{body}
    unlink("fixture.log");
}}
"#
    )
}

const FIXED: &str =
    "    ret = start(e, &b, 0);\n    ret = stop(e);\n    if (ret != 0) {\n        exit(1);\n    }";

#[test]
fn nc_a_double_start_is_refused() {
    let body = "    ret = start(e, &b, 0);\n    ret = start(e, &b, 0);\n    ret = stop(e);\n    \
                if (ret != 0) {\n        exit(1);\n    }";
    // Without the lifecycle the reader keys the engine thread by its entry, so
    // the second start's thread collapses into the first and one checked
    // join looks like it ends both: the release discharges.
    let plain = census(&files(&shape(body)), &FLUENT_BIT_HOPS, &FLUENT_BIT_SYNC);
    assert_eq!(release(&plain, "`unlink`").verdict, SyncVerdict::Discharged);
    // Under the declared lifecycle the second start violates L1.
    let census = run(&shape(body), "stop");
    let unlink = release(&census, "`unlink`");
    not_discharged_with(&unlink.verdict, "paired lifecycle is violated");
    assert!(
        unlink
            .lifecycle_violations
            .iter()
            .any(|v| v.contains("without a stop")),
        "{unlink:#?}"
    );
    let second = lifecycle(&census)
        .calls
        .iter()
        .find(|c| c.reading.callee == "start" && !c.violations.is_empty())
        .expect("the second start is a violating call");
    assert!(second.states.contains("started"), "{second:#?}");
}

#[test]
fn nc_a_stop_that_joins_again_after_a_failed_start_refutes_l4() {
    let census = run(
        &shape(
            "    ret = start_failing(e, &b, 1);\n    ret = stop(e);\n    if (ret != 0) {\n        exit(1);\n    }",
        ),
        "stop",
    );
    assert!(
        matches!(
            status(&census, Clause::StopAfterFailedStartNoEffect, Some("stop")),
            ClauseStatus::Refuted { .. }
        ),
        "{:#?}",
        lifecycle(&census).clauses
    );
    // Withdrawn: the stop is read from its body, and the second join stands.
    let unlink = release(&census, "`unlink`");
    not_discharged_with(&unlink.verdict, "joined a second time");
    assert!(unlink.lifecycle_violations.is_empty(), "{unlink:#?}");
    let stops = governing_at(&unlink.reading, lifecycle(&census));
    assert!(
        stops
            .iter()
            .any(|s| s.contains("L4") && s.contains("withdrawn: read from the body")),
        "{stops:#?}"
    );
}

#[test]
fn nc_a_stop_of_a_never_started_context_is_governed_by_l5() {
    let census = run(
        &shape("    ret = stop(e);\n    if (ret != 0) {\n        exit(1);\n    }"),
        "stop",
    );
    // The stop joins the identifier no start produced: L5 is refuted.
    match status(&census, Clause::StopUnstartedNoEffect, Some("stop")) {
        ClauseStatus::Refuted { witness } => {
            assert!(witness.contains("no start began"), "{witness}");
        }
        other => panic!("{other:?}"),
    }
    // The call itself is permitted: created, no violation.
    let call = lifecycle(&census)
        .calls
        .iter()
        .find(|c| c.reading.callee == "stop")
        .expect("the stop is read");
    assert!(call.violations.is_empty(), "{call:#?}");
    assert!(call.states.contains("created"), "{call:#?}");
    assert!(
        call.governed
            .iter()
            .any(|(clause, _)| *clause == Clause::StopUnstartedNoEffect)
    );
}

#[test]
fn nc_a_destroy_before_stop_is_refused() {
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
    ret = start(e, &b, 0);
    flb_destroy(e);
}
"#;
    let census = run(test, "stop");
    let destroy = release(&census, "`flb_destroy`");
    not_discharged_with(&destroy.verdict, "before its stop");
}

#[test]
fn a_stop_that_joins_on_some_paths_only_holds_its_clause() {
    // `stop_once` returns early unless started; the reader follows both arms
    // of the status test, so after a stop some followed paths join again and
    // some do not: neither in force nor refuted.
    let census = run(
        &shape(
            "    ret = start(e, &b, 0);\n    ret = stop_once(e);\n    if (ret != 0) {\n        exit(1);\n    }",
        ),
        "stop_once",
    );
    assert!(
        matches!(
            status(&census, Clause::StopAfterStopNoEffect, Some("stop_once")),
            ClauseStatus::Held { .. }
        ),
        "{:#?}",
        lifecycle(&census).clauses
    );
}

#[test]
fn the_fixed_shape_conforms_and_its_release_is_unchanged() {
    let plain = census(&files(&shape(FIXED)), &FLUENT_BIT_HOPS, &FLUENT_BIT_SYNC);
    let census = run(&shape(FIXED), "stop");
    let before = release(&plain, "`unlink`");
    let after = release(&census, "`unlink`");
    assert_eq!(before.verdict, after.verdict);
    assert!(after.lifecycle_violations.is_empty());
    assert_eq!(
        status(&census, Clause::StopEndsStarted, Some("stop")),
        ClauseStatus::InForce,
        "{:#?}",
        lifecycle(&census).clauses
    );
    assert!(matches!(
        status(&census, Clause::StopAfterStopNoEffect, Some("stop")),
        ClauseStatus::Refuted { .. }
    ));
    assert_eq!(
        status(&census, Clause::StartRequiresUnstarted, None),
        ClauseStatus::InForce
    );
    for call in &lifecycle(&census).calls {
        if call.reading.root.ends_with("test_case") {
            assert!(call.violations.is_empty(), "{call:#?}");
        }
    }
}

#[test]
fn the_lifecycle_clause_loads_and_a_malformed_one_rejects_the_module() {
    let declared = lifecycle_obligations(&BTreeMap::from([("m.qv".to_owned(), module("stop"))]))
        .expect("the fixture module loads");
    assert_eq!(declared["start"].operation, LifecycleOperation::Start);
    assert_eq!(declared["stop"].operation, LifecycleOperation::Stop);
    assert_eq!(declared["flb_destroy"].parameter, 0);
    assert!(
        load_resource_contract_sources(&BTreeMap::from([("m.qv".to_owned(), module("stop"))]))
            .is_ok()
    );
    for clause in [
        "requires resource_lifecycle(e, restart);",
        "requires resource_lifecycle(e);",
        "requires resource_lifecycle(n, stop);",
        "requires resource_lifecycle(nobody, stop);",
        "requires resource_lifecycle(e, stop); requires resource_lifecycle(e, stop);",
    ] {
        let text = format!(
            "module contract;\nextern fn g(e: ptr<mut, u8>, n: i32) -> i32\ncontract {{\n    \
             requires resource_contract_v1;\n    {clause}\n    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(n);\n    ensures normal resource_return_none();\n}};\n"
        );
        let sources = BTreeMap::from([("m.qv".to_owned(), text)]);
        assert!(lifecycle_obligations(&sources).is_err(), "{clause}");
        assert!(
            load_resource_contract_sources(&sources).is_err(),
            "{clause}"
        );
    }
}

#[test]
fn the_production_module_declares_the_three_roles() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/lib_engine_thread.qv"
    ))
    .expect("the production module");
    let declared = lifecycle_obligations(&BTreeMap::from([(
        "fluent_bit/lib_engine_thread.qv".to_owned(),
        text,
    )]))
    .expect("the production module loads");
    assert_eq!(declared.len(), 3);
    assert_eq!(declared["flb_start"].operation, LifecycleOperation::Start);
    assert_eq!(declared["flb_stop"].operation, LifecycleOperation::Stop);
    assert_eq!(
        declared["flb_destroy"].operation,
        LifecycleOperation::Destroy
    );
}

#[test]
fn every_resource_route_refuses_a_declared_lifecycle_call() {
    let text = module("stop");
    let contracts = load_resource_contract_sources(&BTreeMap::from([("m.qv".to_owned(), text)]))
        .expect("resource-v1 loads the module");
    assert!(contracts.contract("stop").is_some());
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let source = "int stop(void *e); void f(void *e) { stop(e); }";
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
        rendered.contains("call to `stop` outside its declared lifecycle obligation"),
        "{rendered}"
    );
    assert!(!matches!(
        result,
        qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Safe(_)
    ));
}
