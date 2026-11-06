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
        facts.address_taken.insert("f2".into());
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
}
