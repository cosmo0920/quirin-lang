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

//! Bind an installed manifest's non-digest declarations to the manifest
//! template embedded in the package executable, **without executing it**.
//!
//! # What this establishes
//!
//! A first-party package's executable carries its own manifest template as a
//! contiguous byte envelope (see [`envelope`]). Reading it back from the
//! executable's bytes lets a host decide, by comparison alone, whether the
//! installed `verifier-plugin.json` still declares what the binary was built
//! to declare. Every field is compared, so a schema-valid, digest-intact edit
//! of `limits`, `targets`, `capabilities`, `filesystem_inputs` or any other
//! declaration is refused by name instead of being accepted.
//!
//! Only `package_digest` is exempt from the comparison, because a template
//! cannot contain the hash of the executable that contains it: the installed
//! digest is bound separately, against the recomputed hash of the same bytes
//! this module scanned.
//!
//! # What this does not establish
//!
//! This is still a **snapshot check over local bytes**, not a signature, not
//! source authenticity and not an immutable-execution identity:
//!
//! * An attacker who can *rebuild or forge the executable* can embed any
//!   template they like and install a manifest agreeing with it. The binding
//!   removes the "edit the manifest, keep the binary" class only.
//! * Nothing here proves the executable's *behavior* matches its template; the
//!   template is data inside the file, reached without running it.
//! * The package files may still change after the check. A host that needs an
//!   immutable execution identity must protect them itself.
use qcore_schemas::plugin::{PluginEntrypoint, PluginLimits, PluginManifest, SupportedTarget};
use std::fmt;

/// Opening marker of the embedded template envelope, version 1.
pub const TEMPLATE_BEGIN: &str = "<<<qcore.provider-template.v1:begin>>>";
/// Closing marker of the embedded template envelope, version 1.
pub const TEMPLATE_END: &str = "<<<qcore.provider-template.v1:end>>>";
/// Largest embedded template payload accepted, matching the package runtime's
/// own installed-manifest cap.
pub const MAX_TEMPLATE_BYTES: usize = 64 * 1024;

/// Wrap a manifest template in the envelope a package executable embeds.
///
/// Packagers (and tests that construct packages) use this so the envelope
/// spelling has exactly one definition. The provider runtime builds the same
/// bytes at compile time with `concat!`, which is what places them contiguously
/// in the executable.
pub fn envelope(template: &str) -> String {
    format!("{TEMPLATE_BEGIN}\n{template}\n{TEMPLATE_END}")
}

/// Why an executable's embedded template could not be bound to the installed
/// manifest. Every variant is a refusal; none downgrades to "verified".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateBindingError {
    /// No envelope in the executable's bytes.
    Absent,
    /// More than one opening or closing marker: which envelope is authoritative
    /// is not decidable, so no envelope is accepted.
    Ambiguous { begin: usize, end: usize },
    /// An opening marker with no closing marker after it.
    Unterminated,
    /// Payload larger than [`MAX_TEMPLATE_BYTES`].
    Oversized { bytes: usize },
    /// The payload is not valid UTF-8, not JSON, or not a manifest.
    Malformed(String),
    /// The embedded template itself does not satisfy the manifest rules.
    InvalidTemplate(String),
    /// A declared field diverges from the embedded template.
    Divergent {
        field: String,
        installed: String,
        embedded: String,
    },
}

impl fmt::Display for TemplateBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => write!(
                formatter,
                "executable carries no `{TEMPLATE_BEGIN}` manifest template envelope; \
                 its declarations cannot be bound without executing it"
            ),
            Self::Ambiguous { begin, end } => write!(
                formatter,
                "executable carries an ambiguous manifest template envelope \
                 ({begin} opening and {end} closing markers; exactly one of each is required)"
            ),
            Self::Unterminated => formatter.write_str(
                "executable's manifest template envelope is unterminated (no closing marker)",
            ),
            Self::Oversized { bytes } => write!(
                formatter,
                "embedded manifest template is {bytes} bytes, over the {MAX_TEMPLATE_BYTES}-byte limit"
            ),
            Self::Malformed(message) => write!(
                formatter,
                "embedded manifest template is malformed: {message}"
            ),
            Self::InvalidTemplate(message) => write!(
                formatter,
                "embedded manifest template is not a valid manifest: {message}"
            ),
            Self::Divergent {
                field,
                installed,
                embedded,
            } => write!(
                formatter,
                "installed manifest field `{field}` diverges from the executable's embedded \
                 template: installed {installed}, embedded {embedded}"
            ),
        }
    }
}

impl std::error::Error for TemplateBindingError {}

fn occurrences(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }
    (0..=haystack.len() - needle.len())
        .filter(|start| &haystack[*start..start + needle.len()] == needle)
        .collect()
}

/// Read the single manifest template envelope out of an executable's bytes.
///
/// Fails closed on absence, ambiguity (more than one marker of either kind),
/// truncation, size and malformed content. Does not execute anything.
pub fn extract_template(executable: &[u8]) -> Result<PluginManifest, TemplateBindingError> {
    let begins = occurrences(executable, TEMPLATE_BEGIN.as_bytes());
    let ends = occurrences(executable, TEMPLATE_END.as_bytes());
    if begins.is_empty() && ends.is_empty() {
        return Err(TemplateBindingError::Absent);
    }
    if begins.len() > 1 || ends.len() > 1 {
        return Err(TemplateBindingError::Ambiguous {
            begin: begins.len(),
            end: ends.len(),
        });
    }
    let (Some(begin), Some(end)) = (begins.first(), ends.first()) else {
        return Err(TemplateBindingError::Unterminated);
    };
    let payload_start = begin + TEMPLATE_BEGIN.len();
    if *end < payload_start {
        return Err(TemplateBindingError::Unterminated);
    }
    let payload = &executable[payload_start..*end];
    if payload.len() > MAX_TEMPLATE_BYTES {
        return Err(TemplateBindingError::Oversized {
            bytes: payload.len(),
        });
    }
    let text = std::str::from_utf8(payload)
        .map_err(|error| TemplateBindingError::Malformed(error.to_string()))?;
    let template: PluginManifest = serde_json::from_str(text.trim())
        .map_err(|error| TemplateBindingError::Malformed(error.to_string()))?;
    template
        .validate()
        .map_err(|error| TemplateBindingError::InvalidTemplate(error.to_string()))?;
    Ok(template)
}

fn render<T: fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn require<T: PartialEq + fmt::Debug>(
    field: &str,
    installed: &T,
    embedded: &T,
) -> Result<(), TemplateBindingError> {
    if installed == embedded {
        return Ok(());
    }
    Err(TemplateBindingError::Divergent {
        field: field.to_string(),
        installed: render(installed),
        embedded: render(embedded),
    })
}

fn compare_limits(
    installed: &PluginLimits,
    embedded: &PluginLimits,
) -> Result<(), TemplateBindingError> {
    // Destructured so a new schema limit field is a compile error here rather
    // than a silently unbound declaration an execution profile would install.
    let PluginLimits {
        timeout_ms,
        memory_bytes,
        output_bytes,
        additional,
    } = installed;
    require("limits.timeout_ms", timeout_ms, &embedded.timeout_ms)?;
    require("limits.memory_bytes", memory_bytes, &embedded.memory_bytes)?;
    require("limits.output_bytes", output_bytes, &embedded.output_bytes)?;
    // Named per kind so a widened or invented enforced limit is reported by the
    // kind an execution profile would have installed.
    let kinds: std::collections::BTreeSet<&String> = additional
        .keys()
        .chain(embedded.additional.keys())
        .collect();
    for kind in kinds {
        require(
            &format!("limits.additional.{kind}"),
            &additional.get(kind),
            &embedded.additional.get(kind),
        )?;
    }
    Ok(())
}

fn compare_targets(
    installed: &[SupportedTarget],
    embedded: &[SupportedTarget],
) -> Result<(), TemplateBindingError> {
    require("targets.len", &installed.len(), &embedded.len())?;
    for (index, (installed, embedded)) in installed.iter().zip(embedded).enumerate() {
        let SupportedTarget {
            library,
            revisions,
            profiles,
            dialects,
            platforms,
            backends,
        } = installed;
        require(
            &format!("targets[{index}].library"),
            library,
            &embedded.library,
        )?;
        require(
            &format!("targets[{index}].revisions"),
            revisions,
            &embedded.revisions,
        )?;
        require(
            &format!("targets[{index}].profiles"),
            profiles,
            &embedded.profiles,
        )?;
        require(
            &format!("targets[{index}].dialects"),
            dialects,
            &embedded.dialects,
        )?;
        require(
            &format!("targets[{index}].platforms"),
            platforms,
            &embedded.platforms,
        )?;
        require(
            &format!("targets[{index}].backends"),
            backends,
            &embedded.backends,
        )?;
    }
    Ok(())
}

/// Compare every installed declaration against the embedded template.
///
/// `executable_digest` is the recomputed SHA-256 of the same executable bytes
/// the template came from; it replaces the template's build placeholder, which
/// is the single field a template cannot carry. Every other field must agree.
pub fn bind_manifest(
    installed: &PluginManifest,
    template: &PluginManifest,
    executable_digest: &str,
) -> Result<(), TemplateBindingError> {
    // Destructuring pins the field inventory: adding a manifest field without
    // deciding how it binds fails to compile instead of going unchecked.
    let PluginManifest {
        plugin_id,
        package_version,
        protocol_versions,
        evidence_versions,
        entrypoint,
        package_digest,
        targets,
        capabilities,
        required_foreign_summaries,
        required_observation_hooks,
        filesystem_inputs,
        network_access,
        limits,
        trust_class,
        certificate_checker,
        lean_theorem_family,
    } = installed;
    require("plugin_id", plugin_id, &template.plugin_id)?;
    require(
        "package_version",
        package_version,
        &template.package_version,
    )?;
    require(
        "protocol_versions",
        protocol_versions,
        &template.protocol_versions,
    )?;
    require(
        "evidence_versions",
        evidence_versions,
        &template.evidence_versions,
    )?;
    require("entrypoint", entrypoint, &template.entrypoint)?;
    // The one substituted field: an executable cannot embed its own hash, so it
    // is bound to the recomputed digest of the bytes scanned above instead.
    require(
        "package_digest",
        package_digest,
        &executable_digest.to_string(),
    )?;
    compare_targets(targets, &template.targets)?;
    require("capabilities", capabilities, &template.capabilities)?;
    require(
        "required_foreign_summaries",
        required_foreign_summaries,
        &template.required_foreign_summaries,
    )?;
    require(
        "required_observation_hooks",
        required_observation_hooks,
        &template.required_observation_hooks,
    )?;
    require(
        "filesystem_inputs",
        filesystem_inputs,
        &template.filesystem_inputs,
    )?;
    require("network_access", network_access, &template.network_access)?;
    compare_limits(limits, &template.limits)?;
    require("trust_class", trust_class, &template.trust_class)?;
    require(
        "certificate_checker",
        certificate_checker,
        &template.certificate_checker,
    )?;
    require(
        "lean_theorem_family",
        lean_theorem_family,
        &template.lean_theorem_family,
    )?;
    Ok(())
}

/// Extract the template from `executable` and bind `installed` to it.
///
/// This is the whole enable-time gate: locate, parse, then compare. It reads
/// bytes only.
pub fn verify_embedded_template_binding(
    installed: &PluginManifest,
    executable: &[u8],
) -> Result<PluginManifest, TemplateBindingError> {
    let PluginEntrypoint::Executable { .. } = installed.entrypoint else {
        return Err(TemplateBindingError::InvalidTemplate(
            "built-in manifests have no executable to bind".into(),
        ));
    };
    let template = extract_template(executable)?;
    let digest = qcore_schemas::sha256_hex(executable);
    bind_manifest(installed, &template, &digest)?;
    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn template_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: qcore_schemas::plugin::PluginId::new("example.binding").unwrap(),
            package_version: "1.2.3".into(),
            protocol_versions: BTreeSet::from([1]),
            evidence_versions: BTreeSet::from([1, 2]),
            entrypoint: PluginEntrypoint::Executable {
                path: "provider".into(),
            },
            package_digest: "0".repeat(64),
            targets: vec![SupportedTarget {
                library: "example".into(),
                revisions: BTreeSet::from(["*".into()]),
                profiles: BTreeSet::from(["*".into()]),
                dialects: BTreeSet::from(["c11".into()]),
                platforms: BTreeSet::from(["*".into()]),
                backends: BTreeSet::from(["*".into()]),
            }],
            capabilities: BTreeSet::from([
                qcore_schemas::plugin::VerifierCapability::ResourceContracts,
            ]),
            required_foreign_summaries: BTreeSet::new(),
            required_observation_hooks: BTreeSet::new(),
            filesystem_inputs: BTreeSet::new(),
            network_access: false,
            limits: PluginLimits {
                timeout_ms: 5_000,
                memory_bytes: 268_435_456,
                output_bytes: 4_194_304,
                additional: BTreeMap::from([("cpu_seconds".into(), 7)]),
            },
            trust_class: qcore_schemas::plugin::PluginTrustClass::FirstParty,
            certificate_checker: "qcore.example.v1".into(),
            lean_theorem_family: "QCore.Example".into(),
        }
    }

    fn executable(template: &PluginManifest) -> Vec<u8> {
        let mut bytes = b"\x7fELF-ish prefix".to_vec();
        bytes.extend(envelope(&serde_json::to_string_pretty(template).unwrap()).into_bytes());
        bytes.extend(b"trailing machine code");
        bytes
    }

    fn installed(template: &PluginManifest, bytes: &[u8]) -> PluginManifest {
        let mut manifest = template.clone();
        manifest.package_digest = qcore_schemas::sha256_hex(bytes);
        manifest
    }

    #[test]
    fn an_unmodified_package_binds() {
        let template = template_manifest();
        let bytes = executable(&template);
        let installed = installed(&template, &bytes);
        assert_eq!(
            verify_embedded_template_binding(&installed, &bytes).unwrap(),
            template
        );
    }

    #[test]
    fn every_declared_field_class_is_bound() {
        let template = template_manifest();
        let bytes = executable(&template);
        let base = installed(&template, &bytes);
        type Mutation = (&'static str, Box<dyn Fn(&mut PluginManifest)>);
        let mutations: Vec<Mutation> = vec![
            (
                "plugin_id",
                Box::new(|m: &mut PluginManifest| {
                    m.plugin_id = qcore_schemas::plugin::PluginId::new("example.other").unwrap()
                }),
            ),
            (
                "package_version",
                Box::new(|m: &mut PluginManifest| m.package_version = "9.9.9".into()),
            ),
            (
                "protocol_versions",
                Box::new(|m: &mut PluginManifest| m.protocol_versions = BTreeSet::from([1, 2])),
            ),
            (
                "evidence_versions",
                Box::new(|m: &mut PluginManifest| m.evidence_versions = BTreeSet::from([1])),
            ),
            (
                "entrypoint",
                Box::new(|m: &mut PluginManifest| {
                    m.entrypoint = PluginEntrypoint::Executable {
                        path: "other".into(),
                    }
                }),
            ),
            (
                "targets[0].library",
                Box::new(|m: &mut PluginManifest| m.targets[0].library = "other".into()),
            ),
            (
                "targets[0].platforms",
                Box::new(|m: &mut PluginManifest| {
                    m.targets[0].platforms = BTreeSet::from(["linux".into()])
                }),
            ),
            (
                "targets.len",
                Box::new(|m: &mut PluginManifest| {
                    let extra = m.targets[0].clone();
                    m.targets.push(extra);
                }),
            ),
            (
                "capabilities",
                Box::new(|m: &mut PluginManifest| {
                    m.capabilities
                        .insert(qcore_schemas::plugin::VerifierCapability::ObservationHooks);
                }),
            ),
            (
                "required_foreign_summaries",
                Box::new(|m: &mut PluginManifest| {
                    m.required_foreign_summaries = BTreeSet::from(["extra".into()]);
                }),
            ),
            (
                "required_observation_hooks",
                Box::new(|m: &mut PluginManifest| {
                    m.required_observation_hooks = BTreeSet::from(["resize".into()]);
                }),
            ),
            (
                "filesystem_inputs",
                Box::new(|m: &mut PluginManifest| {
                    m.filesystem_inputs = BTreeSet::from(["data".into()]);
                }),
            ),
            (
                "limits.timeout_ms",
                Box::new(|m: &mut PluginManifest| m.limits.timeout_ms = 60_000),
            ),
            (
                "limits.memory_bytes",
                Box::new(|m: &mut PluginManifest| m.limits.memory_bytes = 1 << 34),
            ),
            (
                "limits.output_bytes",
                Box::new(|m: &mut PluginManifest| m.limits.output_bytes = 8_388_608),
            ),
            (
                "limits.additional.cpu_seconds",
                Box::new(|m: &mut PluginManifest| {
                    m.limits.additional.insert("cpu_seconds".into(), 900);
                }),
            ),
            (
                "limits.additional.max_processes",
                Box::new(|m: &mut PluginManifest| {
                    m.limits.additional.insert("max_processes".into(), 64);
                }),
            ),
            (
                "trust_class",
                Box::new(|m: &mut PluginManifest| {
                    m.trust_class = qcore_schemas::plugin::PluginTrustClass::ThirdParty
                }),
            ),
            (
                "certificate_checker",
                Box::new(|m: &mut PluginManifest| m.certificate_checker = "qcore.other.v1".into()),
            ),
            (
                "lean_theorem_family",
                Box::new(|m: &mut PluginManifest| m.lean_theorem_family = "QCore.Other".into()),
            ),
        ];
        for (field, mutate) in mutations {
            let mut candidate = base.clone();
            mutate(&mut candidate);
            assert!(candidate.validate().is_ok(), "{field} stays schema-valid");
            match verify_embedded_template_binding(&candidate, &bytes) {
                Err(TemplateBindingError::Divergent { field: named, .. }) => {
                    assert_eq!(named, field)
                }
                other => panic!("{field} was not refused by name: {other:?}"),
            }
        }
    }

    #[test]
    fn a_digest_edit_is_refused_as_the_substituted_field() {
        let template = template_manifest();
        let bytes = executable(&template);
        let mut candidate = installed(&template, &bytes);
        candidate.package_digest = "a".repeat(64);
        assert!(matches!(
            verify_embedded_template_binding(&candidate, &bytes),
            Err(TemplateBindingError::Divergent { field, .. }) if field == "package_digest"
        ));
    }

    #[test]
    fn absent_ambiguous_truncated_and_corrupt_envelopes_refuse() {
        let template = template_manifest();
        let bytes = executable(&template);
        let base = installed(&template, &bytes);
        assert_eq!(
            verify_embedded_template_binding(&base, b"no envelope here"),
            Err(TemplateBindingError::Absent)
        );
        let doubled = [bytes.clone(), bytes.clone()].concat();
        assert_eq!(
            verify_embedded_template_binding(&base, &doubled),
            Err(TemplateBindingError::Ambiguous { begin: 2, end: 2 })
        );
        let opening = format!("prefix{TEMPLATE_BEGIN}{{\"plugin_id\":\"x.y\"}}");
        assert_eq!(
            verify_embedded_template_binding(&base, opening.as_bytes()),
            Err(TemplateBindingError::Unterminated)
        );
        let reversed = format!("{TEMPLATE_END}payload{TEMPLATE_BEGIN}");
        assert_eq!(
            verify_embedded_template_binding(&base, reversed.as_bytes()),
            Err(TemplateBindingError::Unterminated)
        );
        let corrupt = envelope("{\"plugin_id\": \"example.binding\",");
        assert!(matches!(
            verify_embedded_template_binding(&base, corrupt.as_bytes()),
            Err(TemplateBindingError::Malformed(_))
        ));
        let mut invalid = template.clone();
        invalid.capabilities = BTreeSet::new();
        let invalid = envelope(&serde_json::to_string(&invalid).unwrap());
        assert!(matches!(
            verify_embedded_template_binding(&base, invalid.as_bytes()),
            Err(TemplateBindingError::InvalidTemplate(_))
        ));
    }

    #[test]
    fn an_oversized_payload_refuses_without_parsing() {
        let template = template_manifest();
        let base = installed(&template, &executable(&template));
        let payload = envelope(&" ".repeat(MAX_TEMPLATE_BYTES + 1));
        assert_eq!(
            verify_embedded_template_binding(&base, payload.as_bytes()),
            Err(TemplateBindingError::Oversized {
                bytes: MAX_TEMPLATE_BYTES + 3
            })
        );
    }
}
