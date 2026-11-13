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

//! Explicit, conditional external-linkage resolution premise for W4 imports.
//! A selected source set cannot infer a linker's choice from matching names.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use qcore_c_gateway::legacy_c::CrossUnitExport;
use qcore_schemas::contract::{
    CrossUnitImportIdentity, ResourceContracts, ResourceFunctionContract,
};
use serde::Deserialize;
use serde_json::Value;

/// Opt-in path to a human-supplied link assertion. Unset means the old
/// fail-closed route with no imported summary and byte-identical output.
pub const SELECTION: &str = "QCORE_CROSS_UNIT_LINK_MAP";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u32,
    premise: String,
    symbols: Vec<Symbol>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Symbol {
    function: String,
    home_unit: String,
}

/// One stated link premise, with its exact text pinned by SHA-256.
#[derive(Clone)]
pub struct LinkMap {
    pub digest: String,
    pub premise: String,
    homes: BTreeMap<String, String>,
}

impl LinkMap {
    pub fn load_selected() -> Result<Option<Self>, String> {
        let Some(path) = std::env::var_os(SELECTION) else {
            return Ok(None);
        };
        Self::load(Path::new(&path)).map(Some)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let input: Input = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if input.schema_version != 1 || input.premise.trim().is_empty() || input.symbols.is_empty()
        {
            return Err("cross-unit link map needs version 1, a premise and symbols".into());
        }
        let mut homes = BTreeMap::new();
        for symbol in input.symbols {
            if symbol.function.is_empty()
                || symbol.home_unit.is_empty()
                || homes
                    .insert(symbol.function.clone(), symbol.home_unit)
                    .is_some()
            {
                return Err("cross-unit link map has an empty or duplicate symbol".into());
            }
        }
        Ok(Self {
            digest: qcore_schemas::sha256_hex(&bytes),
            premise: input.premise,
            homes,
        })
    }

    pub fn requested(&self) -> BTreeSet<String> {
        self.homes.keys().cloned().collect()
    }

    pub fn names_for(&self, unit: &str) -> BTreeSet<String> {
        self.homes
            .iter()
            .filter(|(_, home)| home.as_str() == unit)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// The unit named by the supplied resolution premise for this function.
    pub fn home_for(&self, function: &str) -> Option<&str> {
        self.homes.get(function).map(String::as_str)
    }

    /// Select only uniquely exported, exact home names. A missing or
    /// ambiguous home leaves its caller uncontracted. The map's claimed
    /// `home_unit` is never inferred from a name alone.
    pub fn select(
        &self,
        exports: impl IntoIterator<Item = (String, CrossUnitExport)>,
        definitions: impl IntoIterator<Item = (String, String)>,
        loaded: &ResourceContracts,
    ) -> (
        ResourceContracts,
        Vec<qcore_c_gateway::legacy_c::CrossUnitHomeEvidence>,
    ) {
        let mut by_name: BTreeMap<String, Vec<(String, CrossUnitExport)>> = BTreeMap::new();
        for (unit, export) in exports {
            by_name
                .entry(export.home.function.clone())
                .or_default()
                .push((unit, export));
        }
        let mut defined_in: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (unit, name) in definitions {
            if self.homes.contains_key(&name) {
                defined_in.entry(name).or_default().push(unit);
            }
        }
        let mut environment = loaded.clone();
        let mut imports = BTreeMap::<String, ResourceFunctionContract>::new();
        let mut lifetimes = BTreeMap::new();
        let mut identities = BTreeMap::<String, CrossUnitImportIdentity>::new();
        let mut homes = Vec::new();
        for (name, claimed_unit) in &self.homes {
            let Some(candidates) = by_name.get(name) else {
                continue;
            };
            if let [(unit, candidate)] = candidates.as_slice()
                && unit == claimed_unit
                && defined_in
                    .get(name)
                    .is_some_and(|rows| rows == std::slice::from_ref(unit))
                && !loaded.functions().contains_key(name)
            {
                if let Some(lifetime) = &candidate.lifetime {
                    lifetimes.insert(name.clone(), lifetime.clone());
                } else {
                    imports.insert(name.clone(), candidate.contract.clone());
                }
                identities.insert(name.clone(), candidate.home.import_identity());
                homes.push(candidate.home.clone());
            }
        }
        environment.set_link_resolution_sha256(Some(self.digest.clone()));
        environment.set_inferred_imports(imports);
        environment.set_lifetime_imports(lifetimes);
        environment.set_cross_unit_import_identities(identities);
        (environment, homes)
    }
}

// ---- Rule 40: a derived link resolution ------------------------------------

/// Card **LINK-STANDING** (rule 40): opt-in path to a link-resolution
/// artifact (`quirin-link-resolution/1`, written by `prepare --link`). Read
/// by the `current` route only. Exclusive with [`SELECTION`].
pub const RESOLUTION_SELECTION: &str = "QCORE_CROSS_UNIT_LINK_RESOLUTION";

/// Why rule 40 refuses a home for a caller, in decision order
/// (`QCore.CrossUnitSummary.Refusal`). Each refusal leaves the call
/// uncontracted, as it was without the artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Refusal {
    DatabaseDiffers,
    DigestUnverified,
    CallerUnlinked,
    NotExecutable,
    NotInExecutable,
    NotStrong,
    NotDefaultVisibility,
    Duplicate,
    DynamicRelocation,
    FactsRefused,
    HomeUnlinked,
    NotRouteUnit,
    AmbiguousKey,
    TypeUnavailable,
    TypeDiffers,
}

impl Refusal {
    pub const ALL: [Self; 15] = [
        Self::DatabaseDiffers,
        Self::DigestUnverified,
        Self::CallerUnlinked,
        Self::NotExecutable,
        Self::NotInExecutable,
        Self::NotStrong,
        Self::NotDefaultVisibility,
        Self::Duplicate,
        Self::DynamicRelocation,
        Self::FactsRefused,
        Self::HomeUnlinked,
        Self::NotRouteUnit,
        Self::AmbiguousKey,
        Self::TypeUnavailable,
        Self::TypeDiffers,
    ];

    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::DatabaseDiffers => {
                "the link resolution artifact's compile database differs from the route's"
            }
            Self::DigestUnverified => "a digest the link resolution artifact binds does not verify",
            Self::CallerUnlinked => {
                "caller configuration is not a linked, reproduced configuration of the executable"
            }
            Self::NotExecutable => "the linked image is not an executable",
            Self::NotInExecutable => "home is not defined in the executable",
            Self::FactsRefused => "the artifact refuses the home's link facts",
            Self::NotStrong => "home is not a strong global function definition",
            Self::NotDefaultVisibility => "home definition does not have default visibility",
            Self::Duplicate => "home has more than one definition among the linked objects",
            Self::DynamicRelocation => {
                "the executable resolves the home at run time (dynamic relocation)"
            }
            Self::HomeUnlinked => {
                "home configuration is not a linked, reproduced configuration of the executable"
            }
            Self::NotRouteUnit => "home configuration is not a configuration the route analyses",
            Self::AmbiguousKey => {
                "home unit key names another route configuration not shown outside the link"
            }
            Self::TypeUnavailable => qcore_c_gateway::link_call_type::TYPE_UNAVAILABLE,
            Self::TypeDiffers => qcore_c_gateway::link_call_type::TYPE_DIFFERS,
        }
    }
}

/// Every condition rule 40 checks for one (caller, home) pair, in decision
/// order; field `i` failing is [`Refusal::ALL`]`[i]`
/// (`QCore.CrossUnitSummary.DerivedFacts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Facts {
    pub database_matches: bool,
    pub digests_verify: bool,
    pub caller_linked: bool,
    pub executable: bool,
    pub in_executable: bool,
    pub strong_global: bool,
    pub default_visibility: bool,
    pub only_definition: bool,
    pub no_dynamic_relocation: bool,
    /// The artifact itself refused none of the link's facts (map and symbol
    /// tables agree, a unique database entry, not an indirect function,
    /// reproduced); only its route-relative refusals are re-decided here.
    pub facts_agree: bool,
    pub home_linked: bool,
    pub route_unit: bool,
    /// Under the home's `library/file` key, the home's configuration is the
    /// only route configuration the link can use: every other one is shown
    /// outside the link (card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS).
    pub unique_key: bool,
    pub type_available: bool,
    pub type_equal: bool,
}

impl Facts {
    pub const ALL: Self = Self {
        database_matches: true,
        digests_verify: true,
        caller_linked: true,
        executable: true,
        in_executable: true,
        facts_agree: true,
        strong_global: true,
        default_visibility: true,
        only_definition: true,
        no_dynamic_relocation: true,
        home_linked: true,
        route_unit: true,
        unique_key: true,
        type_available: true,
        type_equal: true,
    };

    const fn fields(self) -> [bool; 15] {
        [
            self.database_matches,
            self.digests_verify,
            self.caller_linked,
            self.executable,
            self.in_executable,
            self.strong_global,
            self.default_visibility,
            self.only_definition,
            self.no_dynamic_relocation,
            self.facts_agree,
            self.home_linked,
            self.route_unit,
            self.unique_key,
            self.type_available,
            self.type_equal,
        ]
    }

    #[cfg(test)]
    fn failing(indices: &[usize]) -> Self {
        let mut fields = Self::ALL.fields();
        for &index in indices {
            fields[index] = false;
        }
        let [
            database_matches,
            digests_verify,
            caller_linked,
            executable,
            in_executable,
            strong_global,
            default_visibility,
            only_definition,
            no_dynamic_relocation,
            facts_agree,
            home_linked,
            route_unit,
            unique_key,
            type_available,
            type_equal,
        ] = fields;
        Self {
            database_matches,
            digests_verify,
            caller_linked,
            executable,
            in_executable,
            facts_agree,
            strong_global,
            default_visibility,
            only_definition,
            no_dynamic_relocation,
            home_linked,
            route_unit,
            unique_key,
            type_available,
            type_equal,
        }
    }
}

/// Rule 40's decision: the first condition that fails, or `None` when the
/// derived resolution resolves the call to the home
/// (`QCore.CrossUnitSummary.decide`).
#[must_use]
pub fn decide(facts: Facts) -> Option<Refusal> {
    facts
        .fields()
        .iter()
        .position(|holds| !holds)
        .map(|index| Refusal::ALL[index])
}

/// `STANDING_VECTORS` (`QCore.CrossUnitSummary.standingVectors`): the
/// indices of the failing conditions, then the refusal's index.
#[cfg(test)]
pub(crate) const STANDING_VECTORS: &[(&[usize], Option<usize>)] = &[
    (&[], None),
    (&[0], Some(0)),
    (&[1], Some(1)),
    (&[2], Some(2)),
    (&[3], Some(3)),
    (&[4], Some(4)),
    (&[5], Some(5)),
    (&[6], Some(6)),
    (&[7], Some(7)),
    (&[8], Some(8)),
    (&[9], Some(9)),
    (&[10], Some(10)),
    (&[11], Some(11)),
    (&[12], Some(12)),
    (&[13], Some(13)),
    (&[14], Some(14)),
    (&[2, 14], Some(2)),
    (&[12, 13], Some(12)),
    (&[0, 1, 2], Some(0)),
    (&[9, 11], Some(9)),
];

/// Card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS: a recorded
/// configuration is shown outside the link only when the artifact states
/// `linked: false` and every input's attribution is readable and names it
/// nowhere (`attributed == Some(false)`); anything else may be what the link
/// used (`QCore.CrossUnitSummary.shownOutside`).
#[must_use]
pub const fn shown_outside(linked: Option<bool>, attributed: Option<bool>) -> bool {
    matches!((linked, attributed), (Some(false), Some(false)))
}

/// `OUTSIDE_VECTORS` (`QCore.CrossUnitSummary.outsideVectors`): the
/// artifact's `linked` flag, the attribution, and whether the configuration
/// is shown outside the link.
#[cfg(test)]
pub(crate) const OUTSIDE_VECTORS: &[(Option<bool>, Option<bool>, bool)] = &[
    (Some(false), Some(false), true),
    (Some(false), Some(true), false),
    (Some(false), None, false),
    (Some(true), Some(false), false),
    (Some(true), Some(true), false),
    (Some(true), None, false),
    (None, Some(false), false),
    (None, Some(true), false),
    (None, None, false),
];

/// One compile-database configuration as the artifact records it: its
/// identity and whether the executable links it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigurationIdentity {
    pub file: String,
    pub directory: String,
    pub command_sha256: String,
}

impl ConfigurationIdentity {
    #[must_use]
    pub fn of(entry: &crate::compile_db::CompileEntry) -> Self {
        Self {
            file: entry.file.to_string_lossy().into_owned(),
            directory: entry.directory.to_string_lossy().into_owned(),
            command_sha256: qcore_schemas::sha256_hex(entry.command.as_bytes()),
        }
    }
}

struct ArtifactConfiguration {
    identity: ConfigurationIdentity,
    /// Linked into the executable and reproduced byte for byte by its own
    /// database command.
    linked: bool,
    /// Shown outside the link: every link input states the configurations
    /// whose object has its bytes, and none names this one, so the link used
    /// nothing it compiles (card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS).
    /// `linked: false` alone does not show it: an input whose bytes match
    /// two configurations leaves both unlinked, and either may be the one used.
    outside: bool,
}

/// One referenced function symbol of the artifact, with the home-level
/// facts its record states.
struct ArtifactSymbol {
    name: String,
    configuration: Option<usize>,
    facts: Facts,
    record: Value,
}

/// A loaded link-resolution artifact (rule 40). Nothing in it is trusted
/// before [`DerivedResolution::verify`].
pub struct DerivedResolution {
    pub path: std::path::PathBuf,
    pub artifact_sha256: String,
    pub database_sha256: String,
    pub executable_sha256: String,
    pub link_map_sha256: String,
    executable: bool,
    bound: Vec<(std::path::PathBuf, String)>,
    inputs: Vec<(String, String)>,
    link_directory: std::path::PathBuf,
    configurations: Vec<ArtifactConfiguration>,
    symbols: Vec<ArtifactSymbol>,
}

fn text<'a>(value: &'a Value, what: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("link resolution artifact lacks {what}"))
}

/// The artifact's own refusal reasons that are facts of the link, which the
/// route cannot re-decide; the route-relative reasons it re-decides itself.
const ROUTE_RELATIVE_REASONS: [&str; 2] = [
    "home configuration is not a unit of the current route",
    "home unit key names more than one route configuration",
];

impl DerivedResolution {
    pub fn load_selected() -> Result<Option<Self>, String> {
        let Some(path) = std::env::var_os(RESOLUTION_SELECTION) else {
            return Ok(None);
        };
        Self::load(Path::new(&path)).map(Some)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let artifact: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if artifact["schema"].as_str() != Some(crate::link_resolution::ARTIFACT_SCHEMA) {
            return Err(format!(
                "{}: not a `{}` artifact",
                path.display(),
                crate::link_resolution::ARTIFACT_SCHEMA
            ));
        }
        let mut bound = vec![
            (
                text(&artifact["executable"]["path"], "the executable path")?.into(),
                text(&artifact["executable"]["sha256"], "the executable digest")?.to_owned(),
            ),
            (
                text(&artifact["link"]["link_txt"], "the link command path")?.into(),
                text(
                    &artifact["link"]["link_txt_sha256"],
                    "the link command digest",
                )?
                .to_owned(),
            ),
            (
                text(&artifact["link"]["map"], "the map path")?.into(),
                text(&artifact["link"]["map_sha256"], "the map digest")?.to_owned(),
            ),
        ];
        for tool in ["driver", "linker"] {
            bound.push((
                text(&artifact["toolchain"][tool]["resolved"], "a toolchain path")?.into(),
                text(&artifact["toolchain"][tool]["sha256"], "a toolchain digest")?.to_owned(),
            ));
        }
        let inputs = artifact["inputs"]
            .as_array()
            .ok_or("link resolution artifact lacks its inputs")?
            .iter()
            .map(|input| {
                Ok((
                    text(&input["label"], "an input label")?.to_owned(),
                    text(&input["sha256"], "an input digest")?.to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        // Which configurations a link input is attributed to. One input
        // without a readable list shows no configuration outside the link.
        let attributed = artifact["inputs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|input| {
                input["configurations"]
                    .as_array()
                    .and_then(|ids| ids.iter().map(Value::as_u64).collect::<Option<Vec<_>>>())
            })
            .collect::<Option<Vec<_>>>()
            .map(|lists| lists.into_iter().flatten().collect::<BTreeSet<u64>>());
        let configurations = artifact["configurations"]
            .as_array()
            .ok_or("link resolution artifact lacks its configurations")?
            .iter()
            .enumerate()
            .map(|(id, configuration)| {
                if configuration["id"].as_u64() != Some(id as u64) {
                    return Err("link resolution artifact's configurations are out of order".into());
                }
                Ok(ArtifactConfiguration {
                    identity: ConfigurationIdentity {
                        file: text(&configuration["file"], "a configuration file")?.to_owned(),
                        directory: text(&configuration["directory"], "a configuration directory")?
                            .to_owned(),
                        command_sha256: text(
                            &configuration["command_sha256"],
                            "a configuration command digest",
                        )?
                        .to_owned(),
                    },
                    linked: configuration["linked"].as_bool() == Some(true)
                        && configuration["reproduction"].as_str() == Some("equal"),
                    outside: shown_outside(
                        configuration["linked"].as_bool(),
                        attributed
                            .as_ref()
                            .map(|attributed| attributed.contains(&(id as u64))),
                    ),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let executable = artifact["executable"]["image"]["is_executable"].as_bool() == Some(true);
        let symbols = artifact["symbols"]
            .as_array()
            .ok_or("link resolution artifact lacks its symbols")?
            .iter()
            .map(|record| {
                let name = text(&record["name"], "a symbol name")?.to_owned();
                let definition = &record["definition"];
                let configuration = definition["configuration"]
                    .as_u64()
                    .and_then(|id| usize::try_from(id).ok())
                    .filter(|id| *id < configurations.len());
                let reason = record["reason"].as_str();
                let facts = Facts {
                    executable,
                    in_executable: definition["in"].as_str() == Some("executable"),
                    facts_agree: reason
                        .is_none_or(|reason| ROUTE_RELATIVE_REASONS.contains(&reason))
                        && record["executable"]["in_definer_text"].as_bool() == Some(true),
                    strong_global: record["kind"].as_str() == Some("function")
                        && definition["type"].as_str() == Some("function")
                        && definition["binding"].as_str() == Some("global"),
                    default_visibility: definition["visibility"].as_str() == Some("default"),
                    only_definition: record["definitions"]
                        .as_array()
                        .is_some_and(|all| all.len() == 1),
                    no_dynamic_relocation: record["executable"]["dynamic_relocations"].as_u64()
                        == Some(0),
                    home_linked: configuration.is_some_and(|id| configurations[id].linked),
                    ..Facts::ALL
                };
                Ok(ArtifactSymbol {
                    name,
                    configuration,
                    facts,
                    record: serde_json::json!({
                        "definition": definition,
                        "executable": record["executable"],
                    }),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            path: path.to_path_buf(),
            artifact_sha256: qcore_schemas::sha256_hex(&bytes),
            database_sha256: text(
                &artifact["compile_database"]["sha256"],
                "the database digest",
            )?
            .to_owned(),
            executable_sha256: bound[0].1.clone(),
            link_map_sha256: bound[2].1.clone(),
            executable,
            bound: std::mem::take(&mut bound),
            inputs,
            link_directory: text(&artifact["link"]["directory"], "the link directory")?.into(),
            configurations,
            symbols,
        })
    }

    /// The run-level conditions: the database the route reads is the one
    /// the artifact was read from, and every file the artifact binds by
    /// digest (executable, link command, map, toolchain, linked inputs)
    /// still has that digest.
    pub fn verify(&self, route_database_sha256: &str) -> Result<(), (Refusal, String)> {
        if self.database_sha256 != route_database_sha256 {
            return Err((
                Refusal::DatabaseDiffers,
                format!(
                    "artifact {}, route {route_database_sha256}",
                    self.database_sha256
                ),
            ));
        }
        let differs = |what: &str, found: Result<String, String>, expected: &str| match found {
            Ok(found) if found == expected => Ok(()),
            Ok(found) => Err((
                Refusal::DigestUnverified,
                format!("{what}: {found}, artifact {expected}"),
            )),
            Err(error) => Err((Refusal::DigestUnverified, format!("{what}: {error}"))),
        };
        for (path, expected) in &self.bound {
            differs(
                &path.display().to_string(),
                fs::read(path)
                    .map(|bytes| qcore_schemas::sha256_hex(&bytes))
                    .map_err(|error| error.to_string()),
                expected,
            )?;
        }
        for (label, expected) in &self.inputs {
            differs(
                label,
                crate::link_resolution::input_bytes(&self.link_directory, label)
                    .map(|bytes| qcore_schemas::sha256_hex(&bytes)),
                expected,
            )?;
        }
        Ok(())
    }

    /// The home-level conditions against the route's own configurations
    /// (each with its `library/file` key): a derived [`LinkMap`] naming each
    /// admitted home by the route's key, the caller standing, and the
    /// refusals by symbol.
    #[must_use]
    pub fn derive(&self, route: &[(String, ConfigurationIdentity)]) -> Derived {
        // Card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS: condition 13
        // counts, under each key, the route configurations the link can use.
        // A configuration is left out only when the artifact records it and
        // every record of it is shown outside the link; one the artifact
        // does not record cannot be shown and still counts (fail-closed).
        let mut recorded = BTreeMap::<&ConfigurationIdentity, bool>::new();
        for configuration in &self.configurations {
            *recorded.entry(&configuration.identity).or_insert(true) &= configuration.outside;
        }
        let outside = route
            .iter()
            .filter(|(_, identity)| recorded.get(identity) == Some(&true))
            .map(|(_, identity)| identity.clone())
            .collect::<BTreeSet<_>>();
        let mut keys = BTreeMap::<&str, usize>::new();
        for (key, identity) in route {
            if !outside.contains(identity) {
                *keys.entry(key.as_str()).or_default() += 1;
            }
        }
        let by_identity = route
            .iter()
            .map(|(key, identity)| (identity, key.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut homes = BTreeMap::new();
        let mut records = BTreeMap::new();
        let mut refused = BTreeMap::<Refusal, usize>::new();
        for symbol in &self.symbols {
            let key = symbol
                .configuration
                .and_then(|id| by_identity.get(&self.configurations[id].identity).copied());
            let facts = Facts {
                route_unit: key.is_some(),
                unique_key: key.is_some_and(|key| keys.get(key) == Some(&1))
                    && symbol
                        .configuration
                        .is_some_and(|id| !outside.contains(&self.configurations[id].identity)),
                ..symbol.facts
            };
            match decide(facts) {
                None => {
                    if let Some(key) = key {
                        homes.insert(symbol.name.clone(), key.to_owned());
                        records.insert(symbol.name.clone(), symbol.record.clone());
                    }
                }
                Some(refusal) => *refused.entry(refusal).or_default() += 1,
            }
        }
        let premise = format!(
            "DERIVED (rule 40), not a stated premise: each listed home is the one strong global, \
             default-visibility definition of its name among the objects linked into the \
             executable (sha256 {}), from a reproduced, linked configuration that is the route's \
             only configuration under its key that the link can use (every other one is recorded \
             by the artifact and no link input is attributed to it), and the executable carries \
             no dynamic relocation \
             against it; read from the link-resolution artifact (sha256 {}) bound to the route's \
             compile database (sha256 {}). A caller holds a home only when its own configuration \
             is linked and Clang's type of the callee in its unit equals the home definition's.",
            self.executable_sha256, self.artifact_sha256, self.database_sha256
        );
        let linked = self
            .configurations
            .iter()
            .filter(|configuration| configuration.linked)
            .map(|configuration| configuration.identity.clone())
            .collect();
        Derived {
            map: LinkMap {
                digest: self.artifact_sha256.clone(),
                premise,
                homes,
            },
            standing: LinkStanding {
                linked,
                records,
                home_types: std::sync::Mutex::default(),
            },
            refused,
            outside,
            evidence: serde_json::json!({
                "artifact": self.path,
                "artifact_sha256": self.artifact_sha256,
                "compile_database_sha256": self.database_sha256,
                "executable_sha256": self.executable_sha256,
                "link_map_sha256": self.link_map_sha256,
                "executable": self.executable,
            }),
        }
    }
}

/// What a verified artifact derives for one run.
pub struct Derived {
    /// Rule 4c's input: the admitted homes by the route's key, pinned by the
    /// artifact's digest.
    pub map: LinkMap,
    pub standing: LinkStanding,
    pub refused: BTreeMap<Refusal, usize>,
    /// The route configurations shown outside the link (card
    /// LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS): each exports and
    /// defines nothing in rule 4c's census, since the link used nothing it
    /// compiles, so the linked configuration of its key is the one home.
    pub outside: BTreeSet<ConfigurationIdentity>,
    pub evidence: Value,
}

impl Derived {
    /// Whether the run's census leaves this configuration out.
    #[must_use]
    pub fn outside_the_link(&self, configuration: &ConfigurationIdentity) -> bool {
        self.outside.contains(configuration)
    }

    #[must_use]
    pub fn refused_by_reason(&self) -> Value {
        self.refused
            .iter()
            .map(|(refusal, count)| (refusal.phrase().to_owned(), Value::from(*count)))
            .collect::<serde_json::Map<_, _>>()
            .into()
    }
}

type HomeType = Result<
    (
        qcore_c_gateway::link_call_type::CallType,
        qcore_c_gateway::link_call_type::Layouts,
    ),
    String,
>;

/// The per-caller conditions of rule 40: a linked caller configuration and
/// the type check, applied to the homes rule 4c's scope holds.
pub struct LinkStanding {
    linked: BTreeSet<ConfigurationIdentity>,
    records: BTreeMap<String, Value>,
    /// Each home's type and the layouts of the records it names, read once
    /// per home unit text.
    home_types: std::sync::Mutex<BTreeMap<(String, String), std::sync::Arc<HomeType>>>,
}

impl LinkStanding {
    #[must_use]
    pub fn caller_linked(&self, caller: &ConfigurationIdentity) -> bool {
        self.linked.contains(caller)
    }

    /// The artifact's definition record of an admitted home.
    #[must_use]
    pub fn record(&self, function: &str) -> Option<&Value> {
        self.records.get(function)
    }

    fn home_type(
        &self,
        home: &qcore_c_gateway::legacy_c::CrossUnitHomeEvidence,
    ) -> std::sync::Arc<HomeType> {
        use qcore_c_gateway::link_call_type::{read, record_layouts};
        let key = (
            home.function.clone(),
            home.input_identity.preprocessed_source_sha256.clone(),
        );
        if let Some(found) = self
            .home_types
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
        {
            return found.clone();
        }
        let read = read(
            &home.preprocessed_source,
            &home.request,
            home.dialect,
            &home.function,
        )
        .and_then(|call| {
            let layouts = if call.records.is_empty() {
                BTreeMap::new()
            } else {
                record_layouts(&home.preprocessed_source, &home.request, home.dialect)?
                    .into_iter()
                    .filter(|(name, _)| call.records.contains(name))
                    .collect()
            };
            Ok((call, layouts))
        });
        let read = std::sync::Arc::new(read);
        self.home_types
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, read.clone());
        read
    }

    /// Rule 40 for one caller: keep a held home only when the caller is
    /// linked and the callee's type in the caller unit equals the home
    /// definition's. Returns the kept imports and each refused home with
    /// its reason.
    pub fn admit(
        &self,
        scoped: qcore_c_gateway::legacy_c::CrossUnitCallerImports,
        caller_linked: bool,
        caller: &qcore_c_gateway::legacy_c::PreparedTranslationUnit,
    ) -> (
        qcore_c_gateway::legacy_c::CrossUnitCallerImports,
        Vec<(String, String)>,
    ) {
        use qcore_c_gateway::link_call_type::{agree, read, record_layouts};
        let mut refusals = Vec::new();
        let mut kept = BTreeSet::new();
        let preprocessed = caller.preprocessed_source().ok();
        let request = qcore_c_gateway::legacy_c::LegacyCertificateRequest {
            include_paths: caller.include_paths().to_vec(),
            definitions: caller.definitions().to_vec(),
            preprocessor_options: caller.preprocessor_options().to_vec(),
            input: caller.input().clone(),
        };
        let caller_layouts = std::cell::OnceCell::new();
        for home in &scoped.homes {
            let facts = if caller_linked {
                Facts::ALL
            } else {
                Facts {
                    caller_linked: false,
                    ..Facts::ALL
                }
            };
            let outcome = if decide(facts).is_some() {
                Err((Refusal::CallerUnlinked, String::new()))
            } else {
                let call = preprocessed
                    .ok_or_else(|| "the caller unit was not preprocessed".to_owned())
                    .and_then(|source| read(source, &request, caller.dialect(), &home.function));
                let home_type = self.home_type(home);
                let layouts_of = |name: &str| -> Option<Option<String>> {
                    let layouts = caller_layouts.get_or_init(|| {
                        preprocessed
                            .and_then(|source| {
                                record_layouts(source, &request, caller.dialect()).ok()
                            })
                            .unwrap_or_default()
                    });
                    layouts.get(name).cloned()
                };
                let home_layouts = |name: &str| -> Option<Option<String>> {
                    home_type
                        .as_ref()
                        .as_ref()
                        .ok()
                        .and_then(|(_, layouts)| layouts.get(name).cloned())
                };
                agree(
                    call.as_ref().map_err(String::as_str),
                    &layouts_of,
                    home_type
                        .as_ref()
                        .as_ref()
                        .map(|(call, _)| call)
                        .map_err(String::as_str),
                    &home_layouts,
                )
                .map_err(|detail| {
                    let refusal = if call.is_ok() && home_type.is_ok() {
                        Refusal::TypeDiffers
                    } else {
                        Refusal::TypeUnavailable
                    };
                    (refusal, detail)
                })
            };
            match outcome {
                Ok(()) => {
                    kept.insert(home.function.clone());
                }
                Err((refusal, detail)) => refusals.push((
                    home.function.clone(),
                    if detail.is_empty() || detail == refusal.phrase() {
                        refusal.phrase().to_owned()
                    } else if detail.starts_with(refusal.phrase()) {
                        detail
                    } else {
                        format!("{}: {detail}", refusal.phrase())
                    },
                )),
            }
        }
        let scoped = qcore_c_gateway::legacy_c::cross_unit_retain_held(scoped, |home| {
            kept.contains(&home.function)
        });
        (scoped, refusals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_c_gateway::legacy_c::{
        C11TranslationUnit, LegacyFunctionScope, export_cross_unit_homes, prepare_translation_unit,
    };

    #[test]
    fn selected_home_requires_complete_unique_definition_census() {
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let base = ResourceContracts::default();
        let map = LinkMap {
            digest: "a".repeat(64),
            premise: "All calls to callee resolve to this unit's external definition".into(),
            homes: BTreeMap::from([("callee".into(), "lib/home.c".into())]),
        };
        let prepared = prepare_translation_unit(&C11TranslationUnit {
            source: "void callee(void) { return; }\n",
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let export = export_cross_unit_homes(&prepared, &base, &map.requested(), &map.digest)
            .expect("export")
            .pop()
            .expect("Safe home");
        let found = vec![("lib/home.c".into(), export.clone())];
        let (environment, homes) = map.select(
            found.clone(),
            [("lib/home.c".into(), "callee".into())],
            &base,
        );
        assert_eq!(homes.len(), 1);
        assert!(environment.inferred_imports().contains_key("callee"));
        let (_, ambiguous) = map.select(
            found.clone(),
            [
                ("lib/home.c".into(), "callee".into()),
                ("lib/other.c".into(), "callee".into()),
            ],
            &base,
        );
        assert!(ambiguous.is_empty());
        let (_, missing) = map.select(found, [], &base);
        assert!(missing.is_empty());
    }

    /// Card CURRENT-RUN-WORKER-SPIN: `count` exported homes `f0`..`f{count-1}`
    /// of one home unit, selected under a link map.
    fn exported_homes(
        count: usize,
    ) -> (
        ResourceContracts,
        Vec<qcore_c_gateway::legacy_c::CrossUnitHomeEvidence>,
    ) {
        let base = ResourceContracts::default();
        let names = (0..count)
            .map(|index| format!("f{index}"))
            .collect::<Vec<_>>();
        let map = LinkMap {
            digest: "b".repeat(64),
            premise: "Every listed external call resolves to lib/home.c".into(),
            homes: names
                .iter()
                .map(|name| (name.clone(), "lib/home.c".to_owned()))
                .collect(),
        };
        let source = names
            .iter()
            .map(|name| format!("void {name}(void) {{ return; }}\n"))
            .collect::<String>();
        let prepared = prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let exports = export_cross_unit_homes(&prepared, &base, &map.requested(), &map.digest)
            .expect("export");
        assert_eq!(exports.len(), count);
        map.select(
            exports
                .into_iter()
                .map(|export| ("lib/home.c".to_owned(), export)),
            names
                .iter()
                .map(|name| ("lib/home.c".to_owned(), name.clone())),
            &base,
        )
    }

    fn caller_unit(calls: &[&str]) -> qcore_c_gateway::legacy_c::PreparedTranslationUnit {
        let source = format!(
            "{}void caller(void) {{ {} }}\n",
            calls
                .iter()
                .map(|name| format!("void {name}(void); "))
                .collect::<String>(),
            calls
                .iter()
                .map(|name| format!("{name}(); "))
                .collect::<String>(),
        );
        prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        })
    }

    #[test]
    fn a_caller_holds_and_replays_only_the_homes_it_may_reach() {
        use qcore_c_gateway::legacy_c::{
            UnrestrictedLegacyResult, analyze_prepared_translation_unit_with_cross_unit,
            check_legacy_safety_certificate, cross_unit_for_caller,
        };
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let (environment, homes) = exported_homes(3);
        let caller = caller_unit(&["f0"]);
        let source = caller.preprocessed_source().unwrap();
        let scoped = cross_unit_for_caller(source, &environment, &homes);
        assert_eq!(scoped.reachable, 1);
        assert!(scoped.withheld.is_none());
        assert_eq!(
            scoped
                .homes
                .iter()
                .map(|home| home.function.as_str())
                .collect::<Vec<_>>(),
            ["f0"]
        );
        assert_eq!(
            scoped
                .environment
                .inferred_imports()
                .keys()
                .collect::<Vec<_>>(),
            ["f0"]
        );
        assert_eq!(
            scoped
                .environment
                .cross_unit_import_identities()
                .keys()
                .collect::<Vec<_>>(),
            ["f0"]
        );
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &scoped.environment,
            &scoped.homes,
        )
        .unwrap();
        let UnrestrictedLegacyResult::Safe(certificate) = result else {
            panic!("the reachable import was lost: {result:?}");
        };
        assert_eq!(certificate.cross_unit_homes.len(), 1);
        check_legacy_safety_certificate(&certificate).unwrap();
        // A function the run's member facts name as a value is reachable
        // though the caller never spells it.
        let mut facts = qcore_c_gateway::callback_member_closure::MemberFacts::default();
        facts.stores.insert(
            "T.cb".into(),
            BTreeSet::from([qcore_c_gateway::callback_member_closure::Flow::Function {
                name: "f2".into(),
            }]),
        );
        // An address-taken function no store or setter actual names is no
        // closure's candidate.
        facts.address_taken.insert("f1".into());
        let run = std::sync::Arc::new(qcore_c_gateway::member_run_facts::RunMemberFacts::new(
            vec![facts],
            BTreeMap::new(),
        ));
        let named = qcore_c_gateway::member_run_facts::scoped(Some(run), || {
            cross_unit_for_caller(source, &environment, &homes)
        });
        assert_eq!(
            named
                .homes
                .iter()
                .map(|home| home.function.as_str())
                .collect::<Vec<_>>(),
            ["f0", "f2"]
        );
    }

    #[test]
    fn a_caller_reaching_more_homes_than_the_budget_holds_none() {
        use qcore_c_gateway::legacy_c::{
            CROSS_UNIT_REPLAY_BUDGET_PER_UNIT, UnrestrictedLegacyResult,
            analyze_prepared_translation_unit_with_cross_unit, cross_unit_for_caller,
        };
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let count = CROSS_UNIT_REPLAY_BUDGET_PER_UNIT + 1;
        let (environment, homes) = exported_homes(count);
        let names = (0..count)
            .map(|index| format!("f{index}"))
            .collect::<Vec<_>>();
        let calls = names.iter().map(String::as_str).collect::<Vec<_>>();
        let caller = caller_unit(&calls);
        let source = caller.preprocessed_source().unwrap();
        let scoped = cross_unit_for_caller(source, &environment, &homes);
        assert_eq!(scoped.reachable, count);
        assert!(scoped.homes.is_empty());
        assert!(scoped.environment.inferred_imports().is_empty());
        assert!(scoped.environment.cross_unit_import_identities().is_empty());
        assert!(
            scoped
                .withheld
                .as_deref()
                .is_some_and(|reason| reason.contains("replay budget exhausted"))
        );
        // Exhaustion is Unknown: every call stays uncontracted.
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &scoped.environment,
            &scoped.homes,
        )
        .unwrap();
        assert!(
            matches!(result, UnrestrictedLegacyResult::Unknown { .. }),
            "{result:?}"
        );
        // The check refuses a home set above the budget before any replay.
        let refused = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &environment,
            &homes,
        )
        .unwrap_err()
        .to_string();
        assert!(refused.contains("per-unit replay budget"), "{refused}");
    }

    #[test]
    fn selected_lifetime_home_preserves_effects_through_caller_certificate_replay() {
        use qcore_c_gateway::legacy_c::{
            UnrestrictedLegacyResult, analyze_prepared_translation_unit_with_cross_unit,
            check_legacy_safety_certificate,
        };
        let base = qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "lifetime.qv".into(),
            r#"module contract;
extern fn ev_add(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(loop); ensures normal resource_borrowed(event); ensures normal resource_retained_by(event, loop); ensures normal resource_return_none(); };
extern fn protocol_host(c: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(c); ensures normal resource_return_none(); quant registered #c: collector.event |-> collector.evl; };
"#.into(),
        )])).unwrap();
        let map = LinkMap {
            digest: "a".repeat(64),
            premise: "External starts resolves to lib/home.c".into(),
            homes: BTreeMap::from([("starts".into(), "lib/home.c".into())]),
        };
        let prelude = "struct loop { int fd; }; struct event { int state; }; struct collector { struct loop *evl; struct event event; }; int ev_add(struct loop *, void *);";
        let source = format!(
            "{prelude}\nvoid starts(struct collector *c) {{ ev_add(c->evl, &c->event); }}\n"
        );
        let home = prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let export = export_cross_unit_homes(&home, &base, &map.requested(), &map.digest)
            .unwrap()
            .pop()
            .expect("effect-bearing home");
        let (environment, homes) = map.select(
            [("lib/home.c".into(), export)],
            [("lib/home.c".into(), "starts".into())],
            &base,
        );
        assert!(environment.inferred_imports().is_empty());
        assert_eq!(environment.lifetime_imports()["starts"].effects.len(), 1);
        let caller_source = format!(
            "{prelude}\nvoid starts(struct collector *); void caller(struct collector *c) {{ starts(c); }}\n"
        );
        let caller = prepare_translation_unit(&C11TranslationUnit {
            source: &caller_source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let result = analyze_prepared_translation_unit_with_cross_unit(
            &caller,
            "caller",
            &environment,
            &homes,
        )
        .unwrap();
        let UnrestrictedLegacyResult::Safe(certificate) = result else {
            panic!("selected lifetime import was lost: {result:?}");
        };
        assert_eq!(
            certificate.summary.functions["caller"]
                .lifetime_effects
                .len(),
            1
        );
        check_legacy_safety_certificate(&certificate).unwrap();
    }

    // ---- Card LINK-STANDING (rule 40) ----------------------------------------

    #[test]
    fn standing_vectors_hold() {
        for (failing, expected) in STANDING_VECTORS {
            assert_eq!(
                decide(Facts::failing(failing)),
                expected.map(|index| Refusal::ALL[index]),
                "{failing:?}"
            );
        }
        assert_eq!(decide(Facts::ALL), None);
    }

    /// A link-resolution artifact over real files in a fresh directory:
    /// the bound files exist with the recorded digests, and the database is
    /// `database`.
    struct ArtifactFixture {
        dir: std::path::PathBuf,
        path: std::path::PathBuf,
        database_sha256: String,
    }

    impl Drop for ArtifactFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn identity(file: &str, command: &str) -> ConfigurationIdentity {
        ConfigurationIdentity {
            file: format!("/src/{file}"),
            directory: "/build".into(),
            command_sha256: qcore_schemas::sha256_hex(command.as_bytes()),
        }
    }

    /// The artifact's configurations: 0 home.c, 1 caller.c, 2 other.c (not
    /// linked), 3 amb.c (the linked one of two route configurations under
    /// one key), 4 elsewhere.c (linked, but the route analyses another
    /// configuration of it). Card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS
    /// adds three two-configuration keys: 5/6 two.c (static linked, `-fPIC`
    /// recorded and attributed to no input: outside the link), 7/8 chosen.c
    /// (static linked, `-fPIC` unlinked but attributed, with 9 twin.c, to an
    /// input whose bytes match both: the link may have used it), 10/11
    /// both.c (both linked).
    fn configurations() -> Vec<(ConfigurationIdentity, bool)> {
        vec![
            (identity("lib/home.c", "cc -c home.c"), true),
            (identity("lib/caller.c", "cc -c caller.c"), true),
            (identity("lib/other.c", "cc -c other.c"), false),
            (identity("lib/amb.c", "cc -c amb.c"), true),
            (identity("lib/elsewhere.c", "cc -c elsewhere.c"), true),
            (identity("lib/two.c", "cc -c two.c"), true),
            (identity("lib/two.c", "cc -fPIC -c two.c"), false),
            (identity("lib/chosen.c", "cc -c chosen.c"), true),
            (identity("lib/chosen.c", "cc -fPIC -c chosen.c"), false),
            (identity("lib/twin.c", "cc -c twin.c"), false),
            (identity("lib/both.c", "cc -c both.c"), true),
            (identity("lib/both.c", "cc -fPIC -c both.c"), true),
        ]
    }

    /// The link inputs (label, configurations whose object has its bytes).
    fn inputs() -> Vec<(&'static str, Vec<usize>)> {
        vec![
            ("home.o", vec![0]),
            ("two.o", vec![5]),
            ("chosen.o", vec![7]),
            ("twin.o", vec![8, 9]),
            ("both.o", vec![10]),
            ("both.pic.o", vec![11]),
        ]
    }

    /// The route configurations of the three two-configuration keys.
    fn two_configuration_route() -> Vec<(String, ConfigurationIdentity)> {
        ["two", "chosen", "both"]
            .into_iter()
            .flat_map(|name| {
                let key = format!("lib/{name}.c");
                [
                    (key.clone(), identity(&key, &format!("cc -c {name}.c"))),
                    (
                        key.clone(),
                        identity(&key, &format!("cc -fPIC -c {name}.c")),
                    ),
                ]
            })
            .collect()
    }

    /// The route's configurations, keyed as the route keys them.
    fn route() -> Vec<(String, ConfigurationIdentity)> {
        vec![
            ("lib/home.c".into(), identity("lib/home.c", "cc -c home.c")),
            (
                "lib/caller.c".into(),
                identity("lib/caller.c", "cc -c caller.c"),
            ),
            (
                "lib/other.c".into(),
                identity("lib/other.c", "cc -c other.c"),
            ),
            ("lib/amb.c".into(), identity("lib/amb.c", "cc -c amb.c")),
            (
                "lib/amb.c".into(),
                identity("lib/amb.c", "cc -fPIC -c amb.c"),
            ),
            (
                "lib/elsewhere.c".into(),
                identity("lib/elsewhere.c", "cc -DLIBEVENT -c elsewhere.c"),
            ),
        ]
    }

    /// One symbol record as `prepare --link` writes it.
    #[allow(clippy::too_many_arguments)]
    fn symbol(
        name: &str,
        configuration: Option<usize>,
        defined_in: &str,
        binding: &str,
        visibility: &str,
        definitions: usize,
        relocations: u64,
        reason: Option<&str>,
    ) -> Value {
        serde_json::json!({
            "name": name,
            "kind": "function",
            "definition": {
                "file": "x.o", "in": defined_in, "input": 0, "configuration": configuration,
                "binding": binding, "visibility": visibility, "type": "function",
            },
            "definitions": (0..definitions).map(|id| serde_json::json!({"input": id, "binding": binding})).collect::<Vec<_>>(),
            "referenced_by": [1],
            "executable": {
                "entries": [], "in_definer_text": defined_in == "executable",
                "exported": false, "dynamic_relocations": relocations,
            },
            "home_unit": Value::Null,
            "decision": if reason.is_some() { "refused" } else { "derived" },
            "reason": reason,
            "detail": "",
        })
    }

    fn strong(name: &str, configuration: usize) -> Value {
        symbol(
            name,
            Some(configuration),
            "executable",
            "global",
            "default",
            1,
            0,
            None,
        )
    }

    fn artifact(symbols: Vec<Value>) -> ArtifactFixture {
        artifact_with(symbols, true)
    }

    /// `attribution: false` drops the last input's configuration list.
    fn artifact_with(symbols: Vec<Value>, attribution: bool) -> ArtifactFixture {
        let dir = std::env::temp_dir().join(format!(
            "quirin-link-standing-{}-{}",
            std::process::id(),
            qcore_schemas::sha256_hex(format!("{:?}", std::time::Instant::now()).as_bytes())
        ));
        fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, body: &[u8]| {
            fs::write(dir.join(name), body).unwrap();
            (
                dir.join(name).display().to_string(),
                qcore_schemas::sha256_hex(body),
            )
        };
        let (executable, executable_sha256) = file("fluent-bit", b"executable");
        let (link_txt, link_txt_sha256) = file("link.txt", b"clang++ -o fluent-bit home.o");
        let (map, map_sha256) = file("fluent-bit.map", b"map");
        let (driver, driver_sha256) = file("clang", b"driver");
        let (linker, linker_sha256) = file("ld", b"linker");
        let all = inputs();
        let last = all.len() - 1;
        let inputs = all
            .into_iter()
            .enumerate()
            .map(|(id, (label, configurations))| {
                let body: &[u8] = if label == "home.o" {
                    b"object"
                } else {
                    label.as_bytes()
                };
                let (_, sha256) = file(label, body);
                let mut input = serde_json::json!({"id": id, "label": label, "sha256": sha256});
                if attribution || id != last {
                    input["configurations"] = serde_json::json!(configurations);
                }
                input
            })
            .collect::<Vec<_>>();
        let database = b"[]";
        let database_sha256 = qcore_schemas::sha256_hex(database);
        let artifact = serde_json::json!({
            "schema": crate::link_resolution::ARTIFACT_SCHEMA,
            "executable": {"path": executable, "sha256": executable_sha256,
                "image": {"is_executable": true}},
            "link": {"link_txt": link_txt, "link_txt_sha256": link_txt_sha256,
                "map": map, "map_sha256": map_sha256, "directory": dir},
            "toolchain": {
                "driver": {"path": driver, "resolved": driver, "sha256": driver_sha256},
                "linker": {"path": linker, "resolved": linker, "sha256": linker_sha256},
            },
            "compile_database": {"path": "compile_commands.json", "sha256": database_sha256},
            "inputs": inputs,
            "configurations": configurations().iter().enumerate().map(|(id, (identity, linked))| serde_json::json!({
                "id": id, "file": identity.file, "directory": identity.directory,
                "command_sha256": identity.command_sha256, "linked": linked,
                "reproduction": if *linked { serde_json::json!("equal") } else { Value::Null },
            })).collect::<Vec<_>>(),
            "symbols": symbols,
        });
        let path = dir.join("link-resolution.json");
        fs::write(&path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
        ArtifactFixture {
            dir,
            path,
            database_sha256,
        }
    }

    /// Every home-level condition: one strong default-visibility home is
    /// derived under the route's own key; each other symbol is refused by
    /// the general condition it fails, including a home whose analysed
    /// configuration is not the linked one.
    #[test]
    fn a_derived_resolution_decides_every_home_condition() {
        let fixture = artifact(vec![
            strong("callee", 0),
            symbol(
                "shared",
                None,
                "shared_object",
                "global",
                "default",
                0,
                1,
                Some("defined in a shared object"),
            ),
            symbol(
                "weak_home",
                Some(0),
                "executable",
                "weak",
                "default",
                1,
                0,
                Some("weak definition"),
            ),
            symbol(
                "hidden_home",
                Some(0),
                "executable",
                "global",
                "hidden",
                1,
                0,
                None,
            ),
            symbol(
                "twice",
                Some(0),
                "executable",
                "global",
                "default",
                2,
                0,
                Some("more than one definition among the linked objects"),
            ),
            symbol(
                "interposed",
                Some(0),
                "executable",
                "global",
                "default",
                1,
                2,
                Some("the executable resolves it at run time (dynamic relocation)"),
            ),
            symbol(
                "unreproduced",
                Some(0),
                "executable",
                "global",
                "default",
                1,
                0,
                Some("home object is not reproduced by its compile database entry"),
            ),
            strong("in_unlinked", 2),
            symbol(
                "ambiguous",
                Some(3),
                "executable",
                "global",
                "default",
                1,
                0,
                Some("home unit key names more than one route configuration"),
            ),
            symbol(
                "elsewhere",
                Some(4),
                "executable",
                "global",
                "default",
                1,
                0,
                Some("home configuration is not a unit of the current route"),
            ),
        ]);
        let resolution = DerivedResolution::load(&fixture.path).unwrap();
        resolution.verify(&fixture.database_sha256).unwrap();
        let derived = resolution.derive(&route());
        assert_eq!(
            derived.map.requested(),
            BTreeSet::from(["callee".to_owned()])
        );
        assert_eq!(derived.map.home_for("callee"), Some("lib/home.c"));
        assert_eq!(derived.map.digest, resolution.artifact_sha256);
        assert!(!derived.map.premise.contains("not established"));
        assert_eq!(
            derived.refused,
            BTreeMap::from([
                (Refusal::NotInExecutable, 1),
                (Refusal::NotStrong, 1),
                (Refusal::NotDefaultVisibility, 1),
                (Refusal::Duplicate, 1),
                (Refusal::DynamicRelocation, 1),
                (Refusal::FactsRefused, 1),
                (Refusal::HomeUnlinked, 1),
                (Refusal::NotRouteUnit, 1),
                (Refusal::AmbiguousKey, 1),
            ])
        );
        assert!(
            derived
                .standing
                .caller_linked(&identity("lib/caller.c", "cc -c caller.c"))
        );
        assert!(
            !derived
                .standing
                .caller_linked(&identity("lib/other.c", "cc -c other.c"))
        );
        // The analysed configuration of a file that differs from the linked
        // one is not linked, as a caller either.
        assert!(
            !derived
                .standing
                .caller_linked(&identity("lib/elsewhere.c", "cc -DLIBEVENT -c elsewhere.c"))
        );
        assert!(derived.standing.record("callee").is_some());
    }

    #[test]
    fn outside_vectors_hold() {
        for &(linked, attributed, expected) in OUTSIDE_VECTORS {
            assert_eq!(
                shown_outside(linked, attributed),
                expected,
                "{linked:?} {attributed:?}"
            );
        }
    }

    /// Card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS: condition 13 is
    /// decided over the configurations the link can use. A second
    /// configuration under the home's key that the artifact records and
    /// attributes to no link input leaves the key to the linked one, and the
    /// census leaves it out. One the link may have used (attributed, even
    /// unlinked; linked) or one the artifact does not record keeps the
    /// refusal.
    #[test]
    fn an_unlinked_second_configuration_leaves_the_key_to_the_linked_one() {
        let homes = || {
            vec![
                strong("two_home", 5),
                strong("chosen_home", 7),
                strong("both_home", 10),
                symbol(
                    "amb_home",
                    Some(3),
                    "executable",
                    "global",
                    "default",
                    1,
                    0,
                    Some("home unit key names more than one route configuration"),
                ),
            ]
        };
        let mut route = route();
        route.extend(two_configuration_route());
        let fixture = artifact(homes());
        let resolution = DerivedResolution::load(&fixture.path).unwrap();
        resolution.verify(&fixture.database_sha256).unwrap();
        let derived = resolution.derive(&route);
        assert_eq!(
            derived.map.requested(),
            BTreeSet::from(["two_home".to_owned()])
        );
        assert_eq!(derived.map.home_for("two_home"), Some("lib/two.c"));
        // chosen.c's `-fPIC` object may be the one linked; both.c's is;
        // amb.c's `-fPIC` configuration is not recorded by the artifact.
        assert_eq!(
            derived.refused,
            BTreeMap::from([(Refusal::AmbiguousKey, 3)])
        );
        let two_pic = identity("lib/two.c", "cc -fPIC -c two.c");
        assert!(derived.outside_the_link(&two_pic));
        for kept in [
            identity("lib/two.c", "cc -c two.c"),
            identity("lib/chosen.c", "cc -fPIC -c chosen.c"),
            identity("lib/both.c", "cc -fPIC -c both.c"),
            identity("lib/amb.c", "cc -fPIC -c amb.c"),
            identity("lib/caller.c", "cc -c caller.c"),
        ] {
            assert!(!derived.outside_the_link(&kept), "{kept:?}");
        }
        // The outside configuration is not a linked caller either.
        assert!(!derived.standing.caller_linked(&two_pic));
        assert!(
            derived
                .standing
                .caller_linked(&identity("lib/two.c", "cc -c two.c"))
        );

        // One input without its configuration list shows nothing outside
        // the link: the key is refused again (fail-closed).
        let fixture = artifact_with(homes(), false);
        let resolution = DerivedResolution::load(&fixture.path).unwrap();
        let derived = resolution.derive(&route);
        assert!(derived.map.requested().is_empty());
        assert_eq!(
            derived.refused,
            BTreeMap::from([(Refusal::AmbiguousKey, 4)])
        );
        assert!(derived.outside.is_empty());
    }

    /// The run-level conditions: another database, or a bound file whose
    /// bytes changed, refuses the whole artifact.
    #[test]
    fn a_changed_database_or_bound_file_refuses_the_artifact() {
        let fixture = artifact(vec![strong("callee", 0)]);
        let resolution = DerivedResolution::load(&fixture.path).unwrap();
        let (refusal, _) = resolution.verify(&"0".repeat(64)).unwrap_err();
        assert_eq!(refusal, Refusal::DatabaseDiffers);
        fs::write(fixture.dir.join("home.o"), b"rebuilt").unwrap();
        let (refusal, detail) = resolution.verify(&fixture.database_sha256).unwrap_err();
        assert_eq!(refusal, Refusal::DigestUnverified);
        assert!(detail.starts_with("home.o"), "{detail}");
        fs::write(fixture.dir.join("home.o"), b"object").unwrap();
        resolution.verify(&fixture.database_sha256).unwrap();
        fs::write(fixture.dir.join("fluent-bit"), b"relinked").unwrap();
        let (refusal, _) = resolution.verify(&fixture.database_sha256).unwrap_err();
        assert_eq!(refusal, Refusal::DigestUnverified);
    }

    fn prepared(source: &str) -> qcore_c_gateway::legacy_c::PreparedTranslationUnit {
        prepare_translation_unit(&C11TranslationUnit {
            source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        })
    }

    /// The derived homes of `names`, all in `lib/home.c`, exported from
    /// `home` and selected under the derived map.
    fn derived_homes(
        fixture: &ArtifactFixture,
        home: &str,
    ) -> (
        Derived,
        ResourceContracts,
        Vec<qcore_c_gateway::legacy_c::CrossUnitHomeEvidence>,
    ) {
        let base = ResourceContracts::default();
        let resolution = DerivedResolution::load(&fixture.path).unwrap();
        resolution.verify(&fixture.database_sha256).unwrap();
        let derived = resolution.derive(&route());
        let home = prepared(home);
        let names = home.inventory().definitions().to_vec();
        let exports =
            export_cross_unit_homes(&home, &base, &derived.map.requested(), &derived.map.digest)
                .expect("export");
        let (environment, homes) = derived.map.select(
            exports
                .into_iter()
                .map(|export| ("lib/home.c".to_owned(), export)),
            names
                .into_iter()
                .map(|name| ("lib/home.c".to_owned(), name)),
            &base,
        );
        (derived, environment, homes)
    }

    fn analyse(
        caller: &qcore_c_gateway::legacy_c::PreparedTranslationUnit,
        derived: &Derived,
        linked: bool,
        environment: &ResourceContracts,
        homes: &[qcore_c_gateway::legacy_c::CrossUnitHomeEvidence],
    ) -> (
        qcore_c_gateway::legacy_c::UnrestrictedLegacyResult,
        Vec<(String, String)>,
    ) {
        use qcore_c_gateway::legacy_c::{
            analyze_prepared_translation_unit_with_cross_unit, cross_unit_for_caller,
        };
        let scoped =
            cross_unit_for_caller(caller.preprocessed_source().unwrap(), environment, homes);
        let (scoped, refusals) = derived.standing.admit(scoped, linked, caller);
        let result = analyze_prepared_translation_unit_with_cross_unit(
            caller,
            "caller",
            &scoped.environment,
            &scoped.homes,
        )
        .unwrap();
        (result, refusals)
    }

    const RECORD_HOME: &str =
        "struct s { int a; };\ntypedef struct s s_t;\nvoid callee(s_t *p) { return; }\n";

    /// The derived home is adopted for a linked caller whose declaration in
    /// force has the home definition's type (a typedef spelling and an equal
    /// record layout): the caller is Safe and its certificate replays the
    /// home. An unlinked caller, an unequal call type, an unequal record
    /// layout and a missing declaration each refuse the home: the call stays
    /// uncontracted and the row Unknown.
    #[test]
    fn a_derived_home_is_held_only_by_a_linked_caller_with_an_equal_type() {
        use qcore_c_gateway::legacy_c::{
            UnrestrictedLegacyResult, check_legacy_safety_certificate,
        };
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let fixture = artifact(vec![strong("callee", 0)]);
        let (derived, environment, homes) = derived_homes(&fixture, RECORD_HOME);
        assert_eq!(homes.len(), 1, "the home was not exported");
        let caller = prepared(
            "struct s { int a; };\nvoid callee(struct s *);\nvoid caller(struct s *p) { callee(p); }\n",
        );
        let (result, refusals) = analyse(&caller, &derived, true, &environment, &homes);
        assert!(refusals.is_empty(), "{refusals:?}");
        let UnrestrictedLegacyResult::Safe(certificate) = result else {
            panic!("the derived import was lost: {result:?}");
        };
        assert_eq!(certificate.cross_unit_homes.len(), 1);
        check_legacy_safety_certificate(&certificate).unwrap();

        let unknown = |result: &UnrestrictedLegacyResult| {
            matches!(result, UnrestrictedLegacyResult::Unknown { .. })
        };
        let (result, refusals) = analyse(&caller, &derived, false, &environment, &homes);
        assert!(unknown(&result), "{result:?}");
        assert_eq!(
            refusals,
            [(
                "callee".to_owned(),
                Refusal::CallerUnlinked.phrase().to_owned()
            )]
        );

        let mismatched = prepared(
            "struct s { int a; };\nvoid callee(long *);\nvoid caller(struct s *p) { callee((long *)p); }\n",
        );
        let (result, refusals) = analyse(&mismatched, &derived, true, &environment, &homes);
        assert!(unknown(&result), "{result:?}");
        assert!(
            refusals[0].1.starts_with(Refusal::TypeDiffers.phrase()),
            "{refusals:?}"
        );

        let relaid = prepared(
            "struct s { long a; long b; };\nvoid callee(struct s *);\nvoid caller(struct s *p) { callee(p); }\n",
        );
        let (result, refusals) = analyse(&relaid, &derived, true, &environment, &homes);
        assert!(unknown(&result), "{result:?}");
        assert!(refusals[0].1.contains("different layouts"), "{refusals:?}");

        let block_scope = prepared(
            "struct s { int a; };\nvoid caller(struct s *p) { void callee(struct s *); callee(p); }\n",
        );
        let (result, refusals) = analyse(&block_scope, &derived, true, &environment, &homes);
        assert!(unknown(&result), "{result:?}");
        assert!(
            refusals[0].1.starts_with(Refusal::TypeUnavailable.phrase()),
            "{refusals:?}"
        );
    }

    /// Rule 4c's replay budget is unchanged under a derived resolution: a
    /// caller reaching more derived homes than the budget holds none, and
    /// rule 40 refuses nothing further.
    #[test]
    fn a_caller_over_the_budget_holds_no_derived_home() {
        use qcore_c_gateway::legacy_c::{
            CROSS_UNIT_REPLAY_BUDGET_PER_UNIT, UnrestrictedLegacyResult,
        };
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        let count = CROSS_UNIT_REPLAY_BUDGET_PER_UNIT + 1;
        let names = (0..count)
            .map(|index| format!("f{index}"))
            .collect::<Vec<_>>();
        let fixture = artifact(names.iter().map(|name| strong(name, 0)).collect());
        let home = names
            .iter()
            .map(|name| format!("void {name}(void) {{ return; }}\n"))
            .collect::<String>();
        let (derived, environment, homes) = derived_homes(&fixture, &home);
        assert_eq!(homes.len(), count);
        let calls = names.iter().map(String::as_str).collect::<Vec<_>>();
        let caller = caller_unit(&calls);
        let scoped = qcore_c_gateway::legacy_c::cross_unit_for_caller(
            caller.preprocessed_source().unwrap(),
            &environment,
            &homes,
        );
        assert!(scoped.withheld.is_some());
        let (result, refusals) = analyse(&caller, &derived, true, &environment, &homes);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(
            matches!(result, UnrestrictedLegacyResult::Unknown { .. }),
            "{result:?}"
        );
    }
}
