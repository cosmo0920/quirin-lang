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

use qcore_schemas::plugin::{PluginManifest, VerificationRequestV2, VerificationResponseV2};
use quirin_provider_shared::{reactor_callbacks, runtime};

/// The crate's contract-inheritance namespace (card
/// CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, the user's option B): the package
/// name without `qcore-provider-`, `-` read as `_`. A parent this crate owns
/// is written `libevent::NAME`.
/// It owns Monkey's `monkey/...` modules, so a parent such as `mk_mem_alloc` is
/// written `libevent::mk_mem_alloc`: a namespace names the owning crate, never
/// a function-name prefix.
/// Recorded here in the one spelling `providers/contracts.py` reads,
/// checked against the derivation rule by this crate's tests, and carried
/// into `providers/contract-load-path.json`, where the load path checks it
/// again. The executable never reads it: it is data for the load path.
#[allow(dead_code)]
pub const NAMESPACE: &str = "libevent";

/// The owned modules another contract module may `#include`, sorted: a
/// subset of the logical paths under `contracts/`. None; publishing a
/// module is a deliberate, reviewed act.
#[allow(dead_code)]
pub const PUBLISHED: &[&str] = &[];

/// Each capability inspects the request itself and declines what it does not
/// implement; a declined request falls through to the runtime's Unknown reply.
fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    reactor_callbacks::propose(request, manifest)
}

fn main() {
    if let Err(error) =
        // The markers make the compiled-in template one contiguous, locatable
        // blob in the executable, so a host can bind an installed manifest's
        // declarations to it without executing this program.
        runtime::run_with_v2_and_contracts(
            concat!(
                "<<<qcore.provider-template.v1:begin>>>\n",
                include_str!("../provider-template.json"),
                "<<<qcore.provider-template.v1:end>>>"
            ),
            Some(concat!(
                "<<<qcore.provider-contracts.v1:begin>>>\n",
                include_str!("../contracts.json"),
                "<<<qcore.provider-contracts.v1:end>>>"
            )),
            Some(propose),
        )
    {
        eprintln!("provider: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every owned module under `contracts/`, as `(logical path, text)`.
    fn contract_tree() -> Vec<(String, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
        let mut on_disk = Vec::new();
        // Card W45-TEST-HYGIENE: the tracked contract tree only; an
        // untracked or ignored file under `contracts/` is not a module.
        for path in super::tracked_files::tracked_files(&root) {
            assert!(!path.is_symlink(), "symbolic link: {}", path.display());
            let name = path.strip_prefix(&root).unwrap().to_str().unwrap();
            on_disk.push((
                name.replace('\\', "/"),
                std::fs::read_to_string(&path).unwrap(),
            ));
        }
        on_disk.sort();
        on_disk
    }

    /// Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: the recorded namespace is
    /// the one the load path derives from the package name, and every
    /// published module is an owned one, listed once, in order.
    #[test]
    fn namespace_is_derived_and_published_modules_are_owned() {
        let derived = env!("CARGO_PKG_NAME")
            .strip_prefix("qcore-provider-")
            .expect("an executable contract crate is named qcore-provider-NAME")
            .replace('-', "_");
        assert_eq!(NAMESPACE, derived);
        assert!(PUBLISHED.windows(2).all(|pair| pair[0] < pair[1]));
        let owned = contract_tree();
        for module in PUBLISHED {
            assert!(owned.iter().any(|(name, _)| name == module), "{module}");
        }
    }

    /// Card **SP4-SYMBOLIC-LIBEVENT-PREMISES** (contract specification
    /// C04.18.9): `requires resource_premise(NAME, "STATEMENT")`, a premise
    /// stated in prose, as OpenSSL's checker reads one (`prose_premise`).
    fn prose_premise(clause: &str) -> bool {
        clause
            .strip_prefix("requires resource_premise(")
            .and_then(|rest| rest.split_once(','))
            .is_some_and(|(_, statement)| statement.trim_start().starts_with('"'))
    }

    /// Card **SP4-SYMBOLIC-LIBEVENT-PREMISES**: a premise of this crate is a
    /// formula (C04.18.9), never a sentence. This crate has no checker
    /// plugin, so its contract-tree test is where a prose premise is refused:
    /// every `resource_premise` clause of every owned module, the seven of
    /// `mk_http_thread_destroy`, states a formula.
    #[test]
    fn every_premise_is_a_formula() {
        assert!(prose_premise(
            "requires resource_premise(scheduler_callee_live, \"anything\")"
        ));
        assert!(!prose_premise(
            "requires resource_premise(scheduler_callee_live, quant all #m in #args: #m in #live)"
        ));
        assert!(!prose_premise(
            "requires resource_premise_decomposition(scheduler_callee_live, a, b)"
        ));
        let mut premises = Vec::new();
        for (name, text) in contract_tree() {
            let code = text
                .lines()
                .map(|line| line.split("//").next().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(" ");
            for clause in code.split(';') {
                let clause = clause.split_whitespace().collect::<Vec<_>>().join(" ");
                if clause.starts_with("requires resource_premise(") {
                    assert!(
                        !prose_premise(&clause),
                        "{name}: `{clause}` states its premise in prose; a premise of this \
                         crate is a formula (C04.18.9)"
                    );
                    premises.push((name.clone(), clause));
                }
            }
        }
        assert_eq!(premises.len(), 7, "{premises:?}");
        assert!(
            premises
                .iter()
                .all(|(name, _)| name == "monkey/intrusive_list.qv")
        );
    }

    /// Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: every `#include` this
    /// crate's contracts write names a module the libc crate publishes (the
    /// load path would refuse anything else with `QI004`/`QI005`), through the
    /// declared Cargo edge this test links, so the edge is load-bearing and
    /// not decorative.
    #[test]
    fn every_include_names_a_published_libc_module() {
        let mut includes = 0;
        for (name, text) in contract_tree() {
            for line in text.lines() {
                let line = line.split("//").next().unwrap().trim();
                if !line.starts_with('#') {
                    continue;
                }
                let target = line
                    .strip_prefix("#include \"")
                    .and_then(|rest| rest.strip_suffix("\";"))
                    .unwrap_or_else(|| panic!("{name}: malformed directive `{line}`"));
                let (krate, module) = target.split_once(':').unwrap();
                assert_eq!(krate, "quirin-provider-libc", "{name}: {target}");
                assert!(
                    quirin_provider_libc::PUBLISHED.contains(&module),
                    "{name}: {target} is not published"
                );
                assert!(quirin_provider_libc::contract(module).is_some());
                includes += 1;
            }
        }
        assert!(includes > 0, "no contract includes a libc module");
    }
}
