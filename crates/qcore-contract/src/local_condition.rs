//! S3 scoped condition and registered-footprint syntax, total or reject.
//! Member types, stability, predicate correspondence and per-instance
//! correlation are checked by source consumers, never asserted by this parser.
use qcore_frontend::frontend::{SourceContractClauseKind, SourceDeclaration};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::loop_lifetime::{
    ConditionBinding, ConditionGuard, LocalBlock, LocalClause, LoopLifetimeContract,
    RegisteredProtocol,
};
use std::collections::{BTreeMap, BTreeSet};

pub const SNAPSHOT_PREMISE_NAME: &str = "s3_condition_snapshot_stable";
pub const SNAPSHOT_PREMISE_STATEMENT: &str = "Every member read by this function's condition bindings preserves its entry value until each guarded use.";

fn tokens(source: &str) -> Result<Vec<TokenKind>, String> {
    if source.len() > 65_536 {
        return Err("local-condition-bound: clause exceeds 64 KiB".into());
    }
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .filter(|t| *t != TokenKind::Eof)
        .collect())
}
fn word(t: &TokenKind) -> Option<&str> {
    if let TokenKind::Word(w) = t {
        Some(w)
    } else {
        None
    }
}
fn symbol(t: &TokenKind, s: Symbol) -> bool {
    *t == TokenKind::Symbol(s)
}
fn path(ts: &[TokenKind]) -> Result<Vec<String>, String> {
    if ts.len() > 127 {
        return Err("local-condition-bound: path exceeds 64 member segments".into());
    }
    if ts.is_empty() || ts.len().is_multiple_of(2) {
        return Err("local-condition-path: expected a nonempty dotted path".into());
    }
    ts.iter()
        .enumerate()
        .map(|(i, t)| {
            if i % 2 == 0 {
                word(t)
                    .map(str::to_owned)
                    .ok_or_else(|| "local-condition-path: expected an identifier".into())
            } else if symbol(t, Symbol::Dot) {
                Ok(String::new())
            } else {
                Err("local-condition-path: expected `.`".into())
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into_iter().filter(|s| !s.is_empty()).collect())
}
fn binding(
    source: &str,
    formals: &BTreeMap<&str, (usize, &str)>,
) -> Result<ConditionBinding, String> {
    let ts = tokens(source)?;
    let shape =
        || "local-condition-binding: write `quant #C = (#P.FIELD[.FIELD] != 0);`".to_owned();
    if ts.len() < 11
        || !symbol(&ts[0], Symbol::Hash)
        || !symbol(&ts[2], Symbol::Equal)
        || !symbol(&ts[3], Symbol::LeftParen)
        || !symbol(&ts[4], Symbol::Hash)
        || !symbol(&ts[ts.len() - 3], Symbol::NotEqual)
        || ts[ts.len() - 2] != TokenKind::Integer("0".into())
        || !symbol(&ts[ts.len() - 1], Symbol::RightParen)
    {
        return Err(shape());
    }
    let name = word(&ts[1]).ok_or_else(shape)?;
    let root = word(&ts[5]).ok_or_else(shape)?;
    let Some((parameter, ty)) = formals.get(root) else {
        return Err("local-condition-binding: root is not a host formal".into());
    };
    if !matches!(*ty, "ptr<mut, u8>" | "ptr<const, u8>") {
        return Err("local-condition-binding: root must be a depth-one pointer formal".into());
    }
    if !symbol(&ts[6], Symbol::Dot) {
        return Err(shape());
    }
    let members = path(&ts[7..ts.len() - 3])?;
    Ok(ConditionBinding {
        name: name.into(),
        parameter: *parameter,
        members,
    })
}
/// Read a registered footprint whose two paths belong to the same record.
pub fn registered(source: &str) -> Result<RegisteredProtocol, String> {
    let ts = tokens(source)?;
    let shape = || {
        "registered-protocol-shape: write `quant registered #B: TAG.INTERIOR |-> TAG.HOLDER;`"
            .to_owned()
    };
    if ts.len() < 10 || !symbol(&ts[0], Symbol::Hash) || !symbol(&ts[2], Symbol::Colon) {
        return Err(shape());
    }
    let binder = word(&ts[1]).ok_or_else(shape)?.to_owned();
    let at = ts
        .iter()
        .position(|t| symbol(t, Symbol::MapsTo))
        .ok_or_else(shape)?;
    let left = path(&ts[3..at])?;
    let right = path(&ts[at + 1..])?;
    if left.len() < 2 || right.len() < 2 || left[0] != right[0] {
        return Err(
            "registered-protocol-record: both nonempty paths must belong to the same record".into(),
        );
    }
    Ok(RegisteredProtocol {
        binder,
        record: left[0].clone(),
        interior: left[1..].to_vec(),
        holder: right[1..].to_vec(),
    })
}
/// Structural identity of the supported named premise predicate. This does
/// not establish the premise; a body/use consumer must bind its guarded entry
/// context and preserve the corresponding caller obligation.
pub fn named_prerequisite(source: &str) -> Option<(String, String)> {
    let ts = tokens(source).ok()?;
    match ts.as_slice() {
        [
            TokenKind::Word(kind),
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(name),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::StringLiteral(statement),
            TokenKind::Symbol(Symbol::RightParen),
        ] if kind == "resource_premise" => Some((name.clone(), statement.clone())),
        _ => None,
    }
}
fn prerequisite(source: &str) -> Result<String, String> {
    let ts = tokens(source)?;
    let Some(name) = ts.first().and_then(word) else {
        return Err("local-condition-prerequisite: missing obligation".into());
    };
    if !matches!(
        name,
        "resource_premise" | "resource_loop_owner" | "resource_thread_sync" | "resource_lifecycle"
    ) {
        return Err("local-condition-prerequisite: only named premises and existing loop-owner, thread-sync or lifecycle obligations are conditional; vocabulary/custody/review markers stay unconditional".into());
    }
    Ok(source.to_owned())
}
/// Read all S3 clauses without dropping the ordinary contract's other clauses.
/// The owning resource loader validates each prerequisite with its existing
/// obligation reader before installing the declaration in a withheld table.
pub fn read(decl: &SourceDeclaration) -> Result<LoopLifetimeContract, String> {
    let Some(contract) = &decl.function_contract else {
        return Ok(LoopLifetimeContract::default());
    };
    let formals = decl
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.value.as_str(), (i, p.type_source.as_str())))
        .collect::<BTreeMap<_, _>>();
    let entry_snapshot_stable = contract.clauses.iter().any(|c| {
        c.value.kind == SourceContractClauseKind::Requires
            && named_prerequisite(&c.value.expression_source).as_ref()
                == Some(&(
                    SNAPSHOT_PREMISE_NAME.into(),
                    SNAPSHOT_PREMISE_STATEMENT.into(),
                ))
    });
    let mut out = LoopLifetimeContract {
        entry_snapshot_stable,
        ..Default::default()
    };
    let mut names = BTreeSet::new();
    for c in &contract.clauses {
        if c.value.kind == SourceContractClauseKind::QuantConditionBinding {
            let b = binding(&c.value.expression_source, &formals)?;
            if formals.contains_key(b.name.as_str())
                || matches!(
                    b.name.as_str(),
                    "outer" | "loaded" | "result" | "holds" | "absent" | "registered"
                )
                || !names.insert(b.name.clone())
            {
                return Err(
                    "local-condition-binding: duplicate or shadowed condition binder".into(),
                );
            }
            out.bindings.push(b);
        }
    }
    // C04.18 retains whole-declaration premise-name uniqueness, including
    // scoped prerequisites; validating clauses one at a time must not relax it.
    let mut premise_names = contract
        .clauses
        .iter()
        .filter_map(|clause| {
            (clause.value.kind == SourceContractClauseKind::Requires)
                .then(|| named_prerequisite(&clause.value.expression_source))
                .flatten()
                .map(|(name, _)| name)
        })
        .collect::<BTreeSet<_>>();
    let mut blocks = BTreeSet::new();
    let mut footprints = BTreeSet::new();
    for c in &contract.clauses {
        match &c.value.kind {
            SourceContractClauseKind::QuantRegistered => {
                let p = registered(&c.value.expression_source)?;
                if !footprints.insert((p.record.clone(), p.interior.clone())) {
                    return Err("registered-protocol-duplicate: one footprint per interior".into());
                }
                out.registered.push(p);
            }
            SourceContractClauseKind::QuantConditionBlock {
                condition,
                negated,
                clauses,
            } => {
                if !names.contains(condition) {
                    return Err("local-condition-unbound: block condition has no binding".into());
                }
                if !blocks.insert((condition.clone(), *negated)) {
                    return Err("local-condition-duplicate: repeated polarity block".into());
                }
                let inner = clauses
                    .iter()
                    .map(|c| match c.value.kind {
                        SourceContractClauseKind::Requires => {
                            let text = prerequisite(&c.value.expression_source)?;
                            if let Some((name, _)) = named_prerequisite(&text)
                                && !premise_names.insert(name)
                            {
                                return Err("local-prerequisite-duplicate: resource_premise name repeats in this declaration".into());
                            }
                            Ok(LocalClause::Prerequisite(text))
                        }
                        SourceContractClauseKind::QuantRegistered => {
                            registered(&c.value.expression_source).map(LocalClause::Registered)
                        }
                        _ => Err("local-condition-clause: unsupported conditional clause".into()),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                out.blocks.push(LocalBlock {
                    guard: ConditionGuard {
                        condition: condition.clone(),
                        negated: *negated,
                    },
                    clauses: inner,
                });
            }
            _ => {}
        }
    }
    if out
        .bindings
        .iter()
        .any(|b| !out.blocks.iter().any(|x| x.guard.condition == b.name))
    {
        return Err("local-condition-unused: condition has no local block".into());
    }
    Ok(out)
}

/// Rebuild a declaration with its ordinary clauses and optionally one inner
/// prerequisite. First compare the entire original token stream with its
/// canonical envelope, so unsupported ABI/effects/suffix text cannot vanish.
pub(crate) fn without_local(
    decl: &SourceDeclaration,
    extra: Option<&str>,
) -> Result<SourceDeclaration, String> {
    use qcore_frontend::frontend::{SourceContractExitSelector as E, parse_source_program};
    let clauses = &decl
        .function_contract
        .as_ref()
        .ok_or("missing resource contract")?
        .clauses;
    let render = |keep_local: bool| -> Result<String, String> {
        let mut rendered = Vec::new();
        for c in clauses {
            let prefix =
                match &c.value.kind {
                    SourceContractClauseKind::Requires => "requires",
                    SourceContractClauseKind::Ensures(E::Normal) => "ensures normal",
                    SourceContractClauseKind::QuantLive => "quant live",
                    SourceContractClauseKind::QuantEffect(_) => {
                        return Err("quant effect models are not resource summaries".into());
                    }
                    SourceContractClauseKind::QuantTheorem => return Err(
                        "local-condition: theorem applications have no interpreted local condition"
                            .into(),
                    ),
                    SourceContractClauseKind::QuantProposition => "quant proposition",
                    SourceContractClauseKind::QuantConditionBinding if keep_local => "quant",
                    SourceContractClauseKind::QuantConditionBlock { negated, .. } if keep_local => {
                        if *negated {
                            "quant not"
                        } else {
                            "quant"
                        }
                    }
                    SourceContractClauseKind::QuantRegistered if keep_local => "quant registered",
                    SourceContractClauseKind::QuantConditionBinding
                    | SourceContractClauseKind::QuantConditionBlock { .. }
                    | SourceContractClauseKind::QuantRegistered => continue,
                    _ => {
                        return Err(
                            "local-condition-contract: unsupported clause beside local conditions"
                                .into(),
                        );
                    }
                };
            rendered.push(format!("{prefix} {};", c.value.expression_source));
        }
        if !keep_local && let Some(extra) = extra {
            rendered.push(format!("requires {extra};"));
        }
        let parameters = decl
            .parameters
            .iter()
            .map(|p| format!("{}: {}", p.name.value, p.type_source))
            .collect::<Vec<_>>()
            .join(", ");
        let result = &decl
            .result_type
            .as_ref()
            .ok_or("local-condition-signature: explicit result required")?
            .source;
        Ok(format!(
            "extern fn {}({parameters}) -> {result} contract {{ {} }};",
            decl.name,
            rendered.join(" ")
        ))
    };
    if tokens(&render(true)?)? != decl.tokens {
        return Err(
            "local-condition-signature: unsupported signature, effect or declaration suffix".into(),
        );
    }
    let source = format!("module contract; {}", render(false)?);
    let parsed = parse_source_program(&source).map_err(|e| e.to_string())?;
    Ok(parsed
        .declarations
        .into_iter()
        .next()
        .ok_or("local-condition-signature: missing declaration")?
        .value)
}
