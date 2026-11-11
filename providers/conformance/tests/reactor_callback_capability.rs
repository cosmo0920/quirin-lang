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

//! Real core-held program -> packaged Libevent process -> trusted model replay
//! for the reactor callback capability.
#![cfg(unix)]
#[path = "../../../crates/qcore-verification/tests/fixtures/reactor_program.rs"]
mod fixture;
use fixture::*;

use qcore_core::event_reactor::{self as model, EventOperation};
use qcore_schemas::reactor_program::{REACTOR_PROGRAM_INPUT_KEY, ReactorCallbackWitness};
use qcore_schemas::{PluginId, ProposedVerdict, SemanticWitness};
use qcore_verification::reactor_source::*;
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::collections::BTreeMap;
use std::process::Command;
use std::sync::Arc;

fn plugin() -> PluginId {
    PluginId::new("qcore.libevent").unwrap()
}

fn identity() -> ReactorProgramIdentity {
    ReactorProgramIdentity {
        library: "libevent".into(),
        source_revision: "pinned-source".into(),
        build_profile: BTreeMap::from([("backend".into(), "epoll".into())]),
        profile_assumptions: BTreeMap::from([("library".into(), "libevent".into())]),
    }
}

fn target() -> qcore_schemas::plugin::TargetProfile {
    qcore_schemas::plugin::TargetProfile {
        library: "libevent".into(),
        revision: "pinned-source".into(),
        profile: REACTOR_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: "linux".into(),
        backend: "native".into(),
    }
}

fn task_for(program: &model::EventProgram<TestProfile>) -> ReactorCallbackTask<TestProfile> {
    let import = import_reactor_program(program, &[TestProfile(true)], identity())
        .expect("the admitted family imports");
    ReactorCallbackTask::derive(import, "reactor.c", target()).expect("derives")
}

#[test]
fn the_real_package_proposes_callback_inventories_without_granting_authority() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output = std::env::temp_dir().join(format!(
        "qcore-reactor-callback-package-{}",
        std::process::id()
    ));
    let built = Command::new("python3")
        .arg(root.join("package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(root.join("target/conformance-artifacts"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.libevent")).unwrap(),
        ))
        .unwrap();

    // Positive: the process reads the transported program on its own and the
    // core admits it only after running its own reactor model checker.
    let program = one_shot(1, key(1), 9);
    let replay = task_for(&program);
    let request = replay.request(plugin());
    let result = registry.verify_v2(&request, replay.task(), Some(&replay));
    assert_eq!(
        result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        result.diagnostics
    );
    let certificate = result.certificate.unwrap();
    assert_eq!(certificate.task(), Some(replay.task()));
    assert_eq!(certificate.witness().unwrap().kind(), "reactor_callback");
    let Some(SemanticWitness::ReactorCallback(witness)) = certificate.witness() else {
        panic!("wrong witness kind")
    };
    // Descriptor generation, registration index and parent custody survive.
    assert_eq!(witness.proposal.callbacks.len(), 1);
    assert_eq!(witness.proposal.callbacks[0].operation_index, 1);
    assert_eq!(witness.proposal.callbacks[0].event.id, 1);
    assert_eq!(witness.proposal.callbacks[0].event.generation, 1);
    assert_eq!(witness.proposal.callbacks[0].summary.argument, 9);

    // The request carries the program only; no inventory travels with it.
    let input = serde_json::to_value(request.reactor_program.as_ref().unwrap()).unwrap();
    assert!(input.get("callbacks").is_none() && input.get("proposal").is_none());
    assert!(input.get("operations").is_some());

    // A second, distinct registration is read independently at its own index.
    let mut two = one_shot(1, key(1), 9);
    let second = model::EventKey::new(2, 1);
    let assignment = match &two.operations[1] {
        EventOperation::Assign {
            base,
            descriptor,
            flags,
            priority,
            summary,
            ..
        } => EventOperation::Assign {
            event: second,
            base: *base,
            descriptor: Some(descriptor.unwrap_or(3) + 1),
            flags: *flags,
            priority: *priority,
            summary: summary.clone(),
        },
        other => other.clone(),
    };
    two.operations.insert(2, assignment);
    two.operations
        .insert(3, EventOperation::Add { event: second });
    two.operations.insert(
        two.operations.len() - 1,
        EventOperation::FreeEvent { event: second },
    );
    let two_task = task_for(&two);
    let response = registry.verify_v2(
        &two_task.request(plugin()),
        two_task.task(),
        Some(&two_task),
    );
    // Whether this program checks is the model's decision, not the process's.
    // What is pinned here is that the process reads two registrations.
    if let Some(SemanticWitness::ReactorCallback(witness)) =
        response.certificate.as_ref().and_then(|c| c.witness())
    {
        assert_eq!(witness.proposal.callbacks.len(), 2);
        assert_eq!(witness.proposal.callbacks[1].event, {
            qcore_schemas::reactor::EventKeyRecord {
                id: 2,
                generation: 1,
            }
        });
    } else {
        assert_eq!(response.verdict, ProposedVerdict::Unknown);
    }

    // Missing evidence: no core checker, no registered provider and a package
    // that does not implement this capability each stay Unknown, no certificate.
    let no_checker = registry.verify_v2(&request, replay.task(), None);
    assert_eq!(no_checker.verdict, ProposedVerdict::Unknown);
    assert!(no_checker.diagnostics.iter().any(|d| d.code == "QP016"));
    assert!(no_checker.certificate.is_none());
    assert_eq!(
        VerifierRegistry::default()
            .verify_v2(&request, replay.task(), Some(&replay))
            .verdict,
        ProposedVerdict::Unknown
    );
    let mut others = VerifierRegistry::default();
    others
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.chunkio")).unwrap(),
        ))
        .unwrap();
    let mut foreign_request = request.clone();
    foreign_request.plugin_id = PluginId::new("qcore.chunkio").unwrap();
    let foreign = others.verify_v2(&foreign_request, replay.task(), Some(&replay));
    assert_eq!(foreign.verdict, ProposedVerdict::Unknown);
    assert!(foreign.certificate.is_none());

    // Tampered transport: a well-formed program the core never imported. The
    // process reads it faithfully and proposes an inventory for *that* program;
    // the core refuses it against its own assignments.
    let mut forged = request.clone();
    let projection = forged.reactor_program.as_mut().unwrap();
    let json = serde_json::to_string(&projection.operations)
        .unwrap()
        .replace("\"generation\":1", "\"generation\":2");
    projection.operations = serde_json::from_str(&json).unwrap();
    let rejection = registry.verify_v2(&forged, replay.task(), Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(
        rejection.diagnostics.iter().any(|d| d.code == "QP001"),
        "{:?}",
        rejection.diagnostics
    );
    // Rebinding the forged projection to a recomputed task digest gets past the
    // host's digest binding and is then refused by the core's own comparison.
    let mut forged_task = replay.task().clone();
    forged_task.contract_sources.insert(
        REACTOR_PROGRAM_INPUT_KEY.into(),
        forged.reactor_program.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.certificate.is_none());

    // A program the process reads correctly still cannot buy a refused model
    // run: the core evaluates `check_event_program` over its own program.
    let mut unauthorized = one_shot(1, key(1), 9);
    if let EventOperation::Assign { summary, .. } = &mut unauthorized.operations[1] {
        summary.payload_authority.clear();
    }
    let refused = task_for(&unauthorized);
    assert!(refused.evaluate().is_err(), "the model must refuse it");
    let rejection = registry.verify_v2(&refused.request(plugin()), refused.task(), Some(&refused));
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "a Safe proposal cannot bypass the model run"
    );
    assert!(rejection.certificate.is_none());
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    // A changed program or changed declared identity retires the context.
    let mut relabelled = identity();
    relabelled
        .build_profile
        .insert("backend".into(), "poll".into());
    let other = ReactorCallbackTask::derive(
        import_reactor_program(&program, &[TestProfile(true)], relabelled).unwrap(),
        "reactor.c",
        target(),
    )
    .unwrap();
    let rejection = registry.verify_v2(&other.request(plugin()), other.task(), Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);

    // A disabled provider is Unknown, never an optimistic fallback.
    registry.set_enabled(&plugin(), false).unwrap();
    assert_eq!(
        registry
            .verify_v2(&request, replay.task(), Some(&replay))
            .verdict,
        ProposedVerdict::Unknown
    );
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn a_tampered_witness_from_a_fixture_process_is_refused_by_the_core() {
    // The transport is exercised above; this pins the firewall itself over a
    // matrix of forged witnesses, without rebuilding the package.
    let program = one_shot(1, key(1), 9);
    let task = task_for(&program);
    let base = task.declared_proposal();
    let mutations = [
        ("emptied inventory", {
            let mut value = base.clone();
            value.callbacks.clear();
            value
        }),
        ("retargeted generation", {
            let mut value = base.clone();
            value.callbacks[0].event.generation = 7;
            value
        }),
        ("widened authority", {
            let mut value = base.clone();
            value.callbacks[0].summary.may_spawn_tasks = true;
            value
        }),
        ("moved registration index", {
            let mut value = base.clone();
            value.callbacks[0].operation_index = 2;
            value
        }),
    ];
    for (label, proposal) in mutations {
        let witness = SemanticWitness::ReactorCallback(Box::new(ReactorCallbackWitness {
            task_digest: task.task().task_digest(),
            proposal,
        }));
        assert!(
            qcore_verifier_host::WitnessChecker::check(&task, task.task(), &witness).is_err(),
            "{label} must be refused"
        );
    }
}
