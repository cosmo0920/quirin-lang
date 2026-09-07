//! Private context/rank extraction. Only proved scalar countdown arguments are
//! abstracted; source certificates and all resource effects remain unchanged.
use super::*;

pub(crate) struct ContextFunction {
    pub(crate) parameters: Vec<Option<String>>,
    pub(crate) control_index: usize,
    control: String,
    unsigned: bool,
    allowed_control: BTreeSet<usize>,
    countdowns: BTreeSet<usize>,
    pub(crate) dependencies: BTreeSet<String>,
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

pub(crate) fn prepare(
    tokens: &[Token],
    component: &[String],
) -> Option<BTreeMap<String, ContextFunction>> {
    let mut result = BTreeMap::new();
    for function in component {
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
        let mut allowed = BTreeSet::new();
        for i in body + 1..end {
            if matches!(
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
            ) {
                return None;
            }
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
    // A checked summary borrows the scalar control and is identical at every
    // rank. Erase only its proved `control - 1` actual to a scalar placeholder
    // for the resource CFG; this is NOT a claim that its runtime value is zero.
    // The nonzero proof uses strong induction over unsigned values. Original
    // source remains in the certificate and this exact extraction is re-run.
    let mut abstracted = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        if descriptor.countdowns.contains(&index) {
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
    let mut cfg = build_function_cfg(&abstracted, function, environment).ok()?;
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
    // Keep disjoint allocation identities across the mutually exclusive CFGs.
    let site_offset = zero
        .nodes
        .iter()
        .flat_map(|n| match &n.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![*site],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        })
        .max()
        .map_or(0, |site| site + 1);
    let node_offset = zero.nodes.len();
    for node in &mut nonzero.nodes {
        for successor in &mut node.successors {
            *successor += node_offset;
        }
        match &mut node.instruction {
            LegacyInstruction::Acquire { site, .. } => *site += site_offset,
            LegacyInstruction::ContractCall { produced_sites, .. } => {
                for site in produced_sites {
                    *site += site_offset;
                }
            }
            _ => {}
        }
    }
    let entries = vec![zero.entry, nonzero.entry + node_offset];
    zero.nodes.extend(nonzero.nodes);
    zero.entry = zero.nodes.len();
    zero.nodes.push(LegacyCfgNode {
        instruction: LegacyInstruction::NoOp,
        successors: entries,
        line: 0,
        is_exit: false,
    });
    zero
}
