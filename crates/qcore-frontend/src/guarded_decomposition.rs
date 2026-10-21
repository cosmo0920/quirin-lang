//! Standalone versioned readable syntax, deliberately separate from flow-v1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clause {
    Enable {
        environment: String,
        prerequisites: Vec<String>,
        operation: String,
    },
    Forbid {
        condition: String,
        operation: String,
    },
    Alternatives {
        operation: String,
        cases: Vec<String>,
    },
    Terminal {
        case: String,
        postcondition: String,
    },
}

fn identifier(s: &str) -> Result<String, String> {
    if s.len() > 128 || !crate::premise_flow::identifier(s) {
        Err("guarded-decomposition: malformed or oversized symbol".into())
    } else {
        Ok(s.into())
    }
}

/// Parse a whole bounded document, never a partial prefix or an ordinary source
/// expression. Atomic operator bytes cannot be split by whitespace.
pub fn parse(source: &str) -> Result<Vec<Clause>, String> {
    if source.len() > 65536 {
        return Err("guarded-decomposition: document byte limit".into());
    }
    let mut statements = source.split(';');
    if statements.next().map(str::trim) != Some("guarded_decomposition_v1") {
        return Err("guarded-decomposition: expected exact version1 header".into());
    }
    let mut clauses = Vec::new();
    let mut tail_seen = false;
    for statement in statements {
        let s = statement.trim();
        if s.is_empty() {
            if tail_seen {
                return Err("guarded-decomposition: empty statement".into());
            }
            tail_seen = true;
            continue;
        }
        if tail_seen {
            return Err("guarded-decomposition: empty statement".into());
        }
        if s.len() > 4096 {
            return Err("guarded-decomposition: statement byte limit".into());
        }
        let choices = ["|->", "!->", "||->", "->|"];
        let hits: Vec<_> = choices
            .iter()
            .filter_map(|op| {
                // Check the longest operator first, before its |-> suffix.
                s.find(op).map(|at| (*op, at))
            })
            .collect();
        let (op, at) = if let Some(at) = s.find("||->") {
            ("||->", at)
        } else if hits.len() == 1 {
            hits[0]
        } else {
            return Err("guarded-decomposition: expected one supported operator".into());
        };
        let left = s[..at].trim();
        let right = s[at + op.len()..].trim();
        let clause = match op {
            "|->" => {
                let Some(inner) = left.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
                    return Err("guarded-decomposition: parenthesized environment and prerequisites required".into());
                };
                let terms = inner
                    .split('∧')
                    .map(str::trim)
                    .map(identifier)
                    .collect::<Result<Vec<_>, _>>()?;
                if !(2..=33).contains(&terms.len()) {
                    return Err(
                        "guarded-decomposition: one environment and 1..32 prerequisites required"
                            .into(),
                    );
                }
                Clause::Enable {
                    environment: terms[0].clone(),
                    prerequisites: terms[1..].to_vec(),
                    operation: identifier(right)?,
                }
            }
            "!->" => Clause::Forbid {
                condition: identifier(left)?,
                operation: identifier(right)?,
            },
            "||->" => {
                let cases = right
                    .split('/')
                    .map(str::trim)
                    .map(identifier)
                    .collect::<Result<Vec<_>, _>>()?;
                if !(2..=32).contains(&cases.len()) {
                    return Err(
                        "guarded-decomposition: 2..32 explicit alternatives required".into(),
                    );
                }
                Clause::Alternatives {
                    operation: identifier(left)?,
                    cases,
                }
            }
            "->|" => Clause::Terminal {
                case: identifier(left)?,
                postcondition: identifier(right)?,
            },
            _ => unreachable!(),
        };
        clauses.push(clause);
        if clauses.len() > 256 {
            return Err("guarded-decomposition: clause limit".into());
        }
    }
    if !tail_seen || clauses.is_empty() {
        return Err(
            "guarded-decomposition: nonempty document with final semicolon required".into(),
        );
    }
    Ok(clauses)
}
