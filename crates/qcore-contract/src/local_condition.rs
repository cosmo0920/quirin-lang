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
/// One `quant cell` clause as read: binder, record, cell path, `#same`
/// target (`None` for `#fresh(descriptor)`), guard path, guard, and (card
/// CELL-RESULT-SET-ALIGNMENT) the declared live and absent sets.
type CellClause = (
    String,
    String,
    Vec<String>,
    Option<Vec<String>>,
    Vec<String>,
    qcore_schemas::descriptor_cell::ProtocolGuard,
    CellSets,
);

/// A cell clause's `live_set(SET)` and `absent_set(SET)`, canonical intervals
/// over `int`.
type CellSets = (Option<Vec<(i64, i64)>>, Option<Vec<(i64, i64)>>);

/// Card CELL-RESULT-SET-ALIGNMENT (USER DECISIONS 2026-11-04, Q2 (i) and Q3):
/// split a guard's tokens at its `live_set(SET)` / `absent_set(SET)`
/// suffixes (each at most once, in that order), returning the guard tokens
/// and the sets in `int`'s universe.
fn cell_sets(guard: &[TokenKind]) -> Result<(&[TokenKind], CellSets), String> {
    let int = (i128::from(i32::MIN), i128::from(i32::MAX));
    let shape = || {
        "quant-cell-set: write `live_set(SET)` and then `absent_set(SET)` after the guard, each at most once".to_owned()
    };
    let at = |name: &str| guard.iter().position(|t| word(t) == Some(name));
    let (live_at, absent_at) = (at("live_set"), at("absent_set"));
    let end = live_at.or(absent_at).unwrap_or(guard.len());
    let read = |from: usize, to: usize| -> Result<Vec<(i64, i64)>, String> {
        let run = &guard[from..to];
        if run.len() < 4
            || !symbol(&run[1], Symbol::LeftParen)
            || !symbol(&run[run.len() - 1], Symbol::RightParen)
        {
            return Err(shape());
        }
        let set = crate::bounds_contract::set_of_tokens(&run[2..run.len() - 1], int)?;
        Ok(set
            .intervals()
            .iter()
            .map(|(lo, hi)| {
                (
                    i64::try_from(*lo).unwrap_or(i64::MIN),
                    i64::try_from(*hi).unwrap_or(i64::MAX),
                )
            })
            .collect())
    };
    let sets = match (live_at, absent_at) {
        (None, None) => (None, None),
        (Some(l), None) => (Some(read(l, guard.len())?), None),
        (None, Some(a)) => (None, Some(read(a, guard.len())?)),
        (Some(l), Some(a)) if l < a => (Some(read(l, a)?), Some(read(a, guard.len())?)),
        _ => return Err(shape()),
    };
    if guard
        .iter()
        .filter(|t| matches!(word(t), Some("live_set" | "absent_set")))
        .count()
        > usize::from(live_at.is_some()) + usize::from(absent_at.is_some())
    {
        return Err(shape());
    }
    Ok((&guard[..end], sets))
}

/// A member path of a `quant cell` clause: identifiers joined by `.`, each
/// segment carrying at most one `[K]` with `K` a canonical decimal.
fn cell_path(ts: &[TokenKind]) -> Result<Vec<String>, String> {
    let shape = || "quant-cell-path: expected `IDENT([K])(.IDENT([K]))*`".to_owned();
    if ts.len() > 255 {
        return Err("quant-cell-bound: path exceeds 64 member segments".into());
    }
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let name = ts.get(at).and_then(word).ok_or_else(shape)?;
        if matches!(name, "fresh" | "same" | "pointee" | "result") {
            return Err(shape());
        }
        let mut segment = name.to_owned();
        at += 1;
        if ts.get(at).is_some_and(|t| symbol(t, Symbol::LeftBracket)) {
            let Some(TokenKind::Integer(digits)) = ts.get(at + 1) else {
                return Err(shape());
            };
            if !ts
                .get(at + 2)
                .is_some_and(|t| symbol(t, Symbol::RightBracket))
                || digits.is_empty()
                || !digits.bytes().all(|b| b.is_ascii_digit())
                || (digits.len() > 1 && digits.starts_with('0'))
            {
                return Err(shape());
            }
            segment = format!("{segment}[{digits}]");
            at += 3;
        }
        out.push(segment);
        match ts.get(at) {
            None => return Ok(out),
            Some(t) if symbol(t, Symbol::Dot) => at += 1,
            Some(_) => return Err(shape()),
        }
    }
}

/// Read one `quant cell #B: TAG.CELL |-> #fresh(descriptor) when #B.GUARD OP V;`
/// or `… |-> #same(CELL2) when …` (card MK-EVENT-CELL-CROSS-FUNCTION, grammar
/// G05i). Total or reject.
fn cell(source: &str) -> Result<CellClause, String> {
    use qcore_schemas::descriptor_cell::ProtocolGuard;
    let ts = tokens(source)?;
    let shape = || {
        "quant-cell-shape: write `quant cell #B: TAG.CELL |-> #fresh(descriptor) when #B.GUARD OP V;` or `|-> #same(CELL2) when …`".to_owned()
    };
    if ts.len() < 4 || !symbol(&ts[0], Symbol::Hash) || !symbol(&ts[2], Symbol::Colon) {
        return Err(shape());
    }
    let binder = word(&ts[1]).ok_or_else(shape)?.to_owned();
    if matches!(
        binder.as_str(),
        "result"
            | "outer"
            | "loaded"
            | "holds"
            | "absent"
            | "registered"
            | "fresh"
            | "same"
            | "pointee"
            | "cell"
            | "untouched"
    ) {
        return Err("quant-cell-binder: a reserved `#` word is no binder".into());
    }
    let maps = ts
        .iter()
        .position(|t| symbol(t, Symbol::MapsTo))
        .ok_or_else(shape)?;
    let record = word(ts.get(3).ok_or_else(shape)?)
        .ok_or_else(shape)?
        .to_owned();
    if !ts.get(4).is_some_and(|t| symbol(t, Symbol::Dot)) || maps <= 5 {
        return Err(shape());
    }
    let path = cell_path(&ts[5..maps])?;
    let when = ts
        .iter()
        .position(|t| word(t) == Some("when"))
        .ok_or_else(|| "quant-cell-guard: a protocol cell is stated under its guard".to_owned())?;
    let value = &ts[maps + 1..when];
    let same = match value {
        [
            TokenKind::Symbol(Symbol::Hash),
            TokenKind::Word(fresh),
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(kind),
            TokenKind::Symbol(Symbol::RightParen),
        ] if fresh == "fresh" => {
            if kind != "descriptor" {
                return Err("quant-cell-kind: a protocol cell is a `descriptor`".into());
            }
            None
        }
        [
            TokenKind::Symbol(Symbol::Hash),
            TokenKind::Word(same),
            TokenKind::Symbol(Symbol::LeftParen),
            inner @ ..,
            TokenKind::Symbol(Symbol::RightParen),
        ] if same == "same" => Some(cell_path(inner)?),
        _ => {
            return Err(
                "quant-cell-value: the value is `#fresh(descriptor)` or `#same(CELL)`".into(),
            );
        }
    };
    let (guard, sets) = cell_sets(&ts[when + 1..])?;
    let op_at = guard
        .iter()
        .position(|t| {
            matches!(
                t,
                TokenKind::Symbol(Symbol::NotEqual | Symbol::EqualEqual | Symbol::Greater)
            )
        })
        .ok_or_else(|| {
            "quant-cell-guard: write `when #B.GUARD != -1`, `> 0` or `== 1`".to_owned()
        })?;
    if guard.len() < 4
        || !symbol(&guard[0], Symbol::Hash)
        || word(&guard[1]) != Some(binder.as_str())
        || !symbol(&guard[2], Symbol::Dot)
    {
        return Err("quant-cell-guard: the guard is a member of the clause's own binder".into());
    }
    let guard_path = cell_path(&guard[3..op_at])?;
    let op = match &guard[op_at] {
        TokenKind::Symbol(Symbol::NotEqual) => "!=",
        TokenKind::Symbol(Symbol::EqualEqual) => "==",
        _ => ">",
    };
    let literal = match &guard[op_at + 1..] {
        [TokenKind::Integer(digits)] => digits.parse::<i64>().ok(),
        [TokenKind::Symbol(Symbol::Minus), TokenKind::Integer(digits)] => {
            digits.parse::<i64>().ok().map(|v| -v)
        }
        _ => None,
    };
    let op = literal
        .and_then(|value| ProtocolGuard::from_spelling(op, value))
        .ok_or_else(|| {
            "quant-cell-guard: the admitted guards are `!= -1`, `> 0` and `== 1`".to_owned()
        })?;
    if (sets.0.is_some() || sets.1.is_some()) && (same.is_some() || guard_path != path) {
        return Err(
            "quant-cell-set: `live_set` / `absent_set` stand on a per-end sentinel's own `#fresh` clause"
                .into(),
        );
    }
    Ok((binder, record, path, same, guard_path, op, sets))
}

/// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (grammar G05k): whether a `quant
/// cell` clause is a callee frame, `quant cell CALLEE: TAG |-> #untouched;`,
/// rather than a protocol cell, which opens with its `#` binder.
fn is_frame(source: &str) -> Result<bool, String> {
    Ok(tokens(source)?.first().and_then(word).is_some())
}

/// Read one `quant cell CALLEE: TAG |-> #untouched;` (card
/// CELL-PROTOCOL-RELEASE-WINDOW-FRAMES, grammar G05k; USER DECISION
/// 2026-11-04, verbatim: "(a) Callee frames"). Total or reject.
fn frame(source: &str) -> Result<qcore_schemas::descriptor_cell::CellFrame, String> {
    let ts = tokens(source)?;
    match ts.as_slice() {
        [
            TokenKind::Word(callee),
            TokenKind::Symbol(Symbol::Colon),
            TokenKind::Word(record),
            TokenKind::Symbol(Symbol::MapsTo),
            TokenKind::Symbol(Symbol::Hash),
            TokenKind::Word(untouched),
        ] if untouched == "untouched" => Ok(qcore_schemas::descriptor_cell::CellFrame {
            callee: callee.clone(),
            record: record.clone(),
        }),
        _ => Err("quant-frame-shape: write `quant cell CALLEE: TAG |-> #untouched;`".to_owned()),
    }
}

/// A declaration's callee frames, sorted (G05k): each names a record whose
/// protocol the same declaration states (`quant-frame-record`), and each
/// (callee, record) pair is stated once (`quant-frame-duplicate`).
fn cell_frames(
    frames: Vec<qcore_schemas::descriptor_cell::CellFrame>,
    protocols: &[qcore_schemas::descriptor_cell::CellProtocol],
) -> Result<Vec<qcore_schemas::descriptor_cell::CellFrame>, String> {
    let mut seen = BTreeSet::new();
    for frame in &frames {
        if !protocols
            .iter()
            .any(|protocol| protocol.record == frame.record)
        {
            return Err(format!(
                "quant-frame-record: `{}` states no protocol in this declaration",
                frame.record
            ));
        }
        if !seen.insert(frame.clone()) {
            return Err("quant-frame-duplicate: one frame per callee and record".into());
        }
    }
    Ok(seen.into_iter().collect())
}

/// Card CELL-RESULT-SET-ALIGNMENT (USER DECISION 2026-11-04, Q2 (i)): the
/// loader's refusals over a per-end sentinel's declared sets. An absent set
/// that meets the guard's live edge would read a never-created value as a
/// live descriptor (a `> 0` guard over a stored 1); a live set that meets the
/// absent set leaves no test exact and is refused too.
fn check_cell_sets(
    op: qcore_schemas::descriptor_cell::ProtocolGuard,
    sets: &CellSets,
) -> Result<(), String> {
    let meets = |set: &[(i64, i64)], f: &dyn Fn(i64) -> bool| {
        set.iter().any(|(lo, hi)| {
            [*lo, *hi, -1, 0, 1]
                .into_iter()
                .any(|v| *lo <= v && v <= *hi && f(v))
        })
    };
    if let Some(absent) = &sets.1
        && meets(absent, &|v| op.says_live(v))
    {
        return Err(format!(
            "quant-cell-absent-set: the absent set meets the guard's live edge (`{}`)",
            op.spelling()
        ));
    }
    if let (Some(live), Some(absent)) = &sets
        && live
            .iter()
            .any(|(lo, hi)| absent.iter().any(|(a, b)| lo.max(a) <= hi.min(b)))
    {
        return Err("quant-cell-live-set: the live set meets the absent set".into());
    }
    Ok(())
}

/// Group a declaration's `quant cell` clauses into one protocol per guard of
/// each record, rejecting every disagreement (G05i).
///
/// Card CELL-PROTOCOL-PER-END-GUARDS (USER DECISION 2026-11-03, verbatim:
/// "Per-end sentinels"): a record's clauses are grouped by guard. Besides the
/// one record guard G05i had (a flag, or a sentinel over several `#fresh`
/// cells), each `#fresh` cell may carry its own sentinel (`when #B.CELL > 0`
/// or `!= -1` on the clause's own cell), and a `#same` cell takes its
/// target's guard. A record keeps at most one guard that is not such a
/// per-end sentinel. One protocol is emitted per guard, sorted.
fn cell_protocols(
    clauses: Vec<CellClause>,
) -> Result<Vec<qcore_schemas::descriptor_cell::CellProtocol>, String> {
    use qcore_schemas::descriptor_cell::{CellProtocol, ProtocolCell};
    type Groups = BTreeMap<Vec<String>, CellProtocol>;
    let mut records: BTreeMap<String, (String, Groups)> = BTreeMap::new();
    for (binder, record, path, same, guard, op, sets) in clauses {
        check_cell_sets(op, &sets)?;
        let entry = records
            .entry(record.clone())
            .or_insert_with(|| (binder.clone(), BTreeMap::new()));
        if entry.0 != binder {
            return Err("quant-cell-binder: one binder per record".into());
        }
        if entry
            .1
            .values()
            .any(|group| group.cells.iter().any(|cell| cell.path == path))
        {
            return Err("quant-cell-duplicate: one state per cell".into());
        }
        let group = entry
            .1
            .entry(guard.clone())
            .or_insert_with(|| CellProtocol {
                record: record.clone(),
                cells: Vec::new(),
                guard: guard.clone(),
                op,
                live_set: None,
                absent_set: None,
            });
        if group.op != op {
            return Err("quant-cell-guard: every cell of a record states the same guard".into());
        }
        if sets.0.is_some() {
            group.live_set = sets.0;
        }
        if sets.1.is_some() {
            group.absent_set = sets.1;
        }
        group.cells.push(ProtocolCell { path, same });
    }
    let mut out = Vec::new();
    for (_, (_, groups)) in records {
        let mut record_guards = 0usize;
        let cells_of_record = groups
            .values()
            .flat_map(|group| group.cells.iter().map(|cell| cell.path.clone()))
            .collect::<Vec<_>>();
        for (_, mut protocol) in groups {
            let fresh = |path: &Vec<String>| {
                protocol
                    .cells
                    .iter()
                    .any(|cell| &cell.path == path && cell.same.is_none())
            };
            if !protocol.cells.iter().any(|cell| cell.same.is_none()) {
                return Err(
                    "quant-cell-fresh: a protocol states at least one `#fresh` cell".into(),
                );
            }
            if protocol
                .cells
                .iter()
                .any(|cell| cell.same.as_ref().is_some_and(|target| !fresh(target)))
            {
                return Err("quant-cell-same: `#same` names a `#fresh` cell of its record".into());
            }
            let guard_is_fresh = fresh(&protocol.guard);
            if protocol.op.is_sentinel() && !guard_is_fresh {
                return Err("quant-cell-guard: a `!= -1` or `> 0` guard is a `#fresh` cell".into());
            }
            if !protocol.op.is_sentinel() && cells_of_record.contains(&protocol.guard) {
                return Err("quant-cell-guard: a `== 1` guard is a flag, no cell".into());
            }
            if !protocol.per_end() {
                record_guards += 1;
                if protocol.live_set.is_some() || protocol.absent_set.is_some() {
                    return Err(
                        "quant-cell-set: `live_set` / `absent_set` stand on a per-end sentinel"
                            .into(),
                    );
                }
            }
            protocol.cells.sort();
            out.push(protocol);
        }
        if record_guards > 1 {
            return Err("quant-cell-guard: a record states one record guard; every other guard is its own cell's sentinel".into());
        }
    }
    Ok(out)
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
    let mut cell_clauses = Vec::new();
    let mut frame_clauses = Vec::new();
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
            SourceContractClauseKind::QuantCell => {
                // Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (G05k): a clause
                // that opens with a name, not a `#` binder, is a callee frame.
                if is_frame(&c.value.expression_source)? {
                    frame_clauses.push(frame(&c.value.expression_source)?);
                } else {
                    cell_clauses.push(cell(&c.value.expression_source)?);
                }
            }
            _ => {}
        }
    }
    out.cells = cell_protocols(cell_clauses)?;
    out.frames = cell_frames(frame_clauses, &out.cells)?;
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
                    SourceContractClauseKind::QuantCondition(_) => {
                        return Err(
                            "quant conditions have no interpreted local-condition valuation".into(),
                        );
                    }
                    SourceContractClauseKind::QuantConditionBinding if keep_local => "quant",
                    SourceContractClauseKind::QuantConditionBlock { negated, .. } if keep_local => {
                        if *negated {
                            "quant not"
                        } else {
                            "quant"
                        }
                    }
                    SourceContractClauseKind::QuantRegistered if keep_local => "quant registered",
                    SourceContractClauseKind::QuantCell if keep_local => "quant cell",
                    SourceContractClauseKind::QuantConditionBinding
                    | SourceContractClauseKind::QuantConditionBlock { .. }
                    | SourceContractClauseKind::QuantRegistered
                    | SourceContractClauseKind::QuantCell => continue,
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

/// Card MK-EVENT-CELL-CROSS-FUNCTION (grammar G05i): the `quant cell` reader,
/// total or reject.
#[cfg(test)]
mod cell_protocol_tests {
    use std::collections::BTreeMap;

    use qcore_schemas::descriptor_cell::ProtocolGuard;

    fn host(clauses: &str) -> String {
        format!(
            "module contract;\nextern fn host(c: ptr<mut, u8>) -> unit\ncontract {{ requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none();\n{clauses} }};"
        )
    }

    fn load(
        modules: &[(&str, String)],
    ) -> Result<qcore_schemas::contract::ResourceContracts, String> {
        crate::contract_loader::load_resource_contract_sources(
            &modules
                .iter()
                .map(|(name, text)| ((*name).to_owned(), text.clone()))
                .collect::<BTreeMap<_, _>>(),
        )
        .map_err(|e| format!("{e:?}"))
    }

    const SCHED: &str = "quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] != -1;\n\
         quant cell #s: flb_sched.ch_events[1] |-> #fresh(descriptor) when #s.ch_events[0] != -1;\n\
         quant cell #s: flb_sched.event.fd |-> #same(ch_events[0]) when #s.ch_events[0] != -1;";

    #[test]
    fn the_decided_candidate_loads_to_one_protocol() {
        let env = load(&[("a.qv", host(SCHED))]).expect("loads");
        let protocols = env.cell_protocols();
        let (hosts, protocols) = &protocols["flb_sched"];
        let [protocol] = protocols.as_slice() else {
            panic!("one guard: {protocols:?}");
        };
        assert_eq!(hosts.iter().collect::<Vec<_>>(), ["host"]);
        assert_eq!(protocol.op, ProtocolGuard::NotMinusOne);
        assert_eq!(protocol.guard, ["ch_events[0]"]);
        assert_eq!(protocol.cells.len(), 3);
        assert_eq!(
            protocol.group(&["event".into(), "fd".into()]),
            Some(&["ch_events[0]".to_owned()][..])
        );
        // The host is withheld, as a `quant registered` host is.
        assert!(env.contract("host").is_none());
    }

    #[test]
    fn every_other_spelling_rejects_the_module() {
        for clauses in [
            // a guard outside the three pairs
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] != 0;",
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] >= 0;",
            // no guard
            "quant cell #s: t.a[0] |-> #fresh(descriptor);",
            // another kind
            "quant cell #s: t.a[0] |-> #fresh(heap) when #s.a[0] != -1;",
            // a sentinel guard that is no `#fresh` cell
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.b != -1;",
            // a flag guard that is a cell
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] == 1;",
            // `#same` of no `#fresh` cell
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] != -1;\nquant cell #s: t.m |-> #same(b) when #s.a[0] != -1;",
            // only `#same`
            "quant cell #s: t.m |-> #same(m) when #s.on == 1;",
            // one guard stated two ways
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] != -1;\nquant cell #s: t.a[1] |-> #fresh(descriptor) when #s.a[0] > 0;",
            // card CELL-PROTOCOL-PER-END-GUARDS: two record guards (two
            // guards over several cells, or a flag beside one)
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] != -1;\nquant cell #s: t.a[1] |-> #fresh(descriptor) when #s.a[0] != -1;\nquant cell #s: t.b[0] |-> #fresh(descriptor) when #s.b[0] > 0;\nquant cell #s: t.b[1] |-> #fresh(descriptor) when #s.b[0] > 0;",
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.on == 1;\nquant cell #s: t.b[0] |-> #fresh(descriptor) when #s.a[0] > 0;",
            "quant cell #s: t.n[0] |-> #fresh(descriptor) when #s.on == 1;\nquant cell #s: t.m[0] |-> #fresh(descriptor) when #s.off == 1;",
            // a `#same` under another end's guard
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] > 0;\nquant cell #s: t.a[1] |-> #fresh(descriptor) when #s.a[1] > 0;\nquant cell #s: t.m |-> #same(a[0]) when #s.a[1] > 0;",
            // a per-end guard naming another binder's cell
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] > 0;\nquant cell #r: t.a[1] |-> #fresh(descriptor) when #r.a[1] > 0;",
            // a duplicate cell
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] != -1;\nquant cell #s: t.a[0] |-> #fresh(descriptor) when #s.a[0] != -1;",
            // a non-canonical index, a guard of another binder, a reserved binder
            "quant cell #s: t.a[01] |-> #fresh(descriptor) when #s.a[01] != -1;",
            "quant cell #s: t.a[0] |-> #fresh(descriptor) when #r.a[0] != -1;",
            "quant cell #result: t.a[0] |-> #fresh(descriptor) when #result.a[0] != -1;",
        ] {
            assert!(load(&[("a.qv", host(clauses))]).is_err(), "{clauses}");
        }
    }

    /// Card CELL-PROTOCOL-PER-END-GUARDS (USER DECISION 2026-11-03,
    /// verbatim: "Per-end sentinels"): each end its own sentinel, a `#same`
    /// cell under its target's, beside one record guard.
    #[test]
    fn per_end_sentinels_load_one_protocol_per_guard() {
        const PER_END: &str = "quant cell #t: thi.ch[0] |-> #fresh(descriptor) when #t.ch[0] > 0;\n\
             quant cell #t: thi.ch[1] |-> #fresh(descriptor) when #t.ch[1] > 0;\n\
             quant cell #t: thi.event.fd |-> #same(ch[0]) when #t.ch[0] > 0;\n\
             quant cell #t: thi.q[0] |-> #fresh(descriptor) when #t.q[0] != -1;\n\
             quant cell #t: thi.n[0] |-> #fresh(descriptor) when #t.on == 1;\n\
             quant cell #t: thi.n[1] |-> #fresh(descriptor) when #t.on == 1;";
        let env = load(&[("a.qv", host(PER_END))]).expect("loads");
        let protocols = env.cell_protocols();
        let (_, protocols) = &protocols["thi"];
        let guards = protocols
            .iter()
            .map(|p| (p.guard.join("."), p.op, p.cells.len(), p.per_end()))
            .collect::<Vec<_>>();
        assert_eq!(
            guards,
            [
                ("ch[0]".to_owned(), ProtocolGuard::Positive, 2, true),
                ("ch[1]".to_owned(), ProtocolGuard::Positive, 1, true),
                ("on".to_owned(), ProtocolGuard::EqualOne, 2, false),
                ("q[0]".to_owned(), ProtocolGuard::NotMinusOne, 1, true),
            ]
        );
        // A second declaration stating only some of the guards disagrees.
        let part = host("quant cell #t: thi.ch[0] |-> #fresh(descriptor) when #t.ch[0] > 0;")
            .replace("host(", "other(");
        let env = load(&[("a.qv", host(PER_END)), ("b.qv", part)]).expect("loads");
        assert!(!env.cell_protocols().contains_key("thi"));
    }

    #[test]
    fn two_declarations_that_disagree_leave_the_record_without_a_protocol() {
        let other = host(
            "quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] > 0;",
        )
        .replace("host(", "other(");
        let env = load(&[("a.qv", host(SCHED)), ("b.qv", other)]).expect("loads");
        assert!(!env.cell_protocols().contains_key("flb_sched"));
        let same = host(SCHED).replace("host(", "again(");
        let env = load(&[("a.qv", host(SCHED)), ("b.qv", same)]).expect("loads");
        assert_eq!(env.cell_protocols()["flb_sched"].0.len(), 2);
    }

    /// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (grammar G05k; USER DECISION
    /// 2026-11-04, verbatim: "(a) Callee frames"): a callee frame stands
    /// beside its record's protocol and loads to the record's frame set.
    #[test]
    fn callee_frames_load_beside_their_protocol() {
        let clauses = format!(
            "{SCHED}\nquant cell mk_list_del: flb_sched |-> #untouched;\n\
             quant cell flb_tp_destroy: flb_sched |-> #untouched;"
        );
        let env = load(&[("a.qv", host(&clauses))]).expect("loads");
        let frames = env.cell_frames();
        assert_eq!(
            frames["flb_sched"].iter().collect::<Vec<_>>(),
            ["flb_tp_destroy", "mk_list_del"]
        );
        assert!(!env.cell_frames_accepted());
        let resolved = env.with_human_holds_in_force().expect("frames are held");
        assert!(resolved.cell_frames_accepted());
        assert!(resolved.with_human_holds_in_force().is_none());
        // Without a frame nothing is held for it.
        let env = load(&[("a.qv", host(SCHED))]).expect("loads");
        assert!(env.cell_frames().is_empty());
        assert!(env.with_human_holds_in_force().is_none());
    }

    #[test]
    fn every_other_frame_spelling_rejects_the_module() {
        for frame in [
            // no protocol of the record in this declaration
            "quant cell mk_list_del: other |-> #untouched;",
            // a duplicate
            "quant cell mk_list_del: flb_sched |-> #untouched;\nquant cell mk_list_del: flb_sched |-> #untouched;",
            // another value, a member path, a missing value, a binder
            "quant cell mk_list_del: flb_sched |-> #fresh(descriptor);",
            "quant cell mk_list_del: flb_sched.ch_events[0] |-> #untouched;",
            "quant cell mk_list_del: flb_sched;",
            "quant cell mk_list_del: flb_sched |-> #untouched when #s.ch_events[0] != -1;",
            "quant cell #untouched: flb_sched.ch_events[1] |-> #fresh(descriptor) when #untouched.ch_events[1] > 0;",
        ] {
            let clauses = format!("{SCHED}\n{frame}");
            assert!(load(&[("a.qv", host(&clauses))]).is_err(), "{frame}");
        }
    }

    /// Two declarations stating one record's protocol with different frames
    /// leave the record without frames (its protocol stays).
    #[test]
    fn two_declarations_that_disagree_leave_the_record_without_frames() {
        let framed = format!("{SCHED}\nquant cell mk_list_del: flb_sched |-> #untouched;");
        let other = host(SCHED).replace("host(", "other(");
        let env = load(&[("a.qv", host(&framed)), ("b.qv", other)]).expect("loads");
        assert!(env.cell_protocols().contains_key("flb_sched"));
        assert!(env.cell_frames().is_empty());
        let same = host(&framed).replace("host(", "again(");
        let env = load(&[("a.qv", host(&framed)), ("b.qv", same)]).expect("loads");
        assert_eq!(env.cell_frames()["flb_sched"].len(), 1);
    }
}
