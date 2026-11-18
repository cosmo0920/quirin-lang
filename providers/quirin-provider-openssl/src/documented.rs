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
    /// Card **REFERENCE-DROP-NULL-ACCEPTING**: a counted release (the
    /// parameter at `position`) whose page states, in `sentence`, that a NULL
    /// argument does nothing: a NULL-accepting reference drop,
    /// `resource_reference_dropped(p, nullable)`. The cited snapshot must
    /// record `sentence`, and the range's series is the oldest one whose page
    /// states it.
    NullAcceptingRelease {
        position: usize,
        sentence: &'static str,
    },
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
    /// Card **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: a setter of a callback slot
    /// of an opaque object (C04.19.3): after the call the object at
    /// `holder` keeps the function pointer at `callback` in `slot` (one of
    /// [`SLOTS`]), `resource_callback_slot_registered(H, slot, F)`; with
    /// `retains`, it also keeps the pointer at that position for the
    /// callback (`resource_retained_by(arg, H)`).
    RegistersSlot {
        holder: usize,
        callback: usize,
        slot: &'static str,
        retains: Option<usize>,
    },
}

/// A premise bounding where a declaration is believed, or a callback it
/// states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Premise {
    /// `requires resource_literal_argument(P, 0)`: the declaration applies
    /// only at calls passing a null pointer constant at this position
    /// (C04.5).
    NullArgument(usize),
    /// `requires resource_premise(openssl_unstated_callback_slots, "…")`: the
    /// call may run callbacks of slots no clause of the declaration states
    /// (their setters are not declared); the declaration reads them as
    /// changing no custody the caller holds (C04.18, a named premise
    /// published with every row resting on it). Card
    /// **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: it replaced
    /// `openssl_registered_callbacks`, which covered every slot, stated or
    /// not.
    UnstatedSlots,
    /// Card **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: `ensures normal
    /// resource_callback_slot_invoked(OBJ, slot, A…)` (C04.19.3): during the
    /// call the function pointer the object at `object` keeps in `slot` (one
    /// of [`SLOTS`]) is called zero or more times with `arguments` (`Some(i)`:
    /// parameter `i`; `None`: a value the callee supplies, `_`).
    SlotInvoked {
        object: usize,
        slot: &'static str,
        arguments: &'static [Option<usize>],
    },
    /// `ensures normal resource_callback_invoked(F, A…)` (C04.19): the
    /// callback at `callback` is called during the call only, with these
    /// arguments (`Some(i)`: parameter `i`; `None`: a value the callee
    /// supplies, `_`).
    CallbackInvoked {
        callback: usize,
        arguments: &'static [Option<usize>],
    },
    /// `ensures normal resource_callback_invoked_or_default(F, A…)` (C04.19,
    /// card OPENSSL-PEM-CALLBACK-ARM): as [`Premise::CallbackInvoked`], and
    /// the page documents what the function does itself at a NULL callback
    /// (its own default), so a NULL callback invokes nothing.
    CallbackInvokedOrDefault {
        callback: usize,
        arguments: &'static [Option<usize>],
    },
}

/// The name of the unstated-slots premise.
pub const UNSTATED_SLOTS: &str = "openssl_unstated_callback_slots";

/// Its statement (no `;`, `"` or `//`: the checker reads clauses lexically).
pub const UNSTATED_SLOTS_STATEMENT: &str = "a callback the call may run that no clause of this \
declaration states (a verify, info, message, security, session or client certificate callback of \
the SSL object or its SSL_CTX, or an ex_data callback) takes, releases or retains nothing the \
caller holds";

/// Card **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: a callback slot of an opaque
/// OpenSSL object, stated as C04.19.3's opaque slot. Its `writers` are every
/// function the manual page `page` (at the pinned series, 3.5) names that
/// sets the slot; the checker requires each to be declared with
/// [`Documented::RegistersSlot`] of this slot, so the run's candidates (what
/// the declared setters are handed) are every value the slot can hold.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub key: &'static str,
    pub page: &'static str,
    pub writers: &'static [&'static str],
}

/// The stated slots.
pub const SLOTS: &[Slot] = &[
    // SSL_CTX_set_cert_cb (3.5) NAME: "SSL_CTX_set_cert_cb, SSL_set_cert_cb".
    // The SSL's copy (SSL_set_cert_cb, and SSL_new's copy of its context's)
    // shares the key: the set is every value either can hold.
    Slot {
        key: "ssl_ctx_st.cert_cb",
        page: "SSL_CTX_set_cert_cb",
        writers: &["SSL_CTX_set_cert_cb", "SSL_set_cert_cb"],
    },
    // SSL_CTX_set_default_passwd_cb (3.5): the context's slot. The page's
    // SSL_set_default_passwd_cb writes the SSL's copy, which no declaration
    // invokes (only the context's loaders are declared).
    Slot {
        key: "ssl_ctx_st.default_passwd_cb",
        page: "SSL_CTX_set_default_passwd_cb",
        writers: &["SSL_CTX_set_default_passwd_cb"],
    },
];

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

/// Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: a handshake or I/O call on `ssl` at
/// position 0: the certificate slot, `cert_cb(ssl, arg)`, and the premise
/// for every slot whose setters are not declared.
const HANDSHAKE: &[Premise] = &[
    Premise::UnstatedSlots,
    Premise::SlotInvoked {
        object: 0,
        slot: "ssl_ctx_st.cert_cb",
        arguments: &[Some(0), None],
    },
];

/// The context's password slot, `pem_password_cb(buf, size, rwflag, u)`:
/// four values the callee supplies.
const PASSWORD: Premise = Premise::SlotInvoked {
    object: 0,
    slot: "ssl_ctx_st.default_passwd_cb",
    arguments: &[None, None, None, None],
};

/// The premise alone: a call that may run callbacks of no stated slot.
const UNSTATED: &[Premise] = &[Premise::UnstatedSlots];

/// `PEM_read_bio_*(bp, x, cb, u)`: a NULL `x` (no object to reuse), and the
/// password callback `cb` called during the read with `u` as its last
/// argument, a NULL `cb` meaning OpenSSL's own default routine (card
/// OPENSSL-PEM-CALLBACK-ARM: a premise stands beside a synchronous
/// invocation, gateway rule 57). The other three arguments (`buf`, `size`,
/// `rwflag`) are values the callee supplies.
const PEM_READ: &[Premise] = &[
    Premise::NullArgument(1),
    Premise::CallbackInvokedOrDefault {
        callback: 2,
        arguments: &[None, None, None, Some(3)],
    },
];

use Documented::{
    BorrowedResult, Constructor, CountedConstructor, CountedRelease, NullAcceptingRelease, Plain,
    RegistersSlot, Release, Retains,
};
use Null::Nullable;

/// The documented semantics of every contracted function, sorted by name.
pub const DOCUMENTED: &[DocumentedFunction] = &[
    entry(
        "BIO_free",
        "BIO_new",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
    ),
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
        "3.0",
        DOC_3_0,
        NullAcceptingRelease {
            position: 0,
            sentence: "If the argument to ENGINE_free() is NULL, nothing is done.",
        },
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
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If key is NULL, nothing is done.",
        },
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
    with(
        "SSL_CTX_free",
        "SSL_CTX_free",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If ctx is NULL nothing is done.",
        },
        UNSTATED,
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
        "SSL_CTX_set_cert_cb",
        "SSL_CTX_set_cert_cb",
        "1.0.2",
        DOC_1_0_2,
        RegistersSlot {
            holder: 0,
            callback: 1,
            slot: "ssl_ctx_st.cert_cb",
            retains: Some(2),
        },
    ),
    entry(
        "SSL_CTX_set_default_passwd_cb",
        "SSL_CTX_set_default_passwd_cb",
        "1.0.2",
        DOC_1_0_2,
        RegistersSlot {
            holder: 0,
            callback: 1,
            slot: "ssl_ctx_st.default_passwd_cb",
            retains: None,
        },
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
        &[PASSWORD],
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
        &[Premise::UnstatedSlots, PASSWORD],
    ),
    with(
        "SSL_connect",
        "SSL_connect",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        HANDSHAKE,
    ),
    with(
        "SSL_do_handshake",
        "SSL_do_handshake",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        HANDSHAKE,
    ),
    with(
        "SSL_free",
        "SSL_free",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If ssl is NULL nothing is done.",
        },
        UNSTATED,
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
    with("SSL_read", "SSL_read", "1.0.2", DOC_1_0_2, Plain, HANDSHAKE),
    entry(
        "SSL_set_cert_cb",
        "SSL_CTX_set_cert_cb",
        "1.0.2",
        DOC_1_0_2,
        RegistersSlot {
            holder: 0,
            callback: 1,
            slot: "ssl_ctx_st.cert_cb",
            retains: Some(2),
        },
    ),
    entry(
        "SSL_set_accept_state",
        "SSL_set_connect_state",
        "1.0.2",
        DOC_1_0_2,
        Plain,
    ),
    with(
        "SSL_shutdown",
        "SSL_shutdown",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        UNSTATED,
    ),
    with(
        "SSL_write",
        "SSL_write",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        HANDSHAKE,
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
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
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
