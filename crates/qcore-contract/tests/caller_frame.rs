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

//! Card W48-RK-LOG-CALLER-FRAME (gateway rule 32): the caller-frame
//! projection of the reviewed logger model, and the production declarations'
//! exact premise sets. Lean: `QCore.RkLogCallerFrame`.
use qcore_contract::caller_frame::{
    CheckedCallerFrame, Class, Internal, Projection, Standing, check, check_declarations,
};
use std::collections::BTreeSet;

const MODEL: &[u8] = include_bytes!("../../../docs/reports/logger-effect-contracts/model.json");
const PROJECTION: &[u8] =
    include_bytes!("../../../docs/reports/w48-rk-log-caller-frame/caller-frame.json");
const DECLARATIONS: &str = include_str!(
    "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/log_caller_frame.qv"
);
/// Independently held review digests (never derived from the documents).
const MODEL_SHA256: &str = "1c1ffedb049c363f1003c3eaa1e3110c08828375ab85e12e647578a3f423ed18";
const PROJECTION_SHA256: &str = "fe6ae7d7fa0af443c5c4732b217a0db1fcb5047385f970eab686e7ddee8829fa";
const CB: &str = "rk_plugin_callback_closure";
const NOEV: &str = "rk_log_enqueue_no_event_callback";
const FWD: &str = "rk_log_enqueue_forward_not_last";

fn reviewed() -> CheckedCallerFrame {
    check(MODEL, MODEL_SHA256, PROJECTION, PROJECTION_SHA256).unwrap()
}

fn edited(edit: impl FnOnce(&mut Projection)) -> Result<CheckedCallerFrame, String> {
    let mut doc: Projection = serde_json::from_slice(PROJECTION).unwrap();
    edit(&mut doc);
    let bytes = serde_json::to_vec(&doc).unwrap();
    check(
        MODEL,
        MODEL_SHA256,
        &bytes,
        &qcore_schemas::sha256_hex(&bytes),
    )
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn the_reviewed_projection_derives_three_premises_and_keeps_every_obligation_open() {
    let checked = reviewed();
    assert_eq!(checked.premises(), &set(&[CB, NOEV, FWD]));
    assert_eq!(checked.open_obligations().len(), 15);
    let paths = checked.paths();
    for proved in [
        "filtered",
        "queue_missing_terminating",
        "callback_absent",
        "disabled_no_reply",
    ] {
        assert_eq!(paths[proved], Standing::Proved, "{proved}");
    }
    assert_eq!(paths["disabled_reply_queue"], Standing::Infeasible);
    assert_eq!(
        paths["immediate_callback"],
        Standing::Premised {
            premises: set(&[CB])
        }
    );
    assert_eq!(
        paths["local_queue"],
        Standing::Premised {
            premises: set(&[NOEV])
        }
    );
    assert_eq!(
        paths["forwarded_queue"],
        Standing::Premised {
            premises: set(&[NOEV, FWD])
        }
    );
    assert_eq!(
        paths["forwarded_disabled_no_reply"],
        Standing::Premised {
            premises: set(&[FWD])
        }
    );
    assert_eq!(
        paths["consumer_callback_cleanup"],
        Standing::OutsideFrame {
            prefix: "local_queue".into()
        }
    );
}

#[test]
fn the_production_declarations_state_exactly_the_derived_premises() {
    let checked = reviewed();
    assert_eq!(
        checked.declarations(),
        ["rd_kafka_log0".to_owned(), "rd_kafka_log_buf".to_owned()]
    );
    check_declarations(&checked, DECLARATIONS).unwrap();
}

#[test]
fn a_missing_or_extra_premise_refuses() {
    let checked = reviewed();
    let missing = DECLARATIONS.replace(
        "    requires resource_premise(rk_log_enqueue_forward_not_last,",
        "    // requires resource_premise(rk_log_enqueue_forward_not_last,",
    );
    assert!(check_declarations(&checked, &missing).is_err());
    let extra = DECLARATIONS.replace(
        "    requires resource_contract_v1;\n",
        "    requires resource_contract_v1;\n    requires resource_premise(rk_spare, \"spare\");\n",
    );
    assert!(check_declarations(&checked, &extra).is_err());
}

#[test]
fn a_declaration_with_a_custody_effect_or_an_unborrowed_formal_refuses() {
    let checked = reviewed();
    let released = DECLARATIONS.replacen(
        "ensures normal resource_borrowed(buf);",
        "ensures normal resource_released(buf);",
        1,
    );
    assert!(check_declarations(&checked, &released).is_err());
    let unstated = DECLARATIONS.replacen("    ensures normal resource_borrowed(fmt);\n", "", 1);
    assert!(check_declarations(&checked, &unstated).is_err());
    let data_tail = DECLARATIONS.replace(
        "resource_variadic_borrowed_tail()",
        "resource_variadic_data_tail()",
    );
    assert!(check_declarations(&checked, &data_tail).is_err());
    let owned = DECLARATIONS.replacen(
        "ensures normal resource_return_none();",
        "ensures normal resource_return_owned(nullable);",
        1,
    );
    assert!(check_declarations(&checked, &owned).is_err());
    let absent = DECLARATIONS.replace("rd_kafka_log_buf", "rd_kafka_log_buf_renamed");
    assert!(check_declarations(&checked, &absent).is_err());
}

#[test]
fn a_wrong_digest_or_a_rebound_model_refuses() {
    assert!(check(MODEL, PROJECTION_SHA256, PROJECTION, PROJECTION_SHA256).is_err());
    assert!(check(MODEL, MODEL_SHA256, PROJECTION, MODEL_SHA256).is_err());
    assert!(edited(|d| d.model_sha256 = PROJECTION_SHA256.into()).is_err());
    assert!(edited(|d| d.root = "rd_kafka_log_buf".into()).is_err());
}

#[test]
fn an_unclassified_or_invented_obligation_or_event_refuses() {
    assert!(
        edited(|d| {
            d.frame.remove("tls_thread_name");
        })
        .is_err()
    );
    assert!(
        edited(|d| {
            d.frame.insert(
                "invented".into(),
                Class::Internal(Internal {
                    internal: qcore_contract::caller_frame::Argument::LibraryState,
                    reason: "invented".into(),
                }),
            );
        })
        .is_err()
    );
    assert!(
        edited(|d| {
            d.events.remove("take");
        })
        .is_err()
    );
    assert!(
        edited(|d| {
            d.paths.remove("disabled_reply_queue");
        })
        .is_err()
    );
}

#[test]
fn reclassifying_a_caller_visible_effect_as_internal_changes_the_derivation_and_refuses() {
    // The IO-event callback read as queue-internal would prove the local path;
    // the document's stated standing then differs from the derived one.
    assert!(
        edited(|d| {
            d.events.get_mut("transfer").unwrap().obligations.insert(
                "io_event_effects".into(),
                Class::Internal(Internal {
                    internal: qcore_contract::caller_frame::Argument::QueueInternal,
                    reason: "claimed internal".into(),
                }),
            );
        })
        .is_err()
    );
    // Restating the paths to match drops the premise from the derived set, so
    // the production declarations no longer state exactly that set.
    let weaker = edited(|d| {
        d.events.get_mut("transfer").unwrap().obligations.insert(
            "io_event_effects".into(),
            Class::Internal(Internal {
                internal: qcore_contract::caller_frame::Argument::QueueInternal,
                reason: "claimed internal".into(),
            }),
        );
        d.paths.insert("local_queue".into(), Standing::Proved);
        d.paths.insert(
            "forwarded_queue".into(),
            Standing::Premised {
                premises: set(&[FWD]),
            },
        );
    })
    .unwrap();
    assert!(!weaker.premises().contains(NOEV));
    assert!(check_declarations(&weaker, DECLARATIONS).is_err());
}

#[test]
fn a_boundary_or_infeasible_event_must_carry_its_own_argument() {
    // A boundary event may only hold after-frame obligations.
    assert!(
        edited(|d| {
            d.events.get_mut("take").unwrap().obligations.insert(
                "eventual_consumer_cleanup".into(),
                Class::Internal(Internal {
                    internal: qcore_contract::caller_frame::Argument::QueueInternal,
                    reason: "not after the frame".into(),
                }),
            );
        })
        .is_err()
    );
    // An infeasible event may only rest on the fresh object.
    assert!(
        edited(|d| {
            d.events.get_mut("reply").unwrap().obligations.insert(
                "reply_queue_effects".into(),
                Class::Internal(Internal {
                    internal: qcore_contract::caller_frame::Argument::BorrowedRead,
                    reason: "not fresh".into(),
                }),
            );
        })
        .is_err()
    );
}

#[test]
fn an_outside_frame_path_must_name_its_in_frame_prefix() {
    assert!(
        edited(|d| {
            d.paths.insert(
                "consumer_callback_cleanup".into(),
                Standing::OutsideFrame {
                    prefix: "forwarded_queue".into(),
                },
            );
        })
        .is_err()
    );
    assert!(
        edited(|d| {
            d.paths.insert(
                "consumer_callback_cleanup".into(),
                Standing::Premised {
                    premises: set(&[NOEV]),
                },
            );
        })
        .is_err()
    );
}

/// Rule 32a: the gateway's frame registry is exactly the reviewed
/// projection's declarations, and membership is read only within one frame.
#[test]
fn the_frame_registry_is_the_reviewed_projection_declarations() {
    use qcore_contract::caller_frame::{FRAMES, is_frame_member, within_one_frame};
    let checked = reviewed();
    let declared: BTreeSet<&str> = checked.declarations().iter().map(String::as_str).collect();
    assert_eq!(FRAMES.len(), 1);
    let registered: BTreeSet<&str> = FRAMES[0].iter().copied().collect();
    assert_eq!(registered, declared);
    assert_eq!(registered.len(), FRAMES[0].len(), "no duplicate member");
    assert!(within_one_frame("rd_kafka_log0", "rd_kafka_log_buf"));
    assert!(within_one_frame("rd_kafka_log_buf", "rd_kafka_log0"));
    assert!(within_one_frame("rd_kafka_log0", "rd_kafka_log0"));
    assert!(!within_one_frame(
        "rd_kafka_topic_set_state",
        "rd_kafka_log0"
    ));
    assert!(!within_one_frame("rd_kafka_log0", "rd_kafka_q_enq"));
    assert!(is_frame_member("rd_kafka_log_buf"));
    assert!(!is_frame_member("cb_kafka_logger"));
}
