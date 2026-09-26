//! Card **W45-AUDIT-CHECKPOINTS**: streaming composition of per-group `.qo`
//! checkpoints into one report.
//!
//! [`compose`] first opens and fully verifies every checkpoint, one section at
//! a time, and refuses the set unless it is exactly one run's plan: every
//! group present once, no unit in two groups, and every checkpoint agreeing on
//! the run identity (analyzer, contract set, configuration, environment,
//! revision), the run-wide publication inputs and the plan. Only then does it
//! write anything. It merges the small per-group results (tallies, sources,
//! held calls), writes the report's head, and streams the rows group by group:
//! it holds one group's step sections at a time, never the whole corpus. The
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
};
use crate::pending_review::{HeldCalls, PendingMarker, PendingMarkers};
use crate::report::AuditReport;

/// Where a composition writes; each output is optional, as in the route.
pub struct ComposeOutputs<'a> {
    pub report: Option<&'a Path>,
    pub monkey_source_report: Option<&'a Path>,
    pub binding_directory: Option<&'a Path>,
}

/// Which console lines a composition prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Console {
    /// Everything the whole run prints: the preamble, each group's header and
    /// unit lines, and the totals.
    Full,
    /// Only the totals: a checkpointed run has printed the rest as it went.
    TotalsOnly,
}

/// What a composition found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposeSummary {
    pub groups: usize,
    pub units: u64,
    pub gateway_failures: u64,
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
    let groups = members[0].plan.groups.clone();
    // Merge the small per-group results; print group lines in plan order.
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
        if result.outputs.len() != expected {
            return Err(format!(
                "{}: the result records {} unit(s), the identity {expected}",
                member.path.display(),
                result.outputs.len()
            ));
        }
        if console == Console::Full {
            println!("{}", result.header);
            for output in &result.outputs {
                print!("{output}");
            }
        }
        totals.absorb(result, contracts, &provenance);
    }
    let evidence = global
        .report_evidence_json
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()
        .map_err(|error| error.to_string())?;
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
                module: marker.module.clone(),
                module_sha256: marker.module_sha256.clone(),
            })
            .collect(),
    };
    let pending = crate::pending_review::render(&markers, &totals.held);
    let counts = &totals.counts;
    println!(
        "current revision: {} source file(s), {} configuration(s): {} Safe / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures",
        totals.sources.len(),
        counts["units"],
        counts["safe_units"],
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
        let mut assumptions: Value =
            serde_json::from_str(&global.assumptions_json).map_err(|error| error.to_string())?;
        assumptions["pending_human_review"] = pending;
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
        units: counts["units"],
        gateway_failures: counts["gateway_failures"],
    })
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
        let names = || {
            format!(
                "{} and {}",
                reference.path.display(),
                member.path.display()
            )
        };
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

/// The merged small results of every group.
#[derive(Default)]
struct Totals {
    counts: BTreeMap<&'static str, u64>,
    diagnostic_counts: BTreeMap<String, usize>,
    sources: BTreeSet<String>,
    held: HeldCalls,
    plain_us: u64,
    syntax_us: u64,
    marked_us: u64,
}

impl Totals {
    fn absorb(
        &mut self,
        result: ResultSection,
        contracts: ContractsSection,
        provenance: &ProvenanceSection,
    ) {
        let counts = &result.counts;
        for (name, value) in [
            ("units", counts.units),
            ("data_only_units", counts.data_only_units),
            ("source_functions", counts.source_functions),
            ("projected_functions", counts.projected_functions),
            ("safe_units", counts.safe_units),
            ("unsafe_units", counts.unsafe_units),
            ("unknown_units", counts.unknown_units),
            ("gateway_failures", counts.gateway_failures),
            (
                "absent_from_raw_scan_definitions",
                counts.absent_from_raw_scan_definitions,
            ),
            ("unanalyzable_definitions", counts.unanalyzable_definitions),
            ("enumeration_refused_units", counts.enumeration_refused_units),
            ("withheld_safe_units", counts.withheld_safe_units),
            ("prepared_units", counts.prepared_units),
            (
                "preparation_clang_invocations",
                counts.preparation_clang_invocations,
            ),
        ] {
            *self.counts.entry(name).or_default() += value;
        }
        for category in result.diagnostic_counts {
            *self
                .diagnostic_counts
                .entry(category.category)
                .or_default() += category.count as usize;
        }
        self.sources.extend(result.sources);
        let mut held = HeldCalls::default();
        for call in contracts.held_calls {
            held.call_sites.insert(call.callee.clone(), call.call_sites as usize);
            held.functions.insert(
                call.callee,
                call.functions
                    .into_iter()
                    .map(|function| (function.library, function.file, function.function))
                    .collect(),
            );
        }
        self.held.absorb(held);
        self.plain_us += provenance.plain_preprocessing_elapsed_us;
        self.syntax_us += provenance.syntax_validation_elapsed_us;
        self.marked_us += provenance.line_marked_preprocessing_elapsed_us;
    }
}

/// Stream every group's rows into `path`, one group's step sections at a time.
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
