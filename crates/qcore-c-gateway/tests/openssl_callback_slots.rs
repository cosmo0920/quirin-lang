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

//! Card OPENSSL-SSL-CTX-CALLBACK-SLOTS (gateway rule 12n, amended in place;
//! contract specification C04.19.3): a callback slot of a record the library
//! keeps opaque, `resource_callback_slot_registered(P, TAG.FIELD, F)` and
//! `resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)`.
//!
//! An opaque slot is decided only over the run's facts, only while no unit of
//! the run defines `struct TAG`, only when a declaration registers it, over
//! exactly what the declared setters are handed, and every candidate must
//! compose as the identity (C04.19's loop-safe composition). Each fail-closed
//! arm the brief names has a test: a candidate with no contract in force, an
//! effectful candidate among several registrations, an open set, a slot
//! nothing registers, a slot whose record a unit defines, a slot read without
//! the run's facts; and the member form, unchanged, still never decides a
//! record the run does not define.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, read_unit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{DeclaredStore, RunMemberFacts, declarations_unit, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::callback::CallbackClause;
use qcore_schemas::contract::ResourceContracts;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent callback slot test: {reason}");
            false
        }
    }
}

/// The public headers' view: both objects are opaque.
const RECORDS: &str = "\
typedef struct ssl_ctx_st SSL_CTX;
typedef struct ssl_st SSL;
typedef int pem_password_cb(char *buf, int size, int rwflag, void *u);
void SSL_CTX_set_cert_cb(SSL_CTX *c, int (*cb)(SSL *ssl, void *arg), void *arg);
void SSL_CTX_set_default_passwd_cb(SSL_CTX *ctx, pem_password_cb *cb);
int SSL_write(SSL *ssl, const void *buf, int num);
int SSL_CTX_use_PrivateKey_file(SSL_CTX *ctx, const char *file, int type);
int sec_check(SSL *ssl);
int member_write(SSL *ssl);
int cert_ok(SSL *s, void *arg);
int cert_also_ok(SSL *s, void *arg);
int cert_freeing(SSL *s, void *arg);
int pw_ok(char *buf, int size, int rwflag, void *u);
int pw_uncontracted(char *buf, int size, int rwflag, void *u);
";

const CONTRACTS: &str = "module contract;

extern fn SSL_CTX_set_cert_cb(c: ptr<mut, u8>, cert_cb: fnptr, arg: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(c);
    ensures normal resource_borrowed(cert_cb);
    ensures normal resource_borrowed(arg);
    ensures normal resource_callback_slot_registered(c, ssl_ctx_st.cert_cb, cert_cb);
    ensures normal resource_retained_by(arg, c);
    ensures normal resource_return_none();
};

extern fn SSL_CTX_set_default_passwd_cb(ctx: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_slot_registered(ctx, ssl_ctx_st.default_passwd_cb, cb);
    ensures normal resource_return_none();
};

extern fn SSL_write(ssl: ptr<mut, u8>, buf: ptr<const, u8>, num: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_borrowed(buf);
    ensures normal resource_borrowed(num);
    ensures normal resource_callback_slot_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _);
    ensures normal resource_return_none();
};

extern fn SSL_CTX_use_PrivateKey_file(ctx: ptr<mut, u8>, file: ptr<const, u8>, filetype: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(file);
    ensures normal resource_borrowed(filetype);
    ensures normal resource_callback_slot_invoked(ctx, ssl_ctx_st.default_passwd_cb, _, _, _, _);
    ensures normal resource_return_none();
};

extern fn sec_check(ssl: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_callback_slot_invoked(ssl, ssl_ctx_st.sec_cb, ssl);
    ensures normal resource_return_none();
};

extern fn member_write(ssl: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_callback_member_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _);
    ensures normal resource_return_none();
};

extern fn cert_ok(s: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn cert_also_ok(s: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn cert_freeing(s: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(s, nonnull);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn pw_ok(buf: ptr<mut, u8>, size: i32, rwflag: i32, u: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(buf);
    ensures normal resource_borrowed(size);
    ensures normal resource_borrowed(rwflag);
    ensures normal resource_borrowed(u);
    ensures normal resource_return_none();
};
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "slots.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn facts(prefix: &str, body: &str) -> MemberFacts {
    let text = format!("{prefix}{RECORDS}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

/// The declarations' stores the audit derives from [`CONTRACTS`]
/// (`callback_member_fold::declared_stores` reads a slot registration as
/// a member registration's store).
fn declared() -> MemberFacts {
    declarations_unit(&[
        DeclaredStore::Parameter {
            member: "ssl_ctx_st.cert_cb".into(),
            function: "SSL_CTX_set_cert_cb".into(),
            index: 1,
        },
        DeclaredStore::Parameter {
            member: "ssl_ctx_st.default_passwd_cb".into(),
            function: "SSL_CTX_set_default_passwd_cb".into(),
            index: 1,
        },
    ])
}

const CALLER: &str = "\
int write_some(SSL *s, const char *b) { return SSL_write(s, b, 4); }
int load_key(SSL_CTX *c) { return SSL_CTX_use_PrivateKey_file(c, \"k.pem\", 1); }
int checked(SSL *s) { return sec_check(s); }
int by_member(SSL *s) { return member_write(s); }
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

fn run_of(units: Vec<MemberFacts>) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(units, BTreeMap::new()))
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

fn refused(leaves: &[String], word: &str) -> bool {
    leaves.iter().any(|leaf| leaf.contains(word))
}

/// The loader reads the slot forms as member clauses marked `slot`, and
/// refuses a malformed one (a path below the field, a duplicate, an object
/// the callee supplies) by rejecting the module.
#[test]
fn the_slot_forms_load_and_reject_their_malformed_spellings() {
    let contracts = contracts();
    let setter = contracts
        .retaining_contract("SSL_CTX_set_cert_cb")
        .expect("a retaining declaration");
    assert!(matches!(
        setter.callbacks.as_slice(),
        [CallbackClause::MemberRegistered {
            slot: true,
            callback: 1,
            object: 0,
            ..
        }]
    ));
    assert_eq!(
        setter.callbacks[0].keyword(),
        "resource_callback_slot_registered"
    );
    let write = contracts.retaining_contract("SSL_write").expect("withheld");
    assert!(matches!(
        write.callbacks.as_slice(),
        [CallbackClause::MemberInvoked { slot: true, .. }]
    ));
    assert_eq!(
        write.callbacks[0].keyword(),
        "resource_callback_slot_invoked"
    );
    // A slot is withheld from the function map: a consumer that knows
    // nothing of callbacks sees an uncontracted callee.
    assert!(contracts.contract("SSL_write").is_none());
    let module = |clauses: &str| {
        format!(
            "module contract;\nextern fn f(ssl: ptr<mut, u8>, cb: fnptr) -> unit\ncontract {{\n    \
             requires resource_contract_v1;\n    ensures normal resource_borrowed(ssl);\n    \
             ensures normal resource_borrowed(cb);\n{clauses}    ensures normal \
             resource_return_none();\n}};\n"
        )
    };
    let load = |clauses: &str| {
        load_resource_contract_sources(&BTreeMap::from([("m.qv".to_owned(), module(clauses))]))
    };
    assert!(load("    ensures normal resource_callback_slot_registered(ssl, t.cb, cb);\n").is_ok());
    for bad in [
        // A path below the slot's field.
        "    ensures normal resource_callback_slot_registered(ssl, t.inner.cb, cb);\n",
        // A slot registered twice on one holder.
        "    ensures normal resource_callback_slot_registered(ssl, t.cb, cb);\n    ensures normal \
         resource_callback_slot_invoked(ssl, t.cb, _);\n    ensures normal \
         resource_callback_slot_invoked(ssl, t.cb, _);\n",
        // An object the callee supplies.
        "    ensures normal resource_callback_slot_invoked(supplied_owned, t.cb, supplied_owned);\n",
        // A registration with the wrong arity.
        "    ensures normal resource_callback_slot_registered(ssl, t.cb);\n",
        // The callback as its own holder.
        "    ensures normal resource_callback_slot_registered(cb, t.cb, cb);\n",
    ] {
        assert!(load(bad).is_err(), "{bad}");
    }
}

/// A closed slot: the declared setter's actual is the run's candidate, the
/// identity, so the invocation decides (and the unit-only route refuses).
#[test]
fn a_closed_slot_decides_over_the_declared_setters_actuals() {
    if !clang_available() {
        return;
    }
    let registrar = facts(
        "",
        "void reg(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_ok, 0); SSL_CTX_set_cert_cb(c, cert_also_ok, 0); }\n",
    );
    let run = run_of(vec![registrar, facts("", CALLER), declared()]);
    assert_eq!(
        run.closure("ssl_ctx_st.cert_cb"),
        Ok(names(&["cert_also_ok", "cert_ok"]))
    );
    assert_eq!(run.decides("ssl_ctx_st.cert_cb", true), Ok(()));
    let (status, leaves) = row_of("write_some", Some(run));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    // Without the run's facts a slot is never decided.
    let (status, leaves) = row_of("write_some", None);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "callback-member-open"), "{leaves:?}");
    assert!(refused(&leaves, "keeps opaque"), "{leaves:?}");
}

/// A slot whose declared setter the run never calls holds nothing the
/// application registered: a closed empty set invokes nothing.
#[test]
fn an_unregistered_run_set_invokes_nothing() {
    if !clang_available() {
        return;
    }
    let run = run_of(vec![facts("", CALLER), declared()]);
    assert_eq!(run.closure("ssl_ctx_st.cert_cb"), Ok(BTreeSet::new()));
    let (status, leaves) = row_of("write_some", Some(run));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A candidate with no contract in force refuses the call: a callback is
/// never read benign.
#[test]
fn a_candidate_with_no_contract_refuses() {
    if !clang_available() {
        return;
    }
    let registrar = facts(
        "",
        "void reg(SSL_CTX *c) { SSL_CTX_set_default_passwd_cb(c, pw_uncontracted); }\n",
    );
    let run = run_of(vec![registrar, facts("", CALLER), declared()]);
    let (status, leaves) = row_of("load_key", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "callback-uncontracted"), "{leaves:?}");
    // With a contracted identity candidate the same call decides.
    let registrar = facts(
        "",
        "void reg(SSL_CTX *c) { SSL_CTX_set_default_passwd_cb(c, pw_ok); }\n",
    );
    let run = run_of(vec![registrar, facts("", CALLER), declared()]);
    let (status, leaves) = row_of("load_key", Some(run));
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// Several registrations of one slot: every candidate composes, and one
/// that releases what it receives refuses the call.
#[test]
fn an_effectful_candidate_among_several_registrations_refuses() {
    if !clang_available() {
        return;
    }
    let registrar = facts(
        "",
        "void reg(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_ok, 0); }\n\
         void again(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_freeing, 0); }\n",
    );
    let run = run_of(vec![registrar, facts("", CALLER), declared()]);
    let (status, leaves) = row_of("write_some", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "callback-effectful"), "{leaves:?}");
}

/// Unknown slot state refuses: a setter handed a value the closure cannot
/// read, an address-taken setter, an unread unit.
#[test]
fn an_open_slot_refuses() {
    if !clang_available() {
        return;
    }
    let handed = facts(
        "",
        "void reg(SSL_CTX *c, int (*f)(SSL *, void *)) { SSL_CTX_set_cert_cb(c, f, 0); }\n\
         int (*pick(void))(SSL *, void *);\n\
         void reg2(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, pick(), 0); }\n",
    );
    let run = run_of(vec![handed, facts("", CALLER), declared()]);
    assert!(run.closure("ssl_ctx_st.cert_cb").is_err());
    let (status, leaves) = row_of("write_some", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "callback-member-open"), "{leaves:?}");
    let taken = facts(
        "",
        "void reg(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_ok, 0); }\n\
         void (*keep)(SSL_CTX *, int (*)(SSL *, void *), void *) = SSL_CTX_set_cert_cb;\n",
    );
    let run = run_of(vec![taken, facts("", CALLER), declared()]);
    let (status, leaves) = row_of("write_some", Some(run));
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    let unread = Arc::new(RunMemberFacts::new(
        vec![facts("", CALLER), declared()],
        BTreeMap::from([(
            "x.cpp".to_owned(),
            "a unit that is not C (`cpp`)".to_owned(),
        )]),
    ));
    let (status, leaves) = row_of("write_some", Some(unread));
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A slot no declaration registers has undeclared writers: never decided,
/// even with an empty closure.
#[test]
fn a_slot_nothing_registers_refuses() {
    if !clang_available() {
        return;
    }
    let run = run_of(vec![facts("", CALLER), declared()]);
    assert_eq!(run.closure("ssl_ctx_st.sec_cb"), Ok(BTreeSet::new()));
    assert!(run.decides("ssl_ctx_st.sec_cb", true).is_err());
    let (status, leaves) = row_of("checked", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "no declaration registers"), "{leaves:?}");
}

/// A unit that defines the record can store into it directly: the slot's
/// writers are no longer the declarations alone, so it is never decided.
#[test]
fn a_slot_whose_record_a_unit_defines_refuses() {
    if !clang_available() {
        return;
    }
    let defining = facts(
        "struct ssl_ctx_st { int (*cert_cb)(struct ssl_st *, void *); };\n",
        "void reg(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_ok, 0); }\n",
    );
    let run = run_of(vec![defining, facts("", CALLER), declared()]);
    assert!(run.decides("ssl_ctx_st.cert_cb", true).is_err());
    let (status, leaves) = row_of("write_some", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        refused(&leaves, "defines `struct ssl_ctx_st`"),
        "{leaves:?}"
    );
}

/// The member form is unchanged: a member of a record no unit defines is
/// never decided, even where a declaration registers the same key (only the
/// marked slot form reads an opaque record).
#[test]
fn the_member_form_still_never_decides_an_undefined_record() {
    if !clang_available() {
        return;
    }
    let registrar = facts(
        "",
        "void reg(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_ok, 0); }\n",
    );
    let run = run_of(vec![registrar, facts("", CALLER), declared()]);
    assert!(run.decides("ssl_ctx_st.cert_cb", false).is_err());
    let (status, leaves) = row_of("by_member", Some(run));
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        refused(&leaves, "not a member the run's records define"),
        "{leaves:?}"
    );
}
