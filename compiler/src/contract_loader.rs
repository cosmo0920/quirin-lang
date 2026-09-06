//! Total-or-reject bridge from native contract modules to legacy-C custody.
//!
//! Version 1 accepts only raw scalar/pointer extern signatures and the clauses
//! below. Every parameter and the result must occur exactly once. The marker
//! `requires resource_contract_v1` selects this trusted assumption vocabulary;
//! all remaining clauses must be `ensures normal` calls:
//!
//! - `resource_borrowed(p)` -> Borrowed (no custody requirement).
//! - `resource_{owned,consumed,released,out}(p, nullable|nonnull)` -> that mode.
//! - `resource_{stored,removed}(p, owner, nullable|nonnull)` -> indexed custody.
//! - `resource_return_none()` -> None; `resource_return_owned(nullability)` -> Owned.
//! - `resource_return_reallocated(p, nullability)` -> Reallocated, requiring
//!   an Owned parameter; null failure preserves it, success replaces it once.
//!
//! Acquire/Release/Escape/Neutral compatibility kinds map respectively to Owned
//! results, Released/Consumed parameters, and borrowed-only/None contracts.
//! Non-null error sentinels, conditional effects, quantitative clauses, effects
//! rows, arbitrary predicates and other exits cannot be expressed by the target
//! interface and are rejected. This loader does not weaken a richer contract by
//! dropping its predicates. Observation and lifecycle modules use their own
//! interpreters and are not implicitly selected by this loader.
//!
//! Files are trusted assumptions, not proofs. The exact input texts are retained
//! in ResourceContracts and independently reloaded by the certificate checker.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use crate::legacy_c::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use crate::lexer::{Symbol, TokenKind, lex};

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
            let contract = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            contracts.insert_contract(&decl.name, contract);
        }
    }
    contracts.set_source_assumptions(sources.clone());
    Ok(contracts)
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

fn load_function(decl: &SourceDeclaration) -> Result<ResourceFunctionContract, String> {
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
    let allowed_types = [
        "unit",
        "i32",
        "u32",
        "u8",
        "usize",
        "ptr<mut, u8>",
        "ptr<const, u8>",
        "ptr<mut, ptr<mut, u8>>",
    ];
    for parameter in &decl.parameters {
        let tokens = token_kinds(&parameter.type_source)?;
        if parameter.usage.is_some()
            || parameter.type_source == "unit"
            || !allowed_types
                .iter()
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
    let return_type = allowed_types
        .iter()
        .find(|ty| {
            let expected = format!(
                "extern fn {}({parameter_text}) -> {ty} contract {{ {clause_text} }};",
                decl.name
            );
            token_kinds(&expected).ok().as_ref() == Some(&decl.tokens)
        })
        .ok_or("unsupported signature, effects, or declaration suffix")?;

    let mut marker = false;
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
    Ok(ResourceFunctionContract::new(parameters, result))
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
