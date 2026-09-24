//! Total-or-reject bridge from native contract modules to legacy-C custody.
//!
//! # Interpreter selection
//!
//! `the canonical contract corpus` is shared by three consumers with disjoint vocabularies.
//! The **`requires resource_contract_v1` marker is the selector**: a declaration
//! that carries it must map completely into [`ResourceFunctionContract`], and a
//! declaration that does not carry it is rejected here rather than skipped, so
//! asking this loader for a `ResourceContracts` environment over an
//! observation/lifecycle module fails closed instead of silently producing a
//! weaker table. `compiler/tests/contract_loader.rs::
//! every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped` pins
//! that split over the whole corpus.
//!
//! | Clause family | Owner |
//! | --- | --- |
//! | `requires resource_contract_v1` + the `resource_*` predicates below | this loader |
//! | integer/Boolean ghost predicates (`result == 0`, `field_NAME`, `busy`, `lock`, `allocations`, `releases`, `old(...)` arithmetic) | `contract_observation` and the ChunkIO/source interpreters |
//! | named atomic lifecycle propositions (`chunk_is_live`, `arena_is_discharged`, `descriptor_custody_is_discharged_once`, ...) | the mechanism/lifecycle interpreters |
//! | `own<T>` / `borrow<'a, _, T>` parameters, `use` annotations, `effects { .. }` rows, `quant` clauses, `ensures any`, `ensures error(E)`, `result<T, E>` returns, `resource type` / `error` / `axis` declarations | the QCore language front end and checker |
//!
//! These families are not approximated into custody modes. A resource
//! projection of a lifecycle module is written as its own v1 module (as
//! `fluent_bit/resource_memory.qcore` projects `fluent_bit/memory.qcore`);
//! it is never inferred by dropping predicates this loader cannot check.
//!
//! # Version 1 vocabulary
//!
//! Every parameter and the result must occur exactly once, and all clauses
//! other than the marker must be `ensures normal` calls:
//!
//! - `resource_borrowed(p)` -> Borrowed (no custody requirement).
//! - `resource_{owned,consumed,released,out}(p, nullable|nonnull)` -> that mode.
//! - `resource_{stored,removed}(p, owner, nullable|nonnull)` -> indexed custody.
//! - `resource_return_none()` -> None; `resource_return_owned(nullability)` -> Owned.
//! - `resource_return_reallocated(p, nullability)` -> Reallocated, requiring
//!   an Owned parameter; null failure preserves it, success replaces it once.
//! - `resource_return_alias(p, nullability)` and
//!   `resource_return_interior(p, nullability)` -> Derived (card
//!   W43-LIBC-RESULT-ALIAS): the result is `p`'s own value, or a pointer into
//!   the object `p` points to, and owns nothing. See below.
//! - `resource_variadic_data_tail()` -> the declaration additionally covers
//!   calls that pass *more* actuals than it names. See below.
//!
//! and one further **precondition** beside the marker:
//!
//! - `requires resource_literal_argument(p, LITERAL);` -> the declaration
//!   applies only at call sites whose actual at `p` is exactly `LITERAL`. See
//!   below.
//!
//! # The variadic data tail (card O04-TRACE)
//!
//! A C variadic function (`void flb_trace(const char *, ...)`) has no
//! per-argument declaration for its tail, so before this clause the vocabulary
//! could only describe one arity at a time — card O04-ARMS declared `flb_error`
//! at its single call site's arity and recorded the gap as
//! **`O04-ARMS-VARIADIC`**.
//!
//! `ensures normal resource_variadic_data_tail();` is that missing form. It
//! declares, for **every** argument beyond the named parameter vector:
//!
//! * the callee takes **no custody** of it, and
//! * the callee **retains no pointer** to it past the call.
//!
//! **What it does not say.** It is a trusted assumption about the callee's
//! contract, exactly like every other clause here; it verifies nothing about
//! the callee's body, and it is not a claim that the callee is a `printf`-style
//! formatter, that the tail is type-correct against a format string, or that
//! the tail arguments are read at all. It also does not admit anything on its
//! own: a consumer that honours the tail must still establish, per call site,
//! that each tail actual really is data. The C gateway does exactly that
//! (`qcore_c_gateway::variadic_nested`), and an actual that carries a tracked
//! resource identity is refused there even under this declaration — the
//! declaration covers what it covers.
//!
//! The tail flag is **not** carried on [`ResourceFunctionContract`]. It is
//! rederived from the retained trusted source texts by
//! [`variadic_data_tails`], so a contract environment that has no declared
//! sources — an inferred W4 summary, a provider proposal, a directly
//! constructed table — has no tails at all, which is the fail-closed direction.
//!
//! # Derived results (card W43-LIBC-RESULT-ALIAS)
//!
//! `memcpy` returns its first argument and `strchr` returns a pointer into its
//! first argument or NULL. Neither result is fresh custody, and
//! `resource_return_none()` would drop the relation between the result and the
//! base, so neither was declarable. Two result predicates now state it:
//!
//! * `resource_return_alias(p, n)` — the result is the value of `p` itself
//!   ([`DerivedExtent::Exact`]); with `nullable` the call may return NULL
//!   instead (`gmtime_r`), with `nonnull` it never substitutes NULL (`memcpy`).
//! * `resource_return_interior(p, n)` — the result points into the object `p`
//!   points to ([`DerivedExtent::Interior`]); with `nullable` it may be NULL.
//!
//! Both load to [`ResourceReturnMode::Derived`]. The result is **non-owning**:
//! it acquires, discharges and transfers nothing. `p` must be a
//! `resource_borrowed` parameter of type `ptr<mut, u8>` or `ptr<const, u8>`,
//! and the result type must be one of those two; a base declared
//! `ptr<const, u8>` under a `ptr<mut, u8>` result records `const_dropped`.
//! Every other placement rejects the module. What a consumer must do with the
//! relation — a release of the result is not a release of the base, a release
//! of the base invalidates the result — is contract spec §C04.3; this loader
//! only states it.
//!
//! **Version.** This is an addition to resource vocabulary version 1, as the
//! variadic tail and the literal premise were: the marker is unchanged, every
//! module that loaded before loads to the same functions, and a loader that
//! predates the predicates rejects a module using them (unknown predicate)
//! rather than misreading it. The exact module texts travel with every
//! verdict in `source_assumptions`, so the evidence identity already names
//! which declarations — and therefore which vocabulary — a verdict rests on.
//!
//! # The literal-argument premise (card HASH-CLAUSE)
//!
//! A v1 declaration names one custody mode per parameter and says nothing about
//! argument *values* (§C04.2 has no value-dependent mode). Some real C entry
//! points switch behaviour on a scalar argument:
//! `flb_hash_table_add(ht, key, key_len, val, val_size)` stores the caller's
//! pointer uncopied and never frees it at `val_size == 0`, and allocates its own
//! copy at a positive size. A declaration read off one arm must not be believed
//! on the other.
//!
//! `requires resource_literal_argument(p, LITERAL);` is that restriction, as a
//! first-class clause. It declares that the contract applies **only** at call
//! sites whose actual at parameter `p` is exactly the token sequence a C lexer
//! produces for `LITERAL`; everywhere else a consumer MUST treat the callee as
//! undeclared. The clause can therefore only ever **withdraw** call sites the
//! declaration would otherwise have covered — it can never admit one, and it
//! never alters the modes.
//!
//! It is a `requires` clause because it is a precondition on *applicability*,
//! and it names its parameter by name because every other predicate in this
//! vocabulary does. `LITERAL` is a decimal integer with an optional leading
//! minus and no suffix, alternate base, leading zero or second spelling of zero;
//! a spelling this loader cannot compare exactly against a C token vector is
//! rejected rather than approximated. The declared spelling is therefore
//! **canonical**: one spelling per value. The *call site* may spell the same
//! value several ways, and which of those a consumer admits is decided by the
//! consumer against this canonical form — see
//! `qcore_c_gateway::legacy_literal_premise` and §C04.5.
//!
//! ## Any parameter, deliberately (card HASH-REFINE)
//!
//! `p` may name a parameter of **any** declared type in this vocabulary. The
//! decision is closed rather than open-ended, because the vocabulary is: a
//! parameter is either a [`SCALAR_ABI_TYPES`] projection or a
//! [`POINTER_ABI_TYPES`] one, so restricting the clause could only mean
//! "scalars only". Three measured reasons against that restriction:
//!
//! 1. A literal premise on a pointer parameter is **meaningful**: an integer
//!    constant expression with value `0` is C's null pointer constant
//!    (C11 6.3.2.3p3), so `requires resource_literal_argument(p, 0);` declares
//!    a contract read off the arm where `p` is null. That is a legitimate C04
//!    use, exercised end to end by the gateway's null-sentinel fixture.
//! 2. A premise on any parameter can only ever **withdraw** call sites, so a
//!    premise that no call site can satisfy — a nonzero literal at a pointer
//!    parameter, which is not even valid C without a cast — is vacuous in the
//!    *refusing* direction. Restricting buys no soundness.
//! 3. The retired comment directive imposed no such restriction either, so
//!    adding one would move the migration baseline in the refusing direction
//!    with no measured need.
//!
//! What the loader does still refuse is unchanged: the name must be a parameter
//! of this declaration, and at most one premise may name each parameter.
//!
//! Total-or-reject applies in full: an unknown parameter name, a malformed
//! literal, a second premise on the same parameter, and any other `requires`
//! spelling each reject the **whole module**, so a typo can never leave a
//! declaration silently unrestricted.
//!
//! Like the variadic tail, the premise is **not** carried on
//! [`ResourceFunctionContract`]; it is rederived from the retained trusted
//! source texts by [`literal_argument_premises`], so an environment with no
//! declared sources — an inferred W4 summary, a provider proposal, a directly
//! constructed table — declares no premises. That direction is fail-closed for
//! the *loader* (no premise is claimed) and consumers must treat a rederivation
//! failure over sources that mention the clause as a refusal, never as an empty
//! premise set: `qcore_c_gateway::legacy_literal_premise` does exactly that.
//!
//! The mechanism this clause replaces was the machine-parsed
//! `// qcore-premise: literal_argument(...)` comment directive of card
//! HASH-CONTRACTS. That carrier is **retired**, and a leftover occurrence of it
//! must fail loudly rather than rot: a module whose text still contains the
//! directive introducer is rejected by name — see
//! [`RETIRED_PREMISE_DIRECTIVE`]. Silently ignoring one would drop a declared
//! restriction, which is exactly the fail-open direction.
//!
//! Signatures are opaque C ABI projections, not checked C layouts. The type
//! vocabulary is likewise an explicit table, not a wildcard:
//!
//! - [`SCALAR_ABI_TYPES`] — sizes, counts, flags, descriptors, offsets and
//!   real-valued readings. Borrowed only: a scalar cannot carry custody, so
//!   every custody mode on one is rejected. Card NEIGHBOURS admitted `f32` and
//!   `f64` under exactly the condition this note stated for them — a resource
//!   API in scope that passes one: Fluent Bit's `double flb_time_now()`
//!   (`src/flb_lib.c:936`), whose reading `flb_input_chunk_create` stores in
//!   `ic->create_time`. Before that there was no use, and the table had none.
//! - [`POINTER_ABI_TYPES`] — the custody-bearing projections. `resource_out`
//!   additionally requires `ptr<mut, ptr<mut, u8>>`, and any non-`None` result
//!   requires a pointer return type.
//! - [`FUNCTION_POINTER_ABI_TYPE`] (`fnptr`) — the projection of a pointer to
//!   function. Borrowed only, by the same two gates that hold every scalar
//!   borrowed-only: both key on the leading word `ptr`, which `fnptr` is not.
//! - `unit` is a result type only, never a parameter type.
//!
//! Acquire/Release/Escape/Neutral compatibility kinds map respectively to Owned
//! results, Released/Consumed parameters, and borrowed-only/None contracts.
//! Non-null error sentinels (a resource returned as `MAP_FAILED` or `-1` rather
//! than null) remain unrepresentable: [`ResourceReturnMode`] records only
//! nullability, so expressing them needs target-interface work, not loader
//! vocabulary, and they are rejected meanwhile. Conditional effects, effects
//! rows, quantitative clauses, arbitrary predicates and other exits are
//! likewise rejected.
//!
//! Files are trusted assumptions, not proofs. The exact input texts are retained
//! in ResourceContracts and independently reloaded by the certificate checker.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_inclusion::{
    ContractInclusionError, ContractLoadPath, Elaborated, Elaborator, InclusionCode,
    LoadPathResolver, module_key, reject_unresolved_inclusion, replay_evidence,
};
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::contract::{
    DerivedExtent, ResourceContracts, ResourceFunctionContract, ResourceKind,
    ResourceParameterMode, ResourceReturnMode,
};

/// Opaque C ABI scalar projections. Borrowed only: they carry sizes, counts,
/// flags, descriptors, offsets and real-valued readings, never custody.
/// Widening this table changes which signatures can be *written*, never what a
/// loaded contract means.
///
/// `f32`/`f64` were added by card NEIGHBOURS so that Fluent Bit's
/// `double flb_time_now()` could be declared with its OWN return type rather
/// than projected onto an integer it does not have. Nothing else moved with
/// them: a float is refused in every custody mode exactly as every other scalar
/// is, and a non-`None` result still requires a pointer return type, so no
/// float can occupy a custody position. `unit` and every pointer spelling
/// outside [`POINTER_ABI_TYPES`] stay refused as parameter types.
pub const SCALAR_ABI_TYPES: &[&str] = &[
    "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32", "f64",
];

/// Opaque C ABI pointer projections. These are the custody-bearing types.
pub const POINTER_ABI_TYPES: &[&str] =
    &["ptr<mut, u8>", "ptr<const, u8>", "ptr<mut, ptr<mut, u8>>"];

/// The opaque C ABI projection of a **pointer to function** (card
/// **CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF**).
///
/// It is deliberately *not* an entry of [`POINTER_ABI_TYPES`] and deliberately
/// not spelled `ptr<…>`. A function designator is never a resource identity in
/// this model, so a function pointer can never carry custody, and the two gates
/// that decide custody both key on the leading word `ptr`:
/// `custody mode requires a pointer parameter` and
/// `owned result requires a pointer return type`. Spelling this projection
/// `fnptr` therefore makes every custody mode on it — and an owned result of it
/// — a **load-time rejection**, by the rules that were already there.
///
/// It is not folded onto `u64`/`usize` either. That would be the same
/// eight-byte reading on the LP64 targets audited here, but it would also make
/// a declaration of an *integer* parameter and a declaration of a *callback*
/// parameter indistinguishable, and card LOADER-ABI's whole point is that the
/// declared spelling is compared against the C declaration. A separate spelling
/// keeps the comparison exact in both directions: a `fnptr` declaration is
/// refused for every C type that is not a pointer to function, and every other
/// spelling is refused for one that is.
///
/// Like [`SCALAR_ABI_TYPES`], widening the vocabulary changes which signatures
/// can be *written*, never what a loaded contract means. The condition card
/// NEIGHBOURS stated for `f32`/`f64` is met here too: a resource API in scope
/// passes one — msgpack's
/// `msgpack_packer_init(msgpack_packer *, void *, msgpack_packer_write)`,
/// whose third parameter is `typedef int (*msgpack_packer_write)(void *, const
/// char *, size_t)` and which the production `flb_input_chunk_create` calls.
pub const FUNCTION_POINTER_ABI_TYPE: &str = "fnptr";

/// The introducer of the retired `qcore-premise` comment directive (card
/// HASH-CONTRACTS, superseded by HASH-CLAUSE's `resource_literal_argument`
/// clause).
///
/// A contract module whose text still contains it is **rejected**, anywhere in
/// the text and in any comment shape. Ignoring a leftover directive would drop
/// the restriction it declares and silently widen the declaration back to every
/// call site, so the refusal is deliberately broader than the shape the
/// directive parser recognised: prose that names the retired introducer belongs
/// in this loader's own documentation, never in a contract module.
pub const RETIRED_PREMISE_DIRECTIVE: &str = "qcore-premise:";

/// Reject a module carrying the retired directive.
fn reject_retired_premise_directive(path: &str, source: &str) -> Result<(), String> {
    if source.contains(RETIRED_PREMISE_DIRECTIVE) {
        return Err(format!(
            "{path}: the `{RETIRED_PREMISE_DIRECTIVE}` comment directive is retired; \
             declare `requires resource_literal_argument(PARAMETER, LITERAL);` instead"
        ));
    }
    Ok(())
}

/// One declared literal-argument premise (card HASH-CLAUSE): the contract
/// applies only where the actual at `parameter` is exactly `literal`.
///
/// `literal` is the token *texts* a C lexer produces for the declared spelling,
/// so a consumer comparing against a C token stream compares vectors and never
/// re-parses a number: `0` is `["0"]` and `-1` is `["-", "1"]`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredLiteralArgument {
    /// Index into the declaration's parameter list.
    pub parameter: usize,
    /// The declared literal, as C token texts.
    pub literal: Vec<String>,
}

/// The declared C ABI projection of one admitted declaration (card
/// **LOADER-ABI**).
///
/// The custody modes on [`ResourceFunctionContract`] are what the analyzer
/// applies; these are the *types* the declaration claims those modes sit on.
/// They are discarded by [`ResourceFunctionContract`] on purpose — nothing in
/// the model reads a type — which is exactly why nothing used to compare them
/// against the analyzed translation unit's own C declaration. A consumer that
/// wants that comparison rederives this from the retained trusted texts, the
/// same way [`variadic_data_tails`] and [`literal_argument_premises`] are
/// rederived, so W4 derivation, W4 validation and certificate rechecking all
/// reproduce it from the bytes they already hold.
///
/// Every spelling here is an entry of [`SCALAR_ABI_TYPES`],
/// [`POINTER_ABI_TYPES`] or, for `result` only, `unit`: the loader admits no
/// other spelling, so a consumer compares against a closed table.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredAbiSignature {
    /// The declared parameter type spellings, in declaration order.
    pub parameters: Vec<String>,
    /// The declared result type spelling, `unit` included.
    pub result: String,
    /// Whether the declaration carries `resource_variadic_data_tail()`.
    pub variadic_data_tail: bool,
}

/// Everything one admitted declaration carries beyond its custody modes.
struct LoadedDeclaration {
    contract: ResourceFunctionContract,
    variadic_data_tail: bool,
    literal_arguments: Vec<DeclaredLiteralArgument>,
    abi: DeclaredAbiSignature,
}

/// The parameter vocabulary: every scalar, every pointer, and `fnptr`.
fn parameter_abi_types() -> impl Iterator<Item = &'static str> {
    SCALAR_ABI_TYPES
        .iter()
        .copied()
        .chain(POINTER_ABI_TYPES.iter().copied())
        .chain(std::iter::once(FUNCTION_POINTER_ABI_TYPE))
}

/// The result vocabulary: `unit` plus every parameter type.
fn result_abi_types() -> impl Iterator<Item = &'static str> {
    std::iter::once("unit").chain(parameter_abi_types())
}

/// Load all selected modules atomically. No partial environment escapes on error.
pub fn load_resource_contract_files(paths: &[PathBuf]) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&read_resource_contract_files(paths)?)
}

/// Versioned selection of resource modules; paths are relative to the manifest.
///
/// **Schema 1** selects files: every entry is a path relative to the manifest.
///
/// **Schema 2** (card **CONTRACT-INHERIT-ADOPTION**) adds a required
/// `load_path` (a load-path manifest, relative to this one; see
/// [`crate::load_path_manifest`]) and admits a second kind of entry: a
/// `crate:module` reference, spelled exactly like an `#include` target, which
/// selects a module owned by a crate on that load path and is loaded through
/// [`load_published_resource_contracts`]'s elaboration and v1 agreement gate.
/// An entry is a reference when it contains `:`; a path entry may not. Path
/// entries resolve **identically** to schema 1: the same file, read the same
/// way, retained under the same key. So a schema-2 manifest whose entries are
/// all paths loads exactly the environment its schema-1 twin loads (test
/// `schema_two_path_entries_load_exactly_as_schema_one`). Everything is
/// total-or-reject: a stale or invalid load path, a malformed reference, a
/// module selected twice, or two declarations of one function refuse the
/// whole selection.
pub fn load_resource_contract_manifest(path: &Path) -> Result<ResourceContracts, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        schema_version: u32,
        #[serde(default)]
        load_path: Option<String>,
        modules: Vec<String>,
    }
    let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&source).map_err(|e| format!("{}: {e}", path.display()))?;
    let supported = match manifest.schema_version {
        1 => manifest.load_path.is_none(),
        2 => manifest.load_path.is_some(),
        _ => false,
    };
    if !supported || manifest.modules.is_empty() {
        return Err(format!(
            "{}: unsupported version or empty module selection",
            path.display()
        ));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let relative = |entry: &str| -> Result<PathBuf, String> {
        let module = Path::new(entry);
        if entry.is_empty()
            || module
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err("contract module paths must be relative without traversal".to_owned());
        }
        Ok(parent.join(module))
    };
    let mut paths = Vec::new();
    let mut references = Vec::new();
    for entry in &manifest.modules {
        if manifest.schema_version == 2 && entry.contains(':') {
            references.push(
                published_module_reference(entry)
                    .map_err(|e| format!("{}: {e}", path.display()))?,
            );
        } else {
            paths.push(relative(entry)?);
        }
    }
    let Some(load_path) = manifest.load_path else {
        return load_resource_contract_files(&paths);
    };
    let load_path = relative(&load_path)?;
    // Validated even when every entry is a path: a schema-2 manifest names a
    // load path, so a stale one is refused rather than ignored.
    let load_path = crate::load_path_manifest::load_contract_load_path_manifest(&load_path)?;
    let path_sources = read_resource_contract_files(&paths)?;
    if references.is_empty() {
        return load_resource_contract_sources(&path_sources);
    }
    load_resource_contract_selection(&load_path.load_path, path_sources, &references)
}

/// Parse a manifest's `crate:module` entry, with the `#include` target's
/// admitted spelling (card **CONTRACT-INHERIT-ADOPTION**).
fn published_module_reference(entry: &str) -> Result<PublishedModuleRef, String> {
    use qcore_frontend::contract_inclusion::{
        valid_contract_crate_name, valid_contract_module_path,
    };
    match entry.split_once(':') {
        Some((crate_name, module))
            if valid_contract_crate_name(crate_name) && valid_contract_module_path(module) =>
        {
            Ok(PublishedModuleRef::new(crate_name, module))
        }
        _ => Err(format!(
            "`{entry}` is not a `crate:module` reference (a lowercase crate name, `:`, and a \
             relative `[a-z0-9_/-]+.qcore` module path)"
        )),
    }
}

/// Read selected files into the snapshot [`load_resource_contract_files`]
/// loads, keyed by the path as spelled.
fn read_resource_contract_files(paths: &[PathBuf]) -> Result<BTreeMap<String, String>, String> {
    let mut sources = BTreeMap::new();
    for path in paths {
        let name = path.to_string_lossy().into_owned();
        let source =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if sources.insert(name.clone(), source).is_some() {
            return Err(format!("duplicate contract input {name}"));
        }
    }
    Ok(sources)
}

/// Load an immutable source snapshot, also used for certificate replay.
pub fn load_resource_contract_sources(
    sources: &BTreeMap<String, String>,
) -> Result<ResourceContracts, String> {
    if sources.is_empty() {
        return Err("no resource contract inputs selected".into());
    }
    let mut contracts = ResourceContracts::default();
    for (path, source) in sources {
        reject_retired_premise_directive(path, source)?;
        reject_unresolved_inclusion(path, source)?;
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            if contracts.contract(&decl.name).is_some() {
                return Err(format!("{context}: duplicate function contract"));
            }
            let loaded = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            contracts.insert_contract(&decl.name, loaded.contract);
        }
    }
    contracts.set_source_assumptions(sources.clone());
    Ok(contracts)
}

/// The names declaring a `resource_variadic_data_tail()` in `sources`.
///
/// Derived from the same trusted texts [`load_resource_contract_sources`]
/// reads, and total-or-reject in the same way: a source set this loader would
/// reject yields an error here too, never a partial set.
pub fn variadic_data_tails(sources: &BTreeMap<String, String>) -> Result<BTreeSet<String>, String> {
    let mut tails = BTreeSet::new();
    for (path, source) in sources {
        reject_retired_premise_directive(path, source)?;
        reject_unresolved_inclusion(path, source)?;
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            let loaded = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            if loaded.variadic_data_tail {
                tails.insert(decl.name.clone());
            }
        }
    }
    Ok(tails)
}

/// The declared C ABI projections in `sources`, keyed by function name (card
/// **LOADER-ABI**).
///
/// Derived from the same trusted texts [`load_resource_contract_sources`]
/// reads, and total-or-reject in the same way: a source set this loader would
/// reject yields an error here too, never a partial map. A consumer must
/// therefore treat an error as a refusal of every comparison, never as an
/// empty map — an empty map compares nothing, which is the fail-open
/// direction.
pub fn declared_abi_signatures(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DeclaredAbiSignature>, String> {
    let mut signatures = BTreeMap::new();
    for (path, source) in sources {
        reject_retired_premise_directive(path, source)?;
        reject_unresolved_inclusion(path, source)?;
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            let loaded = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            if signatures.insert(decl.name.clone(), loaded.abi).is_some() {
                return Err(format!("{context}: duplicate function contract"));
            }
        }
    }
    Ok(signatures)
}

/// The declared C ABI projections of an environment, rederived from its
/// **retained trusted sources**.
///
/// An environment with no declared sources — an inferred W4 summary, a
/// provider proposal, a directly constructed table — declares no signatures,
/// so nothing is compared. That is the honest reading: this loader claims no
/// declared type it cannot read, and an inferred summary never had one.
pub fn declared_abi_signatures_of(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, DeclaredAbiSignature>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    declared_abi_signatures(contracts.source_assumptions())
}

/// The literal-argument premises `sources` declares, keyed by function name
/// (card HASH-CLAUSE).
///
/// Derived from the same trusted texts [`load_resource_contract_sources`] reads
/// and total-or-reject in the same way: a source set this loader would reject
/// yields an error here too, never a partial set. A function with no premise is
/// absent from the map, which is *unrestricted* — so a consumer must treat an
/// error as a refusal rather than as an empty map.
pub fn literal_argument_premises(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<DeclaredLiteralArgument>>, String> {
    let mut premises = BTreeMap::new();
    for (path, source) in sources {
        reject_retired_premise_directive(path, source)?;
        reject_unresolved_inclusion(path, source)?;
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            let loaded = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            if !loaded.literal_arguments.is_empty() {
                // Function names are unique across a selection, so a name that
                // is already present is a duplicate declaration, which
                // `load_resource_contract_sources` rejects; refuse here too
                // rather than silently keeping one of them.
                if premises
                    .insert(decl.name.clone(), loaded.literal_arguments)
                    .is_some()
                {
                    return Err(format!("{context}: duplicate function contract"));
                }
            }
        }
    }
    Ok(premises)
}

/// The literal-argument premises of an environment, rederived from its
/// **retained trusted sources**.
///
/// An environment with no declared sources declares no premises. That is the
/// honest reading — the loader claims no restriction it cannot read — and it is
/// why a consumer that acts on premises must refuse, not proceed, when
/// rederivation over sources that mention the clause fails.
pub fn declared_literal_argument_premises(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, Vec<DeclaredLiteralArgument>>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    literal_argument_premises(contracts.source_assumptions())
}

/// The variadic data tails of an environment, rederived from its **retained
/// trusted sources**.
///
/// An environment with no declared sources — an inferred W4 summary, a provider
/// proposal, a directly constructed table — has no tails, which is the
/// fail-closed direction: the tail is a declaration, never an inference.
pub fn declared_variadic_data_tails(
    contracts: &ResourceContracts,
) -> Result<BTreeSet<String>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeSet::new());
    }
    variadic_data_tails(contracts.source_assumptions())
}

/// Recheck a declarative environment against its declared trusted sources
/// without consulting mutable files. Lives here rather than on
/// `ResourceContracts` because the recheck needs the front-end parser, which
/// the schema crate must not depend on.
///
/// Card CONTRACT-INHERIT: an environment that carries inclusion evidence is
/// also rechecked by **replaying** every elaboration from the retained texts
/// alone — no load path, no file — and requiring the replay to reproduce the
/// retained elaborated source byte for byte, the recorded chains, and the
/// consumer agreement between every inherited declaration and its parent.
/// Evidence for a module the environment does not retain is refused.
pub fn check_source_assumptions(contracts: &ResourceContracts) -> Result<(), String> {
    if !contracts.source_assumptions().is_empty() {
        let loaded = load_resource_contract_sources(contracts.source_assumptions())?;
        if loaded.functions() != contracts.functions() {
            return Err("resource contracts differ from their declared source assumptions".into());
        }
    }
    for (key, evidence) in contracts.inclusion_evidence() {
        let Some(elaborated_source) = contracts.source_assumptions().get(key) else {
            return Err(ContractInclusionError {
                code: InclusionCode::Qi016,
                module: key.clone(),
                message: "inclusion evidence for a module the environment does not retain".into(),
            }
            .into());
        };
        let elaborated = replay_evidence(key, evidence, elaborated_source)?;
        inherited_declarations_agree(&elaborated)?;
    }
    Ok(())
}

/// The published provenance of an environment's inherited contracts (card
/// **CONTRACT-INHERIT**), for a report's assumptions block: each selected
/// module's identity, its text as written, every included module's identity
/// and text, and each inheritance chain. `None` for an environment that used
/// no inclusion, so a report without it is unchanged.
#[must_use]
pub fn inclusion_provenance(contracts: &ResourceContracts) -> Option<serde_json::Value> {
    if contracts.inclusion_evidence().is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "trust": "inherited contracts are declared assumptions of the same strength as hand-written ones; each chain records where the assumption was written, not evidence that it holds",
        "modules": contracts.inclusion_evidence(),
    }))
}

/// A module selected from a published load path, by crate and logical path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublishedModuleRef {
    pub crate_name: String,
    pub module: String,
}

impl PublishedModuleRef {
    #[must_use]
    pub fn new(crate_name: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            crate_name: crate_name.into(),
            module: module.into(),
        }
    }
}

/// Load resource-v1 contracts from modules selected on a published load path
/// (card **CONTRACT-INHERIT**).
///
/// Each selected module is elaborated — its `#include` directives resolved
/// under the publication rule, every `contract inherit PARENT;` replaced by
/// the parent's clauses under the positional renaming — and the elaborated
/// texts are loaded by the unchanged total-or-reject v1 loader, keyed
/// `crate:module`. Only the selected modules' declarations enter the
/// environment; an included module contributes parents, not functions.
///
/// Then, for every inherited declaration, the elaborated child and its
/// immediate parent are both loaded as v1 declarations and must be **equal**:
/// custody modes, result mode, literal-argument premises, variadic tail and
/// ABI projection (`QI012` otherwise). That is the check that the renaming
/// means the same thing to this consumer, which reads some positions
/// structurally rather than by parameter lookup. Equality makes every
/// inherited contract exactly as strong as its parent: never weaker.
///
/// The environment carries [`ContractInclusionEvidence`] for every selected
/// module that used inclusion, and is rechecked by
/// [`check_source_assumptions`] before it is returned.
pub fn load_published_resource_contracts(
    load_path: &ContractLoadPath,
    selection: &[PublishedModuleRef],
) -> Result<ResourceContracts, String> {
    load_resource_contract_selection(load_path, BTreeMap::new(), selection)
}

/// Load a mixed selection (card **CONTRACT-INHERIT-ADOPTION**): modules
/// already read from files, retained under their path keys exactly as
/// [`load_resource_contract_sources`] retains them, together with modules
/// selected on a published load path, elaborated and gated exactly as
/// [`load_published_resource_contracts`] does and retained under their
/// `crate:module` keys. One total-or-reject load over the union: a key
/// selected twice, or a function declared twice across the two kinds, refuses
/// the whole selection.
pub fn load_resource_contract_selection(
    load_path: &ContractLoadPath,
    path_sources: BTreeMap<String, String>,
    selection: &[PublishedModuleRef],
) -> Result<ResourceContracts, String> {
    if selection.is_empty() && path_sources.is_empty() {
        return Err("no resource contract inputs selected".into());
    }
    let resolver = LoadPathResolver(load_path);
    let mut elaborator = Elaborator::new(&resolver);
    let mut sources = path_sources;
    let mut evidence = BTreeMap::new();
    let mut elaborated_modules = Vec::new();
    for reference in selection {
        let (identity, text) = load_path.owned_module(&reference.crate_name, &reference.module)?;
        let key = module_key(&identity);
        let elaborated = elaborator.elaborate(identity, &text)?;
        if sources
            .insert(key.clone(), elaborated.text.clone())
            .is_some()
        {
            return Err(format!("duplicate contract input {key}"));
        }
        if elaborated.uses_inclusion {
            evidence.insert(key, elaborated.evidence());
        }
        elaborated_modules.push(elaborated);
    }
    // Agreement first, so a renaming the v1 reading cannot accept is reported
    // as the inheritance refusal it is rather than as an anonymous v1 error.
    for elaborated in &elaborated_modules {
        inherited_declarations_agree(elaborated)?;
    }
    let mut contracts = load_resource_contract_sources(&sources)?;
    contracts.set_inclusion_evidence(evidence);
    check_source_assumptions(&contracts)?;
    Ok(contracts)
}

/// The v1 consumer's agreement check for every inherited declaration of one
/// elaborated module: the loaded child equals the loaded immediate parent.
fn inherited_declarations_agree(elaborated: &Elaborated) -> Result<(), String> {
    for child_name in elaborated.chains.keys() {
        let refuse = |message: String| -> String {
            ContractInclusionError {
                code: InclusionCode::Qi012,
                module: module_key(&elaborated.identity),
                message,
            }
            .into()
        };
        let child = elaborated
            .declaration(child_name)
            .ok_or_else(|| refuse(format!("`{child_name}` is missing after elaboration")))?;
        let parent = elaborated
            .parent_declaration(child_name)
            .ok_or_else(|| refuse(format!("the parent of `{child_name}` is missing")))?;
        let child_loaded = load_function(child)
            .map_err(|e| refuse(format!("`{child_name}` does not load as resource v1: {e}")))?;
        let parent_loaded = load_function(parent).map_err(|e| {
            refuse(format!(
                "the parent `{}` of `{child_name}` does not load as resource v1: {e}",
                parent.name
            ))
        })?;
        if child_loaded.contract != parent_loaded.contract
            || child_loaded.variadic_data_tail != parent_loaded.variadic_data_tail
            || child_loaded.literal_arguments != parent_loaded.literal_arguments
            || child_loaded.abi != parent_loaded.abi
        {
            return Err(refuse(format!(
                "`{child_name}` would not mean what its parent `{}` means under the resource-v1 \
                 reading: the renamed contract loads differently",
                parent.name
            )));
        }
    }
    Ok(())
}

fn token_kinds(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect())
}

fn load_function(decl: &SourceDeclaration) -> Result<LoadedDeclaration, String> {
    if decl.kind != DeclarationKind::ExternFunction {
        return Err("resource modules accept only extern functions".into());
    }
    let clauses = &decl
        .function_contract
        .as_ref()
        .ok_or("missing resource contract")?
        .clauses;
    // Reconstruct the entire declaration, allowing only plain C ABI types and
    // the exact parsed contract block. No ignored signature/tail syntax.
    // `unit` is a result type only, and usage annotations belong to the
    // language-level lifecycle vocabulary, not to this projection.
    let mut parameter_types = Vec::with_capacity(decl.parameters.len());
    for parameter in &decl.parameters {
        let tokens = token_kinds(&parameter.type_source)?;
        let matched =
            parameter_abi_types().find(|ty| token_kinds(ty).ok().as_ref() == Some(&tokens));
        let Some(matched) = matched.filter(|_| parameter.usage.is_none()) else {
            return Err("unsupported parameter type or usage annotation".into());
        };
        // Card LOADER-ABI: the canonical table spelling, not the module's own
        // text, so a consumer comparing against a C declaration compares one
        // closed vocabulary rather than incidental whitespace.
        parameter_types.push(matched.to_owned());
    }
    let parameter_text = decl
        .parameters
        .iter()
        .map(|p| format!("{}: {}", p.name.value, p.type_source))
        .collect::<Vec<_>>()
        .join(", ");
    let clause_text = clauses
        .iter()
        .map(|c| {
            let prefix = match &c.value.kind {
                SourceContractClauseKind::Requires => "requires",
                SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                    "ensures normal"
                }
                _ => return Err("unsupported error, any-exit, or quantitative clause"),
            };
            Ok(format!("{prefix} {};", c.value.expression_source))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    let return_type = result_abi_types()
        .find(|ty| {
            let expected = format!(
                "extern fn {}({parameter_text}) -> {ty} contract {{ {clause_text} }};",
                decl.name
            );
            token_kinds(&expected).ok().as_ref() == Some(&decl.tokens)
        })
        .ok_or("unsupported signature, effects, or declaration suffix")?;

    let mut marker = false;
    let mut variadic_data_tail = false;
    let mut literal_arguments: Vec<DeclaredLiteralArgument> = Vec::new();
    let mut parameters = vec![None; decl.parameters.len()];
    let mut result = None;
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: which validity vocabulary each
    // position's nullability word was spelled in, and each declared kind.
    let mut parameter_words: Vec<Option<ValidityWord>> = vec![None; decl.parameters.len()];
    let mut result_word: Option<ValidityWord> = None;
    let mut parameter_kinds: Vec<Option<ResourceKind>> = vec![None; decl.parameters.len()];
    let mut result_kind: Option<ResourceKind> = None;
    let mut names = BTreeMap::new();
    for (index, parameter) in decl.parameters.iter().enumerate() {
        if names.insert(parameter.name.value.as_str(), index).is_some() {
            return Err("duplicate parameter name".into());
        }
    }
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    for clause in clauses {
        let expression = &clause.value.expression_source;
        if clause.value.kind == SourceContractClauseKind::Requires {
            if token_kinds(expression)? == token_kinds("resource_contract_v1")? {
                if marker {
                    return Err(format!(
                        "unsupported or duplicate requirement `{expression}`"
                    ));
                }
                marker = true;
                continue;
            }
            // Card HASH-CLAUSE. The only other admitted precondition: the
            // declaration applies at a call site only if the actual at the
            // named parameter is exactly the named literal. Anything else is
            // still an unsupported requirement, so the whole module rejects.
            let (parameter, literal) = literal_argument_clause(expression)?;
            let parameter = index(&parameter)?;
            if literal_arguments
                .iter()
                .any(|declared| declared.parameter == parameter)
            {
                return Err(format!(
                    "duplicate literal-argument premise for {}",
                    decl.parameters[parameter].name.value
                ));
            }
            literal_arguments.push(DeclaredLiteralArgument { parameter, literal });
            continue;
        }
        let words = predicate_call(expression)?;
        let words = words.iter().map(String::as_str).collect::<Vec<_>>();
        // The variadic data tail is a *whole-declaration* fact, not a parameter
        // or result mode: it says what the arguments the declaration does not
        // name carry. It takes no argument (a tail has no index to point at)
        // and, like every other clause, may occur at most once.
        if words.as_slice() == ["resource_variadic_data_tail"] {
            if variadic_data_tail {
                return Err("duplicate variadic data tail".into());
            }
            variadic_data_tail = true;
            continue;
        }
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the two kind clauses. Heap is
        // spelled by the absence of a clause, so `heap` itself is refused: one
        // spelling per meaning.
        match words.as_slice() {
            ["resource_kind", p, kind] => {
                let kind = declared_kind(kind)?;
                if parameter_kinds[index(p)?].replace(kind).is_some() {
                    return Err(format!("duplicate kind for {p}"));
                }
                continue;
            }
            ["resource_return_kind", kind] => {
                let kind = declared_kind(kind)?;
                if result_kind.replace(kind).is_some() {
                    return Err("duplicate result kind".into());
                }
                continue;
            }
            _ => {}
        }
        let return_mode = match words.as_slice() {
            ["resource_return_none"] => Some((ResourceReturnMode::None, None)),
            ["resource_return_owned", null] => {
                let (nullable, word) = validity(null)?;
                Some((ResourceReturnMode::Owned { nullable }, Some(word)))
            }
            ["resource_return_reallocated", parameter, null] => {
                let (nullable, word) = validity(null)?;
                Some((
                    ResourceReturnMode::Reallocated {
                        parameter: index(parameter)?,
                        nullable,
                    },
                    Some(word),
                ))
            }
            // Card W43-LIBC-RESULT-ALIAS. `const_dropped` is decided below,
            // from the declared types, once the whole signature is known.
            [
                name @ ("resource_return_alias" | "resource_return_interior"),
                parameter,
                null,
            ] => {
                let (nullable, word) = validity(null)?;
                Some((
                    ResourceReturnMode::Derived {
                        parameter: index(parameter)?,
                        extent: if *name == "resource_return_alias" {
                            DerivedExtent::Exact
                        } else {
                            DerivedExtent::Interior
                        },
                        nullable,
                        const_dropped: false,
                    },
                    Some(word),
                ))
            }
            _ => None,
        };
        if let Some((mode, word)) = return_mode {
            if result.replace(mode).is_some() {
                return Err("duplicate result mode".into());
            }
            result_word = word;
            continue;
        }
        let (parameter, mode, word) = match words.as_slice() {
            ["resource_borrowed", p] => (*p, ResourceParameterMode::Borrowed, None),
            [name, p, null] => {
                let (nullable, word) = validity(null)?;
                let mode = match *name {
                    "resource_owned" => ResourceParameterMode::Owned { nullable },
                    "resource_consumed" => ResourceParameterMode::Consumed { nullable },
                    "resource_released" => ResourceParameterMode::Released { nullable },
                    "resource_out" => ResourceParameterMode::Out { nullable },
                    _ => return Err(format!("unsupported predicate `{expression}`")),
                };
                (*p, mode, Some(word))
            }
            [
                name @ ("resource_stored" | "resource_removed"),
                p,
                owner,
                null,
            ] => {
                let owner_parameter = index(owner)?;
                let (nullable, word) = validity(null)?;
                (
                    *p,
                    if *name == "resource_stored" {
                        ResourceParameterMode::Stored {
                            owner_parameter,
                            nullable,
                        }
                    } else {
                        ResourceParameterMode::Removed {
                            owner_parameter,
                            nullable,
                        }
                    },
                    Some(word),
                )
            }
            _ => return Err(format!("unsupported predicate `{expression}`")),
        };
        let position = index(parameter)?;
        if parameters[position].replace(mode).is_some() {
            return Err(format!("duplicate mode for {parameter}"));
        }
        parameter_words[position] = word;
    }
    if !marker {
        return Err("missing resource_contract_v1 requirement".into());
    }
    let parameters = parameters
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("missing parameter mode")?;
    let mut result = result.ok_or("missing result mode")?;
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: where a kind may stand, and which
    // validity words and ABI types it takes. Checked before the pointer-only
    // rules below, to which an integer-carried kind is the one exception.
    let kinds = check_kinds(
        &KindPositions {
            parameters: &parameters,
            parameter_types: &parameter_types,
            parameter_words: &parameter_words,
            parameter_kinds: &parameter_kinds,
        },
        result,
        return_type,
        result_word,
        result_kind,
    )?;
    let integer_parameter = |i: usize| kinds.parameter_kind(i).is_integer();
    if result != ResourceReturnMode::None
        && !return_type.starts_with("ptr<")
        && !kinds.result_kind.is_integer()
    {
        return Err("owned result requires a pointer return type".into());
    }
    // Card W43-LIBC-RESULT-ALIAS. A derived result names a borrowed base of
    // depth one and is itself a pointer of depth one: the base passes no
    // custody through the call, so the relation is the only thing the result
    // carries, and a pointer-to-pointer or function pointer has no object for
    // it to point into.
    if let ResourceReturnMode::Derived {
        parameter,
        ref mut const_dropped,
        ..
    } = result
    {
        const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];
        if parameters[parameter] != ResourceParameterMode::Borrowed {
            return Err("a derived result requires a borrowed base parameter".into());
        }
        if !DEPTH_ONE.contains(&parameter_types[parameter].as_str())
            || !DEPTH_ONE.contains(&return_type)
        {
            return Err("a derived result requires a depth-one pointer base and result".into());
        }
        *const_dropped =
            parameter_types[parameter] == "ptr<const, u8>" && return_type == "ptr<mut, u8>";
    }
    for (i, mode) in parameters.iter().enumerate() {
        let ty = token_kinds(&decl.parameters[i].type_source)?;
        if *mode != ResourceParameterMode::Borrowed
            && !matches!(ty.first(), Some(TokenKind::Word(word)) if word == "ptr")
            && !integer_parameter(i)
        {
            return Err("custody mode requires a pointer parameter".into());
        }
        if matches!(mode, ResourceParameterMode::Out { .. })
            && ty != token_kinds("ptr<mut, ptr<mut, u8>>")?
        {
            return Err("out mode requires a pointer-to-pointer parameter".into());
        }
        if let ResourceParameterMode::Stored {
            owner_parameter, ..
        }
        | ResourceParameterMode::Removed {
            owner_parameter, ..
        } = mode
            && (*owner_parameter == i
                || !matches!(
                    parameters[*owner_parameter],
                    ResourceParameterMode::Owned { .. }
                ))
        {
            return Err("aggregate owner must be a distinct owned parameter".into());
        }
    }
    if let ResourceReturnMode::Reallocated { parameter, .. } = result
        && !matches!(parameters[parameter], ResourceParameterMode::Owned { .. })
    {
        return Err("reallocation requires an owned input parameter".into());
    }
    let mut contract = ResourceFunctionContract::new(parameters, result);
    contract.parameter_kinds = kinds.parameter_kinds;
    contract.result_kind = kinds.result_kind;
    Ok(LoadedDeclaration {
        contract,
        variadic_data_tail,
        literal_arguments,
        abi: DeclaredAbiSignature {
            parameters: parameter_types,
            result: return_type.to_owned(),
            variadic_data_tail,
        },
    })
}

/// `resource_literal_argument(PARAM, LITERAL)`, exactly (card HASH-CLAUSE).
///
/// This clause is the one place in the v1 vocabulary where an argument is a
/// *value* rather than a name, so it is parsed here rather than through
/// [`predicate_call`], whose identifier-only strictness is load-bearing for
/// every other predicate. `LITERAL` is a decimal integer with an optional
/// leading minus: the returned vector is the C token texts, which is what a
/// consumer compares an actual against. Alternate bases and suffixes lex as an
/// extra token and so fail this match rather than being approximated.
///
/// Card **HASH-REFINE** narrows the *declared* spelling to one canonical form
/// per value, so that the consumer-side spelling equivalence it admits (§C04.5)
/// has a unique target. Two spellings are refused here that the digits-only test
/// used to admit:
///
/// * a **leading zero** (`010`) — that is an octal constant in C, and this
///   clause decides no base; and
/// * a **negative zero** (`-0`) — one value with two declared spellings, which
///   would make the declared literal ambiguous where the call-site relation is
///   not.
///
/// Neither occurs in the corpus, and both refusals are strictly refusing: a
/// module using either no longer loads at all.
fn literal_argument_clause(source: &str) -> Result<(String, Vec<String>), String> {
    let unsupported = || format!("unsupported or duplicate requirement `{source}`");
    let tokens = token_kinds(source)?;
    let [
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Comma),
        literal @ ..,
    ] = tokens.as_slice()
    else {
        return Err(unsupported());
    };
    if name != "resource_literal_argument" {
        return Err(unsupported());
    }
    let (sign, digits) = match literal {
        [
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Integer(digits),
            TokenKind::Symbol(Symbol::RightParen),
        ] => (Some("-".to_owned()), digits),
        [
            TokenKind::Integer(digits),
            TokenKind::Symbol(Symbol::RightParen),
        ] => (None, digits),
        _ => return Err(unsupported()),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(unsupported());
    }
    // One canonical spelling per declared value (card HASH-REFINE): no octal
    // leading zero, and no second spelling of zero.
    if (digits.len() > 1 && digits.starts_with('0')) || (sign.is_some() && digits == "0") {
        return Err(unsupported());
    }
    Ok((
        parameter.clone(),
        sign.into_iter().chain([digits.clone()]).collect(),
    ))
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: which validity vocabulary a
/// position's nullability word belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValidityWord {
    /// `nullable` / `nonnull`: a pointer-carried identity whose failure value
    /// is NULL.
    Pointer,
    /// `sentinel` / `valid`: an integer-carried identity whose failure value is
    /// the sentinel `-1`.
    Integer,
}

/// A position's nullability word: whether the position may carry the failure
/// value, and in which vocabulary that was said. `nullable`/`nonnull` keep
/// their v1 meaning exactly; `sentinel`/`valid` are their integer-carried
/// counterparts and are admitted only where the position's kind is carried by
/// an integer (checked in [`check_kinds`]).
fn validity(word: &str) -> Result<(bool, ValidityWord), String> {
    match word {
        "nullable" => Ok((true, ValidityWord::Pointer)),
        "nonnull" => Ok((false, ValidityWord::Pointer)),
        "sentinel" => Ok((true, ValidityWord::Integer)),
        "valid" => Ok((false, ValidityWord::Integer)),
        _ => Err(format!("unsupported nullability {word}")),
    }
}

/// A declared non-heap kind. `heap` is spelled by absence and refused here, so
/// every meaning has exactly one spelling.
fn declared_kind(word: &str) -> Result<ResourceKind, String> {
    match ResourceKind::from_spelling(word) {
        Some(ResourceKind::Heap) => {
            Err("the heap kind is spelled by omitting the kind clause, not by `heap`".into())
        }
        Some(kind) => Ok(kind),
        None => Err(format!("unsupported resource kind {word}")),
    }
}

/// The parameter half of one declaration, as [`check_kinds`] reads it.
struct KindPositions<'a> {
    parameters: &'a [ResourceParameterMode],
    parameter_types: &'a [String],
    parameter_words: &'a [Option<ValidityWord>],
    parameter_kinds: &'a [Option<ResourceKind>],
}

/// The kinds a declaration's positions carry, once every placement rule holds.
struct DeclaredKinds {
    parameter_kinds: BTreeMap<usize, ResourceKind>,
    result_kind: ResourceKind,
}

impl DeclaredKinds {
    fn parameter_kind(&self, index: usize) -> ResourceKind {
        self.parameter_kinds
            .get(&index)
            .copied()
            .unwrap_or(ResourceKind::Heap)
    }
}

/// One position's validity word and ABI type against its kind.
fn kind_agrees(kind: ResourceKind, word: Option<ValidityWord>, ty: &str) -> Result<(), String> {
    let expected = if kind.is_integer() {
        ValidityWord::Integer
    } else {
        ValidityWord::Pointer
    };
    if word.is_some_and(|word| word != expected) {
        return Err(if kind.is_integer() {
            format!(
                "a {} position spells its validity `sentinel` or `valid`",
                kind.spelling()
            )
        } else {
            format!(
                "a {} position spells its nullability `nullable` or `nonnull`",
                kind.spelling()
            )
        });
    }
    if kind.is_integer() && ty != "i32" {
        return Err(format!("a {} position requires `i32`", kind.spelling()));
    }
    if !kind.is_heap() && !kind.is_integer() && !ty.starts_with("ptr<") {
        return Err(format!(
            "a {} position requires a pointer type",
            kind.spelling()
        ));
    }
    Ok(())
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: every placement rule of a kind.
///
/// * A kind clause names a custody-bearing position: an owned, consumed,
///   released or out parameter, or an owned result. A borrowed position
///   touches no obligation, and a stored, removed or reallocated position is
///   heap only, because the owner cascade and the replacement relation were
///   specified for allocations and a descriptor has neither.
/// * An integer-carried kind (`descriptor`) sits on an `i32`, spells its
///   validity `sentinel`/`valid`, and has no out slot here.
/// * Every other kind sits on an admitted pointer and spells its nullability
///   `nullable`/`nonnull`, exactly as a heap position does; so `sentinel` or
///   `valid` on a heap or pointer-kind position is refused.
fn check_kinds(
    positions: &KindPositions<'_>,
    result: ResourceReturnMode,
    return_type: &str,
    result_word: Option<ValidityWord>,
    result_kind: Option<ResourceKind>,
) -> Result<DeclaredKinds, String> {
    let mut declared = DeclaredKinds {
        parameter_kinds: BTreeMap::new(),
        result_kind: ResourceKind::Heap,
    };
    for (index, mode) in positions.parameters.iter().enumerate() {
        let kind = positions.parameter_kinds[index].unwrap_or(ResourceKind::Heap);
        if !kind.is_heap() {
            match mode {
                ResourceParameterMode::Owned { .. }
                | ResourceParameterMode::Consumed { .. }
                | ResourceParameterMode::Released { .. } => {}
                ResourceParameterMode::Out { .. } if !kind.is_integer() => {}
                ResourceParameterMode::Out { .. } => {
                    return Err(format!(
                        "a {} has no out slot in this vocabulary",
                        kind.spelling()
                    ));
                }
                ResourceParameterMode::Borrowed => {
                    return Err(
                        "a kind names a custody-bearing parameter, not a borrowed one".into(),
                    );
                }
                ResourceParameterMode::Stored { .. } | ResourceParameterMode::Removed { .. } => {
                    return Err("a stored or removed parameter is heap only".into());
                }
            }
            declared.parameter_kinds.insert(index, kind);
        }
        kind_agrees(
            kind,
            positions.parameter_words[index],
            &positions.parameter_types[index],
        )?;
    }
    let kind = result_kind.unwrap_or(ResourceKind::Heap);
    if !kind.is_heap() {
        if !matches!(result, ResourceReturnMode::Owned { .. }) {
            return Err("a result kind requires an owned result".into());
        }
        declared.result_kind = kind;
    }
    kind_agrees(kind, result_word, return_type)?;
    Ok(declared)
}

fn predicate_call(source: &str) -> Result<Vec<String>, String> {
    let tokens = token_kinds(source)?;
    let [
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::LeftParen),
        args @ ..,
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens.as_slice()
    else {
        return Err(format!("unsupported predicate `{source}`"));
    };
    let mut words = vec![name.clone()];
    for (i, token) in args.iter().enumerate() {
        match (i % 2, token) {
            (0, TokenKind::Word(word)) => words.push(word.clone()),
            (1, TokenKind::Symbol(Symbol::Comma)) => {}
            _ => return Err(format!("unsupported predicate argument `{source}`")),
        }
    }
    if !args.is_empty() && args.len() % 2 == 0 {
        return Err("trailing predicate comma".into());
    }
    Ok(words)
}
