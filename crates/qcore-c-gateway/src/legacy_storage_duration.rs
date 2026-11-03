//! Card **CG15-STORAGE-DURATION-AND-VOLATILE** (gateway rule 11h): the
//! route-G adapter of [`crate::storage_duration`]. The hooks in `legacy_c.rs`
//! naming the card are thin; every consultation is a `QCORE_TYPED_PATH_PARITY`
//! row (`position: "storage-duration-…"` / `"volatile-…"`).
//!
//! * **A block-scope static-duration declaration is no event** (a soundness
//!   repair, asked first). A statement Clang confirms is one `DeclStmt` every
//!   declarator of which is `static`, `extern` or thread-local is lowered to
//!   nothing: its initializer is a constant expression evaluated before the
//!   program starts. Before this card a braced one was refused and a plain one
//!   (`static void *cache = 0;`) was lowered as a store on every call.
//! * **`volatile` objects carry no value-derived fact** ([`settle_volatile`],
//!   once per function, over the built CFG). A node that binds a `volatile`
//!   object to a pointer or identity, or hands one to a custody-taking
//!   position, is refused; a known-value store to one is forgotten; a null or
//!   validity test of one refines neither edge.
//! * **A custody decision on a `volatile` read refuses** ([`settle_volatile`]):
//!   a branch node whose condition Clang shows reading a `volatile` object,
//!   and whose two successors' reach differs in a custody-bearing node, is
//!   refused. A branch whose arms carry no custody event (a spin-wait on a
//!   flag) stays admitted.
//! * **The observable trace**: each statement's `volatile` accesses, with the
//!   order C gives them, are parity rows (`position: "volatile-trace"`); no
//!   verdict reads them.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::{LegacyCfg, LegacyInstruction, Parser, Statement, Token};
use crate::storage_duration::{
    Disposition, Form, FunctionFacts, VOLATILE_DECISION_REFUSAL, decide, storage_of,
    volatile_binding_refusal, volatile_trace,
};
use crate::typed_access_path::{StatementIndex, record_row};
use crate::typed_c_ast::{children, kind};

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

impl Parser<'_> {
    fn storage_record(&self, position: &str, line: usize, tokens: &[Token], answer: &str) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": format!("storage-duration-{position}"),
            "answer": answer,
        }));
    }

    /// Hook (rule 11h, a soundness repair asked first): a statement that is
    /// one Clang `DeclStmt` whose every declarator has static or thread
    /// storage duration is no event. `None` leaves the statement to every
    /// other route.
    pub(super) fn static_duration_declaration(
        &mut self,
        tokens: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let statement_texts = texts(tokens);
        if !statement_texts
            .iter()
            .any(|text| crate::storage_duration::STATIC_STORAGE_KEYWORDS.contains(text))
        {
            return None;
        }
        let index = self.typed_positions_index()?.clone();
        let node =
            Self::elaboration_node(&index, tokens.first()?.line, &statement_texts, &|node| {
                kind(node) == "DeclStmt"
            });
        let Some(node) = node else {
            self.storage_record("declaration", line, tokens, "unmatched");
            return None;
        };
        // Every declarator static-duration; a tag the statement defines
        // (`static const struct { … } t[] = …;`) runs nothing either.
        let declarators = children(node);
        let all_static = declarators
            .iter()
            .any(|declaration| kind(declaration) == "VarDecl")
            && declarators.iter().all(|declaration| {
                matches!(kind(declaration), "RecordDecl" | "EnumDecl")
                    || storage_of(declaration, true).is_some_and(|storage| storage.outlives_frame())
            });
        if !all_static {
            self.storage_record("declaration", line, tokens, "not-static-duration");
            return None;
        }
        debug_assert_eq!(decide(Form::StaticDeclaration), Disposition::NoEvent);
        self.storage_record("declaration", line, tokens, "no-event");
        Some(Statement::Instruction(LegacyInstruction::NoOp, line))
    }

    /// Hook (rule 11h), asked after every route of a `return` declined: a
    /// returned value that is the address of a static- or thread-duration
    /// object carries no identity and outlives the frame, so the return is
    /// custody-neutral (`NoOp`, escaping nothing).
    pub(super) fn static_address_return(
        &self,
        expression: &[Token],
    ) -> Option<(LegacyInstruction, bool)> {
        let first = expression.first()?;
        let index = self.typed_positions_index()?;
        let mut wanted = vec!["return"];
        wanted.extend(texts(expression));
        let statement = Self::elaboration_node(index, first.line, &wanted, &|node| {
            kind(node) == "ReturnStmt"
        })?;
        let [value] = children(statement) else {
            return None;
        };
        let automatic = crate::storage_duration::automatic_ids(&index.function);
        let admitted = crate::storage_duration::static_address(value, &automatic);
        self.storage_record(
            "address-return",
            first.line,
            expression,
            if admitted { "no-identity" } else { "declined" },
        );
        admitted.then(|| {
            debug_assert_eq!(decide(Form::StaticAddress), Disposition::NoIdentity);
            (LegacyInstruction::NoOp, false)
        })
    }

    /// Hook (rule 11h), asked after the typed actual declined at a position
    /// that admits a read: the actual is the address of a static- or
    /// thread-duration object, which carries no identity. Matched through the
    /// unique call of `callee` whose actual `position` has exactly these
    /// tokens on `line`.
    pub(super) fn static_address_argument(
        &self,
        callee: &str,
        position: usize,
        argument: &[&str],
        line: usize,
    ) -> Option<super::call_argument_access::ArgumentRead> {
        let index = self.typed_positions_index()?;
        let mut found = Vec::new();
        visit(&index.function, &mut |node| {
            if kind(node) != "CallExpr" {
                return;
            }
            let inner = children(node);
            let named = inner.first().is_some_and(|callee_node| {
                let mut at = callee_node;
                while kind(at) == "ImplicitCastExpr" || kind(at) == "ParenExpr" {
                    match children(at).first() {
                        Some(child) => at = child,
                        None => return false,
                    }
                }
                kind(at) == "DeclRefExpr" && at["referencedDecl"]["name"].as_str() == Some(callee)
            });
            if !named {
                return;
            }
            if let Some(actual) = inner.get(position + 1)
                && index.node_line(actual) == Some(line)
                && index.node_tokens(actual).is_some_and(|tokens| {
                    tokens
                        .iter()
                        .map(String::as_str)
                        .eq(argument.iter().copied())
                })
            {
                found.push(actual.clone());
            }
        });
        let [actual] = found.as_slice() else {
            return None;
        };
        let automatic = crate::storage_duration::automatic_ids(&index.function);
        let admitted = crate::storage_duration::static_address(actual, &automatic);
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": argument,
            "callee": callee,
            "index": position,
            "position": "storage-duration-address-argument",
            "answer": if admitted { "no-identity" } else { "declined" },
        }));
        admitted.then(|| super::call_argument_access::ArgumentRead {
            bases: Vec::new(),
            record_bases: Vec::new(),
            pointer_valued: false,
            chains: Vec::new(),
        })
    }

    /// Card CG15's facts of this function, when Clang's AST is available.
    pub(super) fn storage_facts(&self) -> Option<(StatementIndex, FunctionFacts)> {
        let index = self.typed_positions_index()?.clone();
        let facts = FunctionFacts::read(&index.function, &|node| index.node_line(node));
        Some((index, facts))
    }

    /// Parity only: the observable `volatile` trace of every full expression
    /// statement of the function that reads or writes one.
    pub(super) fn record_volatile_traces(&self, index: &StatementIndex) {
        fn walk(parser: &Parser<'_>, index: &StatementIndex, node: &Value) {
            if kind(node) == "CompoundStmt" {
                for statement in children(node) {
                    if matches!(
                        kind(statement),
                        "CompoundStmt"
                            | "IfStmt"
                            | "WhileStmt"
                            | "DoStmt"
                            | "ForStmt"
                            | "SwitchStmt"
                            | "CaseStmt"
                            | "DefaultStmt"
                            | "LabelStmt"
                    ) {
                        continue;
                    }
                    let trace = volatile_trace(statement);
                    if trace.accesses.is_empty() {
                        continue;
                    }
                    record_row(json!({
                        "function": parser.typed_path_function,
                        "line": index.node_line(statement),
                        "position": "volatile-trace",
                        "count": trace.accesses.len(),
                        "accesses": trace.accesses.iter().map(|access| json!({
                            "object": access.object,
                            "write": access.write,
                        })).collect::<Vec<_>>(),
                        "sequenced": trace.sequenced,
                        "unsequenced": trace.unsequenced,
                        "unsequenced_conflict": trace.unsequenced_conflict(),
                    }));
                }
            }
            for child in children(node) {
                walk(parser, index, child);
            }
        }
        if crate::typed_access_path::parity_enabled() {
            walk(self, index, &index.function);
        }
    }
}

/// Whether the token stream spells a `volatile` qualifier anywhere up to the
/// end of the body opening at `body_start`: every object a function names is
/// declared there (a file-scope one before its use), so a `volatile` object
/// it names is spelled there. Complete, and cheap: a unit with none needs no
/// Clang AST for rule 11h.
pub(super) fn mentions_volatile(tokens: &[Token], body_start: usize) -> bool {
    let end = super::matching(tokens, body_start, "{", "}").map_or(tokens.len(), |end| end + 1);
    tokens[..end.min(tokens.len())].iter().any(|token| {
        matches!(
            token.text.as_str(),
            "volatile" | "__volatile" | "__volatile__"
        )
    })
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// The names a node binds (`true`: a destination), hands to a custody-taking
/// position or reads as a pointer identity (`false`), when it is not
/// custody-neutral in the name.
fn custody_names(instruction: &LegacyInstruction) -> Vec<(&str, bool)> {
    use LegacyInstruction::*;
    match instruction {
        Acquire { destination, .. } => vec![(destination, true)],
        Alias {
            destination,
            source,
        } => vec![(destination, true), (source, false)],
        Release { source } | Escape { source } | ExactFormalReturn { source, .. } => {
            vec![(source, false)]
        }
        SelfInitCall { base, .. } | TrackedSelfInit { base, .. } => vec![(base, false)],
        ContractCall {
            destination,
            arguments,
            contract,
            ..
        } => {
            let mut names = destination
                .iter()
                .map(|name| (name.as_str(), true))
                .collect::<Vec<_>>();
            for (position, argument) in arguments.iter().enumerate() {
                let borrowed = contract.parameters.get(position).is_some_and(|mode| {
                    matches!(
                        mode,
                        qcore_schemas::contract::ResourceParameterMode::Borrowed
                    )
                });
                if let Some(argument) = argument
                    && !borrowed
                {
                    names.push((argument, false));
                }
            }
            names
        }
        PointerStep {
            destination,
            source,
            ..
        } => vec![(destination, true), (source, false)],
        _ => Vec::new(),
    }
}

/// Whether a node changes or reads custody: everything but the neutral
/// scalar, control and refusal forms.
fn custody_bearing(instruction: &LegacyInstruction) -> bool {
    use LegacyInstruction::*;
    match instruction {
        NoOp
        | Unknown { .. }
        | ForgetScalar { .. }
        | BranchOnNull { .. }
        | BranchOnValidity { .. }
        | BranchOnResult { .. }
        | BranchOnDistinct { .. }
        | CountedLoopHeader { .. }
        | CountedLoopStep { .. }
        | ArgumentReads { .. }
        | ScalarMemberStore { .. } => false,
        IndirectScalarAccess {
            borrowed_destination,
            frame,
            ..
        } => *borrowed_destination || frame.is_some(),
        ContractCall {
            destination,
            arguments,
            contract,
            nested,
            container,
            aggregate_reads,
            ..
        } => {
            destination.is_some()
                || !nested.is_empty()
                || container.is_some()
                || !aggregate_reads.is_empty()
                || !matches!(
                    contract.result,
                    qcore_schemas::contract::ResourceReturnMode::None
                )
                || arguments.iter().enumerate().any(|(position, argument)| {
                    argument.is_some()
                        && !contract.parameters.get(position).is_some_and(|mode| {
                            matches!(
                                mode,
                                qcore_schemas::contract::ResourceParameterMode::Borrowed
                            )
                        })
                })
        }
        _ => true,
    }
}

/// The nodes reachable from `start` (inclusive) in `cfg`.
fn reach(cfg: &LegacyCfg, start: usize) -> BTreeSet<usize> {
    let mut seen = BTreeSet::new();
    let mut work = vec![start];
    while let Some(node) = work.pop() {
        if !seen.insert(node) {
            continue;
        }
        work.extend(cfg.nodes[node].successors.iter().copied());
    }
    seen
}

/// The nodes one successor of `branch` reaches and another does not: the
/// region whose execution the branch decides (`QCore.StorageDuration.region`).
pub(crate) fn decided_region(cfg: &LegacyCfg, branch: usize) -> BTreeSet<usize> {
    let successors = &cfg.nodes[branch].successors;
    let reaches = successors
        .iter()
        .map(|&successor| reach(cfg, successor))
        .collect::<Vec<_>>();
    let mut region = BTreeSet::new();
    for (i, one) in reaches.iter().enumerate() {
        for (j, other) in reaches.iter().enumerate() {
            if i != j {
                region.extend(one.difference(other).copied());
            }
        }
    }
    region.remove(&branch);
    region
}

/// Hook (rule 11h), once per function over the built CFG: `volatile` objects
/// carry no value-derived fact, and a custody decision on a `volatile` read
/// refuses. `function` names the parity rows.
pub(super) fn settle_volatile(cfg: &mut LegacyCfg, facts: &FunctionFacts, function: &str) {
    let static_storage = cfg.publications.destinations.clone();
    for node in &mut cfg.nodes {
        let replacement = match &node.instruction {
            LegacyInstruction::Clear { destination } if facts.is_volatile(destination) => {
                debug_assert_eq!(decide(Form::VolatileValueFact), Disposition::Forgotten);
                Some((
                    "value-fact",
                    LegacyInstruction::ForgetScalar {
                        destination: destination.clone(),
                    },
                ))
            }
            LegacyInstruction::BranchOnNull { source, .. }
            | LegacyInstruction::BranchOnValidity { source, .. }
            | LegacyInstruction::BranchOnResult { source, .. }
                if facts.is_volatile(source) =>
            {
                debug_assert_eq!(decide(Form::VolatileRefinement), Disposition::Unrefined);
                Some(("refinement", LegacyInstruction::NoOp))
            }
            LegacyInstruction::BranchOnDistinct { left, right, .. }
                if facts.is_volatile(left) || facts.is_volatile(right) =>
            {
                Some(("refinement", LegacyInstruction::NoOp))
            }
            // A counted loop's index and bound facts are value-derived too.
            LegacyInstruction::CountedLoopHeader { index, bound }
                if facts.is_volatile(index)
                    || matches!(bound, crate::indexed_collection::Term::Var { name, .. }
                        if facts.is_volatile(name)) =>
            {
                Some(("refinement", LegacyInstruction::NoOp))
            }
            LegacyInstruction::CountedLoopStep { index } if facts.is_volatile(index) => Some((
                "value-fact",
                LegacyInstruction::ForgetScalar {
                    destination: index.clone(),
                },
            )),
            // A store into a static-storage object is rule 10's to decide
            // (a tracked identity is refused there as a publication); the
            // binding it leaves is kept from every refinement above.
            instruction => custody_names(instruction)
                .into_iter()
                .find(|(name, destination)| {
                    facts.is_volatile(name) && !(*destination && static_storage.contains(*name))
                })
                .map(|(name, _)| {
                    debug_assert_eq!(decide(Form::VolatileBinding), Disposition::Refused);
                    (
                        "binding",
                        LegacyInstruction::Unknown {
                            description: volatile_binding_refusal(
                                crate::storage_duration::root_name(name),
                            ),
                            writes: super::RefusedWrites::Unbounded,
                        },
                    )
                }),
        };
        if let Some((answer, instruction)) = replacement {
            record_row(json!({
                "function": function,
                "line": node.line,
                "position": "volatile-object",
                "answer": answer,
            }));
            node.instruction = instruction;
        }
    }
    for condition in &facts.volatile_conditions {
        let branches = (0..cfg.nodes.len())
            .filter(|&node| {
                cfg.nodes[node].successors.len() > 1
                    && condition.lines.contains(&cfg.nodes[node].line)
            })
            .collect::<Vec<_>>();
        let mut decided = false;
        for &branch in &branches {
            // A branch another rule already refused keeps its own refusal.
            if matches!(
                cfg.nodes[branch].instruction,
                LegacyInstruction::Unknown { .. }
            ) {
                continue;
            }
            let custody = decided_region(cfg, branch)
                .into_iter()
                .any(|node| custody_bearing(&cfg.nodes[node].instruction));
            if custody {
                debug_assert_eq!(decide(Form::VolatileDecision), Disposition::Refused);
                decided = true;
                cfg.nodes[branch].instruction = LegacyInstruction::Unknown {
                    description: VOLATILE_DECISION_REFUSAL.to_owned(),
                    writes: super::RefusedWrites::Unbounded,
                };
            }
        }
        record_row(json!({
            "function": function,
            "lines": condition.lines,
            "kind": condition.kind,
            "position": "volatile-condition",
            "answer": if branches.is_empty() {
                "no-branch-node"
            } else if decided {
                "custody-decision-refused"
            } else {
                "neutral"
            },
        }));
    }
}

#[cfg(test)]
mod exact_formal_tests {
    use super::*;
    #[test]
    fn exact_formal_identity_is_a_volatile_sensitive_read() {
        let mut cfg = LegacyCfg {
            function: "identity".into(),
            entry: 0,
            nodes: vec![super::super::LegacyCfgNode {
                lookup: None,
                lookup_read: None,
                instruction: LegacyInstruction::ExactFormalReturn {
                    source: "p".into(),
                    parameter: 0,
                },
                successors: vec![],
                line: 1,
                is_exit: true,
                loop_exit_edge: None,
            }],
            publications: Default::default(),
            conditional_names: Default::default(),
            tie_names: Default::default(),
            traversal: Default::default(),
            cell_tests: Default::default(),
        };
        let facts = FunctionFacts {
            volatile_objects: BTreeSet::from(["p".into()]),
            ..Default::default()
        };
        settle_volatile(&mut cfg, &facts, "identity");
        assert!(matches!(
            cfg.nodes[0].instruction,
            LegacyInstruction::Unknown { .. }
        ));
        assert!(cfg.nodes[0].is_exit);
    }
}
