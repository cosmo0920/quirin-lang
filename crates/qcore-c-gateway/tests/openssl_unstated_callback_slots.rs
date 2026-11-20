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

//! Card OPENSSL-UNSTATED-CALLBACK-SLOTS (gateway rule 12n; contract
//! specification C04.19.3, C04.18): the verify, info, message, security,
//! session and client certificate slots `quirin-provider-openssl` now states,
//! read through the crate's own modules (`ssl.qv`, `ssl_ctx.qv`,
//! `ssl_io.qv`, loaded as shipped in the user crate).
//!
//! Each fail-closed arm the brief names has a test: a newly stated slot whose
//! run candidate has no contract in force refuses; a slot registered from a
//! value the closure cannot read refuses; a slot no loaded declaration
//! registers refuses; and the declaration that lost the slot premise
//! (`SSL_CTX_use_certificate_chain_file`) is `Safe` only where every stated
//! invocation discharges, never by the premise's absence alone.

use std::collections::BTreeMap;
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
            eprintln!("skipping Clang-dependent OpenSSL slot test: {reason}");
            false
        }
    }
}

const SSL: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl.qv");
const SSL_CTX: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl_ctx.qv");
const SSL_IO: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl_io.qv");
/// Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: the handshake slots' setters,
/// which the I/O calls' invocations now also need registered.
const SSL_HANDSHAKE: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl_handshake.qv");
const BIO_SLOTS: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/bio_slots.qv");

/// The application's candidates: identity contracts, and one that releases
/// what the security callback is handed as `other`.
const CANDIDATES: &str = "module contract;

extern fn cert_ok(s: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn verify_ok(preverify_ok: i32, x509_ctx: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(preverify_ok);
    ensures normal resource_borrowed(x509_ctx);
    ensures normal resource_return_none();
};

extern fn info_ok(ssl: ptr<const, u8>, where_: i32, ret: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_borrowed(where_);
    ensures normal resource_borrowed(ret);
    ensures normal resource_return_none();
};

extern fn sec_ok(s: ptr<const, u8>, ctx: ptr<const, u8>, op: i32, bits: i32, nid: i32, other: ptr<mut, u8>, ex: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(op);
    ensures normal resource_borrowed(bits);
    ensures normal resource_borrowed(nid);
    ensures normal resource_borrowed(other);
    ensures normal resource_borrowed(ex);
    ensures normal resource_return_none();
};

extern fn sec_freeing(s: ptr<const, u8>, ctx: ptr<const, u8>, op: i32, bits: i32, nid: i32, other: ptr<mut, u8>, ex: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(op);
    ensures normal resource_borrowed(bits);
    ensures normal resource_borrowed(nid);
    ensures normal resource_released(other, nonnull);
    ensures normal resource_borrowed(ex);
    ensures normal resource_return_none();
};

extern fn sec_needs_ctx(s: ptr<const, u8>, ctx: ptr<const, u8>, op: i32, bits: i32, nid: i32, other: ptr<mut, u8>, ex: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_owned(ctx, nonnull);
    ensures normal resource_borrowed(op);
    ensures normal resource_borrowed(bits);
    ensures normal resource_borrowed(nid);
    ensures normal resource_borrowed(other);
    ensures normal resource_borrowed(ex);
    ensures normal resource_return_none();
};

extern fn sec_needs_s(s: ptr<const, u8>, ctx: ptr<const, u8>, op: i32, bits: i32, nid: i32, other: ptr<mut, u8>, ex: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_owned(s, nonnull);
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(op);
    ensures normal resource_borrowed(bits);
    ensures normal resource_borrowed(nid);
    ensures normal resource_borrowed(other);
    ensures normal resource_borrowed(ex);
    ensures normal resource_return_none();
};

extern fn sec_needs_other(s: ptr<const, u8>, ctx: ptr<const, u8>, op: i32, bits: i32, nid: i32, other: ptr<mut, u8>, ex: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(op);
    ensures normal resource_borrowed(bits);
    ensures normal resource_borrowed(nid);
    ensures normal resource_owned(other, nonnull);
    ensures normal resource_borrowed(ex);
    ensures normal resource_return_none();
};

extern fn sec_null_ok(s: ptr<const, u8>, ctx: ptr<const, u8>, op: i32, bits: i32, nid: i32, other: ptr<mut, u8>, ex: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_owned(s, nullable);
    ensures normal resource_owned(ctx, nullable);
    ensures normal resource_borrowed(op);
    ensures normal resource_borrowed(bits);
    ensures normal resource_borrowed(nid);
    ensures normal resource_owned(other, nullable);
    ensures normal resource_owned(ex, nullable);
    ensures normal resource_return_none();
};
";

/// The public headers' view: the objects are opaque; the prototypes as the
/// pinned headers give them.
const HEADERS: &str = "\
typedef struct ssl_ctx_st SSL_CTX;
typedef struct ssl_st SSL;
typedef struct x509_st X509;
typedef struct x509_store_ctx_st X509_STORE_CTX;
typedef int (*SSL_verify_cb)(int preverify_ok, X509_STORE_CTX *x509_ctx);
int SSL_write(SSL *ssl, const void *buf, int num);
int SSL_CTX_use_certificate(SSL_CTX *ctx, X509 *x);
int SSL_CTX_use_certificate_chain_file(SSL_CTX *ctx, const char *file);
void SSL_CTX_set_verify(SSL_CTX *ctx, int mode, SSL_verify_cb callback);
void SSL_set_verify(SSL *s, int mode, SSL_verify_cb callback);
void SSL_CTX_set_info_callback(SSL_CTX *ctx, void (*cb)(const SSL *ssl, int type, int val));
void SSL_CTX_set_msg_callback(SSL_CTX *ctx, void (*cb)(int write_p, int version, int content_type, const void *buf, __SIZE_TYPE__ len, SSL *ssl, void *arg));
void SSL_CTX_set_security_callback(SSL_CTX *ctx, int (*cb)(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex));
void SSL_CTX_set_cert_cb(SSL_CTX *c, int (*cb)(SSL *ssl, void *arg), void *arg);
int verify_ok(int preverify_ok, X509_STORE_CTX *x509_ctx);
int verify_uncontracted(int preverify_ok, X509_STORE_CTX *x509_ctx);
void info_ok(const SSL *ssl, int where_, int ret);
void info_uncontracted(const SSL *ssl, int where_, int ret);
int sec_ok(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex);
int sec_freeing(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex);
int sec_needs_ctx(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex);
int sec_needs_s(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex);
int sec_needs_other(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex);
int sec_null_ok(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid, void *other, void *ex);
int cert_ok(SSL *s, void *arg);
";

const CALLER: &str = "\
int write_some(SSL *s, const char *b) { return SSL_write(s, b, 4); }
int load_chain(SSL_CTX *c) { return SSL_CTX_use_certificate_chain_file(c, \"chain.pem\"); }
int load_certificate(SSL_CTX *c, X509 *x) { return SSL_CTX_use_certificate(c, x); }
void configure(SSL_CTX *c) { SSL_CTX_set_cert_cb(c, cert_ok, 0); }
";

/// The crate's three modules (each declaration's library range and
/// revision as shipped), and the candidates.
fn contracts(modules: &[(&str, &str)]) -> ResourceContracts {
    let mut sources = modules
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect::<BTreeMap<_, _>>();
    sources.insert("candidates.qv".to_owned(), CANDIDATES.to_owned());
    load_resource_contract_sources(&sources).expect("the crate's modules load")
}

fn crate_contracts() -> ResourceContracts {
    contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("ssl_io.qv", SSL_IO),
        ("ssl_handshake.qv", SSL_HANDSHAKE),
        ("bio_slots.qv", BIO_SLOTS),
    ])
}

/// The declarations' stores, as `callback_member_fold::declared_stores`
/// derives them from the loaded contracts.
fn declared(contracts: &ResourceContracts) -> MemberFacts {
    let mut stores = Vec::new();
    for (name, declaration) in contracts.retaining() {
        for clause in &declaration.callbacks {
            if let CallbackClause::MemberRegistered {
                member, callback, ..
            } = clause
            {
                stores.push(DeclaredStore::Parameter {
                    member: member.spelling(),
                    function: name.clone(),
                    index: *callback,
                });
            }
        }
    }
    stores.sort();
    declarations_unit(&stores)
}

fn facts(body: &str) -> MemberFacts {
    let text = format!("{HEADERS}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

fn run_of(registrar: &str, contracts: &ResourceContracts) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(
        vec![facts(registrar), facts(CALLER), declared(contracts)],
        BTreeMap::new(),
    ))
}

fn row_of(
    name: &str,
    run: Arc<RunMemberFacts>,
    contracts: &ResourceContracts,
) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{HEADERS}{CALLER}");
    let outcome = scoped(Some(run), || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            name,
            contracts,
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

fn refused(leaves: &[String], word: &str) -> bool {
    leaves.iter().any(|leaf| leaf.contains(word))
}

/// The crate's declarations load as slot clauses: every new setter a slot
/// registration, every handshake and I/O call nine slot invocations.
#[test]
fn the_crate_states_every_new_slot() {
    let contracts = crate_contracts();
    for (setter, key) in [
        ("SSL_CTX_set_verify", "ssl_ctx_st.verify_callback"),
        ("SSL_set_verify", "ssl_ctx_st.verify_callback"),
        ("SSL_CTX_set_info_callback", "ssl_ctx_st.info_callback"),
        ("SSL_set_info_callback", "ssl_ctx_st.info_callback"),
        ("SSL_CTX_set_msg_callback", "ssl_ctx_st.msg_callback"),
        ("SSL_set_msg_callback", "ssl_ctx_st.msg_callback"),
        ("SSL_CTX_set_security_callback", "ssl_ctx_st.sec_cb"),
        ("SSL_set_security_callback", "ssl_ctx_st.sec_cb"),
        ("SSL_CTX_sess_set_new_cb", "ssl_ctx_st.new_session_cb"),
        ("SSL_CTX_sess_set_remove_cb", "ssl_ctx_st.remove_session_cb"),
        ("SSL_CTX_sess_set_get_cb", "ssl_ctx_st.get_session_cb"),
        ("SSL_CTX_set_client_cert_cb", "ssl_ctx_st.client_cert_cb"),
    ] {
        let declaration = contracts
            .retaining_contract(setter)
            .unwrap_or_else(|| panic!("{setter} is withheld"));
        assert!(
            matches!(declaration.callbacks.as_slice(),
                [CallbackClause::MemberRegistered { slot: true, member, .. }]
                    if member.spelling() == key),
            "{setter}"
        );
        assert!(contracts.contract(setter).is_none(), "{setter}");
    }
    for call in ["SSL_write", "SSL_read", "SSL_connect", "SSL_do_handshake"] {
        let declaration = contracts.retaining_contract(call).expect("withheld");
        // Nine since this card; 27 since OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS;
        // 41 since OPENSSL-BIO-METHODS-AND-CALLBACKS.
        assert_eq!(declaration.callbacks.len(), 41, "{call}");
        assert!(
            declaration
                .callbacks
                .iter()
                .all(|clause| matches!(clause, CallbackClause::MemberInvoked { slot: true, .. })),
            "{call}"
        );
    }
    // SSL_CTX_use_certificate is no longer a plain declaration in the
    // function map: a consumer that knows nothing of callbacks sees it
    // uncontracted, never benign.
    assert!(contracts.contract("SSL_CTX_use_certificate").is_none());
    assert_eq!(
        contracts
            .retaining_contract("SSL_CTX_use_certificate")
            .expect("withheld")
            .callbacks
            .len(),
        1
    );
}

/// Every new slot closed over identity candidates (or empty): the I/O call
/// decides.
#[test]
fn identity_candidates_decide_the_io_call() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let registrar = "\
void reg(SSL_CTX *c, SSL *s) {
    SSL_CTX_set_verify(c, 1, verify_ok);
    SSL_set_verify(s, 1, 0);
    SSL_CTX_set_info_callback(c, info_ok);
    SSL_CTX_set_security_callback(c, sec_ok);
    SSL_CTX_set_cert_cb(c, cert_ok, 0);
}
int cert_ok(SSL *s, void *arg) { return 1; }
";
    let run = run_of(registrar, &contracts);
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert!(
        !refused(&leaves, "callback-"),
        "no callback refusal: {leaves:?}"
    );
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A newly stated slot whose run candidate has no contract in force refuses
/// the call, for the context's setter and the SSL's alike.
#[test]
fn a_new_slot_with_an_uncontracted_candidate_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    for (registrar, candidate) in [
        (
            "void reg(SSL_CTX *c) { SSL_CTX_set_info_callback(c, info_uncontracted); }\n",
            "`info_uncontracted` (from `ssl_ctx_st.info_callback`)",
        ),
        (
            "void reg(SSL *s, int on) { SSL_set_verify(s, 1, on ? verify_uncontracted : 0); }\n",
            "`verify_uncontracted` (from `ssl_ctx_st.verify_callback`)",
        ),
    ] {
        let run = run_of(registrar, &contracts);
        let (status, leaves) = row_of("write_some", run, &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{registrar}");
        assert!(
            leaves
                .iter()
                .any(|leaf| leaf.contains(candidate) && leaf.contains("callback-uncontracted")),
            "{leaves:?}"
        );
    }
}

/// A slot registered from a value the closure cannot read (a formal of
/// another function, a call's result) is open: the call refuses.
#[test]
fn a_new_slot_registered_from_an_undecidable_value_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    // A formal handed to the setter, of a function whose address is taken
    // (so its callers' actuals are not all its values).
    let registrar = "\
void reg(SSL_CTX *c, void (*f)(int, int, int, const void *, __SIZE_TYPE__, SSL *, void *)) { SSL_CTX_set_msg_callback(c, f); }
void (*keep)(SSL_CTX *, void (*)(int, int, int, const void *, __SIZE_TYPE__, SSL *, void *)) = reg;
";
    let run = run_of(registrar, &contracts);
    assert!(run.closure("ssl_ctx_st.msg_callback").is_err());
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "callback-member-open"), "{leaves:?}");
}

/// A slot no loaded declaration registers has undeclared writers: without
/// `ssl_ctx.qv` the session and client certificate slots (whose setters only
/// the context has) are never decided, even with nothing registered.
#[test]
fn a_new_slot_no_declaration_registers_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = contracts(&[("ssl.qv", SSL), ("ssl_io.qv", SSL_IO)]);
    let run = run_of("void nothing(void) { }\n", &contracts);
    assert!(run.decides("ssl_ctx_st.new_session_cb", true).is_err());
    assert!(run.decides("ssl_ctx_st.client_cert_cb", true).is_err());
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "no declaration registers"), "{leaves:?}");
}

/// The declaration that lost `openssl_unstated_callback_slots` decides only
/// over its stated invocations: an effectful security candidate refuses it,
/// and SSL_CTX_use_certificate, which read the callback silently before,
/// refuses the same candidate releasing the caller's certificate.
#[test]
fn dropping_the_premise_never_skips_an_invocation() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let effectful = "void reg(SSL_CTX *c) { SSL_CTX_set_security_callback(c, sec_freeing); }\n";
    let run = run_of(effectful, &contracts);
    for name in ["load_chain", "load_certificate"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(refused(&leaves, "callback-effectful"), "{name}: {leaves:?}");
    }
    let identity = "void reg(SSL_CTX *c) { SSL_CTX_set_security_callback(c, sec_ok); }\n";
    let run = run_of(identity, &contracts);
    for name in ["load_chain", "load_certificate"] {
        let (_, leaves) = row_of(name, run.clone(), &contracts);
        assert!(!refused(&leaves, "callback-"), "{name}: {leaves:?}");
    }
}

/// RETENTION-NULL-CONSTANT-ACTUAL on the crate's own setter: a null `arg`
/// is retained by nothing, so the registration decides.
#[test]
fn a_null_argument_to_a_retaining_setter_retains_nothing() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let run = run_of("int cert_ok(SSL *s, void *arg) { return 1; }\n", &contracts);
    let (_, leaves) = row_of("configure", run, &contracts);
    assert!(!refused(&leaves, "retention-undecided"), "{leaves:?}");
}

/// Card CALLBACK-SUPPLIED-NULL-ACTUAL (gateway rule 13u amended; C04.19 item
/// 2): the security callback's `s` and `ctx` ("One of the two is NULL"),
/// `other` ("NULL when no object is associated") and `ex` are
/// `supplied_nullable` where the callee supplies them. A candidate requiring
/// a live object there refuses the call (`callback-supplied-null`); where the
/// actual is the caller's parameter, the requirement is the call's as before;
/// a candidate accepting NULL decides.
#[test]
fn a_candidate_requiring_a_live_object_where_the_library_may_pass_null_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let null_refusal = |leaves: &[String], candidate: &str| {
        leaves.iter().any(|leaf| {
            leaf.contains(&format!("`{candidate}` (from `ssl_ctx_st.sec_cb`)"))
                && leaf.contains("callback-supplied-null")
        })
    };
    let registrar = |candidate: &str| {
        format!("void reg(SSL_CTX *c) {{ SSL_CTX_set_security_callback(c, {candidate}); }}\n")
    };
    // `ctx` is NULL at a connection's check (SSL_write); at a context's check
    // it is the caller's parameter, so the requirement is the call's.
    let run = run_of(&registrar("sec_needs_ctx"), &contracts);
    let (status, leaves) = row_of("write_some", run.clone(), &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(null_refusal(&leaves, "sec_needs_ctx"), "{leaves:?}");
    for name in ["load_chain", "load_certificate"] {
        let (_, leaves) = row_of(name, run.clone(), &contracts);
        assert!(
            !refused(&leaves, "callback-supplied-null"),
            "{name}: {leaves:?}"
        );
    }
    // `s` is NULL at a context's check; at a connection's it is the
    // connection, the caller's parameter.
    let run = run_of(&registrar("sec_needs_s"), &contracts);
    for name in ["load_chain", "load_certificate"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(null_refusal(&leaves, "sec_needs_s"), "{name}: {leaves:?}");
    }
    let (_, leaves) = row_of("write_some", run, &contracts);
    assert!(!refused(&leaves, "callback-supplied-null"), "{leaves:?}");
    // `other` is NULL with no object; SSL_CTX_use_certificate passes the
    // caller's certificate there.
    let run = run_of(&registrar("sec_needs_other"), &contracts);
    for name in ["write_some", "load_chain"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            null_refusal(&leaves, "sec_needs_other"),
            "{name}: {leaves:?}"
        );
    }
    let (_, leaves) = row_of("load_certificate", run, &contracts);
    assert!(!refused(&leaves, "callback-supplied-null"), "{leaves:?}");
    // Accepting NULL everywhere the page says it may be passed: decided.
    let run = run_of(&registrar("sec_null_ok"), &contracts);
    for name in ["write_some", "load_chain", "load_certificate"] {
        let (_, leaves) = row_of(name, run.clone(), &contracts);
        assert!(!refused(&leaves, "callback-"), "{name}: {leaves:?}");
    }
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}
