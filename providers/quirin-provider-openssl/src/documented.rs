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

//! What the official OpenSSL manual pages document for every contracted
//! function (cards **OPENSSL-PROVIDER-PLUGIN** and
//! **OPENSSL-PROVIDER-REMAINING-CALLEES**; user decision 2026-11-16: the
//! version range comes from the pages' HISTORY sections, `documented` when
//! HISTORY names no version).
//!
//! Each entry names the page and documentation series establishing the
//! range's lower bound (its snapshot is in `doc-snapshots.json`), the range
//! clause that page supports, the custody **shape** the page documents and
//! the premises bounding where the declaration is believed. The checker
//! derives the exact clause set of the declaration from an entry
//! ([`crate::check::expected_clauses`]) and refuses any other.

/// A position's nullability, as the page documents it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Null {
    /// NULL is admitted (a NULL release does nothing; a result may be NULL).
    Nullable,
    /// NULL is not admitted (or the page does not say it is).
    Nonnull,
}

impl Null {
    /// The vocabulary's word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Null::Nullable => "nullable",
            Null::Nonnull => "nonnull",
        }
    }
}

/// The custody shape the official documentation gives a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Documented {
    /// Takes and returns no custody: every parameter borrowed, no result
    /// custody (a static table, a scalar, or nothing).
    Plain,
    /// Releases one reference on a reference-counted object (the parameter
    /// at this position): a reference drop, never a final free.
    CountedRelease(usize),
    /// Creates (or looks up) a reference-counted object and returns one
    /// reference, or NULL on failure.
    CountedConstructor,
    /// Allocates a fresh object that is not reference counted and returns
    /// it, or NULL on failure.
    Constructor,
    /// Frees the object at this position (not reference counted).
    Release(usize, Null),
    /// Returns a pointer the object at this position owns (an internal
    /// pointer, a member, an element): the caller owes nothing and may not
    /// free it.
    BorrowedResult(usize, Null),
    /// The object at `holder` keeps the pointer at `item` (C04.14); the
    /// caller keeps custody of `item`'s object.
    Retains { item: usize, holder: usize },
}

/// A premise bounding where a declaration is believed, or a callback it
/// states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Premise {
    /// `requires resource_literal_argument(P, 0)`: the declaration applies
    /// only at calls passing a null pointer constant at this position
    /// (C04.5).
    NullArgument(usize),
    /// `requires resource_premise(openssl_registered_callbacks, "…")`: the
    /// call may run callbacks registered on the SSL object or its SSL_CTX;
    /// the declaration reads them as changing no custody the caller holds
    /// (C04.18, a named premise published with every row resting on it).
    RegisteredCallbacks,
    /// `ensures normal resource_callback_invoked(F, A…)` (C04.19): the
    /// callback at `callback` is called during the call only, with these
    /// arguments (`Some(i)`: parameter `i`; `None`: a value the callee
    /// supplies, `_`).
    CallbackInvoked {
        callback: usize,
        arguments: &'static [Option<usize>],
    },
}

/// The name of the registered-callbacks premise.
pub const REGISTERED_CALLBACKS: &str = "openssl_registered_callbacks";

/// Its statement (no `;`, `"` or `//`: the checker reads clauses lexically).
pub const REGISTERED_CALLBACKS_STATEMENT: &str = "a callback the SSL object or its SSL_CTX \
carries (verify, certificate, password, info, message or ex_data callbacks) and the call may \
run takes, releases or retains nothing the caller holds";

/// What the documentation supports for one function.
#[derive(Clone, Copy, Debug)]
pub struct DocumentedFunction {
    pub function: &'static str,
    /// The manual page (`man3/<page>`) at `series` establishing the range.
    pub page: &'static str,
    pub series: &'static str,
    /// The range clause, as the declaration must spell it.
    pub range: &'static str,
    pub shape: Documented,
    pub premises: &'static [Premise],
}

const fn entry(
    function: &'static str,
    page: &'static str,
    series: &'static str,
    range: &'static str,
    shape: Documented,
) -> DocumentedFunction {
    DocumentedFunction {
        function,
        page,
        series,
        range,
        shape,
        premises: &[],
    }
}

const fn with(
    function: &'static str,
    page: &'static str,
    series: &'static str,
    range: &'static str,
    shape: Documented,
    premises: &'static [Premise],
) -> DocumentedFunction {
    DocumentedFunction {
        function,
        page,
        series,
        range,
        shape,
        premises,
    }
}

const DOC_1_0_2: &str = "resource_library_version(openssl, documented \"1.0.2\")";
const DOC_1_1_1: &str = "resource_library_version(openssl, documented \"1.1.1\")";
const DOC_3_0: &str = "resource_library_version(openssl, documented \"3.0\")";
const SINCE_1_1_0: &str = "resource_library_version(openssl, since \"1.1.0\")";
const SINCE_3_0_0: &str = "resource_library_version(openssl, since \"3.0.0\")";

/// The registered-callbacks premise alone.
const CALLBACKS: &[Premise] = &[Premise::RegisteredCallbacks];

/// `PEM_read_bio_*(bp, x, cb, u)`: a NULL `x` (no object to reuse) and a
/// NULL `cb` (OpenSSL's own passphrase routine, or `u` read as the
/// passphrase): the vocabulary admits no callback clause beside the `x`
/// premise, so a caller's callback stays outside the declaration.
const PEM_READ: &[Premise] = &[Premise::NullArgument(1), Premise::NullArgument(2)];

use Documented::{
    BorrowedResult, Constructor, CountedConstructor, CountedRelease, Plain, Release, Retains,
};
use Null::Nullable;

/// The documented semantics of every contracted function, sorted by name.
pub const DOCUMENTED: &[DocumentedFunction] = &[
    entry("BIO_free", "BIO_new", "1.0.2", DOC_1_0_2, CountedRelease(0)),
    entry(
        "BIO_new_file",
        "BIO_s_file",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
    ),
    entry(
        "ENGINE_by_id",
        "ENGINE_add",
        "1.1.1",
        DOC_1_1_1,
        CountedConstructor,
    ),
    entry(
        "ENGINE_free",
        "ENGINE_add",
        "1.1.1",
        DOC_1_1_1,
        CountedRelease(0),
    ),
    entry(
        "ERR_clear_error",
        "ERR_clear_error",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry(
        "ERR_error_string_n",
        "ERR_error_string",
        "1.0.2",
        "resource_library_version(openssl, since \"0.9.6\")",
        Plain,
    ),
    entry("ERR_peek_error", "ERR_get_error", "1.0.2", DOC_1_0_2, Plain),
    entry(
        "ERR_peek_last_error",
        "ERR_get_error",
        "1.0.2",
        "resource_library_version(openssl, since \"0.9.7\")",
        Plain,
    ),
    entry(
        "EVP_EncodeBlock",
        "EVP_EncodeInit",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry(
        "EVP_MD_CTX_free",
        "EVP_DigestInit",
        "3.5",
        SINCE_1_1_0,
        Release(0, Nullable),
    ),
    entry(
        "EVP_MD_CTX_new",
        "EVP_DigestInit",
        "3.5",
        SINCE_1_1_0,
        Constructor,
    ),
    entry(
        "EVP_PKEY_free",
        "EVP_PKEY_new",
        "1.0.2",
        DOC_1_0_2,
        CountedRelease(0),
    ),
    entry("EVP_sha256", "EVP_DigestInit", "1.0.2", DOC_1_0_2, Plain),
    entry("EVP_sha512", "EVP_DigestInit", "1.0.2", DOC_1_0_2, Plain),
    entry(
        "OPENSSL_init_ssl",
        "OPENSSL_init_ssl",
        "3.5",
        SINCE_1_1_0,
        Plain,
    ),
    entry(
        "OPENSSL_sk_free",
        "DEFINE_STACK_OF",
        "3.0",
        DOC_3_0,
        Release(0, Nullable),
    ),
    entry(
        "OPENSSL_sk_new_null",
        "DEFINE_STACK_OF",
        "3.0",
        DOC_3_0,
        Constructor,
    ),
    entry(
        "OPENSSL_sk_push",
        "DEFINE_STACK_OF",
        "3.0",
        DOC_3_0,
        Retains { item: 1, holder: 0 },
    ),
    entry(
        "OPENSSL_sk_value",
        "DEFINE_STACK_OF",
        "3.0",
        DOC_3_0,
        BorrowedResult(0, Nullable),
    ),
    with(
        "PEM_read_bio_PrivateKey",
        "pem",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
        PEM_READ,
    ),
    with(
        "PEM_read_bio_X509",
        "pem",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
        PEM_READ,
    ),
    entry(
        "PKCS12_free",
        "X509_dup",
        "1.1.1",
        DOC_1_1_1,
        Release(0, Nullable),
    ),
    entry(
        "SSL_CTX_free",
        "SSL_CTX_free",
        "1.0.2",
        DOC_1_0_2,
        CountedRelease(0),
    ),
    entry(
        "SSL_CTX_get_cert_store",
        "SSL_CTX_set_cert_store",
        "1.0.2",
        DOC_1_0_2,
        BorrowedResult(0, Nullable),
    ),
    entry(
        "SSL_CTX_load_verify_locations",
        "SSL_CTX_load_verify_locations",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry(
        "SSL_CTX_new",
        "SSL_CTX_new",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
    ),
    entry(
        "SSL_CTX_set_default_passwd_cb_userdata",
        "SSL_CTX_set_default_passwd_cb",
        "1.0.2",
        DOC_1_0_2,
        Retains { item: 1, holder: 0 },
    ),
    entry(
        "SSL_CTX_set_default_verify_paths",
        "SSL_CTX_load_verify_locations",
        "1.1.1",
        DOC_1_1_1,
        Plain,
    ),
    entry(
        "SSL_CTX_set_options",
        "SSL_CTX_set_options",
        "3.5",
        SINCE_3_0_0,
        Plain,
    ),
    entry(
        "SSL_CTX_use_PrivateKey",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    with(
        "SSL_CTX_use_PrivateKey_file",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        CALLBACKS,
    ),
    entry(
        "SSL_CTX_use_certificate",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    with(
        "SSL_CTX_use_certificate_chain_file",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        CALLBACKS,
    ),
    with(
        "SSL_connect",
        "SSL_connect",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        CALLBACKS,
    ),
    with(
        "SSL_do_handshake",
        "SSL_do_handshake",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        CALLBACKS,
    ),
    entry(
        "SSL_free",
        "SSL_free",
        "1.0.2",
        DOC_1_0_2,
        CountedRelease(0),
    ),
    entry(
        "SSL_get1_peer_certificate",
        "SSL_get_peer_certificate",
        "3.5",
        SINCE_3_0_0,
        CountedConstructor,
    ),
    entry(
        "SSL_get_certificate",
        "SSL_get_certificate",
        "3.0",
        DOC_3_0,
        BorrowedResult(0, Nullable),
    ),
    entry(
        "SSL_get_client_CA_list",
        "SSL_get_client_CA_list",
        "1.0.2",
        DOC_1_0_2,
        BorrowedResult(0, Nullable),
    ),
    entry("SSL_get_error", "SSL_get_error", "1.0.2", DOC_1_0_2, Plain),
    entry("SSL_new", "SSL_new", "1.0.2", DOC_1_0_2, CountedConstructor),
    with("SSL_read", "SSL_read", "1.0.2", DOC_1_0_2, Plain, CALLBACKS),
    entry(
        "SSL_set_accept_state",
        "SSL_set_connect_state",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry("SSL_shutdown", "SSL_shutdown", "1.0.2", DOC_1_0_2, Plain),
    with(
        "SSL_write",
        "SSL_write",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        CALLBACKS,
    ),
    entry(
        "TLS_server_method",
        "SSL_CTX_new",
        "3.5",
        SINCE_1_1_0,
        Plain,
    ),
    entry(
        "X509_STORE_CTX_get_current_cert",
        "X509_STORE_CTX_get_error",
        "1.0.2",
        DOC_1_0_2,
        BorrowedResult(0, Nullable),
    ),
    entry(
        "X509_STORE_CTX_get_error",
        "X509_STORE_CTX_get_error",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry(
        "X509_STORE_CTX_get_error_depth",
        "X509_STORE_CTX_get_error",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry(
        "X509_STORE_CTX_set_error",
        "X509_STORE_CTX_get_error",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    entry(
        "X509_STORE_free",
        "X509_STORE_new",
        "1.1.1",
        DOC_1_1_1,
        CountedRelease(0),
    ),
    entry(
        "X509_STORE_get0_objects",
        "X509_STORE_get0_param",
        "3.5",
        SINCE_1_1_0,
        BorrowedResult(0, Nullable),
    ),
    entry(
        "X509_STORE_set_flags",
        "X509_STORE_add_cert",
        "1.1.1",
        DOC_1_1_1,
        Plain,
    ),
    entry(
        "X509_free",
        "X509_new",
        "1.0.2",
        DOC_1_0_2,
        CountedRelease(0),
    ),
    entry(
        "X509_get_issuer_name",
        "X509_get_subject_name",
        "1.1.1",
        DOC_1_1_1,
        BorrowedResult(0, Nullable),
    ),
    entry(
        "X509_new",
        "X509_new",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
    ),
    with(
        "d2i_PKCS12_bio",
        "d2i_X509",
        "1.1.1",
        DOC_1_1_1,
        Constructor,
        &[Premise::NullArgument(1)],
    ),
];
