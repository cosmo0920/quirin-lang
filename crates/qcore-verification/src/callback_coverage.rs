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

//! Total accounting for a source-selected callback inventory.
//!
//! This adapter deliberately does not manufacture a composed certificate from
//! source paths or native replay. A caller either supplies complete source and
//! evidence rows for every selected callback, or receives a precise `Unknown`
//! reason on that callback. The remaining source-to-reactor projection gap is
//! retained even when every bounded path was replayed.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackSourceInventory {
    pub function: String,
    pub source_revision: Option<String>,
    pub selected_paths: Option<usize>,
    pub unknown_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackExecutionEvidence {
    pub function: String,
    pub checked_paths: BTreeSet<usize>,
    pub checked_executions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Cycle18CallbackStatus {
    Unknown { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallbackCoverage {
    pub function: String,
    pub source_revision: Option<String>,
    pub selected_paths: Option<usize>,
    pub checked_paths: BTreeSet<usize>,
    pub missing_paths: BTreeSet<usize>,
    pub checked_executions: usize,
    pub cycle18_composition: Cycle18CallbackStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallbackCoverageReport {
    pub source_profile_assumptions: BTreeMap<String, String>,
    pub selected_callbacks: usize,
    pub accounted_callbacks: usize,
    pub selected_paths: usize,
    pub checked_paths: usize,
    pub callbacks: Vec<CallbackCoverage>,
}

/// Account for every selected callback without treating replay coverage as a
/// Cycle 18 event-program certificate.
pub fn account_callback_coverage(
    selected: &BTreeSet<String>,
    source_profile_assumptions: BTreeMap<String, String>,
    inventory: Vec<CallbackSourceInventory>,
    evidence: Option<Vec<CallbackExecutionEvidence>>,
) -> Result<CallbackCoverageReport, String> {
    if selected.is_empty() {
        return Err("callback profile selects no callbacks".into());
    }
    let mut sources = BTreeMap::new();
    for row in inventory {
        if !selected.contains(&row.function) || sources.insert(row.function.clone(), row).is_some()
        {
            return Err("source inventory does not exactly name selected callbacks".into());
        }
    }
    if sources.len() != selected.len() {
        return Err("source inventory omits a selected callback".into());
    }

    let mut executions = BTreeMap::new();
    if let Some(evidence) = evidence {
        for row in evidence {
            if !selected.contains(&row.function)
                || executions.insert(row.function.clone(), row).is_some()
            {
                return Err("execution evidence does not exactly name selected callbacks".into());
            }
        }
        if executions.len() != selected.len() {
            return Err("execution evidence omits a selected callback".into());
        }
    }

    let evidence_supplied = !executions.is_empty();
    let mut callbacks = Vec::with_capacity(selected.len());
    for function in selected {
        let source = &sources[function];
        if source.selected_paths.is_some() == source.unknown_reason.is_some() {
            return Err(format!(
                "callback `{function}` must carry either a path inventory or a source Unknown reason"
            ));
        }
        let execution = executions.get(function);
        let checked_paths = execution
            .map(|row| row.checked_paths.clone())
            .unwrap_or_default();
        match source.selected_paths {
            Some(0) => return Err(format!("callback `{function}` has an empty path inventory")),
            Some(_)
                if source
                    .source_revision
                    .as_deref()
                    .is_none_or(|revision| revision.trim().is_empty()) =>
            {
                return Err(format!(
                    "callback `{function}` path inventory lacks a source revision"
                ));
            }
            None if source
                .unknown_reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty()) =>
            {
                return Err(format!(
                    "callback `{function}` source Unknown lacks a reason"
                ));
            }
            None if execution.is_some_and(|row| {
                row.checked_executions != 0 || !row.checked_paths.is_empty()
            }) =>
            {
                return Err(format!(
                    "callback `{function}` has execution evidence without a source path inventory"
                ));
            }
            _ => {}
        }
        if let Some(execution) = execution
            && (execution.checked_executions < checked_paths.len()
                || (execution.checked_executions > 0) != !checked_paths.is_empty())
        {
            return Err(format!(
                "callback `{function}` execution count does not support its checked path inventory"
            ));
        }
        if let Some(total) = source.selected_paths
            && checked_paths.iter().any(|path| *path >= total)
        {
            return Err(format!(
                "callback `{function}` evidence names a path outside its source inventory"
            ));
        }
        let missing_paths: BTreeSet<usize> = source
            .selected_paths
            .map(|total| {
                (0..total)
                    .filter(|path| !checked_paths.contains(path))
                    .collect()
            })
            .unwrap_or_default();
        let checked_executions = execution.map_or(0, |row| row.checked_executions);
        let reason = if let Some(reason) = &source.unknown_reason {
            format!("callback source import is Unknown: {reason}")
        } else if !evidence_supplied {
            "native callback binding evidence was not supplied; no Cycle 18 event-program composition certificate is issued".into()
        } else if !missing_paths.is_empty() {
            format!(
                "{} of {} selected source paths lack checked native evidence; no Cycle 18 event-program composition certificate is issued",
                missing_paths.len(),
                source
                    .selected_paths
                    .expect("validated source path inventory")
            )
        } else {
            "all selected source paths have checked native execution evidence, but no source-to-Cycle18 EventProgram projection or composed certificate exists".into()
        };
        callbacks.push(CallbackCoverage {
            function: function.clone(),
            source_revision: source.source_revision.clone(),
            selected_paths: source.selected_paths,
            checked_paths,
            missing_paths,
            checked_executions,
            cycle18_composition: Cycle18CallbackStatus::Unknown { reason },
        });
    }
    let selected_paths = callbacks.iter().filter_map(|row| row.selected_paths).sum();
    let checked_paths = callbacks.iter().map(|row| row.checked_paths.len()).sum();
    Ok(CallbackCoverageReport {
        source_profile_assumptions,
        selected_callbacks: callbacks.len(),
        accounted_callbacks: callbacks.len(),
        selected_paths,
        checked_paths,
        callbacks,
    })
}
