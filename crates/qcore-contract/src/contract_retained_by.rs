//! Card **CALLEE-RETENTION-CLASS1**, stage (b): the retention clauses of
//! resource vocabulary version 1 (contract specification §C04.14, grammar
//! G05), read total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_retained_by(p, h);   // h: a parameter, or `global`
//! ensures normal resource_holder_reset(h);
//! ```
//!
//! `p` names a depth-one pointer parameter whose mode is `resource_borrowed`
//! (the caller keeps custody); `h` names a distinct depth-one pointer
//! parameter whose mode is `resource_borrowed`, or is the word `global`
//! (storage of static duration, or a holder no caller object denotes). A
//! parameter is retained at most once; a reset names a depth-one pointer
//! parameter, `resource_borrowed`, at most once. No kind stands on either
//! position. A declaration carrying either clause is withheld from the
//! function map (the loader, [`super::load_resource_contract_sources`]).

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceFunctionContract, ResourceKind, ResourceParameterMode};
use qcore_schemas::retained_by::{
    GuardRelation, RetainingContract, Retention, RetentionGuard, RetentionHolder,
};

/// `resource_retained_by(p, h)`.
pub(super) const RETAINED_BY: &str = "resource_retained_by";

/// `resource_holder_reset(h)`.
pub(super) const HOLDER_RESET: &str = "resource_holder_reset";

/// The holder word for static storage.
pub(super) const GLOBAL: &str = "global";

/// The depth-one pointer types a retained pointer or a holder may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// Card **W43-ABI-ENUM-AND-CONST-PP** (contract spec §C04.14.1, grammar
/// G05, gateway rule 12w): a guard as written, `X OP K`, before the
/// declaration's parameters are known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenGuard {
    pub(super) parameter: String,
    pub(super) relation: GuardRelation,
    pub(super) constant: i128,
}

/// The guard introducer.
pub(super) const WHEN: &str = "when";

/// The integer scalar spellings a guard parameter may have, each with its
/// value range (`isize`/`usize` at the 64-bit width the loader's LP64 reading
/// gives them).
const GUARD_TYPES: [(&str, i128, i128); 10] = [
    ("i8", -(1 << 7), (1 << 7) - 1),
    ("i16", -(1 << 15), (1 << 15) - 1),
    ("i32", -(1 << 31), (1 << 31) - 1),
    ("i64", -(1 << 63), (1 << 63) - 1),
    ("isize", -(1 << 63), (1 << 63) - 1),
    ("u8", 0, (1 << 8) - 1),
    ("u16", 0, (1 << 16) - 1),
    ("u32", 0, (1 << 32) - 1),
    ("u64", 0, (1 << 64) - 1),
    ("usize", 0, (1 << 64) - 1),
];

/// The value range of a guard parameter's declared spelling, `None` for a
/// spelling no guard may name (a pointer, `bool`, a float, `fnptr`,
/// `va_list`).
#[must_use]
pub fn guard_range(spelling: &str) -> Option<(i128, i128)> {
    GUARD_TYPES
        .iter()
        .find(|(name, _, _)| *name == spelling)
        .map(|(_, low, high)| (*low, *high))
}

/// Split `CLAUSE when GUARD`. `Ok(None)` when the expression carries no
/// guard; every other use of the word `when` rejects (total-or-reject).
pub(super) fn split_guard(expression: &str) -> Result<Option<(&str, WrittenGuard)>, String> {
    let words = expression
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|word| *word == WHEN)
        .count();
    if words == 0 {
        return Ok(None);
    }
    let unsupported = || format!("unsupported guard `{expression}`");
    if words > 1 {
        return Err(unsupported());
    }
    let close = expression.rfind(')').ok_or_else(unsupported)?;
    let (clause, rest) = expression.split_at(close + 1);
    let guard = rest
        .trim_start()
        .strip_prefix(WHEN)
        .filter(|guard| guard.starts_with(char::is_whitespace))
        .ok_or_else(unsupported)?;
    Ok(Some((
        clause.trim(),
        parse_guard(guard).ok_or_else(unsupported)?,
    )))
}

/// `X OP K`: an identifier, one of `==` `<` `>` `<=` `>=`, and a canonical
/// decimal literal (an optional leading `-`, no leading zero, no `-0`, no
/// suffix: C04.5's spelling), with blanks between them and nothing else.
fn parse_guard(text: &str) -> Option<WrittenGuard> {
    let text = text.trim();
    let name_end = text
        .find(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .unwrap_or(text.len());
    let (parameter, rest) = text.split_at(name_end);
    if parameter.is_empty() || parameter.starts_with(|ch: char| ch.is_ascii_digit()) {
        return None;
    }
    let rest = rest.trim_start();
    let operator_end = rest
        .find(|ch: char| !matches!(ch, '=' | '<' | '>' | '!'))
        .unwrap_or(rest.len());
    let (operator, literal) = rest.split_at(operator_end);
    let relation = GuardRelation::from_spelling(operator)?;
    let literal = literal.trim();
    let digits = literal.strip_prefix('-').unwrap_or(literal);
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || literal == "-0"
    {
        return None;
    }
    Some(WrittenGuard {
        parameter: parameter.to_owned(),
        relation,
        constant: literal.parse().ok()?,
    })
}

/// Whether the guard splits `[low, high]` into two non-empty parts: some
/// value satisfies it and some value does not. A guard that holds of every
/// value of its parameter's type (or of none) does not partition, and
/// rejects.
#[must_use]
pub fn guard_partitions(relation: GuardRelation, constant: i128, low: i128, high: i128) -> bool {
    (low..=high).contains(&constant)
        && match relation {
            GuardRelation::Eq => low < high,
            GuardRelation::Lt | GuardRelation::Ge => low < constant,
            GuardRelation::Gt | GuardRelation::Le => constant < high,
        }
}

/// Validate the retention clauses of one declaration whose custody contract
/// `contract` is already loaded.
pub(super) fn declaration(
    retentions: &[(String, String, Option<WrittenGuard>)],
    resets: &[String],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    parameter_kinds: &[Option<ResourceKind>],
    names: &BTreeMap<&str, usize>,
) -> Result<RetainingContract, String> {
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    if names.contains_key(GLOBAL) {
        return Err(format!(
            "a retention clause reads `{GLOBAL}` as static storage: rename the parameter \
             `{GLOBAL}`"
        ));
    }
    let position = |name: &str, role: &str| -> Result<usize, String> {
        let at = index(name)?;
        if !DEPTH_ONE.contains(&parameter_types[at].as_str()) {
            return Err(format!(
                "a {role} names a depth-one pointer parameter, not `{name}`"
            ));
        }
        if contract.parameters[at] != ResourceParameterMode::Borrowed {
            return Err(format!(
                "a {role} names a `resource_borrowed` parameter: the caller keeps custody of \
                 `{name}`"
            ));
        }
        if parameter_kinds[at].is_some() {
            return Err(format!("a {role} carries no kind (`{name}`)"));
        }
        Ok(at)
    };
    let mut declared = Vec::with_capacity(retentions.len());
    for (retained, holder, guard) in retentions {
        let retained = position(retained, "retained pointer")?;
        if declared
            .iter()
            .any(|existing: &Retention| existing.retained == retained)
        {
            return Err(format!(
                "duplicate retention of {}",
                names
                    .iter()
                    .find(|(_, at)| **at == retained)
                    .map_or("?", |(name, _)| *name)
            ));
        }
        let holder = if holder == GLOBAL {
            RetentionHolder::Global
        } else {
            let at = position(holder, "holder")?;
            if at == retained {
                return Err(format!("a pointer does not retain itself (`{holder}`)"));
            }
            RetentionHolder::Parameter { index: at }
        };
        // Card W43-ABI-ENUM-AND-CONST-PP (§C04.14.1): a guard names an
        // integer parameter other than the two the clause names, its constant
        // lies in that parameter's range, and it partitions the range.
        let guard = match guard {
            None => None,
            Some(written) => {
                let at = index(&written.parameter)?;
                let (low, high) = guard_range(&parameter_types[at]).ok_or_else(|| {
                    format!(
                        "a guard names an integer parameter, not `{}`",
                        written.parameter
                    )
                })?;
                if at == retained
                    || matches!(holder, RetentionHolder::Parameter { index } if index == at)
                {
                    return Err(format!(
                        "a guard names a parameter its clause does not (`{}`)",
                        written.parameter
                    ));
                }
                if !guard_partitions(written.relation, written.constant, low, high) {
                    return Err(format!(
                        "a guard constant outside `{}`'s range, or a guard that holds of every \
                         value or of none, does not partition it",
                        written.parameter
                    ));
                }
                Some(RetentionGuard {
                    parameter: at,
                    relation: written.relation,
                    constant: written.constant,
                    low,
                    high,
                })
            }
        };
        declared.push(Retention {
            retained,
            holder,
            guard,
        });
    }
    let mut reset = Vec::with_capacity(resets.len());
    for holder in resets {
        let at = position(holder, "reset holder")?;
        if reset.contains(&at) {
            return Err(format!("duplicate holder reset of {holder}"));
        }
        reset.push(at);
    }
    Ok(RetainingContract {
        contract: contract.clone(),
        retentions: declared,
        resets: reset,
    })
}
