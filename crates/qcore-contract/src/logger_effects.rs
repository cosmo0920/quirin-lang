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

//! Versioned proposed logger effect plans. No production verdict consumer.
//!
//! Exact source/model binding authenticates the reviewed inputs, not their C
//! refinement. Returned obligations remain Open. Cyclic call closure is a finite
//! reachability calculation and supplies no recursion termination argument.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub first: usize,
    pub last: usize,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub sha256: String,
    pub sites: Vec<Site>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub calls: Vec<String>,
    pub obligations: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    CreateOp,
    CreateText,
    Forward,
    Reply,
    Transfer,
    Take,
    Callback,
    DestroyText,
    DestroyOp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Terminal {
    Untouched,
    Queued,
    Destroyed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Path {
    pub events: Vec<Event>,
    pub terminal: Terminal,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub version: String,
    pub revision: String,
    pub profile: String,
    pub root: String,
    pub max_nodes: usize,
    pub max_events: usize,
    pub max_forwarding: usize,
    pub sources: BTreeMap<String, Source>,
    pub obligations: BTreeMap<String, String>,
    pub nodes: BTreeMap<String, Node>,
    pub paths: BTreeMap<String, Path>,
}

/// Opaque result: construction validates the complete independently expected
/// document and every alternative. No Deserialize or safety/truth conversion.
#[derive(Debug)]
pub struct CheckedPlan {
    digest: String,
    reachable: BTreeSet<String>,
    open_obligations: BTreeSet<String>,
    terminals: BTreeMap<String, Terminal>,
}
impl CheckedPlan {
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn reachable(&self) -> &BTreeSet<String> {
        &self.reachable
    }
    pub fn open_obligations(&self) -> &BTreeSet<String> {
        &self.open_obligations
    }
    pub fn terminals(&self) -> &BTreeMap<String, Terminal> {
        &self.terminals
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Custody {
    Absent,
    Local,
    Queued,
    Dead,
}

/// Exact bytes (including spelling/whitespace) are pinned by a caller-held
/// review digest. Never derive the expected digest from an untrusted document.
/// Sources are caller-supplied raw blobs, not paths opened by this checker.
pub fn check(
    document: &[u8],
    expected_digest: &str,
    source_blobs: &BTreeMap<String, Vec<u8>>,
) -> Result<CheckedPlan, String> {
    if document.len() > 131_072 || qcore_schemas::sha256_hex(document) != expected_digest {
        return Err("logger-effects: document digest or byte bound".into());
    }
    let p: Plan = serde_json::from_slice(document).map_err(|e| format!("logger-effects: {e}"))?;
    if p.version != "logger-effects-v1"
        || p.revision.len() != 40
        || !p.revision.bytes().all(|c| c.is_ascii_hexdigit())
        || p.profile.trim().is_empty()
        || p.max_nodes == 0
        || p.max_nodes > 64
        || p.max_events == 0
        || p.max_events > 64
        || p.max_forwarding == 0
        || p.max_forwarding > 16
        || p.nodes.len() > p.max_nodes
        || p.paths.is_empty()
        || p.paths.len() > 64
        || p.sources.is_empty()
        || p.sources.len() > 16
        || p.obligations.is_empty()
        || p.obligations.len() > 128
    {
        return Err("logger-effects: version, identity or model bounds".into());
    }
    if p.sources.keys().ne(source_blobs.keys()) {
        return Err("logger-effects: exact source domain".into());
    }
    for (path, pin) in &p.sources {
        if path.starts_with('/')
            || path
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == "..")
        {
            return Err("logger-effects: source spelling".into());
        }
        let bytes = &source_blobs[path];
        if bytes.len() > 2_097_152
            || qcore_schemas::sha256_hex(bytes) != pin.sha256
            || pin.sites.is_empty()
            || pin.sites.len() > 64
        {
            return Err("logger-effects: source digest or bounds".into());
        }
        let lines: Vec<&[u8]> = bytes.split_inclusive(|c| *c == b'\n').collect();
        for site in &pin.sites {
            if site.first == 0
                || site.last < site.first
                || site.last > lines.len()
                || qcore_schemas::sha256_hex(&lines[site.first - 1..site.last].concat())
                    != site.sha256
            {
                return Err("logger-effects: source site digest or range".into());
            }
        }
    }
    let id = |s: &str| {
        !s.is_empty() && s.len() <= 128 && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    };
    for (leaf, proposition) in &p.obligations {
        if !id(leaf)
            || proposition.trim().is_empty()
            || proposition.len() > 4096
            || proposition.chars().any(char::is_control)
        {
            return Err("logger-effects: obligation declaration".into());
        }
    }
    // Inspect every node, including unreachable ones, before computing closure.
    for (name, node) in &p.nodes {
        if !id(name)
            || node.calls.len() > 64
            || node.obligations.len() > 128
            || node.calls.iter().collect::<BTreeSet<_>>().len() != node.calls.len()
            || node.obligations.iter().collect::<BTreeSet<_>>().len() != node.obligations.len()
            || node.calls.iter().any(|c| !p.nodes.contains_key(c))
            || node
                .obligations
                .iter()
                .any(|o| !p.obligations.contains_key(o))
        {
            return Err("logger-effects: incomplete or duplicated node effects".into());
        }
    }
    if !p.nodes.contains_key(&p.root) {
        return Err("logger-effects: root missing".into());
    }
    let mut reachable = BTreeSet::new();
    let mut pending = vec![p.root.clone()];
    let mut open_obligations = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if reachable.insert(name.clone()) {
            let node = &p.nodes[&name];
            pending.extend(node.calls.iter().cloned());
            open_obligations.extend(node.obligations.iter().cloned());
        }
    }
    if reachable.len() != p.nodes.len() || open_obligations.len() != p.obligations.len() {
        return Err("logger-effects: unbound nodes or obligations".into());
    }
    let mut terminals = BTreeMap::new();
    for (name, path) in &p.paths {
        if !id(name) || path.events.len() > p.max_events {
            return Err("logger-effects: path name or event bound".into());
        }
        let (mut op, mut text) = (Custody::Absent, Custody::Absent);
        let mut forwards = 0;
        for event in &path.events {
            use Custody::*;
            use Event::*;
            match event {
                CreateOp if op == Absent && text == Absent => op = Local,
                CreateText if op == Local && text == Absent => text = Local,
                Forward | Reply if op == Local && text == Local => {
                    forwards += 1;
                    if forwards > p.max_forwarding {
                        return Err("logger-effects: recursion trace bound".into());
                    }
                }
                Transfer if op == Local && text == Local => {
                    op = Queued;
                    text = Queued;
                }
                Take if op == Queued && text == Queued => {
                    op = Local;
                    text = Local;
                }
                Callback if (op == Absent && text == Absent) || (op == Local && text == Local) => {}
                DestroyText if op == Local && text == Local => text = Dead,
                DestroyOp if op == Local && text == Dead => op = Dead,
                _ => {
                    return Err(format!(
                        "logger-effects: invalid custody transition {event:?}"
                    ));
                }
            }
        }
        let expected = match path.terminal {
            Terminal::Untouched => (Custody::Absent, Custody::Absent),
            Terminal::Queued => (Custody::Queued, Custody::Queued),
            Terminal::Destroyed => (Custody::Dead, Custody::Dead),
        };
        if (op, text) != expected {
            return Err("logger-effects: unproved terminal disposition".into());
        }
        terminals.insert(name.clone(), path.terminal);
    }
    Ok(CheckedPlan {
        digest: expected_digest.into(),
        reachable,
        open_obligations,
        terminals,
    })
}
