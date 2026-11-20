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
//! candidates decide.
//!
//! Card OPENSSL-FREE-SESSION-REMOVE-INVOCATION (C04.19 amended in place, rule
//! 12n; this target too): `BIO_free`, `SSL_free` and `SSL_CTX_free` state
//! the slots their last drop runs beside the drop, and the loader admits a
//! slot invocation beside drops only.
//!
//! Card OPENSSL-FREE-INDIRECT-CALLBACKS (rule 12n, C04.19.3; this target, so
//! the read census needs no new unit): a certificate store's lookup method
//! shutdown and free slots (`x509_lookup_slots.qv`), stated at
//! `X509_STORE_free`, `SSL_CTX_free` and `SSL_free`, with the same arms.

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
/// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: the remaining handshake slots'
/// setters and the certificate store's.
const SSL_HANDSHAKE_SETTERS: &str = include_str!(
    "../../../providers/quirin-provider-openssl/contracts/openssl/ssl_handshake_setters.qv"
);
const X509_STORE_SLOTS: &str = include_str!(
    "../../../providers/quirin-provider-openssl/contracts/openssl/x509_store_slots.qv"
);
/// Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the teardown slots'
/// setters, which every free running them needs registered.
const TEARDOWN_SLOTS: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/teardown_slots.qv");

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

extern fn verify_store_ok(ok: i32, ctx: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ok);
    ensures normal resource_borrowed(ctx);
    ensures normal resource_return_none();
};

extern fn async_ok(s: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn ticket_gen_ok(s: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn ticket_dec_ok(s: ptr<mut, u8>, ss: ptr<mut, u8>, keyname: ptr<const, u8>, keyname_length: usize, status: i32, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(ss);
    ensures normal resource_borrowed(keyname);
    ensures normal resource_borrowed(keyname_length);
    ensures normal resource_borrowed(status);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn ticket_dec_releasing(s: ptr<mut, u8>, ss: ptr<mut, u8>, keyname: ptr<const, u8>, keyname_length: usize, status: i32, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_released(ss, nonnull);
    ensures normal resource_borrowed(keyname);
    ensures normal resource_borrowed(keyname_length);
    ensures normal resource_borrowed(status);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn secret_ok(s: ptr<mut, u8>, secret: ptr<mut, u8>, secret_len: ptr<mut, u8>, peer_ciphers: ptr<mut, u8>, cipher: ptr<mut, ptr<const, u8>>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(secret);
    ensures normal resource_borrowed(secret_len);
    ensures normal resource_borrowed(peer_ciphers);
    ensures normal resource_borrowed(cipher);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn secret_needs_ciphers(s: ptr<mut, u8>, secret: ptr<mut, u8>, secret_len: ptr<mut, u8>, peer_ciphers: ptr<mut, u8>, cipher: ptr<mut, ptr<const, u8>>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(secret);
    ensures normal resource_borrowed(secret_len);
    ensures normal resource_owned(peer_ciphers, nonnull);
    ensures normal resource_borrowed(cipher);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};
";

/// The public headers' view: the objects are opaque; the prototypes as the
/// pinned headers give them.
// Target-portable spellings: plain `long` (and a typedef of it) is unreadable to
// the declared-ABI reader unless the analysing Clang is LP64 (rule 11q), and
// the UCRT's headers are unread on Windows, so these synthetic declarations
// spell sizes `__SIZE_TYPE__`/`__PTRDIFF_TYPE__` and 64-bit `long long`.
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
typedef long long (*BIO_callback_fn_ex)(BIO *b, int oper, const char *argp, __SIZE_TYPE__ len, int argi, long long argl, int ret, __SIZE_TYPE__ *processed);
void BIO_set_callback_ex(BIO *b, BIO_callback_fn_ex callback);
int BIO_meth_set_write(BIO_METHOD *biom, int (*write)(BIO *, const char *, int));
int BIO_meth_set_destroy(BIO_METHOD *biom, int (*destroy)(BIO *));
long long bio_cb_ok(BIO *b, int oper, const char *argp, __SIZE_TYPE__ len, int argi, long long argl, int ret, __SIZE_TYPE__ *processed);
long long bio_cb_uncontracted(BIO *b, int oper, const char *argp, __SIZE_TYPE__ len, int argi, long long argl, int ret, __SIZE_TYPE__ *processed);
int bio_write_ok(BIO *b, const char *data, int len);
int bio_write_uncontracted(BIO *b, const char *data, int len);
int bio_destroy_releasing(BIO *b);
typedef struct x509_store_st X509_STORE;
typedef struct x509_store_ctx_st X509_STORE_CTX;
typedef int (*X509_STORE_CTX_verify_cb)(int, X509_STORE_CTX *);
void X509_STORE_set_verify_cb(X509_STORE *xs, X509_STORE_CTX_verify_cb verify_cb);
int verify_store_ok(int ok, X509_STORE_CTX *ctx);
int verify_store_uncontracted(int ok, X509_STORE_CTX *ctx);
typedef int (*SSL_async_callback_fn)(SSL *s, void *arg);
int SSL_CTX_set_async_callback(SSL_CTX *ctx, SSL_async_callback_fn callback);
int async_ok(SSL *s, void *arg);
int async_uncontracted(SSL *s, void *arg);
struct ssl_session_st;
struct stack_st_SSL_CIPHER;
typedef int (*SSL_CTX_generate_session_ticket_fn)(SSL *s, void *arg);
typedef int (*SSL_CTX_decrypt_session_ticket_fn)(SSL *s, struct ssl_session_st *ss, const unsigned char *keyname, __SIZE_TYPE__ keyname_length, int status, void *arg);
int SSL_CTX_set_session_ticket_cb(SSL_CTX *ctx, SSL_CTX_generate_session_ticket_fn gen_cb, SSL_CTX_decrypt_session_ticket_fn dec_cb, void *arg);
int ticket_gen_ok(SSL *s, void *arg);
int ticket_dec_ok(SSL *s, struct ssl_session_st *ss, const unsigned char *keyname, __SIZE_TYPE__ keyname_length, int status, void *arg);
int ticket_dec_releasing(SSL *s, struct ssl_session_st *ss, const unsigned char *keyname, __SIZE_TYPE__ keyname_length, int status, void *arg);
typedef int (*tls_session_secret_cb_fn)(SSL *s, void *secret, int *secret_len, struct stack_st_SSL_CIPHER *peer_ciphers, const void **cipher, void *arg);
int SSL_set_session_secret_cb(SSL *s, tls_session_secret_cb_fn session_secret_cb, void *arg);
int secret_ok(SSL *s, void *secret, int *secret_len, struct stack_st_SSL_CIPHER *peer_ciphers, const void **cipher, void *arg);
int secret_needs_ciphers(SSL *s, void *secret, int *secret_len, struct stack_st_SSL_CIPHER *peer_ciphers, const void **cipher, void *arg);
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
        ("ssl_handshake_setters.qv", SSL_HANDSHAKE_SETTERS),
        ("x509_store_slots.qv", X509_STORE_SLOTS),
        // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS.
        ("teardown_slots.qv", TEARDOWN_SLOTS),
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
/// which adds the 14 BIO slots to each). Since
/// OPENSSL-REMAINING-HANDSHAKE-SETTERS 67 and 18.
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
        // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: and the twelve
        // teardown slots of what the handshake frees inside.
        assert_eq!(declaration.callbacks.len(), 79, "{call}");
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
        // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: and twelve teardown
        // slots.
        30
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
    "bio_st.bio_callback_ex",
    "bio_st.bio_callback",
    "bio_method_st.meth_write_ex",
    "bio_method_st.meth_write",
    "bio_method_st.meth_read_ex",
    "bio_method_st.meth_read",
    "bio_method_st.meth_puts",
    "bio_method_st.meth_gets",
    "bio_method_st.meth_ctrl",
    "bio_method_st.meth_create",
    "bio_method_st.meth_destroy",
    "bio_method_st.meth_callback_ctrl",
    "bio_method_st.meth_sendmmsg",
    "bio_method_st.meth_recvmmsg",
];

/// The 14 setters load as their registrations; each I/O call and
/// `SSL_shutdown` invoke every BIO key on the connection with every actual
/// the callee's own.
#[test]
fn the_crate_states_every_bio_slot() {
    let contracts = crate_contracts();
    for (setter, key) in [
        ("BIO_set_callback_ex", "bio_st.bio_callback_ex"),
        ("BIO_set_callback", "bio_st.bio_callback"),
        ("BIO_meth_set_write_ex", "bio_method_st.meth_write_ex"),
        ("BIO_meth_set_write", "bio_method_st.meth_write"),
        ("BIO_meth_set_read_ex", "bio_method_st.meth_read_ex"),
        ("BIO_meth_set_read", "bio_method_st.meth_read"),
        ("BIO_meth_set_puts", "bio_method_st.meth_puts"),
        ("BIO_meth_set_gets", "bio_method_st.meth_gets"),
        ("BIO_meth_set_ctrl", "bio_method_st.meth_ctrl"),
        ("BIO_meth_set_create", "bio_method_st.meth_create"),
        ("BIO_meth_set_destroy", "bio_method_st.meth_destroy"),
        (
            "BIO_meth_set_callback_ctrl",
            "bio_method_st.meth_callback_ctrl",
        ),
        ("BIO_meth_set_sendmmsg", "bio_method_st.meth_sendmmsg"),
        ("BIO_meth_set_recvmmsg", "bio_method_st.meth_recvmmsg"),
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
            // Card CALLBACK-SUPPLIED-NULL-ACTUAL: every actual is the
            // callee's; the callback's `argp` (and the `_ex` form's
            // `processed`) may be NULL by BIO_set_callback (3.5).
            assert_eq!(
                actuals,
                &bio_actuals(key, actuals.len()),
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

// Card OPENSSL-BIO-READER-INVOCATIONS (gateway rule 57 amended; contract
// specification C04.5 "Beside a slot invocation"): the PEM and PKCS#12
// readers state the 14 BIO slots on their BIO formal beside their
// literal-argument premise. In this target too, so the read census needs no
// new unit.

const PEM: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/pem.qv");
const PKCS12: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/pkcs12.qv");
const X509: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/x509.qv");
const EVP: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/evp.qv");

/// The readers' callers: each within its premise (a null `x` or `p12`, a null
/// `cb`, the shape of the predecessor fixture's `read_cert` and Monkey's
/// `tls_load_builtin_credentials`), and each reuse arm outside it.
const READERS: &str = "\
#define NULL ((void *)0)
typedef struct x509_st X509;
typedef struct evp_pkey_st EVP_PKEY;
typedef struct PKCS12_st PKCS12;
typedef int pem_password_cb(char *buf, int size, int rwflag, void *u);
X509 *PEM_read_bio_X509(BIO *out, X509 **x, pem_password_cb *cb, void *u);
EVP_PKEY *PEM_read_bio_PrivateKey(BIO *out, EVP_PKEY **x, pem_password_cb *cb, void *u);
PKCS12 *d2i_PKCS12_bio(BIO *bp, PKCS12 **p12);
void X509_free(X509 *a);
void EVP_PKEY_free(EVP_PKEY *pkey);
void PKCS12_free(PKCS12 *a);
int read_cert(BIO *bio) { X509 *c = PEM_read_bio_X509(bio, NULL, NULL, NULL); if (!c) return -1; X509_free(c); return 0; }
int read_key(BIO *bio) { EVP_PKEY *k = PEM_read_bio_PrivateKey(bio, NULL, NULL, NULL); if (!k) return -1; EVP_PKEY_free(k); return 0; }
int read_p12(BIO *bio) { PKCS12 *p = d2i_PKCS12_bio(bio, NULL); if (!p) return -1; PKCS12_free(p); return 0; }
int read_cert_reuse(BIO *bio, X509 **slot) { X509 *c = PEM_read_bio_X509(bio, slot, NULL, NULL); if (!c) return 0; return 1; }
int read_p12_reuse(BIO *bio, PKCS12 **slot) { PKCS12 *p = d2i_PKCS12_bio(bio, slot); if (!p) return 0; return 1; }
";

const WITHIN: [&str; 3] = ["read_cert", "read_key", "read_p12"];
const OUTSIDE: [&str; 2] = ["read_cert_reuse", "read_p12_reuse"];

fn reader_contracts(with_slots: bool) -> ResourceContracts {
    let mut modules = vec![
        ("pem.qv", PEM),
        ("pkcs12.qv", PKCS12),
        ("x509.qv", X509),
        ("evp.qv", EVP),
        ("teardown_slots.qv", TEARDOWN_SLOTS),
    ];
    if with_slots {
        modules.push(("bio_slots.qv", BIO_SLOTS));
    }
    contracts(&modules)
}

fn reader_run(registrar: &str, contracts: &ResourceContracts) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(
        vec![facts(registrar), facts(READERS), declared(contracts)],
        BTreeMap::new(),
    ))
}

fn reader_row(
    name: &str,
    run: Option<Arc<RunMemberFacts>>,
    contracts: &ResourceContracts,
) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{HEADERS}{READERS}");
    let outcome = scoped(run, || {
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

/// Each reader is withheld and invokes every BIO key on its BIO formal
/// (position 0) with every actual the callee's own; the PEM readers keep
/// their password callback's null default.
#[test]
fn the_readers_state_every_bio_slot() {
    let contracts = reader_contracts(true);
    for (reader, invocations) in [
        // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: and the nine teardown
        // slots of a key the read frees.
        ("PEM_read_bio_X509", 24),
        ("PEM_read_bio_PrivateKey", 24),
        ("d2i_PKCS12_bio", 23),
    ] {
        assert!(contracts.contract(reader).is_none(), "{reader}");
        let declaration = contracts.retaining_contract(reader).expect("withheld");
        assert_eq!(declaration.callbacks.len(), invocations, "{reader}");
        for key in BIO_KEYS {
            let invoked = declaration.callbacks.iter().find(|clause| {
                matches!(clause, CallbackClause::MemberInvoked { slot: true, member, .. }
                    if member.spelling() == key)
            });
            let Some(CallbackClause::MemberInvoked {
                object, actuals, ..
            }) = invoked
            else {
                panic!("{reader} invokes {key}");
            };
            assert!(
                matches!(
                    object,
                    qcore_schemas::callback::MemberObject::Parameter { index: 0 }
                ),
                "{reader} {key}"
            );
            // Card CALLBACK-SUPPLIED-NULL-ACTUAL: every actual is the
            // callee's; the callback's `argp` (and the `_ex` form's
            // `processed`) may be NULL by BIO_set_callback (3.5).
            assert_eq!(
                actuals,
                &bio_actuals(key, actuals.len()),
                "{reader} {key}: {actuals:?}"
            );
        }
        let defaults = declaration
            .callbacks
            .iter()
            .filter(|clause| {
                matches!(
                    clause,
                    CallbackClause::Invoked {
                        null_default: true,
                        ..
                    }
                )
            })
            .count();
        // The BIO slots and (card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS)
        // the nine teardown slots are slot invocations; the rest is the
        // password callback's null default.
        assert_eq!(defaults, invocations - 14 - 9, "{reader}");
    }
}

/// Within the premise, with nothing or only identity candidates registered,
/// the readers decide: the r29 null-callback reading does not regress.
#[test]
fn identity_bio_candidates_decide_the_readers() {
    if !clang_available() {
        return;
    }
    let contracts = reader_contracts(true);
    for registrar in [
        "void nothing(void) { }\n",
        "void reg(BIO *b, BIO_METHOD *m) { BIO_set_callback_ex(b, bio_cb_ok); \
         BIO_meth_set_write(m, bio_write_ok); }\n",
    ] {
        let run = reader_run(registrar, &contracts);
        for name in WITHIN {
            let (status, leaves) = reader_row(name, Some(run.clone()), &contracts);
            assert!(!refused(&leaves, "callback-"), "{name}: {leaves:?}");
            assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
        }
    }
}

/// A BIO candidate with no contract in force, or one releasing the BIO it is
/// handed, refuses every reader within its premise, naming the candidate.
#[test]
fn a_bio_candidate_that_is_not_the_identity_refuses_the_readers() {
    if !clang_available() {
        return;
    }
    let contracts = reader_contracts(true);
    for (registrar, candidate) in [
        (
            "void reg(BIO_METHOD *m) { BIO_meth_set_write(m, bio_write_uncontracted); }\n",
            "bio_write_uncontracted",
        ),
        (
            "void reg(BIO *b) { BIO_set_callback_ex(b, bio_cb_uncontracted); }\n",
            "bio_cb_uncontracted",
        ),
        (
            "void reg(BIO_METHOD *m) { BIO_meth_set_destroy(m, bio_destroy_releasing); }\n",
            "bio_destroy_releasing",
        ),
    ] {
        let run = reader_run(registrar, &contracts);
        for name in WITHIN {
            let (status, leaves) = reader_row(name, Some(run.clone()), &contracts);
            assert_ne!(status, AnalysisStatus::Safe, "{name}");
            assert!(
                leaves
                    .iter()
                    .any(|leaf| leaf.contains(candidate) && leaf.contains("callback-")),
                "{name}: {leaves:?}"
            );
        }
    }
}

/// Outside the premise the call is undeclared before the callback route:
/// no BIO slot is decided there, even with an uncontracted candidate
/// registered.
#[test]
fn a_reader_outside_its_premise_decides_no_bio_slot() {
    if !clang_available() {
        return;
    }
    let contracts = reader_contracts(true);
    let run = reader_run(
        "void reg(BIO *b) { BIO_set_callback_ex(b, bio_cb_uncontracted); }\n",
        &contracts,
    );
    for name in OUTSIDE {
        let (status, leaves) = reader_row(name, Some(run.clone()), &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "outside its declared literal-argument premise"),
            "{name}: {leaves:?}"
        );
        assert!(!refused(&leaves, "callback-"), "{name}: {leaves:?}");
        assert!(
            !refused(&leaves, "bio_cb_uncontracted"),
            "{name}: {leaves:?}"
        );
    }
}

/// Without `bio_slots.qv` no loaded declaration registers a BIO key, and
/// without the run's facts no slot is decided: either way the readers
/// refuse within their premise.
#[test]
fn a_reader_bio_slot_without_registrations_or_run_facts_refuses() {
    if !clang_available() {
        return;
    }
    let unregistered = reader_contracts(false);
    let run = reader_run("void nothing(void) { }\n", &unregistered);
    for name in WITHIN {
        let (status, leaves) = reader_row(name, Some(run.clone()), &unregistered);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "no declaration registers"),
            "{name}: {leaves:?}"
        );
    }
    let contracts = reader_contracts(true);
    for name in WITHIN {
        let (status, leaves) = reader_row(name, None, &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
}

/// Card CALLBACK-SUPPLIED-NULL-ACTUAL: the actuals a BIO slot invocation
/// states: every one `_`, but `supplied_nullable` where `BIO_set_callback`
/// (3.5) passes NULL (`argp`; `processed` of the `_ex` form).
fn bio_actuals(key: &str, arity: usize) -> Vec<CallbackActual> {
    let nullable: &[usize] = match key {
        "bio_st.bio_callback_ex" => &[2, 7],
        "bio_st.bio_callback" => &[2],
        _ => &[],
    };
    (0..arity)
        .map(|at| {
            if nullable.contains(&at) {
                CallbackActual::SuppliedNullable
            } else {
                CallbackActual::Supplied
            }
        })
        .collect()
}

// Card OPENSSL-FREE-SESSION-REMOVE-INVOCATION (contract specification C04.19
// amended in place, gateway rule 12n): a slot invocation beside a reference
// drop. The frees state the session-remove slot (`SSL_free`,
// `SSL_CTX_free`) and a BIO's callback and destroy slots (`BIO_free`,
// `SSL_free`), every actual `_`.

const BIO: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/bio.qv");
const LOOKUP: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/lookup.qv");
/// Card OPENSSL-FREE-INDIRECT-CALLBACKS: the lookup method table setters and
/// the store's free.
const X509_LOOKUP_SLOTS: &str = include_str!(
    "../../../providers/quirin-provider-openssl/contracts/openssl/x509_lookup_slots.qv"
);
const X509_STORE: &str =
    include_str!("../../../providers/quirin-provider-openssl/contracts/openssl/x509_store.qv");

const FREE_HEADERS: &str = "\
typedef struct ssl_method_st SSL_METHOD;
typedef struct ssl_session_st SSL_SESSION;
const SSL_METHOD *TLS_server_method(void);
SSL_CTX *SSL_CTX_new(const SSL_METHOD *meth);
SSL *SSL_new(SSL_CTX *ctx);
void SSL_free(SSL *ssl);
void SSL_CTX_free(SSL_CTX *ctx);
void SSL_CTX_sess_set_remove_cb(SSL_CTX *ctx, void (*remove_session_cb)(SSL_CTX *ctx, SSL_SESSION *sess));
BIO *BIO_new_file(const char *filename, const char *mode);
int BIO_free(BIO *a);
void remove_ok(SSL_CTX *ctx, SSL_SESSION *sess);
void remove_uncontracted(SSL_CTX *ctx, SSL_SESSION *sess);
typedef struct x509_lookup_method_st X509_LOOKUP_METHOD;
typedef struct x509_lookup_st X509_LOOKUP;
X509_STORE *store_new(void);
void X509_STORE_free(X509_STORE *xs);
int X509_LOOKUP_meth_set_free(X509_LOOKUP_METHOD *method, void (*free_fn)(X509_LOOKUP *ctx));
int X509_LOOKUP_meth_set_shutdown(X509_LOOKUP_METHOD *method, int (*shutdown)(X509_LOOKUP *ctx));
void lookup_free_ok(X509_LOOKUP *ctx);
void lookup_free_uncontracted(X509_LOOKUP *ctx);
int lookup_shutdown_ok(X509_LOOKUP *ctx);
int lookup_shutdown_releasing(X509_LOOKUP *ctx);
typedef struct rsa_meth_st RSA_METHOD;
typedef struct rsa_st RSA;
typedef struct engine_st ENGINE;
int RSA_meth_set_finish(RSA_METHOD *rsa, int (*finish)(RSA *rsa));
int ENGINE_set_destroy_function(ENGINE *e, int (*destroy_f)(ENGINE *));
int rsa_finish_uncontracted(RSA *rsa);
int engine_destroy_uncontracted(ENGINE *e);
";

/// Each free acquires its object and drops the one reference it holds.
const FREES: &str = "\
void free_ctx(void) { const SSL_METHOD *m = TLS_server_method(); SSL_CTX *c = SSL_CTX_new(m); if (!c) return; SSL_CTX_free(c); }
void free_ssl(SSL_CTX *c) { SSL *s = SSL_new(c); if (!s) return; SSL_free(s); }
void free_bio(void) { BIO *b = BIO_new_file(\"f\", \"r\"); if (!b) return; BIO_free(b); }
void free_store(void) { X509_STORE *xs = store_new(); if (!xs) return; X509_STORE_free(xs); }
";

const REMOVE_CANDIDATES: &str = "module contract;

extern fn remove_ok(ctx: ptr<mut, u8>, sess: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(sess);
    ensures normal resource_return_none();
};

extern fn store_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
    ensures normal resource_return_kind(reference);
};

extern fn lookup_free_ok(ctx: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ctx);
    ensures normal resource_return_none();
};

extern fn lookup_shutdown_ok(ctx: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ctx);
    ensures normal resource_return_none();
};

extern fn lookup_shutdown_releasing(ctx: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(ctx, nonnull);
    ensures normal resource_return_none();
};
";

fn free_contracts(modules: &[(&str, &str)]) -> ResourceContracts {
    let mut sources = modules
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect::<BTreeMap<_, _>>();
    sources.insert("candidates.qv".to_owned(), CANDIDATES.to_owned());
    sources.insert("remove.qv".to_owned(), REMOVE_CANDIDATES.to_owned());
    load_resource_contract_sources(&sources).expect("the crate's modules load")
}

fn free_crate() -> ResourceContracts {
    free_contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("bio.qv", BIO),
        ("lookup.qv", LOOKUP),
        ("bio_slots.qv", BIO_SLOTS),
        ("x509_store.qv", X509_STORE),
        ("x509_lookup_slots.qv", X509_LOOKUP_SLOTS),
        ("teardown_slots.qv", TEARDOWN_SLOTS),
        ("ssl_io.qv", SSL_IO),
    ])
}

/// Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the teardown slots every
/// free of a key holder states (in the module's order), every actual the
/// library's.
const KEY_FREED: [&str; 9] = [
    "dh_method.dh_meth_finish",
    "dsa_method.dsa_meth_finish",
    "ec_key_method_st.ec_key_meth_finish",
    "engine_st.engine_destroy",
    "engine_st.engine_finish",
    "engine_st.engine_pkey_asn1_meths",
    "engine_st.engine_pkey_meths",
    "evp_pkey_asn1_method_st.asn1_meth_pkey_free",
    "rsa_meth_st.rsa_meth_finish",
];

/// The actuals of a slot a free states: the BIO pages' NULLs, the engine
/// enumerators' NULL method and nids pointers (crypto/engine/tb_pkmeth.c:110,
/// 112 at 3.5.5), and the SSL object handed to the message and info
/// callbacks.
fn free_actuals(key: &str, arity: usize) -> Vec<CallbackActual> {
    let parameter: &[usize] = match key {
        "ssl_ctx_st.msg_callback" => &[5],
        "ssl_ctx_st.info_callback" => &[0],
        _ => &[],
    };
    let nullable: &[usize] = match key {
        "engine_st.engine_pkey_meths" | "engine_st.engine_pkey_asn1_meths" => &[1, 2],
        _ => &[],
    };
    bio_actuals(key, arity)
        .into_iter()
        .enumerate()
        .map(|(at, actual)| {
            if parameter.contains(&at) {
                CallbackActual::Parameter { index: 0 }
            } else if nullable.contains(&at) {
                CallbackActual::SuppliedNullable
            } else {
                actual
            }
        })
        .collect()
}

/// Card OPENSSL-FREE-INDIRECT-CALLBACKS: the lookup method slots every free
/// of a store states, after its own slots.
const LOOKUP_FREED: [&str; 2] = [
    "x509_lookup_method_st.lookup_meth_shutdown",
    "x509_lookup_method_st.lookup_meth_free",
];

/// The row of `name` in `headers` + `body`, under `run`'s member facts.
fn unit_row(
    headers: &str,
    body: &str,
    name: &str,
    run: Option<Arc<RunMemberFacts>>,
    contracts: &ResourceContracts,
) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{headers}{body}");
    let outcome = scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            name,
            contracts,
        )
    })
    .unwrap_or_else(|error| panic!("analysed: {error:?}\n{text}"));
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

fn unit_facts(headers: &str, body: &str) -> MemberFacts {
    let text = format!("{headers}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

fn free_run(registrar: &str, contracts: &ResourceContracts) -> Arc<RunMemberFacts> {
    let headers = format!("{HEADERS}{FREE_HEADERS}");
    Arc::new(RunMemberFacts::new(
        vec![
            unit_facts(&headers, registrar),
            unit_facts(&headers, FREES),
            declared(contracts),
        ],
        BTreeMap::new(),
    ))
}

fn free_row(
    name: &str,
    run: Option<Arc<RunMemberFacts>>,
    contracts: &ResourceContracts,
) -> (AnalysisStatus, Vec<String>) {
    unit_row(
        &format!("{HEADERS}{FREE_HEADERS}"),
        FREES,
        name,
        run,
        contracts,
    )
}

/// The declarations: each free keeps its NULL-accepting drop and states
/// exactly its slots, every actual `_`.
#[test]
fn the_frees_state_their_slots_beside_the_drop() {
    use qcore_schemas::reference_count::ReferenceOperation;
    let contracts = free_crate();
    let bio = [
        "bio_st.bio_callback_ex",
        "bio_st.bio_callback",
        "bio_method_st.meth_destroy",
    ];
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: and the teardown
    // slots; SSL_free also the message and info callbacks (the record
    // layer's clear on its error path) and the digest, cipher, public key
    // method and async wait context cleanups.
    let mut ssl = vec![
        "ssl_ctx_st.remove_session_cb",
        "ssl_ctx_st.msg_callback",
        "ssl_ctx_st.info_callback",
        "bio_st.bio_callback_ex",
        "bio_st.bio_callback",
        "bio_method_st.meth_destroy",
        LOOKUP_FREED[0],
        LOOKUP_FREED[1],
        "evp_md_st.md_meth_cleanup",
        "evp_cipher_st.cipher_meth_cleanup",
        "evp_pkey_method_st.pkey_meth_cleanup",
        "async_wait_ctx_st.wait_fd_cleanup",
    ];
    ssl.extend(KEY_FREED);
    let mut ctx = vec![
        "ssl_ctx_st.remove_session_cb",
        LOOKUP_FREED[0],
        LOOKUP_FREED[1],
    ];
    ctx.extend(KEY_FREED);
    for (call, keys) in [
        ("BIO_free", &bio[..]),
        ("SSL_free", &ssl[..]),
        ("SSL_CTX_free", &ctx[..]),
    ] {
        let declaration = contracts
            .retaining_contract(call)
            .unwrap_or_else(|| panic!("{call} is withheld"));
        assert!(contracts.contract(call).is_none(), "{call}");
        assert!(
            matches!(declaration.references.as_slice(),
                [reference] if reference.operation == ReferenceOperation::Dropped
                    && reference.parameter == 0 && reference.nullable),
            "{call}"
        );
        let invoked = declaration
            .callbacks
            .iter()
            .map(|clause| match clause {
                CallbackClause::MemberInvoked {
                    slot: true,
                    member,
                    actuals,
                    ..
                } => {
                    // Card CALLBACK-SUPPLIED-NULL-ACTUAL: every actual the
                    // callee's, `supplied_nullable` where BIO_set_callback
                    // (3.5) passes NULL ("BIO_free(b) callback_ex(b,
                    // BIO_CB_FREE, NULL, …, NULL)").
                    assert_eq!(
                        actuals,
                        &free_actuals(&member.spelling(), actuals.len()),
                        "{call}: {actuals:?}"
                    );
                    member.spelling()
                }
                other => panic!("{call}: {other:?}"),
            })
            .collect::<Vec<_>>();
        let (mut invoked, mut keys) = (invoked, keys.to_vec());
        invoked.sort();
        keys.sort();
        assert_eq!(invoked, keys, "{call}");
    }
}

/// C04.19 amended: beside reference clauses, only slot invocations, and only
/// beside drops. A keep, a plain member invocation, a synchronous invocation
/// and a registration beside a reference clause still reject.
#[test]
fn a_slot_invocation_stands_beside_drops_only() {
    let module = |clauses: &str| {
        format!(
            "module contract;

extern fn obj_put(o: ptr<mut, u8>, cb: fnptr) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_borrowed(cb);
{clauses}    ensures normal resource_return_none();
}};
"
        )
    };
    let load = |text: String| {
        load_resource_contract_sources(&BTreeMap::from([("obj.qv".to_owned(), text)]))
    };
    let admitted = load(module(
        "    ensures normal resource_reference_dropped(o, nullable);\n    ensures normal \
         resource_callback_slot_invoked(o, obj_st.on_free, o);\n",
    ))
    .expect("a slot invocation beside a drop loads");
    assert!(admitted.contract("obj_put").is_none());
    let declaration = admitted.retaining_contract("obj_put").expect("withheld");
    assert_eq!(declaration.references.len(), 1);
    assert_eq!(declaration.callbacks.len(), 1);
    for clauses in [
        // A slot invocation beside a keep.
        "    ensures normal resource_reference_kept(o);\n    ensures normal \
         resource_callback_slot_invoked(o, obj_st.on_free, o);\n",
        // A plain member invocation beside a drop.
        "    ensures normal resource_reference_dropped(o, nullable);\n    ensures normal \
         resource_callback_member_invoked(o, obj_st.on_free, o);\n",
        // A synchronous invocation beside a drop.
        "    ensures normal resource_reference_dropped(o, nullable);\n    ensures normal \
         resource_callback_invoked(cb, o);\n",
        // A slot registration beside a drop.
        "    ensures normal resource_reference_dropped(o, nullable);\n    ensures normal \
         resource_callback_slot_registered(o, obj_st.on_free, cb);\n",
        // A slot invocation beside a drop and a registration.
        "    ensures normal resource_reference_dropped(o, nullable);\n    ensures normal \
         resource_callback_slot_invoked(o, obj_st.on_free, o);\n    ensures normal \
         resource_callback_slot_registered(o, obj_st.on_other, cb);\n",
    ] {
        let refused = load(module(clauses)).unwrap_err().to_string();
        assert!(
            refused
                .contains("a callback clause stands only beside retentions and plain predicates"),
            "{clauses}: {refused}"
        );
    }
}

/// Identity candidates registered (or nothing): each free decides `Safe`.
#[test]
fn identity_candidates_decide_the_frees() {
    if !clang_available() {
        return;
    }
    let contracts = free_crate();
    for registrar in [
        "void reg(SSL_CTX *c, BIO *b) { SSL_CTX_sess_set_remove_cb(c, remove_ok); \
         BIO_set_callback_ex(b, bio_cb_ok); }\n",
        // Card OPENSSL-FREE-INDIRECT-CALLBACKS: identity lookup functions.
        "void reg(X509_LOOKUP_METHOD *m) { X509_LOOKUP_meth_set_free(m, lookup_free_ok); \
         X509_LOOKUP_meth_set_shutdown(m, lookup_shutdown_ok); }\n",
        "void nothing(void) { }\n",
    ] {
        let run = free_run(registrar, &contracts);
        for name in ["free_ctx", "free_ssl", "free_bio", "free_store"] {
            let (status, leaves) = free_row(name, Some(run.clone()), &contracts);
            assert!(
                !refused(&leaves, "callback-"),
                "{name}: no callback refusal: {leaves:?}"
            );
            assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
        }
    }
}

/// An uncontracted session-remove candidate refuses both frees that run it;
/// a destroy function releasing the BIO it is handed, and an uncontracted
/// BIO callback, refuse `BIO_free` and `SSL_free`. Each names the candidate,
/// and the free that does not run the slot is unaffected.
#[test]
fn a_candidate_that_is_not_the_identity_refuses_the_frees() {
    if !clang_available() {
        return;
    }
    let contracts = free_crate();
    for (registrar, candidate, runs, spared) in [
        (
            "void reg(SSL_CTX *c) { SSL_CTX_sess_set_remove_cb(c, remove_uncontracted); }\n",
            "remove_uncontracted",
            &["free_ctx", "free_ssl"][..],
            "free_bio",
        ),
        (
            "void reg(BIO_METHOD *m) { BIO_meth_set_destroy(m, bio_destroy_releasing); }\n",
            "bio_destroy_releasing",
            &["free_bio", "free_ssl"][..],
            "free_ctx",
        ),
        (
            "void reg(BIO *b) { BIO_set_callback_ex(b, bio_cb_uncontracted); }\n",
            "bio_cb_uncontracted",
            &["free_bio", "free_ssl"][..],
            "free_ctx",
        ),
        // Card OPENSSL-FREE-INDIRECT-CALLBACKS: an uncontracted lookup free
        // function, and a shutdown function releasing the lookup it is handed,
        // refuse every free of a store; `BIO_free` frees none.
        (
            "void reg(X509_LOOKUP_METHOD *m) { X509_LOOKUP_meth_set_free(m, \
             lookup_free_uncontracted); }\n",
            "lookup_free_uncontracted",
            &["free_store", "free_ctx", "free_ssl"][..],
            "free_bio",
        ),
        (
            "void reg(X509_LOOKUP_METHOD *m) { X509_LOOKUP_meth_set_shutdown(m, \
             lookup_shutdown_releasing); }\n",
            "lookup_shutdown_releasing",
            &["free_store", "free_ctx", "free_ssl"][..],
            "free_bio",
        ),
        // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: an uncontracted RSA
        // method finish function or engine destroy function refuses every
        // free that frees keys in turn; `BIO_free` frees none.
        (
            "void reg(RSA_METHOD *m) { RSA_meth_set_finish(m, rsa_finish_uncontracted); }\n",
            "rsa_finish_uncontracted",
            &["free_store", "free_ctx", "free_ssl"][..],
            "free_bio",
        ),
        (
            "void reg(ENGINE *e) { ENGINE_set_destroy_function(e, \
             engine_destroy_uncontracted); }\n",
            "engine_destroy_uncontracted",
            &["free_store", "free_ctx", "free_ssl"][..],
            "free_bio",
        ),
    ] {
        let run = free_run(registrar, &contracts);
        for name in runs {
            let (status, leaves) = free_row(name, Some(run.clone()), &contracts);
            assert_ne!(status, AnalysisStatus::Safe, "{name}");
            assert!(
                leaves
                    .iter()
                    .any(|leaf| leaf.contains(candidate) && leaf.contains("callback-")),
                "{name}: {leaves:?}"
            );
        }
        let (status, leaves) = free_row(spared, Some(run), &contracts);
        assert_eq!(status, AnalysisStatus::Safe, "{spared}: {leaves:?}");
    }
}

/// No run facts, or a key no loaded declaration registers (`bio_slots.qv` or
/// `ssl_ctx.qv` absent): the free refuses, never `Safe`.
#[test]
fn a_free_slot_without_registrations_or_run_facts_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = free_crate();
    for name in ["free_ctx", "free_ssl", "free_bio", "free_store"] {
        let (status, _) = free_row(name, None, &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name} without run facts");
    }
    let without_bio_slots = free_contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("bio.qv", BIO),
        ("lookup.qv", LOOKUP),
        ("x509_store.qv", X509_STORE),
        ("x509_lookup_slots.qv", X509_LOOKUP_SLOTS),
    ]);
    let run = free_run("void nothing(void) { }\n", &without_bio_slots);
    for name in ["free_bio", "free_ssl"] {
        let (status, leaves) = free_row(name, Some(run.clone()), &without_bio_slots);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "no declaration registers"),
            "{name}: {leaves:?}"
        );
    }
    let without_setters = free_contracts(&[
        ("ssl.qv", SSL),
        ("bio.qv", BIO),
        ("lookup.qv", LOOKUP),
        ("bio_slots.qv", BIO_SLOTS),
        ("x509_store.qv", X509_STORE),
        ("x509_lookup_slots.qv", X509_LOOKUP_SLOTS),
    ]);
    let run = free_run("void nothing(void) { }\n", &without_setters);
    let (status, leaves) = free_row("free_ctx", Some(run), &without_setters);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(refused(&leaves, "no declaration registers"), "{leaves:?}");
    // Card OPENSSL-FREE-INDIRECT-CALLBACKS: the lookup method setters absent.
    let without_lookup_setters = free_contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("bio.qv", BIO),
        ("lookup.qv", LOOKUP),
        ("bio_slots.qv", BIO_SLOTS),
        ("x509_store.qv", X509_STORE),
    ]);
    let run = free_run("void nothing(void) { }\n", &without_lookup_setters);
    for name in ["free_store", "free_ctx", "free_ssl"] {
        let (status, leaves) = free_row(name, Some(run.clone()), &without_lookup_setters);
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "no declaration registers"),
            "{name}: {leaves:?}"
        );
    }
    let (status, leaves) = free_row("free_bio", Some(run), &without_lookup_setters);
    assert_eq!(status, AnalysisStatus::Safe, "free_bio: {leaves:?}");
}

/// Card OPENSSL-FREE-INDIRECT-CALLBACKS: `X509_STORE_free` keeps its drop
/// (not NULL-accepting: the 1.1.1 page does not say NULL does nothing) and
/// states exactly the lookup method's shutdown and free slots, the actual the
/// library's `X509_LOOKUP` (`_`); no store slot (`x509_store_st.*`) is stated
/// at any free, and each lookup setter registers its own slot.
#[test]
fn the_store_free_states_the_lookup_slots() {
    use qcore_schemas::reference_count::ReferenceOperation;
    let contracts = free_crate();
    let declaration = contracts
        .retaining_contract("X509_STORE_free")
        .expect("X509_STORE_free is withheld");
    assert!(contracts.contract("X509_STORE_free").is_none());
    assert!(matches!(declaration.references.as_slice(),
        [reference] if reference.operation == ReferenceOperation::Dropped
            && reference.parameter == 0 && !reference.nullable));
    let invoked = declaration
        .callbacks
        .iter()
        .map(|clause| match clause {
            CallbackClause::MemberInvoked {
                slot: true,
                member,
                actuals,
                ..
            } => {
                assert_eq!(actuals, &free_actuals(&member.spelling(), actuals.len()));
                member.spelling()
            }
            other => panic!("{other:?}"),
        })
        .collect::<Vec<_>>();
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: and the teardown slots.
    let mut expected = LOOKUP_FREED.to_vec();
    expected.extend(KEY_FREED);
    assert_eq!(invoked, expected);
    for call in ["X509_STORE_free", "SSL_CTX_free", "SSL_free"] {
        let declaration = contracts.retaining_contract(call).expect("withheld");
        assert!(
            declaration.callbacks.iter().all(|clause| !matches!(clause,
                CallbackClause::MemberInvoked { member, .. }
                    if member.spelling().starts_with("x509_store_st."))),
            "{call}"
        );
    }
    let mut registered = Vec::new();
    for (name, declaration) in contracts.retaining() {
        for clause in &declaration.callbacks {
            if let CallbackClause::MemberRegistered { member, .. } = clause
                && member.spelling().starts_with("x509_lookup_method_st.")
            {
                registered.push((name.clone(), member.spelling()));
            }
        }
    }
    registered.sort();
    assert_eq!(registered.len(), 9, "{registered:?}");
    for (name, key) in &registered {
        let field = key
            .strip_prefix("x509_lookup_method_st.lookup_meth_")
            .unwrap();
        assert_eq!(name, &format!("X509_LOOKUP_meth_set_{field}"));
    }
}

const OBJ: &str = "module contract;

extern fn obj_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
    ensures normal resource_return_kind(reference);
};

extern fn obj_keep(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_reference_kept(o);
    ensures normal resource_return_none();
};

extern fn obj_set_on_free(o: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_slot_registered(o, obj_st.on_free, cb);
    ensures normal resource_return_none();
};

extern fn obj_put(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_reference_dropped(o, nullable);
    ensures normal resource_callback_slot_invoked(o, obj_st.on_free, o);
    ensures normal resource_return_none();
};

extern fn on_free_ok(o: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_return_none();
};

extern fn on_free_live(o: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_owned(o, nonnull);
    ensures normal resource_return_none();
};

extern fn on_free_payload(o: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_return_none();
    quant live #o: obj.payload |-> #released;
};

extern fn on_free_releasing(o: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nonnull);
    ensures normal resource_return_none();
};
";

const OBJ_HEADERS: &str = "\
struct obj { int x; struct obj *payload; };
struct obj *obj_new(void);
void obj_keep(struct obj *o);
void obj_set_on_free(struct obj *o, int (*cb)(struct obj *));
void obj_put(struct obj *o);
int on_free_ok(struct obj *o);
int on_free_live(struct obj *o);
int on_free_payload(struct obj *o);
int on_free_releasing(struct obj *o);
";

/// `once` drops its only reference; `twice` keeps a second one first, so its
/// first drop is not the last.
const OBJ_BODY: &str = "\
void once(void) { struct obj *o = obj_new(); if (!o) return; obj_put(o); }
void twice(void) { struct obj *o = obj_new(); if (!o) return; obj_keep(o); obj_put(o); obj_put(o); }
";

/// The order, and a drop that may not be the last, over a synthetic object
/// whose drop invokes `obj_st.on_free` handing it the dropped formal:
///
/// * an identity candidate decides, whether the drop is the last or not;
/// * a candidate requiring the object live (`resource_owned`) puts that
///   requirement on the call's own position, read before the drop; on a
///   reference the drop then reads as undecided, so the call refuses
///   (`reference-drop-unheld`), never `Safe`;
/// * a candidate stating a member-path state of the object it is handed
///   (rule 30) refuses (`callback-beside-drop`): the call does not decide
///   whether the slot runs, so it never applies the state;
/// * a candidate releasing the object it is handed refuses.
#[test]
fn a_candidate_effect_on_the_dropped_object_refuses() {
    if !clang_available() {
        return;
    }
    let contracts =
        load_resource_contract_sources(&BTreeMap::from([("obj.qv".to_owned(), OBJ.to_owned())]))
            .expect("the synthetic module loads");
    let run = |candidate: &str| {
        Arc::new(RunMemberFacts::new(
            vec![
                unit_facts(
                    OBJ_HEADERS,
                    &format!("void reg(struct obj *o) {{ obj_set_on_free(o, {candidate}); }}\n"),
                ),
                unit_facts(OBJ_HEADERS, OBJ_BODY),
                declared(&contracts),
            ],
            BTreeMap::new(),
        ))
    };
    for name in ["once", "twice"] {
        let (status, leaves) = unit_row(
            OBJ_HEADERS,
            OBJ_BODY,
            name,
            Some(run("on_free_ok")),
            &contracts,
        );
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
        let (status, leaves) = unit_row(
            OBJ_HEADERS,
            OBJ_BODY,
            name,
            Some(run("on_free_live")),
            &contracts,
        );
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "reference-drop-unheld"),
            "{name}: {leaves:?}"
        );
        let (status, leaves) = unit_row(
            OBJ_HEADERS,
            OBJ_BODY,
            name,
            Some(run("on_free_payload")),
            &contracts,
        );
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            refused(&leaves, "callback-beside-drop"),
            "{name}: {leaves:?}"
        );
        let (status, leaves) = unit_row(
            OBJ_HEADERS,
            OBJ_BODY,
            name,
            Some(run("on_free_releasing")),
            &contracts,
        );
        assert_ne!(status, AnalysisStatus::Safe, "{name}");
        assert!(
            leaves
                .iter()
                .any(|leaf| leaf.contains("on_free_releasing") && leaf.contains("callback-")),
            "{name}: {leaves:?}"
        );
    }
}

// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS (gateway rule 12n; contract
// specification C04.19.3, C04.18; this target, so the read census needs no
// new unit): the session ticket, session id, DH parameter, cookie, session
// secret, SRP, async and QUIC pending connection slots
// (`ssl_handshake_setters.qv`) and the twelve slots of a certificate store
// (`x509_store_slots.qv`).

/// The 26 keys every handshake and I/O call invokes since this card, each
/// with its actuals on `ssl` (position 0): `Some(0)` the connection, `None`
/// `_`, and the session secret's `peer_ciphers` `supplied_nullable`.
const REMAINING: [(&str, &[Option<usize>]); 26] = [
    ("ssl_ctx_st.ticket_gen_cb", &[Some(0), None]),
    (
        "ssl_ctx_st.ticket_dec_cb",
        &[Some(0), None, None, None, None, None],
    ),
    ("ssl_ctx_st.generate_session_id", &[Some(0), None, None]),
    ("ssl_ctx_st.tmp_dh_callback", &[Some(0), None, None]),
    ("ssl_ctx_st.gen_stateless_cookie_cb", &[Some(0), None, None]),
    (
        "ssl_ctx_st.verify_stateless_cookie_cb",
        &[Some(0), None, None],
    ),
    ("ssl_ctx_st.app_gen_cookie_cb", &[Some(0), None, None]),
    ("ssl_ctx_st.app_verify_cookie_cb", &[Some(0), None, None]),
    (
        "ssl_st.session_secret_cb",
        &[Some(0), None, None, None, None, None],
    ),
    ("ssl_ctx_st.srp_username_callback", &[Some(0), None, None]),
    ("ssl_ctx_st.srp_client_pwd_callback", &[Some(0), None]),
    ("ssl_ctx_st.srp_verify_param_callback", &[Some(0), None]),
    ("ssl_ctx_st.async_cb", &[Some(0), None]),
    ("ssl_ctx_st.new_pending_conn_cb", &[None, None, None]),
    ("x509_store_st.verify_cb", &[None, None]),
    ("x509_store_st.verify", &[None]),
    ("x509_store_st.get_issuer", &[None, None, None]),
    ("x509_store_st.check_issued", &[None, None, None]),
    ("x509_store_st.check_revocation", &[None]),
    ("x509_store_st.get_crl", &[None, None, None]),
    ("x509_store_st.check_crl", &[None, None]),
    ("x509_store_st.cert_crl", &[None, None, None]),
    ("x509_store_st.check_policy", &[None]),
    ("x509_store_st.lookup_certs", &[None, None]),
    ("x509_store_st.lookup_crls", &[None, None]),
    ("x509_store_st.cleanup_fn", &[None]),
];

fn remaining_actuals(key: &str, actuals: &[Option<usize>]) -> Vec<CallbackActual> {
    actuals
        .iter()
        .enumerate()
        .map(|(at, actual)| match actual {
            Some(index) => CallbackActual::Parameter { index: *index },
            None if key == "ssl_st.session_secret_cb" && at == 3 => {
                CallbackActual::SuppliedNullable
            }
            None => CallbackActual::Supplied,
        })
        .collect()
}

/// The 28 setters load as their registrations (two for the session ticket
/// setter); every handshake and I/O call invokes the 26 keys with the
/// actuals above, and `SSL_shutdown` the async slot.
#[test]
fn the_crate_states_every_remaining_slot() {
    let contracts = crate_contracts();
    for (setter, keys) in [
        (
            "SSL_CTX_set_session_ticket_cb",
            &["ssl_ctx_st.ticket_gen_cb", "ssl_ctx_st.ticket_dec_cb"][..],
        ),
        (
            "SSL_CTX_set_generate_session_id",
            &["ssl_ctx_st.generate_session_id"],
        ),
        (
            "SSL_set_generate_session_id",
            &["ssl_ctx_st.generate_session_id"],
        ),
        (
            "SSL_CTX_set_tmp_dh_callback",
            &["ssl_ctx_st.tmp_dh_callback"],
        ),
        ("SSL_set_tmp_dh_callback", &["ssl_ctx_st.tmp_dh_callback"]),
        (
            "SSL_CTX_set_stateless_cookie_generate_cb",
            &["ssl_ctx_st.gen_stateless_cookie_cb"],
        ),
        (
            "SSL_CTX_set_stateless_cookie_verify_cb",
            &["ssl_ctx_st.verify_stateless_cookie_cb"],
        ),
        (
            "SSL_CTX_set_cookie_generate_cb",
            &["ssl_ctx_st.app_gen_cookie_cb"],
        ),
        (
            "SSL_CTX_set_cookie_verify_cb",
            &["ssl_ctx_st.app_verify_cookie_cb"],
        ),
        ("SSL_set_session_secret_cb", &["ssl_st.session_secret_cb"]),
        (
            "SSL_CTX_set_srp_username_callback",
            &["ssl_ctx_st.srp_username_callback"],
        ),
        (
            "SSL_CTX_set_srp_client_pwd_callback",
            &["ssl_ctx_st.srp_client_pwd_callback"],
        ),
        (
            "SSL_CTX_set_srp_verify_param_callback",
            &["ssl_ctx_st.srp_verify_param_callback"],
        ),
        ("SSL_CTX_set_async_callback", &["ssl_ctx_st.async_cb"]),
        ("SSL_set_async_callback", &["ssl_ctx_st.async_cb"]),
        (
            "SSL_CTX_set_new_pending_conn_cb",
            &["ssl_ctx_st.new_pending_conn_cb"],
        ),
        ("X509_STORE_set_verify_cb", &["x509_store_st.verify_cb"]),
        ("X509_STORE_set_verify", &["x509_store_st.verify"]),
        ("X509_STORE_set_get_issuer", &["x509_store_st.get_issuer"]),
        (
            "X509_STORE_set_check_issued",
            &["x509_store_st.check_issued"],
        ),
        (
            "X509_STORE_set_check_revocation",
            &["x509_store_st.check_revocation"],
        ),
        ("X509_STORE_set_get_crl", &["x509_store_st.get_crl"]),
        ("X509_STORE_set_check_crl", &["x509_store_st.check_crl"]),
        ("X509_STORE_set_cert_crl", &["x509_store_st.cert_crl"]),
        (
            "X509_STORE_set_check_policy",
            &["x509_store_st.check_policy"],
        ),
        (
            "X509_STORE_set_lookup_certs",
            &["x509_store_st.lookup_certs"],
        ),
        ("X509_STORE_set_lookup_crls", &["x509_store_st.lookup_crls"]),
        ("X509_STORE_set_cleanup", &["x509_store_st.cleanup_fn"]),
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
    let invoked = |call: &str| {
        contracts
            .retaining_contract(call)
            .expect("withheld")
            .callbacks
            .iter()
            .filter_map(|clause| match clause {
                CallbackClause::MemberInvoked {
                    slot: true,
                    member,
                    actuals,
                    ..
                } => Some((member.spelling(), actuals.clone())),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for call in ["SSL_write", "SSL_read", "SSL_connect", "SSL_do_handshake"] {
        let stated = invoked(call);
        for (key, actuals) in REMAINING {
            let found = stated
                .iter()
                .filter(|(member, _)| member == key)
                .collect::<Vec<_>>();
            assert_eq!(found.len(), 1, "{call} {key}");
            assert_eq!(found[0].1, remaining_actuals(key, actuals), "{call} {key}");
        }
    }
    let shutdown = invoked("SSL_shutdown");
    assert!(
        shutdown.contains(&(
            "ssl_ctx_st.async_cb".to_owned(),
            remaining_actuals("ssl_ctx_st.async_cb", &[Some(0), None]),
        )),
        "{shutdown:?}"
    );
    assert!(
        !shutdown
            .iter()
            .any(|(member, _)| member.starts_with("x509_store_st.")),
        "SSL_shutdown verifies no chain"
    );
}

/// Identity candidates (and a NULL `arg`) in the store, async, session
/// ticket and session secret slots: the I/O call and `SSL_shutdown` decide.
#[test]
fn identity_remaining_candidates_decide_the_io_call() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let registrar = "\
void reg(SSL_CTX *c, SSL *s, X509_STORE *xs) {
    X509_STORE_set_verify_cb(xs, verify_store_ok);
    SSL_CTX_set_async_callback(c, async_ok);
    SSL_CTX_set_session_ticket_cb(c, ticket_gen_ok, ticket_dec_ok, 0);
    SSL_set_session_secret_cb(s, secret_ok, 0);
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

/// A candidate with no contract in force refuses: the store's verify
/// callback the I/O call, the async callback `SSL_shutdown` too.
#[test]
fn a_remaining_slot_with_an_uncontracted_candidate_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let store =
        "void reg(X509_STORE *xs) { X509_STORE_set_verify_cb(xs, verify_store_uncontracted); }\n";
    for (registrar, candidate, rows) in [
        (
            store,
            "`verify_store_uncontracted` (from `x509_store_st.verify_cb`)",
            &["write_some"][..],
        ),
        (
            "void reg(SSL_CTX *c) { SSL_CTX_set_async_callback(c, async_uncontracted); }\n",
            "`async_uncontracted` (from `ssl_ctx_st.async_cb`)",
            &["write_some", "shut"],
        ),
    ] {
        let run = run_of(registrar, &contracts);
        for name in rows {
            let (status, leaves) = row_of(name, run.clone(), &contracts);
            assert_ne!(status, AnalysisStatus::Safe, "{name}: {registrar}");
            assert!(
                leaves
                    .iter()
                    .any(|leaf| leaf.contains(candidate) && leaf.contains("callback-uncontracted")),
                "{name}: {leaves:?}"
            );
        }
    }
    // SSL_shutdown verifies no chain: an uncontracted store callback leaves
    // it decided.
    let run = run_of(store, &contracts);
    let (status, leaves) = row_of("shut", run, &contracts);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// The session ticket setter registers both its callbacks: a decryption
/// callback releasing the session the library hands it refuses the I/O call
/// as effectful, beside an identity generation callback.
#[test]
fn both_registrations_of_the_session_ticket_setter_are_read() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let run = run_of(
        "void reg(SSL_CTX *c) { SSL_CTX_set_session_ticket_cb(c, ticket_gen_ok, ticket_dec_releasing, 0); }\n",
        &contracts,
    );
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves.iter().any(
            |leaf| leaf.contains("ticket_dec_releasing") && leaf.contains("callback-effectful")
        ),
        "{leaves:?}"
    );
}

/// The session secret callback's `peer_ciphers` is NULL on a client
/// (`supplied_nullable`): a candidate requiring a live stack there refuses.
#[test]
fn a_session_secret_candidate_requiring_the_peer_ciphers_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = crate_contracts();
    let run = run_of(
        "void reg(SSL *s) { SSL_set_session_secret_cb(s, secret_needs_ciphers, 0); }\n",
        &contracts,
    );
    let (status, leaves) = row_of("write_some", run, &contracts);
    assert_ne!(status, AnalysisStatus::Safe);
    assert!(
        leaves.iter().any(|leaf| {
            leaf.contains("`secret_needs_ciphers` (from `ssl_st.session_secret_cb`)")
                && leaf.contains("callback-supplied-null")
        }),
        "{leaves:?}"
    );
}

/// Without this card's modules no loaded declaration registers the new keys:
/// they are never decided, even with nothing registered.
#[test]
fn a_remaining_slot_no_declaration_registers_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = contracts(&[
        ("ssl.qv", SSL),
        ("ssl_ctx.qv", SSL_CTX),
        ("ssl_io.qv", SSL_IO),
        ("ssl_handshake.qv", SSL_HANDSHAKE),
        ("bio_slots.qv", BIO_SLOTS),
    ]);
    let run = run_of("void nothing(void) { }\n", &contracts);
    for (key, _) in REMAINING {
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
