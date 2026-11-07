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
//!
//! Card **GATEWAY-RULE-API** (plan `docs/plans/pluggable-cards/README.md`,
//! seam B): the two tables are generated from the rule registry
//! `crates/gateway-rules/registry.toml` by `python3
//! scripts/gen_gateway_registry.py`, between the `generated:registry`
//! markers below (the Python suite fails when they differ from what it
//! generates, and when a row differs from the frozen route map). Each row is a
//! `qcore_gateway_rule_api` package of its position, and the dispatchers
//! compose the rows with `crate::arm_obligations::consult`: the first answer
//! wins, a row whose rule `QCORE_GATEWAY_RULES_OFF` lists is not asked and
//! falls through, and the end of the table is the existing refusal. With no
//! rule off every row is asked exactly as before. The routes' bodies are
//! unchanged and stay in this crate; the `ask` functions below are the
//! closures the hand-written tables held, named after the route they ask.

use qcore_gateway_rule_api::Answer;
use qcore_gateway_rule_api::traits::{self as package, Host, RulePackage};

use super::condition_desugar::ConditionPlan;
use super::{LegacyInstruction, Parser, Statement, Token, atomic_route, return_refused};

/// One statement's question to the statement fallbacks.
pub(super) struct StatementQuery<'q> {
    pub(super) tokens: &'q [Token],
    pub(super) line: usize,
    pub(super) origin: Option<usize>,
    pub(super) instruction: &'q LegacyInstruction,
}

/// The parser as the rule API's host: its tokens, instructions, statements,
/// condition plans and Clang nodes are the vocabulary a package reads.
impl Host for Parser<'_> {
    type Token = Token;
    type Decision = LegacyInstruction;
    type Statement = Statement;
    type Condition = Box<ConditionPlan>;
    type Node = serde_json::Value;

    fn token_text(token: &Token) -> &str {
        &token.text
    }

    fn token_line(token: &Token) -> usize {
        token.line
    }

    fn refusal(decision: &LegacyInstruction) -> Option<&str> {
        refusal(decision)
    }
}

/// The rule API's statement query over the parser.
type StatementAsk<'q, 'p> = package::StatementQuery<'q, Parser<'p>>;

/// One statement fallback: its rule (or card, where no rule is named) and
/// whether an answer clears the statement's pending element origins (card
/// W45-CONTAINER-ROOT-REACH: an answer that replaces the instruction keeps no
/// element binding of the refused one).
pub(super) struct StatementFallback {
    /// The registry id: the switch the row reads, and the name the order
    /// test and the `route-census` stream use.
    pub(super) rule: &'static str,
    /// The parser route the row asks (the registry's `route`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) route: &'static str,
    clears_origins: bool,
    ask: for<'p, 'q> fn(&mut Parser<'p>, &StatementAsk<'q, 'p>) -> Option<Statement>,
}

impl RulePackage for StatementFallback {
    fn id(&self) -> &'static str {
        self.rule
    }
}

impl<'p> package::StatementFallback<Parser<'p>> for StatementFallback {
    fn ask(&self, host: &mut Parser<'p>, query: &StatementAsk<'_, 'p>) -> Answer<Statement> {
        Answer::from_option((self.ask)(host, query))
    }
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

/// The statement fallbacks' `ask` functions, named after the parser route
/// each asks.
mod statement {
    use super::{Parser, Statement, StatementAsk, block, refusal};

    type Ask<'q, 'p> = StatementAsk<'q, 'p>;

    // W43-INDEX-TRAVERSAL: a stepped access of a refused statement.
    pub(super) fn stepped_access(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser
            .stepped_access(query.tokens, query.decision)
            .map(|sequence| block(query.line, sequence))
    }

    // W44-INDIRECT-CALL-ARGUMENT-ACCESS: a call whose actuals the gate
    // refused, lowered to its actuals' reads and the call.
    pub(super) fn call_argument_access(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser
            .call_argument_access(query.tokens, query.decision)
            .map(|sequence| block(query.line, sequence))
    }

    // CG02-ELABORATION-CORE: only for a refusal.
    pub(super) fn elaborated_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        let description = refusal(query.decision)?;
        parser.elaborated_statement(query.tokens, query.line, description)
    }

    // CG08-CASTS-AND-TYPE-QUERIES (E15): after the elaborator.
    pub(super) fn uncast_pointer_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.uncast_pointer_statement(query.tokens, query.line, query.decision)
    }

    // CG03-CALL-SUMMARY-INSTANTIATION: a call whose lexical substitution was
    // refused or incomplete.
    pub(super) fn substituted_call(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser
            .substituted_call(query.tokens, query.line, query.decision)
            .map(|sequence| block(query.line, sequence))
    }

    // CG05-NESTED-CALL-EFFECTS: the nested-call refusal.
    pub(super) fn nested_call_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser
            .nested_call_statement(query.tokens, query.line, query.decision)
            .map(|sequence| block(query.line, sequence))
    }

    // CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: only for a refusal.
    pub(super) fn connective_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        let description = refusal(query.decision)?;
        parser.connective_statement(query.tokens, query.line, description)
    }

    // CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: only for a refusal.
    pub(super) fn statement_expression_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        let description = refusal(query.decision)?;
        parser.statement_expression_statement(query.tokens, query.line, query.origin, description)
    }

    // CG14-AGGREGATE-VALUES-AND-INITIALIZERS: only for a refusal.
    pub(super) fn aggregate_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.aggregate_statement(query.tokens, query.line, query.decision)
    }

    // CG19-ATOMIC-SOURCE-ROUTE: an uncontracted atomic builtin's refusal, or
    // an `_Atomic` store or read-modify-write statement's. A re-worded
    // refusal is an answer too, which ends the table (ROUTE-MAP finding 9).
    pub(super) fn atomic_routed_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.atomic_routed_statement(query.tokens, query.line, query.decision)
    }

    // CG17-LAYOUT-DERIVED-EXTENTS: a VLA declaration.
    pub(super) fn vla_declaration_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.vla_declaration_statement(query.tokens, query.line, query.decision)
    }

    // CG17-LAYOUT-DERIVED-EXTENTS: a `container_of` value.
    pub(super) fn container_of_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.container_of_statement(query.tokens, query.line, query.decision)
    }

    // CG26-VARIADICS: an uncontracted `va_*` builtin's refusal.
    pub(super) fn va_list_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.va_list_statement(query.tokens, query.line, query.decision)
    }

    // CG26-VARIADICS: the declared tail's refusal of a call statement.
    pub(super) fn variadic_tail_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.variadic_tail_statement(query.tokens, query.line, query.decision)
    }

    // CAST-FORMALIZATION: the uncast statement where the conversion table
    // removes a cast at a value position.
    pub(super) fn uncast_value_statement(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.uncast_value_statement(query.tokens, query.line, query.decision)
    }
}

/// One refused return's question to the return fallbacks.
pub(super) struct ReturnQuery<'q> {
    pub(super) expression: &'q [Token],
    pub(super) line: usize,
    /// Card CG07 (rule 11g): the expression's place in the stream.
    pub(super) origin: usize,
    /// What `return_statement` decided.
    pub(super) statement: &'q Statement,
}

/// The rule API's return query over the parser.
type ReturnAsk<'q, 'p> = package::ReturnQuery<'q, Parser<'p>>;

/// One return fallback and its rule.
pub(super) struct ReturnFallback {
    /// The registry id: the switch the row reads, and the name the order
    /// test and the `route-census` stream use.
    pub(super) rule: &'static str,
    /// The parser route the row asks (the registry's `route`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) route: &'static str,
    ask: for<'p, 'q> fn(&mut Parser<'p>, &ReturnAsk<'q, 'p>) -> Option<Statement>,
}

impl RulePackage for ReturnFallback {
    fn id(&self) -> &'static str {
        self.rule
    }
}

impl<'p> package::ReturnFallback<Parser<'p>> for ReturnFallback {
    fn ask(&self, host: &mut Parser<'p>, query: &ReturnAsk<'_, 'p>) -> Answer<Statement> {
        Answer::from_option((self.ask)(host, query))
    }
}

/// The return fallbacks' `ask` functions. All but 10y and 11i are asked only
/// for a refused return (`return_refused`).
mod returns {
    use super::{Parser, ReturnAsk, Statement, atomic_route, return_refused};

    type Ask<'q, 'p> = ReturnAsk<'q, 'p>;

    // CG02-ELABORATION-CORE.
    pub(super) fn elaborated_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        return_refused(query.decided)
            .then(|| parser.elaborated_return(query.expression, query.line))
            .flatten()
    }

    // CG03-CALL-SUMMARY-INSTANTIATION: after the elaborator.
    pub(super) fn instantiated_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        return_refused(query.decided)
            .then(|| parser.instantiated_return(query.expression, query.line))
            .flatten()
    }

    // CG05-NESTED-CALL-EFFECTS: a refused return or the nested-call refusal
    // (its own condition).
    pub(super) fn nested_call_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        parser.nested_call_return(query.expression, query.line, query.decided)
    }

    // CG06-VALUE-CONNECTIVES-AND-CONDITIONAL.
    pub(super) fn connective_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        return_refused(query.decided)
            .then(|| parser.connective_return(query.expression, query.line))
            .flatten()
    }

    // CG07-GENERIC-AND-STATEMENT-EXPRESSIONS.
    pub(super) fn statement_expression_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        return_refused(query.decided)
            .then(|| parser.statement_expression_return(query.expression, query.line, query.origin))
            .flatten()
    }

    // CG19-ATOMIC-SOURCE-ROUTE: a refused return or an uncontracted atomic
    // builtin's.
    pub(super) fn atomic_routed_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        (return_refused(query.decided) || atomic_route::refused_atomic_return(query.decided))
            .then(|| parser.atomic_routed_return(query.expression, query.line))
            .flatten()
    }

    // CG17-LAYOUT-DERIVED-EXTENTS.
    pub(super) fn container_of_return(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        return_refused(query.decided)
            .then(|| parser.container_of_return(query.expression, query.line))
            .flatten()
    }

    // W48-BOTTOM-UP-CALLEE-BODIES.
    pub(super) fn scalar_return_value(
        parser: &mut Parser<'_>,
        query: &Ask<'_, '_>,
    ) -> Option<Statement> {
        return_refused(query.decided)
            .then(|| parser.scalar_return_value(query.expression, query.line))
            .flatten()
    }
}

// generated:registry:start (from crates/gateway-rules/registry.toml by
// `python3 scripts/gen_gateway_registry.py`; do not edit by hand)

/// The statement fallbacks of `expression_statement`, in consultation
/// order (the registry's `statement-fallback` placements).
#[rustfmt::skip]
pub(super) const STATEMENT_FALLBACKS: &[StatementFallback] = &[
    StatementFallback { rule: "W43-INDEX-TRAVERSAL", route: "stepped_access", clears_origins: false, ask: statement::stepped_access },
    StatementFallback { rule: "W44-INDIRECT-CALL-ARGUMENT-ACCESS", route: "call_argument_access", clears_origins: false, ask: statement::call_argument_access },
    StatementFallback { rule: "10s", route: "elaborated_statement", clears_origins: true, ask: statement::elaborated_statement },
    StatementFallback { rule: "11b", route: "uncast_pointer_statement", clears_origins: true, ask: statement::uncast_pointer_statement },
    StatementFallback { rule: "10u", route: "substituted_call", clears_origins: true, ask: statement::substituted_call },
    StatementFallback { rule: "10y", route: "nested_call_statement", clears_origins: true, ask: statement::nested_call_statement },
    StatementFallback { rule: "10z", route: "connective_statement", clears_origins: true, ask: statement::connective_statement },
    StatementFallback { rule: "11g", route: "statement_expression_statement", clears_origins: true, ask: statement::statement_expression_statement },
    StatementFallback { rule: "11f", route: "aggregate_statement", clears_origins: true, ask: statement::aggregate_statement },
    StatementFallback { rule: "11i", route: "atomic_routed_statement", clears_origins: true, ask: statement::atomic_routed_statement },
    StatementFallback { rule: "11u", route: "vla_declaration_statement", clears_origins: true, ask: statement::vla_declaration_statement },
    StatementFallback { rule: "11u", route: "container_of_statement", clears_origins: true, ask: statement::container_of_statement },
    StatementFallback { rule: "11w", route: "va_list_statement", clears_origins: true, ask: statement::va_list_statement },
    StatementFallback { rule: "11w", route: "variadic_tail_statement", clears_origins: true, ask: statement::variadic_tail_statement },
    StatementFallback { rule: "12h", route: "uncast_value_statement", clears_origins: true, ask: statement::uncast_value_statement },
];

/// The return fallbacks of the `return` arm, in consultation order (the
/// registry's `return-fallback` placements).
#[rustfmt::skip]
pub(super) const RETURN_FALLBACKS: &[ReturnFallback] = &[
    ReturnFallback { rule: "10s", route: "elaborated_return", ask: returns::elaborated_return },
    ReturnFallback { rule: "10u", route: "instantiated_return", ask: returns::instantiated_return },
    ReturnFallback { rule: "10y", route: "nested_call_return", ask: returns::nested_call_return },
    ReturnFallback { rule: "10z", route: "connective_return", ask: returns::connective_return },
    ReturnFallback { rule: "11g", route: "statement_expression_return", ask: returns::statement_expression_return },
    ReturnFallback { rule: "11i", route: "atomic_routed_return", ask: returns::atomic_routed_return },
    ReturnFallback { rule: "11u", route: "container_of_return", ask: returns::container_of_return },
    ReturnFallback { rule: "12f", route: "scalar_return_value", ask: returns::scalar_return_value },
];

// generated:registry:end

impl Parser<'_> {
    /// Ask the statement fallbacks in table order; the first answer wins and
    /// a switched-off rule's row is not asked. (The row index is the census
    /// row's position under `route-census`.)
    pub(super) fn ask_statement_fallbacks(
        &mut self,
        query: &StatementQuery<'_>,
    ) -> Option<Statement> {
        let query = StatementAsk {
            tokens: query.tokens,
            line: query.line,
            origin: query.origin,
            decision: query.instruction,
        };
        let decided = crate::arm_obligations::consult(
            STATEMENT_FALLBACKS,
            |fallback| fallback.rule,
            |fallback| crate::gateway_rules::enabled(fallback.rule),
            |_position, fallback| {
                let answer = package::StatementFallback::ask(fallback, self, &query);
                #[cfg(feature = "route-census")]
                census::record(
                    "statement",
                    _position,
                    fallback.rule,
                    matches!(answer, Answer::Admit(_)),
                );
                answer
            },
        );
        // Card SCALAR-STATEMENT-FORMS: measurement only (feature
        // `scalar-form-census`, off by default).
        #[cfg(feature = "scalar-form-census")]
        if let Some(description) = refusal(query.decision) {
            let texts = query
                .tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            crate::scalar_form_census::record(
                &self.typed_path_function,
                query.line,
                &texts,
                description,
                "fallback",
                Some(decided.is_some()),
            );
        }
        let decided = decided?;
        if STATEMENT_FALLBACKS[decided.index].clears_origins {
            self.element_origins.clear();
        }
        Some(decided.lowering)
    }

    /// Ask the return fallbacks in table order; the first answer wins and a
    /// switched-off rule's row is not asked.
    pub(super) fn ask_return_fallbacks(&mut self, query: &ReturnQuery<'_>) -> Option<Statement> {
        let query = ReturnAsk {
            expression: query.expression,
            line: query.line,
            origin: query.origin,
            decided: query.statement,
        };
        crate::arm_obligations::consult(
            RETURN_FALLBACKS,
            |fallback| fallback.rule,
            |fallback| crate::gateway_rules::enabled(fallback.rule),
            |_position, fallback| {
                let answer = package::ReturnFallback::ask(fallback, self, &query);
                #[cfg(feature = "route-census")]
                census::record(
                    "return",
                    _position,
                    fallback.rule,
                    matches!(answer, Answer::Admit(_)),
                );
                answer
            },
        )
        .map(|decided| decided.lowering)
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
    use crate::gateway_rules::REGISTRY;
    use qcore_gateway_rule_api::{Position, check_registry, ordered_table};

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

    /// Card GATEWAY-RULE-API: the compiled tables are the registry's ordered
    /// placements, row for row (rule and route), and the registry keeps the
    /// API's laws.
    #[test]
    fn the_tables_are_the_registry_placements() {
        assert_eq!(check_registry(REGISTRY), Ok(()));
        let placed = |position| {
            ordered_table(REGISTRY, position)
                .into_iter()
                .map(|(entry, placement)| (entry.id, placement.route))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            placed(Position::StatementFallback),
            STATEMENT_FALLBACKS
                .iter()
                .map(|row| (row.rule, row.route))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            placed(Position::ReturnFallback),
            RETURN_FALLBACKS
                .iter()
                .map(|row| (row.rule, row.route))
                .collect::<Vec<_>>()
        );
    }
}
