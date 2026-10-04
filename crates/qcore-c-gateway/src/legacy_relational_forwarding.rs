//! Card **CALLEE-RETENTION-CLASS1**, stage (a) (gateway rule 12s): relational
//! forwarding and the per-class row of a function whose return-code
//! disposition is in force. It amends rule 11r and adds no vocabulary.
//!
//! # Why
//!
//! A custody-taking insert (`add_item_to_array`, `cfl_kvlist_insert_array`)
//! takes custody of its item on success only, which a return-code disposition
//! (contract spec §C04.13) states. Its callers are almost all *forwarding
//! wrappers*, `return f(ARGS);`, and rule 11r refuses such a call
//! (`relational-untested`): the value is not tested where it stands. The
//! wrapper's own caller is the one that tests it, so the wrapper has the same
//! relation, and nothing in rule 11r could say so.
//!
//! # The forwarding route (rule 12s(1))
//!
//! Consulted after every ordinary route and rule 11r's exit-specific route
//! published nothing for a function the unit defines. Its body qualifies only
//! when **every** `return` is either a plain integer literal or
//! `return f(ARGS);` — the whole returned expression one call, optionally
//! parenthesised — of a callee whose disposition is in force (declared and not
//! withdrawn, or inferred), with at least one such return; every forwarded
//! callee has the same set of class relations; the function's result type is
//! spelled exactly as each forwarded callee's (so C converts nothing between
//! the two values); and every literal lies in one of those relations.
//!
//! The function then takes the callee's relations as its classes. For each
//! class `r` the body is restricted to the exits returning a value in `r`:
//! a literal outside `r` becomes the divergent `for (;;);` (rule 11r's own
//! device), and a forwarded `return f(ARGS);` becomes
//! `{ if (f(ARGS) OP K) return R; for (;;); }` with `x OP K` the relation `r`
//! and `R` a literal in it. Rule 11r's decided test reads that condition, so
//! the call is classified once, under the callee's class `r` on the path that
//! returns, and the other path diverges. The **ordinary** inference and
//! validation then run unchanged on the restricted body, and the class `r`
//! takes the contract validated there. A class that does not validate, a test
//! rule 11r refuses (a callee whose other classes disagree on the false edge),
//! a non-`unit` result, classes that all agree, or classes whose entry
//! requirements differ publish nothing.
//!
//! # The per-class row (rule 12s(2))
//!
//! A function whose own disposition is in force was analysed contract-free:
//! its row kept the forwarded call's `relational-untested` refusal, or an
//! unresolved formal, and was never `Safe`. Its row is now decided per class:
//! each restricted body (as above) is analysed under that class's contract as
//! its entry and must validate against it (`observations_match_contract`,
//! the W4 gate); the published row is then the **joint** body — the original
//! body with each forwarded return lowered to the two-armed decided test
//! `{ if (f(ARGS) OP K) return R; return R'; }` — analysed under the common
//! entry, and it is published only when it reports no diagnostic. Every path
//! of the original body ends in one class's exit or diverges, so the classes
//! together cover every path. Any failure keeps the contract-free row.
//!
//! The Lean model is `QCore.RelationalForwarding`.

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceParameterMode};

use crate::legacy_c::{
    AnalysisStatus, EntryCustody, LegacyFunctionSummary, Token, analyze_cfg_with_entry,
    build_function_cfg, function_parameters,
};
use crate::relational_post::{
    ExitClass, RelationOp, RelationalContract, RelationalOrigin, ReturnRelation, integer_constant,
};

use super::relational::definition;

/// The storage-class words a result-type comparison ignores.
const STORAGE: &[&str] = &["static", "inline", "extern", "__inline", "__inline__"];

/// One `return` of the body: its token span and what it returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Return {
    /// The `return` keyword.
    pub(crate) start: usize,
    /// The statement's `;`.
    pub(crate) end: usize,
    pub(crate) value: Returned,
}

/// What one `return` returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Returned {
    /// A plain integer literal.
    Literal(i64),
    /// `f(ARGS)`: the call's tokens span `[call_start, call_end]`.
    Forward {
        callee: String,
        call_start: usize,
        call_end: usize,
    },
}

/// The `;` that ends the statement starting at `start`, at bracket depth 0.
fn statement_end(tokens: &[Token], start: usize, limit: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().take(limit).skip(start) {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.checked_sub(1)?,
            ";" if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

/// The whole-expression call `[start, end)` denotes, parentheses stripped:
/// `(callee index, closing parenthesis)`.
fn whole_call(tokens: &[Token], mut start: usize, mut end: usize) -> Option<(usize, usize)> {
    while end > start + 1
        && tokens[start].text == "("
        && crate::legacy_c::matching(tokens, start, "(", ")") == Some(end - 1)
    {
        start += 1;
        end -= 1;
    }
    let name = &tokens.get(start)?.text;
    let identifier = name
        .chars()
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && name.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric());
    if !identifier || tokens.get(start + 1)?.text != "(" {
        return None;
    }
    (crate::legacy_c::matching(tokens, start + 1, "(", ")")? == end - 1).then_some((start, end - 1))
}

/// Every `return` of the body `(open, close)`, each a literal or a forwarded
/// call of a callee with a disposition in force in `environment`. `None` when
/// one is anything else.
pub(crate) fn read_returns(
    tokens: &[Token],
    open: usize,
    close: usize,
    environment: &ResourceContracts,
) -> Option<Vec<Return>> {
    let mut returns = Vec::new();
    let mut index = open;
    while index < close {
        if tokens[index].text != "return" {
            index += 1;
            continue;
        }
        let end = statement_end(tokens, index + 1, close)?;
        let texts = tokens[index + 1..end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let value = if let Some(value) = integer_constant(&texts) {
            Returned::Literal(value)
        } else {
            let (callee, call_end) = whole_call(tokens, index + 1, end)?;
            let name = tokens[callee].text.clone();
            let contract = environment.relational_contract(&name)?;
            if contract.withdrawn.is_some() {
                return None;
            }
            Returned::Forward {
                callee: name,
                call_start: callee,
                call_end,
            }
        };
        returns.push(Return {
            start: index,
            end,
            value,
        });
        index = end + 1;
    }
    Some(returns)
}

/// The result-type words of the definition or declaration whose name is at
/// `name`: the specifiers back to the previous statement boundary, without
/// storage-class words and `__attribute__((…))` groups.
fn result_words(tokens: &[Token], name: usize) -> Option<Vec<String>> {
    let start = tokens[..name]
        .iter()
        .rposition(|token| matches!(token.text.as_str(), ";" | "}" | "{"))
        .map_or(0, |index| index + 1);
    let mut words = Vec::new();
    let mut index = start;
    while index < name {
        let text = tokens[index].text.as_str();
        if text == "__attribute__" || text == "__attribute" {
            let close = crate::legacy_c::matching(tokens, index + 1, "(", ")")?;
            index = close + 1;
            continue;
        }
        if !STORAGE.contains(&text) {
            words.push(text.to_owned());
        }
        index += 1;
    }
    let identifier = |word: &String| {
        word.chars()
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
            && word.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    };
    (!words.is_empty() && words.iter().all(identifier)).then_some(words)
}

/// The result-type words of the first declaration of `function` in the stream.
fn declared_result_words(tokens: &[Token], function: &str) -> Option<Vec<String>> {
    let index = tokens.iter().enumerate().position(|(index, token)| {
        token.text == function
            && tokens.get(index + 1).is_some_and(|next| next.text == "(")
            && index > 0
            && !matches!(
                tokens[index - 1].text.as_str(),
                "." | "->" | "=" | "(" | "," | "return" | "&" | "*"
            )
    })?;
    result_words(tokens, index)
}

/// The relations every forwarded callee shares, in the first callee's order.
fn forwarded_relations(
    returns: &[Return],
    environment: &ResourceContracts,
) -> Option<Vec<ReturnRelation>> {
    let mut relations: Option<Vec<ReturnRelation>> = None;
    for value in returns.iter().map(|ret| &ret.value) {
        let Returned::Forward { callee, .. } = value else {
            continue;
        };
        let contract = environment.relational_contract(callee)?;
        let these = contract
            .classes
            .iter()
            .map(|class| class.relation)
            .collect::<Vec<_>>();
        match &relations {
            None => relations = Some(these),
            Some(first) => {
                let mut a = first.clone();
                let mut b = these;
                a.sort();
                b.sort();
                if a != b {
                    return None;
                }
            }
        }
    }
    relations
}

/// A value the relation holds: the constant itself, or its neighbour.
pub(crate) fn representative(relation: ReturnRelation) -> i64 {
    match relation.op {
        RelationOp::Eq | RelationOp::Le | RelationOp::Ge => relation.constant,
        RelationOp::Ne | RelationOp::Gt => relation.constant + 1,
        RelationOp::Lt => relation.constant - 1,
    }
}

fn synthetic(text: &str, line: usize) -> Token {
    Token {
        text: text.to_owned(),
        line,
    }
}

/// The C comparison tokens of `op`, one punctuator per token as the lexer
/// emits them.
fn comparison(op: RelationOp) -> &'static [&'static str] {
    match op {
        RelationOp::Eq => &["=", "="],
        RelationOp::Ne => &["!", "="],
        RelationOp::Lt => &["<"],
        RelationOp::Le => &["<", "="],
        RelationOp::Gt => &[">"],
        RelationOp::Ge => &[">", "="],
    }
}

fn integer_tokens(value: i64, line: usize) -> Vec<Token> {
    if value < 0 {
        vec![
            synthetic("-", line),
            synthetic(&value.unsigned_abs().to_string(), line),
        ]
    } else {
        vec![synthetic(&value.to_string(), line)]
    }
}

fn diverge(line: usize) -> Vec<Token> {
    ["for", "(", ";", ";", ")", ";"]
        .into_iter()
        .map(|text| synthetic(text, line))
        .collect()
}

/// `( CALL OP K )`, the decided test of one class.
fn test(tokens: &[Token], start: usize, end: usize, relation: ReturnRelation) -> Vec<Token> {
    let line = tokens[start].line;
    let mut out = vec![synthetic("(", line)];
    out.extend_from_slice(&tokens[start..=end]);
    out.extend(comparison(relation.op).iter().map(|op| synthetic(op, line)));
    out.extend(integer_tokens(relation.constant, line));
    out.push(synthetic(")", line));
    out
}

/// How one forwarded return is lowered.
enum Lowering {
    /// The restricted body of one class: returns there, diverges otherwise.
    Class(ReturnRelation),
    /// The joint body of a two-class disposition: each arm returns.
    Joint(ReturnRelation, ReturnRelation),
}

/// `tokens` with every `return` of `returns` rewritten for `lowering`.
fn rewrite(tokens: &[Token], returns: &[Return], lowering: &Lowering) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len() + returns.len() * 16);
    let mut copied = 0usize;
    for ret in returns {
        let line = tokens[ret.start].line;
        let replacement = match (&ret.value, lowering) {
            (Returned::Literal(value), Lowering::Class(relation)) => {
                if relation.holds(i128::from(*value)) {
                    continue;
                }
                diverge(line)
            }
            (Returned::Literal(_), Lowering::Joint(..)) => continue,
            (
                Returned::Forward {
                    call_start,
                    call_end,
                    ..
                },
                Lowering::Class(relation),
            ) => {
                let mut statement = vec![synthetic("{", line), synthetic("if", line)];
                statement.extend(test(tokens, *call_start, *call_end, *relation));
                statement.push(synthetic("return", line));
                statement.extend(integer_tokens(representative(*relation), line));
                statement.push(synthetic(";", line));
                statement.extend(diverge(line));
                statement.push(synthetic("}", line));
                statement
            }
            (
                Returned::Forward {
                    call_start,
                    call_end,
                    ..
                },
                Lowering::Joint(first, second),
            ) => {
                let mut statement = vec![synthetic("{", line), synthetic("if", line)];
                statement.extend(test(tokens, *call_start, *call_end, *first));
                statement.push(synthetic("return", line));
                statement.extend(integer_tokens(representative(*first), line));
                statement.push(synthetic(";", line));
                statement.push(synthetic("return", line));
                statement.extend(integer_tokens(representative(*second), line));
                statement.push(synthetic(";", line));
                statement.push(synthetic("}", line));
                statement
            }
        };
        out.extend_from_slice(&tokens[copied..ret.start]);
        out.extend(replacement);
        copied = ret.end + 1;
    }
    out.extend_from_slice(&tokens[copied..]);
    out
}

/// What a parameter mode asks of the caller at entry: nothing, live custody
/// of a nullability, or an out slot of a nullability. `Owned`, `Consumed` and
/// `Released` prescribe the same entry state (`legacy_summary`).
fn entry_requirement(mode: &ResourceParameterMode) -> Option<(u8, bool)> {
    match *mode {
        ResourceParameterMode::Borrowed => Some((0, false)),
        ResourceParameterMode::Owned { nullable }
        | ResourceParameterMode::Consumed { nullable }
        | ResourceParameterMode::Released { nullable } => Some((1, nullable)),
        ResourceParameterMode::Out { nullable } => Some((2, nullable)),
        ResourceParameterMode::Stored { .. } | ResourceParameterMode::Removed { .. } => None,
    }
}

/// Whether every class asks the same of the caller at entry.
pub(crate) fn entries_agree(classes: &[ExitClass]) -> bool {
    let Some(first) = classes.first() else {
        return false;
    };
    classes.iter().all(|class| {
        class.contract.parameters.len() == first.contract.parameters.len()
            && class
                .contract
                .parameters
                .iter()
                .zip(&first.contract.parameters)
                .all(|(a, b)| {
                    let (a, b) = (entry_requirement(a), entry_requirement(b));
                    a.is_some() && a == b
                })
    })
}

/// The body read for rule 12s: the definition and its returns. `None` when
/// the function is not defined in the stream or a return is neither form.
fn qualifying_body(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<(usize, Vec<Return>)> {
    let (name, open, close) = definition(tokens, function)?;
    let returns = read_returns(tokens, open, close, environment)?;
    Some((name, returns))
}

/// Rule 12s(1): the forwarded disposition of `function`, or `None`. `infer`
/// is the ordinary inference and validation, run unchanged on each restricted
/// body.
pub(crate) fn infer_forwarded(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    infer: impl Fn(&[Token], &str, &ResourceContracts) -> Option<ResourceFunctionContract>,
) -> Option<RelationalContract> {
    let (name, returns) = qualifying_body(tokens, function, environment)?;
    let relations = forwarded_relations(&returns, environment)?;
    // The forwarded value is the function's own: C converts nothing only when
    // the two result types are spelled alike.
    let own = result_words(tokens, name)?;
    for ret in &returns {
        if let Returned::Forward { callee, .. } = &ret.value
            && declared_result_words(tokens, callee).as_ref() != Some(&own)
        {
            return None;
        }
    }
    for ret in &returns {
        if let Returned::Literal(value) = ret.value
            && !relations
                .iter()
                .any(|relation| relation.holds(i128::from(value)))
        {
            return None;
        }
    }
    let mut classes = Vec::with_capacity(relations.len());
    for relation in relations {
        let body = rewrite(tokens, &returns, &Lowering::Class(relation));
        let contract = infer(&body, function, environment)?;
        if contract.result != qcore_schemas::contract::ResourceReturnMode::None {
            return None;
        }
        classes.push(ExitClass { relation, contract });
    }
    let summary = RelationalContract {
        origin: RelationalOrigin::InferredValidated,
        classes,
        withdrawn: None,
    };
    (summary.join().is_none() && entries_agree(&summary.classes)).then_some(summary)
}

/// Rule 12s(2): the row of `function`, decided per class of its disposition
/// in force, or `None` (the contract-free row stands).
pub(crate) fn relational_row(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    contract: &RelationalContract,
) -> Option<LegacyFunctionSummary> {
    if contract.withdrawn.is_some() || !entries_agree(&contract.classes) {
        return None;
    }
    let (_, returns) = qualifying_body(tokens, function, environment)?;
    let forwards = returns
        .iter()
        .any(|ret| matches!(ret.value, Returned::Forward { .. }));
    for ret in &returns {
        if let Returned::Literal(value) = ret.value
            && !contract
                .classes
                .iter()
                .any(|class| class.relation.holds(i128::from(value)))
        {
            return None;
        }
    }
    for class in &contract.classes {
        let body = rewrite(tokens, &returns, &Lowering::Class(class.relation));
        let cfg = build_function_cfg(&body, function, environment).ok()?;
        let parameters = function_parameters(&body, function)?;
        let entry = EntryCustody::build(&cfg, &parameters, &class.contract)?;
        let (candidate, observations) = analyze_cfg_with_entry(cfg, &entry);
        if !crate::legacy_summary::observations_match_contract(
            &class.contract,
            &entry,
            &candidate,
            &observations,
        ) {
            return None;
        }
    }
    let joint = if forwards {
        let [first, second] = contract.classes.as_slice() else {
            return None;
        };
        rewrite(
            tokens,
            &returns,
            &Lowering::Joint(first.relation, second.relation),
        )
    } else {
        tokens.to_vec()
    };
    let cfg = build_function_cfg(&joint, function, environment).ok()?;
    let parameters = function_parameters(&joint, function)?;
    let entry = EntryCustody::build(&cfg, &parameters, &contract.classes[0].contract)?;
    let (candidate, _) = analyze_cfg_with_entry(cfg, &entry);
    (candidate.status == AnalysisStatus::Safe && candidate.diagnostics.is_empty())
        .then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(text: &str) -> Vec<Token> {
        let mut out = Vec::new();
        for word in text.split_whitespace() {
            if word.chars().all(|ch| ch.is_ascii_punctuation()) && word.len() > 1 {
                out.extend(word.chars().map(|ch| synthetic(&ch.to_string(), 1)));
            } else {
                out.push(synthetic(word, 1));
            }
        }
        out
    }

    fn texts(tokens: &[Token]) -> String {
        tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn representatives_lie_in_their_relations() {
        for op in RelationOp::ALL {
            for constant in [-2, 0, 3] {
                let relation = ReturnRelation::new(op, constant);
                assert!(relation.holds(i128::from(representative(relation))));
            }
        }
    }

    #[test]
    fn a_forwarded_return_is_lowered_per_class_and_jointly() {
        use qcore_schemas::contract::ResourceReturnMode;
        let tokens = stream("int w ( void * p ) { if ( ! p ) return - 1 ; return ( f ( p ) ) ; }");
        let eq = ReturnRelation::new(RelationOp::Eq, 0);
        let ne = ReturnRelation::new(RelationOp::Ne, 0);
        let mut environment = ResourceContracts::default();
        let class = |relation, mode| ExitClass {
            relation,
            contract: ResourceFunctionContract::new([mode], ResourceReturnMode::None),
        };
        environment.insert_relational(
            "f",
            RelationalContract {
                origin: RelationalOrigin::Declared,
                classes: vec![
                    class(eq, ResourceParameterMode::Consumed { nullable: true }),
                    class(ne, ResourceParameterMode::Owned { nullable: true }),
                ],
                withdrawn: None,
            },
        );
        let (open, close) = (7, tokens.len() - 1);
        let returns = read_returns(&tokens, open, close, &environment).expect("both forms");
        assert_eq!(returns.len(), 2);
        assert_eq!(returns[0].value, Returned::Literal(-1));
        assert!(matches!(&returns[1].value, Returned::Forward { callee, .. } if callee == "f"));
        // An undeclared callee is no forwarded return.
        assert!(read_returns(&tokens, open, close, &ResourceContracts::default()).is_none());
        assert_eq!(
            texts(&rewrite(&tokens, &returns, &Lowering::Class(eq))),
            "int w ( void * p ) { if ( ! p ) for ( ; ; ) ; \
             { if ( f ( p ) = = 0 ) return 0 ; for ( ; ; ) ; } }"
        );
        assert_eq!(
            texts(&rewrite(&tokens, &returns, &Lowering::Class(ne))),
            "int w ( void * p ) { if ( ! p ) return - 1 ; \
             { if ( f ( p ) ! = 0 ) return 1 ; for ( ; ; ) ; } }"
        );
        assert_eq!(
            texts(&rewrite(&tokens, &returns, &Lowering::Joint(eq, ne))),
            "int w ( void * p ) { if ( ! p ) return - 1 ; \
             { if ( f ( p ) = = 0 ) return 0 ; return 1 ; } }"
        );
    }

    #[test]
    fn result_words_skip_storage_and_attributes() {
        let tokens = stream(
            "; static __attribute__ ( ( visibility ( \"default\" ) ) ) cJSON_bool w ( void ) ;",
        );
        let name = tokens.iter().position(|t| t.text == "w").unwrap();
        assert_eq!(result_words(&tokens, name), Some(vec!["cJSON_bool".into()]));
        let tokens = stream("; unsigned int * w ( void ) ;");
        let name = tokens.iter().position(|t| t.text == "w").unwrap();
        assert_eq!(result_words(&tokens, name), None);
    }

    #[test]
    fn entry_requirements_agree_only_on_the_same_entry() {
        use qcore_schemas::contract::ResourceReturnMode;
        let class = |mode| ExitClass {
            relation: ReturnRelation::new(RelationOp::Eq, 0),
            contract: ResourceFunctionContract::new([mode], ResourceReturnMode::None),
        };
        assert!(entries_agree(&[
            class(ResourceParameterMode::Consumed { nullable: true }),
            class(ResourceParameterMode::Owned { nullable: true }),
        ]));
        assert!(!entries_agree(&[
            class(ResourceParameterMode::Consumed { nullable: true }),
            class(ResourceParameterMode::Owned { nullable: false }),
        ]));
        assert!(!entries_agree(&[
            class(ResourceParameterMode::Out { nullable: false }),
            class(ResourceParameterMode::Borrowed),
        ]));
    }
}
