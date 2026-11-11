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

//! Source-bound finite scheduler cleanup contracts, never C safety authority.
//!
//! The trace describes one suspended HTTP coroutine. Borrowed session/event
//! objects survive; the parent owns the embedded HTTP object and coroutine.
//! Actual callback effects, faithful extraction, TLS success, no concurrent
//! writers and no later use of stale TLS/request aliases remain prerequisites.
use qcore_schemas::{hypothesis::Context, sha256_hex};
use std::collections::{BTreeMap, BTreeSet};

pub const SOURCE_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";
pub const SOURCE_FILES: &[(&str, &str)] = &[
    (
        "lib/monkey/mk_server/mk_scheduler.c",
        "9a515fa6d0cc79ee8e70457e5e26e65a9ab82c8febd0bb383c88d35bcc1aedda",
    ),
    (
        "lib/monkey/mk_server/mk_http_thread.c",
        "29fa43236d38b00d8609906f535386d93c600685744cefc1914a2a7ccff3d745",
    ),
    (
        "lib/monkey/include/monkey/mk_thread_libco.h",
        "4625fde94575742200f6ffa6eb4a8e30b12ee0079544cebd4f5de1d22f929fd3",
    ),
    (
        "lib/monkey/include/monkey/mk_http_thread.h",
        "0b33d6ba9cf1490d2a8ec481f08cd9032745c8e0879a2ef6fdb16a4f81b1b4f7",
    ),
    (
        "lib/monkey/include/monkey/mk_scheduler.h",
        "0f4e6c9f133eee464e58b5d02e671746adf982ecf1e4a5708ce46f847b192f0d",
    ),
    (
        "lib/monkey/include/monkey/mk_tls.h",
        "7b99725e937493d38d988ef5ad50944c505ce6c6aeab2eff6e73942191ed3096",
    ),
    (
        "lib/monkey/include/monkey/mk_core/mk_list.h",
        "2deacdc5bb1abfc317be4b46687d569be36aad32aa1816c127e4fe6d5f9e1af1",
    ),
];
pub const MAX_THREADS: usize = 32;
pub const MAX_CALLBACK_EFFECTS: usize = 64;

/// Construction authenticates bytes, not the execution of these C bodies.
#[derive(Debug)]
pub struct SourceBinding {
    digest: String,
}
impl SourceBinding {
    pub fn verify(sources: &BTreeMap<String, Vec<u8>>) -> Result<Self, String> {
        if sources.len() != SOURCE_FILES.len() {
            return Err("scheduler: source inventory".into());
        }
        for (path, digest) in SOURCE_FILES {
            if sources.get(*path).map(|b| sha256_hex(b)).as_deref() != Some(*digest) {
                return Err(format!("scheduler: source mismatch {path}"));
            }
        }
        Ok(Self {
            digest: sha256_hex(
                &serde_json::to_vec(&(
                    "monkey-scheduler-lifetime-model-v1",
                    SOURCE_REVISION,
                    SOURCE_FILES,
                    include_str!(
                        "../../../docs/reports/scheduler-lifetime-contracts/scheduler.qgd"
                    ),
                    include_str!(
                        "../../../docs/reports/scheduler-lifetime-contracts/hypotheses.json"
                    ),
                    include_str!("scheduler_lifetime.rs"),
                    include_str!("../../../proof/QCore/SchedulerLifetime.lean"),
                ))
                .map_err(|e| e.to_string())?,
            ),
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackEffect {
    Read(u32),
    Write(u32),
    Release(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    pub parent: u32,
    pub coroutine: u32,
    pub suspended: bool,
    /// IDs of borrowed session, channel and event; aliases are permitted.
    pub borrowed: [u32; 3],
    /// None is positively absent; Some(empty) is a returning callback with no
    /// modeled resource effects. Neither is inferred from missing evidence.
    pub callback: Option<Vec<CallbackEffect>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub worker: u32,
    pub current_worker: u32,
    /// None, or a live parent token in `threads`; callee/params/borrowed tokens
    /// are invalid roles even when the selected list is empty.
    pub current_thread: Option<u32>,
    pub tls_params: Option<u32>,
    /// Ordered membership of the selected active OR purge list.
    pub selected: Vec<u32>,
    /// The other list survives; disjointness is checked, not assumed.
    pub other: Vec<u32>,
    pub threads: BTreeMap<u32, Thread>,
    pub live: BTreeSet<u32>,
    pub callback_owned: BTreeSet<u32>,
    pub event_connection: BTreeSet<u32>,
}

/// A requested mathematical trace, not a claim that extraction produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    FreeTls(u32),
    ClearTls,
    Unlink(u32),
    ResetEvent(u32),
    Callback(u32, Vec<CallbackEffect>),
    DeleteCoroutine(u32),
    FreeParent(u32),
}

/// Checked finite transition result. This type has no verdict/standing export.
#[derive(Debug)]
pub struct CheckedModel {
    after: Model,
    count: usize,
}
impl CheckedModel {
    pub fn after(&self) -> &Model {
        &self.after
    }
    pub fn count(&self) -> usize {
        self.count
    }
}

fn refuse(message: &str) -> String {
    format!("scheduler: {message}")
}

/// All-or-refuse at the model boundary. Refusal is not a C failure branch:
/// mk_http_thread_destroy returns zero after effects and has no rollback.
pub fn check_model(before: &Model, trace: &[Step]) -> Result<CheckedModel, String> {
    if before.worker != before.current_worker {
        return Err(refuse("wrong worker TLS"));
    }
    if before.threads.len() > MAX_THREADS || before.live.len() > 512 || trace.len() > 2560 {
        return Err(refuse("finite bound"));
    }
    let selected: BTreeSet<_> = before.selected.iter().copied().collect();
    let other: BTreeSet<_> = before.other.iter().copied().collect();
    if selected.len() != before.selected.len()
        || other.len() != before.other.len()
        || !selected.is_disjoint(&other)
        || selected.union(&other).copied().collect::<BTreeSet<_>>()
            != before.threads.keys().copied().collect()
    {
        return Err(refuse("invalid membership"));
    }
    let mut owned = BTreeSet::new();
    for (&id, thread) in &before.threads {
        if id != thread.parent
            || !thread.suspended
            || !owned.insert(thread.parent)
            || !owned.insert(thread.coroutine)
            || !before.live.contains(&thread.parent)
            || !before.live.contains(&thread.coroutine)
            || thread.borrowed.iter().any(|b| !before.live.contains(b))
            || thread
                .callback
                .as_ref()
                .is_some_and(|e| e.len() > MAX_CALLBACK_EFFECTS)
        {
            return Err(refuse(
                "parent ownership, borrowed lifetime or callback bound",
            ));
        }
    }
    if !before.callback_owned.is_subset(&before.live)
        || !before.callback_owned.is_disjoint(&owned)
        || before
            .threads
            .values()
            .flat_map(|t| t.borrowed)
            .any(|p| before.callback_owned.contains(&p))
        || before
            .tls_params
            .is_some_and(|p| before.callback_owned.contains(&p))
        || !before.event_connection.is_subset(&before.live)
        || before
            .event_connection
            .iter()
            .any(|p| !before.threads.values().any(|t| t.borrowed[2] == *p))
        || before
            .current_thread
            .is_some_and(|p| before.callback_owned.contains(&p))
        || before
            .current_thread
            .is_some_and(|p| !before.live.contains(&p))
    {
        return Err(refuse("callback ownership or stale current TLS"));
    }
    if before
        .threads
        .values()
        .flat_map(|t| t.borrowed)
        .any(|b| owned.contains(&b))
        || before.tls_params.is_some_and(|p| {
            owned.contains(&p)
                || !before.live.contains(&p)
                || before.threads.values().any(|t| t.borrowed.contains(&p))
        })
    {
        return Err(refuse("TLS/owned/borrowed separation"));
    }
    if before
        .current_thread
        .is_some_and(|p| !before.threads.contains_key(&p))
    {
        return Err(refuse("current-thread TLS role must be a modeled parent"));
    }
    // mk_thread_destroy does not clear mk_thread TLS. Never manufacture that
    // clear: reject a retiring current-thread alias in this bounded profile.
    if before.current_thread.is_some_and(|p| selected.contains(&p)) {
        return Err(refuse(
            "current-thread alias requires external retirement protocol",
        ));
    }
    let mut after = before.clone();
    let mut expected = Vec::new();
    // Every callback is checked against the complete iteration footprint,
    // including the saved successor and the other list, not only its own item.
    let protected: BTreeSet<_> = owned
        .iter()
        .copied()
        .chain(before.threads.values().flat_map(|t| t.borrowed))
        .chain(before.tls_params)
        .collect();
    for id in &before.selected {
        let thread = &before.threads[id];
        if let Some(p) = after.tls_params.take() {
            expected.extend([Step::FreeTls(p), Step::ClearTls]);
            after.live.remove(&p);
        }
        expected.push(Step::Unlink(*id));
        after.selected.retain(|p| p != id);
        expected.push(Step::ResetEvent(thread.borrowed[2]));
        after.event_connection.insert(thread.borrowed[2]);
        if let Some(effects) = &thread.callback {
            expected.push(Step::Callback(*id, effects.clone()));
            for effect in effects {
                let resource = match effect {
                    CallbackEffect::Read(p)
                    | CallbackEffect::Write(p)
                    | CallbackEffect::Release(p) => *p,
                };
                if !after.live.contains(&resource) {
                    return Err(refuse("callback use after release"));
                }
                if !matches!(effect, CallbackEffect::Read(_))
                    && (protected.contains(&resource) || !after.callback_owned.contains(&resource))
                {
                    return Err(refuse(
                        "callback invalidates protected lifetime/iteration footprint",
                    ));
                }
                if matches!(effect, CallbackEffect::Release(_)) {
                    after.live.remove(&resource);
                    after.callback_owned.remove(&resource);
                }
            }
        }
        expected.extend([
            Step::DeleteCoroutine(thread.coroutine),
            Step::FreeParent(*id),
        ]);
        after.live.remove(&thread.coroutine);
        after.live.remove(id);
        after.threads.remove(id);
    }
    if trace != expected {
        return Err(refuse("trace order, branch, effect or terminal mismatch"));
    }
    Ok(CheckedModel {
        after,
        count: before.selected.len(),
    })
}

/// Adds exact source and all nine caller-held context coordinates to model
/// replay. This cannot authenticate the presented trace as an actual C run.
pub fn check_source_bound_model(
    source: &SourceBinding,
    expected: &Context,
    presented: &Context,
    before: &Model,
    trace: &[Step],
) -> Result<CheckedModel, String> {
    if expected != presented
        || expected.source_identity != source.digest
        || [
            &expected.artifact,
            &expected.property,
            &expected.semantic_profile,
            &expected.scope,
            &expected.bounds,
            &expected.assumptions,
            &expected.build_identity,
            &expected.evidence_version,
        ]
        .iter()
        .any(|s| s.is_empty())
    {
        return Err(refuse("exact context mismatch"));
    }
    check_model(before, trace)
}
