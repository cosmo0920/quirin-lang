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

//! Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS (gateway rule 12n; contract
//! specification C04.19.3, C04.18): the keylog, ALPN/NPN, certificate
//! verification, PSK, ClientHello, record padding, early data and custom
//! extension slots `quirin-provider-openssl` now states, read through the
//! crate's own modules (`ssl.qv`, `ssl_ctx.qv`, `ssl_io.qv`,
//! `ssl_handshake.qv`, loaded as shipped in the user crate).
//!
//! Each fail-closed arm has a test: a new slot whose run candidate has no
//! contract in force refuses (Fluent Bit's keylog and ALPN setters' shape); a
//! candidate of a multi-slot setter that releases what the library hands it
//! refuses; a new slot no loaded declaration registers refuses; and
//! `SSL_shutdown`'s record padding invocation refuses an uncontracted
//! candidate. A row is `Safe` only where every candidate of every stated slot
//! is the identity.
//!
//! Card OPENSSL-BIO-METHODS-AND-CALLBACKS (same rule; this target, so the
//! read census needs no new unit): the BIO callback and method table slots
//! (`bio_slots.qv`), with the same arms: an uncontracted method or callback
//! candidate refuses the I/O call and `SSL_shutdown`, an effectful one
//! refuses, a BIO slot no loaded declaration registers refuses, and identity
//! candidates decide. `BIO_free` states no invocation (its callbacks rest on
//! the premise `openssl_bio_free_callbacks`).

use std::collections::BTreeMap;
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, read_unit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{DeclaredStore, RunMemberFacts, declarations_unit, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::callback::{CallbackActual, CallbackClause};
use qcore_schemas::contract::ResourceContracts;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent OpenSSL handshake slot test: {reason}");
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
const SSL_HANDSHAKE: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/ssl_handshake.qv");
const BIO_SLOTS: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/bio_slots.qv");

/// The application's candidates: identity contracts, and an older-style
/// custom extension free callback that releases the `add_arg` the library
/// hands it.
const CANDIDATES: &str = "module contract;

extern fn keylog_ok(ssl: ptr<const, u8>, line: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_borrowed(line);
    ensures normal resource_return_none();
};

extern fn alpn_ok(ssl: ptr<mut, u8>, out: ptr<mut, ptr<const, u8>>, outlen: ptr<mut, u8>, in_: ptr<const, u8>, inlen: u32, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(outlen);
    ensures normal resource_borrowed(in_);
    ensures normal resource_borrowed(inlen);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn padding_ok(ssl: ptr<mut, u8>, type_: i32, len: usize, arg: ptr<mut, u8>) -> usize
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ssl);
    ensures normal resource_borrowed(type_);
    ensures normal resource_borrowed(len);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn ext_add_ok(s: ptr<mut, u8>, ext_type: u32, out: ptr<mut, ptr<const, u8>>, outlen: ptr<mut, u8>, al: ptr<mut, u8>, add_arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ext_type);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(outlen);
    ensures normal resource_borrowed(al);
    ensures normal resource_borrowed(add_arg);
    ensures normal resource_return_none();
};

extern fn ext_free_ok(s: ptr<mut, u8>, ext_type: u32, out: ptr<const, u8>, add_arg: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ext_type);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(add_arg);
    ensures normal resource_return_none();
};

extern fn ext_free_releasing(s: ptr<mut, u8>, ext_type: u32, out: ptr<const, u8>, add_arg: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ext_type);
    ensures normal resource_borrowed(out);
    ensures normal resource_released(add_arg, nonnull);
    ensures normal resource_return_none();
};

extern fn ext_parse_ok(s: ptr<mut, u8>, ext_type: u32, in_: ptr<const, u8>, inlen: usize, al: ptr<mut, u8>, parse_arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ext_type);
    ensures normal resource_borrowed(in_);
    ensures normal resource_borrowed(inlen);
    ensures normal resource_borrowed(al);
    ensures normal resource_borrowed(parse_arg);
    ensures normal resource_return_none();
};

extern fn bio_cb_ok(b: ptr<mut, u8>, oper: i32, argp: ptr<const, u8>, len: usize, argi: i32, argl: i64, ret: i32, processed: ptr<mut, u8>) -> i64
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(b);
    ensures normal resource_borrowed(oper);
    ensures normal resource_borrowed(argp);
    ensures normal resource_borrowed(len);
    ensures normal resource_borrowed(argi);
    ensures normal resource_borrowed(argl);
    ensures normal resource_borrowed(ret);
    ensures normal resource_borrowed(processed);
    ensures normal resource_return_none();
};

extern fn bio_write_ok(b: ptr<mut, u8>, data: ptr<const, u8>, len: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(b);
    ensures normal resource_borrowed(data);
    ensures normal resource_borrowed(len);
    ensures normal resource_return_none();
};

extern fn bio_destroy_releasing(b: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(b, nonnull);
    ensures normal resource_return_none();
};
";

/// The public headers' view: the objects are opaque; the prototypes as the
/// pinned headers give them.
const HEADERS: &str = "\
typedef struct ssl_ctx_st SSL_CTX;
typedef struct ssl_st SSL;
typedef void (*SSL_CTX_keylog_cb_func)(const SSL *ssl, const char *line);
typedef int (*SSL_CTX_alpn_select_cb_func)(SSL *ssl, const unsigned char **out, unsigned char *outlen, const unsigned char *in, unsigned int inlen, void *arg);
typedef int (*custom_ext_add_cb)(SSL *s, unsigned int ext_type, const unsigned char **out, __SIZE_TYPE__ *outlen, int *al, void *add_arg);
typedef void (*custom_ext_free_cb)(SSL *s, unsigned int ext_type, const unsigned char *out, void *add_arg);
typedef int (*custom_ext_parse_cb)(SSL *s, unsigned int ext_type, const unsigned char *in, __SIZE_TYPE__ inlen, int *al, void *parse_arg);
int SSL_write(SSL *ssl, const void *buf, int num);
int SSL_shutdown(SSL *s);
void SSL_CTX_set_keylog_callback(SSL_CTX *ctx, SSL_CTX_keylog_cb_func cb);
void SSL_CTX_set_alpn_select_cb(SSL_CTX *ctx, SSL_CTX_alpn_select_cb_func cb, void *arg);
int SSL_set_record_padding_callback(SSL *ssl, __SIZE_TYPE__ (*cb)(SSL *ssl, int type, __SIZE_TYPE__ len, void *arg));
int SSL_CTX_add_client_custom_ext(SSL_CTX *ctx, unsigned int ext_type, custom_ext_add_cb add_cb, custom_ext_free_cb free_cb, void *add_arg, custom_ext_parse_cb parse_cb, void *parse_arg);
void keylog_ok(const SSL *ssl, const char *line);
void keylog_uncontracted(const SSL *ssl, const char *line);
int alpn_ok(SSL *ssl, const unsigned char **out, unsigned char *outlen, const unsigned char *in, unsigned int inlen, void *arg);
int alpn_uncontracted(SSL *ssl, const unsigned char **out, unsigned char *outlen, const unsigned char *in, unsigned int inlen, void *arg);
__SIZE_TYPE__ padding_ok(SSL *ssl, int type, __SIZE_TYPE__ len, void *arg);
__SIZE_TYPE__ padding_uncontracted(SSL *ssl, int type, __SIZE_TYPE__ len, void *arg);
int ext_add_ok(SSL *s, unsigned int ext_type, const unsigned char **out, __SIZE_TYPE__ *outlen, int *al, void *add_arg);
void ext_free_ok(SSL *s, unsigned int ext_type, const unsigned char *out, void *add_arg);
void ext_free_releasing(SSL *s, unsigned int ext_type, const unsigned char *out, void *add_arg);
int ext_parse_ok(SSL *s, unsigned int ext_type, const unsigned char *in, __SIZE_TYPE__ inlen, int *al, void *parse_arg);
typedef struct bio_st BIO;
typedef struct bio_method_st BIO_METHOD;
typedef long (*BIO_callback_fn_ex)(BIO *b, int oper, const char *argp, __SIZE_TYPE__ len, int argi, long argl, int ret, __SIZE_TYPE__ *processed);
void BIO_set_callback_ex(BIO *b, BIO_callback_fn_ex callback);
int BIO_meth_set_write(BIO_METHOD *biom, int (*write)(BIO *, const char *, int));
int BIO_meth_set_destroy(BIO_METHOD *biom, int (*destroy)(BIO *));
long bio_cb_ok(BIO *b, int oper, const char *argp, __SIZE_TYPE__ len, int argi, long argl, int ret, __SIZE_TYPE__ *processed);
long bio_cb_uncontracted(BIO *b, int oper, const char *argp, __SIZE_TYPE__ len, int argi, long argl, int ret, __SIZE_TYPE__ *processed);
int bio_write_ok(BIO *b, const char *data, int len);
int bio_write_uncontracted(BIO *b, const char *data, int len);
int bio_destroy_releasing(BIO *b);
";

const CALLER: &str = "\
int write_some(SSL *s, const char *b) { return SSL_write(s, b, 4); }
int shut(SSL *s) { return SSL_shutdown(s); }
";

/// The crate's modules and the candidates.
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

/// The new declarations load as slot clauses: each setter its registrations
/// (three for a custom extension setter), each handshake and I/O call 41
/// slot invocations, `SSL_shutdown` 17 (since OPENSSL-BIO-METHODS-AND-CALLBACKS,
/// which adds the 14 BIO slots to each).
#[test]
fn the_crate_states_every_handshake_slot() {
    let contracts = crate_contracts();
    for (setter, keys) in [
        (
            "SSL_CTX_set_keylog_callback",
            &["ssl_ctx_st.keylog_callback"][..],
        ),
        ("SSL_CTX_set_alpn_select_cb", &["ssl_ctx_st.alpn_select_cb"]),
        (
            "SSL_CTX_set_next_proto_select_cb",
            &["ssl_ctx_st.npn_select_cb"],
        ),
        (
            "SSL_CTX_set_next_protos_advertised_cb",
            &["ssl_ctx_st.npn_advertised_cb"],
        ),
        (
            "SSL_CTX_set_cert_verify_callback",
            &["ssl_ctx_st.app_verify_callback"],
        ),
        (
            "SSL_CTX_set_psk_client_callback",
            &["ssl_ctx_st.psk_client_callback"],
        ),
        (
            "SSL_set_psk_client_callback",
            &["ssl_ctx_st.psk_client_callback"],
        ),
        (
            "SSL_CTX_set_psk_use_session_callback",
            &["ssl_ctx_st.psk_use_session_cb"],
        ),
        (
            "SSL_set_psk_use_session_callback",
            &["ssl_ctx_st.psk_use_session_cb"],
        ),
        (
            "SSL_CTX_set_psk_server_callback",
            &["ssl_ctx_st.psk_server_callback"],
        ),
        (
            "SSL_set_psk_server_callback",
            &["ssl_ctx_st.psk_server_callback"],
        ),
        (
            "SSL_CTX_set_psk_find_session_callback",
            &["ssl_ctx_st.psk_find_session_cb"],
        ),
        (
            "SSL_set_psk_find_session_callback",
            &["ssl_ctx_st.psk_find_session_cb"],
        ),
        (
            "SSL_CTX_set_client_hello_cb",
            &["ssl_ctx_st.client_hello_cb"],
        ),
        (
            "SSL_CTX_set_record_padding_callback",
            &["ssl_ctx_st.record_padding_cb"],
        ),
        (
            "SSL_set_record_padding_callback",
            &["ssl_ctx_st.record_padding_cb"],
        ),
        (
            "SSL_CTX_set_allow_early_data_cb",
            &["ssl_ctx_st.allow_early_data_cb"],
        ),
        (
            "SSL_set_allow_early_data_cb",
            &["ssl_ctx_st.allow_early_data_cb"],
        ),
        (
            "SSL_CTX_add_custom_ext",
            &[
                "ssl_ctx_st.custom_ext_add_cb_ex",
                "ssl_ctx_st.custom_ext_free_cb_ex",
                "ssl_ctx_st.custom_ext_parse_cb_ex",
            ],
        ),
        (
            "SSL_CTX_add_client_custom_ext",
            &[
                "ssl_ctx_st.custom_ext_add_cb",
                "ssl_ctx_st.custom_ext_free_cb",
                "ssl_ctx_st.custom_ext_parse_cb",
            ],
        ),
        (
            "SSL_CTX_add_server_custom_ext",
            &[
                "ssl_ctx_st.custom_ext_add_cb",
                "ssl_ctx_st.custom_ext_free_cb",
                "ssl_ctx_st.custom_ext_parse_cb",
            ],
        ),
    ] {
        let declaration = contracts
            .retaining_contract(setter)
            .unwrap_or_else(|| panic!("{setter} is withheld"));
        let registered = declaration
            .callbacks
            .iter()
            .map(|clause| match clause {
                CallbackClause::MemberRegistered {
                    slot: true, member, ..
                } => member.spelling(),
                other => panic!("{setter}: {other:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(registered, keys, "{setter}");
        assert!(contracts.contract(setter).is_none(), "{setter}");
    }
    for call in ["SSL_write", "SSL_read", "SSL_connect", "SSL_do_handshake"] {
        let declaration = contracts.retaining_contract(call).expect("withheld");
        assert_eq!(declaration.callbacks.len(), 41, "{call}");
        assert!(
            declaration
                .callbacks
                .iter()
                .all(|clause| matches!(clause, CallbackClause::MemberInvoked { slot: true, .. })),
            "{call}"
        );
    }
    assert_eq!(
        contracts
            .retaining_contract("SSL_shutdown")
            .expect("withheld")
            .callbacks
            .len(),
        17
    );
}

/// Every new slot closed over identity candidates (or empty), each setter's
/// `arg` a null constant: the I/O call decides.
#[test]
fn identity_candidates_decide_the_io_call() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let registrar = "\
void reg(SSL_CTX *c, SSL *s) {
    SSL_CTX_set_keylog_callback(c, keylog_ok);
    SSL_CTX_set_alpn_select_cb(c, alpn_ok, 0);
    SSL_set_record_padding_callback(s, padding_ok);
    SSL_CTX_add_client_custom_ext(c, 1000, ext_add_ok, ext_free_ok, 0, ext_parse_ok, 0);
}
";
    let run = run_of(registrar, &contracts);
    for name in ["write_some", "shut"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert!(
            !refused(&leaves, "callback-"),
            "{name}: no callback refusal: {leaves:?}"
        );
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
}

/// Fluent Bit's shape (`SSL_CTX_set_keylog_callback(ssl_ctx, ssl_key_logger)`,
/// `SSL_CTX_set_alpn_select_cb(ctx->ctx, tls_context_server_alpn_select_callback,
/// ctx)`): a candidate with no contract in force refuses the I/O call.
#[test]
fn a_new_slot_with_an_uncontracted_candidate_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    for (registrar, candidate) in [
        (
            "void reg(SSL_CTX *c) { SSL_CTX_set_keylog_callback(c, keylog_uncontracted); }\n",
            "`keylog_uncontracted` (from `ssl_ctx_st.keylog_callback`)",
        ),
        (
            "void reg(SSL_CTX *c) { SSL_CTX_set_alpn_select_cb(c, alpn_uncontracted, 0); }\n",
            "`alpn_uncontracted` (from `ssl_ctx_st.alpn_select_cb`)",
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

/// A multi-slot setter registers each of its callbacks: a free callback that
/// releases the `add_arg` the library hands it refuses the I/O call as
/// effectful,
/// although the add and parse callbacks beside it are identities.
#[test]
fn every_registration_of_a_multi_slot_setter_is_read() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let registrar = "\
void reg(SSL_CTX *c) {
    SSL_CTX_add_client_custom_ext(c, 1000, ext_add_ok, ext_free_releasing, 0, ext_parse_ok, 0);
}
";
    let run = run_of(registrar, &contracts);
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("ext_free_releasing") && leaf.contains("callback-effectful")),
        "{leaves:?}"
    );
}

/// A new slot no loaded declaration registers has undeclared writers:
/// without `ssl_handshake.qv` the keylog and custom extension slots are never
/// decided, even with nothing registered.
#[test]
fn a_new_slot_no_declaration_registers_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("ssl_io.qv", SSL_IO),
    ]);
    let run = run_of("void nothing(void) { }\n", &contracts);
    assert!(run.decides("ssl_ctx_st.keylog_callback", true).is_err());
    assert!(
        run.decides("ssl_ctx_st.custom_ext_free_cb_ex", true)
            .is_err()
    );
    assert!(run.decides("ssl_ctx_st.record_padding_cb", true).is_err());
    for name in ["write_some", "shut"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "no declaration registers"),
            "{name}: {leaves:?}"
        );
    }
}

/// `SSL_shutdown` pads its close_notify record: an uncontracted record
/// padding candidate refuses it.
#[test]
fn shutdown_reads_the_record_padding_slot() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let run = run_of(
        "void reg(SSL *s) { SSL_set_record_padding_callback(s, padding_uncontracted); }\n",
        &contracts,
    );
    let (status, leaves) = row_of("shut", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("padding_uncontracted")
                && leaf.contains("callback-uncontracted")),
        "{leaves:?}"
    );
}

/// Card OPENSSL-BIO-METHODS-AND-CALLBACKS: the BIO keys every I/O call and
/// `SSL_shutdown` invoke.
const BIO_KEYS: [&str; 14] = [
    "bio_st.callback_ex",
    "bio_st.callback",
    "bio_method_st.bwrite",
    "bio_method_st.bwrite_old",
    "bio_method_st.bread",
    "bio_method_st.bread_old",
    "bio_method_st.bputs",
    "bio_method_st.bgets",
    "bio_method_st.ctrl",
    "bio_method_st.create",
    "bio_method_st.destroy",
    "bio_method_st.callback_ctrl",
    "bio_method_st.bsendmmsg",
    "bio_method_st.brecvmmsg",
];

/// The 14 setters load as their registrations; each I/O call and
/// `SSL_shutdown` invoke every BIO key on the connection with every actual
/// the callee's own.
#[test]
fn the_crate_states_every_bio_slot() {
    let contracts = crate_contracts();
    for (setter, key) in [
        ("BIO_set_callback_ex", "bio_st.callback_ex"),
        ("BIO_set_callback", "bio_st.callback"),
        ("BIO_meth_set_write_ex", "bio_method_st.bwrite"),
        ("BIO_meth_set_write", "bio_method_st.bwrite_old"),
        ("BIO_meth_set_read_ex", "bio_method_st.bread"),
        ("BIO_meth_set_read", "bio_method_st.bread_old"),
        ("BIO_meth_set_puts", "bio_method_st.bputs"),
        ("BIO_meth_set_gets", "bio_method_st.bgets"),
        ("BIO_meth_set_ctrl", "bio_method_st.ctrl"),
        ("BIO_meth_set_create", "bio_method_st.create"),
        ("BIO_meth_set_destroy", "bio_method_st.destroy"),
        ("BIO_meth_set_callback_ctrl", "bio_method_st.callback_ctrl"),
        ("BIO_meth_set_sendmmsg", "bio_method_st.bsendmmsg"),
        ("BIO_meth_set_recvmmsg", "bio_method_st.brecvmmsg"),
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
    for call in [
        "SSL_write",
        "SSL_read",
        "SSL_connect",
        "SSL_do_handshake",
        "SSL_shutdown",
    ] {
        let declaration = contracts.retaining_contract(call).expect("withheld");
        for key in BIO_KEYS {
            let invoked = declaration.callbacks.iter().find(|clause| {
                matches!(clause, CallbackClause::MemberInvoked { slot: true, member, .. }
                    if member.spelling() == key)
            });
            let Some(CallbackClause::MemberInvoked { actuals, .. }) = invoked else {
                panic!("{call} invokes {key}");
            };
            assert!(
                actuals
                    .iter()
                    .all(|actual| matches!(actual, CallbackActual::Supplied)),
                "{call} {key}: {actuals:?}"
            );
        }
    }
}

/// Identity BIO candidates beside the identity handshake candidates: the
/// I/O call and `SSL_shutdown` decide.
#[test]
fn identity_bio_candidates_decide_the_io_call() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let registrar = "\
void reg(BIO *b, BIO_METHOD *m) {
    BIO_set_callback_ex(b, bio_cb_ok);
    BIO_meth_set_write(m, bio_write_ok);
}
";
    let run = run_of(registrar, &contracts);
    for name in ["write_some", "shut"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert!(
            !refused(&leaves, "callback-"),
            "{name}: no callback refusal: {leaves:?}"
        );
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
}

/// libevent's `bufferevent_openssl.c` shape (`BIO_meth_set_write(methods,
/// bio_bufferevent_write)`) and a BIO callback with no contract in force:
/// each refuses the I/O call and `SSL_shutdown`, naming the candidate.
#[test]
fn an_uncontracted_bio_candidate_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    for (registrar, candidate) in [
        (
            "void reg(BIO_METHOD *m) { BIO_meth_set_write(m, bio_write_uncontracted); }\n",
            "bio_write_uncontracted",
        ),
        (
            "void reg(BIO *b) { BIO_set_callback_ex(b, bio_cb_uncontracted); }\n",
            "bio_cb_uncontracted",
        ),
    ] {
        let run = run_of(registrar, &contracts);
        for name in ["write_some", "shut"] {
            let (status, leaves) = row_of(name, run.clone(), &contracts);
            assert_ne!(status, AnalysisStatus::Safe, "{name}");
            assert!(
                leaves
                    .iter()
                    .any(|leaf| leaf.contains(candidate) && leaf.contains("callback-uncontracted")),
                "{name}: {leaves:?}"
            );
        }
    }
}

/// A destroy function that releases the BIO the library hands it is not the
/// identity: the I/O call refuses (the method slots are stated beyond the
/// ones a record write needs, and stating more never improves a verdict).
#[test]
fn an_effectful_bio_method_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let run = run_of(
        "void reg(BIO_METHOD *m) { BIO_meth_set_destroy(m, bio_destroy_releasing); }\n",
        &contracts,
    );
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("bio_destroy_releasing") && leaf.contains("callback-")),
        "{leaves:?}"
    );
}

/// Without `bio_slots.qv` no loaded declaration registers a BIO key: the
/// slots are never decided, even with nothing registered.
#[test]
fn a_bio_slot_no_declaration_registers_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("ssl_io.qv", SSL_IO),
        ("ssl_handshake.qv", SSL_HANDSHAKE),
    ]);
    let run = run_of("void nothing(void) { }\n", &contracts);
    for key in BIO_KEYS {
        assert!(run.decides(key, true).is_err(), "{key}");
    }
    for name in ["write_some", "shut"] {
        let (status, leaves) = row_of(name, run.clone(), &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "no declaration registers"),
            "{name}: {leaves:?}"
        );
    }
}

/// `BIO_free` states no callback clause beside its reference drop (C04.19
/// admits none): its callback, destroy function and ex_data callbacks rest
/// on the premise `openssl_bio_free_callbacks`, and the loader still rejects
/// a slot invocation beside the drop.
#[test]
fn bio_free_states_no_invocation_beside_its_drop() {
    let bio = include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/bio.qv");
    assert!(bio.contains("resource_premise(openssl_bio_free_callbacks,"));
    let contracts = contracts(&[("bio.qv", bio), ("bio_slots.qv", BIO_SLOTS)]);
    assert!(
        contracts
            .retaining_contract("BIO_free")
            .is_none_or(|declaration| declaration.callbacks.is_empty())
    );
    let stated = bio.replacen(
        "    ensures normal resource_reference_dropped(a, nullable);\n",
        "    ensures normal resource_reference_dropped(a, nullable);\n    ensures normal \
         resource_callback_slot_invoked(a, bio_st.callback_ex, a, _, _, _, _, _, _, _);\n",
        1,
    );
    assert_ne!(stated, bio);
    let mut sources = BTreeMap::new();
    sources.insert("bio.qv".to_owned(), stated);
    sources.insert("bio_slots.qv".to_owned(), BIO_SLOTS.to_owned());
    let refused = load_resource_contract_sources(&sources).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("a callback clause stands only beside retentions and plain predicates"),
        "{refused}"
    );
}
