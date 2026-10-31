//! Rule 32a's token hold: the decision table, its Lean binding, the reserved
//! reason key and the hold through the rule API's consultation (the
//! gateway's host is exercised by the gateway's `caller_frame_hold.rs` and
//! `rule32a_hold.rs`).

use qcore_gateway_rule_api::traits::{Hold, HoldQuery, Host, RulePackage, TokenHold};
use qcore_gateway_rule_api::{Answer, ReasonCategory, check_reservations, consult};

use super::*;

const LOGGER: &[&str] = &["rd_kafka_log_buf", "rd_kafka_log0"];
const OTHER: &[&str] = &["other_a", "other_b"];
const FRAMES: &[&[&str]] = &[LOGGER, OTHER];

fn held(answer: &Answer<Hold>) -> bool {
    matches!(answer, Answer::Admit(_))
}

#[test]
fn the_decision_vectors_hold() {
    for &(caller_member, callee_member, in_force, expected) in DECISION_VECTORS {
        let caller = if caller_member {
            "rd_kafka_log0"
        } else {
            "outside"
        };
        let callee = if callee_member {
            "rd_kafka_log_buf"
        } else {
            "rd_kafka_q_enq"
        };
        assert_eq!(
            held(&decide(&[LOGGER], caller, callee, in_force)),
            expected,
            "{caller_member} {callee_member} {in_force}"
        );
    }
    assert_eq!(DECISION_VECTORS.len(), 8, "every shape of one frame");
}

#[test]
fn the_lean_hold_vectors_are_the_rust_table() {
    let lean =
        include_str!("../../../../../proof/QCore/CallerFrameHoldSwitch.lean").replace("\r\n", "\n");
    let block = lean
        .split("def holdVectors ")
        .nth(1)
        .expect("holdVectors in the Lean model");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    let rows = block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let fields = row
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(|field| match field.trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("not a Boolean: {other}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 4, "{row}");
            (fields[0], fields[1], fields[2], fields[3])
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, DECISION_VECTORS.to_vec());
}

#[test]
fn a_member_calling_a_member_is_held_under_the_reserved_key() {
    for (caller, callee) in [
        ("rd_kafka_log0", "rd_kafka_log_buf"),
        ("rd_kafka_log_buf", "rd_kafka_log0"),
        ("rd_kafka_log0", "rd_kafka_log0"),
    ] {
        assert_eq!(
            decide(FRAMES, caller, callee, true),
            Answer::admit(Hold {
                reason: SELF_RELIANCE
            }),
            "{caller} -> {callee}"
        );
    }
}

#[test]
fn an_outside_caller_and_an_undeclared_callee_are_not_held() {
    assert_eq!(
        decide(FRAMES, "rd_kafka_mock_cgrp", "rd_kafka_log0", true),
        Answer::Refuse,
        "a caller outside the frame reads the declaration"
    );
    assert_eq!(
        decide(FRAMES, "rd_kafka_log0", "rd_kafka_log_buf", false),
        Answer::Refuse,
        "no declaration in force: nothing to hold"
    );
    assert_eq!(
        decide(FRAMES, "rd_kafka_log0", "other_a", true),
        Answer::Refuse,
        "members of two different frames"
    );
    assert_eq!(
        decide(&[], "rd_kafka_log0", "rd_kafka_log0", true),
        Answer::Refuse
    );
}

#[test]
fn the_reserved_key_is_the_registered_one() {
    assert_eq!(SELF_RELIANCE.key, "caller-frame-self-reliance");
    assert_eq!(
        SELF_RELIANCE.category,
        ReasonCategory::IncompleteObligationCoverage
    );
    assert_eq!(check_reservations([(crate::ID, &SELF_RELIANCE)]), Ok(()));
    let registry = include_str!("../../../registry.toml");
    let row = registry
        .split("[[rule]]")
        .find(|row| row.contains("id = \"32a\""))
        .expect("rule 32a is registered");
    assert!(row.contains("crate = \"rule-32a-caller-frame-hold\""));
    assert!(row.contains("off = \"not-switchable\""));
    assert!(row.contains(
        "{ key = \"caller-frame-self-reliance\", category = \"incomplete-obligation-coverage\" }"
    ));
}

/// A host with string tokens and a fixed declaration set, enough to drive
/// the package through `consult`.
struct Toy {
    declared: &'static [&'static str],
}

impl Host for Toy {
    type Token = &'static str;
    type Decision = ();
    type Statement = ();
    type Condition = ();
    type Node = ();

    fn token_text(token: &Self::Token) -> &str {
        token
    }
    fn token_line(_: &Self::Token) -> usize {
        1
    }
    fn refusal(_: &Self::Decision) -> Option<&str> {
        None
    }
}

impl FrameHost for Toy {
    fn frames(&self) -> &[&[&str]] {
        FRAMES
    }
    fn declaration_in_force(&self, function: &str) -> bool {
        self.declared.contains(&function)
    }
}

#[test]
fn the_hold_answers_through_the_consultation() {
    let host = Toy {
        declared: &["rd_kafka_log_buf", "rd_kafka_log0"],
    };
    let call = ["rd_kafka_log_buf", "(", ")"];
    let ask = |caller: &'static str, callee: &'static str| {
        let query = HoldQuery::<Toy> {
            caller,
            callee,
            call: &call,
            line: 3,
        };
        consult(
            &[CallerFrameHold],
            |_| true,
            |_, package| TokenHold::ask(package, &host, &query),
        )
    };
    let decided = ask("rd_kafka_log0", "rd_kafka_log_buf").expect("held");
    assert_eq!(decided.admission.lowering.reason, SELF_RELIANCE);
    assert!(decided.admission.obligations.is_empty());
    assert!(ask("caller_outside", "rd_kafka_log_buf").is_none());
    let undeclared = Toy { declared: &[] };
    let query = HoldQuery::<Toy> {
        caller: "rd_kafka_log0",
        callee: "rd_kafka_log_buf",
        call: &call,
        line: 3,
    };
    assert_eq!(
        TokenHold::ask(&CallerFrameHold, &undeclared, &query),
        Answer::Refuse
    );
    assert_eq!(CallerFrameHold.id(), "32a");
}
