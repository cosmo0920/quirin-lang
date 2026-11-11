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

//! P08 — the *profile* half of the bounded five-provider composition audit.
//!
//! `providers/conformance/tests/composed_five_providers.rs` audits the five
//! shipped executable packages. This file audits what the repository's own
//! profile data declares, because the two are not the same registry and the
//! difference is the whole reason packaging cannot promote anything:
//!
//! * the default first-party registry is assembled from **two** profile
//!   directories, and no single profile binds all five providers;
//! * every profile-declared provider is a built-in at `0.1.0` that admits
//!   **evidence version 1 only**, so every implemented evidence-v2 capability
//!   route is refused at `QP004` before a provider is ever consulted.
//!
//! Nothing here is a repair. Both facts are fail-closed, and both are recorded
//! in `docs/reports/remaining-p08/README.md` as measured boundaries.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use qcore_compiler::first_party::{load_first_party_manifests, load_first_party_registry};
use qcore_compiler::source_profiles::profiles_root;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, CallbackContext, CallbackSummary, EventFlags, EventKey,
    EventOperation, EventProgram, PayloadAccessKind, ReactorProfile,
};
use qcore_schemas::plugin::{PluginEntrypoint, TargetProfile};
use qcore_schemas::{PluginId, ProposedVerdict};
use qcore_verification::reactor_source::{
    REACTOR_SEMANTIC_PROFILE, ReactorCallbackTask, ReactorProgramIdentity, import_reactor_program,
};

/// The five packages `providers/` ships, in `PluginId` order.
const FIVE: [&str; 5] = [
    "qcore.chunkio",
    "qcore.cmetrics",
    "qcore.ctraces",
    "qcore.librdkafka",
    "qcore.libevent",
];

fn sorted_five() -> Vec<String> {
    let mut ids = FIVE.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    ids.sort();
    ids
}

#[test]
fn the_default_registry_is_five_built_in_providers_assembled_from_two_profiles() {
    let root = profiles_root();
    let manifests = load_first_party_manifests(&root).expect("the in-tree profiles load");
    let ids = manifests
        .iter()
        .map(|manifest| manifest.plugin_id.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        sorted_five(),
        "the default first-party registry is exactly the five shipped provider ids"
    );

    // Which profile directory declares which provider. No single profile binds
    // all five: the Fluent Bit composition profile declares four, and the
    // Libevent source set declares the fifth on its own.
    let mut directories = std::fs::read_dir(&root)
        .expect("profiles root is readable")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<PathBuf>>();
    directories.sort();
    let declared = directories
        .iter()
        .map(|directory| {
            let profile = load_profile_directory(directory).expect("profile loads");
            let ids = profile
                .providers()
                .iter()
                .map(|manifest| manifest.plugin_id.as_str().to_owned())
                .collect::<Vec<_>>();
            (
                directory
                    .file_name()
                    .expect("named directory")
                    .to_string_lossy()
                    .into_owned(),
                ids,
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        declared,
        BTreeMap::from([
            ("chunkio-core".to_owned(), Vec::new()),
            ("cmetrics-counter".to_owned(), Vec::new()),
            // Cycle 18 task-handoff is a source-only fixture profile; it
            // deliberately declares no provider and preserves the five-provider registry.
            ("cycle18-task-handoff".to_owned(), Vec::new()),
            (
                "fluent-bit".to_owned(),
                vec![
                    "qcore.chunkio".to_owned(),
                    "qcore.cmetrics".to_owned(),
                    "qcore.ctraces".to_owned(),
                    "qcore.librdkafka".to_owned(),
                ]
            ),
            ("libevent".to_owned(), vec!["qcore.libevent".to_owned()]),
            // G03-F installed the `libevent-core` resource-projection source
            // set. It binds no provider (`provider: null`), so it adds a row
            // here without changing the load-bearing fact below: still no
            // single profile declares all five.
            ("libevent-core".to_owned(), Vec::new()),
        ]),
        "no single profile declares the whole five-provider composition"
    );
    assert!(
        declared.values().all(|ids| ids.len() < 5),
        "no single profile may declare the whole five-provider composition"
    );

    // Every profile-declared provider is the built-in v1 adapter. The shipped
    // executable packages are at 0.5.0/0.2.0/0.2.0/0.1.0/0.1.0 with evidence
    // versions [1, 2] and real certificate checkers; the profile-declared
    // registry has moved with none of that.
    for manifest in &manifests {
        assert!(
            matches!(manifest.entrypoint, PluginEntrypoint::BuiltIn),
            "{} is not a built-in entry",
            manifest.plugin_id
        );
        assert_eq!(manifest.package_version, "0.1.0", "{}", manifest.plugin_id);
        assert_eq!(
            manifest.protocol_versions,
            BTreeSet::from([1]),
            "{}",
            manifest.plugin_id
        );
        assert_eq!(
            manifest.evidence_versions,
            BTreeSet::from([1]),
            "{} declares an evidence version the profile route cannot answer",
            manifest.plugin_id
        );
        assert_eq!(
            manifest.certificate_checker, "qcore.unsupported_assertions.v1",
            "{}",
            manifest.plugin_id
        );
        assert!(
            manifest.required_observation_hooks.is_empty(),
            "{} would need an observation identity the composed routes never mint",
            manifest.plugin_id
        );
        assert!(
            manifest.filesystem_inputs.is_empty() && !manifest.network_access,
            "{}",
            manifest.plugin_id
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Profile(bool);
impl ReactorProfile for Profile {
    fn threading_enabled(self) -> bool {
        self.0
    }
}

fn one_shot() -> EventProgram<Profile> {
    let event = EventKey::new(1, 1);
    let summary = CallbackSummary {
        argument: 9,
        argument_mode: ArgumentMode::Owned,
        trigger_reasons: BTreeSet::from([ActivationReason::Read]),
        payload_authority: BTreeMap::from([(7, PayloadAccessKind::Write)]),
        may_defer: false,
        may_spawn_tasks: false,
    };
    EventProgram {
        live_arguments: BTreeSet::from([9]),
        operations: vec![
            EventOperation::CreateBase {
                base: 1,
                profile: Profile(true),
            },
            EventOperation::Assign {
                event,
                base: 1,
                descriptor: Some(3),
                flags: EventFlags::new(EventFlags::READ),
                priority: 0,
                summary,
            },
            EventOperation::Add { event },
            EventOperation::BeginDispatch {
                base: 1,
                dispatcher: 10,
            },
            EventOperation::Activate {
                event,
                reason: ActivationReason::Read,
                cross_thread: false,
            },
            EventOperation::CallbackEnter {
                event,
                reason: ActivationReason::Read,
            },
            EventOperation::Access {
                context: CallbackContext::Event(event),
                payload: 7,
                kind: PayloadAccessKind::Write,
                synchronized: false,
            },
            EventOperation::CallbackExit { event },
            EventOperation::EndDispatch { base: 1 },
            EventOperation::FreeEvent { event },
            EventOperation::FreeBase { base: 1 },
        ],
    }
}

#[test]
fn an_implemented_v2_capability_is_refused_by_the_profile_declared_registry() {
    let registry =
        load_first_party_registry(&profiles_root()).expect("the in-tree profiles register");
    let program = one_shot();
    let import = import_reactor_program(
        &program,
        &[Profile(true)],
        ReactorProgramIdentity {
            library: "libevent".into(),
            source_revision: "pinned-source".into(),
            build_profile: BTreeMap::from([("backend".into(), "epoll".into())]),
            profile_assumptions: BTreeMap::from([("library".into(), "libevent".into())]),
        },
    )
    .expect("the admitted family imports");
    let task = ReactorCallbackTask::derive(
        import,
        "reactor.c",
        TargetProfile {
            library: "libevent".into(),
            revision: "pinned-source".into(),
            profile: REACTOR_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
    )
    .expect("derives");

    // The core's own model run accepts this program, so the refusal below is
    // the registry's, not the checker's.
    assert!(task.evaluate().is_ok());

    // Every one of the five is asked. The provider that *implements* this
    // capability as a package answers Unknown here too, because the profile
    // declares its built-in v1 adapter and evidence v2 is not in its manifest.
    for id in FIVE {
        let mut request = task.request(PluginId::new("qcore.libevent").unwrap());
        request.plugin_id = PluginId::new(id).unwrap();
        let outcome = registry.verify_v2(&request, task.task(), Some(&task));
        assert_eq!(
            outcome.verdict,
            ProposedVerdict::Unknown,
            "{id} must not verify through the profile-declared registry"
        );
        assert!(outcome.certificate.is_none(), "{id}");
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "QP004"),
            "{id}: {:?}",
            outcome.diagnostics
        );
    }

    // An id no profile declares is absent, not optimistically resolved.
    let mut absent = task.request(PluginId::new("qcore.libevent").unwrap());
    absent.plugin_id = PluginId::new("qcore.absent").unwrap();
    let outcome = registry.verify_v2(&absent, task.task(), Some(&task));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "QP002")
    );
}
