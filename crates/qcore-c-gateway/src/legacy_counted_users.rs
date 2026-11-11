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

//! Card **W45-FB-RETRY-LIFETIME**: counted users (contract specification
//! §C04.7, gateway constraint 10e).
//!
//! A *counted user* is a reference an object holds on itself through a counter
//! member (`task->users`): an addition is an obligation, a drop discharges it,
//! and a drop without the release check leaves the object possibly
//! unreferenced until some caller runs the check. The resource projection does
//! not see any of this — the counter is a scalar — so a failure path that adds
//! a user and forgets to drop it was never accused (C2), and a path that drops
//! the last user without the check (C3) never either.
//!
//! This pass reads the declarations of the counter operations and of the
//! functions whose exits are specified, and checks each function the unit
//! reports:
//!
//! * **Operations.** A call whose contract in force declares
//!   `resource_user_added|dropped|dropped_checked|release_check(p, COUNTER)`
//!   applies that operation to the object its actual at `p` designates. A
//!   direct write to `B->COUNTER` (`++`, `--`) is the same operation. The
//!   operation is **attributed** only when that actual is a formal of this
//!   function that no statement rebinds; any other operation is unattributed,
//!   and every tracked count becomes undecided.
//! * **Exit clauses.** `resource_user_exit(p, R, D)` states that every return
//!   of the value `R` has added `D` users to `p`'s object. A caller applies it,
//!   forking on the value when the result is stored in a local that a later
//!   condition compares; the body defined here is checked against it.
//! * **Neutrality.** A call adds, drops and checks nothing when its contract
//!   says so (`resource_user_neutral()`), when its body is defined in this unit
//!   and, recursively, writes no counter and calls nothing that is not
//!   neutral, or when its contract in force is a declaration with no counted-
//!   user clause and no body is defined here (premise **UC-FRAME**: a
//!   declaration is complete for the counted-user vocabulary). A call with no
//!   contract and no body, or through an expression, is **opaque**: every
//!   tracked count becomes undecided.
//!
//! Every path of a body is enumerated (branches whose condition this pass
//! decides are pruned: a comparison of a stored call result or of a formal a
//! literal premise fixes). A `switch`, `goto` or label leaves every clause of
//! the function undecided. A loop does too, unless card CG27's loop summary
//! ([`loops`], gateway rule 11x) checks that no token of it performs a
//! counted-user operation, in which case it leaves every count as it found
//! it. At each exit the net count of every tracked formal is compared with the
//! clauses:
//!
//! * a known count that contradicts a clause on an exit that definitely
//!   returns the clause's value **refutes** it: a provisional `Unsafe`
//!   candidate, never an upstream defect, because branch feasibility beyond
//!   what is decided here is not proved;
//! * an undecided count, or an exit whose value is not decided, leaves it
//!   **undecided** (`Unknown`, with the blocker);
//! * otherwise it **holds** for that function under this pass's premises.
//!
//! An exit no clause covers must restore every tracked count; a known nonzero
//! count there is an undeclared residual, and an unchecked drop that reaches
//! an exit without a re-addition or a release check is the C3 shape. Both are
//! violation candidates.
//!
//! The Lean model is `QCore.CountedUsers`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, UserEffect, UserExitClause,
};

use super::{
    LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionSummary, Token, find_function_body,
    function_parameters, is_identifier, literal_premise, matching,
};

/// Card **CG27-LOOPS-RECURSION-BUDGETS** (gateway rule 11x): the loop
/// summary whose invariant is that every count stays as the loop found it.
#[path = "legacy_counted_loops.rs"]
mod loops;

/// The per-clause result the audit report publishes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClauseRecord {
    /// The clause, as a declaration spells it, or `implicit: …` for the
    /// obligation every exit no clause covers carries.
    pub clause: String,
    pub verdict: ClauseVerdict,
    /// Input line of the exit that decides a refutation, or 0.
    pub line: usize,
    /// Why the clause is undecided, or what refutes it.
    pub reason: String,
}

/// A clause's verdict under this pass's premises.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClauseVerdict {
    Held,
    Refuted,
    Undecided,
}

impl ClauseVerdict {
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Refuted => "refuted",
            Self::Undecided => "undecided",
        }
    }
}

/// The most paths one body is enumerated along. Beyond it every clause of the
/// function is undecided.
const PATH_BOUND: usize = 512;

/// A net count, or why it is not known.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Count {
    Known(i32),
    Undecided(String),
}

/// What a scalar local may hold on one path.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    OneOf(BTreeSet<i64>),
    NoneOf(BTreeSet<i64>),
}

impl Value {
    fn may_be(&self, value: i64) -> bool {
        match self {
            Self::OneOf(values) => values.contains(&value),
            Self::NoneOf(values) => !values.contains(&value),
        }
    }

    fn is_exactly(&self, value: i64) -> bool {
        matches!(self, Self::OneOf(values) if values.len() == 1 && values.contains(&value))
    }
}

#[derive(Clone, Debug, Default)]
struct PathState {
    counts: BTreeMap<String, Count>,
    /// Formals dropped without a release check and not re-added since, with
    /// the line of the drop.
    pending: BTreeMap<String, usize>,
    /// Formals a release check may have destroyed.
    checked: BTreeSet<String>,
    values: BTreeMap<String, Value>,
}

impl PathState {
    fn undecide_all(&mut self, reason: &str) {
        for count in self.counts.values_mut() {
            if matches!(count, Count::Known(_)) {
                *count = Count::Undecided(reason.to_owned());
            }
        }
        self.pending.clear();
    }

    fn apply(&mut self, formal: &str, effect: UserEffect, line: usize, what: &str) {
        let Some(count) = self.counts.get_mut(formal) else {
            return;
        };
        if self.checked.contains(formal) {
            *count = Count::Undecided(format!(
                "{what} after a release check that may have destroyed `{formal}`"
            ));
            return;
        }
        let step = match effect {
            UserEffect::Added => 1,
            UserEffect::Dropped | UserEffect::DroppedChecked => -1,
            UserEffect::ReleaseCheck => 0,
        };
        if let Count::Known(value) = count {
            *value += step;
        }
        match effect {
            UserEffect::Added => {
                self.pending.remove(formal);
            }
            // A drop that only rolls back an addition this path made leaves
            // the object as it was at entry, so it owes no release check of
            // its own; a drop below the entry count does.
            UserEffect::Dropped => {
                if !matches!(self.counts.get(formal), Some(Count::Known(value)) if *value >= 0) {
                    self.pending.insert(formal.to_owned(), line);
                }
            }
            UserEffect::DroppedChecked | UserEffect::ReleaseCheck => {
                self.pending.remove(formal);
                self.checked.insert(formal.to_owned());
            }
        }
    }
}

/// The value a `return` hands back.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Returned {
    Void,
    Value(Value),
    Undecided,
}

/// What one path leaves at one exit.
#[derive(Clone, Debug)]
struct Exit {
    line: usize,
    returned: Returned,
    state: PathState,
}

/// One statement of the body.
#[derive(Clone, Debug)]
enum Stmt {
    Block(Vec<Stmt>),
    If {
        condition: (usize, usize),
        then: Box<Stmt>,
        otherwise: Option<Box<Stmt>>,
    },
    Return {
        expression: (usize, usize),
        line: usize,
    },
    Expression((usize, usize)),
    /// Card CG27: a `while`, `for` or `do` (other than `do … while (0)`)
    /// whose extent was read. The walker summarizes it only when [`loops`]
    /// checks its invariant; otherwise it is refused as before.
    Loop((usize, usize)),
    Unsupported(String),
}

/// A call's effect on the counted users, as this pass reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CallEffect {
    Neutral,
    Opaque(String),
}

/// The environment one unit's check reads.
struct Unit<'a> {
    tokens: &'a [Token],
    contracts: &'a ResourceContracts,
    counters: BTreeSet<String>,
    premises: literal_premise::LiteralPremises,
    derived: std::cell::RefCell<BTreeMap<String, CallEffect>>,
    /// The declared callees whose bodies this unit defines, and whether their
    /// clauses hold against them.
    verified: std::cell::RefCell<BTreeMap<String, Result<(), String>>>,
    /// The records of every declared callee a verification consulted.
    consulted: std::cell::RefCell<BTreeMap<String, Vec<ClauseRecord>>>,
}

const KEYWORDS: [&str; 16] = [
    "if",
    "while",
    "for",
    "switch",
    "return",
    "sizeof",
    "_Alignof",
    "__alignof__",
    "_Generic",
    "__typeof__",
    "typeof",
    "__builtin_offsetof",
    "__extension__",
    "__attribute__",
    "do",
    "else",
];

const TYPE_WORDS: [&str; 18] = [
    "struct", "union", "enum", "void", "char", "short", "int", "long", "float", "double", "signed",
    "unsigned", "const", "volatile", "_Bool", "size_t", "uint64_t", "uint32_t",
];

/// Check one reported function. Every environment without a counted-user
/// clause, and every body that touches no declared counter operation, leaves
/// `summary` unchanged.
pub(super) fn check(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    summary: &mut LegacyFunctionSummary,
) {
    let Some((mut records, diagnostics, consulted)) =
        evaluate_function(tokens, function, contracts)
    else {
        return;
    };
    // The declared operations this body relies on, as their own bodies here
    // decided them.
    for (callee, callee_records) in &consulted {
        for record in callee_records {
            records.push(ClauseRecord {
                clause: format!("callee `{callee}`: {}", record.clause),
                ..record.clone()
            });
        }
    }
    if records.is_empty() && diagnostics.is_empty() {
        return;
    }
    summary.diagnostics.extend(diagnostics);
    summary.diagnostics.sort();
    summary.diagnostics.dedup();
    summary.status = super::classify_diagnostics(&summary.diagnostics);
    records.sort();
    records.dedup();
    summary.counted_users = records;
}

/// Card **W45-REFUTED-CONTRACT-WITHDRAWAL**: the refuted records of
/// `function`'s *own* clauses (declared and implicit, never a consulted
/// callee's), exactly as [`check`] decides them over the same stream and
/// environment, with nothing reported. Empty in every environment without a
/// counted-user clause.
pub(super) fn refuted_own_clauses(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Vec<ClauseRecord> {
    evaluate_function(tokens, function, contracts)
        .map(|(records, _, _)| {
            records
                .into_iter()
                .filter(|record| record.verdict == ClauseVerdict::Refuted)
                .collect()
        })
        .unwrap_or_default()
}

type Evaluated = (
    Vec<ClauseRecord>,
    Vec<LegacyDiagnostic>,
    BTreeMap<String, Vec<ClauseRecord>>,
);

/// The shared body of [`check`] and [`refuted_own_clauses`]: the function's
/// own records, its diagnostics and the records of every declared callee a
/// verification consulted, or `None` when the pass leaves it alone.
fn evaluate_function(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<Evaluated> {
    let counters = contracts
        .functions()
        .values()
        .flat_map(|contract| contract.counted_users.effects.values())
        .map(|operation| operation.counter.clone())
        .collect::<BTreeSet<_>>();
    let any_exit_or_neutral = contracts
        .functions()
        .values()
        .any(|contract| !contract.counted_users.is_empty());
    if counters.is_empty() && !any_exit_or_neutral {
        return None;
    }
    let premises = literal_premise::LiteralPremises::read(contracts);
    let restricted = literal_premise::restrict_to_declared_literals(tokens, contracts, &premises);
    let tokens = restricted.as_deref().unwrap_or(tokens);
    let unit = Unit {
        tokens,
        contracts,
        counters,
        premises,
        derived: std::cell::RefCell::default(),
        verified: std::cell::RefCell::default(),
        consulted: std::cell::RefCell::default(),
    };
    let (records, diagnostics) = unit.evaluate(function)?;
    let consulted = unit.consulted.into_inner();
    Some((records, diagnostics, consulted))
}

impl Unit<'_> {
    /// Evaluate one function's counted-user obligations: its records and
    /// diagnostics, or `None` when it touches no counted user at all.
    #[allow(clippy::too_many_lines)]
    fn evaluate(&self, function: &str) -> Option<(Vec<ClauseRecord>, Vec<LegacyDiagnostic>)> {
        let unit = self;
        let tokens = self.tokens;
        let contracts = self.contracts;
        let body = unit.body(function)?;
        let own = contracts
            .contract(function)
            .filter(|contract| !contract.counted_users.is_empty());
        let (owned, origins) = unit.fused(body);
        let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let formals = function_parameters(tokens, function).unwrap_or_default();
        let stable = stable_formals(&texts, &formals);
        // The formals a clause names or a body operation attributes.
        let mut tracked = BTreeSet::new();
        if let Some(own) = own {
            for exit in &own.counted_users.exits {
                if let Some(Some(name)) = formals.get(exit.parameter) {
                    tracked.insert(name.clone());
                }
            }
            for &index in own.counted_users.effects.keys() {
                if let Some(Some(name)) = formals.get(index) {
                    tracked.insert(name.clone());
                }
            }
        }
        let operations = unit.operations(&texts, body.0 + 1, &origins);
        if own.is_none() && operations.is_empty() {
            return None;
        }
        for operation in &operations {
            if let Some(formal) = &operation.formal
                && stable.contains(formal)
            {
                tracked.insert(formal.clone());
            }
        }
        let mut records = Vec::new();
        let mut diagnostics = Vec::new();
        let line = |index: usize| unit.tokens.get(index).map_or(0, |token| token.line);
        let function_line = line(body.0);
        // Unattributed operations are reported once each, whatever is tracked.
        for operation in &operations {
            if operation
                .formal
                .as_ref()
                .is_none_or(|formal| !stable.contains(formal))
            {
                diagnostics.push(LegacyDiagnostic {
                function: function.to_owned(),
                line: operation.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: format!(
                        "counted-user operation {} on `{}`, which is not a formal this function never rebinds",
                        operation.what, operation.actual
                    ),
                },
            });
            }
        }
        // A neutral declaration is checked by the derivation of the body.
        if let Some(own) = own
            && own.counted_users.neutral
        {
            let derived = unit.derive_body(function, &mut BTreeSet::new());
            let (verdict, reason) = match derived {
                CallEffect::Neutral => (ClauseVerdict::Held, String::new()),
                CallEffect::Opaque(reason) => (ClauseVerdict::Undecided, reason),
            };
            records.push(ClauseRecord {
                clause: "resource_user_neutral()".into(),
                verdict,
                line: 0,
                reason,
            });
        }
        if !tracked.is_empty() {
            let statements = parse_block(&texts, 0, texts.len());
            let mut initial = PathState::default();
            for formal in &tracked {
                initial.counts.insert(formal.clone(), Count::Known(0));
            }
            // A literal premise of this function's own declaration fixes the
            // formal's value for the body it is checked against.
            for premise in unit.premises.declared(function) {
                if let (Some(Some(name)), Some(value)) =
                    (formals.get(premise.index), literal_value(&premise.literal))
                {
                    initial
                        .values
                        .insert(name.clone(), Value::OneOf(BTreeSet::from([value])));
                }
            }
            let walker = Walker {
                unit,
                texts: &texts,
                offset: body.0 + 1,
                origins: &origins,
                stable: &stable,
            };
            let mut exits = Vec::new();
            let mut abandoned = None;
            walker.run(
                &statements,
                vec![initial],
                &mut exits,
                &mut abandoned,
                function_line,
            );
            let clauses = own.map(|own| own.counted_users.clone()).unwrap_or_default();
            decide(
                function,
                &formals,
                &tracked,
                &clauses,
                &exits,
                abandoned.as_deref(),
                &mut records,
                &mut diagnostics,
                function_line,
            );
        }
        Some((records, diagnostics))
    }

    /// Whether the declared counted-user clauses of `callee`, whose body is
    /// defined in this unit, hold against that body. A declaration is applied
    /// at a call only when they do: a body here is checked, never replaced.
    fn verify_declared(&self, callee: &str) -> Result<(), String> {
        if let Some(known) = self.verified.borrow().get(callee) {
            return known.clone();
        }
        // A cycle through declared operations is not verified.
        self.verified.borrow_mut().insert(
            callee.to_owned(),
            Err(format!("`{callee}` is verified through itself")),
        );
        let result = match self.evaluate(callee) {
            Some((records, _)) => {
                let result = records
                    .iter()
                    .find(|record| record.verdict != ClauseVerdict::Held)
                    .map_or(Ok(()), |record| {
                        Err(format!(
                            "the declaration of `{callee}` does not hold against its body here \
                             (`{}` {}: {})",
                            record.clause,
                            record.verdict.spelling(),
                            record.reason
                        ))
                    });
                self.consulted
                    .borrow_mut()
                    .insert(callee.to_owned(), records);
                result
            }
            None => Err(format!(
                "`{callee}` states no clause its body can be checked against"
            )),
        };
        self.verified
            .borrow_mut()
            .insert(callee.to_owned(), result.clone());
        result
    }
}

fn literal_value(literal: &[String]) -> Option<i64> {
    match literal {
        [digits] => digits.parse().ok(),
        [minus, digits] if minus == "-" => digits.parse::<i64>().ok().map(|value| -value),
        _ => None,
    }
}

/// The formals no statement rebinds (the reading of card
/// W45-FB-RETRY-LIFETIME's container rule, reused).
fn stable_formals(texts: &[&str], formals: &[Option<String>]) -> BTreeSet<String> {
    let names = formals.iter().flatten().cloned().collect::<BTreeSet<_>>();
    let read = super::container_publication::CallerFormals::read(texts, &names);
    names
        .into_iter()
        .filter(|name| read.contains(name))
        .collect()
}

/// One counter operation the body performs, found lexically.
struct Operation {
    line: usize,
    what: String,
    actual: String,
    formal: Option<String>,
}

impl Unit<'_> {
    fn body(&self, function: &str) -> Option<(usize, usize)> {
        let (start, _) = find_function_body(self.tokens, function)?;
        let end = matching(self.tokens, start, "{", "}")?;
        Some((start, end))
    }

    /// The body's tokens with C's multi-character punctuators fused, and the
    /// body-relative index of each fused token's first token.
    fn fused(&self, body: (usize, usize)) -> (Vec<String>, Vec<usize>) {
        let raw = self.tokens[body.0 + 1..body.1]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        super::container_publication::fuse(&raw).into_iter().unzip()
    }

    fn line(&self, index: usize) -> usize {
        self.tokens.get(index).map_or(0, |token| token.line)
    }

    /// The counter operations a body spells: declared operations at their
    /// declared positions, exit-clause calls at their clause positions, and
    /// direct writes to a declared counter member.
    fn operations(&self, texts: &[&str], offset: usize, origins: &[usize]) -> Vec<Operation> {
        let mut found = Vec::new();
        for index in 0..texts.len() {
            if let Some(write) = counter_write(texts, index, &self.counters) {
                found.push(Operation {
                    line: self.line(offset + origins[index]),
                    what: format!("`{}`", write.spelling),
                    actual: write.base.clone(),
                    formal: write.formal_base.then_some(write.base),
                });
                continue;
            }
            if !(is_identifier(texts[index]) && texts.get(index + 1) == Some(&"(")) {
                continue;
            }
            let Some(contract) = self.contracts.contract(texts[index]) else {
                continue;
            };
            let Some(close) = close_of(texts, index + 1) else {
                continue;
            };
            let arguments = split_arguments(&texts[index + 2..close]);
            let positions = contract
                .counted_users
                .effects
                .keys()
                .copied()
                .chain(
                    contract
                        .counted_users
                        .exits
                        .iter()
                        .map(|exit| exit.parameter),
                )
                .collect::<BTreeSet<_>>();
            for position in positions {
                let actual = arguments.get(position).copied().unwrap_or(&[]);
                found.push(Operation {
                    line: self.line(offset + origins[index]),
                    what: format!("by `{}`", texts[index]),
                    actual: actual.join(" "),
                    formal: match actual {
                        [name] if is_identifier(name) => Some((*name).to_owned()),
                        _ => None,
                    },
                });
            }
        }
        found
    }

    /// The effect of calling `callee` as far as neutrality goes: the declared
    /// effects and exit clauses are applied by the walker before this is asked.
    fn call_effect(&self, callee: &str, visiting: &mut BTreeSet<String>) -> CallEffect {
        let contract = self.contracts.contract(callee);
        if contract.is_some_and(|contract| contract.counted_users.neutral) {
            // Card W45-SR5-SOUNDNESS (`DECLARED-NEUTRAL-UNVERIFIED-AT-CALL`):
            // a declared neutrality of a body this unit defines is applied
            // only when that body verifies it, as every other declared clause
            // is; before, it was applied before the body was examined.
            if self.body(callee).is_some() {
                return match self.verify_declared(callee) {
                    Ok(()) => CallEffect::Neutral,
                    Err(reason) => CallEffect::Opaque(reason),
                };
            }
            return CallEffect::Neutral;
        }
        if self.body(callee).is_some() {
            return self.derive_body(callee, visiting);
        }
        match contract {
            Some(contract) if contract.counted_users.is_empty() => CallEffect::Neutral,
            Some(_) => CallEffect::Opaque(format!(
                "`{callee}` declares counted-user clauses this position does not apply"
            )),
            None => CallEffect::Opaque(format!(
                "call to `{callee}`, which has no contract in force and no body in this unit"
            )),
        }
    }

    /// Flow-insensitive derivation of a body defined in this unit: neutral
    /// when it writes no declared counter and every call it makes is neutral.
    fn derive_body(&self, callee: &str, visiting: &mut BTreeSet<String>) -> CallEffect {
        if let Some(known) = self.derived.borrow().get(callee) {
            return known.clone();
        }
        if !visiting.insert(callee.to_owned()) {
            return CallEffect::Opaque(format!("recursive call to `{callee}`"));
        }
        let Some(body) = self.body(callee) else {
            return CallEffect::Opaque(format!("no body for `{callee}`"));
        };
        let (owned, _) = self.fused(body);
        let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let mut result = CallEffect::Neutral;
        for index in 0..texts.len() {
            if let Some(write) = counter_write(&texts, index, &self.counters) {
                result = CallEffect::Opaque(format!(
                    "`{callee}` writes the counter (`{}`)",
                    write.spelling
                ));
                break;
            }
            if let Some(reason) = opaque_callee(&texts, index) {
                result = CallEffect::Opaque(format!("`{callee}` {reason}"));
                break;
            }
            if !(is_identifier(texts[index])
                && texts.get(index + 1) == Some(&"(")
                && !KEYWORDS.contains(&texts[index]))
            {
                continue;
            }
            let inner = texts[index];
            if let Some(contract) = self.contracts.contract(inner)
                && (!contract.counted_users.effects.is_empty()
                    || !contract.counted_users.exits.is_empty())
            {
                result = CallEffect::Opaque(format!(
                    "`{callee}` performs a counted-user operation through `{inner}`"
                ));
                break;
            }
            if let CallEffect::Opaque(reason) = self.call_effect(inner, visiting) {
                result = CallEffect::Opaque(if inner == callee {
                    reason
                } else {
                    format!("`{callee}` calls `{inner}`: {reason}")
                });
                break;
            }
        }
        visiting.remove(callee);
        self.derived
            .borrow_mut()
            .insert(callee.to_owned(), result.clone());
        result
    }
}

/// A direct write to a declared counter member.
struct CounterWrite {
    base: String,
    formal_base: bool,
    spelling: String,
    step: Option<i32>,
}

/// `B -> C ++`, `B -> C --`, `++ B -> C`, `-- B -> C` (a step of one), or any
/// other assignment to `… -> C` (no step: an undecided write), where `C` is a
/// declared counter member and the write begins at `index`.
fn counter_write(
    texts: &[&str],
    index: usize,
    counters: &BTreeSet<String>,
) -> Option<CounterWrite> {
    let member = |at: usize| {
        texts.get(at + 1) == Some(&"->")
            && texts
                .get(at + 2)
                .is_some_and(|field| counters.contains(*field))
            && is_identifier(texts[at])
            && (at == 0 || !matches!(texts[at - 1], "->" | "."))
    };
    // Prefix forms.
    if matches!(texts[index], "++" | "--") && index + 1 < texts.len() && member(index + 1) {
        let base = texts[index + 1].to_owned();
        return Some(CounterWrite {
            spelling: texts[index..index + 4].join(" "),
            formal_base: true,
            step: Some(if texts[index] == "++" { 1 } else { -1 }),
            base,
        });
    }
    if !member(index) {
        return None;
    }
    let after = texts.get(index + 3).copied()?;
    let step = match after {
        "++" => Some(1),
        "--" => Some(-1),
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" => None,
        _ => return None,
    };
    Some(CounterWrite {
        base: texts[index].to_owned(),
        formal_base: true,
        spelling: texts[index..=index + 3].join(" "),
        step,
    })
}

/// Card W45-SR5-SOUNDNESS: the reason an event behind a conditional operator
/// undecides the counts.
const CONDITIONAL_EVENT: &str = "a call or counter write C may skip behind `?:`, `&&` or `||`, which this pass does not fork on";

/// Card W45-SR5-SOUNDNESS: the token indices C may skip in one fused
/// expression: every token after a `?`, `&&` or `||` up to the end of the
/// group that operator is in. A superset of the conditionally evaluated
/// tokens (the second and third operands of `?:` both count).
fn conditional_positions(texts: &[&str]) -> Vec<bool> {
    let mut conditional = vec![false; texts.len()];
    let mut depth = 0usize;
    let mut starts: Vec<usize> = Vec::new();
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.saturating_sub(1);
                starts.retain(|start| *start <= depth);
            }
            _ => {}
        }
        if matches!(*text, "?" | "&&" | "||") {
            starts.push(depth);
        }
        conditional[index] = starts.iter().any(|start| *start <= depth);
    }
    conditional
}

/// A call through something that is not a name: `) (` after a group that is
/// not a cast, or `] (`. Card W45-SR5-SOUNDNESS: a group that is a call's
/// argument list, followed by `(`, calls the call's result (`f(a)(b)`), which
/// is not a name either (gateway constraint 10c, rule 2).
fn opaque_callee(texts: &[&str], index: usize) -> Option<String> {
    if texts.get(index + 1) != Some(&"(") {
        return None;
    }
    match texts[index] {
        "]" => Some("calls through a subscripted expression".into()),
        ")" => {
            let open = open_of(texts, index)?;
            let first = texts.get(open + 1).copied().unwrap_or("");
            let before = open.checked_sub(1).map(|at| texts[at]);
            let argument_list = before.is_some_and(|word| {
                is_identifier(word) && !KEYWORDS.contains(&word) || word == ")" || word == "]"
            });
            if argument_list {
                Some("calls through a call's result".into())
            } else if TYPE_WORDS.contains(&first) {
                None
            } else {
                Some("calls through an expression".into())
            }
        }
        _ => None,
    }
}

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn open_of(texts: &[&str], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match texts[index] {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_arguments<'a>(texts: &'a [&'a str]) -> Vec<&'a [&'a str]> {
    if texts.is_empty() {
        return Vec::new();
    }
    let mut arguments = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                arguments.push(&texts[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    arguments.push(&texts[start..]);
    arguments
}

/// The end (exclusive) of the statement starting at `start`: its `;` at depth
/// zero, or the end of the range.
fn statement_end(texts: &[&str], start: usize, end: usize) -> usize {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().take(end).skip(start) {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            ";" if depth == 0 => return index + 1,
            _ => {}
        }
    }
    end
}

fn parse_block(texts: &[&str], start: usize, end: usize) -> Vec<Stmt> {
    let mut statements = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let (statement, next) = parse_statement(texts, cursor, end);
        statements.push(statement);
        if next <= cursor {
            statements.push(Stmt::Unsupported("unparsed statement".into()));
            break;
        }
        cursor = next;
    }
    statements
}

fn parse_statement(texts: &[&str], start: usize, end: usize) -> (Stmt, usize) {
    match texts[start] {
        ";" => (Stmt::Block(Vec::new()), start + 1),
        "{" => match close_of(texts, start) {
            Some(close) if close < end => {
                (Stmt::Block(parse_block(texts, start + 1, close)), close + 1)
            }
            _ => (Stmt::Unsupported("unbalanced block".into()), end),
        },
        "if" => {
            let Some(close) = texts
                .get(start + 1)
                .filter(|text| **text == "(")
                .and_then(|_| close_of(texts, start + 1))
            else {
                return (Stmt::Unsupported("malformed if".into()), end);
            };
            let (then, next) = parse_statement(texts, close + 1, end);
            if texts.get(next) == Some(&"else") {
                let (otherwise, after) = parse_statement(texts, next + 1, end);
                (
                    Stmt::If {
                        condition: (start + 2, close),
                        then: Box::new(then),
                        otherwise: Some(Box::new(otherwise)),
                    },
                    after,
                )
            } else {
                (
                    Stmt::If {
                        condition: (start + 2, close),
                        then: Box::new(then),
                        otherwise: None,
                    },
                    next,
                )
            }
        }
        "return" => {
            let stop = statement_end(texts, start, end);
            (
                Stmt::Return {
                    expression: (start + 1, stop.saturating_sub(1).max(start + 1)),
                    line: start,
                },
                stop,
            )
        }
        // `do S while (0);` is S, exactly once (constraint 5c's spelling).
        "do" => {
            let (body, next) = parse_statement(texts, start + 1, end);
            if texts.get(next..next + 5) == Some(&["while", "(", "0", ")", ";"][..]) {
                (body, next + 5)
            } else {
                loop_statement(texts, start, end)
            }
        }
        "while" | "for" => loop_statement(texts, start, end),
        "switch" => (Stmt::Unsupported("a `switch`".into()), end),
        "goto" | "case" | "default" => (Stmt::Unsupported("a jump or label".into()), end),
        "break" | "continue" => (Stmt::Unsupported("a jump or label".into()), end),
        _ => {
            if texts.get(start + 1) == Some(&":") && is_identifier(texts[start]) {
                return (Stmt::Unsupported("a jump or label".into()), end);
            }
            let stop = statement_end(texts, start, end);
            (Stmt::Expression((start, stop)), stop)
        }
    }
}

/// Card CG27: a loop statement whose extent is read, or the pre-existing
/// refusal when it is not.
fn loop_statement(texts: &[&str], start: usize, end: usize) -> (Stmt, usize) {
    match loops::statement_extent(texts, start, end) {
        Some(stop) => (Stmt::Loop((start, stop)), stop),
        None => (Stmt::Unsupported("a loop".into()), end),
    }
}

/// The path enumerator.
struct Walker<'a> {
    unit: &'a Unit<'a>,
    texts: &'a [&'a str],
    offset: usize,
    origins: &'a [usize],
    stable: &'a BTreeSet<String>,
}

impl Walker<'_> {
    fn line(&self, index: usize) -> usize {
        let origin = self.origins.get(index).copied().unwrap_or(index);
        self.unit.line(self.offset + origin)
    }

    fn run(
        &self,
        statements: &[Stmt],
        states: Vec<PathState>,
        exits: &mut Vec<Exit>,
        abandoned: &mut Option<String>,
        function_line: usize,
    ) {
        let falling = self.block(statements, states, exits, abandoned);
        // Falling off the end of the body returns no value.
        for state in falling {
            exits.push(Exit {
                line: function_line,
                returned: Returned::Void,
                state,
            });
        }
    }

    fn block(
        &self,
        statements: &[Stmt],
        mut states: Vec<PathState>,
        exits: &mut Vec<Exit>,
        abandoned: &mut Option<String>,
    ) -> Vec<PathState> {
        for statement in statements {
            if states.is_empty() || abandoned.is_some() {
                return Vec::new();
            }
            states = self.statement(statement, states, exits, abandoned);
            if states.len() + exits.len() > PATH_BOUND {
                *abandoned = Some(format!("more than {PATH_BOUND} paths"));
                return Vec::new();
            }
        }
        states
    }

    fn statement(
        &self,
        statement: &Stmt,
        states: Vec<PathState>,
        exits: &mut Vec<Exit>,
        abandoned: &mut Option<String>,
    ) -> Vec<PathState> {
        match statement {
            Stmt::Block(inner) => self.block(inner, states, exits, abandoned),
            Stmt::Unsupported(what) => {
                *abandoned = Some(format!(
                    "the body contains {what}, which this pass does not model"
                ));
                Vec::new()
            }
            // Card CG27 (rule 11x): a loop whose every token keeps the counts
            // as it found them is summarized by the identity on the counts,
            // forgetting the values of the names it spells; any other loop is
            // refused with the pre-existing wording, never assumed.
            Stmt::Loop(range) => match loops::neutral_loop(self.unit, self.texts, *range) {
                Ok(names) => states
                    .into_iter()
                    .map(|mut state| {
                        for name in &names {
                            state.values.remove(name);
                        }
                        state
                    })
                    .collect(),
                Err(_) => {
                    *abandoned =
                        Some("the body contains a loop, which this pass does not model".into());
                    Vec::new()
                }
            },
            Stmt::Expression(range) => states
                .into_iter()
                .flat_map(|state| {
                    self.expression(*range, state)
                        .into_iter()
                        .map(|(state, _)| state)
                })
                .collect(),
            Stmt::Return { expression, line } => {
                for state in states {
                    for (state, returned) in self.expression(*expression, state) {
                        exits.push(Exit {
                            line: self.line(*line),
                            returned,
                            state,
                        });
                    }
                }
                Vec::new()
            }
            Stmt::If {
                condition,
                then,
                otherwise,
            } => {
                let mut taken = Vec::new();
                let mut skipped = Vec::new();
                for state in states {
                    for (state, _) in self.expression(*condition, state) {
                        let (may_true, may_false) = self.decide(*condition, &state);
                        if may_true {
                            taken.push(state.clone());
                        }
                        if may_false {
                            skipped.push(state);
                        }
                    }
                }
                let mut after = self.statement(then, taken, exits, abandoned);
                match otherwise {
                    Some(otherwise) => {
                        after.extend(self.statement(otherwise, skipped, exits, abandoned))
                    }
                    None => after.extend(skipped),
                }
                after
            }
        }
    }

    /// Whether a condition may be true and may be false on this path. Only a
    /// comparison of a name with a known value set against a literal, a
    /// negated name and a bare name are decided.
    fn decide(&self, (start, end): (usize, usize), state: &PathState) -> (bool, bool) {
        let texts = strip_parentheses(&self.texts[start..end]);
        let literal = |spelling: &[&str]| -> Option<i64> {
            match spelling {
                [digits] => digits.parse().ok(),
                ["-", digits] => digits.parse::<i64>().ok().map(|value| -value),
                _ => None,
            }
        };
        let compare = |name: &str, value: i64, equal: bool| match state.values.get(name) {
            Some(known) => {
                let may_equal = known.may_be(value);
                let must_equal = known.is_exactly(value);
                if equal {
                    (may_equal, !must_equal)
                } else {
                    (!must_equal, may_equal)
                }
            }
            None => (true, true),
        };
        match texts {
            [name, op @ ("==" | "!="), rest @ ..] if is_identifier(name) => {
                literal(rest).map_or((true, true), |value| compare(name, value, *op == "=="))
            }
            [first @ .., op @ ("==" | "!="), name] if is_identifier(name) && !first.is_empty() => {
                literal(first).map_or((true, true), |value| compare(name, value, *op == "=="))
            }
            ["!", name] if is_identifier(name) => compare(name, 0, true),
            [name] if is_identifier(name) => compare(name, 0, false),
            _ => (true, true),
        }
    }

    /// Evaluate the calls and writes of one expression, forking where a stored
    /// call result has declared exit values, and read what it returns.
    fn expression(
        &self,
        (start, end): (usize, usize),
        state: PathState,
    ) -> Vec<(PathState, Returned)> {
        let texts = &self.texts[start..end.min(self.texts.len())];
        let texts = texts.strip_suffix(&[";"]).unwrap_or(texts);
        // `NAME = …`: the destination whose value this expression decides.
        let (destination, value_start) = match texts {
            [name, "=", ..] if is_identifier(name) => (Some(*name), 2),
            // A declaration with an initialiser: `T … NAME = …`.
            _ => match texts.iter().position(|text| *text == "=") {
                Some(at)
                    if at >= 2
                        && is_identifier(texts[at - 1])
                        && texts[..at - 1]
                            .iter()
                            .all(|text| is_identifier(text) || *text == "*") =>
                {
                    (Some(texts[at - 1]), at + 1)
                }
                _ => (None, 0),
            },
        };
        let mut states = vec![state];
        // Card W45-SR5-SOUNDNESS (`COUNTED-USER-CONDITIONAL-OPERAND`): C may
        // skip a call or a counter write behind `?:`, `&&` or `||`, which this
        // pass does not fork on, so such an event undecides every count
        // rather than applying on every path.
        let conditional = conditional_positions(texts);
        // Calls and counter writes, innermost and leftmost first.
        let mut events = Vec::new();
        for index in 0..texts.len() {
            if conditional[index]
                && (counter_write(texts, index, &self.unit.counters).is_some()
                    || (is_identifier(texts[index])
                        && texts.get(index + 1) == Some(&"(")
                        && !KEYWORDS.contains(&texts[index])))
            {
                events.push((index, Event::Opaque(CONDITIONAL_EVENT.into())));
            } else if let Some(write) = counter_write(texts, index, &self.unit.counters) {
                events.push((
                    index + write.spelling.split(' ').count(),
                    Event::Write(write),
                ));
            } else if let Some(reason) = opaque_callee(texts, index) {
                events.push((index, Event::Opaque(reason)));
            } else if is_identifier(texts[index])
                && texts.get(index + 1) == Some(&"(")
                && !KEYWORDS.contains(&texts[index])
                && let Some(close) = close_of(texts, index + 1)
            {
                events.push((
                    close,
                    Event::Call {
                        callee: index,
                        close,
                    },
                ));
            }
        }
        events.sort_by_key(|(order, _)| *order);
        let whole_call = |callee: usize, close: usize| {
            destination.is_some() && callee == value_start && close + 1 == texts.len()
        };
        for (_, event) in events {
            match event {
                Event::Write(write) => {
                    for state in &mut states {
                        match (write.step, self.stable.contains(&write.base)) {
                            (Some(step), true) if state.counts.contains_key(&write.base) => {
                                let effect = if step > 0 {
                                    UserEffect::Added
                                } else {
                                    UserEffect::Dropped
                                };
                                state.apply(
                                    &write.base,
                                    effect,
                                    self.line(start),
                                    "a counter write",
                                );
                            }
                            _ => state.undecide_all(&format!(
                                "a direct counter write `{}`",
                                write.spelling
                            )),
                        }
                    }
                }
                Event::Opaque(reason) => {
                    for state in &mut states {
                        state.undecide_all(&reason);
                    }
                }
                Event::Call { callee, close } => {
                    let name = texts[callee];
                    let arguments = split_arguments(&texts[callee + 2..close]);
                    states = self.call(
                        name,
                        &arguments,
                        whole_call(callee, close).then_some(destination).flatten(),
                        self.line(start + callee),
                        states,
                    );
                }
            }
        }
        // The destination's value, when it is not a forked call result.
        if let Some(destination) = destination {
            let value = &texts[value_start..];
            let is_call = value.first().is_some_and(|word| is_identifier(word))
                && value.get(1) == Some(&"(")
                && close_of(value, 1).is_some_and(|close| close + 1 == value.len());
            if !is_call {
                for state in &mut states {
                    match literal_of(value) {
                        Some(literal) => {
                            state.values.insert(
                                destination.to_owned(),
                                Value::OneOf(BTreeSet::from([literal])),
                            );
                        }
                        None => {
                            state.values.remove(destination);
                        }
                    }
                }
            }
        }
        // Any other write to a name forgets its value.
        for (index, text) in texts.iter().enumerate() {
            if matches!(*text, "++" | "--") {
                for neighbour in [index.checked_sub(1), Some(index + 1)]
                    .into_iter()
                    .flatten()
                {
                    if let Some(name) = texts.get(neighbour) {
                        for state in &mut states {
                            state.values.remove(*name);
                        }
                    }
                }
            }
        }
        let returned = |state: &PathState| -> Returned {
            if texts.is_empty() {
                return Returned::Void;
            }
            let value = strip_parentheses(texts);
            if let Some(literal) = literal_of(value) {
                return Returned::Value(Value::OneOf(BTreeSet::from([literal])));
            }
            if let [name] = value
                && let Some(known) = state.values.get(*name)
            {
                return Returned::Value(known.clone());
            }
            Returned::Undecided
        };
        states
            .into_iter()
            .map(|state| {
                let value = returned(&state);
                (state, value)
            })
            .collect()
    }

    /// Apply one call.
    fn call(
        &self,
        name: &str,
        arguments: &[&[&str]],
        destination: Option<&str>,
        line: usize,
        mut states: Vec<PathState>,
    ) -> Vec<PathState> {
        let contract: Option<&ResourceFunctionContract> = self.unit.contracts.contract(name);
        // A declared counter operation or exit clause whose body this unit
        // defines is applied only when it holds against that body.
        if contract.is_some_and(|contract| {
            !contract.counted_users.effects.is_empty() || !contract.counted_users.exits.is_empty()
        }) && self.unit.body(name).is_some()
            && let Err(reason) = self.unit.verify_declared(name)
        {
            for state in &mut states {
                state.undecide_all(&reason);
            }
            return states;
        }
        let attributed = |position: usize| -> Option<String> {
            match arguments.get(position).copied().unwrap_or(&[]) {
                [actual] if self.stable.contains(*actual) => Some((*actual).to_owned()),
                _ => None,
            }
        };
        if let Some(contract) = contract
            && !contract.counted_users.effects.is_empty()
        {
            for (&position, operation) in &contract.counted_users.effects {
                let what = format!("`{name}`");
                for state in &mut states {
                    match attributed(position) {
                        Some(formal) if state.counts.contains_key(&formal) => {
                            state.apply(&formal, operation.effect, line, &what);
                        }
                        _ => state.undecide_all(&format!(
                            "an unattributed counted-user operation by `{name}`"
                        )),
                    }
                }
            }
            return states;
        }
        if let Some(contract) = contract
            && !contract.counted_users.exits.is_empty()
        {
            return self.exit_call(
                name,
                &contract.counted_users.exits,
                destination,
                &attributed,
                states,
            );
        }
        if let CallEffect::Opaque(reason) = self.unit.call_effect(name, &mut BTreeSet::new()) {
            for state in &mut states {
                state.undecide_all(&reason);
            }
        }
        if let Some(destination) = destination {
            for state in &mut states {
                state.values.remove(destination);
            }
        }
        states
    }

    /// A call whose declaration states exit clauses.
    fn exit_call(
        &self,
        name: &str,
        exits: &[UserExitClause],
        destination: Option<&str>,
        attributed: &dyn Fn(usize) -> Option<String>,
        states: Vec<PathState>,
    ) -> Vec<PathState> {
        let values = exits
            .iter()
            .map(|exit| exit.returned)
            .collect::<BTreeSet<_>>();
        let positions = exits
            .iter()
            .map(|exit| exit.parameter)
            .collect::<BTreeSet<_>>();
        // The clause of `position` for one returned value; a value no clause
        // covers (`None`) leaves the count undecided.
        let delta = |position: usize, returned: Option<i64>| -> Option<i8> {
            let value = returned?;
            exits
                .iter()
                .find(|exit| exit.parameter == position && exit.returned == value)
                .map(|exit| exit.delta)
        };
        let mut forked = Vec::new();
        for state in states {
            // Without a stored result the value is never compared: one path,
            // decided only when every clause of a position agrees.
            let alternatives: Vec<Option<i64>> = if destination.is_some() {
                values.iter().copied().map(Some).chain([None]).collect()
            } else {
                vec![None]
            };
            for alternative in alternatives {
                let mut state = state.clone();
                for &position in &positions {
                    match attributed(position) {
                        Some(formal) if state.counts.contains_key(&formal) => {
                            let agreed = if alternative.is_none() && destination.is_none() {
                                let deltas = exits
                                    .iter()
                                    .filter(|exit| exit.parameter == position)
                                    .map(|exit| exit.delta)
                                    .collect::<BTreeSet<_>>();
                                (deltas.len() == 1).then(|| *deltas.first().expect("one"))
                            } else {
                                delta(position, alternative)
                            };
                            match agreed {
                                Some(step) => {
                                    let effect = match step {
                                        1 => Some(UserEffect::Added),
                                        -1 => Some(UserEffect::Dropped),
                                        _ => None,
                                    };
                                    if let Some(effect) = effect {
                                        state.apply(&formal, effect, 0, &format!("`{name}`"));
                                        // A dropped user through an exit clause
                                        // states only the count; the check it
                                        // owes is the callee's to report.
                                        state.pending.remove(&formal);
                                    }
                                }
                                None => {
                                    if let Some(count) = state.counts.get_mut(&formal) {
                                        *count = Count::Undecided(format!(
                                            "`{name}` returned a value its exit clauses do not cover"
                                        ));
                                    }
                                }
                            }
                        }
                        Some(_) => {}
                        None => state.undecide_all(&format!(
                            "an unattributed counted-user operation by `{name}`"
                        )),
                    }
                }
                if let Some(destination) = destination {
                    let value = match alternative {
                        Some(value) => Value::OneOf(BTreeSet::from([value])),
                        None => Value::NoneOf(values.clone()),
                    };
                    state.values.insert(destination.to_owned(), value);
                }
                forked.push(state);
            }
        }
        forked
    }
}

/// One event of an expression, in evaluation order.
enum Event {
    Write(CounterWrite),
    Opaque(String),
    Call { callee: usize, close: usize },
}

fn strip_parentheses<'a>(mut texts: &'a [&'a str]) -> &'a [&'a str] {
    while texts.first() == Some(&"(") && close_of(texts, 0) == Some(texts.len() - 1) {
        texts = &texts[1..texts.len() - 1];
    }
    texts
}

fn literal_of(texts: &[&str]) -> Option<i64> {
    match strip_parentheses(texts) {
        [digits] if digits.bytes().all(|byte| byte.is_ascii_digit()) => digits.parse().ok(),
        ["-", digits] if digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            digits.parse::<i64>().ok().map(|value| -value)
        }
        _ => None,
    }
}

fn spell_exit(formal: &str, clause: &UserExitClause) -> String {
    let delta = match clause.delta {
        1 => "added",
        -1 => "dropped",
        _ => "restored",
    };
    format!("resource_user_exit({formal}, {}, {delta})", clause.returned)
}

/// Compare every exit with the clauses and publish the verdicts.
#[allow(clippy::too_many_arguments)]
fn decide(
    function: &str,
    formals: &[Option<String>],
    tracked: &BTreeSet<String>,
    clauses: &qcore_schemas::contract::CountedUserClauses,
    exits: &[Exit],
    abandoned: Option<&str>,
    records: &mut Vec<ClauseRecord>,
    diagnostics: &mut Vec<LegacyDiagnostic>,
    function_line: usize,
) {
    let name_of = |position: usize| formals.get(position).cloned().flatten();
    let violation = |line: usize, description: String| LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::CountedUser { description },
    };
    let undecided = |line: usize, description: String| LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("counted users undecided: {description}"),
        },
    };
    // Declared operations of the function itself: its body is the operation.
    let mut declared_effects = BTreeMap::new();
    for (&position, operation) in &clauses.effects {
        if let Some(formal) = name_of(position) {
            declared_effects.insert(formal, operation.clone());
        }
    }
    for formal in tracked {
        let formal_exits = clauses
            .exits
            .iter()
            .filter(|clause| name_of(clause.parameter).as_deref() == Some(formal.as_str()))
            .collect::<Vec<_>>();
        if let Some(abandoned) = abandoned {
            let spelled = if let Some(operation) = declared_effects.get(formal) {
                vec![format!(
                    "{:?}({formal}, {})",
                    operation.effect, operation.counter
                )]
            } else if formal_exits.is_empty() {
                vec![format!(
                    "implicit: every exit restores the users of `{formal}`"
                )]
            } else {
                formal_exits
                    .iter()
                    .map(|clause| spell_exit(formal, clause))
                    .collect()
            };
            for clause in spelled {
                records.push(ClauseRecord {
                    clause,
                    verdict: ClauseVerdict::Undecided,
                    line: 0,
                    reason: abandoned.to_owned(),
                });
            }
            diagnostics.push(undecided(function_line, abandoned.to_owned()));
            continue;
        }
        if let Some(operation) = declared_effects.get(formal) {
            // The body of a declared operation is checked against it.
            let (expected, pending_expected) = match operation.effect {
                UserEffect::Added => (1, false),
                UserEffect::Dropped => (-1, true),
                UserEffect::DroppedChecked => (-1, false),
                UserEffect::ReleaseCheck => (0, false),
            };
            let mut verdict = ClauseVerdict::Held;
            let mut reason = String::new();
            let mut line = 0;
            for exit in exits {
                match exit.state.counts.get(formal) {
                    Some(Count::Known(found))
                        if *found == expected
                            && exit.state.pending.contains_key(formal) == pending_expected
                            && (expected != -1
                                || pending_expected
                                || exit.state.checked.contains(formal)) => {}
                    Some(Count::Known(found)) => {
                        verdict = ClauseVerdict::Refuted;
                        line = exit.line;
                        reason = format!("an exit adds {found} users of `{formal}`");
                        diagnostics.push(violation(exit.line, format!(
                            "the body contradicts its declared counted-user operation on `{formal}`: {reason}"
                        )));
                        break;
                    }
                    other => {
                        if verdict == ClauseVerdict::Held {
                            verdict = ClauseVerdict::Undecided;
                            reason = why_of(other);
                        }
                    }
                }
            }
            if verdict == ClauseVerdict::Undecided {
                diagnostics.push(undecided(
                    function_line,
                    format!("declared operation on `{formal}`: {reason}"),
                ));
            }
            records.push(ClauseRecord {
                clause: format!(
                    "resource_user_{}({formal}, {})",
                    match operation.effect {
                        UserEffect::Added => "added",
                        UserEffect::Dropped => "dropped",
                        UserEffect::DroppedChecked => "dropped_checked",
                        UserEffect::ReleaseCheck => "release_check",
                    },
                    operation.counter
                ),
                verdict,
                line,
                reason,
            });
            continue;
        }
        let covered = formal_exits
            .iter()
            .map(|clause| clause.returned)
            .collect::<BTreeSet<_>>();
        // Each exit clause.
        for clause in &formal_exits {
            let mut verdict = ClauseVerdict::Held;
            let mut reason = String::new();
            let mut line = 0;
            for exit in exits {
                let (applies, definitely) = match &exit.returned {
                    Returned::Void => (false, false),
                    Returned::Undecided => (true, false),
                    Returned::Value(value) => (
                        value.may_be(clause.returned),
                        value.is_exactly(clause.returned),
                    ),
                };
                if !applies {
                    continue;
                }
                match exit.state.counts.get(formal) {
                    Some(Count::Known(found)) if *found == i32::from(clause.delta) => {}
                    Some(Count::Known(found)) if definitely => {
                        verdict = ClauseVerdict::Refuted;
                        line = exit.line;
                        reason = format!(
                            "a path returning {} has added {found} users of `{formal}`",
                            clause.returned
                        );
                        break;
                    }
                    Some(Count::Known(found)) => {
                        if verdict == ClauseVerdict::Held {
                            verdict = ClauseVerdict::Undecided;
                            line = exit.line;
                            reason = format!(
                                "an exit whose value is not decided has added {found} users of `{formal}`"
                            );
                        }
                    }
                    other => {
                        if verdict == ClauseVerdict::Held {
                            verdict = ClauseVerdict::Undecided;
                            line = exit.line;
                            reason = why_of(other);
                        }
                    }
                }
            }
            let spelled = spell_exit(formal, clause);
            match verdict {
                ClauseVerdict::Refuted => {
                    diagnostics.push(violation(line, format!("`{spelled}` is refuted: {reason}")))
                }
                ClauseVerdict::Undecided => {
                    diagnostics.push(undecided(line, format!("`{spelled}`: {reason}")))
                }
                ClauseVerdict::Held => {}
            }
            records.push(ClauseRecord {
                clause: spelled,
                verdict,
                line,
                reason,
            });
        }
        // Exits no clause covers must restore the count, and no exit may
        // leave an unchecked drop behind.
        let mut verdict = ClauseVerdict::Held;
        let mut reason = String::new();
        let mut line = 0;
        for exit in exits {
            if let Some(drop_line) = exit.state.pending.get(formal)
                && matches!(exit.state.counts.get(formal), Some(Count::Known(_)))
            {
                verdict = ClauseVerdict::Refuted;
                line = exit.line;
                reason = format!(
                    "a user of `{formal}` dropped without a release check (line {drop_line}) reaches this exit with none re-added"
                );
                diagnostics.push(violation(exit.line, reason.clone()));
                break;
            }
            let uncovered = match &exit.returned {
                Returned::Void => true,
                Returned::Undecided => covered.is_empty(),
                Returned::Value(value) => !covered.iter().any(|returned| value.may_be(*returned)),
            };
            if !uncovered {
                continue;
            }
            match exit.state.counts.get(formal) {
                Some(Count::Known(0)) => {}
                Some(Count::Known(found)) => {
                    verdict = ClauseVerdict::Refuted;
                    line = exit.line;
                    reason =
                        format!("an exit no clause covers has added {found} users of `{formal}`");
                    diagnostics.push(violation(exit.line, reason.clone()));
                    break;
                }
                other => {
                    if verdict == ClauseVerdict::Held {
                        verdict = ClauseVerdict::Undecided;
                        line = exit.line;
                        reason = why_of(other);
                        diagnostics.push(undecided(
                            exit.line,
                            format!("an exit no clause covers, for `{formal}`: {reason}"),
                        ));
                    }
                }
            }
        }
        records.push(ClauseRecord {
            clause: format!("implicit: every uncovered exit restores the users of `{formal}`"),
            verdict,
            line,
            reason,
        });
    }
}

fn why_of(count: Option<&Count>) -> String {
    match count {
        Some(Count::Undecided(reason)) => reason.clone(),
        Some(Count::Known(_)) => String::new(),
        None => "the count is not tracked".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(source: &str) -> Vec<&str> {
        source.split_whitespace().collect()
    }

    #[test]
    fn counter_writes_are_read_in_both_positions() {
        let counters = BTreeSet::from(["users".to_owned()]);
        let texts = words("t -> users ++ ; -- t -> users ; t -> users = 0 ; t -> other ++");
        let write = counter_write(&texts, 0, &counters).unwrap();
        assert_eq!((write.base.as_str(), write.step), ("t", Some(1)));
        let write = counter_write(&texts, 5, &counters).unwrap();
        assert_eq!((write.base.as_str(), write.step), ("t", Some(-1)));
        let write = counter_write(&texts, 10, &counters).unwrap();
        assert_eq!(write.step, None);
        assert!(counter_write(&texts, 16, &counters).is_none());
    }

    #[test]
    fn a_cast_is_not_an_opaque_callee() {
        let texts = words("( struct t * ) ( p )");
        assert!(opaque_callee(&texts, 4).is_none());
        let texts = words("( f ? g : h ) ( p )");
        assert!(opaque_callee(&texts, 6).is_some());
        // Card W45-SR5-SOUNDNESS: this pin read `f(a)(b)` as a named call,
        // against constraint 10c's rule 2; the call of `f`'s result is opaque.
        let texts = words("f ( a ) ( b )");
        assert!(opaque_callee(&texts, 3).is_some());
    }

    #[test]
    fn statements_parse_into_branches_and_exits() {
        let texts = words("f ( t ) ; if ( r == - 1 ) { return - 1 ; } else return 0 ;");
        let statements = parse_block(&texts, 0, texts.len());
        assert_eq!(statements.len(), 2);
        assert!(matches!(
            statements[1],
            Stmt::If {
                otherwise: Some(_),
                ..
            }
        ));
        // Card CG27: a loop whose extent is read is a `Loop` for the walker
        // to summarize or refuse; an unreadable one keeps the refusal.
        let texts = words("do { f ( t ) ; } while ( 0 ) ; for ( ; ; ) ;");
        let statements = parse_block(&texts, 0, texts.len());
        assert!(matches!(statements[0], Stmt::Block(_)));
        assert!(matches!(statements[1], Stmt::Loop((13, 19))));
        let texts = words("while ( x ) { f ( t ) ;");
        let statements = parse_block(&texts, 0, texts.len());
        assert!(matches!(statements[0], Stmt::Unsupported(_)));
    }

    #[test]
    fn literal_values_are_read_through_parentheses() {
        assert_eq!(literal_of(&words("( - 1 )")), Some(-1));
        assert_eq!(literal_of(&words("0")), Some(0));
        assert_eq!(literal_of(&words("x")), None);
    }
}
