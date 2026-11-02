//! Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS (ledger
//! `RECORD-BODY-TOTAL-OR-REJECT`): the lexical declaration reader reads a
//! record whose only unparsed member is a function pointer, and a function
//! whose only unparsed formal is a function pointer. The shapes are Fluent
//! Bit's `flb_input_collector` (`cb_collect`), `flb_sched_timer` (`cb`) and
//! `mk_event` (`handler`) on route G, over the synthetic `ev_*` callees of
//! `fixtures/contracts/event_loop.qv` (trusted test assumptions: a `Safe` here
//! is `Safe` over them and promotes nothing).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// `fcoll` is the collector's shape: a function-pointer member beside the
/// loop, the event and plain scalars. `facoll` has an array of function
/// pointers instead, which the lexical reader still refuses whole (rule 10k's
/// Clang-typed actual reads it).
const PRELUDE: &str = "\
struct ev_loop; struct obj { int x; };
struct fcoll { int id; struct ev_loop *evl; struct obj event; int (*cb_collect)(void *, int); long seconds; struct fcoll *next; };
struct facoll { int id; struct ev_loop *evl; struct obj event; int (*cbs[2])(void); };
void *ev_alloc(unsigned long n); void ev_free(void *p);
int ev_add(struct ev_loop *loop, void *data); int ev_del(struct ev_loop *loop, void *data);
void ev_touch(struct fcoll *coll);
";

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "event_loop.qcore".to_owned(),
        include_str!("fixtures/contracts/event_loop.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent FB-LOOP-CHAIN-ACCESS test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

fn row(source: &str) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
    let entry = source
        .split('(')
        .next()
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let function = &summary.functions[&entry];
    Row {
        status: function.status,
        unsupported: function
            .diagnostics
            .iter()
            .map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

/// `collector_start`'s and `flb_input_collector_pause`'s shape: a
/// registration and its removal at the loop path of a record that carries a
/// function-pointer member decide exactly as on a record without one.
#[test]
fn a_record_with_a_function_pointer_member_reads_its_other_members() {
    if !clang_available() {
        return;
    }
    let cycle = row(
        "void cycle(struct fcoll *coll) { ev_add(coll->evl, &coll->event); ev_del(coll->evl, &coll->event); }",
    );
    assert!(cycle.safe(), "{cycle:?}");
    // The registration alone now reaches rule 12t instead of the access gate.
    let start = row("void start(struct fcoll *coll) { ev_add(coll->evl, &coll->event); }");
    assert!(!start.safe(), "{start:?}");
    assert!(start.refused_with("retention-undecided"), "{start:?}");
    assert!(!start.refused_with(INDIRECT), "{start:?}");
    // A removal that matches no record is the identity (rule 12t).
    let stop = row("void stop(struct fcoll *coll) { ev_del(coll->evl, &coll->event); }");
    assert!(stop.safe(), "{stop:?}");
}

/// The function-pointer member itself joins no class of the lexical reader,
/// so nothing about it changes: a store of a function-pointer formal into it
/// is decided by the Clang-typed statement reading (rule 10k) exactly as at
/// the base, where the same row is `Safe` with no diagnostic (measured on the
/// base reader with this fixture).
#[test]
fn the_function_pointer_member_is_decided_as_before() {
    if !clang_available() {
        return;
    }
    let store =
        row("void set_cb(struct fcoll *coll, int (*cb)(void *, int)) { coll->cb_collect = cb; }");
    assert!(store.safe(), "{store:?}");
    assert!(store.unsupported.is_empty(), "{store:?}");
}

/// `cio_options_init`'s shape (corpus): scalar stores through a formal the
/// body also hands whole to a callee. Rule 5g's flow-refusing store keeps its
/// domain of records read with every member, so on a record read past a
/// function-pointer member these stores keep the route order they had at the
/// base (the lexical refusal, then the Clang-typed statement reading, which
/// decides them): the reader repair never hands a decided statement to a
/// refusing route. Without that domain the card's first AFTER corpus run
/// measured the corpus row `Safe -> Unknown` (four `unresolved resource
/// identity` leaves); this fixture pins the shape on route G.
#[test]
fn stores_through_a_handed_formal_keep_their_base_route() {
    if !clang_available() {
        return;
    }
    let init =
        row("void init(struct fcoll *coll) { ev_touch(coll); coll->id = 1; coll->seconds = 2; }");
    assert!(init.safe(), "{init:?}");
}

/// Any other parenthesised member still refuses the whole record in the
/// lexical reader (an array of function pointers), so the lexical recogniser
/// admits no member read of `facoll`. Rule 10k's Clang-typed actual does
/// admit `coll->evl`. Until card RETAINING-CALL-MINTED-NAME-ACTUALS (rule
/// 38b) that admission left only the outer classification of rule 12t's
/// withheld `ev_add` / `ev_del`, and the inner one kept the access gate's
/// refusal. It now leaves both, so the cycle decides as `fcoll`'s does, and
/// the registration alone reaches rule 12t.
#[test]
fn an_array_of_function_pointers_is_read_through_the_typed_actual() {
    if !clang_available() {
        return;
    }
    let cycle = row(
        "void cycle_array(struct facoll *coll) { ev_add(coll->evl, &coll->event); ev_del(coll->evl, &coll->event); }",
    );
    assert!(cycle.safe(), "{cycle:?}");
    let start = row("void start_array(struct facoll *coll) { ev_add(coll->evl, &coll->event); }");
    assert!(!start.safe(), "{start:?}");
    assert!(start.refused_with("retention-undecided"), "{start:?}");
    assert!(!start.refused_with(INDIRECT), "{start:?}");
}

/// `flb_sched_timer_cb_create`'s and `collector_create`'s shape: a function
/// whose only unparsed formal is a function pointer reads its other
/// declarations, so a scalar store through a never-bound formal decides. (At
/// the base the Clang-typed statement reading already decided this one row;
/// the reader's own change is pinned by the unit test
/// `a_function_pointer_formal_is_opaque_and_the_function_is_read`. This pins
/// that the now-readable declarations do not make it refuse.)
#[test]
fn a_function_pointer_formal_leaves_the_function_readable() {
    if !clang_available() {
        return;
    }
    let set = row(
        "int set_seconds(struct fcoll *coll, int (*cb)(void *, int), long seconds) { coll->seconds = seconds; return 0; }",
    );
    assert!(set.safe(), "{set:?}");
}
