//! Card **RULE-SWITCH-ENTRY-POINTS** (discharges
//! `GATEWAY-RULE-API-OTHER-ENTRY-POINTS`): which `quirin-audit` command lines
//! record the gateway rule set they run under, and the admission of the ones
//! that do not.
//!
//! `QCORE_GATEWAY_RULES_OFF` changes the gateway's answers, so a published
//! result must say which rules ran (the verdict contract's scope and
//! assumptions). A recording route writes the effective list into every
//! report component (`rule_set.off`, [`crate::report_components::provenance`],
//! which `quirin-audit measure` checks after a run) and, for `current` and the
//! configuration route, into `invocation.json`. Every other command line runs
//! only with nothing switched off
//! ([`qcore_c_gateway::gateway_rules::refuse_unrecorded`]): it never applies
//! the list unrecorded and never silently ignores it. A refused list (an
//! unknown id, a hold, a malformed element) is refused on every command line.
//!
//! [`RECORDING`] and [`UNRECORDED`] partition
//! [`crate::subcommand_plugins::BUILTINS`] (a test holds it, and another holds
//! `BUILTINS` to `main.rs`), so a new built-in cannot dispatch unclassified.

use std::ffi::{OsStr, OsString};

/// Built-ins that write a component report carrying `rule_set.off`: `current`
/// and `contracts` refuse a non-empty list when they are given no report,
/// `pinned` when it is given no diagnostic report, and `prepare` records only
/// through the `current` run that `--run` chains into.
pub const RECORDING: [&str; 4] = ["contracts", "current", "pinned", "prepare"];

/// Built-ins whose output carries no rule set. Some run the gateway's lowering
/// (the bounded audits, `compare`'s hold classification), the others do not
/// analyse; none of them can say which rules ran, so each refuses a non-empty
/// list.
pub const UNRECORDED: [&str; 14] = [
    "atomic-publication",
    "callback-task-handoff",
    "callback-task-handoff-fixture",
    "checkpoint",
    "compare",
    "cycle18-composition",
    "cycle18-obligations",
    "event-payload",
    "native-platform-matrix",
    "observed-wait",
    "pin",
    "plugin",
    "timeout-teardown",
    "wait-failures",
];

/// How a command line treats the rule switch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recording {
    /// It writes `rule_set.off` (its route refuses a non-empty list where it
    /// would write no report).
    Records,
    /// It records nothing: a non-empty list is refused before it starts. The
    /// text names the command line in the refusal.
    Refuses(String),
}

/// The treatment of `quirin-audit FIRST REST...`.
#[must_use]
pub fn recording(first: &OsStr, rest: &[OsString]) -> Recording {
    let Some(first) = first.to_str() else {
        // Not a built-in or a plugin name: the configuration route.
        return Recording::Records;
    };
    if first == "prepare" {
        // `--run` chains into `current`, which records; the preparation
        // alone runs no analysis and records no rule set.
        return if rest.iter().any(|argument| argument == "--run") {
            Recording::Records
        } else {
            Recording::Refuses("quirin-audit prepare without --run".into())
        };
    }
    if RECORDING.contains(&first) {
        return Recording::Records;
    }
    if UNRECORDED.contains(&first) {
        return Recording::Refuses(format!("quirin-audit {first}"));
    }
    if crate::subcommand_plugins::claims(first) {
        // A subcommand plugin starts with a cleared environment (the host's
        // allow-list), so the list would be dropped, not applied; a switched
        // measurement is `quirin-audit measure --rules-off`.
        return Recording::Refuses("a quirin-audit subcommand plugin".into());
    }
    // `quirin-audit [--jobs N] CONFIG.json`: the report and `invocation.json`.
    Recording::Records
}

/// Admit `quirin-audit FIRST REST...` under this process's switch list.
///
/// # Errors
///
/// The list is refused (on every command line), or it is non-empty on a
/// command line that does not record it.
pub fn admit(first: &OsStr, rest: &[OsString]) -> Result<(), String> {
    match recording(first, rest) {
        Recording::Records => qcore_c_gateway::gateway_rules::check()
            .map(|_| ())
            .map_err(|refusal| format!("quirin-audit: {refusal}")),
        Recording::Refuses(label) => qcore_c_gateway::gateway_rules::refuse_unrecorded(&label),
    }
}

#[cfg(test)]
mod tests {
    use super::{RECORDING, Recording, UNRECORDED, recording};
    use std::collections::BTreeSet;
    use std::ffi::{OsStr, OsString};

    #[test]
    fn the_two_lists_partition_the_builtins() {
        let recording: BTreeSet<&str> = RECORDING.into_iter().collect();
        let unrecorded: BTreeSet<&str> = UNRECORDED.into_iter().collect();
        assert!(recording.is_disjoint(&unrecorded));
        let union: BTreeSet<&str> = recording.union(&unrecorded).copied().collect();
        let builtins: BTreeSet<&str> = crate::subcommand_plugins::BUILTINS.into_iter().collect();
        assert_eq!(union, builtins);
    }

    #[test]
    fn recording_routes_plugins_and_prepare_are_classified() {
        for name in UNRECORDED {
            assert_eq!(
                recording(OsStr::new(name), &[]),
                Recording::Refuses(format!("quirin-audit {name}"))
            );
        }
        for name in ["current", "pinned", "contracts", "config.json", "./x"] {
            assert_eq!(recording(OsStr::new(name), &[]), Recording::Records);
        }
        assert!(matches!(
            recording(OsStr::new("prepare"), &[]),
            Recording::Refuses(_)
        ));
        assert_eq!(
            recording(OsStr::new("prepare"), &[OsString::from("--run")]),
            Recording::Records
        );
        for plugin in ["measure", "libevent", "--plugin-path", "--list-plugins"] {
            assert_eq!(
                recording(OsStr::new(plugin), &[]),
                Recording::Refuses("a quirin-audit subcommand plugin".into())
            );
        }
    }
}
