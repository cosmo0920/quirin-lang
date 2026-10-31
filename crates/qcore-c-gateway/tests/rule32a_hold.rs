//! Card RULE32A-PILOT (GATEWAY-RULE-API-HOLD-OFF): rule 32a's caller-frame
//! hold is a fixed hold. No `QCORE_GATEWAY_RULES_OFF` list removes it: a list
//! naming `32a` is refused (and the gateway's fallback for a refused list,
//! every switchable rule off, leaves the hold on), and an accepted list
//! leaves it on. `caller_frame_hold.rs` holds the rule's readings with
//! nothing off; this binary re-runs itself as a child with the variable set
//! (`child_*` tests do nothing unless `RULE32A_HOLD_CHILD` names them), since
//! the switch is read once per process.

use std::collections::BTreeMap;
use std::process::Command;

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit, caller_frame_hold,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CHILD: &str = "RULE32A_HOLD_CHILD";

const DECLARATIONS: &str = include_str!(
    "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/log_caller_frame.qv"
);

const PRELUDE: &str = r#"
typedef struct rd_kafka_conf_s { int log_level; int log_queue; } rd_kafka_conf_t;
typedef struct rd_kafka_s { void *rk_logq; } rd_kafka_t;
"#;

/// A member whose only blocker would be its sibling's declaration, and a
/// caller outside the frame in the same unit.
const FRAME: &str = r#"
static void rd_kafka_log_buf(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, int level,
                             int ctx, const char *fac, const char *buf)
{
    return;
}

void rd_kafka_log0(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, const char *extra,
                   int level, int ctx, const char *fac, const char *fmt, ...)
{
    rd_kafka_log_buf(conf, rk, level, ctx, fac, fmt);
}

void outside_caller(const rd_kafka_conf_t *conf, const rd_kafka_t *rk)
{
    rd_kafka_log0(conf, rk, 0, 7, 0, "FAC", "plain");
}
"#;

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "log_caller_frame.qv".to_owned(),
        DECLARATIONS.to_owned(),
    )]))
    .expect("the production caller-frame declarations load")
}

fn analyse(entry: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}\n{FRAME}"), entry, &contracts())
        .expect("the fixture analyses")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn held(summary: &LegacyFunctionSummary, callee: &str) -> bool {
    summary
        .diagnostics
        .iter()
        .any(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => {
                caller_frame_hold::held_call(description) == Some(callee)
            }
            _ => false,
        })
}

/// The member's row is held `Unknown` on its sibling's call.
fn assert_the_member_is_held() {
    let summary = analyse("rd_kafka_log0");
    let log0 = function(&summary, "rd_kafka_log0");
    assert_eq!(log0.status, AnalysisStatus::Unknown, "{log0:#?}");
    assert!(held(log0, "rd_kafka_log_buf"), "{log0:#?}");
}

/// Run `test` of this binary as a child with `QCORE_GATEWAY_RULES_OFF=list`.
fn run_child(test: &str, list: &str) {
    let status = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env(gateway_rules::RULES_OFF_VARIABLE, list)
        .status()
        .expect("the child test runs");
    assert!(status.success(), "{test} with {list:?}: {status}");
}

fn is_child(test: &str) -> bool {
    std::env::var(CHILD).is_ok_and(|named| named == test)
}

#[test]
fn with_nothing_off_the_hold_applies_and_the_outside_caller_reads_the_declaration() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    assert_the_member_is_held();
    let summary = analyse("outside_caller");
    let caller = function(&summary, "outside_caller");
    assert_eq!(caller.status, AnalysisStatus::Safe, "{caller:#?}");
    assert!(!held(caller, "rd_kafka_log0"));
}

#[test]
fn a_list_naming_the_hold_is_refused_and_the_hold_stays() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    run_child("child_hold_named", "32a");
    run_child("child_hold_named", "35,32a");
}

#[test]
fn child_hold_named() {
    if !is_child("child_hold_named") {
        return;
    }
    let refusal = gateway_rules::check().expect_err("a hold is not switchable");
    assert!(refusal.to_string().contains("32a"), "{refusal}");
    assert!(
        refusal.to_string().contains("cannot be switched off"),
        "{refusal}"
    );
    let off = gateway_rules::effective_off();
    assert!(off.contains("35") && off.contains("34"), "{off:?}");
    assert!(
        !off.contains("32a"),
        "the refused list's fallback leaves the hold on"
    );
    assert_the_member_is_held();
}

#[test]
fn an_accepted_list_leaves_the_hold_on() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    run_child("child_other_rules_off", "33,34,35,36,37");
}

#[test]
fn child_other_rules_off() {
    if !is_child("child_other_rules_off") {
        return;
    }
    let off = gateway_rules::check().expect("switchable rules only");
    assert_eq!(off.ids().len(), 5);
    assert!(!gateway_rules::effective_off().contains("32a"));
    assert_the_member_is_held();
    let summary = analyse("outside_caller");
    assert!(!held(function(&summary, "outside_caller"), "rd_kafka_log0"));
}
