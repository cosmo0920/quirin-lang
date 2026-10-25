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
//!     [--declared PREDICTION.md] [--rules-off IDS] [--repository DIR] \
//!     [--inputs-base DIR] [--jobs N] [--build-jobs N] [--mirror auto|always|never] \
//!     [--mount-root DIR] [--no-slots] [--build-commit TEXT] \
//!     [--source-date-epoch SECONDS|none] [--baselines DIR] \
//!     [--legacy-rows]
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
//! declared-versus-measured check; and the reserved per-rule `contribution`
//! slot. The tool proposes no verdict: it claims nothing through the
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
pub mod mirror;
pub mod rows;
pub mod sides;
pub mod util;

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
[--rules-off IDS] [--repository DIR] [--inputs-base DIR] [--jobs N] [--build-jobs N] \
[--mirror auto|always|never] [--mount-root DIR] [--no-slots] [--build-commit TEXT] \
[--source-date-epoch SECONDS|none] [--baselines DIR] [--legacy-rows]\n       quirin-audit --output <baselines>/<sha> measure \
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
        if options.publish_from.is_none() || !options.sides.is_empty() {
            return Err(format!(
                "publish takes --from SIDE_DIRECTORY and no --side\n{USAGE}"
            ));
        }
        return Ok(options);
    }
    if options.publish_from.is_some() {
        return Err("--from belongs to `measure publish`".into());
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
    let output = canonical(&request.output_directory())?;
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
                        "no evaluation-profile record {} (card EVALUATION-PROFILES has not \
                         landed here); pass --inputs FILE",
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
    let needs_inputs = specs
        .iter()
        .any(|(_, spec)| spec.runs() || matches!(spec, SideSpec::Baseline(_)));
    let (spelled, spelling) = if needs_inputs {
        std::fs::create_dir_all(mount_root.join("src"))
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
    // Build every running side, in parallel.
    let running: Vec<&(String, SideSpec)> = specs.iter().filter(|(_, spec)| spec.runs()).collect();
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
        let mut off = base;
        off.name = format!("{tip}-off");
        off.directory = output.join("sides").join(&off.name);
        std::fs::create_dir_all(&off.directory).map_err(|error| error.to_string())?;
        off.record["side"] = json!(off.name);
        off.record["rules_off"] = json!(rules_off);
        built.push(off);
    }
    // Run every route of every running side; the slots bound concurrency.
    let jobs: Vec<(usize, &Route)> = (0..built.len())
        .flat_map(|side| routes.iter().map(move |route| (side, route)))
        .collect();
    let results: Vec<(usize, String, Result<Value, String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|&(side, route)| {
                let built = &built;
                let spelled = &spelled;
                let context = &context;
                let mirror_binding = &mirror_binding;
                let rules_off = &rules_off;
                scope.spawn(move || {
                    let mut context = context.clone();
                    if built[side].name.ends_with("-off")
                        && built[side].record["rules_off"].is_array()
                    {
                        context.rules_off.clone_from(rules_off);
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
                    );
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
                        || format!("{} with --rules-off", side.spec.kind()),
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
    if let Some(tip) = &switched {
        comparison_pairs.push((format!("{tip}-off"), tip.clone()));
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
                    if let Some(off) = rules_off_mismatch(rows, side, &switched, &rules_off) {
                        return Err(off);
                    }
                }
                for movement in &measured {
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
        "contribution": {
            "reserved": "per-rule contribution (plan seam A): filled from the `<tip>-off -> <tip>` \
                         comparison once RULE-SWITCHES lands; CONTRIBUTION-LEDGERS defines its ledger",
            "rules": if switched.is_some() { json!(rules_off) } else { json!([]) },
        },
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

/// After a rule-switched run, every component of the switched side must
/// record exactly the requested off-list; any other side must record none.
fn rules_off_mismatch(
    rows: &rows::Rows,
    side: &str,
    switched: &Option<String>,
    rules_off: &[String],
) -> Option<String> {
    let expected: Vec<String> = match switched {
        Some(tip) if side == format!("{tip}-off") => rules_off.to_vec(),
        _ => Vec::new(),
    };
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
