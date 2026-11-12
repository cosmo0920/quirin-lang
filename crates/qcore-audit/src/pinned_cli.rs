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

//! Explicit replay of the historical Cycle 19 Fluent Bit evidence.
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

pub const USAGE: &str = "usage: quirin-audit pinned [--jobs N] [--checkpoint-dir DIR [--resume] \
[--accept-base-analyzer BASE --repository DIR]] \
[fluent-bit-root] [monkey-libevent-compile_commands.json] [monkey-callback-source-report.json] \
[resource-diagnostics.json] [native-observation-directory] [resource-contract-manifest.json]";

pub fn main(arguments: Vec<OsString>) -> ExitCode {
    let mut arguments = arguments.into_iter().peekable();
    let mut jobs = None;
    let mut jobs_seen = false;
    let mut literal_paths = false;
    // Card CORPUS-GROUP-CHECKPOINTS: per-group `.qo` checkpoints.
    let mut checkpoint_directory = None;
    let mut resume = false;
    // Card MEASURE-LOOKUP-RECORD-REUSE: a contract-only analyzer acceptance.
    let mut accept_base = None;
    let mut repository = None;
    while let Some(argument) = arguments.peek() {
        if argument == "--help" || argument == "-h" {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        } else if argument == "--" {
            arguments.next();
            literal_paths = true;
            break;
        } else if argument == "--jobs" {
            if jobs_seen {
                eprintln!("--jobs is given twice");
                return ExitCode::FAILURE;
            }
            jobs_seen = true;
            arguments.next();
            let Some(count) = arguments.next() else {
                eprintln!("--jobs requires a positive integer");
                return ExitCode::FAILURE;
            };
            jobs = Some(count.to_string_lossy().into_owned());
        } else if argument == "--checkpoint-dir" {
            if checkpoint_directory.is_some() {
                eprintln!("--checkpoint-dir is given twice");
                return ExitCode::FAILURE;
            }
            arguments.next();
            let Some(directory) = arguments.next().filter(|value| !value.is_empty()) else {
                eprintln!("--checkpoint-dir requires a directory");
                return ExitCode::FAILURE;
            };
            checkpoint_directory = Some(PathBuf::from(directory));
        } else if argument == "--accept-base-analyzer" || argument == "--repository" {
            let option = argument.to_string_lossy().into_owned();
            let slot = if option == "--repository" {
                &mut repository
            } else {
                &mut accept_base
            };
            if slot.is_some() {
                eprintln!("{option} is given twice");
                return ExitCode::FAILURE;
            }
            arguments.next();
            let Some(value) = arguments.next().filter(|value| !value.is_empty()) else {
                eprintln!("{option} requires a value");
                return ExitCode::FAILURE;
            };
            *slot = Some(value);
        } else if argument == "--resume" {
            if resume {
                eprintln!("--resume is given twice");
                return ExitCode::FAILURE;
            }
            arguments.next();
            resume = true;
        } else if argument.to_string_lossy().starts_with('-') {
            eprintln!(
                "unknown pinned option: {}\n{USAGE}",
                argument.to_string_lossy()
            );
            return ExitCode::FAILURE;
        } else {
            break;
        }
    }
    let positional = arguments.collect::<Vec<_>>();
    if !literal_paths
        && let Some(argument) = positional
            .iter()
            .find(|argument| argument.to_string_lossy().starts_with('-'))
    {
        eprintln!(
            "unknown pinned option: {}\n{USAGE}",
            argument.to_string_lossy()
        );
        return ExitCode::FAILURE;
    }
    if resume && checkpoint_directory.is_none() {
        eprintln!("--resume requires --checkpoint-dir");
        return ExitCode::FAILURE;
    }
    if accept_base.is_some() != repository.is_some() {
        eprintln!("--accept-base-analyzer and --repository are given together");
        return ExitCode::FAILURE;
    }
    if accept_base.is_some() && checkpoint_directory.is_none() {
        eprintln!("--accept-base-analyzer requires --checkpoint-dir");
        return ExitCode::FAILURE;
    }
    // The diff is classified before any work; the run that claims it is
    // checked once its contract inputs are resolved.
    let acceptance = match (accept_base, repository) {
        (Some(base), Some(repository)) => match crate::contract_only_diff::classify(
            std::path::Path::new(&repository),
            &base.to_string_lossy(),
            "HEAD",
        ) {
            Ok(acceptance) => Some(acceptance),
            Err(error) => {
                eprintln!("analyzer acceptance refused: {error}");
                return ExitCode::FAILURE;
            }
        },
        _ => None,
    };
    let checkpoints = checkpoint_directory.map(|directory| crate::pinned_route::CheckpointOptions {
        directory,
        resume,
        acceptance,
    });
    let mut arguments = positional.into_iter();
    let jobs = match crate::parallel::resolve_jobs(jobs.as_deref()) {
        Ok(jobs) => crate::parallel::effective_jobs(jobs),
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../fluent-bit"));
    let monkey_database = arguments.next().map(PathBuf::from);
    let monkey_source_report = arguments.next().map(PathBuf::from);
    let diagnostic_report = arguments.next().map(PathBuf::from);
    let binding_directory = arguments.next().map(PathBuf::from);
    let contract_manifest = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    // Card RULE-SWITCH-ENTRY-POINTS: only the diagnostic report's components
    // record `rule_set.off`; without it the run must not use a switch list.
    if diagnostic_report.is_none()
        && let Err(refusal) = qcore_c_gateway::gateway_rules::refuse_unrecorded(
            "quirin-audit pinned without a diagnostic report",
        )
    {
        eprintln!("{refusal}");
        return ExitCode::FAILURE;
    }
    match crate::pinned_route::run_with_checkpoints(
        &root,
        monkey_database.as_deref(),
        monkey_source_report.as_deref(),
        diagnostic_report.as_deref(),
        binding_directory.as_deref(),
        contract_manifest.as_deref(),
        jobs,
        checkpoints.as_ref(),
    ) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Fluent Bit library audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
