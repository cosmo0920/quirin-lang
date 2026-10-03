//! Card **GATEWAY-ANALYSIS-PATH-OPTIMIZATION**: the statement and return
//! fallback orders, written down as tables.
//!
//! Waves 43-48 added each fallback as "asked last, only for a refusal" at the
//! end of `Parser::expression_statement` and of the `return` arm of
//! `Parser::parse_statement`, so the order of consultation was the order of
//! the code. The two tables below are that order, entry for entry, and the
//! dispatchers ask them first to last and stop at the first answer: no entry
//! was added, removed or moved (the card's identity runs are the evidence,
//! `docs/reports/gateway-analysis-path-optimization/`). Each entry keeps its
//! own entry condition (which refusal it reads) inside its `ask`; the table
//! adds no condition of its own. A new fallback is a new row, in the position
//! its rule states, not a new `if` at the end of the parser.
//!
//! The routes asked *before* classification (11o boundary, 11h static
//! declaration, 11b pointer test) and the ones that rewrite the classified
//! instruction (12h actuals, 11o no-return exit) stay inline in
//! `expression_statement`: they are not fallbacks of a refusal. The full
//! consultation order of every position is the card's route map
//! (`ROUTE-MAP.md`).

use super::{LegacyInstruction, Parser, Statement, Token, atomic_route, return_refused};

/// One statement's question to the statement fallbacks.
pub(super) struct StatementQuery<'q> {
    pub(super) tokens: &'q [Token],
    pub(super) line: usize,
    pub(super) origin: Option<usize>,
    pub(super) instruction: &'q LegacyInstruction,
}

/// One statement fallback: its rule (or card, where no rule is named) and
/// whether an answer clears the statement's pending element origins (card
/// W45-CONTAINER-ROOT-REACH: an answer that replaces the instruction keeps no
/// element binding of the refused one).
pub(super) struct StatementFallback {
    /// Read by the order test and the `route-census` stream.
    #[cfg_attr(not(any(test, feature = "route-census")), allow(dead_code))]
    pub(super) rule: &'static str,
    clears_origins: bool,
    ask: for<'p, 'q> fn(&mut Parser<'p>, &StatementQuery<'q>) -> Option<Statement>,
}

fn block(line: usize, instructions: Vec<LegacyInstruction>) -> Statement {
    Statement::Block(
        instructions
            .into_iter()
            .map(|instruction| Statement::Instruction(instruction, line))
            .collect(),
    )
}

fn refusal(instruction: &LegacyInstruction) -> Option<&str> {
    match instruction {
        LegacyInstruction::Unknown { description, .. } => Some(description),
        _ => None,
    }
}

/// The statement fallbacks of `expression_statement`, in consultation order.
pub(super) const STATEMENT_FALLBACKS: &[StatementFallback] = &[
    // W43-INDEX-TRAVERSAL: a stepped access of a refused statement.
    StatementFallback {
        rule: "W43-INDEX-TRAVERSAL",
        clears_origins: false,
        ask: |parser, query| {
            parser
                .stepped_access(query.tokens, query.instruction)
                .map(|sequence| block(query.line, sequence))
        },
    },
    // W44-INDIRECT-CALL-ARGUMENT-ACCESS: a call whose actuals the gate
    // refused, lowered to its actuals' reads and the call.
    StatementFallback {
        rule: "W44-INDIRECT-CALL-ARGUMENT-ACCESS",
        clears_origins: false,
        ask: |parser, query| {
            parser
                .call_argument_access(query.tokens, query.instruction)
                .map(|sequence| block(query.line, sequence))
        },
    },
    // CG02-ELABORATION-CORE: only for a refusal.
    StatementFallback {
        rule: "10s",
        clears_origins: true,
        ask: |parser, query| {
            let description = refusal(query.instruction)?;
            parser.elaborated_statement(query.tokens, query.line, description)
        },
    },
    // CG08-CASTS-AND-TYPE-QUERIES (E15): after the elaborator.
    StatementFallback {
        rule: "11b",
        clears_origins: true,
        ask: |parser, query| {
            parser.uncast_pointer_statement(query.tokens, query.line, query.instruction)
        },
    },
    // CG03-CALL-SUMMARY-INSTANTIATION: a call whose lexical substitution was
    // refused or incomplete.
    StatementFallback {
        rule: "10u",
        clears_origins: true,
        ask: |parser, query| {
            parser
                .substituted_call(query.tokens, query.line, query.instruction)
                .map(|sequence| block(query.line, sequence))
        },
    },
    // CG05-NESTED-CALL-EFFECTS: the nested-call refusal.
    StatementFallback {
        rule: "10y",
        clears_origins: true,
        ask: |parser, query| {
            parser
                .nested_call_statement(query.tokens, query.line, query.instruction)
                .map(|sequence| block(query.line, sequence))
        },
    },
    // CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: only for a refusal.
    StatementFallback {
        rule: "10z",
        clears_origins: true,
        ask: |parser, query| {
            let description = refusal(query.instruction)?;
            parser.connective_statement(query.tokens, query.line, description)
        },
    },
    // CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: only for a refusal.
    StatementFallback {
        rule: "11g",
        clears_origins: true,
        ask: |parser, query| {
            let description = refusal(query.instruction)?;
            parser.statement_expression_statement(
                query.tokens,
                query.line,
                query.origin,
                description,
            )
        },
    },
    // CG14-AGGREGATE-VALUES-AND-INITIALIZERS: only for a refusal.
    StatementFallback {
        rule: "11f",
        clears_origins: true,
        ask: |parser, query| {
            parser.aggregate_statement(query.tokens, query.line, query.instruction)
        },
    },
    // CG19-ATOMIC-SOURCE-ROUTE: an uncontracted atomic builtin's refusal, or
    // an `_Atomic` store or read-modify-write statement's. A re-worded
    // refusal is an answer too, which ends the table (ROUTE-MAP finding 9).
    StatementFallback {
        rule: "11i",
        clears_origins: true,
        ask: |parser, query| {
            parser.atomic_routed_statement(query.tokens, query.line, query.instruction)
        },
    },
    // CG17-LAYOUT-DERIVED-EXTENTS: a VLA declaration.
    StatementFallback {
        rule: "11u",
        clears_origins: true,
        ask: |parser, query| {
            parser.vla_declaration_statement(query.tokens, query.line, query.instruction)
        },
    },
    // CG17-LAYOUT-DERIVED-EXTENTS: a `container_of` value.
    StatementFallback {
        rule: "11u",
        clears_origins: true,
        ask: |parser, query| {
            parser.container_of_statement(query.tokens, query.line, query.instruction)
        },
    },
    // CG26-VARIADICS: an uncontracted `va_*` builtin's refusal.
    StatementFallback {
        rule: "11w",
        clears_origins: true,
        ask: |parser, query| parser.va_list_statement(query.tokens, query.line, query.instruction),
    },
    // CG26-VARIADICS: the declared tail's refusal of a call statement.
    StatementFallback {
        rule: "11w",
        clears_origins: true,
        ask: |parser, query| {
            parser.variadic_tail_statement(query.tokens, query.line, query.instruction)
        },
    },
    // CAST-FORMALIZATION: the uncast statement where the conversion table
    // removes a cast at a value position.
    StatementFallback {
        rule: "12h",
        clears_origins: true,
        ask: |parser, query| {
            parser.uncast_value_statement(query.tokens, query.line, query.instruction)
        },
    },
];

/// One refused return's question to the return fallbacks.
pub(super) struct ReturnQuery<'q> {
    pub(super) expression: &'q [Token],
    pub(super) line: usize,
    /// Card CG07 (rule 11g): the expression's place in the stream.
    pub(super) origin: usize,
    /// What `return_statement` decided.
    pub(super) statement: &'q Statement,
}

/// One return fallback and its rule.
pub(super) struct ReturnFallback {
    /// Read by the order test and the `route-census` stream.
    #[cfg_attr(not(any(test, feature = "route-census")), allow(dead_code))]
    pub(super) rule: &'static str,
    ask: for<'p, 'q> fn(&mut Parser<'p>, &ReturnQuery<'q>) -> Option<Statement>,
}

/// The return fallbacks of the `return` arm, in consultation order. All but
/// 10y and 11i are asked only for a refused return (`return_refused`).
pub(super) const RETURN_FALLBACKS: &[ReturnFallback] = &[
    // CG02-ELABORATION-CORE.
    ReturnFallback {
        rule: "10s",
        ask: |parser, query| {
            return_refused(query.statement)
                .then(|| parser.elaborated_return(query.expression, query.line))
                .flatten()
        },
    },
    // CG03-CALL-SUMMARY-INSTANTIATION: after the elaborator.
    ReturnFallback {
        rule: "10u",
        ask: |parser, query| {
            return_refused(query.statement)
                .then(|| parser.instantiated_return(query.expression, query.line))
                .flatten()
        },
    },
    // CG05-NESTED-CALL-EFFECTS: a refused return or the nested-call refusal
    // (its own condition).
    ReturnFallback {
        rule: "10y",
        ask: |parser, query| {
            parser.nested_call_return(query.expression, query.line, query.statement)
        },
    },
    // CG06-VALUE-CONNECTIVES-AND-CONDITIONAL.
    ReturnFallback {
        rule: "10z",
        ask: |parser, query| {
            return_refused(query.statement)
                .then(|| parser.connective_return(query.expression, query.line))
                .flatten()
        },
    },
    // CG07-GENERIC-AND-STATEMENT-EXPRESSIONS.
    ReturnFallback {
        rule: "11g",
        ask: |parser, query| {
            return_refused(query.statement)
                .then(|| {
                    parser.statement_expression_return(query.expression, query.line, query.origin)
                })
                .flatten()
        },
    },
    // CG19-ATOMIC-SOURCE-ROUTE: a refused return or an uncontracted atomic
    // builtin's.
    ReturnFallback {
        rule: "11i",
        ask: |parser, query| {
            (return_refused(query.statement)
                || atomic_route::refused_atomic_return(query.statement))
            .then(|| parser.atomic_routed_return(query.expression, query.line))
            .flatten()
        },
    },
    // CG17-LAYOUT-DERIVED-EXTENTS.
    ReturnFallback {
        rule: "11u",
        ask: |parser, query| {
            return_refused(query.statement)
                .then(|| parser.container_of_return(query.expression, query.line))
                .flatten()
        },
    },
    // W48-BOTTOM-UP-CALLEE-BODIES.
    ReturnFallback {
        rule: "12f",
        ask: |parser, query| {
            return_refused(query.statement)
                .then(|| parser.scalar_return_value(query.expression, query.line))
                .flatten()
        },
    },
];

impl Parser<'_> {
    /// Ask the statement fallbacks in table order; the first answer wins.
    /// (The row index is the census row's position under `route-census`.)
    #[allow(clippy::unused_enumerate_index)]
    pub(super) fn ask_statement_fallbacks(
        &mut self,
        query: &StatementQuery<'_>,
    ) -> Option<Statement> {
        for (_position, fallback) in STATEMENT_FALLBACKS.iter().enumerate() {
            let answer = (fallback.ask)(self, query);
            #[cfg(feature = "route-census")]
            census::record("statement", _position, fallback.rule, answer.is_some());
            if let Some(statement) = answer {
                if fallback.clears_origins {
                    self.element_origins.clear();
                }
                return Some(statement);
            }
        }
        None
    }

    /// Ask the return fallbacks in table order; the first answer wins.
    #[allow(clippy::unused_enumerate_index)]
    pub(super) fn ask_return_fallbacks(&mut self, query: &ReturnQuery<'_>) -> Option<Statement> {
        for (_position, fallback) in RETURN_FALLBACKS.iter().enumerate() {
            let answer = (fallback.ask)(self, query);
            #[cfg(feature = "route-census")]
            census::record("return", _position, fallback.rule, answer.is_some());
            if answer.is_some() {
                return answer;
            }
        }
        None
    }
}

/// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION, measurement only (cargo feature
/// `route-census`, off by default and enabled by no workspace member): one
/// JSON line per consultation of a table row to `QCORE_ROUTE_CENSUS`, so the
/// route map can name the rows a run never reaches and the rows that never
/// answer. With the feature off this module and every call of it are
/// compiled out.
#[cfg(feature = "route-census")]
mod census {
    use std::io::Write;
    use std::sync::Mutex;

    static STREAM: Mutex<Option<std::fs::File>> = Mutex::new(None);

    pub(super) fn record(table: &str, position: usize, rule: &str, answered: bool) {
        let Some(path) = std::env::var_os("QCORE_ROUTE_CENSUS") else {
            return;
        };
        let mut stream = STREAM.lock().unwrap_or_else(|error| error.into_inner());
        if stream.is_none() {
            *stream = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok();
        }
        if let Some(file) = stream.as_mut() {
            let _ = writeln!(
                file,
                "{{\"table\":\"{table}\",\"position\":{position},\"rule\":\"{rule}\",\"answered\":{answered}}}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RETURN_FALLBACKS, STATEMENT_FALLBACKS};

    /// The tables' order is the rule order the route map documents; a moved
    /// or dropped row fails here before any corpus run.
    #[test]
    fn the_fallback_tables_keep_the_documented_order() {
        let statement = STATEMENT_FALLBACKS
            .iter()
            .map(|fallback| fallback.rule)
            .collect::<Vec<_>>();
        assert_eq!(
            statement,
            [
                "W43-INDEX-TRAVERSAL",
                "W44-INDIRECT-CALL-ARGUMENT-ACCESS",
                "10s",
                "11b",
                "10u",
                "10y",
                "10z",
                "11g",
                "11f",
                "11i",
                "11u",
                "11u",
                "11w",
                "11w",
                "12h",
            ]
        );
        let returns = RETURN_FALLBACKS
            .iter()
            .map(|fallback| fallback.rule)
            .collect::<Vec<_>>();
        assert_eq!(
            returns,
            ["10s", "10u", "10y", "10z", "11g", "11i", "11u", "12f"]
        );
    }
}
