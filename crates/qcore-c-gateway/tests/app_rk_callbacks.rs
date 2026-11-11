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

//! Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (gateway rule 12n, amended): the
//! application's rk_conf callbacks, closed over the run's registrations.
//!
//! USER DECISION 2026-10-14 ("Build the agent's recommended set"): a setter's
//! `resource_callback_member_registered(conf, TAG.FIELD, cb)` is a store of
//! its actual at every call (Q2); a constructor's stated default
//! (`quant live #result: TAG.FIELD |-> f;`) is a store of `f` (D1); a member
//! is keyed by its innermost named record and a union's members share their
//! stores (Q3); with the run's facts installed a unit decides over the run's
//! set (R1), and a run-closed empty set invokes nothing (E1).
//!
//! The four cases the brief names: a closed member, an open member, an
//! unclassified store, and a default candidate.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, Tables, closure, read_unit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{DeclaredStore, RunMemberFacts, declarations_unit, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent rk_conf callback test: {reason}");
            false
        }
    }
}

/// A conf embedded by value in a handle, as `rd_kafka_conf_s` is in
/// `rd_kafka_s`.
const RECORDS: &str = "\
typedef void log_fn(void *);
struct conf_s { log_fn *log_cb; log_fn *err_cb; int level; };
struct handle_s { int kind; struct conf_s conf; };
union slot_u { log_fn *a; log_fn *b; };
struct holder_s { union slot_u slot; };
void quiet_log(void *); void default_log(void *); void freeing_log(void *);
void conf_set_log(struct conf_s *conf, log_fn *cb);
void serve(struct handle_s *rk);
void serve_err(struct handle_s *rk);
void serve_nope(struct handle_s *rk);
";

const CONTRACTS: &str = "module contract;

extern fn conf_set_log(conf: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(conf);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_member_registered(conf, conf_s.log_cb, cb);
    ensures normal resource_return_none();
};

extern fn conf_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    quant live #result: conf_s.log_cb |-> default_log;
    ensures normal resource_return_owned(nonnull);
};

extern fn serve(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_callback_member_invoked(rk, handle_s.conf.log_cb, rk);
    ensures normal resource_return_none();
};

extern fn serve_err(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_callback_member_invoked(rk, handle_s.conf.err_cb, rk);
    ensures normal resource_return_none();
};

extern fn serve_nope(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_callback_member_invoked(rk, handle_s.conf.nope_cb, rk);
    ensures normal resource_return_none();
};

extern fn quiet_log(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_return_none();
};

extern fn default_log(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_return_none();
};

extern fn freeing_log(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(rk, nonnull);
    ensures normal resource_return_none();
};
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "app_rk.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn facts(body: &str) -> MemberFacts {
    let text = format!("{RECORDS}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

/// The declarations' stores the audit derives from [`CONTRACTS`].
fn declared() -> MemberFacts {
    declarations_unit(&[
        DeclaredStore::Parameter {
            member: "conf_s.log_cb".into(),
            function: "conf_set_log".into(),
            index: 1,
        },
        DeclaredStore::Default {
            member: "conf_s.log_cb".into(),
            function: "default_log".into(),
        },
    ])
}

const CALLER: &str = "\
void use(struct handle_s *h) { serve(h); }
void use_err(struct handle_s *h) { serve_err(h); }
void use_nope(struct handle_s *h) { serve_nope(h); }
";

fn row_of(name: &str, run: Option<Arc<RunMemberFacts>>) -> (AnalysisStatus, Vec<String>) {
    let contracts = contracts();
    let text = format!("{RECORDS}{CALLER}");
    let outcome = scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            name,
            &contracts,
        )
    })
    .expect("analysed");
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let row = &summary.functions[name];
    let leaves = row
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect();
    (row.status, leaves)
}

fn run(units: Vec<MemberFacts>) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(units, BTreeMap::new()))
}

/// Q3: `h->conf.log_cb` (an embedded conf) and `c->log_cb` (a pointer to a
/// conf) are one key, `conf_s.log_cb`; a struct copy of a conf into the
/// handle keeps the set closed; a whole-object copy into the outer record
/// opens the embedded member.
#[test]
fn an_embedded_store_and_a_direct_store_share_the_innermost_key() {
    if !clang_available() {
        return;
    }
    let unit = facts(
        "void a(struct handle_s *h) { h->conf.log_cb = quiet_log; }\n\
         void b(struct conf_s *c) { c->log_cb = default_log; }\n\
         void copy(struct handle_s *h, struct conf_s *c) { h->conf = *c; }\n",
    );
    assert_eq!(
        closure(&[&unit], "conf_s.log_cb"),
        Ok(names(&["default_log", "quiet_log"]))
    );
    assert_eq!(
        closure(&[&unit], "handle_s.conf.log_cb"),
        Ok(names(&["default_log", "quiet_log"]))
    );
    let tables = Tables::of(&[&unit]);
    assert_eq!(
        tables.normalise("handle_s.conf.log_cb"),
        Ok(names(&["conf_s.log_cb"]))
    );
    assert!(tables.normalise("handle_s.conf.level").is_err());
    assert!(tables.normalise("handle_s.conf.nope_cb").is_err());
    // A `memcpy` into the handle writes its embedded conf opaquely.
    let copied = facts(
        "void *memcpy(void *, const void *, unsigned long);\n\
         void a(struct handle_s *h) { h->conf.log_cb = quiet_log; }\n\
         void m(struct handle_s *h, const void *p) { memcpy(h, p, sizeof *h); }\n",
    );
    assert!(closure(&[&copied], "conf_s.log_cb").is_err());
}

/// Q3 (`CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS`, second part): a record value
/// copied into a field, by an initializer list or an assignment, carries
/// only values the embedded record's own key already counts, so the copy
/// adds no candidate and opens nothing; the conf's own stores reach the
/// handle's embedded member.
#[test]
fn a_record_value_copied_into_a_field_carries_its_own_keys() {
    if !clang_available() {
        return;
    }
    let unit = facts(
        "void a(struct conf_s *c) { c->log_cb = quiet_log; }\n\
         void init(struct conf_s proto) { struct handle_s h = { 1, proto }; serve(&h); }\n\
         void assign(struct handle_s *h, struct conf_s proto) { h->conf = proto; }\n",
    );
    assert_eq!(
        closure(&[&unit], "handle_s.conf.log_cb"),
        Ok(names(&["quiet_log"]))
    );
    // A nested list in the handle's initializer is mapped onto the conf: the
    // handle's own list, unmappable for it, does not open the conf's member
    // (only an opaque write of the handle does, as the memcpy above).
    let nested = facts("struct handle_s top = { 1, { default_log, 0, 0 } };\n");
    assert_eq!(
        closure(&[&nested], "handle_s.conf.log_cb"),
        Ok(names(&["default_log"]))
    );
}

/// Q3: a union's members share their stores.
#[test]
fn a_unions_members_share_their_stores() {
    if !clang_available() {
        return;
    }
    let unit = facts("void s(struct holder_s *h) { h->slot.a = freeing_log; }\n");
    assert_eq!(
        closure(&[&unit], "holder_s.slot.b"),
        Ok(names(&["freeing_log"]))
    );
    // Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: the declared spelling keeps
    // its arm path (the layout, which puts `a` and `b` on the same bytes,
    // shares them); it was the whole union.
    assert_eq!(
        Tables::of(&[&unit]).normalise("holder_s.slot.b"),
        Ok(names(&["slot_u.*@b"]))
    );
}

/// A closed member: the setter's actual and the stated default are the run's
/// candidates; each is the identity, so the invocation decides.
#[test]
fn a_closed_member_decides_over_the_registrations_and_the_default() {
    if !clang_available() {
        return;
    }
    let registrar = facts("void reg(struct conf_s *c) { conf_set_log(c, quiet_log); }\n");
    let analysed = facts(CALLER);
    let run = run(vec![registrar, analysed, declared()]);
    assert_eq!(
        run.closure("handle_s.conf.log_cb"),
        Ok(names(&["default_log", "quiet_log"]))
    );
    let (status, leaves) = row_of("use", Some(run));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    // Without the run's facts the unit sees no store: refused.
    let (status, leaves) = row_of("use", None);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
}

/// A registered candidate that releases what it receives is refused.
#[test]
fn an_effectful_registration_is_refused() {
    if !clang_available() {
        return;
    }
    let registrar = facts(
        "void reg(struct conf_s *c) { conf_set_log(c, quiet_log); }\n\
         void reg2(struct conf_s *c) { conf_set_log(c, freeing_log); }\n",
    );
    let run = run(vec![registrar, facts(CALLER), declared()]);
    let (status, leaves) = row_of("use", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-effectful")),
        "{leaves:?}"
    );
}

/// An open member: an unclassified store (a variable), an address-taken
/// setter, or an unread unit opens the set, and the invocation is refused.
#[test]
fn an_unclassified_store_or_an_unread_unit_opens_the_member() {
    if !clang_available() {
        return;
    }
    let unclassified =
        facts("log_fn *chosen;\nvoid set(struct handle_s *h) { h->conf.log_cb = chosen; }\n");
    let open = run(vec![unclassified, facts(CALLER), declared()]);
    assert!(open.closure("conf_s.log_cb").is_err());
    let (status, leaves) = row_of("use", Some(open));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
    let taken =
        facts("log_fn *(*p)(void) ; void (*q)(struct conf_s *, log_fn *) = conf_set_log;\n");
    assert!(
        run(vec![taken, facts(CALLER), declared()])
            .closure("conf_s.log_cb")
            .is_err()
    );
    let unread = Arc::new(RunMemberFacts::new(
        vec![facts(CALLER), declared()],
        BTreeMap::from([("plugin.cpp".to_owned(), "a unit that is not C".to_owned())]),
    ));
    let (status, _) = row_of("use", Some(unread));
    assert_ne!(status, AnalysisStatus::Safe);
}

/// A default candidate: with no registration the stated default alone is the
/// set, and an effectful default would be refused like any candidate.
#[test]
fn a_default_candidate_joins_the_run_set() {
    if !clang_available() {
        return;
    }
    let run = run(vec![facts(CALLER), declared()]);
    assert_eq!(run.closure("conf_s.log_cb"), Ok(names(&["default_log"])));
    let (status, leaves) = row_of("use", Some(run));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// E1: a member the run closes empty invokes nothing (only over the run's
/// set); a member that is not a function pointer of a record the run defines
/// is never decided so.
#[test]
fn a_run_closed_empty_member_invokes_nothing_and_a_misspelling_refuses() {
    if !clang_available() {
        return;
    }
    let run = run(vec![facts(CALLER), declared()]);
    assert_eq!(run.closure("handle_s.conf.err_cb"), Ok(names(&[])));
    let (status, leaves) = row_of("use_err", Some(run.clone()));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    let (status, _) = row_of("use_err", None);
    assert_ne!(status, AnalysisStatus::Safe);
    let (status, leaves) = row_of("use_nope", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("not a member the run's records define")),
        "{leaves:?}"
    );
}
