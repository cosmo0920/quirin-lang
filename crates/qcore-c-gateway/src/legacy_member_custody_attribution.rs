//! Card **MEMBER-EFFECT-SUMMARY** step 1 (gateway rule 38i): the gate's
//! refusal of a statement whose memory access is member custody through a
//! formal parent names that premise. A re-attribution, never an admission.
//!
//! ## The gap
//!
//! The lexical gate of [`super::Parser::classify_simple_base`] refuses every
//! statement with an access outside its admitted spans as `indirect memory
//! access is outside the resource projection`. MEMBER-ACCESS-PROJECTION's
//! census of the 046accf6 `current` run (report section 1) found that a large
//! part of the sole-blocker mass of that family is **custody**, not typing:
//! a body releases a member of a formal parent and keeps the parent (rule 5k
//! admits a member release only when the body also releases the parent, P4),
//! stores a pointer that may carry an identity into a member of a formal
//! (`confval->name = name`, `map->unit = cfl_sds_create(unit)`), or hands the
//! address of a formal's embedded member to a releasing callee. The leaf
//! names none of these, so the next census cannot rank them.
//!
//! ## The rule
//!
//! Asked **last** (after every route of the statement, rule 38d's close
//! included, declined), only for exactly the gate's refusal of a statement
//! with no braces that reads no tracked descriptor. The base `P` below is a
//! formal parameter no statement of the body writes or takes the address of
//! (`CallerFormals`, rule 5j's reading: `P` designates the caller's object
//! throughout the body), and the statement is one of:
//!
//! * **release** (`member-release-through-formal`): a whole-statement call
//!   `R(..)` whose contract in force (no review or pin hold) states
//!   `Released` at the position of an actual spelled exactly `P->F`;
//! * **address** (`member-address-release-through-formal`): the same, with
//!   the actual spelled exactly `&P->F` or `&P->F.G…`;
//! * **store** (`member-store-through-formal`): `P->F = V` with `V` a plain
//!   name the lexical reader declares a pointer (a formal or local that may
//!   carry custody), or a whole call `g(..)` whose actuals read no memory and
//!   whose contract in force (no hold) returns custody (`Owned` or
//!   `Reallocated`).
//!
//! The two call forms are confirmed as rule 12k confirms its callee: the
//! statement is classified again with every memory-reading actual read as at
//! a `Borrowed` position ([`super::Parser::reclassified_with_borrowed_actuals`]);
//! only when that reading **admits** the call (so the custody position of the
//! member actual is what refuses it) does the leaf take the name. The store
//! form needs no reading: its only access is the destination `P->F`, and its
//! value is custody-carrying by the conditions above.
//!
//! The node keeps the gate's writes; only the description changes, so no
//! verdict moves (a non-held refusal is `Unknown` whatever its text, and none
//! of the three names is a held form). Every other answer keeps the gate's
//! text byte for byte. Parity runs record every consultation
//! (`position: "member-custody"`).
//!
//! The Lean model is `QCore.MemberCustodyAttribution`.

use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

use super::{
    LegacyInstruction, Parser, Statement, Token, call_argument_access::GATE_REFUSAL,
    call_argument_spans, contains_braces, contains_unmodeled_access, find_call, is_null_constant,
    pin_hold, review_hold, strip_parentheses,
};
use crate::typed_field_access::{DeclaredType, is_name};

/// The key ending a member release's named refusal.
pub(super) const RELEASE_KEY: &str = "member-release-through-formal";
/// The key ending a member store's named refusal.
pub(super) const STORE_KEY: &str = "member-store-through-formal";
/// The key ending a member address release's named refusal.
pub(super) const ADDRESS_KEY: &str = "member-address-release-through-formal";

/// The premise one gate refusal is re-attributed to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Premise {
    /// `R(.., P->F, ..)` at a `Released` position.
    Release { base: String, member: String },
    /// `&P->F[.G…]` at a `Released` position; `path` is `F[.G…]`.
    Address { base: String, path: String },
    /// `P->F = V`, `V` custody-carrying.
    Store { base: String, member: String },
}

impl Premise {
    /// The named refusal.
    #[must_use]
    pub(super) fn description(&self) -> String {
        match self {
            Self::Release { base, member } => format!(
                "release of `{base}->{member}`, a member of the formal `{base}` ({RELEASE_KEY})"
            ),
            Self::Address { base, path } => format!(
                "release of `&{base}->{path}`, an address inside the formal `{base}` \
                 ({ADDRESS_KEY})"
            ),
            Self::Store { base, member } => format!(
                "store of a custody-carrying value into `{base}->{member}`, a member of the \
                 formal `{base}` ({STORE_KEY})"
            ),
        }
    }

    fn key(&self) -> &'static str {
        match self {
            Self::Release { .. } => RELEASE_KEY,
            Self::Address { .. } => ADDRESS_KEY,
            Self::Store { .. } => STORE_KEY,
        }
    }
}

/// The candidate premise of one call actual, before the confirming reading.
fn actual_premise(argument: &[&str]) -> Option<Premise> {
    match strip_parentheses(argument) {
        [base, "-", ">", member] if is_name(base) && is_name(member) => Some(Premise::Release {
            base: (*base).to_owned(),
            member: (*member).to_owned(),
        }),
        ["&", base, "-", ">", member, rest @ ..]
            if is_name(base)
                && is_name(member)
                && rest.len() % 2 == 0
                && rest
                    .chunks(2)
                    .all(|pair| pair[0] == "." && is_name(pair[1])) =>
        {
            let mut path = (*member).to_owned();
            for pair in rest.chunks(2) {
                path.push('.');
                path.push_str(pair[1]);
            }
            Some(Premise::Address {
                base: (*base).to_owned(),
                path,
            })
        }
        _ => None,
    }
}

/// The index of the statement's only top-level simple `=`, if any.
fn simple_assignment(texts: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    let mut found = None;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" if depth == 0 => {
                let previous = index.checked_sub(1).map(|p| texts[p]);
                if texts.get(index + 1) == Some(&"=")
                    || previous.is_some_and(|p| {
                        matches!(
                            p,
                            "=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
                        )
                    })
                {
                    return None;
                }
                if found.is_some() {
                    return None;
                }
                found = Some(index);
            }
            _ => {}
        }
    }
    found
}

/// The index of the `)` closing the `(` at `open`.
fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" => depth += 1,
            ")" => {
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

/// Whether `callee` has a review or pin hold: its refusal is `Pending` and
/// names its own reason, so it is never re-attributed.
fn held(callee: &str) -> bool {
    review_hold::held_callee(callee).is_some() || pin_hold::held_callee(callee).is_some()
}

impl Parser<'_> {
    /// Rule 38i: `statement`, the lowering of `tokens`, with the gate's
    /// refusal named by its member-custody premise, or unchanged.
    pub(super) fn member_custody_attribution(
        &mut self,
        tokens: &[Token],
        line: usize,
        statement: Statement,
    ) -> Statement {
        let Statement::Instruction(
            LegacyInstruction::Unknown {
                description,
                writes,
            },
            at,
        ) = statement
        else {
            return statement;
        };
        if description != GATE_REFUSAL {
            return Statement::Instruction(
                LegacyInstruction::Unknown {
                    description,
                    writes,
                },
                at,
            );
        }
        match self.member_custody_premise(tokens, line) {
            Some(premise) => Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: premise.description(),
                    writes,
                },
                at,
            ),
            None => Statement::Instruction(
                LegacyInstruction::Unknown {
                    description,
                    writes,
                },
                at,
            ),
        }
    }

    /// The premise the gate's refusal of `tokens` is re-attributed to, or
    /// `None` (module documentation).
    fn member_custody_premise(&mut self, tokens: &[Token], line: usize) -> Option<Premise> {
        if contains_braces(tokens) {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !self.descriptor_names.is_empty()
            && super::descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        let (premise, answer) = match simple_assignment(&texts) {
            Some(equal) => self.store_premise(&texts, equal),
            None => self.call_premise(tokens, &texts),
        };
        if let Some(premise) = &premise {
            self.record_member_custody(&texts, line, premise.key(), "attributed");
        } else if let Some(answer) = answer {
            self.record_member_custody(&texts, line, "none", answer);
        }
        premise
    }

    /// The store form. The second value is why a candidate was declined
    /// (parity only).
    fn store_premise(
        &self,
        texts: &[&str],
        equal: usize,
    ) -> (Option<Premise>, Option<&'static str>) {
        let [base, "-", ">", member] = strip_parentheses(&texts[..equal]) else {
            return (None, None);
        };
        if !is_name(base) || !is_name(member) {
            return (None, None);
        }
        if !self.formal_parameters.contains(*base) {
            return (None, None);
        }
        if !self.caller_formals.contains(base) {
            return (None, Some("the formal is rebound"));
        }
        let value = strip_parentheses(&texts[equal + 1..]);
        let carries = match value {
            [name] if is_name(name) && !is_null_constant(value) => {
                self.typed_types.pointer_kind(name).is_some()
                    || matches!(
                        self.typed_types.declared_type(name),
                        Some(DeclaredType::RecordPointer(_))
                    )
            }
            [callee, "(", .., ")"]
                if is_name(callee)
                    && matching_close(value, 1) == Some(value.len() - 1)
                    && !contains_unmodeled_access(value) =>
            {
                !held(callee)
                    && self.contracts.contract(callee).is_some_and(|contract| {
                        matches!(
                            contract.result,
                            ResourceReturnMode::Owned { .. }
                                | ResourceReturnMode::Reallocated { .. }
                        )
                    })
            }
            _ => false,
        };
        if !carries {
            return (None, Some("the value carries no custody this rule reads"));
        }
        (
            Some(Premise::Store {
                base: (*base).to_owned(),
                member: (*member).to_owned(),
            }),
            None,
        )
    }

    /// The two call forms, confirmed by the borrowed reading.
    fn call_premise(
        &mut self,
        tokens: &[Token],
        texts: &[&str],
    ) -> (Option<Premise>, Option<&'static str>) {
        let Some((0, callee)) = find_call(texts) else {
            return (None, None);
        };
        if matching_close(texts, 1) != Some(texts.len() - 1) {
            return (None, None);
        }
        let Some(spans) = call_argument_spans(texts, 0) else {
            return (None, None);
        };
        let Some(contract) = self.contracts.contract(callee) else {
            return (None, None);
        };
        if held(callee) {
            return (None, None);
        }
        let mut candidate = None;
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let argument = &texts[start..end];
            if !contains_unmodeled_access(argument) {
                continue;
            }
            if !matches!(
                contract.parameters.get(index),
                Some(ResourceParameterMode::Released { .. })
            ) {
                continue;
            }
            let Some(premise) = actual_premise(argument) else {
                return (None, Some("a released actual of another shape"));
            };
            if candidate.is_some() {
                return (None, Some("two released member actuals"));
            }
            candidate = Some(premise);
        }
        let Some(premise) = candidate else {
            return (None, None);
        };
        let (Premise::Release { base, .. } | Premise::Address { base, .. }) = &premise else {
            unreachable!("a call actual is a release or an address");
        };
        if !self.formal_parameters.contains(base.as_str()) {
            return (None, None);
        }
        if !self.caller_formals.contains(base) {
            return (None, Some("the formal is rebound"));
        }
        match self.reclassified_with_borrowed_actuals(tokens) {
            Some(LegacyInstruction::Unknown { .. }) => {
                (None, Some("refused with the actuals read as borrowed"))
            }
            Some(_) => (Some(premise), None),
            None => (None, Some("an actual neither reader recognises")),
        }
    }

    /// Parity only: one row per consultation that reached a candidate.
    fn record_member_custody(&self, texts: &[&str], line: usize, premise: &str, answer: &str) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "member-custody",
            "rule": "38i",
            "premise": premise,
            "answer": answer,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<&str> {
        text.split(' ').collect()
    }

    #[test]
    fn actual_shapes() {
        assert_eq!(
            actual_premise(&words("opts - > name")),
            Some(Premise::Release {
                base: "opts".into(),
                member: "name".into()
            })
        );
        assert_eq!(
            actual_premise(&words("( opts - > name )")),
            Some(Premise::Release {
                base: "opts".into(),
                member: "name".into()
            })
        );
        assert_eq!(
            actual_premise(&words("& conf - > interceptors . on_new")),
            Some(Premise::Address {
                base: "conf".into(),
                path: "interceptors.on_new".into()
            })
        );
        assert_eq!(actual_premise(&words("opts - > a - > b")), None);
        assert_eq!(actual_premise(&words("opts [ 0 ]")), None);
        assert_eq!(actual_premise(&words("& conf - > list [ 1 ]")), None);
        assert_eq!(actual_premise(&words("* opts")), None);
    }

    #[test]
    fn only_a_simple_top_level_assignment() {
        assert_eq!(simple_assignment(&words("p - > a = q")), Some(4));
        assert_eq!(simple_assignment(&words("p - > a + = q")), None);
        assert_eq!(simple_assignment(&words("f ( a == b )")), None);
        assert_eq!(simple_assignment(&words("p - > a = q = r")), None);
        assert_eq!(simple_assignment(&words("f ( x = 1 )")), None);
        assert_eq!(simple_assignment(&words("p - > a != q")), None);
    }

    #[test]
    fn every_name_ends_with_its_key_and_is_no_held_form() {
        let premises = [
            Premise::Release {
                base: "p".into(),
                member: "a".into(),
            },
            Premise::Address {
                base: "p".into(),
                path: "a.b".into(),
            },
            Premise::Store {
                base: "p".into(),
                member: "a".into(),
            },
        ];
        for premise in premises {
            let description = premise.description();
            assert!(description.ends_with(&format!("({})", premise.key())));
            assert!(!super::super::verdict::is_held_call(&description));
            assert_ne!(description, GATE_REFUSAL);
        }
    }
}
