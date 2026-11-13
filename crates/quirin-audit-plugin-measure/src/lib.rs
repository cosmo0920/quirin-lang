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

//! `quirin-audit measure` (card **MEASURE-TOOL**, pluggable-cards plan seam F;
//! user decision 8 of 2026-10-25): measurement as a `quirin-audit` subcommand
//! plugin, behind the same out-of-process firewall and receipts as every
//! other subcommand (contract specification C11.2). It replaces the
//! `measure.py`, `compare.py`, `mirror_inputs.py` and `gate_job.sh` copies in
//! the report directories.
//!
//! ```text
//! quirin-audit --plugin-path DIR --output OUT --timeout-ms MS measure \
//!     (--profile NAME | --inputs FILE [--routes R,...]) \
//!     --side NAME=SPEC --side NAME=SPEC [--side ...] [--compare A B]... \
//!     [--declared PREDICTION.md] [--rules-off IDS [--per-rule]] [--repository DIR] \
//!     [--inputs-base DIR] [--jobs N] [--build-jobs N] [--mirror auto|always|never] \
//!     [--mount-root DIR] [--no-slots] [--build-commit TEXT] \
//!     [--source-date-epoch SECONDS|none] [--baselines DIR] \
//!     [--legacy-rows] [--reuse-contract-only [--verify-reuse]]
//! quirin-audit --plugin-path DIR --output <baselines>/<sha> measure publish \
//!     --from OUT/sides/NAME [--routes R,...]
//! ```
//!
//! The output is one **native-comparison record**
//! (`OUT/native-comparison.json`, schema [`COMPARISON_SCHEMA`]): per side its
//! run metadata (binary digest, inputs and contract-bundle digests, its
//! `run.json`); per comparison and route the totals counted from each side's
//! own report, every moved, content-changed and unmatched row keyed by its
//! composition component, the unit rows' derived verdicts, the byte comparison
//! of the report artifacts with the one named clock substitution; rows moved in
//! more than one comparison (named interactions, never summed); the
//! declared-versus-measured check; and the per-rule `contribution` slot: the
//! contribution ledgers of every switched rule and named interaction
//! ([`ledger`], card CONTRIBUTION-LEDGERS). The tool proposes no verdict: it claims nothing through the
//! firewall, and a failed side, refused baseline, uncomparable route or
//! declared-versus-measured mismatch fails the run (`Unknown` at the host).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quirin_audit_plugin::{Request, Response, Severity};
use serde_json::{Value, json};

pub mod baseline;
pub mod compare;
pub mod declared;
pub mod inputs;
pub mod ledger;
pub mod mirror;
pub mod reuse;
pub mod rows;
pub mod sides;
pub mod util;
pub mod waits;

use inputs::Route;
use mirror::Mode;
use sides::{Built, Context, SideSpec};
use util::{read_json, sha_file, write_json};

/// The native-comparison record's schema.
pub const COMPARISON_SCHEMA: &str = "quirin-native-comparison/1";

/// A side's run record (`sides/<name>/run.json`).
pub const RUN_SCHEMA: &str = "quirin-measure-run/1";

/// Usage text.
pub const USAGE: &str = "usage: quirin-audit --output OUT measure (--profile NAME | --inputs FILE \
[--routes R,...]) --side NAME=SPEC --side NAME=SPEC [--compare A B]... [--declared FILE] \
[--rules-off IDS [--per-rule]] [--repository DIR] [--inputs-base DIR] [--jobs N] [--build-jobs N] \
[--mirror auto|always|never] [--mount-root DIR] [--no-slots] [--build-commit TEXT] \
[--source-date-epoch SECONDS|none] [--baselines DIR] [--legacy-rows] \
[--reuse-contract-only [--verify-reuse]]\n       quirin-audit --output <baselines>/<sha> measure \
publish --from OUT/sides/NAME [--routes R,...]\nSPEC: rev:<rev> | <directory> | <binary> | \
baseline:<sha> | run:<side directory>";

/// The default fixed mount root of snapshot sides and the mirror.
pub const DEFAULT_MOUNT_ROOT: &str = "/tmp/quirin-measure";

/// The tool's identity, as every record carries it.
#[must_use]
pub fn tool() -> Value {
    let digest = std::env::current_exe()
        .ok()
        .and_then(|path| sha_file(&path).ok());
    json!({"name": "quirin-audit-measure", "version": env!("CARGO_PKG_VERSION"),
           "sha256": digest})
}

/// The parsed flags of a measurement.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub profiles: Vec<String>,
    pub inputs: Option<String>,
    pub routes: Vec<String>,
    pub sides: Vec<(String, String)>,
    pub compare: Vec<(String, String)>,
    pub declared: Option<String>,
    pub rules_off: Option<Vec<String>>,
    pub per_rule: bool,
    pub repository: Option<String>,
    pub inputs_base: Option<String>,
    pub jobs: Option<u32>,
    pub build_jobs: Option<u32>,
    pub mirror: Option<String>,
    pub mount_root: Option<String>,
    pub no_slots: bool,
    pub build_commit: Option<String>,
    pub source_date_epoch: Option<String>,
    pub baselines: Option<String>,
    pub legacy_rows: bool,
    /// Card MEASURE-CONTRACT-ONLY-SIDE-WIRING ([`reuse`]).
    pub reuse_contract_only: bool,
    pub verify_reuse: bool,
    pub publish_from: Option<String>,
    pub publish: bool,
}

fn plain_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// Parse the rule-switch list: a comma list of plain rule ids.
///
/// # Errors
///
/// An empty list or a malformed id.
pub fn parse_rules_off(text: &str) -> Result<Vec<String>, String> {
    let ids: Vec<String> = text.split(',').map(str::trim).map(str::to_owned).collect();
    if ids.iter().any(|id| !plain_id(id)) {
        return Err(format!(
            "--rules-off `{text}`: a comma list of rule ids (`33,35`)"
        ));
    }
    let distinct: std::collections::BTreeSet<&String> = ids.iter().collect();
    if distinct.len() != ids.len() {
        return Err(format!("--rules-off `{text}` names a rule twice"));
    }
    Ok(ids)
}

/// Parse the arguments after `measure`.
///
/// # Errors
///
/// An unknown, repeated or valueless flag, or a malformed value.
pub fn parse(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut rest = arguments;
    if rest.first().map(String::as_str) == Some("publish") {
        options.publish = true;
        rest = &rest[1..];
    } else if rest.first().map(String::as_str) == Some("run") {
        rest = &rest[1..];
    }
    let mut index = 0;
    let once = |slot: &mut Option<String>, flag: &str, value: &str| {
        if slot.replace(value.to_owned()).is_some() {
            Err(format!("{flag} is given twice"))
        } else {
            Ok(())
        }
    };
    while index < rest.len() {
        let flag = rest[index].as_str();
        match flag {
            "--no-slots" => options.no_slots = true,
            "--legacy-rows" => options.legacy_rows = true,
            "--per-rule" => options.per_rule = true,
            "--reuse-contract-only" | "--verify-reuse" => {
                let slot = if flag == "--verify-reuse" {
                    &mut options.verify_reuse
                } else {
                    &mut options.reuse_contract_only
                };
                if std::mem::replace(slot, true) {
                    return Err(format!("{flag} is given twice"));
                }
            }
            _ => {
                let value = rest
                    .get(index + 1)
                    .ok_or_else(|| format!("{flag} requires a value\n{USAGE}"))?
                    .as_str();
                match flag {
                    "--profile" => options.profiles.push(value.into()),
                    "--inputs" => once(&mut options.inputs, flag, value)?,
                    "--routes" => options
                        .routes
                        .extend(value.split(',').map(str::trim).map(str::to_owned)),
                    "--side" => {
                        let (name, spec) = value
                            .split_once('=')
                            .ok_or_else(|| format!("--side `{value}`: NAME=SPEC"))?;
                        if !plain_id(name) || options.sides.iter().any(|(known, _)| known == name) {
                            return Err(format!("--side name `{name}` is not plain or repeated"));
                        }
                        options.sides.push((name.into(), spec.into()));
                    }
                    "--compare" => {
                        let after = rest
                            .get(index + 2)
                            .ok_or_else(|| "--compare takes two side names".to_owned())?;
                        options.compare.push((value.into(), after.clone()));
                        index += 1;
                    }
                    "--declared" => once(&mut options.declared, flag, value)?,
                    "--rules-off" => {
                        if options.rules_off.replace(parse_rules_off(value)?).is_some() {
                            return Err("--rules-off is given twice".into());
                        }
                    }
                    "--repository" => once(&mut options.repository, flag, value)?,
                    "--inputs-base" => once(&mut options.inputs_base, flag, value)?,
                    "--jobs" | "--build-jobs" => {
                        let number: u32 = value
                            .parse()
                            .ok()
                            .filter(|number| *number > 0)
                            .ok_or_else(|| format!("{flag} needs a positive integer"))?;
                        let slot = if flag == "--jobs" {
                            &mut options.jobs
                        } else {
                            &mut options.build_jobs
                        };
                        if slot.replace(number).is_some() {
                            return Err(format!("{flag} is given twice"));
                        }
                    }
                    "--mirror" => {
                        value.parse::<Mode>()?;
                        once(&mut options.mirror, flag, value)?;
                    }
                    "--mount-root" => once(&mut options.mount_root, flag, value)?,
                    "--build-commit" => once(&mut options.build_commit, flag, value)?,
                    "--source-date-epoch" => {
                        if value != "none" && value.parse::<u64>().is_err() {
                            return Err("--source-date-epoch takes seconds or `none`".into());
                        }
                        once(&mut options.source_date_epoch, flag, value)?;
                    }
                    "--baselines" => once(&mut options.baselines, flag, value)?,
                    "--from" => once(&mut options.publish_from, flag, value)?,
                    other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
                }
                index += 1;
            }
        }
        index += 1;
    }
    if options.publish {
        if options.publish_from.is_none()
            || !options.sides.is_empty()
            || options.reuse_contract_only
            || options.verify_reuse
        {
            return Err(format!(
                "publish takes --from SIDE_DIRECTORY and no --side\n{USAGE}"
            ));
        }
        return Ok(options);
    }
    if options.publish_from.is_some() {
        return Err("--from belongs to `measure publish`".into());
    }
    if options.verify_reuse && !options.reuse_contract_only {
        return Err("--verify-reuse checks --reuse-contract-only: give both".into());
    }
    if options.per_rule && options.rules_off.as_ref().is_none_or(|ids| ids.len() < 2) {
        return Err(
            "--per-rule separates two or more rules: give --rules-off with at least two ids".into(),
        );
    }
    if options.profiles.is_empty() == options.inputs.is_none() {
        return Err(format!(
            "give --profile NAME or --inputs FILE (one of them)\n{USAGE}"
        ));
    }
    if options.sides.len() < 2 {
        return Err(format!(
            "a measurement compares two or more --side NAME=SPEC\n{USAGE}"
        ));
    }
    for (before, after) in &options.compare {
        for name in [before, after] {
            if !options.sides.iter().any(|(known, _)| known == name) {
                return Err(format!("--compare names `{name}`, which is no --side"));
            }
        }
        if before == after {
            return Err(format!(
                "--compare {before} {after} compares a side with itself"
            ));
        }
    }
    Ok(options)
}

fn absolute(request: &Request, text: &str) -> PathBuf {
    request.resolve_argument(text)
}

fn canonical(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Answer a request.
///
/// # Errors
///
/// A refusal or failure, with its reason (the response is then `failed`).
pub fn answer(request: &Request) -> Result<Response, String> {
    let options = parse(&request.arguments)?;
    if options.publish {
        return publish(request, &options);
    }
    measure(request, &options)
}

fn publish(request: &Request, options: &Options) -> Result<Response, String> {
    let output = canonical(&request.output_directory())?;
    let from = canonical(&absolute(
        request,
        options.publish_from.as_deref().unwrap_or(""),
    ))?;
    let written = baseline::publish(&from, &output, &options.routes)?;
    let mut response = Response::completed();
    for directory in written {
        let relative = directory.strip_prefix(&output).unwrap_or(&directory);
        response.push_artifact(
            &output,
            &relative.join(baseline::MANIFEST),
            "measure-baseline",
        )?;
        response.push_artifact(&output, &relative.join("run.json"), "measure-run")?;
        response.push_diagnostic(
            Severity::Info,
            "baseline-published",
            format!("published {}", directory.display()),
        );
    }
    Ok(response)
}

/// A side as the comparison reads it.
#[derive(Clone, Debug)]
struct Resolved {
    spec: String,
    kind: &'static str,
    directory: PathBuf,
    record: Value,
    /// Route name to (exit, stable) when recorded.
    routes: BTreeMap<String, Value>,
}

#[allow(clippy::too_many_lines)]
fn measure(request: &Request, options: &Options) -> Result<Response, String> {
    let started = std::time::Instant::now();
    let output = canonical(&request.output_directory())?;
    // Idle stretches (every route in flight waiting for its slot) are
    // declared to the host through the marker in the output (R7).
    let tracker = waits::Tracker::new(Some(&output));
    let repository = canonical(&absolute(
        request,
        options.repository.as_deref().unwrap_or("."),
    ))?;
    let inputs_base = canonical(&absolute(
        request,
        options
            .inputs_base
            .as_deref()
            .unwrap_or(&repository.display().to_string()),
    ))?;
    let mut routes: Vec<Route> = if let Some(manifest) = &options.inputs {
        inputs::read_manifest(&absolute(request, manifest))?
    } else {
        options
            .profiles
            .iter()
            .map(|name| {
                let path = inputs::profile_record(&repository, name);
                let path = if path.is_absolute() {
                    path
                } else {
                    absolute(request, &path.display().to_string())
                };
                if !path.is_file() {
                    return Err(format!(
                        "no evaluation-profile record {} (contract/evaluations/<name>/evaluation.json, \
                         card EVALUATION-PROFILES); pass --inputs FILE",
                        path.display()
                    ));
                }
                inputs::read_profile(&path)
            })
            .collect::<Result<_, _>>()?
    };
    if !options.routes.is_empty() {
        for name in &options.routes {
            if !routes.iter().any(|route| &route.name == name) {
                return Err(format!(
                    "--routes names `{name}`, which the inputs do not define"
                ));
            }
        }
        routes.retain(|route| options.routes.contains(&route.name));
    }
    inputs::validate(&routes)?;
    let mode: Mode = options.mirror.as_deref().unwrap_or("auto").parse()?;
    let mount_root = PathBuf::from(options.mount_root.as_deref().unwrap_or(DEFAULT_MOUNT_ROOT));
    if !mount_root.is_absolute() {
        return Err("--mount-root must be absolute".into());
    }
    let slot_script = if options.no_slots {
        None
    } else {
        let script = repository.join("scripts/gate_slot.py");
        if !script.is_file() {
            return Err(format!(
                "{} is missing: audits run under `gate_slot.py audit` slots (pass --no-slots to \
                 run without)",
                script.display()
            ));
        }
        Some(script)
    };
    let specs: Vec<(String, SideSpec)> = options
        .sides
        .iter()
        .map(|(name, text)| {
            Ok((
                name.clone(),
                SideSpec::parse(text, Path::new(&request.invocation_directory))?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let comparisons: Vec<(String, String)> = if options.compare.is_empty() {
        vec![(specs[0].0.clone(), specs[1].0.clone())]
    } else {
        options.compare.clone()
    };
    let declaration = match &options.declared {
        Some(path) => {
            let path = absolute(request, path);
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            Some((
                path.clone(),
                sha_file(&path)?,
                declared::parse(&text).map_err(|error| format!("{}: {error}", path.display()))?,
            ))
        }
        None => None,
    };
    let rules_off = options.rules_off.clone().unwrap_or_default();
    let source_date_epoch = match options.source_date_epoch.as_deref() {
        None => Some(sides::DEFAULT_SOURCE_DATE_EPOCH),
        Some("none") => None,
        Some(seconds) => seconds.parse().ok(),
    };
    let context = Context {
        repository: repository.clone(),
        inputs_base: inputs_base.clone(),
        output: output.clone(),
        mount_root: mount_root.clone(),
        jobs: options.jobs.unwrap_or(3),
        build_jobs: options.build_jobs.unwrap_or(6),
        slot_script,
        build_commit: options.build_commit.clone(),
        source_date_epoch,
        rules_off: Vec::new(),
        checkpoint: None,
    };
    // A rule-switched side is the after side of the first comparison,
    // re-run with the switch: it must run, and its binary must advertise
    // the switch (card RULE-SWITCHES), or the measurement is refused before
    // any route runs.
    let switched = if rules_off.is_empty() {
        None
    } else {
        let tip = &comparisons[0].1;
        let spec = &specs
            .iter()
            .find(|(name, _)| name == tip)
            .expect("checked")
            .1;
        if !spec.runs() {
            return Err(format!(
                "--rules-off measures the first comparison's after side `{tip}` with the rules \
                 off, so it must be a side that runs (not {})",
                spec.kind()
            ));
        }
        Some(tip.clone())
    };
    let switch = switched.as_ref().map(|tip| {
        let mut ordered = rules_off.clone();
        ordered.sort_by_key(|id| ledger::rule_order(id));
        ledger::Switch {
            tip: tip.clone(),
            joint: format!("{tip}-off"),
            per_rule: if options.per_rule {
                ordered
                    .iter()
                    .map(|id| (id.clone(), format!("{tip}-off-{id}")))
                    .collect()
            } else {
                Vec::new()
            },
            rules: ordered,
        }
    });
    let switched_sides: BTreeMap<String, Vec<String>> = switch
        .as_ref()
        .map(|switch| {
            let mut sides: BTreeMap<String, Vec<String>> = switch.sides().into_iter().collect();
            // The joint side keeps the off-list as given.
            sides.insert(switch.joint.clone(), rules_off.clone());
            sides
        })
        .unwrap_or_default();
    for name in switched_sides.keys() {
        if options.sides.iter().any(|(known, _)| known == name) {
            return Err(format!(
                "--side {name} collides with the rule-switched side of that name"
            ));
        }
    }
    // Card MEASURE-CONTRACT-ONLY-SIDE-WIRING: whether the first comparison
    // may be measured with the tip under the base side's analyzer; decided
    // finally once the base binary has classified the diff.
    let reuse_candidate = if options.reuse_contract_only {
        reuse::candidate(
            &specs,
            &comparisons,
            &routes,
            &rules_off,
            options.build_commit.as_deref(),
        )
    } else {
        Err("not requested (--reuse-contract-only)".to_owned())
    };
    let full_side = match (&reuse_candidate, options.verify_reuse) {
        (Ok((_, tip)), true) => {
            let name = format!("{tip}{}", reuse::FULL_SUFFIX);
            if options.sides.iter().any(|(known, _)| *known == name) {
                return Err(format!(
                    "--side {name} collides with the --verify-reuse side of that name"
                ));
            }
            Some(name)
        }
        _ => None,
    };
    let needs_inputs = specs
        .iter()
        .any(|(_, spec)| spec.runs() || matches!(spec, SideSpec::Baseline(_)));
    let (spelled, spelling) = if needs_inputs {
        std::fs::create_dir_all(mount_root.join("src"))
            .and_then(|()| std::fs::create_dir_all(mount_root.join(sides::WORK_DIRECTORY)))
            .and_then(|()| std::fs::create_dir_all(mount_root.join("mirror")))
            .map_err(|error| format!("{}: {error}", mount_root.display()))?;
        mirror::prepare(
            &routes,
            &inputs_base,
            &output.join("mirror"),
            &mount_root.join("mirror"),
            mode,
        )?
    } else {
        (
            BTreeMap::new(),
            json!({"spelling": "not prepared (no side runs or reuses a baseline)"}),
        )
    };
    let mirror_binding = (spelling["spelling"] == json!("mirror"))
        .then(|| (output.join("mirror"), mount_root.join("mirror")));
    let inputs_of = |route: &Route| -> Result<Value, String> {
        let databases: BTreeMap<String, String> = spelled
            .get(&route.name)
            .map(|entry| {
                entry
                    .databases
                    .iter()
                    .map(|path| {
                        let relative = path
                            .strip_prefix(&output)
                            .or_else(|_| path.strip_prefix(&inputs_base))
                            .unwrap_or(path);
                        Ok((relative.display().to_string(), sha_file(path)?))
                    })
                    .collect::<Result<_, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(baseline::inputs_record(
            route,
            &spelling,
            &databases,
            source_date_epoch,
        ))
    };
    // Build every running side, in parallel; a reuse candidate's tip waits
    // for its base side's binary, below.
    let deferred_tip = reuse_candidate.as_ref().ok().map(|(_, tip)| tip.clone());
    let running: Vec<&(String, SideSpec)> = specs
        .iter()
        .filter(|(name, spec)| spec.runs() && Some(name) != deferred_tip.as_ref())
        .collect();
    let built: Vec<Result<Built, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = running
            .iter()
            .map(|(name, spec)| {
                let context = &context;
                scope.spawn(move || {
                    eprintln!("measure: preparing side {name} ({})", spec.kind());
                    sides::build(name, spec, context)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| Err("a side's build panicked".into()))
            })
            .collect()
    });
    let mut built: Vec<Built> = built.into_iter().collect::<Result<_, _>>()?;
    // The tip of a reuse candidate: checked out, classified by the base
    // side's binary and given that binary; any refusal builds it as a
    // separate side, exactly as without the option (fail-closed).
    let mut reuse_record = json!({
        "requested": options.reuse_contract_only,
        "taken": false,
        "reason": reuse_candidate.as_ref().err(),
    });
    let mut reuse_plan: Option<(String, String)> = None;
    if let Ok((base_name, tip_name)) = &reuse_candidate {
        let spec = &specs
            .iter()
            .find(|(name, _)| name == tip_name)
            .expect("a candidate side")
            .1;
        let base = built
            .iter()
            .find(|side| &side.name == base_name)
            .expect("a built base")
            .clone();
        eprintln!("measure: preparing side {tip_name} under side {base_name}'s analyzer");
        match reuse::prepare_tip(tip_name, spec, &base, &context) {
            Ok(tip) => {
                reuse_record = json!({
                    "requested": true,
                    "taken": true,
                    "base": base_name,
                    "tip": tip_name,
                    "base_revision": base.revision,
                    "tip_revision": tip.revision,
                    "binary_sha256": tip.record["binary"]["sha256"],
                    "build_commit": tip.build_commit,
                    "classification": tip.record["contract_only_reuse"]["classification"],
                    "working_directory_as_run":
                        mount_root.join(sides::WORK_DIRECTORY).display().to_string(),
                    "checkpoints": routes.iter()
                        .filter(|route| matches!(route.kind, inputs::RouteKind::PinnedCorpus { .. }))
                        .map(|route| (route.name.clone(),
                            reuse::checkpoint_directory(&output, &route.name).display().to_string()))
                        .collect::<BTreeMap<_, _>>(),
                    "verification": full_side.as_ref().map_or(Value::Null, |_| json!("pending")),
                });
                if let Some(full_name) = &full_side {
                    let mut full = tip.clone();
                    full.name.clone_from(full_name);
                    full.directory = output.join("sides").join(full_name);
                    std::fs::create_dir_all(&full.directory).map_err(|error| error.to_string())?;
                    full.record["side"] = json!(full_name);
                    full.record["contract_only_reuse"]["verifies"] = json!(tip_name);
                    built.push(tip);
                    built.push(full);
                } else {
                    built.push(tip);
                }
                reuse_plan = Some((base_name.clone(), tip_name.clone()));
            }
            Err(reason) => {
                eprintln!("measure: side {tip_name} is measured separately: {reason}");
                reuse_record["reason"] = json!(reason);
                reuse::discard(&context, tip_name)?;
                built.push(sides::build(tip_name, spec, &context)?);
            }
        }
    }
    if let Some(tip) = &switched {
        let base = built
            .iter()
            .find(|side| &side.name == tip)
            .expect("built")
            .clone();
        if !sides::advertises_rule_switch(&base.binary_real)? {
            return Err(format!(
                "--rules-off {}: not yet supported. The binary of side `{tip}` ({}) does not \
                 advertise the run-time rule switch {} (card RULE-SWITCHES, plan seam B has not \
                 landed in it); the switch is never ignored silently",
                rules_off.join(","),
                base.binary_real.display(),
                sides::RULES_OFF_VARIABLE
            ));
        }
        for (name, off_list) in &switched_sides {
            let mut off = base.clone();
            off.name.clone_from(name);
            off.directory = output.join("sides").join(&off.name);
            std::fs::create_dir_all(&off.directory).map_err(|error| error.to_string())?;
            off.record["side"] = json!(off.name);
            off.record["rules_off"] = json!(off_list);
            built.push(off);
        }
    }
    // Run every route of every running side; the slots bound concurrency.
    // Every job is begun here, before any can wait, so the tracker never
    // takes a run whose other routes are still starting for an idle one.
    let jobs: Vec<(usize, &Route)> = (0..built.len())
        .flat_map(|side| routes.iter().map(move |route| (side, route)))
        .collect();
    let begun: Vec<waits::Job> = jobs.iter().map(|_| tracker.begin()).collect();
    // Card MEASURE-CONTRACT-ONLY-SIDE-WIRING: the base side's pinned routes
    // that have finished; the tip's wait for theirs before resuming.
    let sealed = (
        std::sync::Mutex::new(std::collections::BTreeSet::<String>::new()),
        std::sync::Condvar::new(),
    );
    let results: Vec<(usize, String, Result<Value, String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .zip(begun)
            .map(|(&(side, route), mut job)| {
                let built = &built;
                let spelled = &spelled;
                let context = &context;
                let mirror_binding = &mirror_binding;
                let switched_sides = &switched_sides;
                let reuse_plan = &reuse_plan;
                let sealed = &sealed;
                let output = &output;
                scope.spawn(move || {
                    let mut context = context.clone();
                    if let Some(off_list) = switched_sides.get(&built[side].name)
                        && built[side].record["rules_off"].is_array()
                    {
                        context.rules_off.clone_from(off_list);
                    }
                    let pinned = matches!(route.kind, inputs::RouteKind::PinnedCorpus { .. });
                    let role = reuse_plan
                        .as_ref()
                        .filter(|_| pinned)
                        .and_then(|(base, tip)| {
                            if &built[side].name == base {
                                Some(None)
                            } else if &built[side].name == tip {
                                Some(Some(&built[side]))
                            } else {
                                None
                            }
                        });
                    // Signals the base route's end however it ends.
                    struct Signal<'a>(
                        &'a (
                            std::sync::Mutex<std::collections::BTreeSet<String>>,
                            std::sync::Condvar,
                        ),
                        String,
                    );
                    impl Drop for Signal<'_> {
                        fn drop(&mut self) {
                            let (done, changed) = self.0;
                            done.lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .insert(self.1.clone());
                            changed.notify_all();
                        }
                    }
                    let mut signal = None;
                    match role {
                        Some(None) => {
                            context.checkpoint = Some(sides::Checkpoint {
                                directory: reuse::checkpoint_directory(output, &route.name),
                                accept_base: None,
                            });
                            signal = Some(Signal(sealed, route.name.clone()));
                        }
                        // The tip resumes from what the base side sealed: it
                        // starts once the base side's route has ended.
                        Some(Some(tip)) => {
                            let (done, changed) = sealed;
                            let mut finished = done
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            while !finished.contains(&route.name) {
                                finished = changed
                                    .wait(finished)
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                            }
                            drop(finished);
                            let base = reuse_plan.as_ref().map(|(base, _)| base).expect("planned");
                            let revision = built
                                .iter()
                                .find(|candidate| &candidate.name == base)
                                .and_then(|candidate| candidate.revision.clone())
                                .expect("a rev side has a revision");
                            context.checkpoint = Some(sides::Checkpoint {
                                directory: reuse::checkpoint_directory(output, &route.name),
                                accept_base: Some((revision, tip.source_run.clone())),
                            });
                        }
                        None => {}
                    }
                    eprintln!("measure: side {} route {}", built[side].name, route.name);
                    let result = sides::run_route(
                        &built[side],
                        route,
                        &spelled[&route.name],
                        mirror_binding
                            .as_ref()
                            .map(|(real, fixed)| (real.as_path(), fixed.as_path())),
                        &context,
                        &mut job,
                    );
                    drop(signal);
                    drop(job);
                    (side, route.name.clone(), result)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a route thread panicked"))
            .collect()
    });
    let mut failures = Vec::new();
    for (side, route_name, result) in results {
        let route = routes
            .iter()
            .find(|route| route.name == route_name)
            .expect("known");
        let entry = match result {
            Ok(mut record) => {
                record["route_definition"] =
                    serde_json::to_value(route).map_err(|e| e.to_string())?;
                record["inputs"] = inputs_of(route)?;
                let report = built[side].directory.join(&route.name).join(route.report());
                record["report_sha256"] = json!(sha_file(&report).ok());
                record["sidecar_sha256"] =
                    json!(sha_file(&report.with_extension("components.json")).ok());
                if record["exit"] != json!(0) {
                    failures.push(format!(
                        "side {} route {} exited {}",
                        built[side].name, route.name, record["exit"]
                    ));
                }
                record
            }
            Err(error) => {
                failures.push(format!(
                    "side {} route {}: {error}",
                    built[side].name, route.name
                ));
                json!({"profile": route.profile, "error": error, "exit": null, "stable": false})
            }
        };
        built[side].record["routes"][&route_name] = entry;
    }
    for side in &mut built {
        side.record["schema"] = json!(RUN_SCHEMA);
        side.record["tool"] = tool();
        side.record["mirror"] = spelling.clone();
        side.record["inputs_base"] = json!(inputs_base.display().to_string());
        side.record["repository"] = json!(repository.display().to_string());
        write_json(&side.directory.join("run.json"), &side.record)?;
    }
    // Resolve every side as the comparison reads it.
    let mut resolved: BTreeMap<String, Resolved> = BTreeMap::new();
    for side in &built {
        resolved.insert(
            side.name.clone(),
            Resolved {
                spec: options
                    .sides
                    .iter()
                    .find(|(name, _)| *name == side.name)
                    .map_or_else(
                        || {
                            format!(
                                "{} with --rules-off {}",
                                side.spec.kind(),
                                switched_sides
                                    .get(&side.name)
                                    .map(|ids| ids.join(","))
                                    .unwrap_or_default()
                            )
                        },
                        |(_, spec)| spec.clone(),
                    ),
                kind: side.spec.kind(),
                directory: side.directory.clone(),
                record: side.record.clone(),
                routes: side.record["routes"]
                    .as_object()
                    .map(|routes| routes.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
            },
        );
    }
    let baselines_root = options.baselines.as_deref().map_or_else(
        || repository.join("private/baselines"),
        |root| absolute(request, root),
    );
    for (name, spec) in &specs {
        let text = options
            .sides
            .iter()
            .find(|(known, _)| known == name)
            .expect("known")
            .1
            .clone();
        match spec {
            SideSpec::Baseline(sha) => {
                let full = sides::resolve_revision(&repository, sha)?;
                let staging = output.join("sides").join(name);
                std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
                let mut route_records = BTreeMap::new();
                let mut manifests = BTreeMap::new();
                for route in &routes {
                    match baseline::verify(
                        &repository,
                        &baselines_root,
                        &full,
                        route,
                        &inputs_of(route)?,
                    ) {
                        Ok((directory, manifest)) => {
                            let run = read_json(&directory.join("run.json"))?;
                            let mut route_record = run["routes"][&route.name].clone();
                            route_record["baseline_directory"] =
                                json!(directory.display().to_string());
                            route_records.insert(route.name.clone(), route_record);
                            manifests.insert(route.name.clone(), manifest);
                        }
                        Err(error) => {
                            failures.push(error.clone());
                            route_records
                                .insert(route.name.clone(), json!({"error": error, "exit": null}));
                        }
                    }
                }
                let record = json!({"schema": RUN_SCHEMA, "side": name, "kind": "baseline",
                    "revision": full, "baselines": baselines_root.display().to_string(),
                    "manifests": manifests, "routes": route_records, "tool": tool()});
                write_json(&staging.join("run.json"), &record)?;
                resolved.insert(
                    name.clone(),
                    Resolved {
                        spec: text,
                        kind: "baseline",
                        directory: baselines_root.join(&full),
                        record,
                        routes: route_records,
                    },
                );
            }
            SideSpec::Run(directory) => {
                let directory = canonical(directory)?;
                let record = read_json(&directory.join("run.json")).unwrap_or_else(|_| {
                    json!({"schema": null, "note": "no run.json: an unrecorded side directory, read as it is"})
                });
                let routes_recorded = record["routes"]
                    .as_object()
                    .map(|routes| routes.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                resolved.insert(
                    name.clone(),
                    Resolved {
                        spec: text,
                        kind: "run",
                        directory,
                        record,
                        routes: routes_recorded,
                    },
                );
            }
            _ => {}
        }
    }
    let mut comparison_pairs = comparisons.clone();
    let mut per_rule_labels = std::collections::BTreeSet::new();
    // The verification comparison (base against the full side) is evidence
    // for the reuse, not a measured comparison: its rows are no interaction.
    let verification_label = match (&reuse_plan, &full_side) {
        (Some((base, _)), Some(full)) => {
            comparison_pairs.push((base.clone(), full.clone()));
            per_rule_labels.insert(format!("{base}->{full}"));
            Some(format!("{base}->{full}"))
        }
        _ => None,
    };
    if let Some(switch) = &switch {
        for (side, _) in switch.sides() {
            if side != switch.joint {
                per_rule_labels.insert(switch.label(&side));
            }
            comparison_pairs.push((side, switch.tip.clone()));
        }
    }
    // Compare.
    let mut records = Vec::new();
    let mut moved: BTreeMap<String, BTreeMap<String, (String, String)>> = BTreeMap::new();
    let mut declared_routes = BTreeMap::new();
    let mut declared_ok = true;
    for (position, (before, after)) in comparison_pairs.iter().enumerate() {
        let label = format!("{before}->{after}");
        let mut per_route = BTreeMap::new();
        for route in &routes {
            let side_route = |side: &Resolved| -> Result<PathBuf, String> {
                if let Some(recorded) = side.routes.get(&route.name) {
                    if recorded["exit"] != json!(0) {
                        return Err(format!(
                            "the route did not complete (exit {})",
                            recorded["exit"]
                        ));
                    }
                    if let Some(directory) = recorded["baseline_directory"].as_str() {
                        return Ok(PathBuf::from(directory).join(&route.name));
                    }
                }
                Ok(side.directory.join(&route.name))
            };
            let (left, right) = (&resolved[before], &resolved[after]);
            let entry = (|| -> Result<Value, String> {
                let (left_dir, right_dir) = (side_route(left)?, side_route(right)?);
                let left_rows = rows::read(&left_dir.join(route.report()), options.legacy_rows)?;
                let right_rows = rows::read(&right_dir.join(route.report()), options.legacy_rows)?;
                let (mut record, measured) = compare::rows(&left_rows, &right_rows);
                let deltas = compare::deltas(&left_rows, &right_rows);
                record["status_deltas"] = json!(deltas);
                record["composition"] = match (&left_rows.contribution, &right_rows.contribution) {
                    (Some(left), Some(right)) => {
                        let (_, union) = compare::union(left, right);
                        if union["conflicts"].as_array().is_some_and(|c| !c.is_empty()) {
                            record["status"] = json!("incomplete");
                            if let Some(reasons) = record["incomplete_because"].as_array_mut() {
                                reasons.push(json!(
                                    "composition-conflict in the union of the two sides (named)"
                                ));
                            }
                        }
                        union
                    }
                    _ => json!({"union": "not formed: a side has no component sidecar"}),
                };
                for (rows, side) in [(&left_rows, before), (&right_rows, after)] {
                    if let Some(off) = rules_off_mismatch(rows, side, &switched_sides) {
                        return Err(off);
                    }
                }
                if switched_sides.contains_key(before.as_str()) {
                    let contracts = right_dir
                        .join(route.report())
                        .with_extension("contracts.json");
                    let contracts = contracts
                        .is_file()
                        .then(|| read_json(&contracts))
                        .transpose()?;
                    compare::annotate(&mut record, &right_rows, contracts.as_ref());
                }
                // A single-rule comparison's rows are attributed in the
                // contribution slot; they are not comparison interactions.
                for movement in measured
                    .iter()
                    .filter(|_| !per_rule_labels.contains(&label))
                {
                    moved
                        .entry(format!("{}::{}", route.name, movement.key))
                        .or_default()
                        .insert(
                            label.clone(),
                            (movement.before.clone(), movement.after.clone()),
                        );
                }
                if position == 0
                    && let Some((_, _, declaration)) = &declaration
                {
                    let check =
                        declared::check_route(declaration.get(&route.name), &measured, &deltas);
                    declared_ok &= check["result"] == json!("match");
                    declared_routes.insert(route.name.clone(), check);
                }
                drop((left_rows, right_rows));
                let mut figures = serde_json::Map::new();
                let mut against = serde_json::Map::new();
                for (label, directory) in [("before", &left_dir), ("after", &right_dir)] {
                    if let Some((summary, baseline)) =
                        profile_figures(&repository, route, directory)?
                    {
                        figures.insert(label.into(), summary);
                        against.insert(label.into(), baseline);
                    }
                }
                record["figures"] = Value::Object(figures);
                record["profile_baseline"] = Value::Object(against);
                record["artifacts"] = compare::artifacts(&left_dir, &right_dir)?;
                record["before_directory"] = json!(left_dir.display().to_string());
                record["after_directory"] = json!(right_dir.display().to_string());
                Ok(record)
            })();
            let entry = entry.unwrap_or_else(|error| {
                failures.push(format!("comparison {label} route {}: {error}", route.name));
                if position == 0 && declaration.is_some() {
                    declared_ok = false;
                    declared_routes.insert(
                        route.name.clone(),
                        json!({"result": "not-compared", "why": error}),
                    );
                }
                json!({"status": "not-compared", "why": error})
            });
            per_route.insert(route.name.clone(), entry);
        }
        records.push(
            json!({"comparison": label, "before": before, "after": after, "routes": per_route}),
        );
    }
    // `--verify-reuse`: the reused tip against the full side, per route.
    if let (Some(label), Some((_, tip)), Some(full)) =
        (&verification_label, &reuse_plan, &full_side)
    {
        let first = &records[0]["routes"];
        let against = records
            .iter()
            .find(|record| record["comparison"] == json!(label))
            .map(|record| record["routes"].clone())
            .unwrap_or(Value::Null);
        let mut verified = BTreeMap::new();
        for route in &routes {
            let directories = [tip, full].map(|name| {
                built
                    .iter()
                    .find(|side| &side.name == name)
                    .map(|side| side.directory.join(&route.name))
            });
            let entry = match directories {
                [Some(reused), Some(fresh)] => match compare::artifacts(&reused, &fresh) {
                    Ok(artifacts) => {
                        reuse::verify_route(&artifacts, &first[&route.name], &against[&route.name])
                    }
                    Err(error) => json!({"error": error}),
                },
                _ => json!({"error": "a side was not run"}),
            };
            if entry["artifacts_byte_identical"] != json!(true)
                || entry["comparison_equal"] != json!(true)
            {
                failures.push(format!(
                    "--verify-reuse route {}: the reused side {tip} is not byte-identical to the \
                     full side {full}",
                    route.name
                ));
            }
            verified.insert(route.name.clone(), entry);
        }
        reuse_record["verification"] = json!({"full_side": full, "comparison": label,
                                              "routes": verified});
    }
    let declared_record = declaration
        .as_ref()
        .map_or(Value::Null, |(path, digest, _)| {
            json!({"path": path.display().to_string(), "sha256": digest,
               "comparison": format!("{}->{}", comparisons[0].0, comparisons[0].1),
               "result": if declared_ok { "match" } else { "mismatch" },
               "routes": declared_routes})
        });
    let sides_record: BTreeMap<String, Value> = resolved
        .iter()
        .map(|(name, side)| {
            let run_json = side.directory.join("run.json");
            let run_json = if side.kind == "baseline" {
                output.join("sides").join(name).join("run.json")
            } else {
                run_json
            };
            (name.clone(), json!({
                "spec": side.spec,
                "kind": side.kind,
                "directory": side.directory.display().to_string(),
                "revision": side.record["revision"],
                "build_commit": side.record["build_commit"],
                "binary": side.record["binary"],
                "bundles": side.record["bundles"],
                "inputs": routes.iter().map(|route| (route.name.clone(),
                    side.routes.get(&route.name).map_or(Value::Null, |r| r["inputs"].clone())))
                    .collect::<BTreeMap<_, _>>(),
                "run_json": {"path": run_json.display().to_string(), "sha256": sha_file(&run_json).ok()},
            }))
        })
        .collect();
    let route_profiles: Vec<(String, String)> = routes
        .iter()
        .map(|route| (route.name.clone(), route.profile.clone()))
        .collect();
    let contribution = switch.as_ref().map_or_else(
        || ledger::slot(None, &records, &route_profiles, &Value::Null),
        |switch| {
            let tip = &sides_record[&switch.tip];
            let measurement = json!({
                "tool": tool(),
                "tip_side": switch.tip,
                "binary_sha256": tip["binary"]["sha256"],
                "revision": tip["revision"],
                "build_commit": tip["build_commit"],
                "bundles": tip["bundles"],
                "source_date_epoch": source_date_epoch,
            });
            if failures.is_empty() {
                ledger::slot(Some(switch), &records, &route_profiles, &measurement)
            } else {
                Ok(
                    json!({"schema": ledger::LEDGER_SCHEMA, "rules": switch.rules,
                          "ledgers": [], "refused": "the measurement failed; no ledger is built"}),
                )
            }
        },
    );
    let contribution = contribution.unwrap_or_else(|error| {
        failures.push(format!("contribution ledgers: {error}"));
        json!({"schema": ledger::LEDGER_SCHEMA, "rules": rules_off, "ledgers": [],
               "refused": error})
    });
    failures.extend(tracker.errors());
    let route_times: BTreeMap<String, BTreeMap<String, Value>> = built
        .iter()
        .map(|side| {
            let routes = routes
                .iter()
                .map(|route| {
                    let recorded = &side.record["routes"][&route.name];
                    (
                        route.name.clone(),
                        json!({"profile": route.profile,
                               "wall_time_ms": recorded["wall_time_ms"],
                               "slot_wait_ms": recorded["slot_wait_ms"],
                               "analysis_time_ms": recorded["analysis_time_ms"]}),
                    )
                })
                .collect();
            (side.name.clone(), routes)
        })
        .collect();
    let timing = waits::timing(
        started.elapsed(),
        &tracker,
        &json!(route_times),
        &request.quotas,
    );
    let complete = failures.is_empty();
    let record = json!({
        "schema": COMPARISON_SCHEMA,
        "tool": tool(),
        "status": if complete { "complete" } else { "failed" },
        "failures": failures,
        "routes": routes.iter().map(|route| json!({"name": route.name, "profile": route.profile,
            "digest": route.digest(), "definition": route})).collect::<Vec<_>>(),
        "inputs_spelling": spelling,
        "sides": sides_record,
        "comparisons": records,
        "interactions": compare::interactions(&moved),
        "declared": declared_record,
        "rules_off": rules_off,
        "source_date_epoch": source_date_epoch,
        "contribution": contribution,
        "contract_only_reuse": reuse_record,
        "timing": timing,
    });
    let record_path = output.join("native-comparison.json");
    write_json(&record_path, &record)?;
    let mut response = Response::completed();
    response.push_artifact(
        &output,
        Path::new("native-comparison.json"),
        "native-comparison",
    )?;
    for side in &built {
        response.push_artifact(&output, &side.directory.join("run.json"), "measure-run")?;
    }
    if spelling["spelling"] == json!("mirror") {
        response.push_artifact(
            &output,
            Path::new("mirror/manifest.json"),
            "measure-mirror-manifest",
        )?;
    }
    for comparison in records.iter() {
        for (route, entry) in comparison["routes"].as_object().into_iter().flatten() {
            if entry["status"] == json!("incomplete") {
                response.push_diagnostic(
                    Severity::Warning,
                    "comparison-incomplete",
                    format!(
                        "{} route {route}: {}",
                        comparison["comparison"], entry["incomplete_because"]
                    ),
                );
            }
        }
    }
    if !complete {
        return Ok(Response::failed(format!(
            "measurement failed ({}); the record is {}",
            record["failures"].as_array().map_or(0, Vec::len),
            record_path.display()
        )));
    }
    if declaration.is_some() && !declared_ok {
        return Ok(Response::failed(format!(
            "declared-versus-measured mismatch: every unpredicted or missing row is named in {}",
            record_path.display()
        )));
    }
    Ok(response)
}

/// A route directory's figures as `scripts/evaluation_profiles.py summarize`
/// computes them (the one authority for the figures; card EVALUATION-PROFILES),
/// and, when the route's profile has a committed record, `compare`'s verdict
/// against its baseline (exit 0: reproduced, 1: moved, 2: frozen) with its
/// lines. `None` when the repository has no such script (an older tree).
///
/// # Errors
///
/// The script fails to summarize an existing report: the figures are then not
/// known, so the route is not compared.
fn profile_figures(
    repository: &Path,
    route: &Route,
    directory: &Path,
) -> Result<Option<(Value, Value)>, String> {
    let script = repository.join("scripts/evaluation_profiles.py");
    if !script.is_file() {
        return Ok(None);
    }
    let report = directory.join(route.report());
    let null = directory.join(format!("{}-null.jsonl", route.name));
    let run = |arguments: &[&str]| {
        std::process::Command::new("python3")
            .arg(&script)
            .args(arguments)
            .arg("--report")
            .arg(&report)
            .arg("--null")
            .arg(&null)
            .current_dir(repository)
            .output()
            .map_err(|error| format!("python3 {}: {error}", script.display()))
    };
    let summary = run(&["summarize"])?;
    if !summary.status.success() {
        return Err(format!(
            "evaluation_profiles.py summarize failed on {}: {}",
            report.display(),
            String::from_utf8_lossy(&summary.stderr).trim()
        ));
    }
    let figures: Value = serde_json::from_slice(&summary.stdout)
        .map_err(|error| format!("evaluation_profiles.py summarize: {error}"))?;
    let record = repository
        .join("contract/evaluations")
        .join(&route.profile)
        .join("evaluation.json");
    let baseline = if record.is_file() {
        let compared = run(&["compare", &route.profile])?;
        json!({
            "profile": route.profile,
            "exit": compared.status.code(),
            "lines": String::from_utf8_lossy(&compared.stdout).lines().collect::<Vec<_>>(),
        })
    } else {
        json!({"profile": route.profile, "record": "none committed"})
    };
    Ok(Some((figures, baseline)))
}

/// After a rule-switched run, every component of a switched side must
/// record exactly that side's off-list; any other side must record none.
fn rules_off_mismatch(
    rows: &rows::Rows,
    side: &str,
    switched_sides: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    let expected: Vec<String> = switched_sides.get(side).cloned().unwrap_or_default();
    if rows.source != "sidecar" {
        return (!expected.is_empty())
            .then(|| format!("side {side}: no component sidecar records the rule off-list"));
    }
    let expected: std::collections::BTreeSet<String> = expected.into_iter().collect();
    let recorded = &rows.rule_off_lists;
    let consistent = if expected.is_empty() {
        recorded.iter().all(std::collections::BTreeSet::is_empty)
    } else {
        recorded.len() == 1 && recorded.contains(&expected)
    };
    (!consistent).then(|| {
        format!(
            "side {side}: the components record rule off-lists {recorded:?}, not {expected:?}; \
             a switch the analyzer did not apply is never reported as applied"
        )
    })
}
