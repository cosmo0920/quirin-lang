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

//! Card **W45-AUDIT-CHECKPOINTS**: streaming composition of per-configuration `.qo`
//! checkpoints into one report.
//!
//! [`compose`] first opens and fully verifies every checkpoint, one section at
//! a time, and refuses the set unless it is exactly one run's plan: every
//! planned unit present once, no repeated unit, and every checkpoint agreeing on
//! the run identity (analyzer, contract set, configuration, environment,
//! revision), the run-wide publication inputs and the plan. Only then does it
//! write anything. It merges the small per-unit results (tallies, sources,
//! held calls), writes the report's head, and streams the rows unit by unit:
//! it holds one unit's step sections at a time, never the whole corpus. The
//! report goes to a temporary sibling and is renamed into place only when
//! complete, so a refusal part-way never leaves a partial report.
//!
//! What it writes is what the whole run writes, byte for byte: the report,
//! its `.contracts.json` assumptions sidecar and the explicit Unknown
//! sidecars. Why that holds is the independence audit in
//! `docs/reports/w45-audit-checkpoints/README.md`: every cross-group input of
//! the whole run's publication is a sum, a union or a run-wide constant.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::checkpoint::{
    CheckpointReader, ContractsSection, GlobalSection, IdentitySection, PlanSection,
    ProvenanceSection, ResultSection, RowStepSection, RunIdentity, render_row, report_frame,
    tallied_verdict,
};
use crate::pending_review::{HeldCalls, PendingMarker, PendingMarkers};
use crate::report::AuditReport;
use qcore_schemas::verdict::Verdict;

/// Where a composition writes; each output is optional, as in the route.
pub struct ComposeOutputs<'a> {
    pub report: Option<&'a Path>,
    pub monkey_source_report: Option<&'a Path>,
    pub binding_directory: Option<&'a Path>,
}

/// Which console lines a composition prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Console {
    /// Everything the whole run prints: the preamble, each library's header and
    /// unit lines, and the totals.
    Full,
    /// Only the totals: a checkpointed run has printed the rest as it went.
    TotalsOnly,
}

/// What a composition found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposeSummary {
    pub groups: usize,
    /// The join of every group's verdict: the run's verdict.
    pub verdict: Verdict,
    pub units: u64,
    pub gateway_failures: u64,
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the composed rows' own
    /// refutations.
    pub refutations: qcore_schemas::contract_refutation::RefutationTable,
}

struct Member {
    path: PathBuf,
    reader: CheckpointReader,
    identity: IdentitySection,
    plan: PlanSection,
    global: GlobalSection,
}

/// The first run-identity field on which two checkpoints disagree.
fn disagreement(left: &RunIdentity, right: &RunIdentity) -> Option<&'static str> {
    if left.route != right.route {
        Some("route")
    } else if left.analyzer != right.analyzer {
        Some("analyzer build")
    } else if left.contracts != right.contracts {
        Some("contract set")
    } else if left.mode != right.mode
        || left.checkout != right.checkout
        || left.source_profile_sha256 != right.source_profile_sha256
    {
        Some("configuration")
    } else if left.source_revision != right.source_revision {
        Some("source revision")
    } else if left.compiler != right.compiler {
        Some("compiler")
    } else if left.environment != right.environment {
        Some("environment")
    } else if left.member_facts_sha256 != right.member_facts_sha256 {
        Some("member facts")
    } else {
        None
    }
}

/// Compose `files` into one run's outputs. See the module documentation.
pub fn compose(
    files: &[PathBuf],
    outputs: &ComposeOutputs<'_>,
    console: Console,
) -> Result<ComposeSummary, String> {
    let mut members = open_members(files)?;
    let global = members[0].global.clone();
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the run's member
    // closure is folded over the whole run. Card MEMBER-PREPASS-OTHER-ROUTES:
    // a checkpoint carries the run's facts when the pre-pass read them (their
    // closures, the fold's environment and each unit's own facts), and the
    // composer folds them as the run does; a run whose contracts invoke a
    // member and whose checkpoints carry no run facts is refused, as before
    // (fail-closed).
    let member_fold = member_fold_section(&global, &mut members)?;
    let groups = members[0].plan.groups.clone();
    // Merge the small per-unit results; print group lines in plan order.
    if console == Console::Full {
        for line in &global.console_preamble {
            println!("{line}");
        }
    }
    let mut totals = Totals::default();
    for member in &mut members {
        let result = member.reader.section::<ResultSection>("result")?;
        let contracts = member.reader.section::<ContractsSection>("contracts")?;
        let provenance = member.reader.section::<ProvenanceSection>("provenance")?;
        let expected = member.identity.group.units.len();
        if result.header != member.plan.groups[member.plan.group as usize].header {
            return Err(format!(
                "{}: result header differs from the run plan",
                member.path.display()
            ));
        }
        if result.outputs.len() != expected {
            return Err(format!(
                "{}: the result records {} unit(s), the identity {expected}",
                member.path.display(),
                result.outputs.len()
            ));
        }
        if console == Console::Full {
            if !result.header.is_empty() {
                println!("{}", result.header);
            }
            for output in &result.outputs {
                print!("{output}");
            }
        }
        totals
            .absorb(result, contracts, &provenance)
            .map_err(|error| format!("{}: {error}", member.path.display()))?;
    }
    let evidence = global
        .report_evidence
        .as_ref()
        .map(crate::checkpoint::BinaryValue::to_json)
        .transpose()?;
    let markers = PendingMarkers {
        declarations: global.pending_declarations as usize,
        markers: global
            .pending_markers
            .iter()
            .map(|marker| PendingMarker {
                function: marker.function.clone(),
                scope: marker.scope.clone(),
                clause: marker.clause.clone(),
                reason: marker.reason.clone(),
                owner: marker.owner.clone(),
                category: marker.category.clone(),
                module: marker.module.clone(),
                module_sha256: marker.module_sha256.clone(),
            })
            .collect(),
    };
    let pending = crate::pending_review::render(&markers, &totals.held);
    let template: crate::contract_revision::SectionTemplate =
        serde_json::from_value(global.revision_template.to_json()?)
            .map_err(|error| error.to_string())?;
    let revision = crate::contract_revision::render(&template, &totals.revision);
    let counts = &totals.counts;
    // Card MEMBER-PREPASS-OTHER-ROUTES: printed where the whole run prints it.
    if let Some(section) = &member_fold {
        println!(
            "callback members: {} demoted",
            section["demoted"].as_array().map_or(0, Vec::len)
        );
    }
    println!(
        "current revision: {} source file(s), {} configuration(s): {} Safe / {} Pending / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures",
        totals.sources.len(),
        counts["units"],
        counts["safe_units"],
        counts["pending_units"],
        counts["unsafe_units"],
        counts["unknown_units"],
        counts["data_only_units"],
        counts["gateway_failures"]
    );
    println!(
        "preparation: {} unit(s), {} Clang invocation(s); -E -P {} us / syntax {} us / line markers {} us",
        counts["prepared_units"],
        counts["preparation_clang_invocations"],
        totals.plain_us,
        totals.syntax_us,
        totals.marked_us,
    );
    println!("{}", crate::pending_review::tally(&pending));
    // Card W45-USER-CONTRACT-PLUGINS: printed only when a checker ran, as the
    // whole run prints it.
    let assumptions: Value = global.assumptions.to_json()?;
    if assumptions["plugin_checks"].is_object() {
        println!(
            "{}",
            crate::user_plugins::tally(&assumptions["plugin_checks"])
        );
    }
    println!("{}", crate::contract_revision::tally(&revision));
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13h): the same fold the whole
    // run makes, one unit at a time, under the premises the run recorded;
    // tallied where the whole run tallies it, after the revision section.
    let dependencies = crate::contract_dependencies::render(
        &dependency_facts(&mut members)?,
        &revision,
        &assumptions[crate::contract_dependencies::PREMISES_KEY],
    );
    println!("{}", crate::contract_dependencies::tally(&dependencies));
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the section the whole run's
    // `finish` writes, built from the same rows one unit at a time under the
    // table the run recorded before its units.
    let applied = crate::refuted_contracts::applied_table(&assumptions)?;
    let refuted_rows = refuted_rows(&mut members)?;
    let refuted = refuted_rows.section(&applied);
    println!("{}", crate::refuted_contracts::tally(&refuted));
    if let Some(report) = outputs.report {
        if let Some(parent) = report.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let document = AuditReport::skeleton(
            &global.source_revision,
            totals.diagnostic_counts.clone(),
            evidence,
        )
        .document(&global.revision_key);
        let (head, tail) = report_frame(&document)?;
        let temporary = PathBuf::from(format!("{}.composing", report.display()));
        let written = write_report(&temporary, &head, &tail, &mut members, &groups);
        if let Err(error) = written {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        fs::rename(&temporary, report).map_err(|error| format!("{}: {error}", report.display()))?;
        let mut assumptions = assumptions;
        assumptions["pending_human_review"] = pending;
        assumptions["contract_revision"] = revision;
        assumptions[crate::refuted_contracts::APPLIED_KEY] = refuted;
        assumptions[crate::contract_dependencies::SECTION_KEY] = dependencies;
        if let Some(section) = member_fold {
            assumptions[crate::callback_member_fold::SECTION_KEY] = section;
        }
        assumptions["source_file_count"] = json!(totals.sources.len());
        assumptions["configuration_count"] = json!(counts["units"]);
        fs::write(
            report.with_extension("contracts.json"),
            serde_json::to_vec_pretty(&assumptions).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    crate::current_route::write_unknown_sidecars(
        &crate::current_route::unknown_sidecar(
            &global.source_revision,
            &global.historical_reference_revision,
        ),
        outputs.report,
        outputs.monkey_source_report,
        outputs.binding_directory,
    )?;
    Ok(ComposeSummary {
        groups: groups.len(),
        verdict: totals.verdict,
        units: counts["units"],
        gateway_failures: counts["gateway_failures"],
        refutations: refuted_rows.published().clone(),
    })
}

/// Card MEMBER-PREPASS-OTHER-ROUTES (rule 12n): the run's member fold over
/// what the checkpoints carry, `None` when no declaration invokes a member or
/// states a result state. Every unit's rows (only what the fold reads) and
/// its recorded facts are folded with the carried environment and run
/// closures, exactly as the run's `finish` folds them. The composer writes
/// the rows as the checkpoints hold them, so a fold that would demote any row
/// refuses the composition; the run itself demoted them before publishing
/// (with the run's facts installed every unit decided over the carried set,
/// so the member part demotes nothing,
/// `QCore.MemberPrepassRoutes.the_composed_decision_is_the_runs`).
fn member_fold_section(
    global: &GlobalSection,
    members: &mut [Member],
) -> Result<Option<Value>, String> {
    let invokes = global.assumptions.to_json()?[crate::callback_member_fold::DECLARATIONS_KEY]
        .as_array()
        .is_some_and(|names| !names.is_empty());
    let carried = global
        .member_fold
        .as_ref()
        .map(crate::checkpoint::BinaryValue::to_json)
        .transpose()?;
    let run = carried.as_ref().map(|carried| &carried["run"]);
    if invokes && run.is_none_or(Value::is_null) {
        return Err(
            "a checkpointed run whose contracts invoke a callback member cannot be composed \
             without the run's member facts: the member closure (rule 12n) is folded over the \
             whole run and these checkpoints carry no pre-pass; run it without checkpoints"
                .into(),
        );
    }
    let Some(carried) = carried else {
        return Ok(None);
    };
    let environment: crate::callback_member_fold::FoldEnvironment =
        serde_json::from_value(carried["environment"].clone())
            .map_err(|error| format!("the carried fold environment: {error}"))?;
    let closures = match &carried["run"] {
        Value::Null => None,
        run => Some(crate::callback_member_fold::closures_from_value(
            &run["closures"],
        )?),
    };
    let mut rows = Vec::new();
    let mut facts = Vec::new();
    for member in members.iter_mut() {
        let steps = ["preparation", "lowering", "verdicts"]
            .map(|name| member.reader.section::<RowStepSection>(name));
        let [preparation, lowering, verdicts] = steps;
        let (preparation, lowering, verdicts) = (preparation?, lowering?, verdicts?);
        for ((preparation, lowering), verdicts) in preparation
            .units
            .iter()
            .zip(&lowering.units)
            .zip(&verdicts.units)
        {
            let mut unit = serde_json::Map::new();
            for member in preparation
                .members
                .iter()
                .chain(&lowering.members)
                .chain(&verdicts.members)
            {
                if matches!(
                    member.key.as_str(),
                    "library" | "file" | "status" | "functions"
                ) {
                    unit.insert(member.key.clone(), member.value.to_json()?);
                }
            }
            rows.push(Value::Object(unit));
        }
        let contracts = member.reader.section::<ContractsSection>("contracts")?;
        for unit in &contracts.callback_members {
            facts.push(
                serde_json::from_value::<crate::callback_member_fold::UnitFacts>(unit.to_json()?)
                    .map_err(|error| {
                    format!("{}: carried unit facts: {error}", member.path.display())
                })?,
            );
        }
    }
    let mut scratch = BTreeMap::new();
    let mut section = crate::callback_member_fold::fold_in(
        &environment,
        &mut rows,
        &facts,
        &mut scratch,
        closures.as_ref(),
    );
    if section.is_null() {
        return Ok(None);
    }
    let demoted = crate::callback_member_fold::demoted_functions(&section);
    if let Some((library, file, function)) = demoted.iter().next() {
        return Err(format!(
            "the run's member fold (rule 12n) demotes {} row(s) these checkpoints hold as Safe \
             or Pending (first: {library}/{file}: {function}); a composer does not rewrite rows: \
             run it without checkpoints",
            demoted.len()
        ));
    }
    if let Value::Object(_) = &carried["run"]["section"] {
        section["prepass"] = carried["run"]["section"].clone();
    }
    Ok(Some(section))
}

/// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the rows' held calls and own
/// refutations, one unit at a time: each unit's `library` and `file` from its
/// preparation step and its `functions` from its verdicts step.
fn refuted_rows(members: &mut [Member]) -> Result<crate::refuted_contracts::RowTally, String> {
    let mut tally = crate::refuted_contracts::RowTally::default();
    for member in members.iter_mut() {
        let preparation = member.reader.section::<RowStepSection>("preparation")?;
        let verdicts = member.reader.section::<RowStepSection>("verdicts")?;
        for (preparation, verdicts) in preparation.units.iter().zip(&verdicts.units) {
            let mut unit = serde_json::Map::new();
            for member in preparation.members.iter().chain(&verdicts.members) {
                if matches!(member.key.as_str(), "library" | "file" | "functions") {
                    unit.insert(member.key.clone(), member.value.to_json()?);
                }
            }
            tally.absorb(&Value::Object(unit));
        }
    }
    Ok(tally)
}

/// Card VERDICT-CONTRACT-DEPENDENCIES: the dependency facts of every unit
/// row, read like [`refuted_rows`].
fn dependency_facts(members: &mut [Member]) -> Result<crate::contract_dependencies::Facts, String> {
    let mut facts = crate::contract_dependencies::Facts::default();
    for member in members.iter_mut() {
        let preparation = member.reader.section::<RowStepSection>("preparation")?;
        let verdicts = member.reader.section::<RowStepSection>("verdicts")?;
        for (preparation, verdicts) in preparation.units.iter().zip(&verdicts.units) {
            let mut unit = serde_json::Map::new();
            for member in preparation.members.iter().chain(&verdicts.members) {
                if matches!(
                    member.key.as_str(),
                    "functions" | crate::contract_dependencies::UNIT_KEY
                ) {
                    unit.insert(member.key.clone(), member.value.to_json()?);
                }
            }
            facts.absorb_unit(&Value::Object(unit));
        }
    }
    Ok(facts)
}

/// Open and verify every file; refuse any set that is not exactly one run's
/// plan. Returns the members in plan order.
fn open_members(files: &[PathBuf]) -> Result<Vec<Member>, String> {
    if files.is_empty() {
        return Err("no checkpoint to compose".into());
    }
    let mut members = Vec::new();
    for path in files {
        let mut reader = CheckpointReader::open(path)?;
        reader.verify()?;
        let identity = reader.section::<IdentitySection>("identity")?;
        let plan = reader.section::<PlanSection>("plan")?;
        let global = reader.section::<GlobalSection>("global")?;
        members.push(Member {
            path: path.clone(),
            reader,
            identity,
            plan,
            global,
        });
    }
    let reference = &members[0];
    for member in &members[1..] {
        let names = || format!("{} and {}", reference.path.display(), member.path.display());
        if let Some(field) = disagreement(&reference.identity.run, &member.identity.run) {
            return Err(format!("{} disagree on the {field}", names()));
        }
        if member.global != reference.global {
            return Err(format!(
                "{} disagree on the run-wide publication inputs (assumptions, markers or revision)",
                names()
            ));
        }
        if member.plan.groups != reference.plan.groups {
            return Err(format!("{} disagree on the run's plan of groups", names()));
        }
    }
    let planned = &reference.plan.groups;
    let mut owners = BTreeMap::new();
    for group in planned {
        for unit in &group.units {
            if let Some(other) = owners.insert(unit, &group.label) {
                return Err(format!(
                    "unit {} is in groups `{other}` and `{}`; groups must be disjoint",
                    unit.expected_source, group.label
                ));
            }
        }
    }
    let mut slots = vec![None; planned.len()];
    for (index, member) in members.iter().enumerate() {
        let position = usize::try_from(member.plan.group).unwrap_or(usize::MAX);
        let Some(group) = planned.get(position) else {
            return Err(format!(
                "{}: plan position {} is outside the plan",
                member.path.display(),
                member.plan.group
            ));
        };
        if member.identity.group.label != group.label {
            return Err(format!(
                "{}: group `{}` sits at the plan position of `{}`",
                member.path.display(),
                member.identity.group.label,
                group.label
            ));
        }
        if member.identity.group.units.len() != group.units.len() {
            return Err(format!(
                "{}: {} unit(s), the plan's group `{}` has {}",
                member.path.display(),
                member.identity.group.units.len(),
                group.label,
                group.units.len()
            ));
        }
        if let Some(other) = slots[position].replace(index) {
            let other: &Member = &members[other];
            return Err(format!(
                "group `{}` is given twice: {} and {}",
                group.label,
                other.path.display(),
                member.path.display()
            ));
        }
    }
    let missing = slots
        .iter()
        .zip(planned)
        .filter(|(slot, _)| slot.is_none())
        .map(|(_, group)| group.label.clone())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "the run's plan has {} group(s); missing: {missing:?}",
            planned.len()
        ));
    }
    let mut by_position = members.into_iter().map(Some).collect::<Vec<_>>();
    Ok(slots
        .into_iter()
        .map(|slot| {
            by_position[slot.expect("every position filled")]
                .take()
                .expect("taken once")
        })
        .collect())
}

/// The composition's verdict fold (card W45-USER-CONTRACT-PLUGINS, §L21.2):
/// the running verdict is joined with the group's verdict, with
/// `qcore_schemas::verdict::Verdict::join` (`Safe` the identity), and the
/// group's tallies are added to the running totals. Rows are never merged —
/// every unit keeps its own — so the tallies are sums, and because the join
/// is associative the composed verdict is the join of every unit's verdict,
/// exactly the whole run's. A group whose recorded verdict disagrees with the
/// verdict its own tallies imply ([`tallied_verdict`]) is refused.
fn fold_verdicts(
    verdict: &mut Verdict,
    totals: &mut BTreeMap<&'static str, u64>,
    group: &ResultSection,
) -> Result<(), String> {
    let recorded = group.verdict;
    let tallied = tallied_verdict(&group.counts);
    if recorded != tallied {
        return Err(format!(
            "group `{}` records verdict {recorded} but its tallies imply {tallied}",
            group.header
        ));
    }
    *verdict = verdict.join(recorded);
    let counts = &group.counts;
    for (name, value) in [
        ("units", counts.units),
        ("data_only_units", counts.data_only_units),
        ("source_functions", counts.source_functions),
        ("projected_functions", counts.projected_functions),
        ("safe_units", counts.safe_units),
        ("pending_units", counts.pending_units),
        ("unsafe_units", counts.unsafe_units),
        ("unknown_units", counts.unknown_units),
        ("gateway_failures", counts.gateway_failures),
        (
            "absent_from_raw_scan_definitions",
            counts.absent_from_raw_scan_definitions,
        ),
        ("unanalyzable_definitions", counts.unanalyzable_definitions),
        (
            "enumeration_refused_units",
            counts.enumeration_refused_units,
        ),
        ("withheld_safe_units", counts.withheld_safe_units),
        ("prepared_units", counts.prepared_units),
        (
            "preparation_clang_invocations",
            counts.preparation_clang_invocations,
        ),
    ] {
        *totals.entry(name).or_default() += value;
    }
    Ok(())
}

/// The merged small results of every group.
struct Totals {
    verdict: Verdict,
    counts: BTreeMap<&'static str, u64>,
    diagnostic_counts: BTreeMap<String, usize>,
    sources: BTreeSet<String>,
    held: HeldCalls,
    revision: crate::contract_revision::ReportFacts,
    plain_us: u64,
    syntax_us: u64,
    marked_us: u64,
}

impl Default for Totals {
    fn default() -> Self {
        Self {
            verdict: Verdict::Safe,
            counts: BTreeMap::new(),
            diagnostic_counts: BTreeMap::new(),
            sources: BTreeSet::new(),
            held: HeldCalls::default(),
            revision: crate::contract_revision::ReportFacts::default(),
            plain_us: 0,
            syntax_us: 0,
            marked_us: 0,
        }
    }
}

impl Totals {
    fn absorb(
        &mut self,
        result: ResultSection,
        contracts: ContractsSection,
        provenance: &ProvenanceSection,
    ) -> Result<(), String> {
        fold_verdicts(&mut self.verdict, &mut self.counts, &result)?;
        for category in result.diagnostic_counts {
            *self.diagnostic_counts.entry(category.category).or_default() +=
                category.count as usize;
        }
        self.sources.extend(result.sources);
        let rows = |functions: Vec<crate::checkpoint::HeldFunction>| {
            functions
                .into_iter()
                .map(|function| (function.library, function.file, function.function))
                .collect::<BTreeSet<_>>()
        };
        let mut held = HeldCalls::default();
        for call in contracts.held_calls {
            held.call_sites
                .insert(call.callee.clone(), call.call_sites as usize);
            held.functions.insert(call.callee, rows(call.functions));
        }
        self.held.absorb(held);
        let mut revision = crate::contract_revision::ReportFacts {
            analysed: contracts.analysed_functions.into_iter().collect(),
            ..Default::default()
        };
        for call in contracts.pin_held_calls {
            revision.held.insert(
                call.callee,
                (call.call_sites as usize, rows(call.functions)),
            );
        }
        self.revision.absorb(revision);
        self.plain_us += provenance.plain_preprocessing_elapsed_us;
        self.syntax_us += provenance.syntax_validation_elapsed_us;
        self.marked_us += provenance.line_marked_preprocessing_elapsed_us;
        Ok(())
    }
}

/// Stream every group's rows into `path`, one unit's step sections at a time.
fn write_report(
    path: &Path,
    head: &str,
    tail: &str,
    members: &mut [Member],
    groups: &[crate::checkpoint::PlannedGroup],
) -> Result<(), String> {
    let file = File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let io = |error: std::io::Error| format!("{}: {error}", path.display());
    writer.write_all(head.as_bytes()).map_err(io)?;
    let total = groups.iter().map(|group| group.units.len()).sum::<usize>();
    if total == 0 {
        writer.write_all(b"[]").map_err(io)?;
    } else {
        writer.write_all(b"[\n").map_err(io)?;
        let mut first = true;
        let mut row = String::new();
        for member in members.iter_mut() {
            let preparation = member.reader.section::<RowStepSection>("preparation")?;
            let lowering = member.reader.section::<RowStepSection>("lowering")?;
            let verdicts = member.reader.section::<RowStepSection>("verdicts")?;
            let expected = member.identity.group.units.len();
            if preparation.units.len() != expected
                || lowering.units.len() != expected
                || verdicts.units.len() != expected
            {
                return Err(format!(
                    "{}: step sections disagree on the unit count",
                    member.path.display()
                ));
            }
            for ((preparation, lowering), verdicts) in preparation
                .units
                .iter()
                .zip(&lowering.units)
                .zip(&verdicts.units)
            {
                if !first {
                    writer.write_all(b",\n").map_err(io)?;
                }
                first = false;
                row.clear();
                row.push_str("    ");
                render_row([preparation, lowering, verdicts], 2, &mut row)?;
                writer.write_all(row.as_bytes()).map_err(io)?;
            }
        }
        writer.write_all(b"\n  ]").map_err(io)?;
    }
    writer.write_all(tail.as_bytes()).map_err(io)?;
    writer.flush().map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::{CountsRecord, unit_verdict};

    fn group(verdicts: &[&str]) -> ResultSection {
        let mut counts = CountsRecord::default();
        for status in verdicts {
            counts.units += 1;
            match *status {
                "Safe" => counts.safe_units += 1,
                "Pending" => counts.pending_units += 1,
                "Unknown" => counts.unknown_units += 1,
                "Unsafe" => counts.unsafe_units += 1,
                "data_only" => counts.data_only_units += 1,
                other => panic!("{other}"),
            }
        }
        ResultSection {
            header: "g".into(),
            outputs: Vec::new(),
            counts,
            diagnostic_counts: Vec::new(),
            sources: Vec::new(),
            verdict: Verdict::fold(verdicts.iter().map(|status| unit_verdict(status))),
        }
    }

    /// Card W45-USER-CONTRACT-PLUGINS's follow-up W45-CHECKPOINT-COMPOSE-SEMIGROUP:
    /// a composed run's verdict is the fold of every member unit's verdict,
    /// however the units are grouped, and a `Pending` member keeps it from `Safe`.
    #[test]
    fn a_composed_verdict_is_the_fold_of_its_members() {
        let units = ["Safe", "data_only", "Pending", "Safe", "Unknown", "Safe"];
        let whole = Verdict::fold(units.iter().map(|status| unit_verdict(status)));
        for split in 0..=units.len() {
            let mut verdict = Verdict::Safe;
            let mut totals = BTreeMap::new();
            for part in [&units[..split], &units[split..]] {
                fold_verdicts(&mut verdict, &mut totals, &group(part)).unwrap();
            }
            assert_eq!(verdict, whole, "split at {split}");
            assert_eq!(totals["units"], units.len() as u64);
            assert_eq!(totals["pending_units"], 1);
        }
        let mut verdict = Verdict::Safe;
        let mut totals = BTreeMap::new();
        for part in [&["Safe", "data_only"][..], &["Pending", "Safe"][..]] {
            fold_verdicts(&mut verdict, &mut totals, &group(part)).unwrap();
        }
        assert_eq!(verdict, Verdict::Pending);
        let mut none = Verdict::fold([]);
        assert_eq!(none, Verdict::Safe);
        fold_verdicts(&mut none, &mut BTreeMap::new(), &group(&["Unsafe"])).unwrap();
        assert_eq!(none, Verdict::Unsafe);
    }

    #[test]
    fn a_group_whose_verdict_disagrees_with_its_tallies_is_refused() {
        let mut forged = group(&["Pending", "Safe"]);
        forged.verdict = Verdict::Safe;
        let error = fold_verdicts(&mut Verdict::Safe, &mut BTreeMap::new(), &forged).unwrap_err();
        assert!(error.contains("records verdict Safe but its tallies imply Pending"));
        assert!(postcard::from_bytes::<Verdict>(&[0x7f]).is_err());
        assert_eq!(unit_verdict("gateway_failure"), Verdict::Unknown);
        assert_eq!(unit_verdict("something new"), Verdict::Unknown);
    }
}
