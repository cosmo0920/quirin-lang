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

//! The OpenSSL contract checker (cards **OPENSSL-PROVIDER-PLUGIN** and
//! **OPENSSL-PROVIDER-REMAINING-CALLEES**).
//!
//! The audit sends one request per binding (`quirin-plugin.json`): one
//! `parameter_mode_respected` obligation per bound function, bound to the
//! module's text digest. For each function the checker decides:
//!
//! 1. **The module** is one this checker was built with ([`crate::MODULES`]):
//!    its SHA-256 is the request's `source_digest`.
//! 2. **The prototype**: the pinned headers declare the function
//!    ([`crate::PINNED_PROTOTYPES`]), and the declaration's parameter and
//!    result kinds equal the contract's ([`contract_type`]: a C pointer to
//!    const is `ptr<const, u8>`, any other object pointer `ptr<mut, u8>`, a
//!    pointer to a pointer `ptr<mut, ptr<B, u8>>` with `B` the innermost
//!    pointee's qualifier, a pointer to a function typedef `fnptr`, `int`
//!    `i32`, `long` `i64`, `unsigned long` and `uint64_t` `u64`, `size_t`
//!    `usize`, `void` `unit`).
//! 3. **The version range**: the declaration states exactly one
//!    `resource_library_version(openssl, …)` and it admits the pinned headers'
//!    version (3.5.5).
//! 4. **The documented semantics** ([`DOCUMENTED`]): the range is the one the
//!    official manual page supports, that page was fetched
//!    ([`crate::DOC_SNAPSHOTS`]), and the declaration's clauses are exactly
//!    the set [`expected_clauses`] derives from the documented shape and
//!    premises: a release of a reference-counted object is a reference drop
//!    (never a final free), a constructor of one returns a reference that may
//!    be NULL, a getter returning what its argument owns is a borrowed result,
//!    a holder keeping a pointer is a retention, every other position is
//!    borrowed, and nothing else is stated. Card
//!    **OPENSSL-SSL-CTX-CALLBACK-SLOTS**: a setter of a callback slot of an
//!    opaque object states its slot registration (and its retention of
//!    `arg`), a call that may run a stated slot states its invocation, every
//!    writer the slot's page names must be declared as such a setter
//!    ([`slot_writers_declared`]), and a call that may run a slot whose
//!    setters are not declared names the premise `openssl_unstated_callback_slots`.
//!    Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: a slot's page's NAME list must
//!    be exactly the writers and not-writers of the slots citing it, and a
//!    call whose every slot is stated names only `openssl_ex_data_callbacks`
//!    for the ex_data registry.
//!    Card **OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS**: a setter may register
//!    several slots at once (each registration beside the shape's is a
//!    [`Premise::SlotRegistered`], with a second retention a
//!    [`Premise::RetainedBy`]), and it is a writer of each.
//!    Card **OPENSSL-BIO-METHODS-AND-CALLBACKS**: a BIO's callback and its
//!    method table's functions are slots too (`bio_st.*`, `bio_method_st.*`),
//!    run by the handshake and I/O calls and `SSL_shutdown` beside the
//!    library-value premise `openssl_builtin_bio_methods`; a drop of a BIO
//!    (`BIO_free`, `SSL_free`) names `openssl_bio_free_callbacks`.
//!    Card **OPENSSL-FREE-SESSION-REMOVE-INVOCATION**: a slot invocation
//!    stands beside a reference drop (C04.19 amended), so a BIO's drop states
//!    its callback and destroy slots, the frees of a connection or context
//!    state the session-remove slot, and the free premise names only the
//!    BIO class's ex_data free callbacks.
//!    Card **OPENSSL-EX-DATA-REGISTRY**: a declaration that allocates,
//!    copies or finally frees an object of an ex_data class (directly, or a
//!    replaced or temporary object it frees inside) names
//!    `openssl_ex_data_callbacks`, exactly as [`EX_DATA_AUDIT`] reads it, and
//!    every constructor and free is in that audit ([`ex_data_audited`]).
//!    Card **REFERENCE-DROP-NULL-ACCEPTING**: a release whose page states that a
//!    NULL argument does nothing is a NULL-accepting drop,
//!    `resource_reference_dropped(p, nullable)`, and the cited snapshot must
//!    record that exact sentence; the range's series is then the oldest one
//!    whose page states it.
//!    Card **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**: every release is
//!    in the free audit [`FREE_AUDIT`] ([`free_audited`]): a free that may
//!    run a function of a method, provider or engine of an object it frees
//!    names `openssl_unstated_callback_slots` in exactly the statement of
//!    its object kind, one whose run is named by another premise names that
//!    premise, and one freeing nothing such names no free statement.
//!
//! Every function passing all four is answered with a `Safe` **claim** and no
//! certificate, one diagnostic per function naming what was checked; the
//! audit's firewall publishes it `Pending` (`plugin-claim-uncertified`). Any
//! failure answers `Unknown` with the failure as a diagnostic, which the
//! firewall publishes `Unknown`. Neither is `Safe`. The checker reads only
//! what is compiled into it; it opens no file and no network connection.

use qcore_schemas::library_version::LibraryVersion;
use qcore_schemas::plugin::{
    PluginDiagnostic, PluginManifest, ProposedVerdict, VerificationRequestV2,
    VerificationResponseV2,
};
use serde_json::Value;

pub use crate::documented::{
    BIO_FREE, BIO_FREE_STATEMENT, BUILTIN_BIO_METHODS, BUILTIN_BIO_METHODS_STATEMENT,
    BUILTIN_LOOKUP_METHODS, BUILTIN_LOOKUP_METHODS_STATEMENT, DOCUMENTED, Documented,
    DocumentedFunction, EX_DATA, EX_DATA_AUDIT, EX_DATA_CLASSES, EX_DATA_STATEMENT, ExData,
    FREE_AUDIT, FreeStatement, Frees, NULLABLE_ARGUMENTS, Null, NullableArgument, Premise, SLOTS,
    Slot, UNSTATED_CERTIFICATE_FREE_STATEMENT, UNSTATED_DIGEST_CONTEXT_FREE_STATEMENT,
    UNSTATED_ENGINE_FREE_STATEMENT, UNSTATED_FREE_STATEMENT, UNSTATED_KEY_FREE_STATEMENT,
    UNSTATED_PKCS12_FREE_STATEMENT, UNSTATED_SLOTS, UNSTATED_SLOTS_STATEMENT, nullable_argument,
};

/// The crate name a request's translation unit carries.
pub const CRATE: &str = "quirin-provider-openssl";

/// One declaration of a module, read lexically: its parameters' types, its
/// result type and its clauses (comments removed, whitespace collapsed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub parameters: Vec<(String, String)>,
    pub result: String,
    pub clauses: Vec<String>,
}

fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The declaration of `function` in a module's text.
#[must_use]
pub fn declaration(module: &str, function: &str) -> Option<Declaration> {
    let text = strip_comments(module);
    let start = text.find(&format!("extern fn {function}("))?;
    let rest = &text[start + format!("extern fn {function}(").len()..];
    let close = rest.find(')')?;
    // Split at the commas outside `<…>` (`ptr<mut, u8>` holds one).
    let mut pieces = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    let list = &rest[..close];
    for (index, character) in list.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                pieces.push(&list[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    pieces.push(&list[start..]);
    let parameters = pieces
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (name, ty) = p.split_once(':')?;
            Some((name.trim().to_owned(), collapse(ty)))
        })
        .collect::<Option<Vec<_>>>()?;
    let after = &rest[close + 1..];
    let result = collapse(after.split_once("->")?.1.split("contract").next()?);
    let body_start = after.find('{')?;
    let body_end = after.find("};")?;
    let clauses = after[body_start + 1..body_end]
        .split(';')
        .map(collapse)
        .filter(|clause| !clause.is_empty())
        .collect();
    Some(Declaration {
        parameters,
        result,
        clauses,
    })
}

/// The pinned headers' typedefs of function types, whose pointers are
/// `fnptr`.
pub const FUNCTION_TYPEDEFS: &[&str] = &["pem_password_cb"];

/// Card **OPENSSL-UNSTATED-CALLBACK-SLOTS**: the pinned headers' typedefs of
/// function *pointer* types (`typedef int (*SSL_verify_cb)(int preverify_ok,
/// X509_STORE_CTX *x509_ctx);`), which are `fnptr` themselves. Card
/// **OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS**: and the handshake callback
/// types its setters take, each `typedef R (*NAME)(P…);` in the pinned
/// `openssl/ssl.h`.
pub const FUNCTION_POINTER_TYPEDEFS: &[&str] = &[
    "SSL_verify_cb",
    "SSL_CTX_keylog_cb_func",
    "SSL_CTX_alpn_select_cb_func",
    "SSL_CTX_npn_select_cb_func",
    "SSL_CTX_npn_advertised_cb_func",
    "SSL_psk_client_cb_func",
    "SSL_psk_use_session_cb_func",
    "SSL_psk_server_cb_func",
    "SSL_psk_find_session_cb_func",
    "SSL_client_hello_cb_fn",
    "SSL_allow_early_data_cb_fn",
    "SSL_custom_ext_add_cb_ex",
    "SSL_custom_ext_free_cb_ex",
    "SSL_custom_ext_parse_cb_ex",
    "custom_ext_add_cb",
    "custom_ext_free_cb",
    "custom_ext_parse_cb",
    // Card OPENSSL-BIO-METHODS-AND-CALLBACKS: the BIO callback types,
    // `typedef long (*NAME)(BIO *b, …);` in the pinned `openssl/bio.h`.
    "BIO_callback_fn_ex",
    "BIO_callback_fn",
    // Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: the remaining handshake
    // callback types (pinned `openssl/ssl.h`) and the certificate store's
    // callback types (pinned `openssl/x509_vfy.h`), each `typedef R
    // (*NAME)(P…);`.
    "SSL_CTX_generate_session_ticket_fn",
    "SSL_CTX_decrypt_session_ticket_fn",
    "GEN_SESSION_CB",
    "tls_session_secret_cb_fn",
    "SSL_async_callback_fn",
    "SSL_new_pending_conn_cb_fn",
    "X509_STORE_CTX_verify_cb",
    "X509_STORE_CTX_verify_fn",
    "X509_STORE_CTX_get_issuer_fn",
    "X509_STORE_CTX_check_issued_fn",
    "X509_STORE_CTX_check_revocation_fn",
    "X509_STORE_CTX_get_crl_fn",
    "X509_STORE_CTX_check_crl_fn",
    "X509_STORE_CTX_cert_crl_fn",
    "X509_STORE_CTX_check_policy_fn",
    "X509_STORE_CTX_lookup_certs_fn",
    "X509_STORE_CTX_lookup_crls_fn",
    "X509_STORE_CTX_cleanup_fn",
    // Card OPENSSL-FREE-INDIRECT-CALLBACKS: the lookup method function types
    // the get_by and ctrl setters take, each `typedef int (*NAME)(X509_LOOKUP
    // *ctx, …);` in the pinned `openssl/x509_vfy.h`.
    "X509_LOOKUP_ctrl_fn",
    "X509_LOOKUP_get_by_subject_fn",
    "X509_LOOKUP_get_by_issuer_serial_fn",
    "X509_LOOKUP_get_by_fingerprint_fn",
    "X509_LOOKUP_get_by_alias_fn",
];

/// The contract type a C type from a pinned prototype corresponds to.
#[must_use]
pub fn contract_type(c_type: &str) -> Option<&'static str> {
    let c_type = collapse(c_type);
    // Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: a pointer to a function spelled in
    // place, `R (*)(P…)` (its name dropped by [`prototype_types`]), is
    // `fnptr`; nothing else with parentheses has a spelling.
    if c_type.contains('(') {
        let is_function_pointer = c_type.split_once("(*)(").is_some_and(|(result, rest)| {
            !result.trim().is_empty()
                && !result.contains(['(', ')'])
                && rest.ends_with(')')
                && !rest[..rest.len() - 1].contains(['(', ')'])
        });
        return is_function_pointer.then_some("fnptr");
    }
    let stars = c_type.matches('*').count();
    let base = c_type.trim_end_matches(['*', ' ']);
    if FUNCTION_TYPEDEFS.contains(&base) {
        // A pointer to a function type is `fnptr`; a pointer to that pointer
        // has no spelling.
        return (stars == 1).then_some("fnptr");
    }
    if FUNCTION_POINTER_TYPEDEFS.contains(&base) {
        // The typedef is the pointer itself; a pointer to it has no spelling.
        return (stars == 0).then_some("fnptr");
    }
    match stars {
        0 => {}
        1 if c_type.ends_with('*') => {
            return Some(if c_type.starts_with("const ") {
                "ptr<const, u8>"
            } else {
                "ptr<mut, u8>"
            });
        }
        // `T **` / `const T **`: the slot is unqualified, the innermost
        // pointee carries the qualifier (C04.1, rule 12v).
        2 if c_type.ends_with("**") && !c_type[..c_type.len() - 2].contains('*') => {
            return Some(if c_type.starts_with("const ") {
                "ptr<mut, ptr<const, u8>>"
            } else {
                "ptr<mut, ptr<mut, u8>>"
            });
        }
        _ => return None,
    }
    match c_type.as_str() {
        "void" => Some("unit"),
        "int" => Some("i32"),
        // Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: the custom extension
        // setters' `ext_type` and `context`.
        "unsigned int" => Some("u32"),
        "long" => Some("i64"),
        "unsigned long" | "uint64_t" => Some("u64"),
        "size_t" => Some("usize"),
        _ => None,
    }
}

/// A pinned declaration's (parameter types, result type), as C types.
#[must_use]
pub fn prototype_types(declaration: &str, function: &str) -> Option<(Vec<String>, String)> {
    let open = declaration.find(&format!("{function}("))?;
    let result = declaration[..open].trim().to_owned();
    let inner = declaration[open + function.len() + 1..].strip_suffix(')')?;
    // Split at the commas outside parentheses (an in-place function pointer
    // `int (*cb)(SSL *ssl, void *arg)` holds one).
    let mut pieces = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (index, character) in inner.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => {
                pieces.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    pieces.push(&inner[start..]);
    let parameters = if inner.trim() == "void" {
        Vec::new()
    } else {
        pieces
            .into_iter()
            .map(|parameter| {
                // An in-place function pointer: drop its name, `R (*name)(P…)`
                // to `R (*)(P…)`, and the parameter names inside.
                if let Some((result, rest)) = parameter.split_once("(*")
                    && let Some((name, list)) = rest.split_once(")(")
                    && name.trim().chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    let list = list.strip_suffix(')').unwrap_or(list);
                    let types = list
                        .split(',')
                        .map(|inner| {
                            let inner = inner.trim();
                            let cut = inner.rfind([' ', '*']).map_or(0, |index| index + 1);
                            let (base, name) = inner.split_at(cut);
                            if !base.trim().is_empty()
                                && !name.is_empty()
                                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                            {
                                collapse(base)
                            } else {
                                collapse(inner)
                            }
                        })
                        .collect::<Vec<_>>();
                    return collapse(&format!("{} (*)({})", result.trim(), types.join(", ")));
                }
                // Drop a trailing parameter name, if any: the identifier
                // after the last space or `*`, when a type remains before it.
                let parameter = parameter.trim();
                let cut = parameter.rfind([' ', '*']).map_or(0, |index| index + 1);
                let (base, name) = parameter.split_at(cut);
                if !base.trim().is_empty()
                    && !name.is_empty()
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    collapse(base)
                } else {
                    collapse(parameter)
                }
            })
            .collect()
    };
    Some((parameters, result))
}

/// Decide one function; `Ok(note)` when every check passes.
///
/// # Errors
///
/// The first check that fails, as the diagnostic text.
pub fn check_function(module_path: &str, module: &str, function: &str) -> Result<String, String> {
    let documented = DOCUMENTED
        .iter()
        .find(|entry| entry.function == function)
        .ok_or_else(|| format!("{function}: no documented semantics in this checker"))?;
    let declaration = declaration(module, function)
        .ok_or_else(|| format!("{function}: `{module_path}` does not declare it"))?;
    let pinned: Value = serde_json::from_str(crate::PINNED_PROTOTYPES)
        .map_err(|error| format!("pinned prototypes: {error}"))?;
    let pinned_version = pinned["version"]
        .as_str()
        .and_then(|text| LibraryVersion::parse(text).ok())
        .ok_or("pinned prototypes carry no version")?;
    let prototype = pinned["prototypes"][function]["declaration"]
        .as_str()
        .ok_or_else(|| format!("{function}: the pinned headers do not declare it"))?;
    let header = pinned["prototypes"][function]["header"]
        .as_str()
        .unwrap_or_default();
    let header_sha256 = pinned["headers"][header].as_str().unwrap_or_default();
    // 2. The prototype.
    let (c_parameters, c_result) = prototype_types(prototype, function)
        .ok_or_else(|| format!("{function}: unreadable pinned prototype `{prototype}`"))?;
    let expected_result = contract_type(&c_result)
        .ok_or_else(|| format!("{function}: unmapped C result type `{c_result}`"))?;
    if declaration.result != expected_result {
        return Err(format!(
            "{function}: the contract returns `{}` but the pinned prototype `{prototype}` returns \
             `{expected_result}`",
            declaration.result
        ));
    }
    if c_parameters.len() != declaration.parameters.len() {
        return Err(format!(
            "{function}: the contract has {} parameter(s), the pinned prototype `{prototype}` {}",
            declaration.parameters.len(),
            c_parameters.len()
        ));
    }
    for ((name, ty), c_type) in declaration.parameters.iter().zip(&c_parameters) {
        let expected = contract_type(c_type)
            .ok_or_else(|| format!("{function}: unmapped C parameter type `{c_type}`"))?;
        if ty != expected {
            return Err(format!(
                "{function}: parameter `{name}` is `{ty}` but the pinned prototype `{prototype}` \
                 makes it `{expected}`"
            ));
        }
    }
    // 3. The version range.
    let ranges = declaration
        .clauses
        .iter()
        .filter(|clause| clause.starts_with("requires resource_library_version("))
        .collect::<Vec<_>>();
    let [range] = ranges.as_slice() else {
        return Err(format!(
            "{function}: the declaration must state exactly one library version range"
        ));
    };
    let range = range.trim_start_matches("requires ").to_owned();
    if range != documented.range {
        return Err(format!(
            "{function}: the declaration states `{range}` but the documentation supports `{}`",
            documented.range
        ));
    }
    let bound = parse_range(&range).ok_or_else(|| format!("{function}: unreadable `{range}`"))?;
    if !(bound.0 <= pinned_version && bound.1.is_none_or(|removed| pinned_version < removed)) {
        return Err(format!(
            "{function}: `{range}` does not admit the pinned OpenSSL {pinned_version}"
        ));
    }
    // 4. The documented semantics.
    let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS)
        .map_err(|error| format!("doc snapshots: {error}"))?;
    let page = snapshots["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|page| {
            page["page"] == documented.page
                && page["series"] == documented.series
                && page["fetched"] == true
        })
        .ok_or_else(|| {
            format!(
                "{function}: no fetched snapshot of {} at series {}",
                documented.page, documented.series
            )
        })?;
    let names = declaration
        .parameters
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let expected = expected_clauses(documented, &names).ok_or_else(|| {
        format!("{function}: the documented shape names a parameter the declaration lacks")
    })?;
    let mut stated = declaration.clauses.clone();
    stated.sort();
    if stated != expected {
        let missing = expected
            .iter()
            .filter(|clause| !stated.contains(clause))
            .cloned()
            .collect::<Vec<_>>();
        let extra = stated
            .iter()
            .filter(|clause| !expected.contains(clause))
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "{function}: the contract does not state exactly the documented {} \
             (missing: {missing:?}; not documented: {extra:?})",
            describe(documented.shape)
        ));
    }
    // Card REFERENCE-DROP-NULL-ACCEPTING: a NULL-accepting drop stands only
    // on a snapshot that records the page's exact NULL sentence.
    if let Documented::NullAcceptingRelease { sentence, .. } = documented.shape {
        let stated = page["null_sentences"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|stated| stated.as_str() == Some(sentence));
        if !stated {
            return Err(format!(
                "{function}: the snapshot of {} at series {} does not record \"{sentence}\"",
                documented.page, documented.series
            ));
        }
    }
    // Card OPENSSL-EX-DATA-REGISTRY: a declaration that creates or frees an
    // object of an ex_data class names the ex_data premise, and the audit
    // accounts for it.
    ex_data_audited(documented, EX_DATA_AUDIT)
        .map_err(|failure| format!("{function}: {failure}"))?;
    // Card OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: a free names what it
    // runs on the objects it frees, exactly as the free audit reads it.
    free_audited(documented, FREE_AUDIT).map_err(|failure| format!("{function}: {failure}"))?;
    // Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: every slot the declaration states
    // has every writer its page names declared as its setter.
    for slot in stated_slots(documented) {
        slot_writers_declared(slot, &snapshots)
            .map_err(|failure| format!("{function}: {failure}"))?;
        // Card CALLBACK-SUPPLIED-NULL-ACTUAL: and every nullable argument of
        // it is cited from its page and used.
        nullable_arguments_cited(slot, &snapshots)
            .map_err(|failure| format!("{function}: {failure}"))?;
    }
    Ok(format!(
        "{function}: prototype `{prototype}` ({header}, sha256 {}) matches the contract; `{range}` \
         admits the pinned OpenSSL {pinned_version}; custody as documented at {} (sha256 {} as \
         fetched)",
        &header_sha256[..header_sha256.len().min(12)],
        page["url"].as_str().unwrap_or_default(),
        &page["sha256"].as_str().unwrap_or_default()[..12],
    ))
}

/// Card **OPENSSL-EX-DATA-REGISTRY**: whether a shape creates or frees an
/// object (a constructor, a release, a reference drop).
#[must_use]
pub const fn creates_or_frees(shape: Documented) -> bool {
    matches!(
        shape,
        Documented::CountedConstructor
            | Documented::Constructor
            | Documented::Release(..)
            | Documented::CountedRelease(_)
            | Documented::NullAcceptingRelease { .. }
    )
}

/// Card **OPENSSL-EX-DATA-REGISTRY**: the entry's reading in `audit` holds.
///
/// * The entry carries [`Premise::ExDataCallbacks`] exactly when `audit`
///   lists it as [`ExData::Runs`], with at least one class, each one of
///   [`EX_DATA_CLASSES`].
/// * An entry whose shape creates or frees an object ([`creates_or_frees`])
///   is listed, either way: a constructor or free is never left out of the
///   audit, so none reads the registry silently.
/// * `audit` lists the entry at most once.
///
/// # Errors
///
/// The first of these that fails, as the diagnostic text.
pub fn ex_data_audited(
    documented: &DocumentedFunction,
    audit: &[(&str, ExData)],
) -> Result<(), String> {
    let listed = audit
        .iter()
        .filter(|(function, _)| *function == documented.function)
        .map(|(_, reading)| *reading)
        .collect::<Vec<_>>();
    let carries = documented.premises.contains(&Premise::ExDataCallbacks);
    match listed.as_slice() {
        [] if carries => Err(format!(
            "names `{EX_DATA}` but the ex_data audit does not list it"
        )),
        [] if creates_or_frees(documented.shape) => Err(format!(
            "its {} creates or frees an object, and the ex_data audit does not list it",
            describe(documented.shape)
        )),
        [] | [ExData::NoClass { .. }] if !carries => Ok(()),
        [ExData::Runs { classes, .. }] => {
            if !carries {
                return Err(format!(
                    "may run the ex_data callbacks of {classes:?} but does not name `{EX_DATA}`"
                ));
            }
            if classes.is_empty() {
                return Err("its ex_data audit entry names no class".to_owned());
            }
            if let Some(unknown) = classes.iter().find(|c| !EX_DATA_CLASSES.contains(c)) {
                return Err(format!(
                    "its ex_data audit entry names {unknown}, which is no ex_data class \
                     (CRYPTO_get_ex_new_index 3.5)"
                ));
            }
            Ok(())
        }
        [ExData::NoClass { .. }] => Err(format!(
            "names `{EX_DATA}` but the ex_data audit reads it as touching no ex_data class"
        )),
        _ => Err("the ex_data audit lists it more than once".to_owned()),
    }
}

/// Card **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**: whether a shape
/// releases an object (a free or a reference drop).
#[must_use]
pub const fn releases(shape: Documented) -> bool {
    matches!(
        shape,
        Documented::Release(..)
            | Documented::CountedRelease(_)
            | Documented::NullAcceptingRelease { .. }
    )
}

/// Card **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**: the entry's reading
/// in the free audit `audit` holds.
///
/// * An entry whose shape releases an object ([`releases`]) is listed, and
///   so is every entry naming a free statement
///   ([`Premise::UnstatedFreeSlots`]): no free reads the functions of the
///   objects it frees silently.
/// * [`Frees::Runs`]: the entry names the premise in exactly that
///   statement, once, and not the handshake's statement as well.
/// * [`Frees::NamedBy`]: the entry names that premise and no free statement.
/// * [`Frees::Nothing`]: the entry names no free statement.
/// * `audit` lists the entry at most once.
///
/// # Errors
///
/// The first of these that fails, as the diagnostic text.
pub fn free_audited(
    documented: &DocumentedFunction,
    audit: &[(&str, Frees)],
) -> Result<(), String> {
    let listed = audit
        .iter()
        .filter(|(function, _)| *function == documented.function)
        .map(|(_, reading)| *reading)
        .collect::<Vec<_>>();
    let named = documented
        .premises
        .iter()
        .filter_map(|premise| match premise {
            Premise::UnstatedFreeSlots(statement) => Some(*statement),
            _ => None,
        })
        .collect::<Vec<_>>();
    match listed.as_slice() {
        [] if !named.is_empty() => Err(format!(
            "names `{UNSTATED_SLOTS}` in the statement of a free but the free audit does not \
             list it"
        )),
        [] if releases(documented.shape) => Err(format!(
            "its {} frees an object, and the free audit does not list it",
            describe(documented.shape)
        )),
        [] => Ok(()),
        [Frees::Runs { statement, .. }] => match named.as_slice() {
            [] => Err(format!(
                "may run functions of methods, providers or engines of the objects it frees \
                 ({statement:?} free) but does not name `{UNSTATED_SLOTS}` in the statement of \
                 that free"
            )),
            [one] if one != statement => Err(format!(
                "names `{UNSTATED_SLOTS}` in the statement of a {one:?} free, but the free \
                 audit reads it as a {statement:?} free"
            )),
            [_] if documented.premises.contains(&Premise::UnstatedSlots) => Err(format!(
                "names `{UNSTATED_SLOTS}` in both the handshake's statement and a free's"
            )),
            [_] => Ok(()),
            _ => Err(format!(
                "names `{UNSTATED_SLOTS}` in more than one statement of a free"
            )),
        },
        [Frees::NamedBy { premise, .. }] => {
            if !named.is_empty() {
                Err(format!(
                    "names `{UNSTATED_SLOTS}` in the statement of a free, but the free audit \
                     reads what it runs in turn as named by another premise"
                ))
            } else if !documented.premises.contains(premise) {
                Err(format!(
                    "the free audit reads what it runs in turn as named by {premise:?}, which \
                     it does not name"
                ))
            } else {
                Ok(())
            }
        }
        [Frees::Nothing { .. }] if named.is_empty() => Ok(()),
        [Frees::Nothing { .. }] => Err(format!(
            "names `{UNSTATED_SLOTS}` in the statement of a free, but the free audit reads it \
             as freeing nothing whose functions a user can set"
        )),
        _ => Err("the free audit lists it more than once".to_owned()),
    }
}

/// What a shape is, for a diagnostic.
#[must_use]
pub fn describe(shape: Documented) -> &'static str {
    match shape {
        Documented::Plain => "plain call (no custody taken or returned)",
        Documented::CountedRelease(_) => {
            "counted release (one reference dropped, never a final free)"
        }
        Documented::NullAcceptingRelease { .. } => {
            "NULL-accepting counted release (one reference dropped, nothing done on NULL)"
        }
        Documented::CountedConstructor => "returned reference (one reference, or NULL)",
        Documented::Constructor => "fresh object (owned, or NULL)",
        Documented::Release(..) => "release of an object that is not reference counted",
        Documented::BorrowedResult(..) => "borrowed result (owned by the argument)",
        Documented::Retains { .. } => "retention (the holder keeps the pointer)",
        Documented::RegistersSlot { .. } => {
            "slot registration (the holder keeps the callback in one of its slots)"
        }
    }
}

/// Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: the slots an entry registers or
/// invokes.
#[must_use]
pub fn stated_slots(documented: &DocumentedFunction) -> Vec<&'static str> {
    let mut slots = documented
        .premises
        .iter()
        .filter_map(|premise| match premise {
            Premise::SlotInvoked { slot, .. } | Premise::SlotRegistered { slot, .. } => Some(*slot),
            _ => None,
        })
        .collect::<Vec<_>>();
    if let Documented::RegistersSlot { slot, .. } = documented.shape {
        slots.push(slot);
    }
    slots
}

/// Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: the slots an entry
/// registers, by its shape and by its further registrations.
#[must_use]
pub fn registered_slots(documented: &DocumentedFunction) -> Vec<&'static str> {
    let mut slots = documented
        .premises
        .iter()
        .filter_map(|premise| match premise {
            Premise::SlotRegistered { slot, .. } => Some(*slot),
            _ => None,
        })
        .collect::<Vec<_>>();
    if let Documented::RegistersSlot { slot, .. } = documented.shape {
        slots.push(slot);
    }
    slots
}

/// Card **CALLBACK-SUPPLIED-NULL-ACTUAL**: every [`NULLABLE_ARGUMENTS`] entry
/// of `slot` cites a sentence its page's snapshot at 3.5 records (in
/// `null_sentences` or `argument_null_sentences`), and some documented
/// invocation of the slot receives a value the callee supplies at its
/// position (an entry no `_` reads would be vacuous).
///
/// # Errors
///
/// The first entry that is not so, as the diagnostic text.
pub fn nullable_arguments_cited(slot: &str, snapshots: &Value) -> Result<(), String> {
    for nullable in NULLABLE_ARGUMENTS.iter().filter(|entry| entry.slot == slot) {
        nullable_argument_cited(nullable, snapshots)?;
    }
    Ok(())
}

/// One [`NullableArgument`] as [`nullable_arguments_cited`] requires it.
///
/// # Errors
///
/// Why it is not so, as the diagnostic text.
pub fn nullable_argument_cited(
    nullable: &NullableArgument,
    snapshots: &Value,
) -> Result<(), String> {
    let NullableArgument {
        slot,
        position,
        sentence,
    } = *nullable;
    let stated = SLOTS
        .iter()
        .find(|known| known.key == slot)
        .ok_or_else(|| format!("nullable argument of `{slot}`, which is not a stated slot"))?;
    let page = snapshots["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|page| {
            page["page"] == stated.page && page["series"] == "3.5" && page["fetched"] == true
        })
        .ok_or_else(|| format!("no fetched snapshot of {} at 3.5", stated.page))?;
    let recorded = ["null_sentences", "argument_null_sentences"]
        .iter()
        .flat_map(|key| page[*key].as_array().into_iter().flatten())
        .any(|recorded| recorded.as_str() == Some(sentence));
    if !recorded {
        return Err(format!(
            "`{slot}` argument {position} is read as nullable, but the snapshot of {} at 3.5 \
             does not record \"{sentence}\"",
            stated.page
        ));
    }
    let used = DOCUMENTED
        .iter()
        .flat_map(|entry| entry.premises)
        .any(|premise| {
            matches!(premise, Premise::SlotInvoked { slot: invoked, arguments, .. }
                if *invoked == slot && arguments.get(position) == Some(&None))
        });
    if !used {
        return Err(format!(
            "`{slot}` argument {position} is read as nullable, but no documented invocation \
             of the slot receives a value the callee supplies there"
        ));
    }
    Ok(())
}

/// Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: `slot` is one of [`SLOTS`]; its
/// page, as fetched at the pinned series (3.5), names every one of its
/// writers; each writer is documented as a [`Documented::RegistersSlot`] of
/// this slot and declared in a module of this crate; and no other entry
/// registers it. Then the run's candidates of the slot (what its declared
/// setters are handed) are every value it can hold.
///
/// # Errors
///
/// The first writer that is not so, as the diagnostic text.
pub fn slot_writers_declared(slot: &str, snapshots: &Value) -> Result<(), String> {
    let stated = SLOTS
        .iter()
        .find(|known| known.key == slot)
        .ok_or_else(|| format!("`{slot}` is not a slot this checker documents"))?;
    let name = snapshots["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|page| {
            page["page"] == stated.page && page["series"] == "3.5" && page["fetched"] == true
        })
        .and_then(|page| page["name"].as_str())
        .ok_or_else(|| {
            format!(
                "no fetched snapshot of {} at 3.5 records its NAME",
                stated.page
            )
        })?;
    let named = name
        .split(" - ")
        .next()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>();
    // Card OPENSSL-UNSTATED-CALLBACK-SLOTS: every function the page names is
    // a writer or a not-writer of a slot citing the page, and nothing else is
    // listed, so a writer the page names is never left out unremarked.
    let accounted = SLOTS
        .iter()
        .filter(|known| known.page == stated.page)
        .flat_map(|known| known.writers.iter().chain(known.not_writers))
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let listed = named
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(unaccounted) = listed.difference(&accounted).next() {
        return Err(format!(
            "{} (3.5) names {unaccounted}, which no slot citing the page lists as a writer or \
             a not-writer",
            stated.page
        ));
    }
    if let Some(unnamed) = accounted.difference(&listed).next() {
        return Err(format!(
            "`{slot}`'s page {} (3.5) does not name {unnamed}",
            stated.page
        ));
    }
    for writer in stated.writers {
        if !named.contains(writer) {
            return Err(format!(
                "`{slot}`'s writer {writer} is not named by {} (3.5)",
                stated.page
            ));
        }
        let registers = DOCUMENTED
            .iter()
            .any(|entry| entry.function == *writer && registered_slots(entry).contains(&slot));
        if !registers {
            return Err(format!(
                "`{slot}`'s writer {writer} is not documented as its registration"
            ));
        }
        let declared = crate::MODULES
            .iter()
            .any(|(_, text)| strip_comments(text).contains(&format!("extern fn {writer}(")));
        if !declared {
            return Err(format!("`{slot}`'s writer {writer} is not declared"));
        }
    }
    for entry in DOCUMENTED {
        if registered_slots(entry).contains(&slot) && !stated.writers.contains(&entry.function) {
            return Err(format!(
                "{} registers `{slot}` but is not one of its writers",
                entry.function
            ));
        }
    }
    Ok(())
}

/// The exact clause set, sorted, of a declaration stating `documented` over
/// parameters named `parameters` (in order); `None` when the shape or a
/// premise names a position the declaration lacks.
#[must_use]
pub fn expected_clauses(
    documented: &DocumentedFunction,
    parameters: &[String],
) -> Option<Vec<String>> {
    let name = |position: usize| parameters.get(position).cloned();
    let mut clauses = vec![
        "requires resource_contract_v1".to_owned(),
        "requires resource_revision(flexible)".to_owned(),
        format!("requires {}", documented.range),
    ];
    for premise in documented.premises {
        clauses.push(match premise {
            Premise::NullArgument(position) => {
                format!(
                    "requires resource_literal_argument({}, 0)",
                    name(*position)?
                )
            }
            Premise::UnstatedSlots => format!(
                "requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_SLOTS_STATEMENT}\")"
            ),
            Premise::UnstatedFreeSlots(statement) => format!(
                "requires resource_premise({UNSTATED_SLOTS}, \"{}\")",
                statement.text()
            ),
            Premise::BuiltinLookupMethods => format!(
                "requires resource_premise({BUILTIN_LOOKUP_METHODS}, \"{BUILTIN_LOOKUP_METHODS_STATEMENT}\")"
            ),
            Premise::ExDataCallbacks => {
                format!("requires resource_premise({EX_DATA}, \"{EX_DATA_STATEMENT}\")")
            }
            Premise::BuiltinBioMethods => format!(
                "requires resource_premise({BUILTIN_BIO_METHODS}, \"{BUILTIN_BIO_METHODS_STATEMENT}\")"
            ),
            Premise::BioFreeCallbacks => {
                format!("requires resource_premise({BIO_FREE}, \"{BIO_FREE_STATEMENT}\")")
            }
            Premise::SlotInvoked {
                object,
                slot,
                arguments,
            } => {
                let mut actuals = vec![name(*object)?, (*slot).to_owned()];
                for (at, argument) in arguments.iter().enumerate() {
                    actuals.push(match argument {
                        Some(position) => name(*position)?,
                        // Card CALLBACK-SUPPLIED-NULL-ACTUAL: the page says
                        // the callback may receive NULL there.
                        None if nullable_argument(slot, at) => "supplied_nullable".to_owned(),
                        None => "_".to_owned(),
                    });
                }
                format!(
                    "ensures normal resource_callback_slot_invoked({})",
                    actuals.join(", ")
                )
            }
            Premise::SlotRegistered {
                holder,
                callback,
                slot,
            } => format!(
                "ensures normal resource_callback_slot_registered({}, {slot}, {})",
                name(*holder)?,
                name(*callback)?
            ),
            Premise::RetainedBy { item, holder } => format!(
                "ensures normal resource_retained_by({}, {})",
                name(*item)?,
                name(*holder)?
            ),
            Premise::CallbackInvoked {
                callback,
                arguments,
            }
            | Premise::CallbackInvokedOrDefault {
                callback,
                arguments,
            } => {
                let mut actuals = vec![name(*callback)?];
                for argument in *arguments {
                    actuals.push(match argument {
                        Some(position) => name(*position)?,
                        None => "_".to_owned(),
                    });
                }
                let keyword = if matches!(premise, Premise::CallbackInvokedOrDefault { .. }) {
                    "resource_callback_invoked_or_default"
                } else {
                    "resource_callback_invoked"
                };
                format!("ensures normal {keyword}({})", actuals.join(", "))
            }
        });
    }
    for (position, parameter) in parameters.iter().enumerate() {
        clauses.push(match documented.shape {
            Documented::Release(released, null) if released == position => {
                format!(
                    "ensures normal resource_released({parameter}, {})",
                    null.word()
                )
            }
            _ => format!("ensures normal resource_borrowed({parameter})"),
        });
    }
    match documented.shape {
        Documented::Plain => clauses.push("ensures normal resource_return_none()".to_owned()),
        Documented::CountedRelease(position) => {
            clauses.push(format!(
                "ensures normal resource_reference_dropped({})",
                name(position)?
            ));
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::NullAcceptingRelease { position, .. } => {
            clauses.push(format!(
                "ensures normal resource_reference_dropped({}, nullable)",
                name(position)?
            ));
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::CountedConstructor => {
            clauses.push("ensures normal resource_return_owned(nullable)".to_owned());
            clauses.push("ensures normal resource_return_kind(reference)".to_owned());
        }
        Documented::Constructor => {
            clauses.push("ensures normal resource_return_owned(nullable)".to_owned());
        }
        Documented::Release(position, _) => {
            name(position)?;
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::BorrowedResult(position, null) => clauses.push(format!(
            "ensures normal resource_return_borrowed_from({}, {})",
            name(position)?,
            null.word()
        )),
        Documented::Retains { item, holder } => {
            clauses.push(format!(
                "ensures normal resource_retained_by({}, {})",
                name(item)?,
                name(holder)?
            ));
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
        Documented::RegistersSlot {
            holder,
            callback,
            slot,
            retains,
        } => {
            clauses.push(format!(
                "ensures normal resource_callback_slot_registered({}, {slot}, {})",
                name(holder)?,
                name(callback)?
            ));
            if let Some(item) = retains {
                clauses.push(format!(
                    "ensures normal resource_retained_by({}, {})",
                    name(item)?,
                    name(holder)?
                ));
            }
            clauses.push("ensures normal resource_return_none()".to_owned());
        }
    }
    clauses.sort();
    Some(clauses)
}

/// `(lower, removed)` of a range clause, read with the schema's version
/// parser.
#[must_use]
pub fn parse_range(range: &str) -> Option<(LibraryVersion, Option<LibraryVersion>)> {
    let inner = range
        .strip_prefix("resource_library_version(")?
        .strip_suffix(')')?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let quoted = |text: &str| -> Option<LibraryVersion> {
        LibraryVersion::parse(text.strip_prefix('"')?.strip_suffix('"')?).ok()
    };
    let lower = |part: &str| {
        part.strip_prefix("since ")
            .or_else(|| part.strip_prefix("documented "))
            .and_then(quoted)
    };
    match parts.as_slice() {
        ["openssl", lower_part] => Some((lower(lower_part)?, None)),
        ["openssl", lower_part, removed] => Some((
            lower(lower_part)?,
            Some(quoted(removed.strip_prefix("removed ")?)?),
        )),
        _ => None,
    }
}

/// The checker's answer to one request: `Safe` (a claim) when every bound
/// function passes, `Unknown` otherwise, with one diagnostic per function.
#[must_use]
pub fn answer(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    let functions = quirin_provider_plugin::checker::requested_functions(request);
    if functions.is_empty() {
        return None;
    }
    let module = crate::MODULES.iter().find(|(path, text)| {
        request.unit.translation_unit == format!("{CRATE}:{path}")
            && quirin_provider_plugin::sha256_hex(text.as_bytes()) == request.unit.source_digest
    });
    let mut notes = Vec::new();
    let mut verdict = ProposedVerdict::Safe;
    match module {
        None => {
            verdict = ProposedVerdict::Unknown;
            notes.push(format!(
                "`{}` (sha256 {}) is not a module text this checker was built with",
                request.unit.translation_unit, request.unit.source_digest
            ));
        }
        Some((path, text)) => {
            for function in functions {
                match check_function(path, text, function) {
                    Ok(note) => notes.push(note),
                    Err(failure) => {
                        verdict = ProposedVerdict::Unknown;
                        notes.push(failure);
                    }
                }
            }
        }
    }
    let mut response = quirin_provider_plugin::checker::respond(
        request,
        manifest,
        verdict,
        "OpenSSL contract check (prototype, version range, documented semantics); a claim, not a \
         certificate",
        None,
    );
    response
        .diagnostics
        .extend(notes.into_iter().map(|message| PluginDiagnostic {
            code: format!("{}.binding", manifest.plugin_id),
            message,
            fact_id: None,
        }));
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_contract_passes_its_own_checks() {
        for documented in DOCUMENTED {
            let (path, text) = crate::MODULES
                .iter()
                .find(|(_, text)| text.contains(&format!("extern fn {}(", documented.function)))
                .unwrap();
            let note = check_function(path, text, documented.function)
                .unwrap_or_else(|failure| panic!("{failure}"));
            assert!(note.contains("admits the pinned OpenSSL 3.5.5"), "{note}");
        }
    }

    #[test]
    fn every_declared_function_is_documented_and_bound() {
        let manifest: Value = serde_json::from_str(include_str!("../quirin-plugin.json")).unwrap();
        let bound = manifest["checkers"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|binding| binding["functions"].as_array().unwrap().clone())
            .map(|function| function.as_str().unwrap().to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        let mut declared = std::collections::BTreeSet::new();
        for (_, text) in crate::MODULES {
            for line in strip_comments(text).lines() {
                if let Some(rest) = line.trim().strip_prefix("extern fn ") {
                    declared.insert(rest.split('(').next().unwrap().to_owned());
                }
            }
        }
        let documented = DOCUMENTED
            .iter()
            .map(|entry| entry.function.to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            declared, bound,
            "every declaration is behind a checker binding"
        );
        assert_eq!(declared, documented);
    }

    #[test]
    fn a_drifted_contract_is_refused() {
        let (path, text) = crate::MODULES
            .iter()
            .find(|(path, _)| *path == "openssl/x509.qv")
            .unwrap();
        // A final free in place of a counted drop.
        let freed = text.replace(
            "resource_reference_dropped(a, nullable)",
            "resource_released(a, nullable)",
        );
        assert!(check_function(path, &freed, "X509_free").is_err());
        // Card REFERENCE-DROP-NULL-ACCEPTING: a drop that does not state the
        // documented NULL behaviour, and the NULL-accepting form from a series
        // whose page does not state it.
        let strict = text.replace(
            "resource_reference_dropped(a, nullable)",
            "resource_reference_dropped(a)",
        );
        assert!(check_function(path, &strict, "X509_free").is_err());
        let older = text.replace("documented \"1.1.1\"", "documented \"1.0.2\"");
        assert!(check_function(path, &older, "X509_free").is_err());
        // A range the documentation does not support.
        let ranged = text.replace("documented \"1.1.1\"", "since \"0.9.8\"");
        assert!(check_function(path, &ranged, "X509_free").is_err());
        // A range that excludes the pinned version.
        let removed = text.replace(
            "documented \"1.1.1\")",
            "documented \"1.1.1\", removed \"3.0.0\")",
        );
        assert!(check_function(path, &removed, "X509_free").is_err());
        // A signature the pinned prototype does not have.
        let typed = text.replace("X509_free(a: ptr<mut, u8>)", "X509_free(a: ptr<const, u8>)");
        assert!(check_function(path, &typed, "X509_free").is_err());
        // No range at all.
        let unranged = text.replace(
            "    requires resource_library_version(openssl, documented \"1.1.1\");\n",
            "",
        );
        assert!(check_function(path, &unranged, "X509_free").is_err());
    }

    /// Card REFERENCE-DROP-NULL-ACCEPTING: every NULL-accepting drop cites a
    /// fetched page at its range's series that records the NULL sentence,
    /// and no older series checked records it (the bound is the oldest).
    #[test]
    fn every_null_accepting_drop_cites_the_oldest_series_stating_it() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let pages = snapshots["pages"].as_array().unwrap();
        let mut seen = 0;
        for documented in DOCUMENTED {
            let Documented::NullAcceptingRelease { sentence, .. } = documented.shape else {
                continue;
            };
            seen += 1;
            let bound = LibraryVersion::parse(documented.series).unwrap();
            for page in pages
                .iter()
                .filter(|page| page["page"] == documented.page && page["fetched"] == true)
            {
                let series = LibraryVersion::parse(page["series"].as_str().unwrap()).unwrap();
                let states = page["null_sentences"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|stated| stated.as_str() == Some(sentence));
                if series < bound {
                    assert!(!states, "{}: {} states it", documented.function, series);
                }
                if series == bound {
                    assert!(
                        states,
                        "{}: {} does not state it",
                        documented.function, series
                    );
                }
            }
        }
        assert_eq!(seen, 6);
    }

    #[test]
    fn prototypes_map_to_contract_types() {
        assert_eq!(
            prototype_types(
                "SSL_CTX *SSL_CTX_new(const SSL_METHOD *meth)",
                "SSL_CTX_new"
            ),
            Some((vec!["const SSL_METHOD *".into()], "SSL_CTX *".into()))
        );
        assert_eq!(
            prototype_types("void SSL_CTX_free(SSL_CTX *)", "SSL_CTX_free"),
            Some((vec!["SSL_CTX *".into()], "void".into()))
        );
        assert_eq!(
            prototype_types(
                "int OPENSSL_init_ssl(uint64_t opts, const OPENSSL_INIT_SETTINGS *settings)",
                "OPENSSL_init_ssl"
            ),
            Some((
                vec!["uint64_t".into(), "const OPENSSL_INIT_SETTINGS *".into()],
                "int".into()
            ))
        );
        assert_eq!(
            prototype_types("void ERR_clear_error(void)", "ERR_clear_error"),
            Some((vec![], "void".into()))
        );
        assert_eq!(contract_type("const SSL_METHOD *"), Some("ptr<const, u8>"));
        assert_eq!(contract_type("SSL *"), Some("ptr<mut, u8>"));
        // Card OPENSSL-PROVIDER-REMAINING-CALLEES: the shapes its prototypes
        // add, each exactly one spelling, and no reading of anything else.
        assert_eq!(contract_type("long"), Some("i64"));
        assert_eq!(contract_type("unsigned long"), Some("u64"));
        assert_eq!(contract_type("size_t"), Some("usize"));
        assert_eq!(contract_type("X509 **"), Some("ptr<mut, ptr<mut, u8>>"));
        assert_eq!(
            contract_type("const char **"),
            Some("ptr<mut, ptr<const, u8>>")
        );
        assert_eq!(contract_type("pem_password_cb *"), Some("fnptr"));
        assert_eq!(contract_type("char *const *"), None);
        assert_eq!(contract_type("X509 ***"), None);
        assert_eq!(contract_type("pem_password_cb **"), None);
        // Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: `unsigned int` is
        // `u32`; `unsigned` alone and `unsigned char` have no spelling.
        assert_eq!(contract_type("unsigned int"), Some("u32"));
        assert_eq!(contract_type("unsigned"), None);
        assert_eq!(contract_type("unsigned char"), None);
        assert_eq!(
            prototype_types(
                "X509 *PEM_read_bio_X509(BIO *out, X509 **x, pem_password_cb *cb, void *u)",
                "PEM_read_bio_X509"
            ),
            Some((
                vec![
                    "BIO *".into(),
                    "X509 **".into(),
                    "pem_password_cb *".into(),
                    "void *".into()
                ],
                "X509 *".into()
            ))
        );
        assert_eq!(
            prototype_types(
                "void *OPENSSL_sk_value(const OPENSSL_STACK *, int)",
                "OPENSSL_sk_value"
            ),
            Some((
                vec!["const OPENSSL_STACK *".into(), "int".into()],
                "void *".into()
            ))
        );
    }

    fn module_of(function: &str) -> (&'static str, &'static str) {
        let (path, text) = crate::MODULES
            .iter()
            .find(|(_, text)| text.contains(&format!("extern fn {function}(")))
            .unwrap();
        (path, text)
    }

    #[test]
    fn a_drifted_shape_of_the_new_readings_is_refused() {
        // A borrowed result read as an owned one: a getter whose result the
        // page says the caller must not free.
        let (path, text) = module_of("SSL_get_certificate");
        let owned = text.replacen(
            "ensures normal resource_return_borrowed_from(ssl, nullable);",
            "ensures normal resource_return_owned(nullable);",
            1,
        );
        assert!(check_function(path, &owned, "SSL_get_certificate").is_err());
        // A retention dropped: the borrowed-and-forgotten reading C04.14
        // forbids.
        let (path, text) = module_of("OPENSSL_sk_push");
        let forgotten = text.replace("    ensures normal resource_retained_by(data, st);\n", "");
        assert!(check_function(path, &forgotten, "OPENSSL_sk_push").is_err());
        // A returned reference read as a fresh heap object (a final free
        // would then discharge it).
        let (path, text) = module_of("X509_new");
        let heap = text.replacen(
            "    ensures normal resource_return_kind(reference);\n",
            "",
            1,
        );
        assert!(check_function(path, &heap, "X509_new").is_err());
        // The unstated-slots premise removed from an I/O call.
        let (path, text) = module_of("SSL_write");
        let premise = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_SLOTS_STATEMENT}\");\n"
        );
        assert!(text.contains(&premise));
        let unpremised = text.replacen(&premise, "", 1);
        assert!(check_function(path, &unpremised, "SSL_write").is_err());
        // The null-argument premise removed: the reuse arm would read as the
        // fresh-object arm.
        let (path, text) = module_of("PEM_read_bio_X509");
        let widened = text.replacen("    requires resource_literal_argument(x, 0);\n", "", 1);
        assert!(check_function(path, &widened, "PEM_read_bio_X509").is_err());
        // Card OPENSSL-PEM-CALLBACK-ARM: the invocation removed (a caller's
        // password callback would read as a benign borrowed pointer), its
        // `u` actual dropped to `_`, the null default dropped (the plain
        // invocation), and the retired null-callback premise restored are
        // each refused.
        let invoked = "    ensures normal resource_callback_invoked_or_default(cb, _, _, _, u);\n";
        assert!(text.contains(invoked));
        assert!(!text.contains("resource_literal_argument(cb, 0)"));
        let benign = text.replacen(invoked, "", 1);
        assert!(check_function(path, &benign, "PEM_read_bio_X509").is_err());
        let unnamed = text.replacen(
            invoked,
            "    ensures normal resource_callback_invoked_or_default(cb, _, _, _, _);\n",
            1,
        );
        assert!(check_function(path, &unnamed, "PEM_read_bio_X509").is_err());
        let plain = text.replacen(
            invoked,
            "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n",
            1,
        );
        assert!(check_function(path, &plain, "PEM_read_bio_X509").is_err());
        let narrowed = text.replacen(
            "    requires resource_literal_argument(x, 0);\n",
            "    requires resource_literal_argument(x, 0);\n    requires resource_literal_argument(cb, 0);\n",
            1,
        );
        assert!(check_function(path, &narrowed, "PEM_read_bio_X509").is_err());
        // A release of a non-counted object read as a counted drop, and a
        // counted drop read as a final free.
        let (path, text) = module_of("PKCS12_free");
        let counted = text.replacen(
            "ensures normal resource_released(a, nullable);",
            "ensures normal resource_borrowed(a);\n    ensures normal resource_reference_dropped(a);",
            1,
        );
        assert!(check_function(path, &counted, "PKCS12_free").is_err());
        let (path, text) = module_of("X509_STORE_free");
        let freed = text.replacen(
            "    ensures normal resource_borrowed(xs);\n    ensures normal resource_reference_dropped(xs);\n",
            "    ensures normal resource_released(xs, nullable);\n",
            1,
        );
        assert!(check_function(path, &freed, "X509_STORE_free").is_err());
        // A since bound the HISTORY section does not give.
        let (path, text) = module_of("SSL_CTX_set_options");
        let older = text.replacen("since \"3.0.0\"", "documented \"1.0.2\"", 1);
        assert!(check_function(path, &older, "SSL_CTX_set_options").is_err());
    }

    /// Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: the slot readings drift-refused.
    #[test]
    fn a_drifted_slot_reading_is_refused() {
        // A stated invocation dropped: the certificate callback would read
        // benign under no clause at all.
        let (path, text) = module_of("SSL_write");
        let invocation =
            "    ensures normal resource_callback_slot_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _);\n";
        assert!(text.contains(invocation));
        let dropped = text.replacen(invocation, "", 1);
        assert!(check_function(path, &dropped, "SSL_write").is_err());
        // The invocation read through the wrong slot or with the connection
        // dropped from its actuals.
        let wrong = text.replacen(
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _)",
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.default_passwd_cb, ssl, _)",
            1,
        );
        assert!(check_function(path, &wrong, "SSL_write").is_err());
        let supplied = text.replacen(
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _)",
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.cert_cb, _, _)",
            1,
        );
        assert!(check_function(path, &supplied, "SSL_write").is_err());
        // The member form in place of the slot form.
        let member = text.replacen(
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _)",
            "resource_callback_member_invoked(ssl, ssl_ctx_st.cert_cb, ssl, _)",
            1,
        );
        assert!(check_function(path, &member, "SSL_write").is_err());
        // The premise-free loader given the premise back, and its stated
        // invocation removed.
        let (path, text) = module_of("SSL_CTX_use_PrivateKey_file");
        let premise = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_SLOTS_STATEMENT}\");\n"
        );
        let head = "    requires resource_library_version(openssl, documented \"1.0.2\");\n";
        let start = text.find("extern fn SSL_CTX_use_PrivateKey_file(").unwrap();
        let at = start + text[start..].find(head).unwrap() + head.len();
        let premised = format!("{}{premise}{}", &text[..at], &text[at..]);
        assert!(check_function(path, &premised, "SSL_CTX_use_PrivateKey_file").is_err());
        let unstated = text.replacen(
            "    ensures normal resource_callback_slot_invoked(ctx, ssl_ctx_st.default_passwd_cb, _, _, _, _);\n",
            "",
            1,
        );
        assert_ne!(unstated, text);
        assert!(check_function(path, &unstated, "SSL_CTX_use_PrivateKey_file").is_err());
        // A setter's retention of `arg` dropped (the borrowed-and-forgotten
        // reading), and its registration dropped.
        let (path, text) = module_of("SSL_CTX_set_cert_cb");
        let forgotten = text.replacen("    ensures normal resource_retained_by(arg, c);\n", "", 1);
        assert!(check_function(path, &forgotten, "SSL_CTX_set_cert_cb").is_err());
        let unregistered = text.replacen(
            "    ensures normal resource_callback_slot_registered(c, ssl_ctx_st.cert_cb, cert_cb);\n",
            "",
            1,
        );
        assert!(check_function(path, &unregistered, "SSL_CTX_set_cert_cb").is_err());
        // The frees keep the NULL-accepting drop and name the premise (card
        // OPENSSL-FREE-INDIRECT-CALLBACKS: in the statement of a free).
        let premise = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_FREE_STATEMENT}\");\n"
        );
        let (path, text) = module_of("SSL_free");
        let start = text.find("extern fn SSL_free(").unwrap();
        let at = start + text[start..].find(&premise).unwrap();
        let unpremised = format!("{}{}", &text[..at], &text[at + premise.len()..]);
        assert!(check_function(path, &unpremised, "SSL_free").is_err());
    }

    /// Card **OPENSSL-FREE-INDIRECT-CALLBACKS**: the lookup method table
    /// setters register their slots; `X509_STORE_free`, `SSL_CTX_free` and
    /// `SSL_free` state the shutdown and free slots with the library's
    /// `X509_LOOKUP` (`_`), name the built-in lookup premise, and carry the
    /// unstated-slots premise in the statement of a free; any other reading
    /// is refused.
    #[test]
    fn a_drifted_free_indirect_reading_is_refused() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let lookup_slots = SLOTS
            .iter()
            .filter(|slot| slot.page == "X509_LOOKUP_meth_new")
            .collect::<Vec<_>>();
        assert_eq!(lookup_slots.len(), 9);
        for slot in &lookup_slots {
            assert!(slot.key.starts_with("x509_lookup_method_st.lookup_meth_"));
            slot_writers_declared(slot.key, &snapshots).unwrap();
            let [writer] = slot.writers else {
                panic!("{}: one writer", slot.key)
            };
            let (path, text) = module_of(writer);
            assert_eq!(path, "openssl/x509_lookup_slots.qv");
            check_function(path, text, writer).unwrap();
            // The registration dropped.
            let start = text.find(&format!("extern fn {writer}(")).unwrap();
            let registration = format!(", {}, ", slot.key);
            let at = start + text[start..].find(&registration).unwrap();
            let moved = format!(
                "{}{}",
                &text[..at],
                text[at..].replacen(&registration, ", x509_store_st.verify_cb, ", 1)
            );
            assert!(check_function(path, &moved, writer).is_err(), "{writer}");
            // The letter release read as 1.1.0.
            let ranged = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen("documented \"1.1.1\"", "since \"1.1.0\"", 1)
            );
            assert!(check_function(path, &ranged, writer).is_err(), "{writer}");
        }
        // The page's NAME list loses a not-writer: refused.
        let mut stripped = snapshots.clone();
        for page in stripped["pages"].as_array_mut().unwrap() {
            if page["page"] == "X509_LOOKUP_meth_new" && page["series"] == "3.5" {
                let name = page["name"]
                    .as_str()
                    .unwrap()
                    .replace(" X509_OBJECT_set1_X509,", "");
                page["name"] = Value::from(name);
            }
        }
        assert!(
            slot_writers_declared("x509_lookup_method_st.lookup_meth_free", &stripped).is_err()
        );
        let unstated = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_FREE_STATEMENT}\");\n"
        );
        let builtin = format!(
            "    requires resource_premise({BUILTIN_LOOKUP_METHODS}, \"{BUILTIN_LOOKUP_METHODS_STATEMENT}\");\n"
        );
        for (call, object) in [
            ("X509_STORE_free", "xs"),
            ("SSL_CTX_free", "ctx"),
            ("SSL_free", "ssl"),
        ] {
            let (path, text) = module_of(call);
            check_function(path, text, call).unwrap();
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let edited = |from: &str, to: &str| {
                let edited = format!("{}{}", &text[..start], text[start..].replacen(from, to, 1));
                assert_ne!(edited, text, "{call}: {from}");
                edited
            };
            for slot in ["lookup_meth_shutdown", "lookup_meth_free"] {
                let line = format!(
                    "    ensures normal resource_callback_slot_invoked({object}, \
                     x509_lookup_method_st.{slot}, _);\n"
                );
                // Each invocation dropped.
                assert!(
                    check_function(path, &edited(&line, ""), call).is_err(),
                    "{call} {slot}"
                );
                // The freed object handed to the callback.
                let handed = edited(
                    &format!("x509_lookup_method_st.{slot}, _)"),
                    &format!("x509_lookup_method_st.{slot}, {object})"),
                );
                assert!(
                    check_function(path, &handed, call).is_err(),
                    "{call} {slot}"
                );
            }
            // Another lookup slot stated beside the drop.
            let extra = edited(
                "    ensures normal resource_return_none();\n",
                &format!(
                    "    ensures normal resource_callback_slot_invoked({object}, \
                     x509_lookup_method_st.lookup_meth_init, _);\n    ensures normal \
                     resource_return_none();\n"
                ),
            );
            assert!(check_function(path, &extra, call).is_err(), "{call}");
            // The built-in premise dropped.
            assert!(
                check_function(path, &edited(&builtin, ""), call).is_err(),
                "{call}"
            );
            // The unstated premise dropped, or read in the handshake's
            // statement.
            assert!(
                check_function(path, &edited(&unstated, ""), call).is_err(),
                "{call}"
            );
            let handshake = edited(UNSTATED_FREE_STATEMENT, UNSTATED_SLOTS_STATEMENT);
            assert!(check_function(path, &handshake, call).is_err(), "{call}");
            // A store slot stated at the free (the store's cleanup function
            // runs at a verification context's cleanup, not here).
            let cleanup = edited(
                "    ensures normal resource_return_none();\n",
                &format!(
                    "    ensures normal resource_callback_slot_invoked({object}, \
                     x509_store_st.cleanup_fn, _);\n    ensures normal \
                     resource_return_none();\n"
                ),
            );
            assert!(check_function(path, &cleanup, call).is_err(), "{call}");
        }
        // The handshake's statement still names the lookup methods (card
        // OPENSSL-LOOKUP-METHOD-HANDSHAKE-INVOCATIONS); the free's names
        // neither the lookup methods nor ex_data, and no key segment is a
        // bare common word.
        assert!(UNSTATED_SLOTS_STATEMENT.contains("X509_LOOKUP_METHOD"));
        assert!(!UNSTATED_FREE_STATEMENT.contains("X509_LOOKUP_METHOD"));
        assert!(!UNSTATED_FREE_STATEMENT.contains("ex_data"));
        for slot in &lookup_slots {
            let field = slot.key.rsplit('.').next().unwrap();
            assert!(!["free", "init", "shutdown", "ctrl", "new_item"].contains(&field));
        }
    }

    /// Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: every stated slot has every
    /// writer its page names declared as its setter; an unknown slot is
    /// refused.
    #[test]
    fn every_slot_has_every_writer_declared() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        for slot in SLOTS {
            slot_writers_declared(slot.key, &snapshots).unwrap();
        }
        assert_eq!(SLOTS.len(), 77);
        // Card OPENSSL-UNSTATED-CALLBACK-SLOTS: a ctrl arm stays unstated.
        assert!(slot_writers_declared("ssl_ctx_st.servername_cb", &snapshots).is_err());
        // A page that does not name a writer refuses the slot.
        let mut stripped = snapshots.clone();
        for page in stripped["pages"].as_array_mut().unwrap() {
            if page["page"] == "SSL_CTX_set_cert_cb" {
                page["name"] =
                    Value::from("SSL_CTX_set_cert_cb - handle certificate callback function");
            }
        }
        assert!(slot_writers_declared("ssl_ctx_st.cert_cb", &stripped).is_err());
        // Every entry that registers or invokes a slot names a known one.
        for documented in DOCUMENTED {
            for slot in stated_slots(documented) {
                assert!(SLOTS.iter().any(|known| known.key == slot), "{slot}");
            }
        }
    }

    /// Card OPENSSL-UNSTATED-CALLBACK-SLOTS: a page's NAME list is exactly
    /// the writers and not-writers of the slots citing it, so a writer the
    /// page names is never left out, and a writer it does not name refuses.
    #[test]
    fn a_page_naming_an_unlisted_function_refuses_its_slots() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let rename = |page_name: &str, name: &str| {
            let mut changed = snapshots.clone();
            for page in changed["pages"].as_array_mut().unwrap() {
                if page["page"] == page_name && page["series"] == "3.5" {
                    page["name"] = Value::from(name);
                }
            }
            changed
        };
        // A setter the page names that no slot lists (a writer left out).
        let extra = rename(
            "SSL_CTX_set_info_callback",
            "SSL_CTX_set_info_callback, SSL_CTX_get_info_callback, SSL_set_info_callback, \
             SSL_get_info_callback, SSL_set_other_info_callback - handle information callback",
        );
        let refused = slot_writers_declared("ssl_ctx_st.info_callback", &extra).unwrap_err();
        assert!(refused.contains("SSL_set_other_info_callback"), "{refused}");
        // A writer the page no longer names.
        let missing = rename(
            "SSL_CTX_set_verify",
            "SSL_CTX_set_verify, SSL_CTX_set_verify_depth - set peer certificate verification \
             parameters",
        );
        assert!(slot_writers_declared("ssl_ctx_st.verify_callback", &missing).is_err());
        // Three slots of one page: a getter dropped from the page refuses
        // each of them.
        let session = rename(
            "SSL_CTX_sess_set_get_cb",
            "SSL_CTX_sess_set_new_cb, SSL_CTX_sess_set_remove_cb, SSL_CTX_sess_set_get_cb - \
             provide callback functions",
        );
        for key in [
            "ssl_ctx_st.new_session_cb",
            "ssl_ctx_st.remove_session_cb",
            "ssl_ctx_st.get_session_cb",
        ] {
            assert!(slot_writers_declared(key, &session).is_err(), "{key}");
        }
    }

    /// Card OPENSSL-UNSTATED-CALLBACK-SLOTS: the new slot readings
    /// drift-refused.
    #[test]
    fn a_drifted_unstated_slot_reading_is_refused() {
        let premise = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_SLOTS_STATEMENT}\");\n"
        );
        let ex_data =
            format!("    requires resource_premise({EX_DATA}, \"{EX_DATA_STATEMENT}\");\n");
        // Each newly stated invocation of a handshake call dropped, or read
        // with a supplied value where the page names the connection.
        let (path, text) = module_of("SSL_read");
        let start = text.find("extern fn SSL_read(").unwrap();
        let end = start + text[start..].find("};").unwrap();
        let body = &text[start..end];
        for slot in [
            "verify_callback",
            "info_callback",
            "msg_callback",
            "sec_cb",
            "new_session_cb",
            "remove_session_cb",
            "get_session_cb",
            "client_cert_cb",
        ] {
            let line = body
                .lines()
                .find(|line| line.contains(&format!("ssl_ctx_st.{slot},")))
                .unwrap_or_else(|| panic!("SSL_read states {slot}"));
            let dropped = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen(&format!("{line}\n"), "", 1)
            );
            assert!(
                check_function(path, &dropped, "SSL_read").is_err(),
                "{slot}"
            );
        }
        let supplied = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(
                "resource_callback_slot_invoked(ssl, ssl_ctx_st.info_callback, ssl, _, _)",
                "resource_callback_slot_invoked(ssl, ssl_ctx_st.info_callback, _, _, _)",
                1,
            )
        );
        assert_ne!(supplied, text);
        assert!(check_function(path, &supplied, "SSL_read").is_err());
        // The handshake calls keep the narrowed premise (the ctrl arms).
        let unpremised = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(&premise, "", 1)
        );
        assert!(check_function(path, &unpremised, "SSL_read").is_err());
        // SSL_CTX_use_certificate: its security invocation dropped, the
        // caller's certificate read as a supplied value, the context's check
        // read as the connection's.
        let (path, text) = module_of("SSL_CTX_use_certificate");
        let invocation = "    ensures normal resource_callback_slot_invoked(ctx, ssl_ctx_st.sec_cb, supplied_nullable, ctx, _, _, _, x, supplied_nullable);\n";
        assert!(text.contains(invocation));
        let silent = text.replacen(invocation, "", 1);
        assert!(check_function(path, &silent, "SSL_CTX_use_certificate").is_err());
        let supplied = text.replacen(
            "ssl_ctx_st.sec_cb, supplied_nullable, ctx, _, _, _, x, supplied_nullable)",
            "ssl_ctx_st.sec_cb, supplied_nullable, ctx, _, _, _, _, supplied_nullable)",
            1,
        );
        assert!(check_function(path, &supplied, "SSL_CTX_use_certificate").is_err());
        let connection = text.replacen(
            "ssl_ctx_st.sec_cb, supplied_nullable, ctx, _, _, _, x, supplied_nullable)",
            "ssl_ctx_st.sec_cb, ctx, supplied_nullable, _, _, _, x, supplied_nullable)",
            1,
        );
        assert!(check_function(path, &connection, "SSL_CTX_use_certificate").is_err());
        // SSL_CTX_use_certificate_chain_file: the slot premise given back in
        // place of the ex_data one, and the ex_data premise dropped.
        // Card OPENSSL-EX-DATA-REGISTRY: other declarations of the module
        // name the ex_data premise too, so the drift is applied to this one.
        let (path, text) = module_of("SSL_CTX_use_certificate_chain_file");
        let start = text
            .find("extern fn SSL_CTX_use_certificate_chain_file(")
            .unwrap();
        assert!(text[start..].contains(&ex_data));
        let widened = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(&ex_data, &premise, 1)
        );
        assert!(check_function(path, &widened, "SSL_CTX_use_certificate_chain_file").is_err());
        let silent = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(&ex_data, "", 1)
        );
        assert!(check_function(path, &silent, "SSL_CTX_use_certificate_chain_file").is_err());
        // A setter registering the wrong slot, or a callback the prototype
        // does not give it.
        let (path, text) = module_of("SSL_CTX_set_verify");
        let wrong = text.replacen(
            "resource_callback_slot_registered(ctx, ssl_ctx_st.verify_callback, verify_callback)",
            "resource_callback_slot_registered(ctx, ssl_ctx_st.cert_cb, verify_callback)",
            1,
        );
        assert!(check_function(path, &wrong, "SSL_CTX_set_verify").is_err());
        // SSL_shutdown keeps its premise beside the info and message
        // invocations.
        let (path, text) = module_of("SSL_shutdown");
        let start = text.find("extern fn SSL_shutdown(").unwrap();
        let unpremised = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(&premise, "", 1)
        );
        assert!(check_function(path, &unpremised, "SSL_shutdown").is_err());
        // The pointer typedef is `fnptr` only by value.
        assert_eq!(contract_type("SSL_verify_cb"), Some("fnptr"));
        assert_eq!(contract_type("SSL_verify_cb *"), None);
    }

    #[test]
    fn an_in_place_function_pointer_is_fnptr() {
        assert_eq!(
            prototype_types(
                "void SSL_CTX_set_cert_cb(SSL_CTX *c, int (*cb)(SSL *ssl, void *arg), void *arg)",
                "SSL_CTX_set_cert_cb"
            ),
            Some((
                vec![
                    "SSL_CTX *".into(),
                    "int (*)(SSL *, void *)".into(),
                    "void *".into()
                ],
                "void".into()
            ))
        );
        assert_eq!(contract_type("int (*)(SSL *, void *)"), Some("fnptr"));
        assert_eq!(contract_type("int (**)(SSL *, void *)"), None);
        assert_eq!(contract_type("int (*)(int (*)(void))"), None);
        assert_eq!(contract_type("(*)(void)"), None);
    }

    /// Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: the handshake slots this
    /// card states, drift-refused.
    #[test]
    fn a_drifted_handshake_slot_reading_is_refused() {
        let premise = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_SLOTS_STATEMENT}\");\n"
        );
        // Each new invocation of a handshake call dropped.
        let (path, text) = module_of("SSL_connect");
        let start = text.find("extern fn SSL_connect(").unwrap();
        let end = start + text[start..].find("};").unwrap();
        let body = &text[start..end];
        let new_slots = [
            "keylog_callback",
            "alpn_select_cb",
            "npn_select_cb",
            "npn_advertised_cb",
            "app_verify_callback",
            "psk_client_callback",
            "psk_use_session_cb",
            "psk_server_callback",
            "psk_find_session_cb",
            "client_hello_cb",
            "record_padding_cb",
            "allow_early_data_cb",
            "custom_ext_add_cb_ex",
            "custom_ext_free_cb_ex",
            "custom_ext_parse_cb_ex",
            "custom_ext_add_cb",
            "custom_ext_free_cb",
            "custom_ext_parse_cb",
        ];
        for slot in new_slots {
            let line = body
                .lines()
                .find(|line| line.contains(&format!("ssl_ctx_st.{slot},")))
                .unwrap_or_else(|| panic!("SSL_connect states {slot}"));
            let dropped = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen(&format!("{line}\n"), "", 1)
            );
            assert!(
                check_function(path, &dropped, "SSL_connect").is_err(),
                "{slot}"
            );
        }
        // The connection read as a supplied value, and the keylog line read as
        // the connection.
        let supplied = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(
                "ssl_ctx_st.keylog_callback, ssl, _)",
                "ssl_ctx_st.keylog_callback, _, _)",
                1,
            )
        );
        assert_ne!(supplied, text);
        assert!(check_function(path, &supplied, "SSL_connect").is_err());
        // The certificate verification callback receives no connection.
        let connected = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(
                "ssl_ctx_st.app_verify_callback, _, _)",
                "ssl_ctx_st.app_verify_callback, ssl, _)",
                1,
            )
        );
        assert_ne!(connected, text);
        assert!(check_function(path, &connected, "SSL_connect").is_err());
        // The handshake calls keep the narrowed premise (the ctrl arms, CT,
        // ex_data).
        let unpremised = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(&premise, "", 1)
        );
        assert!(check_function(path, &unpremised, "SSL_connect").is_err());
        // SSL_shutdown's record padding invocation dropped.
        let (path, text) = module_of("SSL_shutdown");
        let start = text.find("extern fn SSL_shutdown(").unwrap();
        let padding = "    ensures normal resource_callback_slot_invoked(ssl, ssl_ctx_st.record_padding_cb, ssl, _, _, _);\n";
        assert!(text[start..].contains(padding));
        let silent = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(padding, "", 1)
        );
        assert!(check_function(path, &silent, "SSL_shutdown").is_err());
        // A setter's retention of `arg` dropped (borrowed and forgotten).
        let (path, text) = module_of("SSL_CTX_set_alpn_select_cb");
        let forgotten = text.replacen(
            "    ensures normal resource_retained_by(arg, ctx);\n",
            "",
            1,
        );
        assert_ne!(forgotten, text);
        assert!(check_function(path, &forgotten, "SSL_CTX_set_alpn_select_cb").is_err());
        // A multi-slot setter: each registration and the second retention are
        // required, and an `_ex` callback is not an older-style one.
        let (path, text) = module_of("SSL_CTX_add_custom_ext");
        let start = text.find("extern fn SSL_CTX_add_custom_ext(").unwrap();
        for line in [
            "    ensures normal resource_callback_slot_registered(ctx, ssl_ctx_st.custom_ext_add_cb_ex, add_cb);\n",
            "    ensures normal resource_callback_slot_registered(ctx, ssl_ctx_st.custom_ext_free_cb_ex, free_cb);\n",
            "    ensures normal resource_callback_slot_registered(ctx, ssl_ctx_st.custom_ext_parse_cb_ex, parse_cb);\n",
            "    ensures normal resource_retained_by(add_arg, ctx);\n",
            "    ensures normal resource_retained_by(parse_arg, ctx);\n",
        ] {
            assert!(text[start..].contains(line), "{line}");
            let dropped = format!("{}{}", &text[..start], text[start..].replacen(line, "", 1));
            assert!(
                check_function(path, &dropped, "SSL_CTX_add_custom_ext").is_err(),
                "{line}"
            );
        }
        let old_style = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(
                "ssl_ctx_st.custom_ext_free_cb_ex, free_cb)",
                "ssl_ctx_st.custom_ext_free_cb, free_cb)",
                1,
            )
        );
        assert!(check_function(path, &old_style, "SSL_CTX_add_custom_ext").is_err());
        // The new callback types are `fnptr` only by value.
        for typedef in FUNCTION_POINTER_TYPEDEFS {
            assert_eq!(contract_type(typedef), Some("fnptr"), "{typedef}");
            assert_eq!(contract_type(&format!("{typedef} *")), None, "{typedef}");
        }
    }

    /// Card OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: a setter registering
    /// several slots is a writer of each; dropping one registration from its
    /// documented entry leaves that slot's writer undeclared, and a slot no
    /// page lists (the CT validation callback, written by the library's own
    /// `SSL_CTX_enable_ct`) is not one this checker documents.
    #[test]
    fn a_multi_slot_writer_writes_each_slot() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let custom = DOCUMENTED
            .iter()
            .find(|entry| entry.function == "SSL_CTX_add_custom_ext")
            .unwrap();
        assert_eq!(
            registered_slots(custom),
            vec![
                "ssl_ctx_st.custom_ext_free_cb_ex",
                "ssl_ctx_st.custom_ext_parse_cb_ex",
                "ssl_ctx_st.custom_ext_add_cb_ex",
            ]
        );
        for key in [
            "ssl_ctx_st.custom_ext_add_cb_ex",
            "ssl_ctx_st.custom_ext_free_cb_ex",
            "ssl_ctx_st.custom_ext_parse_cb_ex",
            "ssl_ctx_st.custom_ext_add_cb",
            "ssl_ctx_st.custom_ext_free_cb",
            "ssl_ctx_st.custom_ext_parse_cb",
        ] {
            slot_writers_declared(key, &snapshots).unwrap();
        }
        assert!(slot_writers_declared("ssl_ctx_st.ct_validation_callback", &snapshots).is_err());
        // A page naming a writer of the custom extension slots that no slot
        // lists refuses all six.
        let mut extra = snapshots.clone();
        for page in extra["pages"].as_array_mut().unwrap() {
            if page["page"] == "SSL_extension_supported" && page["series"] == "3.5" {
                let name = page["name"].as_str().unwrap().replacen(
                    "SSL_CTX_add_server_custom_ext,",
                    "SSL_CTX_add_server_custom_ext, SSL_CTX_add_other_custom_ext,",
                    1,
                );
                page["name"] = Value::from(name);
            }
        }
        for key in [
            "ssl_ctx_st.custom_ext_add_cb_ex",
            "ssl_ctx_st.custom_ext_parse_cb",
        ] {
            let refused = slot_writers_declared(key, &extra).unwrap_err();
            assert!(
                refused.contains("SSL_CTX_add_other_custom_ext"),
                "{refused}"
            );
        }
        // Every page this card cites is fetched at 3.5 with its NAME.
        for slot in SLOTS {
            assert!(
                snapshots["pages"].as_array().unwrap().iter().any(|page| {
                    page["page"] == slot.page
                        && page["series"] == "3.5"
                        && page["fetched"] == true
                        && page["name"].is_string()
                }),
                "{}",
                slot.page
            );
        }
    }

    /// Card OPENSSL-BIO-METHODS-AND-CALLBACKS: the BIO callback and method
    /// slots, the built-in methods premise and the BIO free premise, each
    /// drift-refused.
    #[test]
    fn a_drifted_bio_slot_reading_is_refused() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let bio_keys = [
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
        for key in bio_keys {
            slot_writers_declared(key, &snapshots).unwrap();
        }
        let builtin = format!(
            "    requires resource_premise({BUILTIN_BIO_METHODS}, \"{BUILTIN_BIO_METHODS_STATEMENT}\");\n"
        );
        let freed =
            format!("    requires resource_premise({BIO_FREE}, \"{BIO_FREE_STATEMENT}\");\n");
        // Each I/O call and SSL_shutdown: every BIO invocation dropped, an
        // actual read as the connection, the built-in premise dropped.
        for (call, object) in [
            ("SSL_write", "ssl"),
            ("SSL_read", "ssl"),
            ("SSL_connect", "ssl"),
            ("SSL_do_handshake", "s"),
            ("SSL_shutdown", "ssl"),
        ] {
            let (path, text) = module_of(call);
            check_function(path, text, call).unwrap();
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let end = start + text[start..].find("};").unwrap();
            let body = &text[start..end];
            for key in bio_keys {
                let line = body
                    .lines()
                    .find(|line| line.contains(&format!("({object}, {key}, ")))
                    .unwrap_or_else(|| panic!("{call} states {key}"));
                let dropped = format!(
                    "{}{}",
                    &text[..start],
                    text[start..].replacen(&format!("{line}\n"), "", 1)
                );
                assert!(
                    check_function(path, &dropped, call).is_err(),
                    "{call} {key}"
                );
            }
            let connected = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen(
                    &format!("({object}, bio_st.bio_callback_ex, _,"),
                    &format!("({object}, bio_st.bio_callback_ex, {object},"),
                    1,
                )
            );
            assert_ne!(connected, text);
            assert!(check_function(path, &connected, call).is_err(), "{call}");
            assert!(body.contains(builtin.trim_end()), "{call}");
            let unpremised = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen(&builtin, "", 1)
            );
            assert!(check_function(path, &unpremised, call).is_err(), "{call}");
            // The narrowed statement: the old one, naming a BIO's methods,
            // is no longer the premise.
            let old = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen(
                    "of the SSL_CTX holds, or an ex_data callback)",
                    "of the SSL_CTX holds, a method or callback of a BIO the SSL object holds, \
                     or an ex_data callback)",
                    1,
                )
            );
            assert_ne!(old, text);
            assert!(check_function(path, &old, call).is_err(), "{call}");
        }
        // Card OPENSSL-BIO-READER-INVOCATIONS: the PEM and PKCS#12 readers
        // state the 14 slots on their BIO formal beside the premise: each
        // invocation dropped, the BIO given as the callback's actual, the
        // built-in premise dropped or read in its old SSL-only statement, and
        // the literal-argument premise dropped each refuse.
        let ssl_only = BUILTIN_BIO_METHODS_STATEMENT.replacen(
            "run on a BIO the call reads or writes through or frees (its BIO argument or a BIO \
             the SSL object holds) or one chained to it",
            "run on a BIO the SSL object holds or one chained to it",
            1,
        );
        assert_ne!(ssl_only, BUILTIN_BIO_METHODS_STATEMENT);
        for (call, object, premise) in [
            ("PEM_read_bio_X509", "out", "x"),
            ("PEM_read_bio_PrivateKey", "out", "x"),
            ("d2i_PKCS12_bio", "bp", "p12"),
        ] {
            let (path, text) = module_of(call);
            check_function(path, text, call).unwrap();
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let end = start + text[start..].find("};").unwrap();
            let body = &text[start..end];
            let edited = |from: &str, to: &str| {
                let edited = format!("{}{}", &text[..start], text[start..].replacen(from, to, 1));
                assert_ne!(edited, text, "{call}: {from}");
                edited
            };
            for key in bio_keys {
                let line = body
                    .lines()
                    .find(|line| line.contains(&format!("({object}, {key}, ")))
                    .unwrap_or_else(|| panic!("{call} states {key}"));
                let dropped = edited(&format!("{line}\n"), "");
                assert!(
                    check_function(path, &dropped, call).is_err(),
                    "{call} {key}"
                );
            }
            let handed = edited(
                &format!("({object}, bio_st.bio_callback_ex, _,"),
                &format!("({object}, bio_st.bio_callback_ex, {object},"),
            );
            assert!(check_function(path, &handed, call).is_err(), "{call}");
            assert!(body.contains(builtin.trim_end()), "{call}");
            let unpremised = edited(&builtin, "");
            assert!(check_function(path, &unpremised, call).is_err(), "{call}");
            let old = edited(BUILTIN_BIO_METHODS_STATEMENT, &ssl_only);
            assert!(check_function(path, &old, call).is_err(), "{call}");
            let unrestricted = edited(
                &format!("    requires resource_literal_argument({premise}, 0);\n"),
                "",
            );
            assert!(check_function(path, &unrestricted, call).is_err(), "{call}");
        }
        // The old statement is no longer the premise on the I/O calls either.
        let (path, text) = module_of("SSL_write");
        let old = text.replacen(BUILTIN_BIO_METHODS_STATEMENT, &ssl_only, 1);
        assert_ne!(old, text);
        assert!(check_function(path, &old, "SSL_write").is_err());
        // Card OPENSSL-FREE-SESSION-REMOVE-INVOCATION: BIO_free and SSL_free
        // state the callback and destroy slots beside their drop (C04.19
        // amended), on the freed object with every actual `_`, beside the
        // built-in premise and the free premise narrowed to ex_data. Each
        // invocation dropped, the freed BIO handed to the callback, the
        // built-in premise dropped, the free premise dropped or read in its
        // old statement (covering the callback and destroy function), and
        // the frees' old reading (premise, no invocation) each refuse.
        let old_freed = freed.replacen(
            "the ex_data free callbacks of the BIO class",
            "the callback the BIO holds, run with BIO_CB_FREE, the destroy function of its \
             method and the ex_data free callbacks of the BIO class",
            1,
        );
        assert_ne!(old_freed, freed);
        let free_keys = [
            "bio_st.bio_callback_ex",
            "bio_st.bio_callback",
            "bio_method_st.meth_destroy",
        ];
        for (call, object) in [("BIO_free", "a"), ("SSL_free", "ssl")] {
            let (path, text) = module_of(call);
            check_function(path, text, call).unwrap();
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let end = start + text[start..].find("};").unwrap();
            let body = &text[start..end];
            let edited = |from: &str, to: &str| {
                let edited = format!("{}{}", &text[..start], text[start..].replacen(from, to, 1));
                assert_ne!(edited, text, "{call}: {from}");
                edited
            };
            assert!(body.contains(freed.trim_end()), "{call}");
            assert!(body.contains(builtin.trim_end()), "{call}");
            let mut lines = Vec::new();
            for key in free_keys {
                let line = body
                    .lines()
                    .find(|line| line.contains(&format!("({object}, {key}, ")))
                    .unwrap_or_else(|| panic!("{call} states {key}"));
                lines.push(line);
                let dropped = edited(&format!("{line}\n"), "");
                assert!(
                    check_function(path, &dropped, call).is_err(),
                    "{call} {key}"
                );
            }
            // No other BIO slot is stated beside the drop.
            for key in bio_keys {
                assert_eq!(
                    body.matches(&format!("({object}, {key}, ")).count(),
                    usize::from(free_keys.contains(&key)),
                    "{call} {key}"
                );
            }
            let handed = edited(
                &format!("({object}, bio_st.bio_callback_ex, _,"),
                &format!("({object}, bio_st.bio_callback_ex, {object},"),
            );
            assert!(check_function(path, &handed, call).is_err(), "{call}");
            let silent = edited(&freed, "");
            assert!(check_function(path, &silent, call).is_err(), "{call}");
            let unbuilt = edited(&builtin, "");
            assert!(check_function(path, &unbuilt, call).is_err(), "{call}");
            let widened = edited(&freed, &old_freed);
            assert!(check_function(path, &widened, call).is_err(), "{call}");
            let mut old = text[start..].to_owned();
            for line in &lines {
                old = old.replacen(&format!("{line}\n"), "", 1);
            }
            let old = format!(
                "{}{}",
                &text[..start],
                old.replacen(&builtin, "", 1)
                    .replacen(&freed, &old_freed, 1)
            );
            assert!(check_function(path, &old, call).is_err(), "{call}");
        }
        // The session-remove invocation on both frees: dropped, or handed
        // the freed object, refuses; the frees keep the unstated premise.
        let unstated = format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{UNSTATED_FREE_STATEMENT}\");\n"
        );
        for (call, object) in [("SSL_CTX_free", "ctx"), ("SSL_free", "ssl")] {
            let (path, text) = module_of(call);
            check_function(path, text, call).unwrap();
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let line = format!(
                "    ensures normal resource_callback_slot_invoked({object}, \
                 ssl_ctx_st.remove_session_cb, _, _);\n"
            );
            assert!(text[start..].contains(&line), "{call}");
            let edited = |from: &str, to: &str| {
                let edited = format!("{}{}", &text[..start], text[start..].replacen(from, to, 1));
                assert_ne!(edited, text, "{call}: {from}");
                edited
            };
            let dropped = edited(&line, "");
            assert!(check_function(path, &dropped, call).is_err(), "{call}");
            let handed = edited(
                &format!("({object}, ssl_ctx_st.remove_session_cb, _,"),
                &format!("({object}, ssl_ctx_st.remove_session_cb, {object},"),
            );
            assert!(check_function(path, &handed, call).is_err(), "{call}");
            let unpremised = edited(&unstated, "");
            assert!(check_function(path, &unpremised, call).is_err(), "{call}");
        }
        // A setter's registration moved to another slot, or dropped.
        let (path, text) = module_of("BIO_meth_set_write");
        let moved = text.replacen(
            "(biom, bio_method_st.meth_write, write)",
            "(biom, bio_method_st.meth_write_ex, write)",
            1,
        );
        assert_ne!(moved, text);
        assert!(check_function(path, &moved, "BIO_meth_set_write").is_err());
        let (path, text) = module_of("BIO_set_callback");
        let dropped = text.replacen(
            "    ensures normal resource_callback_slot_registered(b, bio_st.bio_callback, callback);\n",
            "",
            1,
        );
        assert_ne!(dropped, text);
        assert!(check_function(path, &dropped, "BIO_set_callback").is_err());
        // A page naming one more method setter refuses every method slot; a
        // built-in method is not a slot this checker documents.
        let mut extra = snapshots.clone();
        for page in extra["pages"].as_array_mut().unwrap() {
            if page["page"] == "BIO_meth_new" && page["series"] == "3.5" {
                let name = page["name"].as_str().unwrap().replacen(
                    "BIO_meth_set_ctrl,",
                    "BIO_meth_set_ctrl, BIO_meth_set_other,",
                    1,
                );
                page["name"] = Value::from(name);
            }
        }
        for key in &bio_keys[2..] {
            let refused = slot_writers_declared(key, &extra).unwrap_err();
            assert!(refused.contains("BIO_meth_set_other"), "{refused}");
        }
        assert!(slot_writers_declared("bio_method_st.socket_write", &snapshots).is_err());
        // The new callback types are `fnptr` only by value.
        for typedef in ["BIO_callback_fn_ex", "BIO_callback_fn"] {
            assert_eq!(contract_type(typedef), Some("fnptr"));
            assert_eq!(contract_type(&format!("{typedef} *")), None);
        }
    }

    #[test]
    fn every_documented_page_was_fetched() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        for documented in DOCUMENTED {
            assert!(
                snapshots["pages"].as_array().unwrap().iter().any(|page| {
                    page["page"] == documented.page
                        && page["series"] == documented.series
                        && page["fetched"] == true
                }),
                "{}: {}@{}",
                documented.function,
                documented.page,
                documented.series
            );
        }
    }

    /// Card CALLBACK-SUPPLIED-NULL-ACTUAL: every nullable argument is cited
    /// from its page and used; a dropped or an uncited mark is refused.
    #[test]
    fn a_drifted_nullable_argument_is_refused() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        assert_eq!(NULLABLE_ARGUMENTS.len(), 10);
        for nullable in NULLABLE_ARGUMENTS {
            nullable_argument_cited(nullable, &snapshots)
                .unwrap_or_else(|failure| panic!("{nullable:?}: {failure}"));
        }
        // A sentence the snapshot does not record.
        let uncited = NullableArgument {
            sentence: "The connection may be NULL.",
            ..NULLABLE_ARGUMENTS[0]
        };
        let refused = nullable_argument_cited(&uncited, &snapshots).unwrap_err();
        assert!(refused.contains("does not record"), "{refused}");
        // A position every invocation fills with a parameter (`s` of the
        // PSK client callback is the connection): vacuous.
        let vacuous = NullableArgument {
            slot: "ssl_ctx_st.psk_client_callback",
            position: 0,
            ..NULLABLE_ARGUMENTS[4]
        };
        let refused = nullable_argument_cited(&vacuous, &snapshots).unwrap_err();
        assert!(refused.contains("no documented invocation"), "{refused}");
        // A slot the checker does not state.
        let unknown = NullableArgument {
            slot: "ssl_ctx_st.no_such_cb",
            ..NULLABLE_ARGUMENTS[0]
        };
        assert!(nullable_argument_cited(&unknown, &snapshots).is_err());
        // The cited snapshot without its argument sentences.
        let mut stripped = snapshots.clone();
        for page in stripped["pages"].as_array_mut().unwrap() {
            if page["page"] == "SSL_CTX_set_security_level" {
                page.as_object_mut()
                    .unwrap()
                    .remove("argument_null_sentences");
            }
        }
        assert!(nullable_arguments_cited("ssl_ctx_st.sec_cb", &stripped).is_err());
        // The declarations: a mark dropped (read as `_`, a value the callee
        // always fills), or a `_` read as nullable where no page says so.
        let (path, text) = module_of("SSL_read");
        let start = text.find("extern fn SSL_read(").unwrap();
        let (head, tail) = text.split_at(start);
        let in_read = |from: &str, to: &str| format!("{head}{}", tail.replacen(from, to, 1));
        let marked = "resource_callback_slot_invoked(ssl, ssl_ctx_st.sec_cb, ssl, supplied_nullable, _, _, _, supplied_nullable, supplied_nullable)";
        assert!(tail.contains(marked));
        for drifted in [
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.sec_cb, ssl, _, _, _, _, supplied_nullable, supplied_nullable)",
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.sec_cb, ssl, supplied_nullable, _, _, _, _, supplied_nullable)",
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.sec_cb, ssl, supplied_nullable, supplied_nullable, _, _, supplied_nullable, supplied_nullable)",
        ] {
            let changed = in_read(marked, drifted);
            assert!(
                check_function(path, &changed, "SSL_read").is_err(),
                "{drifted}"
            );
        }
        let unmarked = in_read(
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.psk_client_callback, ssl, supplied_nullable,",
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.psk_client_callback, ssl, _,",
        );
        assert_ne!(unmarked, text);
        assert!(check_function(path, &unmarked, "SSL_read").is_err());
        let invented = in_read(
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.info_callback, ssl, _, _)",
            "resource_callback_slot_invoked(ssl, ssl_ctx_st.info_callback, ssl, supplied_nullable, _)",
        );
        assert_ne!(invented, text);
        assert!(check_function(path, &invented, "SSL_read").is_err());
        // The readers' BIO callbacks.
        let (path, text) = module_of("PEM_read_bio_X509");
        let unmarked = text.replacen(
            "resource_callback_slot_invoked(out, bio_st.bio_callback, _, _, supplied_nullable, _, _, _)",
            "resource_callback_slot_invoked(out, bio_st.bio_callback, _, _, _, _, _, _)",
            1,
        );
        assert_ne!(unmarked, text);
        assert!(check_function(path, &unmarked, "PEM_read_bio_X509").is_err());
    }

    /// Card OPENSSL-EX-DATA-REGISTRY: every declaration the audit reads as
    /// running ex_data callbacks names the premise in its module, dropping it
    /// refuses, and every constructor and free is audited.
    #[test]
    fn every_ex_data_class_object_names_the_premise() {
        let ex_data =
            format!("    requires resource_premise({EX_DATA}, \"{EX_DATA_STATEMENT}\");\n");
        let mut runs = 0;
        for (function, reading) in EX_DATA_AUDIT {
            let documented = DOCUMENTED
                .iter()
                .filter(|entry| entry.function == *function)
                .collect::<Vec<_>>();
            assert_eq!(documented.len(), 1, "{function} is documented once");
            assert_eq!(
                EX_DATA_AUDIT.iter().filter(|(f, _)| f == function).count(),
                1,
                "{function} is audited once"
            );
            let (path, text) = module_of(function);
            let start = text.find(&format!("extern fn {function}(")).unwrap();
            let end = start + text[start..].find("};").unwrap();
            let declared = text[start..end].contains(ex_data.trim());
            match reading {
                ExData::Runs { classes, .. } => {
                    runs += 1;
                    assert!(declared, "{function} names {EX_DATA}");
                    assert!(!classes.is_empty(), "{function}");
                    for class in *classes {
                        assert!(EX_DATA_CLASSES.contains(class), "{function}: {class}");
                    }
                    let dropped = format!(
                        "{}{}",
                        &text[..start],
                        text[start..].replacen(&ex_data, "", 1)
                    );
                    assert_ne!(dropped, *text, "{function}");
                    let refused = check_function(path, &dropped, function).unwrap_err();
                    assert!(refused.contains(EX_DATA), "{function}: {refused}");
                }
                ExData::NoClass { .. } => {
                    assert!(!declared, "{function} names no ex_data premise");
                    assert!(creates_or_frees(documented[0].shape), "{function}");
                }
            }
        }
        // The audit's count: the 29 declarations this card premised and
        // SSL_CTX_use_certificate_chain_file.
        assert_eq!(runs, 30);
        for documented in DOCUMENTED {
            let audited = EX_DATA_AUDIT.iter().any(|(f, _)| *f == documented.function);
            if creates_or_frees(documented.shape) {
                assert!(
                    audited,
                    "{} creates or frees an object",
                    documented.function
                );
            }
            if documented.premises.contains(&Premise::ExDataCallbacks) {
                assert!(
                    EX_DATA_AUDIT
                        .iter()
                        .any(|(f, reading)| *f == documented.function
                            && matches!(reading, ExData::Runs { .. })),
                    "{}",
                    documented.function
                );
            }
        }
        // Every module names the premise only where the audit reads it.
        let named = crate::MODULES
            .iter()
            .map(|(_, text)| strip_comments(text).matches(ex_data.trim()).count())
            .sum::<usize>();
        assert_eq!(named, runs);
    }

    /// Card OPENSSL-EX-DATA-REGISTRY: each way the audit can drift refuses.
    #[test]
    fn a_drifted_ex_data_reading_is_refused() {
        const RUNS: ExData = ExData::Runs {
            classes: &["X509"],
            how: "allocates a certificate",
        };
        const NONE: ExData = ExData::NoClass { why: "no class" };
        let premised = |shape| DocumentedFunction {
            function: "X509_new",
            page: "X509_new",
            series: "1.0.2",
            range: "",
            shape,
            premises: &[Premise::ExDataCallbacks],
        };
        let bare = |shape| DocumentedFunction {
            function: "X509_new",
            page: "X509_new",
            series: "1.0.2",
            range: "",
            shape,
            premises: &[],
        };
        let constructor = Documented::CountedConstructor;
        // As landed.
        ex_data_audited(&premised(constructor), &[("X509_new", RUNS)]).unwrap();
        ex_data_audited(&bare(Documented::Plain), &[]).unwrap();
        ex_data_audited(&bare(constructor), &[("X509_new", NONE)]).unwrap();
        // A constructor, a release and a reference drop left out of the audit.
        for shape in [
            constructor,
            Documented::Constructor,
            Documented::Release(0, Null::Nullable),
            Documented::CountedRelease(0),
            Documented::NullAcceptingRelease {
                position: 0,
                sentence: "",
            },
        ] {
            let refused = ex_data_audited(&bare(shape), &[]).unwrap_err();
            assert!(refused.contains("does not list it"), "{refused}");
        }
        // The premise dropped from a declaration the audit reads as running.
        let refused = ex_data_audited(&bare(constructor), &[("X509_new", RUNS)]).unwrap_err();
        assert!(refused.contains("does not name"), "{refused}");
        // The premise on a declaration the audit does not list, or reads as
        // touching no class.
        assert!(ex_data_audited(&premised(Documented::Plain), &[]).is_err());
        assert!(ex_data_audited(&premised(constructor), &[("X509_new", NONE)]).is_err());
        // A class that is none, no class, and a second listing.
        let unknown = ExData::Runs {
            classes: &["PKCS12"],
            how: "",
        };
        let refused =
            ex_data_audited(&premised(constructor), &[("X509_new", unknown)]).unwrap_err();
        assert!(refused.contains("PKCS12"), "{refused}");
        let empty = ExData::Runs {
            classes: &[],
            how: "",
        };
        assert!(ex_data_audited(&premised(constructor), &[("X509_new", empty)]).is_err());
        assert!(
            ex_data_audited(
                &premised(constructor),
                &[("X509_new", RUNS), ("X509_new", RUNS)]
            )
            .is_err()
        );
        // The checker refuses the crate's own X509_new with the premise
        // dropped, and SSL_set_accept_state (a plain call the audit reads).
        for function in [
            "X509_new",
            "SSL_set_accept_state",
            "SSL_new",
            "EVP_MD_CTX_free",
        ] {
            let (path, text) = module_of(function);
            let start = text.find(&format!("extern fn {function}(")).unwrap();
            let ex_data =
                format!("    requires resource_premise({EX_DATA}, \"{EX_DATA_STATEMENT}\");\n");
            let dropped = format!(
                "{}{}",
                &text[..start],
                text[start..].replacen(&ex_data, "", 1)
            );
            assert!(
                check_function(path, &dropped, function).is_err(),
                "{function}"
            );
        }
    }

    /// The premise line of a free statement, as a module spells it.
    fn free_premise(statement: FreeStatement) -> String {
        format!(
            "    requires resource_premise({UNSTATED_SLOTS}, \"{}\");\n",
            statement.text()
        )
    }

    /// Card OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: every release is in
    /// the free audit; every free the audit reads as running names its
    /// statement in its module, and dropping or swapping it refuses; each
    /// statement is named exactly where the audit reads it and names only
    /// what its object kind's free runs.
    #[test]
    fn every_free_names_what_it_runs_on_the_objects_it_frees() {
        let mut runs = 0;
        for (function, reading) in FREE_AUDIT {
            let documented = DOCUMENTED
                .iter()
                .filter(|entry| entry.function == *function)
                .collect::<Vec<_>>();
            assert_eq!(documented.len(), 1, "{function} is documented once");
            assert_eq!(
                FREE_AUDIT.iter().filter(|(f, _)| f == function).count(),
                1,
                "{function} is audited once"
            );
            assert!(releases(documented[0].shape), "{function} is a release");
            let (path, text) = module_of(function);
            let start = text.find(&format!("extern fn {function}(")).unwrap();
            let end = start + text[start..].find("};").unwrap();
            let declaration = &text[start..end];
            let named = FreeStatement::ALL
                .iter()
                .filter(|s| declaration.contains(free_premise(**s).trim()))
                .copied()
                .collect::<Vec<_>>();
            match reading {
                Frees::Runs { statement, .. } => {
                    runs += 1;
                    assert_eq!(named, vec![*statement], "{function}");
                    let premise = free_premise(*statement);
                    let dropped = format!(
                        "{}{}",
                        &text[..start],
                        text[start..].replacen(&premise, "", 1)
                    );
                    assert_ne!(dropped, *text, "{function}");
                    let refused = check_function(path, &dropped, function).unwrap_err();
                    assert!(refused.contains(UNSTATED_SLOTS), "{function}: {refused}");
                    for other in FreeStatement::ALL {
                        if other == *statement {
                            continue;
                        }
                        let swapped = format!(
                            "{}{}",
                            &text[..start],
                            text[start..].replacen(&premise, &free_premise(other), 1)
                        );
                        assert!(
                            check_function(path, &swapped, function).is_err(),
                            "{function}: {other:?}"
                        );
                    }
                    // The handshake's statement does not stand in for it.
                    let handshake = format!(
                        "{}{}",
                        &text[..start],
                        text[start..].replacen(statement.text(), UNSTATED_SLOTS_STATEMENT, 1)
                    );
                    assert!(
                        check_function(path, &handshake, function).is_err(),
                        "{function}"
                    );
                }
                Frees::NamedBy { premise, .. } => {
                    assert!(named.is_empty(), "{function}");
                    assert!(documented[0].premises.contains(premise), "{function}");
                }
                Frees::Nothing { .. } => assert!(named.is_empty(), "{function}"),
            }
        }
        // The eight frees reaching keys, certificates, stores, engines or
        // contexts: r39's three and this card's five.
        assert_eq!(runs, 8);
        for documented in DOCUMENTED {
            let audited = FREE_AUDIT.iter().any(|(f, _)| *f == documented.function);
            assert_eq!(
                audited,
                releases(documented.shape),
                "{}",
                documented.function
            );
        }
        // Every module names each statement only where the audit reads it.
        for statement in FreeStatement::ALL {
            let named = crate::MODULES
                .iter()
                .map(|(_, text)| {
                    strip_comments(text)
                        .matches(free_premise(statement).trim())
                        .count()
                })
                .sum::<usize>();
            let audited = FREE_AUDIT
                .iter()
                .filter(|(_, reading)| {
                    matches!(reading, Frees::Runs { statement: s, .. } if *s == statement)
                })
                .count();
            assert_eq!(named, audited, "{statement:?}");
        }
        // Each statement names only what its free can run: none of the new
        // ones names the SSL object's or context's objects, only the PKCS#12
        // free a CRL, only the digest context free a digest or EVP_PKEY_CTX,
        // and the engine free (structural) no finish function and no key.
        for statement in [
            UNSTATED_KEY_FREE_STATEMENT,
            UNSTATED_CERTIFICATE_FREE_STATEMENT,
            UNSTATED_PKCS12_FREE_STATEMENT,
            UNSTATED_ENGINE_FREE_STATEMENT,
            UNSTATED_DIGEST_CONTEXT_FREE_STATEMENT,
        ] {
            for absent in ["SSL", "QUIC", "async", "ex_data", "X509_LOOKUP_METHOD"] {
                assert!(!statement.contains(absent), "{absent} in {statement}");
            }
            assert_eq!(
                statement.contains("X509_CRL_METHOD"),
                statement == UNSTATED_PKCS12_FREE_STATEMENT,
                "{statement}"
            );
            assert_eq!(
                statement.contains("EVP_PKEY_CTX"),
                statement == UNSTATED_DIGEST_CONTEXT_FREE_STATEMENT,
                "{statement}"
            );
            assert!(!statement.contains(';') && !statement.contains('"'));
            assert!(!statement.contains("//"));
        }
        assert!(!UNSTATED_ENGINE_FREE_STATEMENT.contains("finish"));
        assert!(!UNSTATED_ENGINE_FREE_STATEMENT.contains("key management"));
        assert!(UNSTATED_ENGINE_FREE_STATEMENT.contains("destroy function"));
        for statement in [
            UNSTATED_KEY_FREE_STATEMENT,
            UNSTATED_CERTIFICATE_FREE_STATEMENT,
            UNSTATED_PKCS12_FREE_STATEMENT,
            UNSTATED_DIGEST_CONTEXT_FREE_STATEMENT,
        ] {
            for named in [
                "key management",
                "teardown function",
                "EVP_PKEY_ASN1_METHOD",
                "RSA_METHOD, DSA_METHOD, DH_METHOD or EC_KEY_METHOD",
                "finish and destroy functions of an ENGINE",
            ] {
                assert!(statement.contains(named), "{named} not in {statement}");
            }
        }
    }

    /// Card OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: each way the free
    /// audit can drift refuses.
    #[test]
    fn a_drifted_free_reading_is_refused() {
        const RUNS: Frees = Frees::Runs {
            statement: FreeStatement::Key,
            how: "",
        };
        const BIO: Frees = Frees::NamedBy {
            premise: Premise::BuiltinBioMethods,
            why: "",
        };
        const NOTHING: Frees = Frees::Nothing { why: "" };
        let free = |premises| DocumentedFunction {
            function: "EVP_PKEY_free",
            page: "EVP_PKEY_new",
            series: "1.1.1",
            range: "",
            shape: Documented::Release(0, Null::Nullable),
            premises,
        };
        const KEY: Premise = Premise::UnstatedFreeSlots(FreeStatement::Key);
        const CERTIFICATE: Premise = Premise::UnstatedFreeSlots(FreeStatement::Certificate);
        // As landed.
        free_audited(&free(&[KEY]), &[("EVP_PKEY_free", RUNS)]).unwrap();
        free_audited(&free(&[]), &[("EVP_PKEY_free", NOTHING)]).unwrap();
        free_audited(
            &free(&[Premise::BuiltinBioMethods]),
            &[("EVP_PKEY_free", BIO)],
        )
        .unwrap();
        // A plain call left out of the audit names no free statement.
        let plain = DocumentedFunction {
            shape: Documented::Plain,
            ..free(&[])
        };
        free_audited(&plain, &[]).unwrap();
        // Every release shape left out of the audit refuses.
        for shape in [
            Documented::Release(0, Null::Nullable),
            Documented::CountedRelease(0),
            Documented::NullAcceptingRelease {
                position: 0,
                sentence: "",
            },
        ] {
            let refused = free_audited(
                &DocumentedFunction {
                    shape,
                    ..free(&[KEY])
                },
                &[],
            )
            .unwrap_err();
            assert!(refused.contains("does not list it"), "{refused}");
            let refused =
                free_audited(&DocumentedFunction { shape, ..free(&[]) }, &[]).unwrap_err();
            assert!(refused.contains("does not list it"), "{refused}");
        }
        // A free statement on a plain call the audit does not list.
        let premised_plain = DocumentedFunction {
            shape: Documented::Plain,
            ..free(&[KEY])
        };
        assert!(free_audited(&premised_plain, &[]).is_err());
        // The premise dropped, swapped for another kind's statement, doubled,
        // or read in the handshake's statement beside the free's.
        let refused = free_audited(&free(&[]), &[("EVP_PKEY_free", RUNS)]).unwrap_err();
        assert!(refused.contains("does not name"), "{refused}");
        let refused = free_audited(&free(&[CERTIFICATE]), &[("EVP_PKEY_free", RUNS)]).unwrap_err();
        assert!(refused.contains("Certificate"), "{refused}");
        assert!(free_audited(&free(&[KEY, CERTIFICATE]), &[("EVP_PKEY_free", RUNS)]).is_err());
        assert!(
            free_audited(
                &free(&[KEY, Premise::UnstatedSlots]),
                &[("EVP_PKEY_free", RUNS)]
            )
            .is_err()
        );
        assert!(
            free_audited(&free(&[Premise::UnstatedSlots]), &[("EVP_PKEY_free", RUNS)]).is_err()
        );
        // A free statement where the audit reads another premise or nothing,
        // and the other premise dropped.
        assert!(
            free_audited(
                &free(&[KEY, Premise::BuiltinBioMethods]),
                &[("EVP_PKEY_free", BIO)]
            )
            .is_err()
        );
        assert!(free_audited(&free(&[]), &[("EVP_PKEY_free", BIO)]).is_err());
        assert!(free_audited(&free(&[KEY]), &[("EVP_PKEY_free", NOTHING)]).is_err());
        // A second listing.
        assert!(
            free_audited(
                &free(&[KEY]),
                &[("EVP_PKEY_free", RUNS), ("EVP_PKEY_free", RUNS)]
            )
            .is_err()
        );
        // The checker refuses the crate's own frees with the premise dropped,
        // and the stack free with a statement added.
        let (path, text) = module_of("OPENSSL_sk_free");
        let start = text.find("extern fn OPENSSL_sk_free(").unwrap();
        let anchor = "    requires resource_revision(flexible);\n";
        let added = format!(
            "{}{}",
            &text[..start],
            text[start..].replacen(
                anchor,
                &format!("{anchor}{}", free_premise(FreeStatement::Key)),
                1
            )
        );
        assert_ne!(added, *text);
        assert!(check_function(path, &added, "OPENSSL_sk_free").is_err());
    }

    /// Card OPENSSL-REMAINING-HANDSHAKE-SETTERS: the remaining handshake
    /// slots and the certificate store's slots, drift-refused; the setters
    /// no page documents and the page's macro aliases are never writers.
    #[test]
    fn a_drifted_remaining_setter_reading_is_refused() {
        let snapshots: Value = serde_json::from_str(crate::DOC_SNAPSHOTS).unwrap();
        let pinned: Value = serde_json::from_str(crate::PINNED_PROTOTYPES).unwrap();
        let new_keys = [
            "ssl_ctx_st.ticket_gen_cb",
            "ssl_ctx_st.ticket_dec_cb",
            "ssl_ctx_st.generate_session_id",
            "ssl_ctx_st.tmp_dh_callback",
            "ssl_ctx_st.gen_stateless_cookie_cb",
            "ssl_ctx_st.verify_stateless_cookie_cb",
            "ssl_ctx_st.app_gen_cookie_cb",
            "ssl_ctx_st.app_verify_cookie_cb",
            "ssl_st.session_secret_cb",
            "ssl_ctx_st.srp_username_callback",
            "ssl_ctx_st.srp_client_pwd_callback",
            "ssl_ctx_st.srp_verify_param_callback",
            "ssl_ctx_st.async_cb",
            "ssl_ctx_st.new_pending_conn_cb",
            "x509_store_st.verify_cb",
            "x509_store_st.verify",
            "x509_store_st.get_issuer",
            "x509_store_st.check_issued",
            "x509_store_st.check_revocation",
            "x509_store_st.get_crl",
            "x509_store_st.check_crl",
            "x509_store_st.cert_crl",
            "x509_store_st.check_policy",
            "x509_store_st.lookup_certs",
            "x509_store_st.lookup_crls",
            "x509_store_st.cleanup_fn",
        ];
        for key in new_keys {
            slot_writers_declared(key, &snapshots).unwrap_or_else(|failure| panic!("{failure}"));
        }
        // Each new invocation dropped from every handshake and I/O call.
        for call in ["SSL_write", "SSL_read", "SSL_connect", "SSL_do_handshake"] {
            let (path, text) = module_of(call);
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let end = start + text[start..].find("};").unwrap();
            let body = &text[start..end];
            for key in new_keys {
                let line = body
                    .lines()
                    .find(|line| line.contains(&format!(", {key}, ")))
                    .unwrap_or_else(|| panic!("{call} states {key}"));
                let dropped = format!(
                    "{}{}",
                    &text[..start],
                    text[start..].replacen(&format!("{line}\n"), "", 1)
                );
                assert!(
                    check_function(path, &dropped, call).is_err(),
                    "{call} {key}"
                );
            }
        }
        let (path, text) = module_of("SSL_connect");
        let start = text.find("extern fn SSL_connect(").unwrap();
        let drifted = |from: &str, to: &str| {
            let changed = format!("{}{}", &text[..start], text[start..].replacen(from, to, 1));
            assert_ne!(changed, text, "{from}");
            check_function(path, &changed, "SSL_connect").is_err()
        };
        // The client's NULL peer_ciphers read as a live value.
        assert!(drifted(
            "ssl_st.session_secret_cb, ssl, _, _, supplied_nullable, _, _)",
            "ssl_st.session_secret_cb, ssl, _, _, _, _, _)",
        ));
        // The QUIC callback's preallocated connection read as the call's.
        assert!(drifted(
            "ssl_ctx_st.new_pending_conn_cb, _, _, _)",
            "ssl_ctx_st.new_pending_conn_cb, ssl, _, _)",
        ));
        // A store callback handed the connection.
        assert!(drifted(
            "x509_store_st.verify_cb, _, _)",
            "x509_store_st.verify_cb, ssl, _)",
        ));
        // The statement before this card, which did not name the CT callback,
        // the undocumented setters' slots or the lookup methods.
        let old = "ticket key callbacks, or an ex_data callback)";
        let new = "ticket key callbacks, the certificate transparency validation callback, and \
                   the not-resumable session and session ticket extension callbacks, whose \
                   setters no manual page documents, a function of an X509_LOOKUP_METHOD of a \
                   lookup the certificate store of the SSL_CTX holds, or an ex_data callback)";
        assert!(UNSTATED_SLOTS_STATEMENT.contains(new));
        // Card OPENSSL-FREE-INDIRECT-CALLBACKS: the frees carry their own
        // statement now (`a_drifted_free_indirect_reading_is_refused`).
        for call in ["SSL_connect", "SSL_shutdown"] {
            let (path, text) = module_of(call);
            let start = text.find(&format!("extern fn {call}(")).unwrap();
            let stale = format!("{}{}", &text[..start], text[start..].replacen(new, old, 1));
            assert_ne!(stale, text, "{call}");
            assert!(check_function(path, &stale, call).is_err(), "{call}");
        }
        // SSL_shutdown's async invocation dropped.
        let (path, text) = module_of("SSL_shutdown");
        let start = text.find("extern fn SSL_shutdown(").unwrap();
        let line = "    ensures normal resource_callback_slot_invoked(ssl, ssl_ctx_st.async_cb, ssl, _);\n";
        assert!(text[start..].contains(line));
        let silent = format!("{}{}", &text[..start], text[start..].replacen(line, "", 1));
        assert!(check_function(path, &silent, "SSL_shutdown").is_err());
        // The session ticket setter's second registration and its retention.
        let (path, text) = module_of("SSL_CTX_set_session_ticket_cb");
        for line in [
            "    ensures normal resource_callback_slot_registered(ctx, ssl_ctx_st.ticket_dec_cb, dec_cb);\n",
            "    ensures normal resource_retained_by(arg, ctx);\n",
        ] {
            let start = text
                .find("extern fn SSL_CTX_set_session_ticket_cb(")
                .unwrap();
            assert!(text[start..].contains(line), "{line}");
            let dropped = format!("{}{}", &text[..start], text[start..].replacen(line, "", 1));
            assert!(
                check_function(path, &dropped, "SSL_CTX_set_session_ticket_cb").is_err(),
                "{line}"
            );
        }
        // The page's macro aliases have no pinned prototype, no documented
        // entry and no declaration: a call through one is its writer's call.
        for alias in [
            "X509_STORE_set_verify_cb_func",
            "X509_STORE_set_verify_func",
            "X509_STORE_set_lookup_crls_cb",
        ] {
            assert!(pinned["prototypes"][alias].is_null(), "{alias}");
            assert!(
                DOCUMENTED.iter().all(|entry| entry.function != alias),
                "{alias}"
            );
            assert!(
                SLOTS.iter().any(|slot| slot.not_writers.contains(&alias)),
                "{alias}"
            );
        }
        // The setters no manual page documents are not declared, and their
        // slots are not ones this checker documents.
        for undocumented in [
            "SSL_CTX_set_not_resumable_session_callback",
            "SSL_set_not_resumable_session_callback",
            "SSL_set_session_ticket_ext_cb",
        ] {
            assert!(
                DOCUMENTED
                    .iter()
                    .all(|entry| entry.function != undocumented)
            );
            assert!(
                snapshots["pages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|page| page["page"] != undocumented || page["fetched"] == false),
                "{undocumented}"
            );
        }
        for key in [
            "ssl_ctx_st.not_resumable_session_cb",
            "ssl_st.session_ticket_ext_cb",
        ] {
            assert!(slot_writers_declared(key, &snapshots).is_err(), "{key}");
        }
    }
}
