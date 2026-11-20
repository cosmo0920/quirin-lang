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
    /// call may run callbacks no clause of the declaration states (a slot
    /// whose setters are not declared, a slot whose invocation the
    /// vocabulary cannot state beside the declaration's other clauses, a
    /// BIO's method or callback, an ex_data callback); the declaration reads
    /// them as changing no custody the caller holds (C04.18, a named premise
    /// published with every row resting on it). Card
    /// **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: it replaced
    /// `openssl_registered_callbacks`, which covered every slot, stated or
    /// not; card **OPENSSL-UNSTATED-CALLBACK-SLOTS** narrowed it again.
    UnstatedSlots,
    /// Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: `requires
    /// resource_premise(openssl_ex_data_callbacks, "…")`: every slot the call
    /// may run is stated, and the call may still run the ex_data callbacks
    /// `CRYPTO_get_ex_new_index` registers per object class (a global
    /// registry, not a slot of an object), which the declaration reads as
    /// changing no custody the caller holds. Card **OPENSSL-EX-DATA-REGISTRY**:
    /// named by every declaration that allocates, copies or finally frees an
    /// object of an ex_data class ([`EX_DATA_AUDIT`]), beside any other
    /// premise whose statement also mentions ex_data.
    ExDataCallbacks,
    /// Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: `requires
    /// resource_premise(openssl_builtin_bio_methods, "…")`, the library-value
    /// premise beside the stated method slots: a function of a method the
    /// library defines itself (`BIO_s_socket`, `BIO_s_mem`, …), which no
    /// setter is handed and so is in no run's slot set, changes no custody
    /// the caller holds.
    BuiltinBioMethods,
    /// Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: `requires
    /// resource_premise(openssl_bio_free_callbacks, "…")`: a drop of a BIO's
    /// last reference runs its callback (`BIO_CB_FREE`), its method's destroy
    /// function and the BIO class's ex_data free callbacks. Card
    /// **OPENSSL-FREE-SESSION-REMOVE-INVOCATION**: the first two are stated
    /// beside the drop now (C04.19 amended), so the premise covers only the
    /// ex_data free callbacks, which are no slot.
    BioFreeCallbacks,
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
    /// Card **OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS**: `ensures normal
    /// resource_callback_slot_registered(H, slot, F)` beside the shape's own
    /// registration: a setter that registers several callbacks at once
    /// (`SSL_CTX_add_custom_ext`'s `add_cb`, `free_cb` and `parse_cb`) is a
    /// writer of each of their slots.
    SlotRegistered {
        holder: usize,
        callback: usize,
        slot: &'static str,
    },
    /// Card **OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS**: `ensures normal
    /// resource_retained_by(item, holder)` beside the shape's own retention
    /// (C04.14): the holder keeps a second pointer for its callbacks
    /// (`SSL_CTX_add_custom_ext`'s `parse_arg` beside `add_arg`).
    RetainedBy { item: usize, holder: usize },
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
/// Card OPENSSL-UNSTATED-CALLBACK-SLOTS: every slot this crate declares the
/// setters of is stated where a declaration may run it, so what remains is
/// named by kind. Card OPENSSL-BIO-METHODS-AND-CALLBACKS: a BIO's methods and
/// callback are stated slots now, so the statement no longer names them.
/// Card **SP1-SYMBOLIC-OPENSSL-PREMISES** (C04.18.9; user decision
/// 2026-11-20): the statement is a formula, in its canonical printing. A
/// callback slot of the SSL object or its SSL_CTX (the `SSL_CTX_callback_ctrl`
/// arms among them), or an ex_data callback the call may run, that no clause
/// of the declaration states takes, releases or retains nothing the caller
/// holds.
pub const UNSTATED_SLOTS_STATEMENT: &str = "#slots(#of(SSL, #args) | #ctx(#of(SSL, #args)) | \
#of(SSL_CTX, #args)) \\ #stated |-> #inert && quant all #o in #created | #copied | #freed: \
#ex_data(#class(#o)) \\ #stated |-> #inert";

/// Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: the name of the built-in BIO
/// methods premise.
pub const BUILTIN_BIO_METHODS: &str = "openssl_builtin_bio_methods";

/// Its statement. Card OPENSSL-BIO-READER-INVOCATIONS: the BIO is the one
/// the call reads or writes through, its own BIO argument (the PEM and
/// PKCS#12 readers) or one the SSL object holds (the I/O calls). Card
/// OPENSSL-FREE-SESSION-REMOVE-INVOCATION: or frees (`BIO_free`, `SSL_free`:
/// a built-in method's destroy function). Card
/// **SP1-SYMBOLIC-OPENSSL-PREMISES**: a formula. For every BIO of that chain
/// (its BIO argument or a BIO the SSL object holds, and those chained to it)
/// whose method the library defines itself, each function of the method, as
/// run on that BIO, takes, releases or retains nothing the caller holds, and
/// so does every callback it runs in turn on the SSL object of a BIO_f_ssl
/// BIO when that object is not an argument of the call.
pub const BUILTIN_BIO_METHODS_STATEMENT: &str = "quant all #b in #chain(#of(BIO, #args) | \
#bios(#of(SSL, #args))): #method(#b) in #builtin -> #fns(#b) |-> #inert && quant all #f in \
#fns(#b): #in_turn(#f, #ssl_of(#b) \\ #args) |-> #inert";

/// Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: the name of the BIO free
/// premise.
pub const BIO_FREE: &str = "openssl_bio_free_callbacks";

/// Its statement. Card OPENSSL-FREE-SESSION-REMOVE-INVOCATION: narrowed to
/// the ex_data free callbacks; the callback and destroy slots are stated.
/// Card **SP1-SYMBOLIC-OPENSSL-PREMISES**: a formula. For its BIO argument,
/// or a BIO the SSL object holds and the BIOs chained to it, whose last
/// reference the call drops, the ex_data free callbacks of the BIO class
/// take, release or retain nothing the caller holds.
pub const BIO_FREE_STATEMENT: &str = "quant all #b in #of(BIO, #args) | \
#chain(#bios(#of(SSL, #args))): #b in #freed -> #ex_data_free(BIO) |-> #inert";

/// Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: the name of the ex_data premise.
pub const EX_DATA: &str = "openssl_ex_data_callbacks";

/// Its statement. Card **SP1-SYMBOLIC-OPENSSL-PREMISES**: a formula. Every
/// new, dup or free callback `CRYPTO_get_ex_new_index` registers for the
/// class of an object the call creates, copies or frees takes, releases or
/// retains nothing the caller holds when the call runs it.
pub const EX_DATA_STATEMENT: &str = "quant all #o in #created | #copied | #freed: \
#ex_data(#class(#o)) |-> #inert";

/// Card **OPENSSL-EX-DATA-REGISTRY**: the classes whose objects carry
/// ex_data, as `CRYPTO_get_ex_new_index` (3.5) DESCRIPTION lists them ("The
/// specific structures are: …"). For each, "When a structure is initially
/// allocated … the new_func() is called for every defined index", "When a
/// structure is free'd … the free_func() is called for every defined index",
/// and `dup_func()` runs when an SSL, SSL_SESSION, EC_KEY or BIO chain is
/// copied. The registry is global per class: `CRYPTO_get_ex_new_index` takes
/// a class index and no object.
pub const EX_DATA_CLASSES: &[&str] = &[
    "BIO",
    "DH",
    "DSA",
    "EC_KEY",
    "ENGINE",
    "EVP_PKEY",
    "RSA",
    "SSL",
    "SSL_CTX",
    "SSL_SESSION",
    "UI",
    "UI_METHOD",
    "X509",
    "X509_STORE",
    "X509_STORE_CTX",
];

/// Card **OPENSSL-EX-DATA-REGISTRY**: how a declaration stands to the ex_data
/// registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExData {
    /// The call allocates, copies or finally frees an object of each of
    /// `classes` (one of [`EX_DATA_CLASSES`]), directly or as a replaced or
    /// temporary object it frees inside, so it may run that class's ex_data
    /// callbacks: the declaration names [`Premise::ExDataCallbacks`].
    Runs {
        classes: &'static [&'static str],
        how: &'static str,
    },
    /// A shape that creates or frees an object, of no ex_data class and
    /// freeing none: the declaration names no ex_data premise.
    NoClass { why: &'static str },
}

/// Card **OPENSSL-EX-DATA-REGISTRY**: the audit. Every documented function
/// carrying [`Premise::ExDataCallbacks`] is listed here as
/// [`ExData::Runs`], and every one whose shape creates or frees an object
/// (a constructor, a release, a reference drop) is listed either way, so a
/// new constructor or free cannot read the registry silently
/// ([`crate::check::ex_data_audited`]). A plain call is listed only when it
/// runs the callbacks; a plain call not listed creates and frees no object of
/// an ex_data class (the report records that audit).
pub const EX_DATA_AUDIT: &[(&str, ExData)] = &[
    (
        "BIO_free",
        ExData::Runs {
            classes: &["BIO"],
            how: "the last drop frees the BIO (and the BIOs chained to it)",
        },
    ),
    (
        "BIO_new_file",
        ExData::Runs {
            classes: &["BIO"],
            how: "allocates a BIO",
        },
    ),
    (
        "ENGINE_by_id",
        ExData::Runs {
            classes: &["ENGINE"],
            how: "allocates a dynamic engine, freed when it fails to load",
        },
    ),
    (
        "ENGINE_free",
        ExData::Runs {
            classes: &["ENGINE"],
            how: "the last structural reference frees the ENGINE",
        },
    ),
    (
        "EVP_MD_CTX_free",
        ExData::Runs {
            classes: &["EVP_PKEY"],
            how: "a signing context's EVP_PKEY_CTX may hold the last reference to its key",
        },
    ),
    (
        "EVP_MD_CTX_new",
        ExData::NoClass {
            why: "an EVP_MD_CTX is no ex_data class and a fresh one holds nothing",
        },
    ),
    (
        "EVP_PKEY_free",
        ExData::Runs {
            classes: &["EVP_PKEY"],
            how: "the last reference frees the EVP_PKEY",
        },
    ),
    (
        "OPENSSL_init_ssl",
        ExData::Runs {
            classes: &["ENGINE"],
            how: "the configuration it loads by default may create and free engines",
        },
    ),
    (
        "OPENSSL_sk_free",
        ExData::NoClass {
            why: "frees the stack only, never its elements",
        },
    ),
    (
        "OPENSSL_sk_new_null",
        ExData::NoClass {
            why: "a stack is no ex_data class",
        },
    ),
    (
        "PEM_read_bio_PrivateKey",
        ExData::Runs {
            classes: &["EVP_PKEY"],
            how: "allocates the key it returns, freed on a failed read",
        },
    ),
    (
        "PEM_read_bio_X509",
        ExData::Runs {
            classes: &["X509"],
            how: "allocates the certificate it returns, freed on a failed read",
        },
    ),
    (
        "PKCS12_free",
        ExData::Runs {
            classes: &["X509"],
            how: "frees the certificates a signed PKCS#7 safe holds",
        },
    ),
    (
        "SSL_CTX_free",
        ExData::Runs {
            classes: &["SSL_CTX", "X509_STORE", "X509", "EVP_PKEY", "SSL_SESSION"],
            how: "the last drop frees the context, its store, certificates, keys and cached \
                  sessions",
        },
    ),
    (
        "SSL_CTX_load_verify_locations",
        ExData::Runs {
            classes: &["X509"],
            how: "reads CAfile's certificates, freeing each the store already holds",
        },
    ),
    (
        "SSL_CTX_new",
        ExData::Runs {
            classes: &["SSL_CTX", "X509_STORE"],
            how: "allocates the context and its certificate store",
        },
    ),
    (
        "SSL_CTX_set_default_verify_paths",
        ExData::Runs {
            classes: &["X509"],
            how: "the default file lookup reads certificates, freeing each the store already holds",
        },
    ),
    (
        "SSL_CTX_use_PrivateKey",
        ExData::Runs {
            classes: &["EVP_PKEY", "X509"],
            how: "frees the key it replaces, and a certificate the new key does not match",
        },
    ),
    (
        "SSL_CTX_use_PrivateKey_file",
        ExData::Runs {
            classes: &["EVP_PKEY", "X509"],
            how: "reads a temporary key, freed after use, and replaces the context's key",
        },
    ),
    (
        "SSL_CTX_use_certificate",
        ExData::Runs {
            classes: &["X509", "EVP_PKEY"],
            how: "frees the certificate it replaces, and a key the new certificate does not match",
        },
    ),
    (
        "SSL_CTX_use_certificate_chain_file",
        ExData::Runs {
            classes: &["X509"],
            how: "reads certificates and frees the chain and certificate it replaces",
        },
    ),
    (
        "SSL_connect",
        ExData::Runs {
            classes: &["SSL_SESSION", "X509", "EVP_PKEY"],
            how: "the handshake creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "SSL_do_handshake",
        ExData::Runs {
            classes: &["SSL_SESSION", "X509", "EVP_PKEY"],
            how: "the handshake creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "SSL_free",
        ExData::Runs {
            classes: &["SSL", "SSL_SESSION", "X509", "BIO"],
            how: "the last drop frees the connection, its session, peer certificate and BIOs",
        },
    ),
    (
        "SSL_get1_peer_certificate",
        ExData::NoClass {
            why: "takes one more reference on the peer certificate, allocating nothing",
        },
    ),
    (
        "SSL_new",
        ExData::Runs {
            classes: &["SSL", "BIO"],
            how: "allocates the connection and the memory BIOs of its record layers",
        },
    ),
    (
        "SSL_read",
        ExData::Runs {
            classes: &["SSL_SESSION", "X509", "EVP_PKEY"],
            how: "a handshake it runs creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "SSL_set_accept_state",
        ExData::Runs {
            classes: &["BIO"],
            how: "resets the record layer (RECORD_LAYER_reset, 3.5 sources), freeing and \
                  creating memory BIOs",
        },
    ),
    (
        "SSL_shutdown",
        ExData::Runs {
            classes: &["SSL_SESSION"],
            how: "a NewSessionTicket read before the close_notify creates a session",
        },
    ),
    (
        "SSL_write",
        ExData::Runs {
            classes: &["SSL_SESSION", "X509", "EVP_PKEY"],
            how: "a handshake it runs creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "X509_STORE_free",
        ExData::Runs {
            classes: &["X509_STORE", "X509"],
            how: "the last reference frees the store and the certificates it holds",
        },
    ),
    (
        "X509_free",
        ExData::Runs {
            classes: &["X509"],
            how: "the last reference frees the certificate",
        },
    ),
    (
        "X509_new",
        ExData::Runs {
            classes: &["X509"],
            how: "allocates a certificate",
        },
    ),
    (
        "d2i_PKCS12_bio",
        ExData::Runs {
            classes: &["X509"],
            how: "a signed PKCS#7 safe's certificates are decoded, freed on a failed parse",
        },
    ),
];

/// Card **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: a callback slot of an opaque
/// OpenSSL object, stated as C04.19.3's opaque slot. Its `writers` are every
/// function the manual page `page` (at the pinned series, 3.5) names that
/// sets the slot; the checker requires each to be declared with
/// [`Documented::RegistersSlot`] of this slot, so the run's candidates (what
/// the declared setters are handed) are every value the slot can hold.
///
/// Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: `not_writers` lists every other
/// function the page's NAME section names (getters, the userdata setters,
/// writers of a copy no declaration invokes). The checker requires the NAME
/// list to be exactly the union, over the slots citing the page, of their
/// writers and not-writers, so a writer the page names cannot be left out
/// unremarked.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub key: &'static str,
    pub page: &'static str,
    pub writers: &'static [&'static str],
    pub not_writers: &'static [&'static str],
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
        not_writers: &[],
    },
    // SSL_CTX_set_default_passwd_cb (3.5): the context's slot. The page's
    // SSL_set_default_passwd_cb writes the SSL's copy, which no declaration
    // invokes (only the context's loaders are declared).
    Slot {
        key: "ssl_ctx_st.default_passwd_cb",
        page: "SSL_CTX_set_default_passwd_cb",
        writers: &["SSL_CTX_set_default_passwd_cb"],
        not_writers: &[
            "SSL_CTX_set_default_passwd_cb_userdata",
            "SSL_CTX_get_default_passwd_cb",
            "SSL_CTX_get_default_passwd_cb_userdata",
            "SSL_set_default_passwd_cb",
            "SSL_set_default_passwd_cb_userdata",
            "SSL_get_default_passwd_cb",
            "SSL_get_default_passwd_cb_userdata",
        ],
    },
    // Card OPENSSL-UNSTATED-CALLBACK-SLOTS. SSL_CTX_set_verify (3.5): the
    // verify callback of the context and of the SSL ("If no special callback
    // was set before, the default callback for the underlying ctx is used,
    // that was valid at the time ssl was created"): one key.
    Slot {
        key: "ssl_ctx_st.verify_callback",
        page: "SSL_CTX_set_verify",
        writers: &["SSL_CTX_set_verify", "SSL_set_verify"],
        not_writers: &[
            "SSL_get_ex_data_X509_STORE_CTX_idx",
            "SSL_CTX_set_verify_depth",
            "SSL_set_verify_depth",
            "SSL_verify_cb",
            "SSL_verify_client_post_handshake",
            "SSL_set_post_handshake_auth",
            "SSL_CTX_set_post_handshake_auth",
        ],
    },
    // SSL_CTX_set_info_callback (3.5): "The setting for ctx is overridden
    // from the setting for a specific SSL object, if specified": one key.
    Slot {
        key: "ssl_ctx_st.info_callback",
        page: "SSL_CTX_set_info_callback",
        writers: &["SSL_CTX_set_info_callback", "SSL_set_info_callback"],
        not_writers: &["SSL_CTX_get_info_callback", "SSL_get_info_callback"],
    },
    // SSL_CTX_set_msg_callback (3.5): the context's settings "will be copied
    // to new SSL objects by SSL_new"; SSL_set_msg_callback modifies the
    // SSL's: one key. The *_arg setters keep the callback's argument, not
    // the callback; SSL_trace is a callback a setter may be handed.
    Slot {
        key: "ssl_ctx_st.msg_callback",
        page: "SSL_CTX_set_msg_callback",
        writers: &["SSL_CTX_set_msg_callback", "SSL_set_msg_callback"],
        not_writers: &[
            "SSL_CTX_set_msg_callback_arg",
            "SSL_set_msg_callback_arg",
            "SSL_trace",
        ],
    },
    // SSL_CTX_set_security_level (3.5): "An application defined security
    // callback is installed with SSL_CTX_set_security_callback() or
    // SSL_set_security_callback()"; SSL_new copies the context's: one key.
    Slot {
        key: "ssl_ctx_st.sec_cb",
        page: "SSL_CTX_set_security_level",
        writers: &["SSL_CTX_set_security_callback", "SSL_set_security_callback"],
        not_writers: &[
            "SSL_CTX_set_security_level",
            "SSL_set_security_level",
            "SSL_CTX_get_security_level",
            "SSL_get_security_level",
            "SSL_CTX_get_security_callback",
            "SSL_get_security_callback",
            "SSL_CTX_set0_security_ex_data",
            "SSL_set0_security_ex_data",
            "SSL_CTX_get0_security_ex_data",
            "SSL_get0_security_ex_data",
        ],
    },
    // SSL_CTX_sess_set_get_cb (3.5): three slots of the context, one setter
    // each; the getters are listed once.
    Slot {
        key: "ssl_ctx_st.new_session_cb",
        page: "SSL_CTX_sess_set_get_cb",
        writers: &["SSL_CTX_sess_set_new_cb"],
        not_writers: &[
            "SSL_CTX_sess_get_new_cb",
            "SSL_CTX_sess_get_remove_cb",
            "SSL_CTX_sess_get_get_cb",
        ],
    },
    Slot {
        key: "ssl_ctx_st.remove_session_cb",
        page: "SSL_CTX_sess_set_get_cb",
        writers: &["SSL_CTX_sess_set_remove_cb"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.get_session_cb",
        page: "SSL_CTX_sess_set_get_cb",
        writers: &["SSL_CTX_sess_set_get_cb"],
        not_writers: &[],
    },
    // SSL_CTX_set_client_cert_cb (3.5): the context's one writer.
    Slot {
        key: "ssl_ctx_st.client_cert_cb",
        page: "SSL_CTX_set_client_cert_cb",
        writers: &["SSL_CTX_set_client_cert_cb"],
        not_writers: &["SSL_CTX_get_client_cert_cb"],
    },
    // Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS. SSL_CTX_set_keylog_callback
    // (3.5): the context's one writer; the callback type is named too.
    Slot {
        key: "ssl_ctx_st.keylog_callback",
        page: "SSL_CTX_set_keylog_callback",
        writers: &["SSL_CTX_set_keylog_callback"],
        not_writers: &["SSL_CTX_get_keylog_callback", "SSL_CTX_keylog_cb_func"],
    },
    // SSL_CTX_set_alpn_select_cb (3.5): three slots of the context, one
    // writer each (ALPN selection, NPN selection, NPN advertisement); the
    // protocol list setters, the selection helper and the getters are listed
    // once.
    Slot {
        key: "ssl_ctx_st.alpn_select_cb",
        page: "SSL_CTX_set_alpn_select_cb",
        writers: &["SSL_CTX_set_alpn_select_cb"],
        not_writers: &[
            "SSL_CTX_set_alpn_protos",
            "SSL_set_alpn_protos",
            "SSL_select_next_proto",
            "SSL_get0_alpn_selected",
            "SSL_get0_next_proto_negotiated",
        ],
    },
    Slot {
        key: "ssl_ctx_st.npn_select_cb",
        page: "SSL_CTX_set_alpn_select_cb",
        writers: &["SSL_CTX_set_next_proto_select_cb"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.npn_advertised_cb",
        page: "SSL_CTX_set_alpn_select_cb",
        writers: &["SSL_CTX_set_next_protos_advertised_cb"],
        not_writers: &[],
    },
    // SSL_CTX_set_cert_verify_callback (3.5): "SSL objects that are created
    // from ctx inherit the setting": the context's one writer.
    Slot {
        key: "ssl_ctx_st.app_verify_callback",
        page: "SSL_CTX_set_cert_verify_callback",
        writers: &["SSL_CTX_set_cert_verify_callback"],
        not_writers: &[],
    },
    // SSL_CTX_set_psk_client_callback (3.5): two slots (the TLSv1.2 client
    // callback and the TLSv1.3 use-session callback), each with its context
    // and SSL writer; the callback types are listed once.
    Slot {
        key: "ssl_ctx_st.psk_client_callback",
        page: "SSL_CTX_set_psk_client_callback",
        writers: &[
            "SSL_CTX_set_psk_client_callback",
            "SSL_set_psk_client_callback",
        ],
        not_writers: &["SSL_psk_client_cb_func", "SSL_psk_use_session_cb_func"],
    },
    Slot {
        key: "ssl_ctx_st.psk_use_session_cb",
        page: "SSL_CTX_set_psk_client_callback",
        writers: &[
            "SSL_CTX_set_psk_use_session_callback",
            "SSL_set_psk_use_session_callback",
        ],
        not_writers: &[],
    },
    // SSL_CTX_use_psk_identity_hint (3.5): the two server slots (TLSv1.2
    // server callback, TLSv1.3 find-session callback); the identity hint
    // setters keep a copy of a string, not a callback.
    Slot {
        key: "ssl_ctx_st.psk_server_callback",
        page: "SSL_CTX_use_psk_identity_hint",
        writers: &[
            "SSL_CTX_set_psk_server_callback",
            "SSL_set_psk_server_callback",
        ],
        not_writers: &[
            "SSL_psk_server_cb_func",
            "SSL_psk_find_session_cb_func",
            "SSL_CTX_use_psk_identity_hint",
            "SSL_use_psk_identity_hint",
        ],
    },
    Slot {
        key: "ssl_ctx_st.psk_find_session_cb",
        page: "SSL_CTX_use_psk_identity_hint",
        writers: &[
            "SSL_CTX_set_psk_find_session_callback",
            "SSL_set_psk_find_session_callback",
        ],
        not_writers: &[],
    },
    // SSL_CTX_set_client_hello_cb (3.5): the context's one writer; the
    // other names are the callback type and the accessors it may call.
    Slot {
        key: "ssl_ctx_st.client_hello_cb",
        page: "SSL_CTX_set_client_hello_cb",
        writers: &["SSL_CTX_set_client_hello_cb"],
        not_writers: &[
            "SSL_client_hello_cb_fn",
            "SSL_client_hello_isv2",
            "SSL_client_hello_get0_legacy_version",
            "SSL_client_hello_get0_random",
            "SSL_client_hello_get0_session_id",
            "SSL_client_hello_get0_ciphers",
            "SSL_client_hello_get0_compression_methods",
            "SSL_client_hello_get1_extensions_present",
            "SSL_client_hello_get_extension_order",
            "SSL_client_hello_get0_ext",
        ],
    },
    // SSL_CTX_set_record_padding_callback (3.5): "The value set in ctx is
    // copied to a new SSL by SSL_new()": one key. The `_arg` setters keep the
    // callback's argument, not the callback; block padding is a size.
    Slot {
        key: "ssl_ctx_st.record_padding_cb",
        page: "SSL_CTX_set_record_padding_callback",
        writers: &[
            "SSL_CTX_set_record_padding_callback",
            "SSL_set_record_padding_callback",
        ],
        not_writers: &[
            "SSL_CTX_set_record_padding_callback_arg",
            "SSL_set_record_padding_callback_arg",
            "SSL_CTX_get_record_padding_callback_arg",
            "SSL_get_record_padding_callback_arg",
            "SSL_CTX_set_block_padding",
            "SSL_CTX_set_block_padding_ex",
            "SSL_set_block_padding",
            "SSL_set_block_padding_ex",
        ],
    },
    // SSL_read_early_data (3.5): the early data slot's two writers, beside
    // the early data I/O calls and limits (none declared).
    Slot {
        key: "ssl_ctx_st.allow_early_data_cb",
        page: "SSL_read_early_data",
        writers: &[
            "SSL_CTX_set_allow_early_data_cb",
            "SSL_set_allow_early_data_cb",
        ],
        not_writers: &[
            "SSL_set_max_early_data",
            "SSL_CTX_set_max_early_data",
            "SSL_get_max_early_data",
            "SSL_CTX_get_max_early_data",
            "SSL_set_recv_max_early_data",
            "SSL_CTX_set_recv_max_early_data",
            "SSL_get_recv_max_early_data",
            "SSL_CTX_get_recv_max_early_data",
            "SSL_SESSION_get_max_early_data",
            "SSL_SESSION_set_max_early_data",
            "SSL_write_early_data",
            "SSL_read_early_data",
            "SSL_get_early_data_status",
            "SSL_allow_early_data_cb_fn",
        ],
    },
    // SSL_extension_supported (3.5): six slots, the add, free and parse
    // callbacks of the `_ex` style (SSL_CTX_add_custom_ext) and of the older
    // style (SSL_CTX_add_client_custom_ext, SSL_CTX_add_server_custom_ext),
    // each setter registering three; the callback types and
    // SSL_extension_supported are listed once.
    Slot {
        key: "ssl_ctx_st.custom_ext_add_cb_ex",
        page: "SSL_extension_supported",
        writers: &["SSL_CTX_add_custom_ext"],
        not_writers: &[
            "SSL_extension_supported",
            "SSL_custom_ext_add_cb_ex",
            "SSL_custom_ext_free_cb_ex",
            "SSL_custom_ext_parse_cb_ex",
            "custom_ext_add_cb",
            "custom_ext_free_cb",
            "custom_ext_parse_cb",
        ],
    },
    Slot {
        key: "ssl_ctx_st.custom_ext_free_cb_ex",
        page: "SSL_extension_supported",
        writers: &["SSL_CTX_add_custom_ext"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.custom_ext_parse_cb_ex",
        page: "SSL_extension_supported",
        writers: &["SSL_CTX_add_custom_ext"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.custom_ext_add_cb",
        page: "SSL_extension_supported",
        writers: &[
            "SSL_CTX_add_client_custom_ext",
            "SSL_CTX_add_server_custom_ext",
        ],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.custom_ext_free_cb",
        page: "SSL_extension_supported",
        writers: &[
            "SSL_CTX_add_client_custom_ext",
            "SSL_CTX_add_server_custom_ext",
        ],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.custom_ext_parse_cb",
        page: "SSL_extension_supported",
        writers: &[
            "SSL_CTX_add_client_custom_ext",
            "SSL_CTX_add_server_custom_ext",
        ],
        not_writers: &[],
    },
    // Card OPENSSL-BIO-METHODS-AND-CALLBACKS. BIO_set_callback (3.5): a BIO's
    // callback in two formats, one writer each; the getters, the callback
    // argument macros, the debugging callbacks (values a setter may be
    // handed) and the callback types are listed once.
    Slot {
        key: "bio_st.bio_callback_ex",
        page: "BIO_set_callback",
        writers: &["BIO_set_callback_ex"],
        not_writers: &[
            "BIO_get_callback_ex",
            "BIO_get_callback",
            "BIO_set_callback_arg",
            "BIO_get_callback_arg",
            "BIO_debug_callback",
            "BIO_debug_callback_ex",
            "BIO_callback_fn_ex",
            "BIO_callback_fn",
        ],
    },
    Slot {
        key: "bio_st.bio_callback",
        page: "BIO_set_callback",
        writers: &["BIO_set_callback"],
        not_writers: &[],
    },
    // BIO_meth_new (3.5): one slot per function of a method table, one setter
    // each; the type index, the table's constructor and destructor and the
    // twelve getters are listed once. The holder is the BIO_METHOD a setter
    // is handed; the library's own methods are no setter's value (premise
    // `openssl_builtin_bio_methods`).
    Slot {
        key: "bio_method_st.meth_write_ex",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_write_ex"],
        not_writers: &[
            "BIO_get_new_index",
            "BIO_meth_new",
            "BIO_meth_free",
            "BIO_meth_get_read_ex",
            "BIO_meth_get_write_ex",
            "BIO_meth_get_write",
            "BIO_meth_get_read",
            "BIO_meth_get_puts",
            "BIO_meth_get_gets",
            "BIO_meth_get_ctrl",
            "BIO_meth_get_create",
            "BIO_meth_get_destroy",
            "BIO_meth_get_callback_ctrl",
            "BIO_meth_get_sendmmsg",
            "BIO_meth_get_recvmmsg",
        ],
    },
    Slot {
        key: "bio_method_st.meth_write",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_write"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_read_ex",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_read_ex"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_read",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_read"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_puts",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_puts"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_gets",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_gets"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_ctrl",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_ctrl"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_create",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_create"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_destroy",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_destroy"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_callback_ctrl",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_callback_ctrl"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_sendmmsg",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_sendmmsg"],
        not_writers: &[],
    },
    Slot {
        key: "bio_method_st.meth_recvmmsg",
        page: "BIO_meth_new",
        writers: &["BIO_meth_set_recvmmsg"],
        not_writers: &[],
    },
];

/// Card **CALLBACK-SUPPLIED-NULL-ACTUAL**: an argument a slot's callback may
/// receive as NULL, by its page. At every invocation of `slot` whose actual
/// at `position` is a value the callee supplies (`_`), the declaration
/// states `supplied_nullable` instead (C04.19 item 2), so a candidate
/// requiring a live object there (`resource_owned(nonnull)`) is refused. A
/// parameter actual there is unchanged: it is the caller's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NullableArgument {
    /// The slot, one of [`SLOTS`].
    pub slot: &'static str,
    /// The callback's argument position.
    pub position: usize,
    /// The sentence of the slot's page at 3.5 saying so, exactly as its
    /// snapshot records it (`null_sentences` or `argument_null_sentences`).
    pub sentence: &'static str,
}

/// The arguments the stated slots' pages (3.5) say may be NULL. The survey
/// of every stated slot is in `docs/reports/callback-supplied-null-actual/`;
/// an argument no page sentence covers stays `_`.
pub const NULLABLE_ARGUMENTS: &[NullableArgument] = &[
    // SSL_CTX_set_security_level (3.5), "APPLICATION-DEFINED SECURITY
    // CALLBACKS": `cb(s, ctx, op, bits, nid, other, ex)`.
    NullableArgument {
        slot: "ssl_ctx_st.sec_cb",
        position: 0,
        sentence: "One of the two is NULL: checks originating from a connection pass s (with \
                   ctx NULL), and checks originating from a context pass ctx (with s NULL).",
    },
    NullableArgument {
        slot: "ssl_ctx_st.sec_cb",
        position: 1,
        sentence: "One of the two is NULL: checks originating from a connection pass s (with \
                   ctx NULL), and checks originating from a context pass ctx (with s NULL).",
    },
    NullableArgument {
        slot: "ssl_ctx_st.sec_cb",
        position: 5,
        sentence: "It is NULL when no object is associated with the operation.",
    },
    NullableArgument {
        slot: "ssl_ctx_st.sec_cb",
        position: 6,
        sentence: "ex The extra data pointer previously set with \
                   SSL_CTX_set0_security_ex_data() or SSL_set0_security_ex_data(), or NULL.",
    },
    // SSL_CTX_set_psk_client_callback (3.5): `client(ssl, hint, identity,
    // max_identity_len, psk, max_psk_len)` (NOTES) and `use_session(ssl, md,
    // id, idlen, sess)` (DESCRIPTION).
    NullableArgument {
        slot: "ssl_ctx_st.psk_client_callback",
        position: 1,
        sentence: "Note that parameter hint given to the callback may be NULL .",
    },
    NullableArgument {
        slot: "ssl_ctx_st.psk_use_session_cb",
        position: 1,
        sentence: "The first time the callback is called for a connection the md parameter is \
                   NULL.",
    },
    // BIO_set_callback (3.5), "CALLBACK OPERATIONS": `callback_ex(b, oper,
    // argp, len, argi, argl, ret, processed)` and `callback(b, oper, argp,
    // argi, argl, ret)`; the table of calls passes NULL at `argp` and
    // `processed`.
    NullableArgument {
        slot: "bio_st.bio_callback_ex",
        position: 2,
        sentence: "BIO_free(b) callback_ex(b, BIO_CB_FREE, NULL, 0, 0, 0L, 1L, NULL) or \
                   callback(b, BIO_CB_FREE, NULL, 0L, 0L, 1L) is called before the free \
                   operation.",
    },
    NullableArgument {
        slot: "bio_st.bio_callback_ex",
        position: 7,
        sentence: "BIO_free(b) callback_ex(b, BIO_CB_FREE, NULL, 0, 0, 0L, 1L, NULL) or \
                   callback(b, BIO_CB_FREE, NULL, 0L, 0L, 1L) is called before the free \
                   operation.",
    },
    NullableArgument {
        slot: "bio_st.bio_callback",
        position: 2,
        sentence: "BIO_free(b) callback_ex(b, BIO_CB_FREE, NULL, 0, 0, 0L, 1L, NULL) or \
                   callback(b, BIO_CB_FREE, NULL, 0L, 0L, 1L) is called before the free \
                   operation.",
    },
];

/// Whether `slot`'s argument at `position` may be NULL by its page
/// ([`NULLABLE_ARGUMENTS`]).
#[must_use]
pub fn nullable_argument(slot: &str, position: usize) -> bool {
    NULLABLE_ARGUMENTS
        .iter()
        .any(|nullable| nullable.slot == slot && nullable.position == position)
}

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
const SINCE_0_9_7: &str = "resource_library_version(openssl, since \"0.9.7\")";
const SINCE_1_1_1: &str = "resource_library_version(openssl, since \"1.1.1\")";
const SINCE_3_2_0: &str = "resource_library_version(openssl, since \"3.2.0\")";

/// Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: a method table setter added in
/// 1.1.0 (`BIO_meth_new` 1.1.1 HISTORY: "The functions described here were
/// added in OpenSSL 1.1.0"), registering the function pointer at position 1
/// into `slot` of the table at position 0.
const fn meth(function: &'static str, slot: &'static str) -> DocumentedFunction {
    entry(
        function,
        "BIO_meth_new",
        "1.1.1",
        SINCE_1_1_0,
        registers(1, slot),
    )
}

/// Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: a setter that registers the
/// function pointer at `callback` into `slot` of the object at position 0,
/// keeping no other pointer.
const fn registers(callback: usize, slot: &'static str) -> Documented {
    Documented::RegistersSlot {
        holder: 0,
        callback,
        slot,
        retains: None,
    }
}

/// Card **OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS**: a setter that registers
/// the function pointer at `callback` into `slot` of the object at position 0
/// and keeps the pointer at `arg` for it.
const fn registers_with_arg(callback: usize, slot: &'static str, arg: usize) -> Documented {
    Documented::RegistersSlot {
        holder: 0,
        callback,
        slot,
        retains: Some(arg),
    }
}

/// Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: a slot of the connection at
/// position 0, invoked with these actuals.
const fn on_ssl(slot: &'static str, arguments: &'static [Option<usize>]) -> Premise {
    Premise::SlotInvoked {
        object: 0,
        slot,
        arguments,
    }
}

/// The verify callback, `verify_callback(preverify_ok, x509_ctx)`: two
/// values the callee supplies.
const VERIFY: Premise = on_ssl("ssl_ctx_st.verify_callback", &[None, None]);
/// The info callback, `callback(ssl, where, ret)`.
const INFO: Premise = on_ssl("ssl_ctx_st.info_callback", &[Some(0), None, None]);
/// The message callback, `cb(write_p, version, content_type, buf, len, ssl,
/// arg)`: `ssl` is "The SSL object that received or sent the message".
const MESSAGE: Premise = on_ssl(
    "ssl_ctx_st.msg_callback",
    &[None, None, None, None, None, Some(0), None],
);
/// The security callback of a connection's check, `cb(s, ctx, op, bits, nid,
/// other, ex)`: "checks originating from a connection pass s (with ctx
/// NULL)".
const SECURITY: Premise = on_ssl(
    "ssl_ctx_st.sec_cb",
    &[Some(0), None, None, None, None, None, None],
);
/// The session callbacks: `new_session_cb(ssl, sess)`,
/// `remove_session_cb(ctx, sess)` (the session's context, not a formal) and
/// `get_session_cb(ssl, data, len, copy)`.
const NEW_SESSION: Premise = on_ssl("ssl_ctx_st.new_session_cb", &[Some(0), None]);
const REMOVE_SESSION: Premise = on_ssl("ssl_ctx_st.remove_session_cb", &[None, None]);
const GET_SESSION: Premise = on_ssl("ssl_ctx_st.get_session_cb", &[Some(0), None, None, None]);
/// The client certificate callback, `client_cert_cb(ssl, x509, pkey)`.
const CLIENT_CERT: Premise = on_ssl("ssl_ctx_st.client_cert_cb", &[Some(0), None, None]);

/// Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: the keylog callback, called
/// "with two items: the ssl object associated with the connection, and line".
const KEYLOG: Premise = on_ssl("ssl_ctx_st.keylog_callback", &[Some(0), None]);
/// The ALPN selection callback, `cb(ssl, out, outlen, in, inlen, arg)`.
const ALPN_SELECT: Premise = on_ssl(
    "ssl_ctx_st.alpn_select_cb",
    &[Some(0), None, None, None, None, None],
);
/// The NPN selection callback, `cb(s, out, outlen, in, inlen, arg)`.
const NPN_SELECT: Premise = on_ssl(
    "ssl_ctx_st.npn_select_cb",
    &[Some(0), None, None, None, None, None],
);
/// The NPN advertisement callback, `cb(ssl, out, outlen, arg)`.
const NPN_ADVERTISED: Premise =
    on_ssl("ssl_ctx_st.npn_advertised_cb", &[Some(0), None, None, None]);
/// The certificate verification callback, `callback(x509_store_ctx, arg)`:
/// two values the callee supplies (the connection is not an argument).
const APP_VERIFY: Premise = on_ssl("ssl_ctx_st.app_verify_callback", &[None, None]);
/// The PSK callbacks: `client(ssl, hint, identity, max_identity_len, psk,
/// max_psk_len)`, `use_session(ssl, md, id, idlen, sess)`, `server(ssl,
/// identity, psk, max_psk_len)`, `find_session(ssl, identity, identity_len,
/// sess)`.
const PSK_CLIENT: Premise = on_ssl(
    "ssl_ctx_st.psk_client_callback",
    &[Some(0), None, None, None, None, None],
);
const PSK_USE_SESSION: Premise = on_ssl(
    "ssl_ctx_st.psk_use_session_cb",
    &[Some(0), None, None, None, None],
);
const PSK_SERVER: Premise = on_ssl(
    "ssl_ctx_st.psk_server_callback",
    &[Some(0), None, None, None],
);
const PSK_FIND_SESSION: Premise = on_ssl(
    "ssl_ctx_st.psk_find_session_cb",
    &[Some(0), None, None, None],
);
/// The ClientHello callback, `cb(s, al, arg)`.
const CLIENT_HELLO: Premise = on_ssl("ssl_ctx_st.client_hello_cb", &[Some(0), None, None]);
/// The record padding callback, `cb(ssl, type, len, arg)`: "invoked for every
/// record before encryption".
const RECORD_PADDING: Premise =
    on_ssl("ssl_ctx_st.record_padding_cb", &[Some(0), None, None, None]);
/// The early data callback, `cb(s, arg)`.
const ALLOW_EARLY_DATA: Premise = on_ssl("ssl_ctx_st.allow_early_data_cb", &[Some(0), None]);
/// The custom extension callbacks. `_ex` style: `add_cb(s, ext_type, context,
/// out, outlen, x, chainidx, al, add_arg)`, `free_cb(s, ext_type, context, out,
/// add_arg)`, `parse_cb(s, ext_type, context, in, inlen, x, chainidx, al,
/// parse_arg)`; older style: `add_cb(s, ext_type, out, outlen, al, add_arg)`,
/// `free_cb(s, ext_type, out, add_arg)`, `parse_cb(s, ext_type, in, inlen, al,
/// parse_arg)`.
const CUSTOM_ADD_EX: Premise = on_ssl(
    "ssl_ctx_st.custom_ext_add_cb_ex",
    &[Some(0), None, None, None, None, None, None, None, None],
);
const CUSTOM_FREE_EX: Premise = on_ssl(
    "ssl_ctx_st.custom_ext_free_cb_ex",
    &[Some(0), None, None, None, None],
);
const CUSTOM_PARSE_EX: Premise = on_ssl(
    "ssl_ctx_st.custom_ext_parse_cb_ex",
    &[Some(0), None, None, None, None, None, None, None, None],
);
const CUSTOM_ADD: Premise = on_ssl(
    "ssl_ctx_st.custom_ext_add_cb",
    &[Some(0), None, None, None, None, None],
);
const CUSTOM_FREE: Premise = on_ssl(
    "ssl_ctx_st.custom_ext_free_cb",
    &[Some(0), None, None, None],
);
const CUSTOM_PARSE: Premise = on_ssl(
    "ssl_ctx_st.custom_ext_parse_cb",
    &[Some(0), None, None, None, None, None],
);

/// Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: a slot of a BIO the
/// connection at position 0 holds, every actual the callee's own (the BIO,
/// never the connection).
/// Card **OPENSSL-BIO-READER-INVOCATIONS**: or of the BIO at position 0 a
/// reader reads through, or one chained to it (every actual `_` there too).
const fn on_bio(slot: &'static str, arguments: &'static [Option<usize>]) -> Premise {
    Premise::SlotInvoked {
        object: 0,
        slot,
        arguments,
    }
}

/// The BIO callback slots and the method table slots, each with its
/// function's arity (`callback_ex(b, oper, argp, len, argi, argl, ret,
/// processed)`, `callback(b, oper, argp, argi, argl, ret)`, then the method
/// functions' prototypes in `BIO_meth_new`'s synopsis and the pinned header),
/// and the library-value premise for the built-in methods. Run wherever the
/// call writes or reads a record through the SSL's BIOs, and wherever a
/// reader reads through its BIO argument.
const BIO_SLOTS: [Premise; 15] = [
    Premise::BuiltinBioMethods,
    on_bio("bio_st.bio_callback_ex", &[None; 8]),
    on_bio("bio_st.bio_callback", &[None; 6]),
    on_bio("bio_method_st.meth_write_ex", &[None; 4]),
    on_bio("bio_method_st.meth_write", &[None; 3]),
    on_bio("bio_method_st.meth_read_ex", &[None; 4]),
    on_bio("bio_method_st.meth_read", &[None; 3]),
    on_bio("bio_method_st.meth_puts", &[None; 2]),
    on_bio("bio_method_st.meth_gets", &[None; 3]),
    on_bio("bio_method_st.meth_ctrl", &[None; 4]),
    on_bio("bio_method_st.meth_create", &[None; 1]),
    on_bio("bio_method_st.meth_destroy", &[None; 1]),
    on_bio("bio_method_st.meth_callback_ctrl", &[None; 3]),
    on_bio("bio_method_st.meth_sendmmsg", &[None; 6]),
    on_bio("bio_method_st.meth_recvmmsg", &[None; 6]),
];

/// Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: a handshake or I/O call on `ssl` at
/// position 0: the certificate slot, `cert_cb(ssl, arg)`, and the premise
/// for every slot whose setters are not declared. Card
/// OPENSSL-UNSTATED-CALLBACK-SLOTS: and every slot this crate now declares
/// the setters of, each run during a handshake (which the I/O calls run when
/// needed). Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: and the keylog,
/// ALPN/NPN, certificate verification, PSK, ClientHello, record padding,
/// early data and custom extension slots.
const HANDSHAKE: &[Premise] = &[
    Premise::UnstatedSlots,
    // Card OPENSSL-EX-DATA-REGISTRY.
    Premise::ExDataCallbacks,
    on_ssl("ssl_ctx_st.cert_cb", &[Some(0), None]),
    VERIFY,
    INFO,
    MESSAGE,
    SECURITY,
    NEW_SESSION,
    REMOVE_SESSION,
    GET_SESSION,
    CLIENT_CERT,
    KEYLOG,
    ALPN_SELECT,
    NPN_SELECT,
    NPN_ADVERTISED,
    APP_VERIFY,
    PSK_CLIENT,
    PSK_USE_SESSION,
    PSK_SERVER,
    PSK_FIND_SESSION,
    CLIENT_HELLO,
    RECORD_PADDING,
    ALLOW_EARLY_DATA,
    CUSTOM_ADD_EX,
    CUSTOM_FREE_EX,
    CUSTOM_PARSE_EX,
    CUSTOM_ADD,
    CUSTOM_FREE,
    CUSTOM_PARSE,
    // Card OPENSSL-BIO-METHODS-AND-CALLBACKS.
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[3],
    BIO_SLOTS[4],
    BIO_SLOTS[5],
    BIO_SLOTS[6],
    BIO_SLOTS[7],
    BIO_SLOTS[8],
    BIO_SLOTS[9],
    BIO_SLOTS[10],
    BIO_SLOTS[11],
    BIO_SLOTS[12],
    BIO_SLOTS[13],
    BIO_SLOTS[14],
];

/// Card OPENSSL-UNSTATED-CALLBACK-SLOTS: `SSL_shutdown` sends or reads the
/// close_notify alert: the info callback ("an alert appears") and the
/// message callback (every protocol message sent or received); the premise
/// for every other callback it may run. Card
/// OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: and the record padding callback,
/// "invoked for every record before encryption", whose `type` may be
/// `SSL3_RT_ALERT` (SSL_CTX_set_record_padding_callback, 3.5).
/// Card OPENSSL-BIO-METHODS-AND-CALLBACKS: and the BIO slots, through which
/// the alert is written and read.
const SHUTDOWN: &[Premise] = &[
    Premise::UnstatedSlots,
    // Card OPENSSL-EX-DATA-REGISTRY.
    Premise::ExDataCallbacks,
    INFO,
    MESSAGE,
    RECORD_PADDING,
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[3],
    BIO_SLOTS[4],
    BIO_SLOTS[5],
    BIO_SLOTS[6],
    BIO_SLOTS[7],
    BIO_SLOTS[8],
    BIO_SLOTS[9],
    BIO_SLOTS[10],
    BIO_SLOTS[11],
    BIO_SLOTS[12],
    BIO_SLOTS[13],
    BIO_SLOTS[14],
];

/// Card OPENSSL-BIO-METHODS-AND-CALLBACKS: `BIO_free` drops a BIO reference;
/// `SSL_free` keeps the unstated-slots premise and frees the SSL's BIOs.
/// Card **OPENSSL-FREE-SESSION-REMOVE-INVOCATION** (C04.19 amended: a slot
/// invocation stands beside a reference drop): a BIO's last drop runs its
/// callback with `BIO_CB_FREE` and its method's destroy function, stated
/// with every actual `_`, a built-in method's destroy function under
/// [`Premise::BuiltinBioMethods`], and the narrowed free premise names only
/// the BIO class's ex_data free callbacks. `SSL_free` and `SSL_CTX_free`
/// state the session-remove invocation ("called for all sessions in the
/// internal session cache when SSL_CTX_free(3) is called" and "when a
/// connection was not shutdown cleanly", `SSL_CTX_sess_set_get_cb` 3.5).
/// Both keep the unstated-slots premise: freeing the context frees its
/// certificate store, whose lookup methods' free and shutdown functions are
/// user-settable (`X509_LOOKUP_meth_set_free`).
/// Card OPENSSL-EX-DATA-REGISTRY: all three name the ex_data premise too
/// (each last drop frees an object of an ex_data class).
const BIO_FREED: &[Premise] = &[
    Premise::BioFreeCallbacks,
    Premise::ExDataCallbacks,
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[11],
];
const SSL_FREED: &[Premise] = &[
    Premise::UnstatedSlots,
    Premise::BioFreeCallbacks,
    Premise::ExDataCallbacks,
    REMOVE_SESSION,
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[11],
];
const SSL_CTX_FREED: &[Premise] = &[
    Premise::UnstatedSlots,
    Premise::ExDataCallbacks,
    REMOVE_SESSION,
];

/// The context's password slot, `pem_password_cb(buf, size, rwflag, u)`:
/// four values the callee supplies.
const PASSWORD: Premise = Premise::SlotInvoked {
    object: 0,
    slot: "ssl_ctx_st.default_passwd_cb",
    arguments: &[None, None, None, None],
};

/// Card OPENSSL-UNSTATED-CALLBACK-SLOTS: the security callback of a context's
/// check on the certificates the call loads from a file: `s` NULL (a value
/// the callee supplies), `ctx` the context, `other` a certificate the callee
/// read.
const SECURITY_CTX_LOADED: Premise = Premise::SlotInvoked {
    object: 0,
    slot: "ssl_ctx_st.sec_cb",
    arguments: &[None, Some(0), None, None, None, None, None],
};

/// The same check on the caller's certificate at position 1: `other` "is
/// the X509 certificate" (SSL_SECOP_EE_KEY, SSL_SECOP_CA_KEY).
const SECURITY_CTX_CERTIFICATE: Premise = Premise::SlotInvoked {
    object: 0,
    slot: "ssl_ctx_st.sec_cb",
    arguments: &[None, Some(0), None, None, None, Some(1), None],
};

/// Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: `SSL_CTX_add_custom_ext`'s
/// two other registrations (`free_cb` at 4, `parse_cb` at 6) and its second
/// retention (`parse_arg` at 7), beside the shape's `add_cb` (3) and
/// `add_arg` (5).
const CUSTOM_EXT_EX: &[Premise] = &[
    Premise::SlotRegistered {
        holder: 0,
        callback: 4,
        slot: "ssl_ctx_st.custom_ext_free_cb_ex",
    },
    Premise::SlotRegistered {
        holder: 0,
        callback: 6,
        slot: "ssl_ctx_st.custom_ext_parse_cb_ex",
    },
    Premise::RetainedBy { item: 7, holder: 0 },
];

/// The older style setters' (`free_cb` at 3, `parse_cb` at 5, `parse_arg` at
/// 6), beside `add_cb` (2) and `add_arg` (4).
const CUSTOM_EXT: &[Premise] = &[
    Premise::SlotRegistered {
        holder: 0,
        callback: 3,
        slot: "ssl_ctx_st.custom_ext_free_cb",
    },
    Premise::SlotRegistered {
        holder: 0,
        callback: 5,
        slot: "ssl_ctx_st.custom_ext_parse_cb",
    },
    Premise::RetainedBy { item: 6, holder: 0 },
];

/// Card **OPENSSL-EX-DATA-REGISTRY**: the ex_data premise alone, for a
/// declaration that may run no unstated slot but creates, copies or frees an
/// object of an ex_data class ([`EX_DATA_AUDIT`]).
const EX_DATA_ONLY: &[Premise] = &[Premise::ExDataCallbacks];

/// `PEM_read_bio_*(bp, x, cb, u)`: a NULL `x` (no object to reuse), and the
/// password callback `cb` called during the read with `u` as its last
/// argument, a NULL `cb` meaning OpenSSL's own default routine (card
/// OPENSSL-PEM-CALLBACK-ARM: a premise stands beside a synchronous
/// invocation, gateway rule 57). The other three arguments (`buf`, `size`,
/// `rwflag`) are values the callee supplies.
const PEM_READ: &[Premise] = &[
    Premise::NullArgument(1),
    // Card OPENSSL-EX-DATA-REGISTRY: the read allocates the X509 or EVP_PKEY.
    Premise::ExDataCallbacks,
    Premise::CallbackInvokedOrDefault {
        callback: 2,
        arguments: &[None, None, None, Some(3)],
    },
    // Card OPENSSL-BIO-READER-INVOCATIONS: the read runs the slots of the
    // caller's BIO at position 0 (gateway rule 57 amended: a slot invocation
    // stands beside the premise).
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[3],
    BIO_SLOTS[4],
    BIO_SLOTS[5],
    BIO_SLOTS[6],
    BIO_SLOTS[7],
    BIO_SLOTS[8],
    BIO_SLOTS[9],
    BIO_SLOTS[10],
    BIO_SLOTS[11],
    BIO_SLOTS[12],
    BIO_SLOTS[13],
    BIO_SLOTS[14],
];

/// Card **OPENSSL-BIO-READER-INVOCATIONS**: `d2i_PKCS12_bio(bp, p12)`: a NULL
/// `p12` (no object to reuse or free), and the slots of the BIO `bp` at
/// position 0 it parses from.
const PKCS12_READ: &[Premise] = &[
    Premise::NullArgument(1),
    // Card OPENSSL-EX-DATA-REGISTRY: a signed safe's certificates are decoded.
    Premise::ExDataCallbacks,
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[3],
    BIO_SLOTS[4],
    BIO_SLOTS[5],
    BIO_SLOTS[6],
    BIO_SLOTS[7],
    BIO_SLOTS[8],
    BIO_SLOTS[9],
    BIO_SLOTS[10],
    BIO_SLOTS[11],
    BIO_SLOTS[12],
    BIO_SLOTS[13],
    BIO_SLOTS[14],
];

use Documented::{
    BorrowedResult, Constructor, CountedConstructor, CountedRelease, NullAcceptingRelease, Plain,
    RegistersSlot, Release, Retains,
};
use Null::Nullable;

/// The documented semantics of every contracted function, sorted by name.
pub const DOCUMENTED: &[DocumentedFunction] = &[
    with(
        "BIO_free",
        "BIO_new",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
        BIO_FREED,
    ),
    meth(
        "BIO_meth_set_callback_ctrl",
        "bio_method_st.meth_callback_ctrl",
    ),
    meth("BIO_meth_set_create", "bio_method_st.meth_create"),
    meth("BIO_meth_set_ctrl", "bio_method_st.meth_ctrl"),
    meth("BIO_meth_set_destroy", "bio_method_st.meth_destroy"),
    meth("BIO_meth_set_gets", "bio_method_st.meth_gets"),
    meth("BIO_meth_set_puts", "bio_method_st.meth_puts"),
    meth("BIO_meth_set_read", "bio_method_st.meth_read"),
    meth("BIO_meth_set_read_ex", "bio_method_st.meth_read_ex"),
    entry(
        "BIO_meth_set_recvmmsg",
        "BIO_meth_new",
        "3.5",
        SINCE_3_2_0,
        registers(1, "bio_method_st.meth_recvmmsg"),
    ),
    entry(
        "BIO_meth_set_sendmmsg",
        "BIO_meth_new",
        "3.5",
        SINCE_3_2_0,
        registers(1, "bio_method_st.meth_sendmmsg"),
    ),
    meth("BIO_meth_set_write", "bio_method_st.meth_write"),
    meth("BIO_meth_set_write_ex", "bio_method_st.meth_write_ex"),
    with(
        "BIO_new_file",
        "BIO_s_file",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
        EX_DATA_ONLY,
    ),
    // Card OPENSSL-BIO-METHODS-AND-CALLBACKS: the 1.0.2 page documents
    // BIO_set_callback as a macro, so both callback setters cite 1.1.1.
    entry(
        "BIO_set_callback",
        "BIO_set_callback",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "bio_st.bio_callback"),
    ),
    entry(
        "BIO_set_callback_ex",
        "BIO_set_callback",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "bio_st.bio_callback_ex"),
    ),
    with(
        "ENGINE_by_id",
        "ENGINE_add",
        "1.1.1",
        DOC_1_1_1,
        CountedConstructor,
        EX_DATA_ONLY,
    ),
    with(
        "ENGINE_free",
        "ENGINE_add",
        "3.0",
        DOC_3_0,
        NullAcceptingRelease {
            position: 0,
            sentence: "If the argument to ENGINE_free() is NULL, nothing is done.",
        },
        EX_DATA_ONLY,
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
    with(
        "EVP_MD_CTX_free",
        "EVP_DigestInit",
        "3.5",
        SINCE_1_1_0,
        Release(0, Nullable),
        EX_DATA_ONLY,
    ),
    entry(
        "EVP_MD_CTX_new",
        "EVP_DigestInit",
        "3.5",
        SINCE_1_1_0,
        Constructor,
    ),
    with(
        "EVP_PKEY_free",
        "EVP_PKEY_new",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If key is NULL, nothing is done.",
        },
        EX_DATA_ONLY,
    ),
    entry("EVP_sha256", "EVP_DigestInit", "1.0.2", DOC_1_0_2, Plain),
    entry("EVP_sha512", "EVP_DigestInit", "1.0.2", DOC_1_0_2, Plain),
    with(
        "OPENSSL_init_ssl",
        "OPENSSL_init_ssl",
        "3.5",
        SINCE_1_1_0,
        Plain,
        EX_DATA_ONLY,
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
    with(
        "PKCS12_free",
        "X509_dup",
        "1.1.1",
        DOC_1_1_1,
        Release(0, Nullable),
        EX_DATA_ONLY,
    ),
    with(
        "SSL_CTX_add_client_custom_ext",
        "SSL_extension_supported",
        "1.1.1",
        DOC_1_1_1,
        registers_with_arg(2, "ssl_ctx_st.custom_ext_add_cb", 4),
        CUSTOM_EXT,
    ),
    with(
        "SSL_CTX_add_custom_ext",
        "SSL_extension_supported",
        "3.5",
        SINCE_1_1_1,
        registers_with_arg(3, "ssl_ctx_st.custom_ext_add_cb_ex", 5),
        CUSTOM_EXT_EX,
    ),
    with(
        "SSL_CTX_add_server_custom_ext",
        "SSL_extension_supported",
        "1.1.1",
        DOC_1_1_1,
        registers_with_arg(2, "ssl_ctx_st.custom_ext_add_cb", 4),
        CUSTOM_EXT,
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
        SSL_CTX_FREED,
    ),
    entry(
        "SSL_CTX_get_cert_store",
        "SSL_CTX_set_cert_store",
        "1.0.2",
        DOC_1_0_2,
        BorrowedResult(0, Nullable),
    ),
    with(
        "SSL_CTX_load_verify_locations",
        "SSL_CTX_load_verify_locations",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        EX_DATA_ONLY,
    ),
    with(
        "SSL_CTX_new",
        "SSL_CTX_new",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
        EX_DATA_ONLY,
    ),
    entry(
        "SSL_CTX_sess_set_get_cb",
        "SSL_CTX_sess_set_get_cb",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.get_session_cb"),
    ),
    entry(
        "SSL_CTX_sess_set_new_cb",
        "SSL_CTX_sess_set_get_cb",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.new_session_cb"),
    ),
    entry(
        "SSL_CTX_sess_set_remove_cb",
        "SSL_CTX_sess_set_get_cb",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.remove_session_cb"),
    ),
    entry(
        "SSL_CTX_set_allow_early_data_cb",
        "SSL_read_early_data",
        "3.5",
        SINCE_1_1_1,
        registers_with_arg(1, "ssl_ctx_st.allow_early_data_cb", 2),
    ),
    entry(
        "SSL_CTX_set_alpn_select_cb",
        "SSL_CTX_set_alpn_select_cb",
        "1.0.2",
        DOC_1_0_2,
        registers_with_arg(1, "ssl_ctx_st.alpn_select_cb", 2),
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
        "SSL_CTX_set_cert_verify_callback",
        "SSL_CTX_set_cert_verify_callback",
        "1.0.2",
        SINCE_0_9_7,
        registers_with_arg(1, "ssl_ctx_st.app_verify_callback", 2),
    ),
    entry(
        "SSL_CTX_set_client_cert_cb",
        "SSL_CTX_set_client_cert_cb",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.client_cert_cb"),
    ),
    entry(
        "SSL_CTX_set_client_hello_cb",
        "SSL_CTX_set_client_hello_cb",
        "3.5",
        SINCE_1_1_1,
        registers_with_arg(1, "ssl_ctx_st.client_hello_cb", 2),
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
    with(
        "SSL_CTX_set_default_verify_paths",
        "SSL_CTX_load_verify_locations",
        "1.1.1",
        DOC_1_1_1,
        Plain,
        EX_DATA_ONLY,
    ),
    entry(
        "SSL_CTX_set_info_callback",
        "SSL_CTX_set_info_callback",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.info_callback"),
    ),
    entry(
        "SSL_CTX_set_keylog_callback",
        "SSL_CTX_set_keylog_callback",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "ssl_ctx_st.keylog_callback"),
    ),
    entry(
        "SSL_CTX_set_msg_callback",
        "SSL_CTX_set_msg_callback",
        "1.0.2",
        SINCE_0_9_7,
        registers(1, "ssl_ctx_st.msg_callback"),
    ),
    entry(
        "SSL_CTX_set_next_proto_select_cb",
        "SSL_CTX_set_alpn_select_cb",
        "1.1.1",
        DOC_1_1_1,
        registers_with_arg(1, "ssl_ctx_st.npn_select_cb", 2),
    ),
    entry(
        "SSL_CTX_set_next_protos_advertised_cb",
        "SSL_CTX_set_alpn_select_cb",
        "1.1.1",
        DOC_1_1_1,
        registers_with_arg(1, "ssl_ctx_st.npn_advertised_cb", 2),
    ),
    entry(
        "SSL_CTX_set_options",
        "SSL_CTX_set_options",
        "3.5",
        SINCE_3_0_0,
        Plain,
    ),
    entry(
        "SSL_CTX_set_psk_client_callback",
        "SSL_CTX_set_psk_client_callback",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.psk_client_callback"),
    ),
    entry(
        "SSL_CTX_set_psk_find_session_callback",
        "SSL_CTX_use_psk_identity_hint",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.psk_find_session_cb"),
    ),
    entry(
        "SSL_CTX_set_psk_server_callback",
        "SSL_CTX_use_psk_identity_hint",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.psk_server_callback"),
    ),
    entry(
        "SSL_CTX_set_psk_use_session_callback",
        "SSL_CTX_set_psk_client_callback",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.psk_use_session_cb"),
    ),
    entry(
        "SSL_CTX_set_record_padding_callback",
        "SSL_CTX_set_record_padding_callback",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.record_padding_cb"),
    ),
    entry(
        "SSL_CTX_set_security_callback",
        "SSL_CTX_set_security_level",
        "3.5",
        SINCE_1_1_0,
        registers(1, "ssl_ctx_st.sec_cb"),
    ),
    entry(
        "SSL_CTX_set_verify",
        "SSL_CTX_set_verify",
        "1.0.2",
        DOC_1_0_2,
        registers(2, "ssl_ctx_st.verify_callback"),
    ),
    with(
        "SSL_CTX_use_PrivateKey",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        EX_DATA_ONLY,
    ),
    with(
        "SSL_CTX_use_PrivateKey_file",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        &[Premise::ExDataCallbacks, PASSWORD],
    ),
    with(
        "SSL_CTX_use_certificate",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        &[Premise::ExDataCallbacks, SECURITY_CTX_CERTIFICATE],
    ),
    with(
        "SSL_CTX_use_certificate_chain_file",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        &[Premise::ExDataCallbacks, PASSWORD, SECURITY_CTX_LOADED],
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
        SSL_FREED,
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
    with(
        "SSL_new",
        "SSL_new",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
        EX_DATA_ONLY,
    ),
    with("SSL_read", "SSL_read", "1.0.2", DOC_1_0_2, Plain, HANDSHAKE),
    with(
        "SSL_set_accept_state",
        "SSL_set_connect_state",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        EX_DATA_ONLY,
    ),
    entry(
        "SSL_set_allow_early_data_cb",
        "SSL_read_early_data",
        "3.5",
        SINCE_1_1_1,
        registers_with_arg(1, "ssl_ctx_st.allow_early_data_cb", 2),
    ),
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
        "SSL_set_info_callback",
        "SSL_CTX_set_info_callback",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.info_callback"),
    ),
    entry(
        "SSL_set_msg_callback",
        "SSL_CTX_set_msg_callback",
        "1.0.2",
        SINCE_0_9_7,
        registers(1, "ssl_ctx_st.msg_callback"),
    ),
    entry(
        "SSL_set_psk_client_callback",
        "SSL_CTX_set_psk_client_callback",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.psk_client_callback"),
    ),
    entry(
        "SSL_set_psk_find_session_callback",
        "SSL_CTX_use_psk_identity_hint",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.psk_find_session_cb"),
    ),
    entry(
        "SSL_set_psk_server_callback",
        "SSL_CTX_use_psk_identity_hint",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.psk_server_callback"),
    ),
    entry(
        "SSL_set_psk_use_session_callback",
        "SSL_CTX_set_psk_client_callback",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.psk_use_session_cb"),
    ),
    entry(
        "SSL_set_record_padding_callback",
        "SSL_CTX_set_record_padding_callback",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.record_padding_cb"),
    ),
    entry(
        "SSL_set_security_callback",
        "SSL_CTX_set_security_level",
        "3.5",
        SINCE_1_1_0,
        registers(1, "ssl_ctx_st.sec_cb"),
    ),
    entry(
        "SSL_set_verify",
        "SSL_CTX_set_verify",
        "1.0.2",
        DOC_1_0_2,
        registers(2, "ssl_ctx_st.verify_callback"),
    ),
    with(
        "SSL_shutdown",
        "SSL_shutdown",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        SHUTDOWN,
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
    with(
        "X509_STORE_free",
        "X509_STORE_new",
        "1.1.1",
        DOC_1_1_1,
        CountedRelease(0),
        EX_DATA_ONLY,
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
    with(
        "X509_free",
        "X509_new",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
        EX_DATA_ONLY,
    ),
    entry(
        "X509_get_issuer_name",
        "X509_get_subject_name",
        "1.1.1",
        DOC_1_1_1,
        BorrowedResult(0, Nullable),
    ),
    with(
        "X509_new",
        "X509_new",
        "1.0.2",
        DOC_1_0_2,
        CountedConstructor,
        EX_DATA_ONLY,
    ),
    with(
        "d2i_PKCS12_bio",
        "d2i_X509",
        "1.1.1",
        DOC_1_1_1,
        Constructor,
        PKCS12_READ,
    ),
];
