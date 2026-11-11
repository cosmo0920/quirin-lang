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

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: quirin-audit CONFIG.json");
        eprintln!("       {}", qcore_audit::current_cli::USAGE);
        eprintln!("       {}", qcore_audit::pinned_cli::USAGE);
        eprintln!("       {}", qcore_audit::prepare::USAGE);
        eprintln!("       {}", qcore_audit::no_build::USAGE);
        eprintln!("       {}", qcore_audit::contract_pin::USAGE);
        eprintln!("       {}", qcore_audit::revision_compare::USAGE);
        eprintln!("       {}", qcore_audit::checkpoint_cli::USAGE);
        eprintln!("       {}", qcore_audit::plugin_cli::USAGE);
        eprintln!("       {}", qcore_audit::subcommand_plugins::USAGE);
        eprintln!("       {}", qcore_audit::atomic_publication::USAGE);
        return ExitCode::FAILURE;
    };
    // Card GATEWAY-RULE-API: a rule-switch list naming an unknown id, a hold
    // or a malformed element is refused before any route runs, never ignored.
    // Card RULE-SWITCH-ENTRY-POINTS: a command line that does not record the
    // rule set (`rule_set.off`) also refuses a non-empty list.
    let rest = std::env::args_os().skip(2).collect::<Vec<_>>();
    if let Err(refusal) = qcore_audit::rule_switch_entry::admit(&path, &rest) {
        eprintln!("{refusal}");
        return ExitCode::FAILURE;
    }
    if path == "pinned" {
        return qcore_audit::pinned_cli::main(arguments.collect());
    }
    if path == "checkpoint" {
        // Card W45-AUDIT-CHECKPOINTS: verify, decompose, recompose and compose
        // per-group `.qo` checkpoints.
        let mut rest = Vec::new();
        for argument in arguments {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return qcore_audit::checkpoint_cli::main(rest);
    }
    if path == "prepare" {
        // Card W44-QUIRIN-AUDIT-PREPARE: the Clang CMake build and the Monkey
        // inputs, formerly two Python scripts; `--run` chains into `current`.
        let mut rest = Vec::new();
        for argument in arguments {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return qcore_audit::prepare::main(rest);
    }
    if path == "current" {
        // Card W44-QUIRIN-AUDIT-PROVIDERS: the configured current-revision
        // route, with contracts selected by provider crate or manifest.
        let mut rest = Vec::new();
        for argument in arguments {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return qcore_audit::current_cli::main(rest);
    }
    if path == "plugin" {
        // Card W45-USER-CONTRACT-PLUGINS: the resource views of user crates.
        let mut rest = Vec::new();
        for argument in arguments {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return qcore_audit::plugin_cli::main(rest);
    }
    if path == "pin" || path == "compare" {
        // Card W45-CONTRACT-REVISION-MODE: write or check contract locks after
        // human review, and compare two runs.
        let mut rest = Vec::new();
        for argument in arguments {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return if path == "pin" {
            qcore_audit::contract_pin::main(rest)
        } else {
            qcore_audit::revision_compare::main(rest)
        };
    }
    if path == "contracts" {
        // Card W44-CONTRACTS-ONLY-NO-BUILD: contracts-only with no compilation
        // database or prepared build; verdicts relative to the assumptions.
        let mut rest = Vec::new();
        for argument in arguments {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return qcore_audit::no_build::main(rest);
    }
    if path == "atomic-publication" {
        // Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): a failed or
        // undecided publication check is Unknown and exits non-zero.
        let Some(config) = arguments.next() else {
            eprintln!("usage: {}", qcore_audit::atomic_publication::USAGE);
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: {}", qcore_audit::atomic_publication::USAGE);
            return ExitCode::FAILURE;
        }
        return match qcore_audit::atomic_publication::run_atomic_publication_config(&PathBuf::from(
            config,
        )) {
            Ok(true) => {
                println!(
                    "Bounded atomic publication checked (Safe, one candidate execution); report written."
                );
                ExitCode::SUCCESS
            }
            Ok(false) => {
                eprintln!("atomic publication not established (Unknown); report written.");
                ExitCode::FAILURE
            }
            Err(error) => {
                eprintln!("atomic publication rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "callback-task-handoff" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit callback-task-handoff CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit callback-task-handoff CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::callback_task_handoff::run_callback_task_handoff_config(
            &PathBuf::from(config),
        ) {
            Ok(()) => {
                println!("Bounded callback task-handoff obligations checked; report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("callback-task-handoff audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "callback-task-handoff-fixture" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit callback-task-handoff-fixture CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit callback-task-handoff-fixture CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::callback_task_handoff::run_taskful_fixture_config(&PathBuf::from(
            config,
        )) {
            Ok(()) => {
                println!("Bounded taskful callback handoff checked; report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("taskful callback handoff audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "cycle18-obligations" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit cycle18-obligations CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit cycle18-obligations CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::cycle18_composition::run_cycle18_obligations_config(
            &PathBuf::from(config),
        ) {
            Ok(()) => {
                println!(
                    "Per-callback Cycle 18 obligations measured; callbacks without every obligation discharged remain Unknown. Report written."
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Cycle 18 callback obligations rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "cycle18-composition" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit cycle18-composition CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit cycle18-composition CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::cycle18_composition::run_cycle18_composition_config(
            &PathBuf::from(config),
        ) {
            Ok(()) => {
                println!(
                    "Bounded Cycle 18 evidence joined; aggregate remains Unknown. Report written."
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Cycle 18 composition rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "timeout-teardown" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit timeout-teardown CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit timeout-teardown CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::timeout_teardown::run_timeout_teardown_config(&PathBuf::from(
            config,
        )) {
            Ok(()) => {
                println!("Bounded cb_timeout self-teardown checked; report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("timeout-teardown audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "native-platform-matrix" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit native-platform-matrix CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit native-platform-matrix CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::native_platform_matrix::run_native_platform_matrix_config(
            &PathBuf::from(config),
        ) {
            Ok(()) => {
                println!("Cycle 18 native platform matrix checked; bounded report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("native-platform-matrix audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "event-payload" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit event-payload CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit event-payload CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::event_payload::run_event_payload_config(&PathBuf::from(config)) {
            Ok(()) => {
                println!("Bounded cb_event payload checked; report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("event-payload audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "observed-wait" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit observed-wait CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit observed-wait CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::observed_wait::run_observed_wait_config(&PathBuf::from(config)) {
            Ok(()) => {
                println!("Observed timed-wait composition checked; bounded report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("observed-wait audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    if path == "wait-failures" {
        let Some(config) = arguments.next() else {
            eprintln!("usage: quirin-audit wait-failures CONFIG.json");
            return ExitCode::FAILURE;
        };
        if arguments.next().is_some() {
            eprintln!("usage: quirin-audit wait-failures CONFIG.json");
            return ExitCode::FAILURE;
        }
        return match qcore_audit::wait_failures::run_wait_failures_config(&PathBuf::from(config)) {
            Ok(()) => {
                println!("Timed-wait failure paths checked; bounded report written.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("wait-failures audit rejected (Unknown): {error}");
                ExitCode::FAILURE
            }
        };
    }
    // Card W45-AUDIT-SUBCOMMAND-PLUGINS: after every built-in, an external
    // subcommand name or a host option runs `quirin-audit-NAME` from the
    // declared plugin search path.
    if path
        .to_str()
        .is_some_and(qcore_audit::subcommand_plugins::claims)
    {
        let mut rest = Vec::new();
        for argument in std::iter::once(path).chain(arguments) {
            match argument.into_string() {
                Ok(argument) => rest.push(argument),
                Err(argument) => {
                    eprintln!("non-UTF-8 argument {argument:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
        return qcore_audit::subcommand_plugins::main(rest);
    }
    // Card W44-QUIRIN-AUDIT-PROVIDERS: `quirin-audit [--jobs N] CONFIG.json`
    // (or `CONFIG.json --jobs N`); the flag takes precedence over the
    // configuration's `jobs` and over QCORE_AUDIT_JOBS.
    let mut rest = std::iter::once(path).chain(arguments).collect::<Vec<_>>();
    let mut jobs = None;
    if let Some(position) = rest.iter().position(|argument| argument == "--jobs") {
        let Some(count) = rest.get(position + 1).and_then(|count| count.to_str()) else {
            eprintln!("--jobs requires a positive integer");
            return ExitCode::FAILURE;
        };
        match qcore_audit::parallel::resolve_jobs(Some(count)) {
            Ok(count) => jobs = Some(count),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
        rest.drain(position..=position + 1);
    }
    if rest.len() != 1 {
        eprintln!("usage: quirin-audit [--jobs N] CONFIG.json");
        return ExitCode::FAILURE;
    }
    let path = rest.remove(0);
    match qcore_audit::run_config_with_jobs(&PathBuf::from(path), jobs) {
        Ok(counts) => {
            println!(
                "{} units: {} Safe / {} Pending / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures",
                counts.units,
                counts.safe_units,
                counts.pending_units,
                counts.unsafe_units,
                counts.unknown_units,
                counts.data_only_units,
                counts.gateway_failures
            );
            println!(
                "preparation: {} unit(s), {} Clang invocation(s); -E -P {} us / syntax {} us / line markers {} us",
                counts.prepared_units,
                counts.preparation_clang_invocations,
                counts.plain_preprocessing_elapsed_us,
                counts.syntax_validation_elapsed_us,
                counts.line_marked_preprocessing_elapsed_us,
            );
            if counts.gateway_failures == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("audit rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
