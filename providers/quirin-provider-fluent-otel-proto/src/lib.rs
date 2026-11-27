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
    for (kind, p, o) in [
        ("proto", pinned.1, observed.1),
        ("vendored", pinned.2, observed.2),
    ] {
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

    // The crate is data only (no serde_json, conformance pin): the two JSON
    // files are read by their fixed layout (`json.dumps(indent=1,
    // sort_keys=True)`), member by member, total-or-panic.

    /// The `"name": "value"` members of the object `"key": {` found after
    /// `from` in `text`, in order.
    fn object(text: &str, from: &str, key: &str) -> Vec<(String, String)> {
        let at = text.find(from).expect(from);
        let rest = &text[at..];
        let open = rest.find(&format!("\"{key}\": {{")).expect(key);
        let body = &rest[open..];
        let close = body.find('}').expect("closing brace");
        body[..close]
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let (name, value) = line.trim().trim_end_matches(',').split_once(": ").unwrap();
                (
                    name.trim_matches('"').to_owned(),
                    value.trim_matches('"').to_owned(),
                )
            })
            .collect()
    }

    /// The string items of the array `"key": [` found after `from`.
    fn array(text: &str, from: &str, key: &str) -> Vec<String> {
        let at = text.find(from).expect(from);
        let rest = &text[at..];
        let open = rest.find(&format!("\"{key}\": [")).expect(key);
        let body = &rest[open..];
        let close = body.find(']').expect("closing bracket");
        body[..close]
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.trim()
                    .trim_end_matches(',')
                    .trim_matches('"')
                    .to_owned()
            })
            .collect()
    }

    /// The string member `"key": "value"` found after `from`.
    fn scalar(text: &str, from: &str, key: &str) -> String {
        let at = text.find(from).expect(from);
        let rest = &text[at..];
        let open = rest.find(&format!("\"{key}\": \"")).expect(key) + key.len() + 5;
        let close = rest[open..].find('"').unwrap();
        rest[open..open + close].to_owned()
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
        let id = IDENTITY;
        assert!(id.contains("\"schema\": \"quirin-protocol-identity/1\""));
        assert!(scalar(id, "{", "basis").contains("v1.10.0"));
        let commit = scalar(id, "\"protocol\": {", "commit");
        assert_eq!(commit, "3a6e438003e673112efd4a00e9836c00672eae3c");
        let proto = object(id, "\"protocol\": {", "files");
        assert_eq!(proto.len(), 11);
        let vendored = object(id, "{", "vendored");
        assert_eq!(vendored.len(), 22);
        assert!(vendored.iter().any(|(k, _)| k == "protobuf-c/protobuf-c.c"));
        assert!(vendored.iter().all(|(k, _)| !k.contains("pprofextended")));
        let census = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/reports/otel-proto-audit/census.json"),
        )
        .unwrap();
        assert_eq!(
            scalar(&census, "\"protocol_identity\": {", "commit"),
            commit
        );
        assert_eq!(object(&census, "\"protocol_identity\": {", "files"), proto);
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
            assert_eq!(declared, array(id, "\"declared\": {", key), "{module}");
        }
    }

    /// A stale identity is refused: a changed commit, a changed, missing or
    /// extra digest each yields a problem; the pinned identity yields none.
    #[test]
    fn a_stale_identity_is_refused() {
        let commit = scalar(IDENTITY, "\"protocol\": {", "commit");
        let proto = object(IDENTITY, "\"protocol\": {", "files");
        let vendored = object(IDENTITY, "{", "vendored");
        let pinned = (commit.as_str(), &view(&proto)[..], &view(&vendored)[..]);
        assert!(check_identity(pinned, pinned).is_empty());
        let zero = "0".repeat(40);
        assert!(!check_identity(pinned, (zero.as_str(), pinned.1, pinned.2)).is_empty());
        let mut changed = vendored.clone();
        changed[0].1 = "0".repeat(64);
        assert!(!check_identity(pinned, (pinned.0, pinned.1, &view(&changed)[..])).is_empty());
        let fewer = &view(&proto)[1..];
        assert!(!check_identity(pinned, (pinned.0, fewer, pinned.2)).is_empty());
        let mut more = view(&vendored);
        more.push(("opentelemetry/proto/x.pb-c.c", "00"));
        assert!(!check_identity(pinned, (pinned.0, pinned.1, &more[..])).is_empty());
        assert!(!check_identity((pinned.0, &[], &[]), pinned).is_empty());
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
