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
    /// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: `requires
    /// resource_premise(openssl_unstated_callback_slots, "…")` with the
    /// statement of a free ([`UNSTATED_FREE_STATEMENT`]): the free states
    /// every slot of the objects it frees whose setters are declared, and the
    /// premise names what remains by name, the functions of methods,
    /// providers and engines of objects it frees in turn that no setter
    /// declared here writes. Card
    /// **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**: named by every free
    /// the free audit ([`FREE_AUDIT`]) reads as running such functions, the
    /// frees of keys, certificates, PKCS#12 objects, engines and digest
    /// contexts among them. The statement (C04.18.9) quantifies over the
    /// call's own `#freed`, so one statement names exactly what each free
    /// runs.
    UnstatedFreeSlots,
    /// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: `requires
    /// resource_premise(openssl_unstated_callback_slots, …)` with the
    /// statement of a free through a released parameter
    /// ([`UNSTATED_RELEASED_FREE_STATEMENT`]): a free whose argument the
    /// declaration releases (`resource_released`) can state no slot
    /// invocation (C04.19.3 reads a slot through a borrowed parameter), so
    /// its premise names every teardown function no clause states, the
    /// settable ones included.
    UnstatedReleasedFreeSlots,
    /// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: `requires
    /// resource_premise(openssl_builtin_teardown_functions, …)`
    /// ([`BUILTIN_TEARDOWN_STATEMENT`]), the library-value premise beside the
    /// stated teardown slots: a teardown function the library defines itself
    /// (a built-in method table's, a built-in engine's, or a dispatch function
    /// of the default, base, legacy or null provider, read at 3.5.5), which
    /// no setter of the run is handed, changes no custody the caller holds.
    BuiltinTeardown,
    /// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS** (user decision
    /// 2026-11-21, "Named premise + built-in source"): `requires
    /// resource_premise(openssl_loaded_module_functions, …)`
    /// ([`LOADED_TEARDOWN_STATEMENT`]): a teardown function of a provider or
    /// engine module the library loads from outside the run changes no
    /// custody the caller holds.
    LoadedTeardown,
    /// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: `requires
    /// resource_premise(openssl_builtin_lookup_methods, "…")`, the
    /// library-value premise beside the stated lookup method slots: a
    /// function of a lookup method the library defines itself
    /// (`X509_LOOKUP_file`, `X509_LOOKUP_hash_dir`, `X509_LOOKUP_store`),
    /// which no setter is handed and so is in no run's slot set, changes no
    /// custody the caller holds.
    BuiltinLookupMethods,
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
/// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: every documented function
/// pointer setter of a handshake slot is declared and its slot stated, so the
/// statement names what is left by name: the ctrl arms, the CT validation
/// callback (written by the library's own `SSL_CTX_enable_ct`), the
/// not-resumable session and session ticket extension callbacks (whose
/// setters no manual page documents), the functions of a lookup method the
/// context's certificate store holds (card OPENSSL-FREE-INDIRECT-CALLBACKS
/// declares their setters), and ex_data. Card
/// **SP1-SYMBOLIC-OPENSSL-PREMISES** (C04.18.9): a formula, in its canonical
/// printing. A callback slot of the SSL object or its SSL_CTX (the named
/// arms and callbacks among them), a function of the method of a lookup the
/// SSL_CTX's certificate store holds, or an ex_data callback the call may run,
/// that no clause of the declaration states takes, releases or retains
/// nothing the caller holds.
/// Card **OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS**: widened by the free
/// statement ([`UNSTATED_FREE_STATEMENT`], verbatim) as its last conjunct.
/// The handshake and I/O calls and `SSL_shutdown` free peer and ephemeral
/// keys, peer certificates, key exchange and signature contexts, transcript
/// digests and record-layer cipher contexts inside the call (OpenSSL 3.5.5,
/// `ssl/statem/`, `ssl/s3_lib.c`, the record layer), and a declaration names
/// each premise once (C04.18), so the handshake's own statement names what
/// those frees run.
pub const UNSTATED_SLOTS_STATEMENT: &str = "#slots(#of(SSL, #args) | #ctx(#of(SSL, #args)) | \
#of(SSL_CTX, #args)) \\ #stated |-> #inert && (quant all #l in #lookups(#store(#ctx(#of(SSL, \
#args)) | #of(SSL_CTX, #args))): #fns(#l) \\ #stated |-> #inert) && (quant all #o in #created | \
#copied | #freed: #ex_data(#class(#o)) \\ #stated |-> #inert) && quant all #o in #freed: \
#teardown(#o) \\ (#stated | #settable) |-> #inert";

/// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: the statement of the
/// unstated-slots premise on a free (`SSL_free`, `SSL_CTX_free`,
/// `X509_STORE_free`). A free runs none of the handshake's unstated kinds
/// (the ctrl arms, CT and the undocumented handshake setters run during a
/// handshake), its ex_data callbacks are `openssl_ex_data_callbacks`, and the
/// lookup methods' shutdown and free functions are stated slots now. What
/// remains, by the source survey of OpenSSL 3.5.5 (report section 2), is
/// named: the functions a key's provider, `EVP_PKEY_ASN1_METHOD`,
/// `RSA_METHOD`, `DSA_METHOD`, `DH_METHOD` or `EC_KEY_METHOD` and ENGINE run
/// when the key is freed, the client certificate engine, a CRL's
/// `X509_CRL_METHOD`, the async wait context's cleanup functions, the QUIC
/// TLS callbacks, and what a QUIC object's free runs (not traced).
/// Card **SP1-SYMBOLIC-OPENSSL-PREMISES** (C04.18.9): a formula. Every
/// callback the free of an object the call frees (directly or in turn) runs
/// from the object's methods, provider or engine, or holds itself, that no
/// clause states and whose setter no declaration of this crate states,
/// takes, releases or retains nothing the caller holds. The prose named its
/// members "among them"; each is a member of `#teardown` (report section 4).
pub const UNSTATED_FREE_STATEMENT: &str =
    "quant all #o in #freed: #teardown(#o) \\ (#stated | #settable) |-> #inert";

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: the name of the
/// library-value premise of the frees.
pub const BUILTIN_TEARDOWN: &str = "openssl_builtin_teardown_functions";

/// Its statement (C04.18.9). For every object the call frees, directly or in
/// turn, each function of its teardown that the library defines itself
/// (`#library`) takes, releases or retains nothing the caller holds. A
/// callback such a function runs in turn on an object it frees is a member
/// of that object's own teardown, so it is stated, or under this premise or
/// the loaded one, on its own (reading R6 of the report).
pub const BUILTIN_TEARDOWN_STATEMENT: &str =
    "quant all #o in #freed: quant all #f in #teardown(#o): #f in #library -> #f |-> #inert";

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: the name of the one
/// premise the user decided for the providers (and engine modules) the
/// library loads.
pub const LOADED_TEARDOWN: &str = "openssl_loaded_module_functions";

/// Its statement. For every object the call frees, each function of its
/// teardown that a module the library loads defines (`#loaded`: a provider
/// other than the built-in default, base, legacy and null providers, or an
/// engine the dynamic engine loads) takes, releases or retains nothing the
/// caller holds.
pub const LOADED_TEARDOWN_STATEMENT: &str =
    "quant all #o in #freed: quant all #f in #teardown(#o): #f in #loaded -> #f |-> #inert";

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: every teardown slot
/// (a [`SLOTS`] or [`SOURCE_SLOTS`] key run when an object is freed in
/// turn). A free states exactly the ones its audit lists.
pub const TEARDOWN_SLOTS: &[&str] = &[
    "evp_pkey_asn1_method_st.asn1_meth_pkey_free",
    "rsa_meth_st.rsa_meth_finish",
    "dsa_method.dsa_meth_finish",
    "dh_method.dh_meth_finish",
    "ec_key_method_st.ec_key_meth_finish",
    "engine_st.engine_finish",
    "engine_st.engine_destroy",
    "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths",
    "evp_md_st.md_meth_cleanup",
    "evp_cipher_st.cipher_meth_cleanup",
    "evp_pkey_method_st.pkey_meth_cleanup",
    "async_wait_ctx_st.wait_fd_cleanup",
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: the statement of the
/// unstated-slots premise on a free through a released parameter
/// (`PKCS12_free`, `EVP_MD_CTX_free`). Every callback the free of an object
/// the call frees runs from the object's methods, provider or engine, or
/// holds itself, that no clause states, takes, releases or retains nothing
/// the caller holds. It does not subtract `#settable`: such a free cannot
/// state the slots whose setters are declared.
pub const UNSTATED_RELEASED_FREE_STATEMENT: &str =
    "quant all #o in #freed: #teardown(#o) \\ #stated |-> #inert";

/// Card **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**: how a free stands
/// to the functions of methods, providers and engines of the objects it
/// frees (the free audit, read like [`EX_DATA_AUDIT`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frees {
    /// The free may run such functions (OpenSSL 3.5.5, `how`). Card
    /// **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: it states exactly the
    /// teardown slots `slots` (each one of [`TEARDOWN_SLOTS`]) and names
    /// [`Premise::BuiltinTeardown`] and [`Premise::LoadedTeardown`]; it names
    /// [`Premise::UnstatedFreeSlots`] exactly when `left` is not empty, the
    /// items no declaration can state yet, unless it names the handshake's
    /// statement (card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS), whose
    /// free conjunct names them, and then never the free statement.
    Runs {
        how: &'static str,
        slots: &'static [&'static str],
        left: &'static [&'static str],
    },
    /// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: the free releases
    /// its argument (`resource_released`), so it can state no teardown slot:
    /// it names [`Premise::UnstatedReleasedFreeSlots`] and both teardown
    /// premises, and states no teardown slot. `left` names why.
    RunsThroughRelease {
        how: &'static str,
        left: &'static [&'static str],
    },
    /// What the free may run in turn is named by this other premise, which
    /// the declaration names; it names no free statement.
    NamedBy { premise: Premise, why: &'static str },
    /// The free frees nothing whose functions a user can set: it names no
    /// free statement.
    Nothing { why: &'static str },
}

/// Card **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**: the free audit.
/// Every documented function whose shape is a release (a free or a
/// reference drop) is listed, and every one naming the free statement is
/// listed as [`Frees::Runs`], so a free reaching a key, certificate, engine
/// or context cannot read their functions silently
/// ([`crate::check::free_audited`]). The readings cite the OpenSSL 3.5.5
/// sources, as the report's survey table does.
/// Card **OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS**: and every
/// declaration whose [`EX_DATA_AUDIT`] reading names a class of
/// [`INNER_FREE_CLASSES`] is listed too ([`crate::check::inner_frees_audited`]),
/// so a call that frees a key or certificate inside, without being a free,
/// cannot read their functions silently either. A [`Frees::Runs`] reading is
/// then met by the free statement or by the handshake's statement, which
/// conjoins it.
pub const FREE_AUDIT: &[(&str, Frees)] = &[
    (
        "BIO_free",
        Frees::NamedBy {
            premise: Premise::BuiltinBioMethods,
            why: "the callback and destroy slots are stated; what a built-in method's destroy \
                  function frees in turn (the SSL object of a BIO_f_ssl BIO) is named by \
                  openssl_builtin_bio_methods",
        },
    ),
    (
        "ENGINE_free",
        Frees::Runs {
            how: "the last structural reference runs the engine's public key method and ASN.1 \
                  method callbacks and its destroy function (engine_free_util)",
            slots: ENGINE_STRUCTURAL,
            left: &[],
        },
    ),
    (
        "EVP_MD_CTX_free",
        Frees::RunsThroughRelease {
            how: "frees the EVP_PKEY_CTX (method cleanup, provider freectx, engine, keys, a \
                  MAC context's cipher context) and clears the digest (cleanup, provider \
                  freectx, engine)",
            left: &[RELEASED_ARGUMENT],
        },
    ),
    (
        "EVP_PKEY_free",
        Frees::Runs {
            how: "the last reference runs the key's key management free, ASN.1 method free, \
                  RSA/DSA/DH/EC_KEY method finish and ENGINE finish",
            slots: KEY_TEARDOWN,
            left: &[],
        },
    ),
    (
        "OPENSSL_sk_free",
        Frees::Nothing {
            why: "frees the stack only, never its elements",
        },
    ),
    (
        "PEM_read_bio_PrivateKey",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "frees a decoded key lacking the selected components, a key a failed \
                  legacy decode built, and the decoder context (pem_pkey.c)",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST],
        },
    ),
    (
        "PEM_read_bio_X509",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "a failed decode frees the partial certificate and the public key it \
                  decoded (tasn_dec.c, x509_pubkey_ex_free), and the key's decoder context",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST],
        },
    ),
    (
        "PKCS12_free",
        Frees::RunsThroughRelease {
            how: "frees the PKCS#7 safes' certificates (and so their keys), signer info keys \
                  and CRLs",
            left: &[RELEASED_ARGUMENT, CRL_METHOD_FREE],
        },
    ),
    (
        "SSL_CTX_free",
        Frees::Runs {
            how: "the last drop frees the context's keys, certificates, stores and client \
                  certificate engine",
            slots: KEY_TEARDOWN,
            left: &[CRL_METHOD_FREE],
        },
    ),
    (
        "SSL_CTX_load_verify_locations",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "the file lookup frees what it read from CAfile: certificates and CRLs \
                  the store did not keep, and every private key (by_file.c, X509_INFO_free)",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST, CRL_METHOD_FREE],
        },
    ),
    (
        "SSL_CTX_set_default_verify_paths",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "the file lookup frees what it read from the default file: certificates \
                  and CRLs the store did not keep, and every private key (by_file.c)",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST, CRL_METHOD_FREE],
        },
    ),
    (
        "SSL_CTX_use_PrivateKey",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "frees the key it replaces (ssl_set_pkey); earlier series also free a \
                  certificate the key does not match",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST],
        },
    ),
    (
        "SSL_CTX_use_PrivateKey_file",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "frees the key it read after use, the key it replaces, and what the PEM \
                  or DER read frees on failure",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST],
        },
    ),
    (
        "SSL_CTX_use_certificate",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "frees the certificate it replaces and a private key the new certificate \
                  does not match (ssl_set_cert)",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST],
        },
    ),
    (
        "SSL_CTX_use_certificate_chain_file",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "frees the certificates it read, the chain it clears, and the certificate \
                  and key SSL_CTX_use_certificate replaces",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST],
        },
    ),
    (
        "SSL_connect",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "the handshake frees peer and ephemeral keys, peer certificates and chains, \
                  key exchange and signature contexts and transcript digests (ssl/statem, \
                  s3_lib.c), and the record layer's cipher contexts at a key change",
            slots: HANDSHAKE_TEARDOWN,
            left: &[],
        },
    ),
    (
        "SSL_do_handshake",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "the handshake frees peer and ephemeral keys, peer certificates and chains, \
                  key exchange and signature contexts and transcript digests (ssl/statem, \
                  s3_lib.c), and the record layer's cipher contexts at a key change",
            slots: HANDSHAKE_TEARDOWN,
            left: &[],
        },
    ),
    (
        "SSL_free",
        Frees::Runs {
            how: "the last drop frees the connection's keys, certificates, stores, digest and \
                  cipher contexts, async wait context and QUIC objects, and may free its \
                  context",
            slots: CONNECTION_TEARDOWN,
            left: &[CRL_METHOD_FREE, QUIC_TLS_RELEASE],
        },
    ),
    (
        "SSL_read",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "the handshake frees peer and ephemeral keys, peer certificates and chains, \
                  key exchange and signature contexts and transcript digests (ssl/statem, \
                  s3_lib.c), and the record layer's cipher contexts at a key change",
            slots: HANDSHAKE_TEARDOWN,
            left: &[],
        },
    ),
    (
        "SSL_shutdown",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "a post-handshake message read before the close_notify (a \
                  NewSessionTicket, a KeyUpdate) replaces the session or the record layer's \
                  cipher contexts",
            slots: HANDSHAKE_TEARDOWN,
            left: &[],
        },
    ),
    (
        "SSL_write",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "the handshake frees peer and ephemeral keys, peer certificates and chains, \
                  key exchange and signature contexts and transcript digests (ssl/statem, \
                  s3_lib.c), and the record layer's cipher contexts at a key change",
            slots: HANDSHAKE_TEARDOWN,
            left: &[],
        },
    ),
    (
        "X509_STORE_free",
        Frees::Runs {
            how: "the last reference frees the certificates (and so their keys) and CRLs the \
                  store holds",
            slots: KEY_TEARDOWN,
            left: &[CRL_METHOD_FREE],
        },
    ),
    (
        "X509_free",
        Frees::Runs {
            how: "the last reference frees the certificate's public key",
            slots: KEY_TEARDOWN,
            left: &[],
        },
    ),
    (
        "X509_new",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Nothing {
            why: "a failed allocation frees only the partial certificate, which holds no \
                  decoded key",
        },
    ),
    (
        "d2i_PKCS12_bio",
        // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS.
        Frees::Runs {
            how: "a failed parse frees the partial object: a signed PKCS#7 safe's \
                  certificates, their public keys and its CRLs (tasn_dec.c)",
            slots: KEY_TEARDOWN,
            left: &[INNER_FREES_REST, CRL_METHOD_FREE],
        },
    ),
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: what a free can run
/// that no declaration can state yet. A CRL's `X509_CRL_METHOD` free
/// function: its only writer, `X509_CRL_METHOD_new`, stores into the method
/// it returns, and a slot registration's holder is a parameter (C04.19.3).
pub const CRL_METHOD_FREE: &str = "the crl_free function of the X509_CRL_METHOD of a CRL freed in \
     turn (X509_CRL_METHOD_new registers into its result)";

/// The teardown slots of a free whose argument the declaration releases: a
/// slot invocation reads a borrowed parameter (C04.19.3), so none can be
/// stated through it.
pub const RELEASED_ARGUMENT: &str = "the teardown slots of the objects it frees in turn, which a \
     slot invocation cannot read through the released argument (C04.19.3)";

/// The QUIC TLS `crypto_release_rcd` callback of a TLS SSL object given
/// `SSL_set_quic_tls_cbs`, run by `RECORD_LAYER_clear` at its free: the setter
/// is handed a dispatch table, not a function pointer (C04.19.3).
pub const QUIC_TLS_RELEASE: &str = "the crypto_release_rcd function of the QUIC TLS dispatch table \
     SSL_set_quic_tls_cbs installs (a dispatch table, not a function pointer)";

/// The teardown slots of a key (and so of a certificate, a store, a context
/// or a PKCS#12 object, which free keys in turn): the ASN.1 method's free,
/// the RSA, DSA, DH or EC_KEY method's finish, and an engine's finish,
/// destroy and method enumerators (a key's engine may drop its last
/// reference).
pub const KEY_TEARDOWN: &[&str] = &[
    "evp_pkey_asn1_method_st.asn1_meth_pkey_free",
    "rsa_meth_st.rsa_meth_finish",
    "dsa_method.dsa_meth_finish",
    "dh_method.dh_meth_finish",
    "ec_key_method_st.ec_key_meth_finish",
    "engine_st.engine_finish",
    "engine_st.engine_destroy",
    "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths",
];

/// `ENGINE_free`: a structural free runs no finish function.
pub const ENGINE_STRUCTURAL: &[&str] = &[
    "engine_st.engine_destroy",
    "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths",
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: what a call that frees
/// inside (card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS) runs beyond the
/// teardown slots it states, which this card did not survey: it keeps the
/// free statement for it.
pub const INNER_FREES_REST: &str = "whatever else the objects the call frees inside run, which \
     the inner-free survey names and this card did not narrow";

/// The handshake and I/O calls and `SSL_shutdown`: a key's teardown slots and
/// the legacy digest, cipher and public key method cleanups of the contexts
/// they free (transcript digests, record-layer cipher contexts, key exchange
/// and signature contexts). The handshake statement's free conjunct names
/// the rest.
pub const HANDSHAKE_TEARDOWN: &[&str] = &[
    "evp_pkey_asn1_method_st.asn1_meth_pkey_free",
    "rsa_meth_st.rsa_meth_finish",
    "dsa_method.dsa_meth_finish",
    "dh_method.dh_meth_finish",
    "ec_key_method_st.ec_key_meth_finish",
    "engine_st.engine_finish",
    "engine_st.engine_destroy",
    "engine_st.engine_pkey_meths",
    "engine_st.engine_pkey_asn1_meths",
    "evp_md_st.md_meth_cleanup",
    "evp_cipher_st.cipher_meth_cleanup",
    "evp_pkey_method_st.pkey_meth_cleanup",
];

/// `SSL_free`: every teardown slot.
pub const CONNECTION_TEARDOWN: &[&str] = TEARDOWN_SLOTS;

/// Card **OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS**: the ex_data classes
/// whose objects' free runs functions of methods, providers and engines (a
/// key's, and a certificate's through its public key: `evp_pkey_free_it`,
/// `x509_pubkey_ex_free`, OpenSSL 3.5.5). A declaration whose
/// [`EX_DATA_AUDIT`] reading names one is in [`FREE_AUDIT`], whatever its
/// shape.
pub const INNER_FREE_CLASSES: &[&str] = &["EVP_PKEY", "X509"];

/// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: the name of the built-in lookup
/// methods premise.
pub const BUILTIN_LOOKUP_METHODS: &str = "openssl_builtin_lookup_methods";

/// Its statement. The three built-in methods' free functions release only
/// their own method data (`by_dir.c`'s `free_dir`, `by_store.c`'s
/// `by_store_free`; `X509_LOOKUP_file` has none) and none has a shutdown
/// function (OpenSSL 3.5.5).
/// Card **SP1-SYMBOLIC-OPENSSL-PREMISES**: a formula. For every lookup of a
/// certificate store the call frees whose method the library defines itself,
/// each function of the method, as run on that lookup, takes, releases or
/// retains nothing the caller holds.
pub const BUILTIN_LOOKUP_METHODS_STATEMENT: &str = "quant all #l in #lookups(#of(X509_STORE, #freed)): #method(#l) in #builtin -> \
#fns(#l) |-> #inert";

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
            // Card OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: a legacy key's
            // free frees its RSA, DSA, DH or EC_KEY, and an ENGINE's last
            // reference may drop with a key's or the digest's.
            classes: &["EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
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
            classes: &["EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "the last reference frees the EVP_PKEY, a legacy key's RSA, DSA, DH or \
                  EC_KEY, and may drop an ENGINE's last reference",
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
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "allocates the key it returns, freed on a failed read",
        },
    ),
    (
        "PEM_read_bio_X509",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "allocates the certificate it returns, freed on a failed read",
        },
    ),
    (
        "PKCS12_free",
        ExData::Runs {
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "frees the certificates a signed PKCS#7 safe holds, and their keys",
        },
    ),
    (
        "SSL_CTX_free",
        ExData::Runs {
            // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: a legacy key's
            // free frees its RSA, DSA, DH or EC_KEY, and an ENGINE's last
            // reference may drop with a key's or the client certificate
            // engine's.
            classes: &[
                "SSL_CTX",
                "X509_STORE",
                "X509",
                "EVP_PKEY",
                "SSL_SESSION",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
            how: "the last drop frees the context, its store, certificates, keys and cached \
                  sessions",
        },
    ),
    (
        "SSL_CTX_load_verify_locations",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
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
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "the default file lookup reads certificates, freeing each the store already holds",
        },
    ),
    (
        "SSL_CTX_use_PrivateKey",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "frees the key it replaces, and a certificate the new key does not match",
        },
    ),
    (
        "SSL_CTX_use_PrivateKey_file",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "reads a temporary key, freed after use, and replaces the context's key",
        },
    ),
    (
        "SSL_CTX_use_certificate",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "frees the certificate it replaces, and a key the new certificate does not match",
        },
    ),
    (
        "SSL_CTX_use_certificate_chain_file",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "reads certificates and frees the chain and certificate it replaces",
        },
    ),
    (
        "SSL_connect",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &[
                "SSL_SESSION",
                "X509",
                "EVP_PKEY",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
            how: "the handshake creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "SSL_do_handshake",
        ExData::Runs {
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &[
                "SSL_SESSION",
                "X509",
                "EVP_PKEY",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
            how: "the handshake creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "SSL_free",
        ExData::Runs {
            classes: &[
                "SSL",
                "SSL_SESSION",
                "X509",
                "BIO",
                "EVP_PKEY",
                "X509_STORE",
                "SSL_CTX",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
            how: "the last drop frees the connection, its session, peer certificate, BIOs and \
                  certificate stores, and may drop its context's last reference",
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
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &[
                "SSL_SESSION",
                "X509",
                "EVP_PKEY",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
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
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &[
                "SSL_SESSION",
                "X509",
                "EVP_PKEY",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
            how: "a handshake it runs creates and frees sessions, peer certificates and keys",
        },
    ),
    (
        "X509_STORE_free",
        ExData::Runs {
            classes: &[
                "X509_STORE",
                "X509",
                "EVP_PKEY",
                "RSA",
                "DSA",
                "DH",
                "EC_KEY",
                "ENGINE",
            ],
            how: "the last reference frees the store, the certificates it holds and their keys",
        },
    ),
    (
        "X509_free",
        ExData::Runs {
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
            how: "the last reference frees the certificate and its public key",
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
            // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a key it frees may
            // be a legacy key (its RSA, DSA, DH or EC_KEY) holding an ENGINE.
            classes: &["X509", "EVP_PKEY", "RSA", "DSA", "DH", "EC_KEY", "ENGINE"],
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
    // Card OPENSSL-REMAINING-HANDSHAKE-SETTERS. SSL_CTX_set_session_ticket_cb
    // (3.5): one setter, two slots (gen_cb "invoked when a session ticket is
    // about to be created", dec_cb "invoked after session ticket decryption
    // has been attempted"); the ticket application data accessors and the
    // two callback types are listed once.
    Slot {
        key: "ssl_ctx_st.ticket_gen_cb",
        page: "SSL_CTX_set_session_ticket_cb",
        writers: &["SSL_CTX_set_session_ticket_cb"],
        not_writers: &[
            "SSL_SESSION_get0_ticket_appdata",
            "SSL_SESSION_set1_ticket_appdata",
            "SSL_CTX_generate_session_ticket_fn",
            "SSL_CTX_decrypt_session_ticket_fn",
        ],
    },
    Slot {
        key: "ssl_ctx_st.ticket_dec_cb",
        page: "SSL_CTX_set_session_ticket_cb",
        writers: &["SSL_CTX_set_session_ticket_cb"],
        not_writers: &[],
    },
    // SSL_CTX_set_generate_session_id (3.5): the context's and the SSL's
    // session id generator, one key (the set is every value either holds);
    // the cache query and the callback type are not writers.
    Slot {
        key: "ssl_ctx_st.generate_session_id",
        page: "SSL_CTX_set_generate_session_id",
        writers: &[
            "SSL_CTX_set_generate_session_id",
            "SSL_set_generate_session_id",
        ],
        not_writers: &["SSL_has_matching_session_id", "GEN_SESSION_CB"],
    },
    // SSL_CTX_set_tmp_dh_callback (3.5): "the functions
    // SSL_CTX_set_tmp_dh_callback() or SSL_set_tmp_dh_callback() to set the
    // callback for the SSL_CTX or SSL object respectively": one key. The
    // automatic and explicit parameter setters keep parameters, not a
    // callback.
    Slot {
        key: "ssl_ctx_st.tmp_dh_callback",
        page: "SSL_CTX_set_tmp_dh_callback",
        writers: &["SSL_CTX_set_tmp_dh_callback", "SSL_set_tmp_dh_callback"],
        not_writers: &[
            "SSL_CTX_set_dh_auto",
            "SSL_set_dh_auto",
            "SSL_CTX_set0_tmp_dh_pkey",
            "SSL_set0_tmp_dh_pkey",
            "SSL_CTX_set_tmp_dh",
            "SSL_set_tmp_dh",
        ],
    },
    // SSL_CTX_set_stateless_cookie_generate_cb (3.5): four slots of the
    // context, one writer each (the stateless TLS 1.3 pair and the DTLS
    // pair).
    Slot {
        key: "ssl_ctx_st.gen_stateless_cookie_cb",
        page: "SSL_CTX_set_stateless_cookie_generate_cb",
        writers: &["SSL_CTX_set_stateless_cookie_generate_cb"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.verify_stateless_cookie_cb",
        page: "SSL_CTX_set_stateless_cookie_generate_cb",
        writers: &["SSL_CTX_set_stateless_cookie_verify_cb"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.app_gen_cookie_cb",
        page: "SSL_CTX_set_stateless_cookie_generate_cb",
        writers: &["SSL_CTX_set_cookie_generate_cb"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.app_verify_cookie_cb",
        page: "SSL_CTX_set_stateless_cookie_generate_cb",
        writers: &["SSL_CTX_set_cookie_verify_cb"],
        not_writers: &[],
    },
    // SSL_set_session_secret_cb (3.5): the SSL's one writer; no context
    // setter exists. The key names the SSL's slot.
    Slot {
        key: "ssl_st.session_secret_cb",
        page: "SSL_set_session_secret_cb",
        writers: &["SSL_set_session_secret_cb"],
        not_writers: &["tls_session_secret_cb_fn"],
    },
    // SSL_CTX_set_srp_password (3.5): three slots of the context, one writer
    // each; the parameter setters, the callback argument setter and the
    // getters are listed once.
    Slot {
        key: "ssl_ctx_st.srp_username_callback",
        page: "SSL_CTX_set_srp_password",
        writers: &["SSL_CTX_set_srp_username_callback"],
        not_writers: &[
            "SSL_CTX_set_srp_username",
            "SSL_CTX_set_srp_password",
            "SSL_CTX_set_srp_strength",
            "SSL_CTX_set_srp_cb_arg",
            "SSL_set_srp_server_param",
            "SSL_set_srp_server_param_pw",
            "SSL_get_srp_g",
            "SSL_get_srp_N",
            "SSL_get_srp_username",
            "SSL_get_srp_userinfo",
        ],
    },
    Slot {
        key: "ssl_ctx_st.srp_client_pwd_callback",
        page: "SSL_CTX_set_srp_password",
        writers: &["SSL_CTX_set_srp_client_pwd_callback"],
        not_writers: &[],
    },
    Slot {
        key: "ssl_ctx_st.srp_verify_param_callback",
        page: "SSL_CTX_set_srp_password",
        writers: &["SSL_CTX_set_srp_verify_param_callback"],
        not_writers: &[],
    },
    // SSL_set_async_callback (3.5): "All SSL objects generated based on this
    // SSL_CTX will get this callback": one key for the context's and the
    // SSL's. The `_arg` setters keep the argument, not the callback.
    Slot {
        key: "ssl_ctx_st.async_cb",
        page: "SSL_set_async_callback",
        writers: &["SSL_CTX_set_async_callback", "SSL_set_async_callback"],
        not_writers: &[
            "SSL_CTX_set_async_callback_arg",
            "SSL_set_async_callback_arg",
            "SSL_get_async_status",
            "SSL_async_callback_fn",
        ],
    },
    // SSL_CTX_set_new_pending_conn_cb (3.5): the context's one writer; the
    // page's callback type name (`SSL_set_new_pending_conn_cb_fn`, which the
    // pinned header spells `SSL_new_pending_conn_cb_fn`) is not a writer.
    Slot {
        key: "ssl_ctx_st.new_pending_conn_cb",
        page: "SSL_CTX_set_new_pending_conn_cb",
        writers: &["SSL_CTX_set_new_pending_conn_cb"],
        not_writers: &["SSL_set_new_pending_conn_cb_fn"],
    },
    // X509_STORE_set_verify_cb_func (3.5): twelve slots of a certificate
    // store, one `X509_STORE_set_*` writer each. "All the callbacks from a
    // X509_STORE are inherited by the corresponding X509_STORE_CTX structure
    // when it is initialized" (NOTES), and the handshake verifies the peer
    // through such a context over the SSL_CTX's store. The holder reaches the
    // connection only through other calls (SSL_CTX_new's own store,
    // SSL_CTX_set_cert_store); the key is run-wide, as for a BIO method. The
    // three aliases "available as macros" expand to their writers, so a call
    // through one is a call to a declared writer; the getters, the default
    // issuer lookup and the callback types are not writers.
    Slot {
        key: "x509_store_st.verify_cb",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_verify_cb"],
        not_writers: &[
            "X509_STORE_set_verify_cb_func",
            "X509_STORE_set_verify_func",
            "X509_STORE_set_lookup_crls_cb",
            "X509_STORE_get_verify_cb",
            "X509_STORE_CTX_get_verify",
            "X509_STORE_get_get_issuer",
            "X509_STORE_get_check_issued",
            "X509_STORE_get_check_revocation",
            "X509_STORE_get_get_crl",
            "X509_STORE_get_check_crl",
            "X509_STORE_get_cert_crl",
            "X509_STORE_get_check_policy",
            "X509_STORE_get_lookup_certs",
            "X509_STORE_get_lookup_crls",
            "X509_STORE_get_cleanup",
            "X509_STORE_CTX_get1_issuer",
            "X509_STORE_CTX_cert_crl_fn",
            "X509_STORE_CTX_check_crl_fn",
            "X509_STORE_CTX_check_issued_fn",
            "X509_STORE_CTX_check_policy_fn",
            "X509_STORE_CTX_check_revocation_fn",
            "X509_STORE_CTX_cleanup_fn",
            "X509_STORE_CTX_get_crl_fn",
            "X509_STORE_CTX_get_issuer_fn",
            "X509_STORE_CTX_lookup_certs_fn",
            "X509_STORE_CTX_lookup_crls_fn",
        ],
    },
    Slot {
        key: "x509_store_st.verify",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_verify"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.get_issuer",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_get_issuer"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.check_issued",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_check_issued"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.check_revocation",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_check_revocation"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.get_crl",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_get_crl"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.check_crl",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_check_crl"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.cert_crl",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_cert_crl"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.check_policy",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_check_policy"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.lookup_certs",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_lookup_certs"],
        not_writers: &[],
    },
    Slot {
        key: "x509_store_st.lookup_crls",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_lookup_crls"],
        not_writers: &[],
    },
    // Spelled after the page's `X509_STORE_CTX_cleanup_fn`: a bare `cleanup`
    // segment is a pre-pass token that makes the run read a unit it cannot
    // parse (x509_store_slots.qv says why).
    Slot {
        key: "x509_store_st.cleanup_fn",
        page: "X509_STORE_set_verify_cb_func",
        writers: &["X509_STORE_set_cleanup"],
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
    // Card OPENSSL-FREE-INDIRECT-CALLBACKS. X509_LOOKUP_meth_new (3.5): one
    // slot per function of a lookup method table, one setter each; the
    // table's type, constructor and destructor, the nine getters, the four
    // function pointer types and the two X509_OBJECT setters are listed once.
    // The holder is the X509_LOOKUP_METHOD a setter is handed; the library's
    // own methods are no setter's value (premise
    // `openssl_builtin_lookup_methods`). Each key is spelled after its setter
    // (`lookup_meth_free`), never the field (`free`): a key segment is a
    // member pre-pass token.
    Slot {
        key: "x509_lookup_method_st.lookup_meth_new_item",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_new_item"],
        not_writers: &[
            "X509_LOOKUP_METHOD",
            "X509_LOOKUP_meth_new",
            "X509_LOOKUP_meth_free",
            "X509_LOOKUP_meth_get_new_item",
            "X509_LOOKUP_meth_get_free",
            "X509_LOOKUP_meth_get_init",
            "X509_LOOKUP_meth_get_shutdown",
            "X509_LOOKUP_ctrl_fn",
            "X509_LOOKUP_meth_get_ctrl",
            "X509_LOOKUP_get_by_subject_fn",
            "X509_LOOKUP_meth_get_get_by_subject",
            "X509_LOOKUP_get_by_issuer_serial_fn",
            "X509_LOOKUP_meth_get_get_by_issuer_serial",
            "X509_LOOKUP_get_by_fingerprint_fn",
            "X509_LOOKUP_meth_get_get_by_fingerprint",
            "X509_LOOKUP_get_by_alias_fn",
            "X509_LOOKUP_meth_get_get_by_alias",
            "X509_OBJECT_set1_X509",
            "X509_OBJECT_set1_X509_CRL",
        ],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_free",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_free"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_init",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_init"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_shutdown",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_shutdown"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_ctrl",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_ctrl"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_get_by_subject",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_get_by_subject"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_get_by_issuer_serial",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_get_by_issuer_serial"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_get_by_fingerprint",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_get_by_fingerprint"],
        not_writers: &[],
    },
    Slot {
        key: "x509_lookup_method_st.lookup_meth_get_by_alias",
        page: "X509_LOOKUP_meth_new",
        writers: &["X509_LOOKUP_meth_set_get_by_alias"],
        not_writers: &[],
    },
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the teardown slots the
    // frees run on the objects they free in turn (teardown_slots.qv). Each
    // key is spelled after its setter, never a bare field word (`finish`,
    // `cleanup`, `destroy`): a key segment is a member pre-pass token. The
    // holder is the method table or engine a setter is handed; the tables
    // and engines the library defines itself are no setter's value (premise
    // `openssl_builtin_teardown_functions`).
    // RSA_meth_new (3.5): the finish function "will be called in response to the
    // application calling RSA_free()" (`r->meth->finish(r)`,
    // crypto/rsa/rsa_lib.c:149-150 at 3.5.5). Only the finish slot runs at a
    // free; the page's other names write slots no declaration invokes and are
    // listed as not-writers.
    Slot {
        key: "rsa_meth_st.rsa_meth_finish",
        page: "RSA_meth_new",
        writers: &["RSA_meth_set_finish"],
        not_writers: &[
            "RSA_meth_get0_app_data",
            "RSA_meth_set0_app_data",
            "RSA_meth_new",
            "RSA_meth_free",
            "RSA_meth_dup",
            "RSA_meth_get0_name",
            "RSA_meth_set1_name",
            "RSA_meth_get_flags",
            "RSA_meth_set_flags",
            "RSA_meth_get_pub_enc",
            "RSA_meth_set_pub_enc",
            "RSA_meth_get_pub_dec",
            "RSA_meth_set_pub_dec",
            "RSA_meth_get_priv_enc",
            "RSA_meth_set_priv_enc",
            "RSA_meth_get_priv_dec",
            "RSA_meth_set_priv_dec",
            "RSA_meth_get_mod_exp",
            "RSA_meth_set_mod_exp",
            "RSA_meth_get_bn_mod_exp",
            "RSA_meth_set_bn_mod_exp",
            "RSA_meth_get_init",
            "RSA_meth_set_init",
            "RSA_meth_get_finish",
            "RSA_meth_get_sign",
            "RSA_meth_set_sign",
            "RSA_meth_get_verify",
            "RSA_meth_set_verify",
            "RSA_meth_get_keygen",
            "RSA_meth_set_keygen",
            "RSA_meth_get_multi_prime_keygen",
            "RSA_meth_set_multi_prime_keygen",
        ],
    },
    // DSA_meth_new (3.5): the finish function, "called in response to the
    // application calling DSA_free()" (`r->meth->finish(r)`,
    // crypto/dsa/dsa_lib.c:225-226 at 3.5.5).
    Slot {
        key: "dsa_method.dsa_meth_finish",
        page: "DSA_meth_new",
        writers: &["DSA_meth_set_finish"],
        not_writers: &[
            "DSA_meth_new",
            "DSA_meth_free",
            "DSA_meth_dup",
            "DSA_meth_get0_name",
            "DSA_meth_set1_name",
            "DSA_meth_get_flags",
            "DSA_meth_set_flags",
            "DSA_meth_get0_app_data",
            "DSA_meth_set0_app_data",
            "DSA_meth_get_sign",
            "DSA_meth_set_sign",
            "DSA_meth_get_sign_setup",
            "DSA_meth_set_sign_setup",
            "DSA_meth_get_verify",
            "DSA_meth_set_verify",
            "DSA_meth_get_mod_exp",
            "DSA_meth_set_mod_exp",
            "DSA_meth_get_bn_mod_exp",
            "DSA_meth_set_bn_mod_exp",
            "DSA_meth_get_init",
            "DSA_meth_set_init",
            "DSA_meth_get_finish",
            "DSA_meth_get_paramgen",
            "DSA_meth_set_paramgen",
            "DSA_meth_get_keygen",
            "DSA_meth_set_keygen",
        ],
    },
    // DH_meth_new (3.5): the finish function, run by DH_free
    // (`r->meth->finish(r)`, crypto/dh/dh_lib.c:149-150 at 3.5.5).
    Slot {
        key: "dh_method.dh_meth_finish",
        page: "DH_meth_new",
        writers: &["DH_meth_set_finish"],
        not_writers: &[
            "DH_meth_new",
            "DH_meth_free",
            "DH_meth_dup",
            "DH_meth_get0_name",
            "DH_meth_set1_name",
            "DH_meth_get_flags",
            "DH_meth_set_flags",
            "DH_meth_get0_app_data",
            "DH_meth_set0_app_data",
            "DH_meth_get_generate_key",
            "DH_meth_set_generate_key",
            "DH_meth_get_compute_key",
            "DH_meth_set_compute_key",
            "DH_meth_get_bn_mod_exp",
            "DH_meth_set_bn_mod_exp",
            "DH_meth_get_init",
            "DH_meth_set_init",
            "DH_meth_get_finish",
            "DH_meth_get_generate_params",
            "DH_meth_set_generate_params",
        ],
    },
    // ENGINE_add (3.5): an engine's finish function runs when its last
    // functional reference is released (`e->finish(e)`,
    // crypto/engine/eng_init.c:66-69 at 3.5.5) and its destroy function at its
    // last structural reference (`e->destroy(e)`,
    // crypto/engine/eng_lib.c:95-96). Every other name on the page writes no
    // slot a declaration invokes.
    Slot {
        key: "engine_st.engine_finish",
        page: "ENGINE_add",
        writers: &["ENGINE_set_finish_function"],
        not_writers: &[
            "ENGINE_get_DH",
            "ENGINE_get_DSA",
            "ENGINE_by_id",
            "ENGINE_get_cipher_engine",
            "ENGINE_get_default_DH",
            "ENGINE_get_default_DSA",
            "ENGINE_get_default_RAND",
            "ENGINE_get_default_RSA",
            "ENGINE_get_digest_engine",
            "ENGINE_get_first",
            "ENGINE_get_last",
            "ENGINE_get_next",
            "ENGINE_get_prev",
            "ENGINE_new",
            "ENGINE_get_ciphers",
            "ENGINE_get_ctrl_function",
            "ENGINE_get_digests",
            "ENGINE_get_destroy_function",
            "ENGINE_get_finish_function",
            "ENGINE_get_init_function",
            "ENGINE_get_load_privkey_function",
            "ENGINE_get_load_pubkey_function",
            "ENGINE_load_private_key",
            "ENGINE_load_public_key",
            "ENGINE_get_RAND",
            "ENGINE_get_RSA",
            "ENGINE_get_id",
            "ENGINE_get_name",
            "ENGINE_get_cmd_defns",
            "ENGINE_get_cipher",
            "ENGINE_get_digest",
            "ENGINE_add",
            "ENGINE_cmd_is_executable",
            "ENGINE_ctrl",
            "ENGINE_ctrl_cmd",
            "ENGINE_ctrl_cmd_string",
            "ENGINE_finish",
            "ENGINE_free",
            "ENGINE_get_flags",
            "ENGINE_init",
            "ENGINE_register_DH",
            "ENGINE_register_DSA",
            "ENGINE_register_RAND",
            "ENGINE_register_RSA",
            "ENGINE_register_all_complete",
            "ENGINE_register_ciphers",
            "ENGINE_register_complete",
            "ENGINE_register_digests",
            "ENGINE_remove",
            "ENGINE_set_DH",
            "ENGINE_set_DSA",
            "ENGINE_set_RAND",
            "ENGINE_set_RSA",
            "ENGINE_set_ciphers",
            "ENGINE_set_cmd_defns",
            "ENGINE_set_ctrl_function",
            "ENGINE_set_default",
            "ENGINE_set_default_DH",
            "ENGINE_set_default_DSA",
            "ENGINE_set_default_RAND",
            "ENGINE_set_default_RSA",
            "ENGINE_set_default_ciphers",
            "ENGINE_set_default_digests",
            "ENGINE_set_default_string",
            "ENGINE_set_digests",
            "ENGINE_set_flags",
            "ENGINE_set_id",
            "ENGINE_set_init_function",
            "ENGINE_set_load_privkey_function",
            "ENGINE_set_load_pubkey_function",
            "ENGINE_set_name",
            "ENGINE_up_ref",
            "ENGINE_get_table_flags",
            "ENGINE_cleanup",
            "ENGINE_load_builtin_engines",
            "ENGINE_register_all_DH",
            "ENGINE_register_all_DSA",
            "ENGINE_register_all_RAND",
            "ENGINE_register_all_RSA",
            "ENGINE_register_all_ciphers",
            "ENGINE_register_all_digests",
            "ENGINE_set_table_flags",
            "ENGINE_unregister_DH",
            "ENGINE_unregister_DSA",
            "ENGINE_unregister_RAND",
            "ENGINE_unregister_RSA",
            "ENGINE_unregister_ciphers",
            "ENGINE_unregister_digests",
        ],
    },
    Slot {
        key: "engine_st.engine_destroy",
        page: "ENGINE_add",
        writers: &["ENGINE_set_destroy_function"],
        not_writers: &[],
    },
    // EVP_PKEY_ASN1_METHOD (3.5): "The pkey_free() method helps freeing the
    // internals of pkey", run when a legacy EVP_PKEY whose method this is is
    // freed (evp_pkey_free_legacy, crypto/evp/p_lib.c at 3.5.5).
    Slot {
        key: "evp_pkey_asn1_method_st.asn1_meth_pkey_free",
        page: "EVP_PKEY_ASN1_METHOD",
        writers: &["EVP_PKEY_asn1_set_free"],
        not_writers: &[
            "EVP_PKEY_ASN1_METHOD",
            "EVP_PKEY_asn1_new",
            "EVP_PKEY_asn1_copy",
            "EVP_PKEY_asn1_free",
            "EVP_PKEY_asn1_add0",
            "EVP_PKEY_asn1_add_alias",
            "EVP_PKEY_asn1_set_public",
            "EVP_PKEY_asn1_set_private",
            "EVP_PKEY_asn1_set_param",
            "EVP_PKEY_asn1_set_ctrl",
            "EVP_PKEY_asn1_set_item",
            "EVP_PKEY_asn1_set_siginf",
            "EVP_PKEY_asn1_set_check",
            "EVP_PKEY_asn1_set_public_check",
            "EVP_PKEY_asn1_set_param_check",
            "EVP_PKEY_asn1_set_security_bits",
            "EVP_PKEY_asn1_set_set_priv_key",
            "EVP_PKEY_asn1_set_set_pub_key",
            "EVP_PKEY_asn1_set_get_priv_key",
            "EVP_PKEY_asn1_set_get_pub_key",
            "EVP_PKEY_get0_asn1",
        ],
    },
    // ASYNC_WAIT_CTX_new (3.5): the cleanup routine "will automatically get
    // called when the ASYNC_WAIT_CTX is freed" (`curr->cleanup(ctx, curr->key,
    // curr->fd, curr->custom_data)`, crypto/async/async_wait.c:32-33 at 3.5.5).
    Slot {
        key: "async_wait_ctx_st.wait_fd_cleanup",
        page: "ASYNC_WAIT_CTX_new",
        writers: &["ASYNC_WAIT_CTX_set_wait_fd"],
        not_writers: &[
            "ASYNC_WAIT_CTX_new",
            "ASYNC_WAIT_CTX_free",
            "ASYNC_WAIT_CTX_get_fd",
            "ASYNC_WAIT_CTX_get_all_fds",
            "ASYNC_WAIT_CTX_get_changed_fds",
            "ASYNC_WAIT_CTX_clear_fd",
            "ASYNC_WAIT_CTX_set_callback",
            "ASYNC_WAIT_CTX_get_callback",
            "ASYNC_WAIT_CTX_set_status",
            "ASYNC_WAIT_CTX_get_status",
            "ASYNC_callback_fn",
            "ASYNC_STATUS_UNSUPPORTED",
            "ASYNC_STATUS_ERR",
            "ASYNC_STATUS_OK",
            "ASYNC_STATUS_EAGAIN",
        ],
    },
    // EVP_MD_meth_new (3.5): "This cleanup function is called by
    // EVP_MD_CTX_reset() and EVP_MD_CTX_free()" (`ctx->digest->cleanup(ctx)`,
    // crypto/evp/digest.c:32-34 at 3.5.5).
    Slot {
        key: "evp_md_st.md_meth_cleanup",
        page: "EVP_MD_meth_new",
        writers: &["EVP_MD_meth_set_cleanup"],
        not_writers: &[
            "EVP_MD_meth_new",
            "EVP_MD_meth_dup",
            "EVP_MD_meth_free",
            "EVP_MD_meth_set_input_blocksize",
            "EVP_MD_meth_set_result_size",
            "EVP_MD_meth_set_app_datasize",
            "EVP_MD_meth_set_flags",
            "EVP_MD_meth_set_init",
            "EVP_MD_meth_set_update",
            "EVP_MD_meth_set_final",
            "EVP_MD_meth_set_copy",
            "EVP_MD_meth_set_ctrl",
            "EVP_MD_meth_get_input_blocksize",
            "EVP_MD_meth_get_result_size",
            "EVP_MD_meth_get_app_datasize",
            "EVP_MD_meth_get_flags",
            "EVP_MD_meth_get_init",
            "EVP_MD_meth_get_update",
            "EVP_MD_meth_get_final",
            "EVP_MD_meth_get_copy",
            "EVP_MD_meth_get_cleanup",
            "EVP_MD_meth_get_ctrl",
        ],
    },
    // EVP_CIPHER_meth_new (3.5): the cleanup function "to do extra cleanup
    // before the method's private data structure is cleaned out", run when a
    // context using a legacy cipher is reset or freed (EVP_CIPHER_CTX_reset,
    // `ctx->cipher->cleanup(ctx)`, crypto/evp/evp_enc.c:57 at 3.5.5). Found by
    // this card's census: the record layer's cipher contexts and a MAC
    // context's cipher context are freed in turn.
    Slot {
        key: "evp_cipher_st.cipher_meth_cleanup",
        page: "EVP_CIPHER_meth_new",
        writers: &["EVP_CIPHER_meth_set_cleanup"],
        not_writers: &[
            "EVP_CIPHER_meth_new",
            "EVP_CIPHER_meth_dup",
            "EVP_CIPHER_meth_free",
            "EVP_CIPHER_meth_set_iv_length",
            "EVP_CIPHER_meth_set_flags",
            "EVP_CIPHER_meth_set_impl_ctx_size",
            "EVP_CIPHER_meth_set_init",
            "EVP_CIPHER_meth_set_do_cipher",
            "EVP_CIPHER_meth_set_set_asn1_params",
            "EVP_CIPHER_meth_set_get_asn1_params",
            "EVP_CIPHER_meth_set_ctrl",
            "EVP_CIPHER_meth_get_init",
            "EVP_CIPHER_meth_get_do_cipher",
            "EVP_CIPHER_meth_get_cleanup",
            "EVP_CIPHER_meth_get_set_asn1_params",
            "EVP_CIPHER_meth_get_get_asn1_params",
            "EVP_CIPHER_meth_get_ctrl",
        ],
    },
    // EVP_PKEY_meth_new (3.5): the cleanup function, run by EVP_PKEY_CTX_free
    // (`ctx->pmeth->cleanup(ctx)`, crypto/evp/pmeth_lib.c:394-395 at 3.5.5).
    Slot {
        key: "evp_pkey_method_st.pkey_meth_cleanup",
        page: "EVP_PKEY_meth_new",
        writers: &["EVP_PKEY_meth_set_cleanup"],
        not_writers: &[
            "EVP_PKEY_meth_new",
            "EVP_PKEY_meth_free",
            "EVP_PKEY_meth_copy",
            "EVP_PKEY_meth_find",
            "EVP_PKEY_meth_add0",
            "EVP_PKEY_METHOD",
            "EVP_PKEY_meth_set_init",
            "EVP_PKEY_meth_set_copy",
            "EVP_PKEY_meth_set_paramgen",
            "EVP_PKEY_meth_set_keygen",
            "EVP_PKEY_meth_set_sign",
            "EVP_PKEY_meth_set_verify",
            "EVP_PKEY_meth_set_verify_recover",
            "EVP_PKEY_meth_set_signctx",
            "EVP_PKEY_meth_set_verifyctx",
            "EVP_PKEY_meth_set_encrypt",
            "EVP_PKEY_meth_set_decrypt",
            "EVP_PKEY_meth_set_derive",
            "EVP_PKEY_meth_set_ctrl",
            "EVP_PKEY_meth_set_digestsign",
            "EVP_PKEY_meth_set_digestverify",
            "EVP_PKEY_meth_set_check",
            "EVP_PKEY_meth_set_public_check",
            "EVP_PKEY_meth_set_param_check",
            "EVP_PKEY_meth_set_digest_custom",
            "EVP_PKEY_meth_get_init",
            "EVP_PKEY_meth_get_copy",
            "EVP_PKEY_meth_get_cleanup",
            "EVP_PKEY_meth_get_paramgen",
            "EVP_PKEY_meth_get_keygen",
            "EVP_PKEY_meth_get_sign",
            "EVP_PKEY_meth_get_verify",
            "EVP_PKEY_meth_get_verify_recover",
            "EVP_PKEY_meth_get_signctx",
            "EVP_PKEY_meth_get_verifyctx",
            "EVP_PKEY_meth_get_encrypt",
            "EVP_PKEY_meth_get_decrypt",
            "EVP_PKEY_meth_get_derive",
            "EVP_PKEY_meth_get_ctrl",
            "EVP_PKEY_meth_get_digestsign",
            "EVP_PKEY_meth_get_digestverify",
            "EVP_PKEY_meth_get_check",
            "EVP_PKEY_meth_get_public_check",
            "EVP_PKEY_meth_get_param_check",
            "EVP_PKEY_meth_get_digest_custom",
            "EVP_PKEY_meth_remove",
        ],
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
    // Card OPENSSL-REMAINING-HANDSHAKE-SETTERS. SSL_set_session_secret_cb
    // (3.5): `session_secret_cb(s, secret, secret_len, peer_ciphers, cipher,
    // arg)`. No other new page states a callback argument NULL (the session
    // ticket page's "Either callback function may be set to NULL" is about
    // the setter's arguments, and its dec_cb `ss` is described as unusable,
    // not NULL).
    NullableArgument {
        slot: "ssl_st.session_secret_cb",
        position: 3,
        sentence: "On the client side the peer_ciphers stack will always be NULL.",
    },
];

/// Whether `slot`'s argument at `position` may be NULL by its page
/// ([`NULLABLE_ARGUMENTS`]).
#[must_use]
pub fn nullable_argument(slot: &str, position: usize) -> bool {
    NULLABLE_ARGUMENTS
        .iter()
        .any(|nullable| nullable.slot == slot && nullable.position == position)
        // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: or by the pinned
        // source, for a source-derived slot.
        || SOURCE_NULLABLE_ARGUMENTS
            .iter()
            .any(|nullable| nullable.slot == slot && nullable.position == position)
}

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS** (user decision
/// 2026-11-21, "Admit pinned source"): the series of an entry that no 3.5
/// manual page documents and that the pinned OpenSSL 3.5.5 release source
/// specifies instead. Such an entry's `page` is the source file defining the
/// function, its range is `since "3.5.5"` (the only release read), and it is
/// cited in [`SOURCE_CITATIONS`]; it is source-derived, never documented.
pub const SOURCE_SERIES: &str = "source 3.5.5";

/// The sha256 of the pinned release tarball the source citations read
/// (`openssl-3.5.5.tar.gz`, as published by openssl.org).
pub const SOURCE_TARBALL_SHA256: &str =
    "b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89";

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: where the pinned
/// source defines a source-derived function, and what its body does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCitation {
    pub function: &'static str,
    /// The defining file, relative to the release's root.
    pub file: &'static str,
    /// The line of the definition's first line.
    pub line: u32,
    /// What the body does, as read.
    pub reading: &'static str,
}

/// The source-derived functions (user decision 2026-11-21), each read at
/// OpenSSL 3.5.5 ([`SOURCE_TARBALL_SHA256`]).
pub const SOURCE_CITATIONS: &[SourceCitation] = &[
    SourceCitation {
        function: "EC_KEY_METHOD_set_init",
        file: "crypto/ec/ec_kmeth.c",
        line: 194,
        reading: "stores init, finish, copy, set_group, set_private and set_public into the \
                  method (lines 204-209) and returns nothing; EC_KEY_free runs the finish \
                  function (crypto/ec/ec_key.c:84-85)",
    },
    SourceCitation {
        function: "ENGINE_set_pkey_meths",
        file: "crypto/engine/tb_pkmeth.c",
        line: 92,
        reading: "stores the function into the engine (line 94) and returns 1; the last \
                  structural reference calls it to enumerate the methods to free \
                  (engine_pkey_meths_free, lines 110 and 112)",
    },
    SourceCitation {
        function: "ENGINE_set_pkey_asn1_meths",
        file: "crypto/engine/tb_asnmth.c",
        line: 105,
        reading: "stores the function into the engine (line 107) and returns 1; the last \
                  structural reference calls it to enumerate the methods to free \
                  (engine_pkey_asn1_meths_free, lines 123 and 125)",
    },
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: a callback slot whose
/// writers no 3.5 manual page names (so [`Slot`]'s NAME accounting cannot
/// apply). Its writers are every function of the pinned source that stores
/// into the field, each declared with a source-derived entry; `invoked` cites
/// where the source runs it.
#[derive(Clone, Copy, Debug)]
pub struct SourceSlot {
    pub key: &'static str,
    pub writers: &'static [&'static str],
    pub invoked: &'static str,
}

/// The source-derived slots. A search of the 3.5.5 source for stores into
/// each field (`->pkey_meths =`, `->pkey_asn1_meths =`, `->finish =` of an
/// `EC_KEY_METHOD` and the other five) finds only these writers, beside
/// `EC_KEY_METHOD_new`'s copy of a method (the library's own or one the
/// writers built) and `ENGINE_cpy` in the dynamic engine.
pub const SOURCE_SLOTS: &[SourceSlot] = &[
    SourceSlot {
        key: "ec_key_method_st.ec_key_meth_init",
        writers: &["EC_KEY_METHOD_set_init"],
        invoked: "crypto/ec/ec_key.c (EC_KEY_new_ex's init); no declaration invokes it",
    },
    SourceSlot {
        key: "ec_key_method_st.ec_key_meth_finish",
        writers: &["EC_KEY_METHOD_set_init"],
        invoked: "crypto/ec/ec_key.c:84-85 (EC_KEY_free)",
    },
    SourceSlot {
        key: "ec_key_method_st.ec_key_meth_copy",
        writers: &["EC_KEY_METHOD_set_init"],
        invoked: "crypto/ec/ec_key.c (EC_KEY_copy); no declaration invokes it",
    },
    SourceSlot {
        key: "ec_key_method_st.ec_key_meth_set_group",
        writers: &["EC_KEY_METHOD_set_init"],
        invoked: "crypto/ec/ec_key.c (EC_KEY_set_group); no declaration invokes it",
    },
    SourceSlot {
        key: "ec_key_method_st.ec_key_meth_set_private",
        writers: &["EC_KEY_METHOD_set_init"],
        invoked: "crypto/ec/ec_key.c (EC_KEY_set_private_key); no declaration invokes it",
    },
    SourceSlot {
        key: "ec_key_method_st.ec_key_meth_set_public",
        writers: &["EC_KEY_METHOD_set_init"],
        invoked: "crypto/ec/ec_key.c (EC_KEY_set_public_key); no declaration invokes it",
    },
    SourceSlot {
        key: "engine_st.engine_pkey_meths",
        writers: &["ENGINE_set_pkey_meths"],
        invoked: "crypto/engine/tb_pkmeth.c:110,112 (engine_pkey_meths_free)",
    },
    SourceSlot {
        key: "engine_st.engine_pkey_asn1_meths",
        writers: &["ENGINE_set_pkey_asn1_meths"],
        invoked: "crypto/engine/tb_asnmth.c:123,125 (engine_pkey_asn1_meths_free)",
    },
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: an argument the pinned
/// source passes as NULL to a source-derived slot's callback, the source-side
/// counterpart of [`NULLABLE_ARGUMENTS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceNullableArgument {
    pub slot: &'static str,
    pub position: usize,
    pub citation: &'static str,
}

/// The engine's method enumerators are called first with a NULL method
/// pointer (to read the nids), then with a NULL nids pointer (to fetch each
/// method).
pub const SOURCE_NULLABLE_ARGUMENTS: &[SourceNullableArgument] = &[
    SourceNullableArgument {
        slot: "engine_st.engine_pkey_meths",
        position: 1,
        citation: "crypto/engine/tb_pkmeth.c:110 `e->pkey_meths(e, NULL, &pknids, 0)`",
    },
    SourceNullableArgument {
        slot: "engine_st.engine_pkey_meths",
        position: 2,
        citation: "crypto/engine/tb_pkmeth.c:112 `e->pkey_meths(e, &pkm, NULL, pknids[i])`",
    },
    SourceNullableArgument {
        slot: "engine_st.engine_pkey_asn1_meths",
        position: 1,
        citation: "crypto/engine/tb_asnmth.c:123 `e->pkey_asn1_meths(e, NULL, &pknids, 0)`",
    },
    SourceNullableArgument {
        slot: "engine_st.engine_pkey_asn1_meths",
        position: 2,
        citation: "crypto/engine/tb_asnmth.c:125 `e->pkey_asn1_meths(e, &pkm, NULL, pknids[i])`",
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
const SINCE_0_9_7: &str = "resource_library_version(openssl, since \"0.9.7\")";
const SINCE_1_1_1: &str = "resource_library_version(openssl, since \"1.1.1\")";
const SINCE_3_2_0: &str = "resource_library_version(openssl, since \"3.2.0\")";
// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS.
const SINCE_1_0_1: &str = "resource_library_version(openssl, since \"1.0.1\")";
const SINCE_3_5_0: &str = "resource_library_version(openssl, since \"3.5.0\")";
// Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: a source-derived entry
// ([`SOURCE_SERIES`]) admits the release whose source was read.
const SINCE_3_5_5: &str = "resource_library_version(openssl, since \"3.5.5\")";
const DOC_3_2: &str = "resource_library_version(openssl, documented \"3.2\")";

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

/// Card **OPENSSL-REMAINING-HANDSHAKE-SETTERS**: a certificate store setter
/// (`X509_STORE_set_verify_cb_func` 3.5 HISTORY: the `X509_STORE_set_*`
/// setters "were added in OpenSSL 1.1.0"; `X509_STORE_set_verify_cb` is named
/// both there and as "added in OpenSSL 1.0.0", and the later bound is taken),
/// registering the function pointer at position 1 into `slot` of the store
/// at position 0.
const fn store(function: &'static str, slot: &'static str) -> DocumentedFunction {
    entry(
        function,
        "X509_STORE_set_verify_cb_func",
        "3.5",
        SINCE_1_1_0,
        registers(1, slot),
    )
}

/// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: a lookup method table setter
/// (`X509_LOOKUP_meth_new` HISTORY at 1.1.1, 3.0 and 3.5: "added in OpenSSL
/// 1.1.0i", a letter release the version vocabulary cannot spell, so the
/// range is the oldest series documenting them, 1.1.1), registering the
/// function pointer at position 1 into `slot` of the table at position 0.
const fn lookup(function: &'static str, slot: &'static str) -> DocumentedFunction {
    entry(
        function,
        "X509_LOOKUP_meth_new",
        "1.1.1",
        DOC_1_1_1,
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

/// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: the session ticket callbacks,
/// `gen_cb(s, arg)` and `dec_cb(s, ss, keyname, keyname_len, status, arg)`
/// ("the SSL_SESSION ss has not yet been assigned to the SSL s").
const TICKET_GEN: Premise = on_ssl("ssl_ctx_st.ticket_gen_cb", &[Some(0), None]);
const TICKET_DEC: Premise = on_ssl(
    "ssl_ctx_st.ticket_dec_cb",
    &[Some(0), None, None, None, None, None],
);
/// The session id generator, `GEN_SESSION_CB(ssl, id, id_len)`.
const GENERATE_SESSION_ID: Premise =
    on_ssl("ssl_ctx_st.generate_session_id", &[Some(0), None, None]);
/// The DH parameter callback, `tmp_dh_callback(ssl, is_export, keylength)`:
/// "The SSL object for the current connection is supplied as an argument."
const TMP_DH: Premise = on_ssl("ssl_ctx_st.tmp_dh_callback", &[Some(0), None, None]);
/// The cookie callbacks, each `(ssl, cookie, cookie_len)`.
const GEN_STATELESS_COOKIE: Premise =
    on_ssl("ssl_ctx_st.gen_stateless_cookie_cb", &[Some(0), None, None]);
const VERIFY_STATELESS_COOKIE: Premise = on_ssl(
    "ssl_ctx_st.verify_stateless_cookie_cb",
    &[Some(0), None, None],
);
const APP_GEN_COOKIE: Premise = on_ssl("ssl_ctx_st.app_gen_cookie_cb", &[Some(0), None, None]);
const APP_VERIFY_COOKIE: Premise =
    on_ssl("ssl_ctx_st.app_verify_cookie_cb", &[Some(0), None, None]);
/// The session secret callback, `session_secret_cb(s, secret, secret_len,
/// peer_ciphers, cipher, arg)` (`peer_ciphers` NULL on a client:
/// [`NULLABLE_ARGUMENTS`]).
const SESSION_SECRET: Premise = on_ssl(
    "ssl_st.session_secret_cb",
    &[Some(0), None, None, None, None, None],
);
/// The SRP callbacks: "The callback parameters are the SSL connection s, a
/// writable error flag ad and the extra argument arg" (username), and "the
/// SSL connection s and the extra argument arg" (client password, verify
/// param).
const SRP_USERNAME: Premise = on_ssl("ssl_ctx_st.srp_username_callback", &[Some(0), None, None]);
const SRP_CLIENT_PWD: Premise = on_ssl("ssl_ctx_st.srp_client_pwd_callback", &[Some(0), None]);
const SRP_VERIFY_PARAM: Premise = on_ssl("ssl_ctx_st.srp_verify_param_callback", &[Some(0), None]);
/// The async callback, `SSL_async_callback_fn(s, arg)`: run during an
/// asynchronous SSL call when a retry occurs (the page's step 4).
const ASYNC: Premise = on_ssl("ssl_ctx_st.async_cb", &[Some(0), None]);
/// The QUIC pending connection callback, `(c, new_ssl, arg)`: the listener's
/// context and a connection the QUIC stack preallocated, neither the call's
/// connection.
const NEW_PENDING_CONN: Premise = on_ssl("ssl_ctx_st.new_pending_conn_cb", &[None, None, None]);
/// The certificate store's callbacks, run by the verification of the peer's
/// chain through an `X509_STORE_CTX` the library makes: every actual is the
/// library's (`verify_cb(ok, ctx)`, `verify(ctx)`, `get_issuer(issuer, ctx,
/// x)`, `check_issued(ctx, x, issuer)`, `check_revocation(ctx)`,
/// `get_crl(ctx, crl, x)`, `check_crl(ctx, crl)`, `cert_crl(ctx, crl, x)`,
/// `check_policy(ctx)`, `lookup_certs(ctx, nm)`, `lookup_crls(ctx, nm)`,
/// `cleanup(ctx)`).
const STORE_SLOTS: [Premise; 12] = [
    on_ssl("x509_store_st.verify_cb", &[None; 2]),
    on_ssl("x509_store_st.verify", &[None; 1]),
    on_ssl("x509_store_st.get_issuer", &[None; 3]),
    on_ssl("x509_store_st.check_issued", &[None; 3]),
    on_ssl("x509_store_st.check_revocation", &[None; 1]),
    on_ssl("x509_store_st.get_crl", &[None; 3]),
    on_ssl("x509_store_st.check_crl", &[None; 2]),
    on_ssl("x509_store_st.cert_crl", &[None; 3]),
    on_ssl("x509_store_st.check_policy", &[None; 1]),
    on_ssl("x509_store_st.lookup_certs", &[None; 2]),
    on_ssl("x509_store_st.lookup_crls", &[None; 2]),
    on_ssl("x509_store_st.cleanup_fn", &[None; 1]),
];

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
/// early data and custom extension slots. Card
/// OPENSSL-REMAINING-HANDSHAKE-SETTERS: and the session ticket, session id,
/// DH parameter, cookie, session secret, SRP, async and QUIC pending
/// connection slots, and the twelve slots of a certificate store.
const HANDSHAKE: &[Premise] = &[
    Premise::UnstatedSlots,
    // Card OPENSSL-EX-DATA-REGISTRY.
    Premise::ExDataCallbacks,
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the teardown slots of
    // the keys and the digest, cipher and public key contexts the call frees
    // inside, which the handshake statement's free conjunct no longer covers
    // once their setters are declared.
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
    CONTEXT_FREED[0],
    CONTEXT_FREED[1],
    CONTEXT_FREED[2],
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
    // Card OPENSSL-REMAINING-HANDSHAKE-SETTERS.
    TICKET_GEN,
    TICKET_DEC,
    GENERATE_SESSION_ID,
    TMP_DH,
    GEN_STATELESS_COOKIE,
    VERIFY_STATELESS_COOKIE,
    APP_GEN_COOKIE,
    APP_VERIFY_COOKIE,
    SESSION_SECRET,
    SRP_USERNAME,
    SRP_CLIENT_PWD,
    SRP_VERIFY_PARAM,
    ASYNC,
    NEW_PENDING_CONN,
    STORE_SLOTS[0],
    STORE_SLOTS[1],
    STORE_SLOTS[2],
    STORE_SLOTS[3],
    STORE_SLOTS[4],
    STORE_SLOTS[5],
    STORE_SLOTS[6],
    STORE_SLOTS[7],
    STORE_SLOTS[8],
    STORE_SLOTS[9],
    STORE_SLOTS[10],
    STORE_SLOTS[11],
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
/// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: and the async callback, which
/// an asynchronous SSL call may run ("Application sets SSL_MODE_ASYNC and
/// makes an asynchronous SSL call", SSL_set_async_callback 3.5; SSL_shutdown
/// is one).
const SHUTDOWN: &[Premise] = &[
    Premise::UnstatedSlots,
    // Card OPENSSL-EX-DATA-REGISTRY.
    Premise::ExDataCallbacks,
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the teardown slots of
    // the keys and the digest, cipher and public key contexts the call frees
    // inside, which the handshake statement's free conjunct no longer covers
    // once their setters are declared.
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
    CONTEXT_FREED[0],
    CONTEXT_FREED[1],
    CONTEXT_FREED[2],
    INFO,
    MESSAGE,
    RECORD_PADDING,
    ASYNC,
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
/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: `SSL_free` also
/// states the teardown slots of the keys, engines, digest, cipher and public
/// key contexts and async wait context it frees in turn, and the message
/// and info callbacks, which the record layer's clear can run on its error
/// path (`ossl_statem_fatal`, then `ssl3_send_alert`, `s3_msg.c` 3.5.5, the
/// QUIC trace of the report).
const SSL_FREED: &[Premise] = &[
    Premise::UnstatedFreeSlots,
    Premise::BioFreeCallbacks,
    Premise::ExDataCallbacks,
    REMOVE_SESSION,
    MESSAGE,
    INFO,
    BIO_SLOTS[0],
    BIO_SLOTS[1],
    BIO_SLOTS[2],
    BIO_SLOTS[11],
    LOOKUP_FREED[0],
    LOOKUP_FREED[1],
    LOOKUP_FREED[2],
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
    CONTEXT_FREED[0],
    CONTEXT_FREED[1],
    CONTEXT_FREED[2],
    ASYNC_FREED,
];
const SSL_CTX_FREED: &[Premise] = &[
    Premise::UnstatedFreeSlots,
    Premise::ExDataCallbacks,
    REMOVE_SESSION,
    LOOKUP_FREED[0],
    LOOKUP_FREED[1],
    LOOKUP_FREED[2],
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
];

/// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: the last drop of a certificate
/// store runs, for each lookup it holds, the method's shutdown function and
/// then its free function (`X509_STORE_free` calls `X509_LOOKUP_shutdown`
/// and `X509_LOOKUP_free`, `x509_lu.c` 3.5.5), each handed the library's
/// `X509_LOOKUP` (`_`); a built-in method's under
/// [`Premise::BuiltinLookupMethods`]. `SSL_CTX_free` frees the context's
/// store ("SSL_CTX_set_cert_store() will take ownership of the store") and
/// its `CERT`'s verify and chain stores; `SSL_free` frees its `CERT`'s
/// stores and may drop its context's last reference. The twelve
/// `x509_store_st.*` slots are not run at a store's free: the store's
/// cleanup function is run by `X509_STORE_CTX_cleanup`, a verification
/// context's, which the handshake states.
const LOOKUP_FREED: [Premise; 3] = [
    Premise::BuiltinLookupMethods,
    on_ssl("x509_lookup_method_st.lookup_meth_shutdown", &[None; 1]),
    on_ssl("x509_lookup_method_st.lookup_meth_free", &[None; 1]),
];
/// `X509_STORE_free`: the lookups' slots, the unstated premise for what the
/// store's certificates and CRLs run when freed (their keys' methods,
/// providers and engines, a CRL's method), and ex_data.
const X509_STORE_FREED: &[Premise] = &[
    Premise::UnstatedFreeSlots,
    Premise::ExDataCallbacks,
    LOOKUP_FREED[0],
    LOOKUP_FREED[1],
    LOOKUP_FREED[2],
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**: a teardown slot run
/// at a free on an object the free reaches in turn (`_`, the library's) or
/// on its own argument (`Some(0)`).
const fn on_freed(slot: &'static str, arguments: &'static [Option<usize>]) -> Premise {
    Premise::SlotInvoked {
        object: 0,
        slot,
        arguments,
    }
}

/// The teardown slots of a key freed in turn (every actual the library's):
/// `pkey_free(pkey)`, `finish(rsa)` and its siblings, an engine's
/// `finish(e)`, `destroy(e)` and its method enumerators `(e, pmeth, nids,
/// nid)`, and the two premises for the teardown functions no setter is
/// handed.
const KEY_FREED: [Premise; 11] = [
    Premise::BuiltinTeardown,
    Premise::LoadedTeardown,
    on_freed("evp_pkey_asn1_method_st.asn1_meth_pkey_free", &[None]),
    on_freed("rsa_meth_st.rsa_meth_finish", &[None]),
    on_freed("dsa_method.dsa_meth_finish", &[None]),
    on_freed("dh_method.dh_meth_finish", &[None]),
    on_freed("ec_key_method_st.ec_key_meth_finish", &[None]),
    on_freed("engine_st.engine_finish", &[None]),
    on_freed("engine_st.engine_destroy", &[None]),
    on_freed("engine_st.engine_pkey_meths", &[None; 4]),
    on_freed("engine_st.engine_pkey_asn1_meths", &[None; 4]),
];

/// The legacy digest, cipher and public key method cleanups of contexts
/// freed in turn, each handed the library's context.
const CONTEXT_FREED: [Premise; 3] = [
    on_freed("evp_md_st.md_meth_cleanup", &[None]),
    on_freed("evp_cipher_st.cipher_meth_cleanup", &[None]),
    on_freed("evp_pkey_method_st.pkey_meth_cleanup", &[None]),
];

/// The async wait context's fd cleanup functions, `cleanup(ctx, key, fd,
/// custom_data)`, every actual the library's.
const ASYNC_FREED: Premise = on_freed("async_wait_ctx_st.wait_fd_cleanup", &[None; 4]);

/// `X509_free`: what a key freed in turn runs, beside the ex_data premise.
/// `PKCS12_free` releases its argument, so it states no slot and names the
/// released free's statement.
const CERTIFICATE_FREED: &[Premise] = &[
    Premise::ExDataCallbacks,
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
];
const PKCS12_FREED: &[Premise] = &[
    Premise::UnstatedReleasedFreeSlots,
    Premise::ExDataCallbacks,
    Premise::BuiltinTeardown,
    Premise::LoadedTeardown,
];

/// `EVP_PKEY_free`: the key's own ASN.1 method free is handed the argument.
const PKEY_FREED: &[Premise] = &[
    Premise::ExDataCallbacks,
    KEY_FREED[0],
    KEY_FREED[1],
    on_freed("evp_pkey_asn1_method_st.asn1_meth_pkey_free", &[Some(0)]),
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
];

/// `ENGINE_free`: a structural free, handed the argument: its destroy
/// function and method enumerators, never its finish function.
const ENGINE_FREED: &[Premise] = &[
    Premise::ExDataCallbacks,
    Premise::BuiltinTeardown,
    Premise::LoadedTeardown,
    on_freed("engine_st.engine_destroy", &[Some(0)]),
    on_freed("engine_st.engine_pkey_meths", &[Some(0), None, None, None]),
    on_freed(
        "engine_st.engine_pkey_asn1_meths",
        &[Some(0), None, None, None],
    ),
];

/// `EVP_MD_CTX_free`: it releases its argument, so it states no teardown
/// slot and names the released free's statement.
const DIGEST_CONTEXT_FREED: &[Premise] = &[
    Premise::UnstatedReleasedFreeSlots,
    Premise::ExDataCallbacks,
    Premise::BuiltinTeardown,
    Premise::LoadedTeardown,
];

/// Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**:
/// `EC_KEY_METHOD_set_init(meth, init, finish, copy, set_group, set_private,
/// set_public)` stores all six (crypto/ec/ec_kmeth.c:204-209 at 3.5.5): the
/// shape registers `finish` (2), these the other five.
const EC_KEY_METHOD_INIT: &[Premise] = &[
    Premise::SlotRegistered {
        holder: 0,
        callback: 1,
        slot: "ec_key_method_st.ec_key_meth_init",
    },
    Premise::SlotRegistered {
        holder: 0,
        callback: 3,
        slot: "ec_key_method_st.ec_key_meth_copy",
    },
    Premise::SlotRegistered {
        holder: 0,
        callback: 4,
        slot: "ec_key_method_st.ec_key_meth_set_group",
    },
    Premise::SlotRegistered {
        holder: 0,
        callback: 5,
        slot: "ec_key_method_st.ec_key_meth_set_private",
    },
    Premise::SlotRegistered {
        holder: 0,
        callback: 6,
        slot: "ec_key_method_st.ec_key_meth_set_public",
    },
];

/// `ASYNC_WAIT_CTX_set_wait_fd(ctx, key, fd, custom_data, cleanup)`: the
/// context keeps `key` (1) beside `custom_data` (3), the shape's retention
/// (async_wait.c:54-56 at 3.5.5).
const WAIT_FD_KEY: &[Premise] = &[Premise::RetainedBy { item: 1, holder: 0 }];

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

/// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: `SSL_CTX_set_session_ticket_cb`'s
/// second registration (`dec_cb` at 2), beside the shape's `gen_cb` (1) and
/// `arg` (3), which "is passed to the callbacks".
const SESSION_TICKET_CB: &[Premise] = &[Premise::SlotRegistered {
    holder: 0,
    callback: 2,
    slot: "ssl_ctx_st.ticket_dec_cb",
}];

/// Card **OPENSSL-EX-DATA-REGISTRY**: the ex_data premise alone, for a
/// declaration that may run no unstated slot but creates, copies or frees an
/// object of an ex_data class ([`EX_DATA_AUDIT`]).
const EX_DATA_ONLY: &[Premise] = &[Premise::ExDataCallbacks];

/// Card **OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS**: a call that is not a
/// free but frees a key or certificate inside (a replaced one, or one it read
/// and does not keep) names the unstated-slots premise in the statement of a
/// free ([`FREE_AUDIT`]: its `#freed` counts objects freed inside the call)
/// beside the ex_data premise. Card **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**:
/// and states the teardown slots of a key it frees ([`KEY_FREED`]), which
/// the free statement no longer covers once their setters are declared.
const INNER_FREED: &[Premise] = &[
    Premise::UnstatedFreeSlots,
    Premise::ExDataCallbacks,
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
];

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
    // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: a failed read frees
    // the key or certificate it built (and a certificate's public key), and
    // the decoder context, in the free statement.
    Premise::UnstatedFreeSlots,
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: and the teardown slots
    // of the key it frees, stated through the BIO formal (the key is
    // run-wide).
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
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
    // Card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: and freed, with their
    // keys and the safe's CRLs, on a failed parse.
    Premise::UnstatedFreeSlots,
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the keys' teardown slots.
    KEY_FREED[0],
    KEY_FREED[1],
    KEY_FREED[2],
    KEY_FREED[3],
    KEY_FREED[4],
    KEY_FREED[5],
    KEY_FREED[6],
    KEY_FREED[7],
    KEY_FREED[8],
    KEY_FREED[9],
    KEY_FREED[10],
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
        ENGINE_FREED,
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
        DIGEST_CONTEXT_FREED,
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
        PKEY_FREED,
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
        PKCS12_FREED,
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
        INNER_FREED,
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
        "SSL_CTX_set_async_callback",
        "SSL_set_async_callback",
        "3.5",
        SINCE_3_0_0,
        registers(1, "ssl_ctx_st.async_cb"),
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
        "SSL_CTX_set_cookie_generate_cb",
        "SSL_CTX_set_stateless_cookie_generate_cb",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "ssl_ctx_st.app_gen_cookie_cb"),
    ),
    entry(
        "SSL_CTX_set_cookie_verify_cb",
        "SSL_CTX_set_stateless_cookie_generate_cb",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "ssl_ctx_st.app_verify_cookie_cb"),
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
        INNER_FREED,
    ),
    entry(
        "SSL_CTX_set_generate_session_id",
        "SSL_CTX_set_generate_session_id",
        "1.0.2",
        SINCE_0_9_7,
        registers(1, "ssl_ctx_st.generate_session_id"),
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
        "SSL_CTX_set_new_pending_conn_cb",
        "SSL_CTX_set_new_pending_conn_cb",
        "3.5",
        SINCE_3_5_0,
        registers_with_arg(1, "ssl_ctx_st.new_pending_conn_cb", 2),
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
    with(
        "SSL_CTX_set_session_ticket_cb",
        "SSL_CTX_set_session_ticket_cb",
        "3.5",
        SINCE_1_1_1,
        registers_with_arg(1, "ssl_ctx_st.ticket_gen_cb", 3),
        SESSION_TICKET_CB,
    ),
    entry(
        "SSL_CTX_set_srp_client_pwd_callback",
        "SSL_CTX_set_srp_password",
        "3.5",
        SINCE_1_0_1,
        registers(1, "ssl_ctx_st.srp_client_pwd_callback"),
    ),
    entry(
        "SSL_CTX_set_srp_username_callback",
        "SSL_CTX_set_srp_password",
        "3.5",
        SINCE_1_0_1,
        registers(1, "ssl_ctx_st.srp_username_callback"),
    ),
    entry(
        "SSL_CTX_set_srp_verify_param_callback",
        "SSL_CTX_set_srp_password",
        "3.5",
        SINCE_1_0_1,
        registers(1, "ssl_ctx_st.srp_verify_param_callback"),
    ),
    entry(
        "SSL_CTX_set_stateless_cookie_generate_cb",
        "SSL_CTX_set_stateless_cookie_generate_cb",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.gen_stateless_cookie_cb"),
    ),
    entry(
        "SSL_CTX_set_stateless_cookie_verify_cb",
        "SSL_CTX_set_stateless_cookie_generate_cb",
        "3.5",
        SINCE_1_1_1,
        registers(1, "ssl_ctx_st.verify_stateless_cookie_cb"),
    ),
    entry(
        "SSL_CTX_set_tmp_dh_callback",
        "SSL_CTX_set_tmp_dh_callback",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.tmp_dh_callback"),
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
        INNER_FREED,
    ),
    with(
        "SSL_CTX_use_PrivateKey_file",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        &[
            Premise::UnstatedFreeSlots,
            Premise::ExDataCallbacks,
            PASSWORD,
            KEY_FREED[0],
            KEY_FREED[1],
            KEY_FREED[2],
            KEY_FREED[3],
            KEY_FREED[4],
            KEY_FREED[5],
            KEY_FREED[6],
            KEY_FREED[7],
            KEY_FREED[8],
            KEY_FREED[9],
            KEY_FREED[10],
        ],
    ),
    with(
        "SSL_CTX_use_certificate",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        &[
            Premise::UnstatedFreeSlots,
            Premise::ExDataCallbacks,
            SECURITY_CTX_CERTIFICATE,
            KEY_FREED[0],
            KEY_FREED[1],
            KEY_FREED[2],
            KEY_FREED[3],
            KEY_FREED[4],
            KEY_FREED[5],
            KEY_FREED[6],
            KEY_FREED[7],
            KEY_FREED[8],
            KEY_FREED[9],
            KEY_FREED[10],
        ],
    ),
    with(
        "SSL_CTX_use_certificate_chain_file",
        "SSL_CTX_use_certificate",
        "1.0.2",
        DOC_1_0_2,
        Plain,
        &[
            Premise::UnstatedFreeSlots,
            Premise::ExDataCallbacks,
            PASSWORD,
            SECURITY_CTX_LOADED,
            KEY_FREED[0],
            KEY_FREED[1],
            KEY_FREED[2],
            KEY_FREED[3],
            KEY_FREED[4],
            KEY_FREED[5],
            KEY_FREED[6],
            KEY_FREED[7],
            KEY_FREED[8],
            KEY_FREED[9],
            KEY_FREED[10],
        ],
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
        "SSL_set_async_callback",
        "SSL_set_async_callback",
        "3.5",
        SINCE_3_0_0,
        registers(1, "ssl_ctx_st.async_cb"),
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
        "SSL_set_generate_session_id",
        "SSL_CTX_set_generate_session_id",
        "1.0.2",
        SINCE_0_9_7,
        registers(1, "ssl_ctx_st.generate_session_id"),
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
        "SSL_set_session_secret_cb",
        "SSL_set_session_secret_cb",
        "3.2",
        DOC_3_2,
        registers_with_arg(1, "ssl_st.session_secret_cb", 2),
    ),
    entry(
        "SSL_set_tmp_dh_callback",
        "SSL_CTX_set_tmp_dh_callback",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "ssl_ctx_st.tmp_dh_callback"),
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
    // Card OPENSSL-FREE-INDIRECT-CALLBACKS: the lookup method table setters.
    lookup(
        "X509_LOOKUP_meth_set_ctrl",
        "x509_lookup_method_st.lookup_meth_ctrl",
    ),
    lookup(
        "X509_LOOKUP_meth_set_free",
        "x509_lookup_method_st.lookup_meth_free",
    ),
    lookup(
        "X509_LOOKUP_meth_set_get_by_alias",
        "x509_lookup_method_st.lookup_meth_get_by_alias",
    ),
    lookup(
        "X509_LOOKUP_meth_set_get_by_fingerprint",
        "x509_lookup_method_st.lookup_meth_get_by_fingerprint",
    ),
    lookup(
        "X509_LOOKUP_meth_set_get_by_issuer_serial",
        "x509_lookup_method_st.lookup_meth_get_by_issuer_serial",
    ),
    lookup(
        "X509_LOOKUP_meth_set_get_by_subject",
        "x509_lookup_method_st.lookup_meth_get_by_subject",
    ),
    lookup(
        "X509_LOOKUP_meth_set_init",
        "x509_lookup_method_st.lookup_meth_init",
    ),
    lookup(
        "X509_LOOKUP_meth_set_new_item",
        "x509_lookup_method_st.lookup_meth_new_item",
    ),
    lookup(
        "X509_LOOKUP_meth_set_shutdown",
        "x509_lookup_method_st.lookup_meth_shutdown",
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
        X509_STORE_FREED,
    ),
    entry(
        "X509_STORE_get0_objects",
        "X509_STORE_get0_param",
        "3.5",
        SINCE_1_1_0,
        BorrowedResult(0, Nullable),
    ),
    store("X509_STORE_set_cert_crl", "x509_store_st.cert_crl"),
    store("X509_STORE_set_check_crl", "x509_store_st.check_crl"),
    store("X509_STORE_set_check_issued", "x509_store_st.check_issued"),
    store("X509_STORE_set_check_policy", "x509_store_st.check_policy"),
    store(
        "X509_STORE_set_check_revocation",
        "x509_store_st.check_revocation",
    ),
    store("X509_STORE_set_cleanup", "x509_store_st.cleanup_fn"),
    entry(
        "X509_STORE_set_flags",
        "X509_STORE_add_cert",
        "1.1.1",
        DOC_1_1_1,
        Plain,
    ),
    store("X509_STORE_set_get_crl", "x509_store_st.get_crl"),
    store("X509_STORE_set_get_issuer", "x509_store_st.get_issuer"),
    store("X509_STORE_set_lookup_certs", "x509_store_st.lookup_certs"),
    store("X509_STORE_set_lookup_crls", "x509_store_st.lookup_crls"),
    store("X509_STORE_set_verify", "x509_store_st.verify"),
    store("X509_STORE_set_verify_cb", "x509_store_st.verify_cb"),
    with(
        "X509_free",
        "X509_new",
        "1.1.1",
        DOC_1_1_1,
        NullAcceptingRelease {
            position: 0,
            sentence: "If a is NULL nothing is done.",
        },
        CERTIFICATE_FREED,
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
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the setters of the
    // teardown slots (teardown_slots.qv). The documented ones cite their 3.5
    // page's HISTORY; the source-derived ones (user decision 2026-11-21,
    // "Admit pinned source") cite the pinned 3.5.5 source in
    // [`SOURCE_CITATIONS`] and are never documented.
    entry(
        "RSA_meth_set_finish",
        "RSA_meth_new",
        "3.5",
        SINCE_1_1_0,
        registers(1, "rsa_meth_st.rsa_meth_finish"),
    ),
    entry(
        "DSA_meth_set_finish",
        "DSA_meth_new",
        "3.5",
        SINCE_1_1_0,
        registers(1, "dsa_method.dsa_meth_finish"),
    ),
    entry(
        "DH_meth_set_finish",
        "DH_meth_new",
        "3.5",
        SINCE_1_1_0,
        registers(1, "dh_method.dh_meth_finish"),
    ),
    with(
        "EC_KEY_METHOD_set_init",
        "crypto/ec/ec_kmeth.c",
        SOURCE_SERIES,
        SINCE_3_5_5,
        registers(2, "ec_key_method_st.ec_key_meth_finish"),
        EC_KEY_METHOD_INIT,
    ),
    entry(
        "ENGINE_set_finish_function",
        "ENGINE_add",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "engine_st.engine_finish"),
    ),
    entry(
        "ENGINE_set_destroy_function",
        "ENGINE_add",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "engine_st.engine_destroy"),
    ),
    entry(
        "ENGINE_set_pkey_meths",
        "crypto/engine/tb_pkmeth.c",
        SOURCE_SERIES,
        SINCE_3_5_5,
        registers(1, "engine_st.engine_pkey_meths"),
    ),
    entry(
        "ENGINE_set_pkey_asn1_meths",
        "crypto/engine/tb_asnmth.c",
        SOURCE_SERIES,
        SINCE_3_5_5,
        registers(1, "engine_st.engine_pkey_asn1_meths"),
    ),
    entry(
        "EVP_PKEY_asn1_set_free",
        "EVP_PKEY_ASN1_METHOD",
        "1.1.1",
        DOC_1_1_1,
        registers(1, "evp_pkey_asn1_method_st.asn1_meth_pkey_free"),
    ),
    with(
        "ASYNC_WAIT_CTX_set_wait_fd",
        "ASYNC_WAIT_CTX_new",
        "3.5",
        SINCE_1_1_0,
        registers_with_arg(4, "async_wait_ctx_st.wait_fd_cleanup", 3),
        WAIT_FD_KEY,
    ),
    entry(
        "EVP_MD_meth_set_cleanup",
        "EVP_MD_meth_new",
        "3.5",
        SINCE_1_1_0,
        registers(1, "evp_md_st.md_meth_cleanup"),
    ),
    entry(
        "EVP_CIPHER_meth_set_cleanup",
        "EVP_CIPHER_meth_new",
        "3.5",
        SINCE_1_1_0,
        registers(1, "evp_cipher_st.cipher_meth_cleanup"),
    ),
    entry(
        "EVP_PKEY_meth_set_cleanup",
        "EVP_PKEY_meth_new",
        "1.0.2",
        DOC_1_0_2,
        registers(1, "evp_pkey_method_st.pkey_meth_cleanup"),
    ),
];
