//! Private context/rank extraction. Only proved scalar countdown arguments are
//! abstracted; source certificates and all resource effects remain unchanged.
use super::*;

/// Card **CG27-LOOPS-RECURSION-BUDGETS** (gateway rule 11x): the signed
/// threshold rank, consulted only for a component this module's own
/// [`prepare`] refuses.
#[path = "legacy_signed_rank.rs"]
pub(crate) mod signed_rank;

pub(crate) struct ContextFunction {
    pub(crate) parameters: Vec<Option<String>>,
    pub(crate) control_index: usize,
    control: String,
    unsigned: bool,
    allowed_control: BTreeSet<usize>,
    countdowns: BTreeSet<usize>,
    /// Card CG27 (rule 11x): the token range `start -> (length, true in the
    /// base context)` of each checked sign guard on the control. Empty on the
    /// zero/nonzero route, whose guards the CFG already reads as null tests.
    guards: BTreeMap<usize, (usize, bool)>,
    pub(crate) dependencies: BTreeSet<String>,
}

/// The checked signature of one component member: its formals, their type
/// words, the single integer control and the body's braces. Shared by the
/// zero/nonzero route and the signed threshold route (card CG27).
pub(crate) struct MemberHeader {
    pub(crate) parameters: Vec<Option<String>>,
    pub(crate) types: Vec<Vec<String>>,
    pub(crate) control_index: usize,
    pub(crate) control: String,
    pub(crate) body: usize,
    pub(crate) end: usize,
}

// Deliberately small ABI grammar: explicit built-in integer controls and flat
// pointer parameters. Typedefs, attributes, volatile/atomic controls and old-style
// declarations are not evidence for the exhaustive integer partition.
fn builtin_integer(words: &[String]) -> bool {
    matches!(
        words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["int"]
            | ["signed"]
            | ["unsigned"]
            | ["signed", "int"]
            | ["unsigned", "int"]
            | ["short"]
            | ["short", "int"]
            | ["unsigned", "short"]
            | ["unsigned", "short", "int"]
            | ["long"]
            | ["long", "int"]
            | ["unsigned", "long"]
            | ["unsigned", "long", "int"]
            | ["long", "long"]
            | ["long", "long", "int"]
            | ["unsigned", "long", "long"]
            | ["unsigned", "long", "long", "int"]
            | ["char"]
            | ["signed", "char"]
            | ["unsigned", "char"]
            | ["_Bool"]
    )
}

fn parameter_types(tokens: &[Token]) -> Option<Vec<Vec<String>>> {
    let names = parse_parameter_declarators(tokens)?;
    let groups = tokens.split(|token| token.text == ",").collect::<Vec<_>>();
    if names.len() != groups.len() {
        return None;
    }
    groups
        .iter()
        .zip(names)
        .map(|(group, name)| {
            let end = group.len().checked_sub(usize::from(name.is_some()))?;
            let words = group[..end]
                .iter()
                .map(|token| token.text.clone())
                .collect::<Vec<_>>();
            if words.is_empty()
                || words.iter().any(|word| {
                    !matches!(
                        word.as_str(),
                        "void"
                            | "char"
                            | "int"
                            | "short"
                            | "long"
                            | "signed"
                            | "unsigned"
                            | "_Bool"
                            | "*"
                            | "const"
                    )
                })
            {
                return None;
            }
            Some(words)
        })
        .collect()
}

/// The signature checks every ranked route shares: one built-in integer
/// control, every other formal a flat pointer, and every visible top-level
/// prototype agreeing with the definition. Loop and jump keywords anywhere in
/// the body refuse.
pub(crate) fn member_header(tokens: &[Token], function: &str) -> Option<MemberHeader> {
    let (body, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, body, "{", "}")?;
    let name_index = (0..body).rev().find(|&i| {
        tokens[i].text == *function && tokens.get(i + 1).is_some_and(|t| t.text == "(")
    })?;
    let close = matching(tokens, name_index + 1, "(", ")")?;
    let types = parameter_types(&tokens[name_index + 2..close])?;
    let parameters = function_parameters(tokens, function)?;
    let controls = types
        .iter()
        .enumerate()
        .filter(|(_, words)| builtin_integer(words))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if controls.len() != 1
        || types
            .iter()
            .enumerate()
            .any(|(i, words)| i != controls[0] && !words.contains(&"*".to_owned()))
    {
        return None;
    }
    let control_index = controls[0];
    let control = parameters[control_index].clone()?;
    // All visible top-level prototypes must agree with the definition. C11
    // checks this too; the reduced token entry point needs its own guard.
    let mut depth = 0usize;
    let mut declaration_start = 0usize;
    let mut signature = None;
    for i in 0..tokens.len() {
        if depth == 0 && tokens[i].text == *function {
            if tokens.get(i + 1)?.text != "(" {
                return None;
            }
            let last = matching(tokens, i + 1, "(", ")")?;
            if !matches!(tokens.get(last + 1)?.text.as_str(), ";" | "{")
                || parameter_types(&tokens[i + 2..last])? != types
            {
                return None;
            }
            let returns = tokens[declaration_start..i]
                .iter()
                .filter(|t| !matches!(t.text.as_str(), "extern" | "static" | "inline"))
                .map(|t| t.text.clone())
                .collect::<Vec<_>>();
            if returns.is_empty()
                || returns.iter().any(|t| {
                    !matches!(
                        t.as_str(),
                        "void"
                            | "*"
                            | "int"
                            | "char"
                            | "long"
                            | "short"
                            | "unsigned"
                            | "signed"
                            | "const"
                    )
                })
            {
                return None;
            }
            if signature.as_ref().is_some_and(|s| *s != returns) {
                return None;
            }
            signature = Some(returns);
        }
        match tokens[i].text.as_str() {
            "{" => depth += 1,
            "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    declaration_start = i + 1;
                }
            }
            ";" if depth == 0 => declaration_start = i + 1,
            _ => {}
        }
    }
    if (body + 1..end).any(|i| {
        matches!(
            tokens[i].text.as_str(),
            "while"
                | "for"
                | "do"
                | "switch"
                | "goto"
                | "case"
                | "default"
                | "break"
                | "continue"
                | ":"
        )
    }) {
        return None;
    }
    Some(MemberHeader {
        parameters,
        types,
        control_index,
        control,
        body,
        end,
    })
}

pub(crate) fn prepare(
    tokens: &[Token],
    component: &[String],
) -> Option<BTreeMap<String, ContextFunction>> {
    let mut result = BTreeMap::new();
    for function in component {
        let MemberHeader {
            parameters,
            types,
            control_index,
            control,
            body,
            end,
        } = member_header(tokens, function)?;
        let mut allowed = BTreeSet::new();
        for i in body + 1..end {
            if tokens[i].text == "if" {
                if tokens.get(i + 1)?.text != "(" {
                    return None;
                }
                let last = matching(tokens, i + 1, "(", ")")?;
                let condition = &tokens[i + 2..last];
                if condition.iter().any(|t| t.text == control) {
                    let parsed = pointer_null_condition(condition)?;
                    if parsed.source != control {
                        return None;
                    }
                    // Zero/nonzero only: no pointer NULL macros/casts or scalar
                    // operations admitted by a broader expression classifier.
                    if condition.iter().any(|t| {
                        !matches!(t.text.as_str(), "(" | ")" | "!" | "=" | "0") && t.text != control
                    }) {
                        return None;
                    }
                    allowed.extend(i + 2..last);
                }
            }
        }
        result.insert(
            function.clone(),
            ContextFunction {
                parameters,
                control_index,
                control,
                unsigned: types[control_index].iter().any(|word| word == "unsigned"),
                allowed_control: allowed,
                countdowns: BTreeSet::new(),
                guards: BTreeMap::new(),
                dependencies: BTreeSet::new(),
            },
        );
    }
    let unsigned_component = result.values().all(|descriptor| descriptor.unsigned);
    for function in component {
        let descriptor = &result[function];
        let (body, _) = find_function_body(tokens, function)?;
        let end = matching(tokens, body, "{", "}")?;
        let mut countdowns = BTreeSet::new();
        let mut allowed = descriptor.allowed_control.clone();
        let mut dependencies = BTreeSet::new();
        for i in body + 1..end {
            let Some(callee) = result.get(&tokens[i].text) else {
                continue;
            };
            if tokens.get(i + 1)?.text != "(" {
                return None;
            }
            let last = matching(tokens, i + 1, "(", ")")?;
            if tokens.get(last + 1)?.text != ";" {
                return None;
            }
            let actuals = tokens[i + 2..last]
                .split(|t| t.text == ",")
                .collect::<Vec<_>>();
            if actuals.len() != callee.parameters.len() {
                return None;
            }
            let counter = actuals[callee.control_index];
            if counter.len() == 3
                && counter[0].text == descriptor.control
                && counter[1].text == "-"
                && counter[2].text == "1"
                && unsigned_component
            {
                // On a nonzero unsigned input, integer promotion/subtraction
                // cannot underflow. Conversion to the callee's unsigned type
                // can only reduce the natural value further. The zero CFG is
                // separately checked with ALL component contracts unavailable,
                // so an underflowing zero-context call cannot establish a base.
                let start = i
                    + 2
                    + actuals[..callee.control_index]
                        .iter()
                        .map(|a| a.len() + 1)
                        .sum::<usize>();
                countdowns.insert(start);
                allowed.insert(start);
            } else if counter.len() != 1 || counter[0].text != "0" {
                return None;
            }
            if actuals.iter().enumerate().any(|(index, a)| {
                index != callee.control_index
                    && (a.len() != 1 || !(is_identifier(&a[0].text) || a[0].text == "0"))
            }) {
                return None;
            }
            // Only standalone calls or the exact RHS of an assignment. Nested
            // calls, return expressions and function-valued aliases reject.
            if !matches!(
                tokens[i - 1].text.as_str(),
                "{" | "}" | ";" | "=" | ")" | "else"
            ) {
                return None;
            }
            dependencies.insert(tokens[i].text.clone());
        }
        if (body + 1..end).any(|i| tokens[i].text == descriptor.control && !allowed.contains(&i)) {
            return None;
        }
        let descriptor = result.get_mut(function)?;
        descriptor.dependencies = dependencies;
        descriptor.countdowns = countdowns;
    }
    Some(result)
}

pub(crate) fn contextual_cfg(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    descriptor: &ContextFunction,
    zero: bool,
) -> Option<LegacyCfg> {
    contextual_cfg_under(tokens, function, environment, descriptor, zero, None)
}

/// [`contextual_cfg`] with `own` read as the function's own contract where
/// the body's construction reads it (card
/// W47-SUMMARY-VALIDATION-PROTECTED-FORMALS); calls still resolve through
/// `environment` alone.
pub(crate) fn contextual_cfg_under(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    descriptor: &ContextFunction,
    zero: bool,
    own: Option<&ResourceFunctionContract>,
) -> Option<LegacyCfg> {
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f).
    crate::contract_dependencies::within(function, || {
        contextual_cfg_unscoped(tokens, function, environment, descriptor, zero, own)
    })
}

fn contextual_cfg_unscoped(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    descriptor: &ContextFunction,
    zero: bool,
    own: Option<&ResourceFunctionContract>,
) -> Option<LegacyCfg> {
    // A checked summary borrows the scalar control and is identical at every
    // rank. Erase only its proved `control - 1` actual to a scalar placeholder
    // for the resource CFG; this is NOT a claim that its runtime value is zero.
    // The nonzero proof uses strong induction over unsigned values. Original
    // source remains in the certificate and this exact extraction is re-run.
    let mut abstracted = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        // Card CG27 (rule 11x): a checked sign guard is respelled as the null
        // test whose truth agrees with it in each context (`CONTROL == 0` is
        // true exactly in the base context), so the selection below decides
        // it. The original guard stays in the certificate source.
        if let Some(&(length, true_in_base)) = descriptor.guards.get(&index) {
            let line = tokens[index].line;
            let spelled: [&str; 4] = if true_in_base {
                [descriptor.control.as_str(), "=", "=", "0"]
            } else {
                [descriptor.control.as_str(), "!", "=", "0"]
            };
            abstracted.extend(spelled.iter().map(|text| Token {
                text: (*text).to_owned(),
                line,
            }));
            index += length;
        } else if descriptor.countdowns.contains(&index) {
            abstracted.push(Token {
                text: "0".to_owned(),
                line: tokens[index].line,
            });
            index += 3;
        } else {
            abstracted.push(tokens[index].clone());
            index += 1;
        }
    }
    let mut cfg = build_function_cfg_with(&abstracted, function, environment, false, own).ok()?;
    for node in &mut cfg.nodes {
        if let LegacyInstruction::BranchOnNull {
            source,
            true_is_null,
        } = &node.instruction
            && *source == descriptor.control
        {
            if node.successors.len() != 2 {
                return None;
            }
            let chosen = node.successors[usize::from(*true_is_null != zero)];
            node.instruction = LegacyInstruction::NoOp;
            node.successors = vec![chosen];
        }
    }
    // Remove only graph-unreachable nodes after selecting the immutable
    // context guards. This preserves every potentially reachable instruction
    // and keeps certificate containment independent of analyzer reachability.
    let mut reachable = BTreeSet::new();
    let mut pending = vec![cfg.entry];
    while let Some(index) = pending.pop() {
        if reachable.insert(index) {
            pending.extend(cfg.nodes.get(index)?.successors.iter().copied());
        }
    }
    let indices = reachable
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new))
        .collect::<BTreeMap<_, _>>();
    cfg.entry = *indices.get(&cfg.entry)?;
    cfg.nodes = cfg
        .nodes
        .into_iter()
        .enumerate()
        .filter_map(|(index, mut node)| {
            if !reachable.contains(&index) {
                return None;
            }
            for successor in &mut node.successors {
                *successor = indices[successor];
            }
            Some(node)
        })
        .collect();
    // The certificate firewall rejects even abstract-state-infeasible Unknown
    // nodes. Do not publish a contextual certificate that relies on deleting
    // an additional path using a custody precondition rather than its context.
    if cfg
        .nodes
        .iter()
        .any(|node| matches!(node.instruction, LegacyInstruction::Unknown { .. }))
    {
        return None;
    }
    Some(cfg)
}

pub(crate) fn combine(mut zero: LegacyCfg, mut nonzero: LegacyCfg) -> LegacyCfg {
    // Contextually combined recursive bodies are outside the acyclic local
    // provenance profile, even if a specialized CFG has no syntactic cycle.
    zero.conditional_names.clear();
    // Card SUP07-REFUSED-TEST-ACCUSATION: nor rule 12e's.
    zero.tie_names.clear();
    // Keep disjoint allocation identities across the mutually exclusive CFGs.
    let site_offset = zero
        .nodes
        .iter()
        .flat_map(|n| super::instruction_produced_sites(&n.instruction))
        .max()
        .map_or(0, |site| site + 1);
    let node_offset = zero.nodes.len();
    for node in &mut nonzero.nodes {
        for successor in &mut node.successors {
            *successor += node_offset;
        }
        super::offset_produced_sites(&mut node.instruction, site_offset);
    }
    // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): each half's guard tests
    // stay on their own branch nodes.
    let shifted = std::mem::take(&mut nonzero.cell_tests)
        .into_iter()
        .map(|(node, step)| (node + node_offset, step))
        .collect::<Vec<_>>();
    zero.cell_tests.extend(shifted);
    let entries = vec![zero.entry, nonzero.entry + node_offset];
    // Both contexts are built from the same body, so their CTWIN-PUBLISH
    // destination sets agree; the union is taken rather than assumed so that a
    // future context whose set differs cannot silently lose one half's
    // refusals.
    // CTWIN-RESIDUALS: one grouped record now, unioned in both directions by
    // `absorb`, so a destination, a scalar name or a recorded line from either
    // context survives into the combination.
    zero.publications
        .absorb(std::mem::take(&mut nonzero.publications));
    // Card W43-INDEX-TRAVERSAL: the traversal names only *admit* (a joined
    // or copied fact becomes no fact), so a combination keeps a name only when
    // both contexts declared it the same way. They are built from one body, so
    // this is the identity in practice and the fail-closed answer otherwise.
    zero.traversal.scalars = zero
        .traversal
        .scalars
        .intersection(&nonzero.traversal.scalars)
        .cloned()
        .collect();
    zero.traversal.pointers = zero
        .traversal
        .pointers
        .intersection(&nonzero.traversal.pointers)
        .cloned()
        .collect();
    zero.nodes.extend(nonzero.nodes);
    zero.entry = zero.nodes.len();
    zero.nodes.push(LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction: LegacyInstruction::NoOp,
        successors: entries,
        line: 0,
        is_exit: false,
        loop_exit_edge: None,
    });
    zero
}
