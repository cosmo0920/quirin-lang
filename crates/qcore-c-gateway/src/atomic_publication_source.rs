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

//! Closed, bounded source route for C11 release/acquire publication.
//!
//! Acceptance is conditional: if the consumer takes the branch guarded by the
//! observed ready value, the payload write happens before its payload read.
//! Termination and eventual observation are deliberately outside this route.
//!
//! Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i) reads the flag's store
//! and load through the atomic source route (`crate::atomic_source_route`):
//! any routed plain store or load of the flag (`__c11_atomic_*`, the
//! `<stdatomic.h>` generics, the `_Atomic` operator forms, and GNU `__atomic_*`
//! builtins on a non-`_Atomic` `int` flag every access of which is one), with
//! an integer-literal or canonical `memory_order` order; a consume load and
//! every read-modify-write, compare-exchange or fence is refused by the
//! route's name. The consumer may test `seen != READY` (return early) or
//! `seen == READY` (use the payload inside). The certificate records the
//! initialise -> publish -> observe -> use stages it derived
//! ([`AtomicPublicationStages`]); reclamation is not derived (card CG29).

use serde_json::Value;
use std::collections::BTreeMap;

use crate::legacy_c::{C11TranslationUnit, LegacyFunctionScope, validate_unrestricted_c11};
use crate::typed_c_ast::{children, kind, translation_unit_ast};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationSourceProfile<'a> {
    pub publisher: &'a str,
    pub consumer: &'a str,
    pub flag: &'a str,
    pub payload: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationExecutionBounds {
    /// Declared premise; a definition cannot prove its dynamic call count.
    pub publisher_invocations: usize,
    pub successful_consumer_invocations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicPublicationOrder {
    Relaxed,
    Acquire,
    Release,
    SequentiallyConsistent,
}

/// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the publication stages
/// the extractor derived from source, each named by the form it read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationStages {
    /// The flag's static initializer.
    pub initialise: String,
    /// The payload write, then the flag's store.
    pub publish: String,
    /// The flag's load and the ready-value test.
    pub observe: String,
    /// The payload read on the observing branch.
    pub use_: String,
    /// Where the route stops.
    pub reclaim: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationSourceCertificate {
    pub artifact_raw_sha256: String,
    pub artifact_preprocessed_sha256: String,
    /// Caller-declared label; the digests bind the actual source bytes.
    pub source_revision: String,
    /// Digest of every declared input/configuration field that can select the
    /// preprocessed source or typed AST. The two artifact digests alone do not
    /// bind frontend flags or the declared input identity.
    pub analysis_configuration_sha256: String,
    pub property: &'static str,
    pub semantic_profile: &'static str,
    pub scope: &'static str,
    pub evidence_version: &'static str,
    pub publisher: String,
    pub consumer: String,
    pub flag: String,
    pub payload: String,
    pub initial_value: i64,
    pub published_value: i64,
    /// The theorem applies only on the consumer branch that observed this value.
    pub successful_observation: i64,
    pub store_order: AtomicPublicationOrder,
    pub load_order: AtomicPublicationOrder,
    pub bounds: AtomicPublicationExecutionBounds,
    pub assumptions: Vec<&'static str>,
    /// Card CG19 (rule 11i): the derived stages.
    pub stages: AtomicPublicationStages,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicPublicationSourceRefusal {
    InvalidSource(String),
    InvalidProfile,
    MissingOrAmbiguousDeclaration(&'static str),
    RedeclaredProfileEntity(&'static str),
    PlainFlagHasNoSynchronization,
    VolatileFlagHasNoSynchronization,
    UnsupportedAtomicFlag,
    FlagMustBeStatic,
    PayloadMustBeStaticInt,
    UnsupportedInitializer,
    UnsupportedPublisher,
    UnsupportedConsumer,
    UnsupportedHiddenAccess,
    ExtraFlagAccess,
    ExtraPayloadAccess,
    ExtraEntryCall,
    UnsupportedMemoryOrder,
    UnsupportedExecutionBounds,
    /// Card CG19 (rule 11i): the atomic source route's named refusal of the
    /// flag's store or load (a consume load, a read-modify-write, a
    /// compare-exchange or a fence).
    NamedAtomicRefusal(String),
}

pub fn extract_atomic_publication_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &AtomicPublicationSourceProfile<'_>,
    bounds: AtomicPublicationExecutionBounds,
) -> Result<AtomicPublicationSourceCertificate, AtomicPublicationSourceRefusal> {
    if source_revision.is_empty()
        || [
            profile.publisher,
            profile.consumer,
            profile.flag,
            profile.payload,
        ]
        .iter()
        .any(|name| name.is_empty())
        || profile.publisher == profile.consumer
        || profile.flag == profile.payload
    {
        return Err(AtomicPublicationSourceRefusal::InvalidProfile);
    }
    if bounds.publisher_invocations != 1 || bounds.successful_consumer_invocations != 1 {
        return Err(AtomicPublicationSourceRefusal::UnsupportedExecutionBounds);
    }
    let source = validate_unrestricted_c11(unit)
        .map_err(|error| AtomicPublicationSourceRefusal::InvalidSource(format!("{error:?}")))?;
    let ast = translation_unit_ast(&source, unit)
        .map_err(AtomicPublicationSourceRefusal::InvalidSource)?;
    if contains_hidden_access(&ast) {
        return Err(AtomicPublicationSourceRefusal::UnsupportedHiddenAccess);
    }
    let top = children(&ast);
    let flag = one_top_level(top, "VarDecl", profile.flag, "flag")?;
    let payload = one_top_level(top, "VarDecl", profile.payload, "payload")?;
    let publisher = one_definition(top, profile.publisher, "publisher")?;
    let consumer = one_definition(top, profile.consumer, "consumer")?;

    let flag_type =
        type_spelling(flag).ok_or(AtomicPublicationSourceRefusal::UnsupportedAtomicFlag)?;
    // Qualifier refusals are attributed only after checking the full bounded
    // publication shape and access inventory, never from the declaration alone.
    // Card CG19 (rule 11i): a non-`_Atomic` `int` flag whose publisher store
    // and consumer load are both GNU `__atomic_*` builtins is an atomic
    // object under those builtins' semantics; the inventory below proves they
    // are its only accesses.
    let gnu_atomic = flag_type == "int" && gnu_atomic_flag(publisher, consumer);
    let non_atomic_refusal = match flag_type {
        "int" if gnu_atomic => None,
        "int" => Some(AtomicPublicationSourceRefusal::PlainFlagHasNoSynchronization),
        "volatile int" => Some(AtomicPublicationSourceRefusal::VolatileFlagHasNoSynchronization),
        "_Atomic(int)" => None,
        _ => return Err(AtomicPublicationSourceRefusal::UnsupportedAtomicFlag),
    };
    if flag["storageClass"].as_str() != Some("static") {
        return Err(AtomicPublicationSourceRefusal::FlagMustBeStatic);
    }
    if type_spelling(payload) != Some("int")
        || payload["storageClass"].as_str() != Some("static")
        || !children(payload).is_empty()
    {
        return Err(AtomicPublicationSourceRefusal::PayloadMustBeStaticInt);
    }
    let [initializer] = children(flag) else {
        // Attributes are semantic input too. In particular, an AsmLabelAttr
        // can make two distinct declarations denote the same linker object,
        // invalidating the separate-location premise. The flag declaration is
        // therefore exactly one initializer expression and nothing else.
        return Err(AtomicPublicationSourceRefusal::UnsupportedInitializer);
    };
    let initial_value =
        integer(initializer).ok_or(AtomicPublicationSourceRefusal::UnsupportedInitializer)?;
    let flag_id = node_id(flag).ok_or(AtomicPublicationSourceRefusal::UnsupportedAtomicFlag)?;
    let payload_id =
        node_id(payload).ok_or(AtomicPublicationSourceRefusal::PayloadMustBeStaticInt)?;

    let atomic = non_atomic_refusal.is_none();
    // Card CG19 (rule 11i): the canonical enumeration is needed only for an
    // enumerator order; a literal order (`__ATOMIC_RELEASE`) is Clang's own.
    let memory_orders = if atomic {
        canonical_memory_orders(&ast).unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let (published_value, store_order, publish_form) =
        match_publisher(publisher, flag_id, payload_id, &memory_orders, atomic)?;
    let (load_order, observe_form) = match_consumer(
        consumer,
        flag_id,
        payload_id,
        published_value,
        &memory_orders,
        atomic,
    )?;
    let publisher_id =
        node_id(publisher).ok_or(AtomicPublicationSourceRefusal::UnsupportedPublisher)?;
    let consumer_id =
        node_id(consumer).ok_or(AtomicPublicationSourceRefusal::UnsupportedConsumer)?;
    for (expected_kind, name, selected_id, label) in [
        ("VarDecl", profile.flag, flag_id, "flag"),
        ("VarDecl", profile.payload, payload_id, "payload"),
        ("FunctionDecl", profile.publisher, publisher_id, "publisher"),
        ("FunctionDecl", profile.consumer, consumer_id, "consumer"),
    ] {
        if has_other_declaration_named(&ast, expected_kind, name, selected_id) {
            return Err(AtomicPublicationSourceRefusal::RedeclaredProfileEntity(
                label,
            ));
        }
    }

    let mut flag_references = 0;
    let mut payload_references = 0;
    count_references(&ast, flag_id, &mut flag_references);
    count_references(&ast, payload_id, &mut payload_references);
    if flag_references != 2 {
        return Err(AtomicPublicationSourceRefusal::ExtraFlagAccess);
    }
    if payload_references != 2 {
        return Err(AtomicPublicationSourceRefusal::ExtraPayloadAccess);
    }
    let mut publisher_references = 0;
    let mut consumer_references = 0;
    count_references(&ast, publisher_id, &mut publisher_references);
    count_references(&ast, consumer_id, &mut consumer_references);
    if publisher_references != 0 || consumer_references != 0 {
        return Err(AtomicPublicationSourceRefusal::ExtraEntryCall);
    }

    if let Some(refusal) = non_atomic_refusal {
        return Err(refusal);
    }

    Ok(AtomicPublicationSourceCertificate {
        artifact_raw_sha256: qcore_schemas::sha256_hex(unit.source.as_bytes()),
        artifact_preprocessed_sha256: qcore_schemas::sha256_hex(source.as_bytes()),
        source_revision: source_revision.into(),
        analysis_configuration_sha256: analysis_configuration_sha256(unit),
        property: "closed typed source facts for one bounded atomic publication candidate",
        semantic_profile: "c11-bounded-atomic-publication-source-v1",
        scope: "the selected full preprocessed translation unit, exactly one declared publisher invocation, and exactly one successful selected consumer invocation",
        evidence_version: "ct41-sync-model-v1",
        publisher: profile.publisher.into(),
        consumer: profile.consumer.into(),
        flag: profile.flag.into(),
        payload: profile.payload.into(),
        initial_value,
        published_value,
        successful_observation: published_value,
        store_order,
        load_order,
        bounds,
        assumptions: {
            let mut assumptions = vec![
                "the publisher entry is invoked exactly once in the bounded execution",
                "exactly one consumer takes the successful observation branch",
                "the full Clang AST faithfully represents the preprocessed C translation unit",
            ];
            if gnu_atomic {
                assumptions.push(
                    "GNU __atomic builtins on a non-_Atomic int object, its only accesses, are atomic operations (Clang/GCC builtin semantics, not C11 text)",
                );
            }
            assumptions
        },
        stages: AtomicPublicationStages {
            initialise: format!("static `{}` = {initial_value}", profile.flag),
            publish: format!(
                "`{}` written, then {publish_form} of `{}` = {published_value} ({store_order:?})",
                profile.payload, profile.flag
            ),
            observe: format!("{observe_form} ({load_order:?})"),
            use_: format!("`{}` read on the observing branch", profile.payload),
            reclaim: "not derived: reclamation is outside this route (CG29)",
        },
    })
}

/// Card CG19 (rule 11i): whether the publisher's flag store and the
/// consumer's flag load are both GNU `__atomic_*` builtins.
fn gnu_atomic_flag(publisher: &Value, consumer: &Value) -> bool {
    let gnu = |node: &Value| {
        kind(node) == "AtomicExpr"
            && node["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("__atomic_"))
    };
    let store = function_body(publisher)
        .and_then(|body| children(body).get(1))
        .is_some_and(gnu);
    let load = function_body(consumer)
        .and_then(|body| children(body).first())
        .and_then(|statement| children(statement).first())
        .and_then(|declaration| children(declaration).first())
        .is_some_and(|load| gnu(unwrap_implicit(load)));
    store && load
}

/// Card CG19 (rule 11i): the atomic source route's answer for a flag access,
/// with the canonical enumeration (or `memory_order_consume` by name) as the
/// enumerator resolver.
fn route_flag_access<'a>(
    node: &'a Value,
    canonical: &BTreeMap<String, String>,
    shape: AtomicPublicationSourceRefusal,
) -> Result<crate::atomic_source_route::RoutedAtomic<'a>, AtomicPublicationSourceRefusal> {
    use crate::atomic_source_route::{
        AtomicRouteRefusal as R, atomic_node_name, classify_builtin, classify_qualified,
        consume_only,
    };
    let resolver = |reference: &Value| {
        let name = reference["referencedDecl"]["name"].as_str()?;
        let id = reference["referencedDecl"]["id"].as_str()?;
        if canonical.get(name).map(String::as_str) == Some(id) {
            match name {
                "memory_order_relaxed" => Some(0),
                "memory_order_acquire" => Some(2),
                "memory_order_release" => Some(3),
                "memory_order_seq_cst" => Some(5),
                _ => None,
            }
        } else {
            consume_only(reference)
        }
    };
    let answer = if atomic_node_name(node).is_some() {
        classify_builtin(node, &resolver)
    } else {
        classify_qualified(node).ok_or_else(|| shape.clone())?
    };
    let routed = answer.map_err(|refusal| match refusal {
        R::UnevaluatedOrder(_) | R::InvalidOrder(_) | R::UnevaluatedStrength(_) => {
            AtomicPublicationSourceRefusal::UnsupportedMemoryOrder
        }
        R::ConsumeLoad(_) | R::ConsumeOrder(_) | R::LegacySync(_) => {
            AtomicPublicationSourceRefusal::NamedAtomicRefusal(refusal.reason())
        }
        R::ByPointer(_) | R::NonScalarValue(_) | R::UnsupportedShape(_) | R::Unlisted(_) => {
            shape.clone()
        }
    })?;
    // Card CG28 (rule 11t): a read-modify-write, compare-exchange or fence is
    // now routed to the weak-memory model, but it is not the one-flag
    // publication shape this extractor decides; it stays refused by name.
    use crate::atomic_source_route::Operation as O;
    match routed.operation {
        O::Load | O::Store | O::Init => Ok(routed),
        O::ReadModifyWrite | O::CompareExchange | O::ThreadFence | O::SignalFence => {
            Err(AtomicPublicationSourceRefusal::NamedAtomicRefusal(format!(
                "atomic `{}` is routed to the weak-memory model but is not a publication shape this route decides (CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD)",
                routed.form
            )))
        }
    }
}

fn publication_order(order: Option<qcore_core::MemoryOrder>) -> Option<AtomicPublicationOrder> {
    match order? {
        qcore_core::MemoryOrder::Relaxed => Some(AtomicPublicationOrder::Relaxed),
        qcore_core::MemoryOrder::Acquire => Some(AtomicPublicationOrder::Acquire),
        qcore_core::MemoryOrder::Release => Some(AtomicPublicationOrder::Release),
        qcore_core::MemoryOrder::SequentiallyConsistent => {
            Some(AtomicPublicationOrder::SequentiallyConsistent)
        }
        qcore_core::MemoryOrder::AcquireRelease => None,
    }
}

/// Whether a routed access's object is the flag: `&flag` for a builtin, the
/// `flag` lvalue itself for a qualified form.
fn routed_object_is_flag(node: &Value, object: &Value, flag: &str) -> bool {
    if crate::atomic_source_route::atomic_node_name(node).is_some() {
        address_of(unwrap_implicit(object), flag)
    } else {
        reference_is(object, flag)
    }
}

fn contains_hidden_access(node: &Value) -> bool {
    // These nodes can name an object only in a string or assembler symbol, so
    // DeclRefExpr identity counting cannot prove the closed-world access set.
    // Reject them across the complete AST, including unrelated functions and
    // declarations: either can name the selected static objects.
    matches!(
        kind(node),
        "GCCAsmStmt" | "MSAsmStmt" | "FileScopeAsmDecl" | "AsmLabelAttr" | "AliasAttr"
    ) || children(node).iter().any(contains_hidden_access)
}

fn has_other_declaration_named(
    node: &Value,
    expected_kind: &str,
    name: &str,
    selected_id: &str,
) -> bool {
    (kind(node) == expected_kind
        && node["name"].as_str() == Some(name)
        && node_id(node).is_some_and(|id| id != selected_id))
        || children(node)
            .iter()
            .any(|child| has_other_declaration_named(child, expected_kind, name, selected_id))
}

fn analysis_configuration_sha256(unit: &C11TranslationUnit<'_>) -> String {
    let function_scope = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => "preprocessed-translation-unit",
        LegacyFunctionScope::MainSourceDefinitions => "main-source-definitions",
    };
    let value = serde_json::json!({
        "include_paths": unit.include_paths,
        "definitions": unit.definitions,
        "preprocessor_options": unit.preprocessor_options,
        "dialect": unit.dialect.clang_argument(),
        "function_scope": function_scope,
        "input": unit.input,
    });
    qcore_schemas::sha256_hex(
        &serde_json::to_vec(&value).expect("translation-unit configuration is serializable"),
    )
}

fn one_top_level<'a>(
    top: &'a [Value],
    expected_kind: &str,
    name: &str,
    label: &'static str,
) -> Result<&'a Value, AtomicPublicationSourceRefusal> {
    let found = top
        .iter()
        .filter(|node| kind(node) == expected_kind && node["name"].as_str() == Some(name))
        .collect::<Vec<_>>();
    match found.as_slice() {
        [node] => Ok(node),
        _ => Err(AtomicPublicationSourceRefusal::MissingOrAmbiguousDeclaration(label)),
    }
}

fn one_definition<'a>(
    top: &'a [Value],
    name: &str,
    label: &'static str,
) -> Result<&'a Value, AtomicPublicationSourceRefusal> {
    let found = top
        .iter()
        .filter(|node| {
            kind(node) == "FunctionDecl"
                && node["name"].as_str() == Some(name)
                && children(node)
                    .iter()
                    .any(|child| kind(child) == "CompoundStmt")
        })
        .collect::<Vec<_>>();
    match found.as_slice() {
        [node] => Ok(node),
        _ => Err(AtomicPublicationSourceRefusal::MissingOrAmbiguousDeclaration(label)),
    }
}

fn function_body(function: &Value) -> Option<&Value> {
    let nodes = children(function);
    if nodes
        .iter()
        .any(|node| !matches!(kind(node), "ParmVarDecl" | "CompoundStmt"))
    {
        return None;
    }
    let bodies = nodes
        .iter()
        .filter(|node| kind(node) == "CompoundStmt")
        .collect::<Vec<_>>();
    match bodies.as_slice() {
        [body] => Some(*body),
        _ => None,
    }
}

fn type_spelling(node: &Value) -> Option<&str> {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
}

fn node_id(node: &Value) -> Option<&str> {
    node["id"].as_str()
}

fn referenced_id(node: &Value) -> Option<&str> {
    (kind(node) == "DeclRefExpr")
        .then(|| node["referencedDecl"]["id"].as_str())
        .flatten()
}

fn unwrap_implicit(mut node: &Value) -> &Value {
    while matches!(kind(node), "ImplicitCastExpr" | "ParenExpr") {
        let [child] = children(node) else { break };
        node = child;
    }
    node
}

fn integer(node: &Value) -> Option<i64> {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" => {
                let [child] = children(node) else { return None };
                node = child;
            }
            // Clang represents `_Atomic int flag = 0` by this value-preserving
            // wrapper. Every integer conversion, including a narrowing
            // `long`/`unsigned` literal, stays unsupported: looking through an
            // `IntegralCast` would compare the token value rather than the C
            // value stored in the int object.
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("NonAtomicToAtomic") => {
                let [child] = children(node) else { return None };
                if type_spelling(node) != Some("_Atomic(int)") {
                    return None;
                }
                node = child;
            }
            _ => break,
        }
    }
    (kind(node) == "IntegerLiteral" && type_spelling(node) == Some("int"))
        .then(|| node["value"].as_str()?.parse().ok())
        .flatten()
}

fn reference_is(node: &Value, id: &str) -> bool {
    referenced_id(unwrap_implicit(node)) == Some(id)
}

fn address_of(node: &Value, id: &str) -> bool {
    kind(node) == "UnaryOperator"
        && node["opcode"].as_str() == Some("&")
        && matches!(children(node), [child] if reference_is(child, id))
}

fn canonical_memory_orders(
    ast: &Value,
) -> Result<BTreeMap<String, String>, AtomicPublicationSourceRefusal> {
    let enums = children(ast)
        .iter()
        .filter(|node| kind(node) == "EnumDecl" && node["name"].as_str() == Some("memory_order"))
        .collect::<Vec<_>>();
    let [memory_order] = enums.as_slice() else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder);
    };
    // AtomicExpr executes Clang's numeric builtin order, not the spelling of
    // an enum member. A user-defined `memory_order_release = 0` is relaxed.
    // Require evaluated int constants for the four orders this route models.
    let mut orders = BTreeMap::new();
    for (name, expected_value) in [
        ("memory_order_relaxed", "0"),
        ("memory_order_acquire", "2"),
        ("memory_order_release", "3"),
        ("memory_order_seq_cst", "5"),
    ] {
        let member = children(memory_order)
            .iter()
            .find(|node| kind(node) == "EnumConstantDecl" && node["name"].as_str() == Some(name))
            .ok_or(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder)?;
        let [value] = children(member) else {
            return Err(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder);
        };
        if kind(value) != "ConstantExpr"
            || type_spelling(value) != Some("int")
            || value["value"].as_str() != Some(expected_value)
        {
            return Err(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder);
        }
        let id = node_id(member).ok_or(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder)?;
        orders.insert(name.to_owned(), id.to_owned());
    }
    Ok(orders)
}

fn match_publisher(
    function: &Value,
    flag: &str,
    payload: &str,
    orders: &BTreeMap<String, String>,
    atomic: bool,
) -> Result<(i64, AtomicPublicationOrder, String), AtomicPublicationSourceRefusal> {
    let body =
        function_body(function).ok_or(AtomicPublicationSourceRefusal::UnsupportedPublisher)?;
    let [payload_write, flag_store] = children(body) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedPublisher);
    };
    let [destination, payload_value] = children(payload_write) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedPublisher);
    };
    if kind(payload_write) != "BinaryOperator"
        || payload_write["opcode"].as_str() != Some("=")
        || !reference_is(destination, payload)
        || integer(payload_value).is_none()
    {
        return Err(AtomicPublicationSourceRefusal::UnsupportedPublisher);
    }
    if !atomic {
        let [destination, value] = children(flag_store) else {
            return Err(AtomicPublicationSourceRefusal::UnsupportedPublisher);
        };
        if kind(flag_store) != "BinaryOperator"
            || flag_store["opcode"].as_str() != Some("=")
            || !reference_is(destination, flag)
        {
            return Err(AtomicPublicationSourceRefusal::UnsupportedPublisher);
        }
        let value = integer(value).ok_or(AtomicPublicationSourceRefusal::UnsupportedPublisher)?;
        // This placeholder never leaves extraction: the non-atomic arm refuses
        // after the common inventory checks, before constructing a certificate.
        return Ok((value, AtomicPublicationOrder::Relaxed, String::new()));
    }
    // Card CG19 (rule 11i): the flag store through the atomic source route.
    let routed = route_flag_access(
        flag_store,
        orders,
        AtomicPublicationSourceRefusal::UnsupportedPublisher,
    )?;
    if routed.operation != crate::atomic_source_route::Operation::Store
        || !routed_object_is_flag(flag_store, routed.object, flag)
    {
        return Err(AtomicPublicationSourceRefusal::UnsupportedPublisher);
    }
    let value = routed
        .value
        .ok_or(AtomicPublicationSourceRefusal::UnsupportedPublisher)?;
    let published = integer(value).ok_or(AtomicPublicationSourceRefusal::UnsupportedPublisher)?;
    let order = publication_order(routed.order)
        .ok_or(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder)?;
    Ok((published, order, routed.form))
}

fn match_consumer(
    function: &Value,
    flag: &str,
    payload: &str,
    published: i64,
    orders: &BTreeMap<String, String>,
    atomic: bool,
) -> Result<(AtomicPublicationOrder, String), AtomicPublicationSourceRefusal> {
    let body =
        function_body(function).ok_or(AtomicPublicationSourceRefusal::UnsupportedConsumer)?;
    let [load_statement, gate, payload_return] = children(body) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    };
    let [seen_decl] = children(load_statement) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    };
    let seen = node_id(seen_decl).ok_or(AtomicPublicationSourceRefusal::UnsupportedConsumer)?;
    let [load] = children(seen_decl) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    };
    if kind(load_statement) != "DeclStmt"
        || kind(seen_decl) != "VarDecl"
        || type_spelling(seen_decl) != Some("int")
    {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    }
    let (load_order, form) = if atomic {
        // Card CG19 (rule 11i): the flag load through the atomic source route.
        let load = unwrap_integral_free(load);
        let routed = route_flag_access(
            load,
            orders,
            AtomicPublicationSourceRefusal::UnsupportedConsumer,
        )?;
        if routed.operation != crate::atomic_source_route::Operation::Load
            || !routed_object_is_flag(load, routed.object, flag)
        {
            return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
        }
        let order = publication_order(routed.order)
            .ok_or(AtomicPublicationSourceRefusal::UnsupportedMemoryOrder)?;
        (order, routed.form)
    } else {
        if !reference_is(load, flag) {
            return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
        }
        (AtomicPublicationOrder::Relaxed, String::new())
    };
    let [condition, then_return] = children(gate) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    };
    let [observed, ready] = children(condition) else {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    };
    let returns_integer = |statement: &Value| {
        kind(statement) == "ReturnStmt"
            && matches!(children(statement), [value] if integer(value).is_some())
    };
    let returns_payload = |statement: &Value| {
        kind(statement) == "ReturnStmt"
            && matches!(children(statement), [value] if reference_is(value, payload))
    };
    // `if (seen != READY) return K; return payload;`, or (card CG19, rule
    // 11i) `if (seen == READY) return payload; return K;`: the payload is
    // read only on the branch that observed READY.
    let gate_shape = match condition["opcode"].as_str() {
        Some("!=") => returns_integer(then_return) && returns_payload(payload_return),
        Some("==") => returns_payload(then_return) && returns_integer(payload_return),
        _ => false,
    };
    if kind(gate) != "IfStmt"
        || kind(condition) != "BinaryOperator"
        || !reference_is(observed, seen)
        || integer(ready) != Some(published)
        || !gate_shape
    {
        return Err(AtomicPublicationSourceRefusal::UnsupportedConsumer);
    }
    let test = if condition["opcode"].as_str() == Some("==") {
        "=="
    } else {
        "!="
    };
    Ok((
        load_order,
        format!("{form} of the flag, tested `seen {test} {published}`"),
    ))
}

/// The load under the declaration's own value conversion: only Clang's
/// identity-preserving wrappers, never an integer conversion.
fn unwrap_integral_free(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        let [child] = children(node) else { break };
        node = child;
    }
    node
}

fn count_references(node: &Value, id: &str, count: &mut usize) {
    if referenced_id(node) == Some(id) {
        *count += 1;
    }
    for child in children(node) {
        count_references(child, id, count);
    }
}
