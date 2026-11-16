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

//! Who said what in a composed row (card **W48-PLUGIN-VERDICT-PRESERVATION**;
//! language specification §L21.2, contract specification §C11.1–§C11.2).
//!
//! A published row's verdict is the join (`Safe < Pending < Unknown <
//! Unsafe`) of its contributors' verdicts, and its reasons are the union of
//! theirs. The join alone forgets who contributed what: a `Pending` row does
//! not say which plugin was uncertified, nor that the plugin itself proposed
//! `Unsafe`. So every row a plugin contributes to keeps each contributor's
//! record — its [`Source`], what a plugin answered before the firewall
//! classified it ([`Answer`]), the verdict it contributes and its reasons —
//! and this module recovers them ([`project_row`], [`project_function`],
//! [`project_unit`]), refusing a row whose published value is not the join of
//! the records it carries.
//!
//! Two representations carry the records:
//!
//! * an external subcommand's report row (`quirin-audit NAME`,
//!   [`crate::subcommand_plugins`]) carries them explicitly as
//!   `contributions`, and its `verdict` and `reasons` are [`compose`] of them;
//! * a checker plugin's function row ([`crate::user_plugins`]) carries the
//!   function's own status as `host_status` and every plugin's record in
//!   `plugin_links` (each with its `answer`); its unit row carries the unit's
//!   own verdict as `host_verdict`, and the unit's records are its own, its
//!   linked functions' own and their links. A `.qo` checkpoint keeps both
//!   byte for byte, so a composed report projects as the whole run's does.
//!
//! The answer is a record, not evidence: only the firewall's classification
//! reaches a verdict (standing rule 10h). The Lean model is the
//! `Contributions` section of `QCore.VerdictSemigroup`.

use qcore_schemas::verdict::{ReasonedVerdict, Verdict, VerdictReason};
use qcore_verifier_host::{PluginAnswer, PluginClaim, ProposedVerdict};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What kind of contributor a record comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// The host's own analysis (an audit report row, a function's or unit's
    /// own status).
    Host,
    /// A subcommand plugin's run as a whole: `Safe` when it completed,
    /// `Unknown` when it failed.
    Run,
    /// A plugin's classified answer.
    Plugin,
}

/// A contributor: its kind and its identity (a plugin's id or
/// `quirin-audit-NAME@<sha256 prefix>` label; the host's subject).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub kind: SourceKind,
    pub label: String,
}

impl Source {
    #[must_use]
    pub fn new(kind: SourceKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
        }
    }
}

/// What a plugin itself answered, before the firewall classified it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The verdict the plugin proposed (`Safe`, `Unsafe` or `Unknown`), or
    /// `None` when no answer was read (a gate, spawn, transport or protocol
    /// failure).
    pub proposed: Option<String>,
    /// The firewall's class of the answer ([`claim_kind`]).
    pub claim: String,
    /// The firewall's refusal code, when the class is a refusal.
    pub code: Option<String>,
    /// The plugin's own reasons, as it gave them, in its order.
    pub reasons: Vec<VerdictReason>,
}

impl Answer {
    /// A checker plugin's answer (§C11.1): its proposal and its own
    /// diagnostics, beside the class.
    #[must_use]
    pub fn of_checker(answer: &PluginAnswer) -> Self {
        Self {
            proposed: answer.proposed.map(proposed_spelling).map(str::to_owned),
            claim: claim_kind(&answer.claim).to_owned(),
            code: answer.claim.code().map(str::to_owned),
            reasons: answer
                .diagnostics
                .iter()
                .map(|diagnostic| VerdictReason {
                    code: diagnostic.code.clone(),
                    detail: diagnostic.message.clone(),
                })
                .collect(),
        }
    }

    /// A subcommand plugin's claim (§C11.2): its proposal and its reason.
    #[must_use]
    pub fn of_claim(proposed: &str, reason: &str, classified: &PluginClaim) -> Self {
        Self {
            proposed: Some(proposed.to_owned()),
            claim: claim_kind(classified).to_owned(),
            code: classified.code().map(str::to_owned),
            reasons: vec![VerdictReason {
                code: "plugin-reason".into(),
                detail: reason.to_owned(),
            }],
        }
    }
}

/// The report spelling of a proposal.
#[must_use]
pub const fn proposed_spelling(proposed: ProposedVerdict) -> &'static str {
    match proposed {
        ProposedVerdict::Safe => "Safe",
        ProposedVerdict::Unsafe => "Unsafe",
        ProposedVerdict::Unknown => "Unknown",
    }
}

/// The firewall's class of a claim, as every plugin record spells it.
#[must_use]
pub const fn claim_kind(claim: &PluginClaim) -> &'static str {
    match claim {
        PluginClaim::CertifiedSafe(_) => "certified-safe",
        PluginClaim::ValidatedUnsafe { .. } => "validated-unsafe",
        PluginClaim::UncertifiedSafe { .. } => "uncertified-safe",
        PluginClaim::UnvalidatedUnsafe { .. } => "unvalidated-unsafe",
        PluginClaim::Undecided { .. } => "undecided",
        PluginClaim::Failure { .. } => "failure",
    }
}

/// One contributor's record in a composed row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contribution {
    pub source: Source,
    /// A plugin's own answer; `None` for the host and a run.
    pub answer: Option<Answer>,
    /// The verdict it contributes, with its reasons.
    pub verdict: ReasonedVerdict,
}

impl Contribution {
    /// The record as a row carries it.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "source": self.source,
            "answer": self.answer,
            "verdict": self.verdict.verdict(),
            "reasons": self.verdict.reasons(),
        })
    }
}

/// The published reasoned verdict of a row: the join of its records' values
/// and the union of their reasons (`Row.published_is_the_fold`).
#[must_use]
pub fn compose(contributions: &[Contribution]) -> ReasonedVerdict {
    ReasonedVerdict::fold(
        contributions
            .iter()
            .map(|contribution| contribution.verdict.clone()),
    )
}

/// A record recovered from a published row.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recovered {
    pub source: Source,
    pub answer: Option<Answer>,
    pub verdict: Verdict,
    pub reasons: Vec<VerdictReason>,
}

/// The records of one source (`Row.project`).
#[must_use]
pub fn of_source<'a>(records: &'a [Recovered], source: &Source) -> Vec<&'a Recovered> {
    records
        .iter()
        .filter(|record| &record.source == source)
        .collect()
}

fn verdict_of(value: &Value, what: &str) -> Result<Verdict, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{what} has no verdict"))?
        .parse()
}

fn reasons_of(value: &Value, what: &str) -> Result<Vec<VerdictReason>, String> {
    serde_json::from_value(value.clone()).map_err(|error| format!("{what}: reasons: {error}"))
}

/// Refuse records whose join is not the published value.
fn check_join(records: &[Recovered], published: Verdict, what: &str) -> Result<(), String> {
    let joined = Verdict::fold(records.iter().map(|record| record.verdict));
    if joined != published {
        return Err(format!(
            "{what} publishes {published}, but the join of its {} record(s) is {joined}",
            records.len()
        ));
    }
    Ok(())
}

/// Recover every record of a row that carries them as `contributions` (an
/// external subcommand's report row), checking that its `verdict` is their
/// join and its `reasons` exactly their union.
///
/// # Errors
///
/// A row without records, a malformed record, or a published value or
/// reason multiset that is not the composition of the records.
pub fn project_row(row: &Value) -> Result<Vec<Recovered>, String> {
    let records: Vec<Recovered> = serde_json::from_value(
        row.get("contributions")
            .cloned()
            .ok_or("the row carries no `contributions`")?,
    )
    .map_err(|error| format!("contributions: {error}"))?;
    let published = verdict_of(&row["verdict"], "the row")?;
    check_join(&records, published, "the row")?;
    let mut expected = records
        .iter()
        .flat_map(|record| record.reasons.iter().cloned())
        .collect::<Vec<_>>();
    expected.sort();
    let mut reasons = reasons_of(&row["reasons"], "the row")?;
    reasons.sort();
    if reasons != expected {
        return Err("the row's reasons are not the union of its records' reasons".into());
    }
    Ok(records)
}

/// The records of one `plugin_links` entry.
fn link_record(link: &Value) -> Result<Recovered, String> {
    let plugin = link["plugin"]
        .as_str()
        .ok_or("a plugin link names no plugin")?;
    let answer = serde_json::from_value(link["answer"].clone())
        .map_err(|error| format!("plugin link `{plugin}`: answer: {error}"))?;
    Ok(Recovered {
        source: Source::new(SourceKind::Plugin, plugin),
        answer: Some(answer),
        verdict: verdict_of(&link["verdict"], "a plugin link")?,
        reasons: reasons_of(&link["reasons"], "a plugin link")?,
    })
}

/// The records of a function row that applies a bound contract: the
/// function's own status (`host_status`) and each plugin link. A function
/// row without links has none.
fn function_records(function: &Value) -> Result<Vec<Recovered>, String> {
    let Some(links) = function.get("plugin_links") else {
        return Ok(Vec::new());
    };
    let name = function["function"].as_str().unwrap_or_default();
    let mut records = vec![Recovered {
        source: Source::new(SourceKind::Host, name),
        answer: None,
        verdict: verdict_of(&function["host_status"], "a linked function row's host")?,
        reasons: Vec::new(),
    }];
    for link in links.as_array().ok_or("`plugin_links` is not an array")? {
        records.push(link_record(link)?);
    }
    Ok(records)
}

/// Recover every record of a checker-plugin function row (§C11.1) and check
/// that its `status` is their join. A row no plugin linked has none.
///
/// # Errors
///
/// A malformed record, or a status that is not the join of the records.
pub fn project_function(function: &Value) -> Result<Vec<Recovered>, String> {
    let records = function_records(function)?;
    if !records.is_empty() {
        check_join(
            &records,
            verdict_of(&function["status"], "a function row")?,
            "a function row",
        )?;
    }
    Ok(records)
}

/// Recover every record of a checker-plugin unit row: the unit's own verdict
/// (`host_verdict`), and each linked function's own status and links; check
/// that the unit's `status` is their join. A unit no plugin linked has none.
///
/// # Errors
///
/// A malformed record, or a status that is not the join of the records.
pub fn project_unit(unit: &Value) -> Result<Vec<Recovered>, String> {
    let Some(host) = unit.get("host_verdict") else {
        return Ok(Vec::new());
    };
    let file = unit["file"].as_str().unwrap_or_default();
    let mut records = vec![Recovered {
        source: Source::new(SourceKind::Host, file),
        answer: None,
        verdict: verdict_of(&host["status"], "a linked unit row's host")?,
        reasons: Vec::new(),
    }];
    for function in unit["functions"].as_array().into_iter().flatten() {
        records.extend(project_function(function)?);
    }
    check_join(
        &records,
        verdict_of(&unit["status"], "a unit row")?,
        "a unit row",
    )?;
    Ok(records)
}

#[cfg(test)]
mod tests {
    use qcore_schemas::reason::ReasonCategory;
    use qcore_schemas::verdict::PendingReason;

    use super::*;

    fn plugin(label: &str, proposed: &str, verdict: ReasonedVerdict) -> Contribution {
        Contribution {
            source: Source::new(SourceKind::Plugin, label),
            answer: Some(Answer {
                proposed: Some(proposed.into()),
                claim: "test".into(),
                code: None,
                reasons: vec![VerdictReason {
                    code: "plugin-reason".into(),
                    detail: format!("{label} says {proposed}"),
                }],
            }),
            verdict,
        }
    }

    fn row(contributions: &[Contribution]) -> Value {
        let published = compose(contributions);
        json!({
            "contributions": contributions.iter().map(Contribution::to_json).collect::<Vec<_>>(),
            "verdict": published.verdict(),
            "reasons": published.reasons(),
        })
    }

    fn pending(detail: &str) -> ReasonedVerdict {
        ReasonedVerdict::pending(PendingReason::PluginClaimUncertified, detail)
    }

    fn unknown(detail: &str) -> ReasonedVerdict {
        ReasonedVerdict::unknown(ReasonCategory::ProviderFailure, detail).unwrap()
    }

    /// Two plugins disagreeing beside a `Safe` host: the row is their join,
    /// and each plugin's own answer and reasons come back out.
    #[test]
    fn two_disagreeing_plugins_are_each_recovered() {
        let host = Contribution {
            source: Source::new(SourceKind::Host, "audit-report"),
            answer: None,
            verdict: ReasonedVerdict::safe(),
        };
        let a = plugin("a", "Safe", pending("a uncertified"));
        let b = plugin("b", "Unknown", unknown("b failed"));
        let row = row(&[host, a.clone(), b.clone()]);
        assert_eq!(row["verdict"], "Unknown");
        let records = project_row(&row).unwrap();
        for contribution in [&a, &b] {
            let own = of_source(&records, &contribution.source);
            assert_eq!(own.len(), 1);
            assert_eq!(own[0].answer, contribution.answer);
            assert_eq!(own[0].verdict, contribution.verdict.verdict());
            assert_eq!(own[0].reasons, contribution.verdict.reasons());
        }
    }

    /// A row whose published value or reasons are not the composition of
    /// its records is refused: nothing can be published above or below them.
    #[test]
    fn a_row_that_is_not_its_join_is_refused() {
        let a = plugin("a", "Safe", pending("a uncertified"));
        let mut lowered = row(std::slice::from_ref(&a));
        lowered["verdict"] = json!("Safe");
        assert!(project_row(&lowered).unwrap_err().contains("join"));
        let mut erased = row(std::slice::from_ref(&a));
        erased["reasons"] = json!([]);
        assert!(project_row(&erased).unwrap_err().contains("union"));
        let mut dropped = row(&[a.clone(), plugin("b", "Unknown", unknown("b"))]);
        dropped["contributions"].as_array_mut().unwrap().pop();
        assert!(project_row(&dropped).is_err());
    }

    /// The checker route's representation projects the same way, and a unit
    /// raised by a link is the join of its own, its function's and the link.
    #[test]
    fn checker_rows_project_to_host_and_links() {
        let link = json!({
            "callee": "acme_pool_get", "plugin": "acme.pool_check", "crate": "c", "module": "m",
            "verdict": "Pending",
            "reasons": [{"code": "plugin-claim-uncertified", "detail": "d"}],
            "answer": {"proposed": "Unsafe", "claim": "unvalidated-unsafe", "code": "QP010",
                       "reasons": [{"code": "acme.pool_check.leak", "detail": "it leaks"}]},
        });
        let function = json!({"function": "use_pool", "status": "Pending",
                              "host_status": "Safe", "plugin_links": [link]});
        let unit = json!({"file": "pool_user.c", "status": "Pending",
                          "host_verdict": {"status": "Safe", "reason": null, "reason_code": null},
                          "functions": [function, {"function": "no_pool", "status": "Safe"}]});
        let records = project_unit(&unit).unwrap();
        assert_eq!(records.len(), 3);
        let own = of_source(
            &records,
            &Source::new(SourceKind::Plugin, "acme.pool_check"),
        );
        assert_eq!(own[0].verdict, Verdict::Pending);
        let answer = own[0].answer.as_ref().unwrap();
        assert_eq!(answer.proposed.as_deref(), Some("Unsafe"));
        assert_eq!(answer.reasons[0].detail, "it leaks");
        let mut lowered = unit.clone();
        lowered["status"] = json!("Safe");
        assert!(project_unit(&lowered).is_err());
        let mut unlinked = unit;
        unlinked["functions"][0]["status"] = json!("Safe");
        assert!(project_unit(&unlinked).is_err());
    }
}
