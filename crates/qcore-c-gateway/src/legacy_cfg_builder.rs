//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the CFG builder: parsed
//! statements compiled to `LegacyCfg` nodes, with each condition's compile
//! precedence (ROUTE-MAP section 3).
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

pub(super) struct CfgBuilder {
    pub(super) nodes: Vec<LegacyCfgNode>,
    pub(super) labels: BTreeMap<String, LegacyNodeId>,
    pub(super) pending_gotos: Vec<(LegacyNodeId, String, LegacyNodeId)>,
    pub(super) break_targets: Vec<LegacyNodeId>,
    pub(super) continue_targets: Vec<LegacyNodeId>,
    pub(super) switch_cases: Vec<SwitchTargets>,
    /// Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): the branch nodes whose
    /// condition is a declared cell protocol's guard test.
    pub(super) cell_tests: BTreeMap<LegacyNodeId, descriptor_cells_route::ProtocolStep>,
}

#[derive(Default)]
pub(super) struct SwitchTargets {
    pub(super) entries: Vec<LegacyNodeId>,
    pub(super) has_default: bool,
}

impl CfgBuilder {
    pub(super) fn build(
        function: &str,
        statements: &[Statement],
        publications: PublicationFacts,
    ) -> LegacyCfg {
        let mut builder = Self {
            nodes: Vec::new(),
            labels: BTreeMap::new(),
            pending_gotos: Vec::new(),
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            switch_cases: Vec::new(),
            cell_tests: BTreeMap::new(),
        };
        let exit = builder.push(LegacyInstruction::NoOp, Vec::new(), 0, true);
        let entry = builder.compile_sequence(statements, exit);
        for (node, label, fallback) in std::mem::take(&mut builder.pending_gotos) {
            if let Some(target) = builder.labels.get(&label).copied() {
                builder.nodes[node].successors = vec![target];
            } else {
                builder.nodes[node].instruction = LegacyInstruction::Unknown {
                    description: format!("goto to unresolved label `{label}`"),
                    writes: RefusedWrites::Unbounded,
                };
                builder.nodes[node].successors = vec![fallback];
            }
        }
        LegacyCfg {
            function: function.to_owned(),
            entry,
            nodes: builder.nodes,
            publications,
            conditional_names: BTreeSet::new(),
            tie_names: BTreeSet::new(),
            traversal: TraversalNames::default(),
            cell_tests: builder.cell_tests,
        }
    }

    pub(super) fn compile_sequence(
        &mut self,
        statements: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        statements
            .iter()
            .rev()
            .fold(continuation, |next, statement| {
                self.compile(statement, next)
            })
    }

    pub(super) fn compile(
        &mut self,
        statement: &Statement,
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        match statement {
            Statement::Instruction(instruction, line) => {
                self.push(instruction.clone(), vec![continuation], *line, false)
            }
            // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o).
            Statement::PathEnd(instruction, line) => {
                self.push(instruction.clone(), Vec::new(), *line, false)
            }
            Statement::Block(statements) => self.compile_sequence(statements, continuation),
            Statement::Return {
                line,
                writes,
                expression_supported,
                returned_place,
                returned_evaluation,
                returned_evaluation_escapes,
                returns_null_constant,
            } => {
                // Card C-GATEWAY-RETURN-CALL-W4. `return f(args);` becomes the
                // two nodes `PLACE = f(args);` and `return PLACE;` — the same C,
                // and the pair the analyzer has decided since Cycle 9. The call
                // node is an ordinary interior node; the exit is an ordinary
                // `EscapeIfOwned`, so `observe_exit` needs no arm for a call
                // result.
                if let Some(call) = returned_evaluation {
                    if matches!(**call, LegacyInstruction::ExactFormalReturn { .. }) {
                        return self.push((**call).clone(), Vec::new(), *line, true);
                    }
                    let exit_instruction = if *returned_evaluation_escapes {
                        LegacyInstruction::EscapeIfOwned {
                            source: return_call::RETURN_RESULT_PLACE.to_owned(),
                        }
                    } else {
                        LegacyInstruction::NoOp
                    };
                    let exit = self.push(exit_instruction, Vec::new(), *line, true);
                    return self.push((**call).clone(), vec![exit], *line, false);
                }
                let instruction = if !*expression_supported {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in return expression"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                } else if let Some(source) = returned_place {
                    LegacyInstruction::EscapeIfOwned {
                        source: source.clone(),
                    }
                } else if *returns_null_constant {
                    // Card W43-RESULT-ALIAS-RETURNED. The place can never be
                    // bound, so the transfer is the `NoOp` it replaces; only
                    // `observe_exit` reads the name.
                    LegacyInstruction::EscapeIfOwned {
                        source: return_call::NULL_RETURN_PLACE.to_owned(),
                    }
                } else {
                    LegacyInstruction::NoOp
                };
                self.push(instruction, Vec::new(), *line, true)
            }
            Statement::If {
                line,
                writes,
                condition_supported,
                condition_call,
                condition_access,
                null_condition,
                distinct_condition,
                element_condition,
                condition_plan,
                cell_test,
                then_body,
                else_body,
            } => {
                let then_entry = self.compile_sequence(then_body, continuation);
                let else_entry = self.compile_sequence(else_body, continuation);
                // Card W44-BRANCH-CONDITION-CALLS: present only where the arm
                // below would have pushed the refusal.
                if let Some(plan) = condition_plan {
                    // The plan's nodes are pushed in order from here.
                    let first = self.nodes.len();
                    let entry = self.compile_planned_if(plan, *line, then_entry, else_entry);
                    // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): a guard
                    // test is read on the plan's one node that branches to
                    // both exits (none when there is not exactly one).
                    if let Some(step) = cell_test {
                        use condition_desugar::Target;
                        let deciding = plan
                            .nodes
                            .iter()
                            .enumerate()
                            .filter(|(_, node)| {
                                node.successors
                                    .iter()
                                    .any(|t| matches!(t, Target::True | Target::False))
                            })
                            .collect::<Vec<_>>();
                        if let [(index, node)] = deciding.as_slice() {
                            match node.successors.as_slice() {
                                [Target::True, Target::False] => {
                                    self.cell_tests.insert(first + index, (**step).clone());
                                }
                                [Target::False, Target::True] => {
                                    self.cell_tests.insert(first + index, step.flipped());
                                }
                                _ => {}
                            }
                        }
                    }
                    return entry;
                }
                let instruction = if let Some((base, index, true_is_null)) = element_condition {
                    // W43-INDEXED-RESOURCE-COLLECTIONS: the condition was
                    // refused (see its parse), so the fallback is exactly the
                    // refusal the arm below would have pushed.
                    LegacyInstruction::BranchOnElementNull {
                        base: base.clone(),
                        index: index.clone(),
                        true_is_null: *true_is_null,
                        fallback: Box::new(LegacyInstruction::Unknown {
                            description:
                                "unmodeled call, aggregate or indirect access in branch condition"
                                    .to_owned(),
                            writes: writes.clone(),
                        }),
                    }
                } else if let Some(condition) = null_condition {
                    condition.branch()
                } else if let Some(condition) = distinct_condition {
                    // C02-P-ALIAS. A supported neutral condition would have
                    // become `NoOp`; this carries the same absence of custody
                    // effect and additionally lets the distinct edge discharge
                    // one may-alias obligation.
                    LegacyInstruction::BranchOnDistinct {
                        left: condition.left.clone(),
                        right: condition.right.clone(),
                        true_is_distinct: condition.true_is_distinct,
                    }
                } else if *condition_supported || condition_call.is_some() {
                    LegacyInstruction::NoOp
                } else if let Some(access) = condition_access {
                    // Card W43-INDEX-TRAVERSAL: the typed read, with each
                    // base's dereference obligation at the branch.
                    (**access).clone()
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in branch condition"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                let branch = self.push(instruction, vec![then_entry, else_entry], *line, false);
                // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): the branch's
                // edges read the protocol test; its instruction is unchanged.
                if let Some(step) = cell_test {
                    self.cell_tests.insert(branch, (**step).clone());
                }
                condition_call.as_ref().map_or(branch, |call| {
                    self.push((**call).clone(), vec![branch], *line, false)
                })
            }
            Statement::While {
                line,
                writes,
                condition_supported,
                condition_call,
                condition_access,
                null_condition,
                condition_plan,
                body,
            } => {
                // Card W44-BRANCH-CONDITION-CALLS.
                if let Some(plan) = condition_plan {
                    return self.compile_planned_while(plan, *line, body, continuation);
                }
                let instruction = if let Some(condition) = null_condition {
                    condition.branch()
                } else if let Some(call) = condition_call {
                    (**call).clone()
                } else if *condition_supported {
                    LegacyInstruction::NoOp
                } else if let Some(access) = condition_access {
                    (**access).clone()
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in loop condition"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                let header = self.push(instruction, Vec::new(), *line, false);
                self.break_targets.push(continuation);
                self.continue_targets.push(header);
                let body_entry = self.compile_sequence(body, header);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = vec![body_entry, continuation];
                // SCALAR-ROUTE-EXIT-EDGE: index 1 is the successor that leaves
                // the loop.
                self.nodes[header].loop_exit_edge = Some(1);
                header
            }
            Statement::For {
                line,
                counted,
                writes,
                init,
                condition_always_true,
                condition_supported,
                condition_call,
                condition_access,
                null_condition,
                condition_plan,
                increment,
                list_step,
                body,
            } => {
                // Card W44-BRANCH-CONDITION-CALLS. A counted loop requires a
                // supported condition, so it never carries a plan. Card
                // W45-FB-BACKLOG-CUSTODY: neither does a list iteration, whose
                // condition is supported, so a plan never has a `list_step`.
                debug_assert!(condition_plan.is_none() || list_step.is_none());
                if let Some(plan) = condition_plan {
                    return self.compile_planned_for(
                        plan,
                        *line,
                        init.as_deref(),
                        increment.as_deref(),
                        body,
                        continuation,
                    );
                }
                let instruction = condition_call.as_ref().map_or_else(
                    || {
                        branch_instruction(
                            *condition_supported,
                            null_condition,
                            condition_access.as_deref(),
                            writes,
                            "unmodeled call, aggregate or indirect access in for-loop condition",
                        )
                    },
                    |call| (**call).clone(),
                );
                // W43-INDEXED-RESOURCE-COLLECTIONS: a recognised counted loop's
                // header and step replace exactly a `NoOp` and a forget of the
                // index (see `counted_loop`), with the same effect on every map
                // but the collections.
                let instruction = match counted {
                    Some((index, bound)) => LegacyInstruction::CountedLoopHeader {
                        index: index.clone(),
                        bound: bound.clone(),
                    },
                    None => instruction,
                };
                let header = self.push(instruction, Vec::new(), *line, false);
                // Card W45-FB-BACKLOG-CUSTODY: the second increment node, when
                // there is one, leads to the header; the first leads to it.
                let step_target = list_step.as_ref().map_or(header, |clause| {
                    let (instruction, line) = clause.as_ref();
                    self.push(instruction.clone(), vec![header], *line, false)
                });
                let increment_entry = increment.as_ref().map_or(step_target, |clause| {
                    let (instruction, line) = clause.as_ref();
                    let instruction = match counted {
                        Some((index, _)) => LegacyInstruction::CountedLoopStep {
                            index: index.clone(),
                        },
                        None => instruction.clone(),
                    };
                    self.push(instruction, vec![step_target], *line, false)
                });
                self.break_targets.push(continuation);
                self.continue_targets.push(increment_entry);
                let body_entry = self.compile_sequence(body, increment_entry);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = if *condition_always_true {
                    vec![body_entry]
                } else {
                    vec![body_entry, continuation]
                };
                // SCALAR-ROUTE-EXIT-EDGE. A `for (;;)` header has no exit
                // successor to protect — that route removes it at construction
                // and is a different, unowned neighbour
                // (`FOREVER-FOR-EXIT-EDGE-UNOWNED`), deliberately untouched.
                self.nodes[header].loop_exit_edge = (!*condition_always_true).then_some(1);
                init.as_ref().map_or(header, |clause| {
                    let (instruction, line) = clause.as_ref();
                    self.push(instruction.clone(), vec![header], *line, false)
                })
            }
            Statement::DoWhile {
                line,
                writes,
                condition_supported,
                condition_call,
                condition_access,
                condition_is_literal_zero,
                null_condition,
                condition_plan,
                body,
            } => {
                // Card W44-BRANCH-CONDITION-CALLS. A call-bearing condition is
                // never the literal `0`, so the constant rule never applies.
                if let Some(plan) = condition_plan {
                    return self.compile_planned_do_while(plan, *line, body, continuation);
                }
                let instruction = condition_call.as_ref().map_or_else(
                    || {
                        branch_instruction(
                            *condition_supported,
                            null_condition,
                            condition_access.as_deref(),
                            writes,
                            "unmodeled call, aggregate or indirect access in do-while condition",
                        )
                    },
                    |call| (**call).clone(),
                );
                let header = self.push(instruction, Vec::new(), *line, false);
                self.break_targets.push(continuation);
                self.continue_targets.push(header);
                let body_entry = self.compile_sequence(body, header);
                self.continue_targets.pop();
                self.break_targets.pop();
                // `do S while (0)` executes S exactly once (C11 §6.8.5.2), so
                // the controlling expression has the single successor
                // `continuation`: the CFG is sequential composition, and the
                // back edge to `body_entry` would be a path the C does not
                // have. Removing it is the admitting direction (invariant 1),
                // which is why the key is the literal token and the rule is
                // proved in `QCore.ConstantDoWhile`. A `continue` in the body
                // still targets `header`, which is exactly C's "jump to the
                // controlling expression", and so now exits.
                self.nodes[header].successors = if *condition_is_literal_zero {
                    vec![continuation]
                } else {
                    vec![body_entry, continuation]
                };
                // SCALAR-ROUTE-EXIT-EDGE. Under card
                // `C-GATEWAY-CONSTANT-DO-WHILE`'s rule the continuation is the
                // header's only successor, so it is index 0; otherwise index 1,
                // as for `while`. Either way it is the edge that leaves.
                self.nodes[header].loop_exit_edge = Some(usize::from(!*condition_is_literal_zero));
                body_entry
            }
            Statement::Switch {
                line,
                writes,
                expression_supported,
                expression_call,
                expression_access,
                expression_plan,
                body,
            } => {
                // Card W44-BRANCH-CONDITION-CALLS.
                if let Some(plan) = expression_plan {
                    return self.compile_planned_switch(plan, *line, body, continuation);
                }
                self.break_targets.push(continuation);
                self.switch_cases.push(SwitchTargets::default());
                let _body_entry = self.compile_sequence(body, continuation);
                let targets = self.switch_cases.pop().expect("switch target scope");
                self.break_targets.pop();
                let mut successors = targets.entries;
                if !targets.has_default || successors.is_empty() {
                    successors.push(continuation);
                }
                successors.sort_unstable();
                successors.dedup();
                let instruction = if let Some(call) = expression_call {
                    (**call).clone()
                } else if *expression_supported {
                    LegacyInstruction::NoOp
                } else if let Some(access) = expression_access {
                    (**access).clone()
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in switch expression"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                self.push(instruction, successors, *line, false)
            }
            Statement::Case {
                line,
                writes,
                expression_supported,
                is_default,
            } => {
                let instruction = if *expression_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description:
                            "unmodeled call, aggregate or indirect access in case expression"
                                .to_owned(),
                        writes: writes.clone(),
                    }
                };
                let entry = self.push(instruction, vec![continuation], *line, false);
                if let Some(targets) = self.switch_cases.last_mut() {
                    targets.entries.push(entry);
                    targets.has_default |= *is_default;
                } else {
                    self.nodes[entry].instruction = LegacyInstruction::Unknown {
                        description: "case label outside switch".to_owned(),
                        writes: RefusedWrites::Unbounded,
                    };
                }
                entry
            }
            Statement::Label { line, name } => {
                let entry = self.push(LegacyInstruction::NoOp, vec![continuation], *line, false);
                if self.labels.insert(name.clone(), entry).is_some() {
                    self.nodes[entry].instruction = LegacyInstruction::Unknown {
                        description: format!("duplicate label `{name}`"),
                        writes: RefusedWrites::Unbounded,
                    };
                }
                entry
            }
            Statement::Goto { line, label } => {
                let node = self.push(LegacyInstruction::NoOp, Vec::new(), *line, false);
                if let Some(target) = self.labels.get(label).copied() {
                    self.nodes[node].successors = vec![target];
                } else {
                    self.pending_gotos.push((node, label.clone(), continuation));
                }
                node
            }
            Statement::Break { line } => {
                if let Some(target) = self.break_targets.last().copied() {
                    self.push(LegacyInstruction::NoOp, vec![target], *line, false)
                } else {
                    self.push(
                        LegacyInstruction::Unknown {
                            description: "break outside loop or switch".to_owned(),
                            writes: RefusedWrites::Unbounded,
                        },
                        vec![continuation],
                        *line,
                        false,
                    )
                }
            }
            Statement::Continue { line } => {
                if let Some(target) = self.continue_targets.last().copied() {
                    self.push(LegacyInstruction::NoOp, vec![target], *line, false)
                } else {
                    self.push(
                        LegacyInstruction::Unknown {
                            description: "continue outside loop".to_owned(),
                            writes: RefusedWrites::Unbounded,
                        },
                        vec![continuation],
                        *line,
                        false,
                    )
                }
            }
        }
    }

    pub(super) fn push(
        &mut self,
        instruction: LegacyInstruction,
        successors: Vec<LegacyNodeId>,
        line: usize,
        is_exit: bool,
    ) -> LegacyNodeId {
        let id = self.nodes.len();
        self.nodes.push(LegacyCfgNode {
            lookup: None,
            lookup_read: None,
            instruction,
            successors,
            line,
            is_exit,
            // Set only by the three loop headers, where the successors are
            // assigned after the body is compiled.
            loop_exit_edge: None,
        });
        id
    }
}

pub(super) fn branch_instruction(
    condition_supported: bool,
    null_condition: &Option<PointerNullCondition>,
    condition_access: Option<&LegacyInstruction>,
    writes: &RefusedWrites,
    unsupported_description: &str,
) -> LegacyInstruction {
    if let Some(condition) = null_condition {
        condition.branch()
    } else if condition_supported {
        LegacyInstruction::NoOp
    } else if let Some(access) = condition_access {
        // Card W43-INDEX-TRAVERSAL: the typed read of a refused condition.
        access.clone()
    } else {
        LegacyInstruction::Unknown {
            description: unsupported_description.to_owned(),
            writes: writes.clone(),
        }
    }
}

// `pub(crate)` since card HYGIENE-14 so `borrow_liveness` classifies the
// analyzer's own status bits rather than restating their numeric values. The
// values, the join and every use here are unchanged.
pub(crate) const LIVE: u8 = 1;
pub(crate) const RELEASED: u8 = 2;
pub(crate) const ESCAPED: u8 = 4;
pub(crate) const CONTAINED: u8 = 8;
