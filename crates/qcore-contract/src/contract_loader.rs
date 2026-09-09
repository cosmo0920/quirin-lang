//! Total-or-reject bridge from native contract modules to legacy-C custody.
//!
//! # Interpreter selection
//!
//! `contract/qcore` is shared by three consumers with disjoint vocabularies.
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
//! - `resource_variadic_data_tail()` -> the declaration additionally covers
//!   calls that pass *more* actuals than it names. See below.
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
//! Signatures are opaque C ABI projections, not checked C layouts. The type
//! vocabulary is likewise an explicit table, not a wildcard:
//!
//! - [`SCALAR_ABI_TYPES`] — sizes, counts, flags, descriptors and offsets.
//!   Borrowed only: a scalar cannot carry custody, so every custody mode on
//!   one is rejected. Float scalars are deliberately absent; no resource API
//!   in scope passes one, and this loader adds no vocabulary without a use.
//! - [`POINTER_ABI_TYPES`] — the custody-bearing projections. `resource_out`
//!   additionally requires `ptr<mut, ptr<mut, u8>>`, and any non-`None` result
//!   requires a pointer return type.
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

use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

/// Opaque C ABI scalar projections. Borrowed only: they carry sizes, counts,
/// flags, descriptors and offsets, never custody. Widening this table changes
/// which signatures can be *written*, never what a loaded contract means.
pub const SCALAR_ABI_TYPES: &[&str] = &[
    "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];

/// Opaque C ABI pointer projections. These are the custody-bearing types.
pub const POINTER_ABI_TYPES: &[&str] =
    &["ptr<mut, u8>", "ptr<const, u8>", "ptr<mut, ptr<mut, u8>>"];

/// The result vocabulary: `unit` plus every parameter type.
fn result_abi_types() -> impl Iterator<Item = &'static str> {
    std::iter::once("unit")
        .chain(SCALAR_ABI_TYPES.iter().copied())
        .chain(POINTER_ABI_TYPES.iter().copied())
}

/// Load all selected modules atomically. No partial environment escapes on error.
pub fn load_resource_contract_files(paths: &[PathBuf]) -> Result<ResourceContracts, String> {
    let mut sources = BTreeMap::new();
    for path in paths {
        let name = path.to_string_lossy().into_owned();
        let source =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if sources.insert(name.clone(), source).is_some() {
            return Err(format!("duplicate contract input {name}"));
        }
    }
    load_resource_contract_sources(&sources)
}

/// Versioned selection of resource modules; paths are relative to the manifest.
pub fn load_resource_contract_manifest(path: &Path) -> Result<ResourceContracts, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        schema_version: u32,
        modules: Vec<PathBuf>,
    }
    let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&source).map_err(|e| format!("{}: {e}", path.display()))?;
    if manifest.schema_version != 1 || manifest.modules.is_empty() {
        return Err(format!(
            "{}: unsupported version or empty module selection",
            path.display()
        ));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let paths = manifest
        .modules
        .iter()
        .map(|module| {
            if module
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err("contract module paths must be relative without traversal".to_owned());
            }
            Ok(parent.join(module))
        })
        .collect::<Result<Vec<_>, String>>()?;
    load_resource_contract_files(&paths)
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
            let (contract, _tail) = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            contracts.insert_contract(&decl.name, contract);
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
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        for declaration in parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            let (_, tail) = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            if tail {
                tails.insert(decl.name.clone());
            }
        }
    }
    Ok(tails)
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
pub fn check_source_assumptions(contracts: &ResourceContracts) -> Result<(), String> {
    if !contracts.source_assumptions().is_empty() {
        let loaded = load_resource_contract_sources(contracts.source_assumptions())?;
        if loaded.functions() != contracts.functions() {
            return Err("resource contracts differ from their declared source assumptions".into());
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

fn load_function(decl: &SourceDeclaration) -> Result<(ResourceFunctionContract, bool), String> {
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
    for parameter in &decl.parameters {
        let tokens = token_kinds(&parameter.type_source)?;
        if parameter.usage.is_some()
            || !SCALAR_ABI_TYPES
                .iter()
                .chain(POINTER_ABI_TYPES.iter())
                .any(|ty| token_kinds(ty).ok().as_ref() == Some(&tokens))
        {
            return Err("unsupported parameter type or usage annotation".into());
        }
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
    let mut parameters = vec![None; decl.parameters.len()];
    let mut result = None;
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
            if marker || token_kinds(expression)? != token_kinds("resource_contract_v1")? {
                return Err(format!(
                    "unsupported or duplicate requirement `{expression}`"
                ));
            }
            marker = true;
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
        let return_mode = match words.as_slice() {
            ["resource_return_none"] => Some(ResourceReturnMode::None),
            ["resource_return_owned", null] => Some(ResourceReturnMode::Owned {
                nullable: nullability(null)?,
            }),
            ["resource_return_reallocated", parameter, null] => {
                Some(ResourceReturnMode::Reallocated {
                    parameter: index(parameter)?,
                    nullable: nullability(null)?,
                })
            }
            _ => None,
        };
        if let Some(mode) = return_mode {
            if result.replace(mode).is_some() {
                return Err("duplicate result mode".into());
            }
            continue;
        }
        let (parameter, mode) = match words.as_slice() {
            ["resource_borrowed", p] => (*p, ResourceParameterMode::Borrowed),
            [name, p, null] => {
                let nullable = nullability(null)?;
                let mode = match *name {
                    "resource_owned" => ResourceParameterMode::Owned { nullable },
                    "resource_consumed" => ResourceParameterMode::Consumed { nullable },
                    "resource_released" => ResourceParameterMode::Released { nullable },
                    "resource_out" => ResourceParameterMode::Out { nullable },
                    _ => return Err(format!("unsupported predicate `{expression}`")),
                };
                (*p, mode)
            }
            [
                name @ ("resource_stored" | "resource_removed"),
                p,
                owner,
                null,
            ] => {
                let owner_parameter = index(owner)?;
                let nullable = nullability(null)?;
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
                )
            }
            _ => return Err(format!("unsupported predicate `{expression}`")),
        };
        if parameters[index(parameter)?].replace(mode).is_some() {
            return Err(format!("duplicate mode for {parameter}"));
        }
    }
    if !marker {
        return Err("missing resource_contract_v1 requirement".into());
    }
    let parameters = parameters
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("missing parameter mode")?;
    let result = result.ok_or("missing result mode")?;
    if result != ResourceReturnMode::None && !return_type.starts_with("ptr<") {
        return Err("owned result requires a pointer return type".into());
    }
    for (i, mode) in parameters.iter().enumerate() {
        let ty = token_kinds(&decl.parameters[i].type_source)?;
        if *mode != ResourceParameterMode::Borrowed
            && !matches!(ty.first(), Some(TokenKind::Word(word)) if word == "ptr")
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
    Ok((
        ResourceFunctionContract::new(parameters, result),
        variadic_data_tail,
    ))
}

fn nullability(word: &str) -> Result<bool, String> {
    match word {
        "nullable" => Ok(true),
        "nonnull" => Ok(false),
        _ => Err(format!("unsupported nullability {word}")),
    }
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
