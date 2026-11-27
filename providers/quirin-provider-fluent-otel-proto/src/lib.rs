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

//! Shared Quirin contract modules for the generated fluent-otel-proto
//! functions and the protobuf-c runtime entry points they reach (card
//! **OTEL-PROTO-PHASE-B1**; USER DECISIONS 2026-11-27, "New provider crate",
//! "Phase it, accept 7").
//!
//! `fluent_otel_proto/generated.qv` declares the 78 generated
//! `opentelemetry__proto__*` functions the OpenTelemetry consumer units call;
//! `fluent_otel_proto/runtime.qv` the four protobuf-c entry points. Both are
//! written by `scripts/generate.py` from the vendored tree, which refuses a
//! function whose prototype or body is not the protoc-c 1.5 shape of its
//! family. Every declaration is pinned (`contracts.lock`) to its generated
//! definition and prototype: a regenerated file holds the declarations whose
//! sites changed (`contract-pin-mismatch`, Unknown).
//!
//! [`IDENTITY`] is the crate's protocol identity on the vendored basis
//! (opentelemetry-proto v1.10.0): the pinned opentelemetry-proto commit and
//! `.proto` digests, and the digest of every compiled `.pb-c.c`/`.pb-c.h`
//! and of `protobuf-c.c`/`protobuf-c.h`. [`check_identity`] lists every
//! difference between it and an observed identity; `scripts/generate.py
//! --check` refuses a vendored tree or protocol clone that differs. The crate
//! is data only and carries no checker: the shipped load path carries no
//! checker bindings (`providers/contracts.py`, user decision 2026-11-16).

/// The crate's contract-inheritance namespace (card CONTRACT-INHERIT-ADOPTION):
/// the package name without `quirin-provider-`, `-` read as `_`.
pub const NAMESPACE: &str = "fluent_otel_proto";

/// The owned modules another contract module may `#include`, sorted. None:
/// publishing a module is a deliberate, reviewed act.
pub const PUBLISHED: &[&str] = &[];

/// Every contract module this crate owns, as `(logical path, source text)`,
/// sorted by logical path.
pub const CONTRACTS: &[(&str, &str)] = &[
    (
        "fluent_otel_proto/generated.qv",
        include_str!("../contracts/fluent_otel_proto/generated.qv"),
    ),
    (
        "fluent_otel_proto/runtime.qv",
        include_str!("../contracts/fluent_otel_proto/runtime.qv"),
    ),
];

/// The protocol identity the declarations rest on (`protocol-identity.json`).
pub const IDENTITY: &str = include_str!("../protocol-identity.json");

/// The source text of one owned module, by logical path.
pub fn contract(path: &str) -> Option<&'static str> {
    CONTRACTS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| *text)
}

/// One digest map of an identity: the `.proto` files or the vendored files.
pub type Digests<'a> = &'a [(&'a str, &'a str)];

/// Every difference between the pinned identity (`commit`, `.proto`
/// digests, vendored digests) and an observed one; empty only when they are
/// equal. A use of the declarations under a non-empty answer is refused
/// (`otp_identity_refused_unless_pinned`).
pub fn check_identity(
    pinned: (&str, Digests, Digests),
    observed: (&str, Digests, Digests),
) -> Vec<String> {
    let mut problems = Vec::new();
    if pinned.0 != observed.0 {
        problems.push(format!(
            "protocol commit {} != pinned {}",
            observed.0, pinned.0
        ));
    }
    for (kind, p, o) in [("proto", pinned.1, observed.1), ("vendored", pinned.2, observed.2)] {
        if p.is_empty() {
            problems.push(format!("no pinned {kind} digest"));
        }
        for (name, digest) in p {
            match o.iter().find(|(n, _)| n == name) {
                None => problems.push(format!("{kind} {name}: pinned, absent")),
                Some((_, d)) if d != digest => {
                    problems.push(format!("{kind} {name}: sha256 {d} != pinned {digest}"))
                }
                Some(_) => {}
            }
        }
        for (name, _) in o {
            if !p.iter().any(|(n, _)| n == name) {
                problems.push(format!("{kind} {name}: present, not pinned"));
            }
        }
    }
    problems
}

#[cfg(test)]
#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn identity() -> Value {
        serde_json::from_str(IDENTITY).expect("protocol-identity.json parses")
    }

    fn digests(v: &Value) -> Vec<(String, String)> {
        v.as_object()
            .unwrap()
            .iter()
            .map(|(k, d)| (k.clone(), d.as_str().unwrap().to_owned()))
            .collect()
    }

    fn view(v: &[(String, String)]) -> Vec<(&str, &str)> {
        v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
    }

    #[test]
    fn namespace_is_derived_and_published_modules_are_owned() {
        let derived = env!("CARGO_PKG_NAME")
            .strip_prefix("quirin-provider-")
            .expect("a contract crate is named quirin-provider-NAME")
            .replace('-', "_");
        assert_eq!(NAMESPACE, derived);
        assert!(PUBLISHED.is_empty());
    }

    #[test]
    fn index_is_exactly_the_contract_tree() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
        let mut on_disk = Vec::new();
        for path in super::tracked_files::tracked_files(&root) {
            assert!(!path.is_symlink(), "symbolic link: {}", path.display());
            let name = path.strip_prefix(&root).unwrap().to_str().unwrap();
            on_disk.push((
                name.replace('\\', "/"),
                std::fs::read_to_string(&path).unwrap(),
            ));
        }
        on_disk.sort();
        let indexed: Vec<_> = CONTRACTS
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect();
        assert_eq!(indexed, on_disk);
        assert_eq!(contract("absent.qv"), None);
    }

    /// The identity is on the vendored basis, pins the eleven `.proto` files
    /// of the census and the 22 compiled generated and runtime files, excludes
    /// the stale `pprofextended` pair, and names exactly the declared
    /// functions in module order.
    #[test]
    fn the_identity_pins_the_protocol_and_every_compiled_file() {
        let id = identity();
        assert_eq!(id["schema"], "quirin-protocol-identity/1");
        assert!(id["basis"].as_str().unwrap().contains("v1.10.0"));
        assert_eq!(
            id["protocol"]["commit"],
            "3a6e438003e673112efd4a00e9836c00672eae3c"
        );
        assert_eq!(id["protocol"]["files"].as_object().unwrap().len(), 11);
        let vendored = id["vendored"].as_object().unwrap();
        assert_eq!(vendored.len(), 22);
        assert!(vendored.contains_key("protobuf-c/protobuf-c.c"));
        assert!(vendored.keys().all(|k| !k.contains("pprofextended")));
        let census: Value = serde_json::from_str(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../docs/reports/otel-proto-audit/census.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(id["protocol"]["commit"], census["protocol_identity"]["commit"]);
        assert_eq!(id["protocol"]["files"], census["protocol_identity"]["files"]);
        for (module, key) in [
            ("fluent_otel_proto/generated.qv", "generated"),
            ("fluent_otel_proto/runtime.qv", "runtime"),
        ] {
            let declared: Vec<&str> = contract(module)
                .unwrap()
                .lines()
                .filter_map(|l| l.strip_prefix("extern fn "))
                .map(|l| l.split('(').next().unwrap())
                .collect();
            let listed: Vec<&str> = id["declared"][key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(declared, listed, "{module}");
        }
    }

    /// A stale identity is refused: a changed commit, a changed, missing or
    /// extra digest each yields a problem; the pinned identity yields none.
    #[test]
    fn a_stale_identity_is_refused() {
        let id = identity();
        let commit = id["protocol"]["commit"].as_str().unwrap();
        let proto = digests(&id["protocol"]["files"]);
        let vendored = digests(&id["vendored"]);
        let pinned = (commit, &view(&proto)[..], &view(&vendored)[..]);
        assert!(check_identity(pinned, pinned).is_empty());
        assert!(!check_identity(pinned, ("0".repeat(40).as_str(), pinned.1, pinned.2)).is_empty());
        let mut changed = vendored.clone();
        changed[0].1 = "0".repeat(64);
        assert!(!check_identity(pinned, (commit, pinned.1, &view(&changed)[..])).is_empty());
        let fewer = &view(&proto)[1..];
        assert!(!check_identity(pinned, (commit, fewer, pinned.2)).is_empty());
        let mut more = view(&vendored);
        more.push(("opentelemetry/proto/x.pb-c.c", "00"));
        assert!(!check_identity(pinned, (commit, pinned.1, &more[..])).is_empty());
        assert!(!check_identity((commit, &[], &[]), pinned).is_empty());
    }

    /// libc stays in quirin-provider-libc: no declared name is a libc name.
    #[test]
    fn no_libc_name_is_declared() {
        let names = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../quirin-provider-libc/libc-names.txt"),
        )
        .unwrap();
        let libc: std::collections::BTreeSet<&str> = names
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .collect();
        assert!(libc.contains("malloc"));
        for (_, text) in CONTRACTS {
            for name in text
                .lines()
                .filter_map(|l| l.strip_prefix("extern fn "))
                .map(|l| l.split('(').next().unwrap())
            {
                assert!(!libc.contains(name), "{name} is a libc name");
            }
        }
    }
}
