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
}
