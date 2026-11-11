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

//! Runtime-only list callback candidate contracts (RK-PLUGIN-INTERCEPTOR-CANDIDATES).
//!
//! An inventory is a trusted, reviewable restriction, bound into the loader's
//! outer statement by its exact digest and to reviewed source bytes. It never
//! enters the static callback registry. In particular a list is not a function
//! pointer, and union arms are kept distinct by their list identity here.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

use qcore_contract::load_path_manifest::LoadPathManifest;
use qcore_contract::outer_contract::{EnvelopeLine, MemberEnvelope, OuterContracts};
use qcore_schemas::sha256_hex;
use serde::Deserialize;
use serde_json::{Value, json};

/// Optional inventory beside a provider's outer lock.
pub const FILE: &str = "outer-candidates.json";
const MARKER: &str = "outer-candidates:";
const CUSTODY: &str = "all-pointer-arguments-borrowed-no-retention-or-global-custody-effects";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    reviewed_revision: String,
    sources: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    list_member: String,
    aliases: Vec<String>,
    element_record: String,
    callback_member: String,
    setter: String,
    callback_signature: String,
    successful_result: i32,
    lifecycle: String,
    duplicates: String,
    custody: String,
    review: String,
    registration_lifetime: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema_version: u32,
    kind: String,
    normalisation: String,
    static_invocation_support: bool,
    reviewed_identities: Vec<Identity>,
    candidates: Vec<Candidate>,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(|c: char| c.is_ascii_digit())
        && value.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn member(value: &str) -> bool {
    value.contains('.') && value.split('.').all(identifier)
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn relative(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn parse(text: &str) -> Result<Inventory, String> {
    let inventory: Inventory = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if inventory.schema_version != 1
        || inventory.kind != "outer-list-callback-candidates"
        || inventory.normalisation != "exact-source-bytes-sha256"
        || inventory.static_invocation_support
        || inventory.reviewed_identities.is_empty()
        || inventory.candidates.is_empty()
    {
        return Err("unsupported outer candidate inventory header".to_owned());
    }
    let mut keys = BTreeSet::new();
    for candidate in &inventory.candidates {
        if !member(&candidate.list_member)
            || !identifier(&candidate.element_record)
            || !member(&candidate.callback_member)
            || !identifier(&candidate.setter)
            || candidate.callback_signature.is_empty()
            || candidate.successful_result != 0
            || !["configuration", "on-new-before-initialized-non-share"]
                .contains(&candidate.lifecycle.as_str())
            || !["allowed", "conflict-without-append"].contains(&candidate.duplicates.as_str())
            || candidate.custody != CUSTODY
            || candidate.registration_lifetime
                != "callback-code-and-opaque-valid-through-last-invocation"
            || candidate.review.trim().is_empty()
        {
            return Err(format!(
                "unsupported list candidate `{}`",
                candidate.list_member
            ));
        }
        for key in std::iter::once(&candidate.list_member).chain(&candidate.aliases) {
            if !member(key) || !keys.insert(key) {
                return Err(format!(
                    "invalid or duplicate candidate list identity `{key}`"
                ));
            }
        }
    }
    for identity in &inventory.reviewed_identities {
        if identity.reviewed_revision.len() != 40
            || !identity
                .reviewed_revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || identity.sources.is_empty()
            || identity
                .sources
                .iter()
                .any(|(p, d)| !relative(p) || !digest(d))
        {
            return Err("invalid candidate reviewed source identity".to_owned());
        }
    }
    Ok(inventory)
}

fn source_bytes(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    if root.join(".git").exists() {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show", &format!("HEAD:{path}")])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!("cannot read candidate source `{path}` from HEAD"));
        }
        Ok(output.stdout)
    } else {
        fs::read(root.join(path)).map_err(|e| e.to_string())
    }
}

fn repository_text_digest(bytes: &[u8]) -> String {
    // Git's source identity is the LF blob. A Windows checkout may translate
    // only those line endings to CRLF; do not let that platform presentation
    // change an inventory digest cited by a contract.
    let normalized = bytes
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect::<Vec<_>>();
    sha256_hex(&normalized.join(&b'\n'))
}

fn sources_match(inventory: &Inventory, root: &Path) -> bool {
    let mut observed = BTreeMap::new();
    inventory.reviewed_identities.iter().any(|identity| {
        identity.sources.iter().all(|(path, expected)| {
            observed
                .entry(path.clone())
                .or_insert_with(|| source_bytes(root, path).map(|b| sha256_hex(&b)))
                .as_ref()
                == Ok(expected)
        })
    })
}

/// Validate candidate inventories and hold every citing outer loader when its
/// inventory's source pin is not in force. A missing, ambiguous, malformed or
/// unbound inventory refuses selection; no static registry is modified.
///
/// # Errors
/// Invalid or missing inventory, duplicate keys, missing exact digest citation.
pub fn apply(
    contracts: &mut OuterContracts,
    manifest: &LoadPathManifest,
    root: &Path,
) -> Result<Value, String> {
    let mut records = Vec::new();
    let mut inventories = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for (owner, contract_root) in &manifest.contract_roots {
        let Some(parent) = contract_root.parent() else {
            continue;
        };
        let path = parent.join(FILE);
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        let inventory = parse(text)?;
        let hash = repository_text_digest(&bytes);
        let matches = sources_match(&inventory, root);
        for candidate in &inventory.candidates {
            for key in std::iter::once(&candidate.list_member).chain(&candidate.aliases) {
                if owners.insert(key.clone(), hash.clone()).is_some() {
                    return Err(format!("ambiguous candidate list `{key}`"));
                }
            }
        }
        records.push(json!({"owner":owner,"path":path,"sha256":hash,"in_force":matches,
            "static_invocation_support":false,"contract":serde_json::from_str::<Value>(text).map_err(|e|e.to_string())?}));
        inventories.insert(hash, matches);
    }
    let mut holds = validate_envelopes(contracts, &owners, &inventories)?;
    holds.sort();
    holds.dedup();
    for name in &holds {
        contracts.hold_for_pin_mismatch(
            name,
            "candidate-source-mismatch",
            "the list candidate inventory does not match reviewed source bytes",
        )?;
    }
    Ok(
        json!({"inventories":records,"held_loaders":holds,"meaning":"runtime review restrictions only; static list invocation and static closure remain unsupported"}),
    )
}

fn validate_envelopes(
    contracts: &OuterContracts,
    owners: &BTreeMap<String, String>,
    inventories: &BTreeMap<String, bool>,
) -> Result<Vec<String>, String> {
    let mut holds = Vec::new();
    for (name, loader) in &contracts.loaders {
        let Some(statement) = &loader.statement else {
            continue;
        };
        let references = statement
            .split(MARKER)
            .skip(1)
            .map(|tail| {
                let end = tail
                    .find(|c: char| !c.is_ascii_hexdigit())
                    .unwrap_or(tail.len());
                &tail[..end]
            })
            .collect::<BTreeSet<_>>();
        for hash in &references {
            let Some(in_force) = inventories.get(*hash) else {
                return Err(format!(
                    "outer `{name}` cites missing candidate inventory `{hash}`"
                ));
            };
            if !in_force {
                holds.push(name.clone());
            }
        }
        for line in &loader.envelope {
            if let EnvelopeLine::Member {
                envelope: MemberEnvelope::Registered,
                ..
            } = line
                && let Some(hash) = owners.get(&line.key())
                && !references.contains(hash.as_str())
            {
                return Err(format!(
                    "outer `{name}` has an unbound list candidate `{}`",
                    line.key()
                ));
            }
        }
    }
    Ok(holds)
}

#[cfg(test)]
mod tests {
    use super::*;
    const INVENTORY: &str =
        include_str!("../../../providers/qcore-provider-librdkafka/outer-candidates.json");
    #[test]
    fn inventory_keeps_fourteen_lists_and_union_arms_distinct() {
        let inventory = parse(INVENTORY).unwrap();
        assert_eq!(inventory.candidates.len(), 14);
        let arms = inventory
            .candidates
            .iter()
            .map(|c| &c.callback_member)
            .collect::<BTreeSet<_>>();
        assert_eq!(arms.len(), 14);
        assert_eq!(inventory.reviewed_identities.len(), 2);
        assert!(!inventory.static_invocation_support);
    }
    #[test]
    fn unsupported_modes_and_ambiguous_identities_refuse() {
        for (old, new) in [
            (CUSTODY, "owned"),
            (
                "\"static_invocation_support\": false",
                "\"static_invocation_support\": true",
            ),
            ("\"successful_result\": 0", "\"successful_result\": 1"),
            ("exact-source-bytes-sha256", "unverified"),
            ("configuration", "any-time"),
            (
                "callback-code-and-opaque-valid-through-last-invocation",
                "unchecked-lifetime",
            ),
        ] {
            assert!(parse(&INVENTORY.replace(old, new)).is_err(), "{old}");
        }
        let mut json: Value = serde_json::from_str(INVENTORY).unwrap();
        json["candidates"][1]["list_member"] = json["candidates"][0]["list_member"].clone();
        assert!(parse(&json.to_string()).is_err());
    }
    #[test]
    fn missing_unbound_or_stale_candidate_never_covers_a_runtime_fact() {
        use qcore_contract::outer_contract::load_outer_contract_sources;
        let hash = "a".repeat(64);
        let text = format!(
            r#"module contract;
extern fn load(config: ptr<mut, u8>) -> unit
contract {{
    requires outer_contract_v1;
    quant #outer [[contract]] plugin, "neutral callbacks outer-candidates:{hash}";
    quant #outer where #config in #loaded [[runtime]];
    quant #outer #loaded: conf.list |-> #registered;
}};
"#
        );
        let contracts =
            load_outer_contract_sources(&BTreeMap::from([("test:outer.qv".to_owned(), text)]))
                .unwrap();
        let owners = BTreeMap::from([("conf.list".to_owned(), hash.clone())]);
        assert!(
            validate_envelopes(&contracts, &owners, &BTreeMap::new())
                .unwrap_err()
                .contains("missing")
        );
        assert_eq!(
            validate_envelopes(
                &contracts,
                &owners,
                &BTreeMap::from([(hash.clone(), false)])
            )
            .unwrap(),
            vec!["load"]
        );
        assert!(
            validate_envelopes(&contracts, &owners, &BTreeMap::from([(hash.clone(), true)]))
                .unwrap()
                .is_empty()
        );
        let mut unbound = contracts;
        unbound.loaders.get_mut("load").unwrap().statement = Some("neutral callbacks".into());
        assert!(
            validate_envelopes(&unbound, &owners, &BTreeMap::from([(hash, true)]))
                .unwrap_err()
                .contains("unbound")
        );
    }
    #[test]
    fn source_pins_use_exact_reviewed_bytes_and_reject_path_traversal() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qcore-candidate-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("source.c"), b"reviewed bytes").unwrap();
        let mut inventory = parse(INVENTORY).unwrap();
        inventory.reviewed_identities.truncate(1);
        inventory.reviewed_identities[0].sources =
            BTreeMap::from([("source.c".into(), sha256_hex(b"reviewed bytes"))]);
        assert!(sources_match(&inventory, &root));
        fs::write(root.join("source.c"), b"changed bytes").unwrap();
        assert!(!sources_match(&inventory, &root));
        fs::remove_file(root.join("source.c")).unwrap();
        assert!(!sources_match(&inventory, &root));
        fs::remove_dir(root).unwrap();
        assert!(!relative("../source.c"));
        assert!(!relative("/source.c"));
    }
    #[test]
    fn inventory_digest_accepts_only_checkout_line_ending_translation() {
        assert_eq!(
            repository_text_digest(b"first\nsecond\n"),
            repository_text_digest(b"first\r\nsecond\r\n")
        );
        assert_ne!(
            repository_text_digest(b"first\nsecond\n"),
            repository_text_digest(b"first\rsecond\n")
        );
    }
    #[test]
    fn the_paired_envelopes_hold_when_candidate_source_is_missing() {
        use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest =
            load_contract_load_path_manifest(&repo.join("providers/contract-load-path.json"))
                .unwrap();
        let (mut contracts, _) = crate::outer_selection::load_views(&manifest).unwrap();
        let missing =
            std::env::temp_dir().join(format!("qcore-no-candidate-source-{}", std::process::id()));
        let record = apply(&mut contracts, &manifest, &missing).unwrap();
        assert_eq!(
            record["held_loaders"],
            json!(["flb_plugin_load", "rd_kafka_plugin_new"])
        );
        assert!(!contracts.in_force("rk_plugin_library"));
        assert!(!contracts.in_force("flb_external_plugin"));
        assert_eq!(record["inventories"][0]["static_invocation_support"], false);
    }
}
