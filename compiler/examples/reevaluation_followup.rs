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

//! Follow-up audit on the post-containment implementation.
//! Prints diagnostic JSON; exits 1 if intended expectations fail.
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use qcore_compiler::weak_memory::{
    C11Execution, MemoryEvent, MemoryEventKind as K, RelationEdge as E, check_c11_execution,
};
use qcore_compiler::*;

// The provider scripts are written by a separate process
// (`tests/support/process_fixture.rs`): this probe forks only from its main
// thread today, but no write descriptor on an executable is held in-process.
#[cfg(unix)]
#[path = "../../tests/support/process_fixture.rs"]
mod process_fixture;

fn request() -> VerificationRequest {
    VerificationRequest {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION,
        request_id: "followup".into(),
        plugin_id: PluginId::new("qcore.libevent").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "libevent".into(),
            revision: "audit".into(),
            profile: "default".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "audit.c".into(),
        },
        facts: vec![],
        assumptions: BTreeMap::new(),
    }
}

fn response(req: &VerificationRequest, verdict: ProposedVerdict) -> VerificationResponse {
    VerificationResponse {
        protocol_version: req.protocol_version,
        request_id: req.request_id.clone(),
        plugin_id: req.plugin_id.clone(),
        verdict,
        diagnostics: vec![],
        evidence: None,
    }
}

struct CountingProvider {
    manifest: PluginManifest,
    calls: Arc<AtomicUsize>,
}
impl VerifierProvider for CountingProvider {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn verify(&self, req: &VerificationRequest) -> Result<VerificationResponse, ProviderFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(response(req, ProposedVerdict::Unsafe))
    }
}

fn event(id: u32, thread: u32, kind: K) -> MemoryEvent {
    MemoryEvent { id, thread, kind }
}

#[cfg(unix)]
fn process_probes(results: &mut Vec<serde_json::Value>) {
    use process_fixture::write_executable;
    use std::{
        fs,
        time::{Instant, SystemTime, UNIX_EPOCH},
    };
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package =
        std::env::temp_dir().join(format!("qcore-followup-{}-{unique}", std::process::id()));
    fs::create_dir(&package).unwrap();
    let script = package.join("provider.sh");
    let mut manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .expect("Libevent fixture manifest");
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider.sh".into(),
    };
    manifest.package_digest = "0".repeat(64);
    manifest.limits.timeout_ms = 20;
    write_executable(&script, b"#!/bin/sh\n/bin/sleep 0.4 &\nwait\n");
    let provider = UnverifiedProvider::new(manifest.clone(), &package).unwrap();
    let start = Instant::now();
    let outcome = provider.verify(&request());
    let elapsed_ms = start.elapsed().as_millis();
    results.push(
        serde_json::json!({"id": "VP04_descendant_holds_pipes", "configured_timeout_ms": 20,
        "elapsed_ms": elapsed_ms, "detail": format!("{outcome:?}"),
        "pass": elapsed_ms < 200, "note": "200 ms tolerance; bounded descendant sleeps 400 ms"}),
    );

    // Payload budgets must agree between the public codec and process reader.
    let req = request();
    let mut reply = response(&req, ProposedVerdict::Unknown);
    reply.diagnostics.push(PluginDiagnostic {
        code: "qcore.libevent.pending".into(),
        message: "x".repeat(4096),
        fact_id: None,
    });
    let payload_bytes = serde_json::to_vec(&reply).unwrap().len();
    let frame = encode_protocol_frame(&reply, payload_bytes).unwrap();
    let codec_ok = decode_protocol_frame::<VerificationResponse>(&frame, payload_bytes).is_ok();
    fs::write(package.join("response.bin"), &frame).unwrap();
    write_executable(
        &script,
        b"#!/bin/sh\n/bin/cat > /dev/null\n/bin/cat response.bin\n",
    );
    manifest.limits.timeout_ms = 1000;
    manifest.limits.output_bytes = payload_bytes;
    let provider = UnverifiedProvider::new(manifest.clone(), &package).unwrap();
    let outcome = provider.verify(&req);
    results.push(serde_json::json!({"id": "VP07_frame_limit_disagreement", "payload_bytes": payload_bytes,
        "frame_bytes": frame.len(), "codec_accepts": codec_ok, "process_result": format!("{outcome:?}"),
        "pass": codec_ok && outcome.is_ok()}));
    manifest.limits.output_bytes = frame.len();
    let outcome = UnverifiedProvider::new(manifest.clone(), &package)
        .unwrap()
        .verify(&req);
    results.push(
        serde_json::json!({"id": "CONTROL_frame_limit_includes_header", "pass": outcome.is_ok(),
        "detail": format!("{:?}", outcome.as_ref().map(|reply| reply.verdict))}),
    );

    // Encoding failure returns before a spawned child's lifecycle is guarded.
    write_executable(
        &script,
        b"#!/bin/sh\n/bin/sleep 0.05\n: > after-return.marker\n",
    );
    manifest.limits.output_bytes = 1;
    manifest.limits.timeout_ms = 20;
    let outcome = UnverifiedProvider::new(manifest, &package)
        .unwrap()
        .verify(&req);
    let marker_at_return = package.join("after-return.marker").exists();
    std::thread::sleep(std::time::Duration::from_millis(200));
    let marker_after_return = package.join("after-return.marker").exists();
    results.push(
        serde_json::json!({"id": "VP08_provider_survives_encoding_failure",
        "detail": format!("{outcome:?}"), "marker_at_return": marker_at_return,
        "marker_after_return": marker_after_return, "pass": !marker_after_return}),
    );
    fs::remove_dir_all(package).unwrap();
}

fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example reevaluation_followup");
    let mut results = vec![];
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(CountingProvider {
            manifest: first_party_manifests()
                .into_iter()
                .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
                .expect("Libevent fixture manifest"),
            calls: calls.clone(),
        }))
        .unwrap();
    let compiled = compile_files_with_registry(
        &[SourceFile::new(
            "scope.qc",
            "verify scope check { plugin qcore.libevent profile default capabilities { resource_contracts }; } fn main() -> unit { return; }",
        )],
        &CompileOptions::default(),
        &registry,
    );
    let invocations = calls.load(Ordering::SeqCst);
    results.push(serde_json::json!({"id": "VP05_scope_never_invokes_provider", "compilation_succeeded": compiled.is_ok(),
        "provider_invocations": invocations, "pass": invocations > 0 || compiled.as_ref().is_err_and(|errors| errors.iter().any(|error| error.code == "Q701")),
        "detail": format!("{:?}", compiled.as_ref().map(|m| &m.entry)), "accepted_boundary": "dispatch or explicit Q701 unsupported-integration rejection"}));
    let outcome = registry.verify(&request());
    results.push(serde_json::json!({"id": "VP06_unsupported_unsafe_proposal", "verdict": format!("{:?}", outcome.verdict),
        "diagnostics": outcome.diagnostics, "pass": outcome.verdict == ProposedVerdict::Unknown}));
    results.push(serde_json::json!({"id": "CONTROL_direct_provider_invoked", "pass": calls.load(Ordering::SeqCst) == invocations + 1}));

    // W1 literal-site migration: the probe environment is loaded from a
    // `.qcore` module through the total-or-reject v1 loader.
    let contracts = contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "q_base.qcore".to_owned(),
        include_str!("../../crates/qcore-c-gateway/tests/fixtures/contracts/q_base.qv").to_owned(),
    )]))
    .unwrap();
    let source = "void *q_alloc(void); void q_free(void *); void *make(void) { void *p = q_alloc(); return p; } int main(void) { void *p = make(); return 0; }";
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "main",
        &contracts,
    );
    results.push(serde_json::json!({"id": "LC01_returned_owner_lost_at_internal_call", "pass": !matches!(&result, Ok(UnrestrictedLegacyResult::Safe(_))),
        "detail": format!("{result:?}")}));
    let direct = "void *q_alloc(void); void q_free(void *); int main(void) { void *p = q_alloc(); return 0; }";
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: direct,
            ..Default::default()
        },
        "main",
        &contracts,
    );
    results.push(serde_json::json!({"id": "CONTROL_direct_allocation_leak", "pass": matches!(&result, Ok(UnrestrictedLegacyResult::Unsafe(_))),
        "detail": format!("{result:?}")}));

    // Two acquisitions cannot both succeed without an intervening unlock.
    let result = check_c11_execution(&C11Execution {
        events: vec![
            event(0, 1, K::MutexLock { mutex: 0 }),
            event(1, 2, K::MutexLock { mutex: 0 }),
        ],
        mutex_order: BTreeMap::from([(0, vec![0, 1])]),
        ..Default::default()
    });
    results.push(serde_json::json!({"id": "WM05_simultaneous_mutex_owners", "pass": result.is_err(), "detail": format!("{result:?}")}));
    let result = check_c11_execution(&C11Execution {
        events: vec![
            event(0, 1, K::NonAtomicWrite { location: 0 }),
            event(1, 1, K::MutexUnlock { mutex: 0 }),
            event(2, 2, K::MutexLock { mutex: 0 }),
            event(3, 2, K::NonAtomicWrite { location: 0 }),
        ],
        synchronizes_with: vec![E::new(1, 2)],
        mutex_order: BTreeMap::from([(0, vec![1, 2])]),
        ..Default::default()
    });
    results.push(serde_json::json!({"id": "WM05_unowned_unlock_manufactures_order", "pass": result.is_err(), "detail": format!("{result:?}")}));
    #[cfg(unix)]
    process_probes(&mut results);
    let failures = results.iter().filter(|r| r["pass"] == false).count();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"schema_version": 1, "failed_expectations": failures, "probes": results})).unwrap());
    if failures > 0 {
        std::process::exit(1);
    }
}
