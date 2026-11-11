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

//! Graph reporting cannot mint a core certificate or add a witness family.
use qcore_schemas::hypothesis::*;
use qcore_verifier_host::{ProposedVerdict, VerificationOutcome, compose_verification};
#[test]
fn supported_provider_graph_claims_never_cross_the_certificate_firewall() {
    for warrant in [
        Warrant::Derived,
        Warrant::Observed,
        Warrant::Declared,
        Warrant::HumanReviewed,
        Warrant::ProviderProposed,
    ] {
        let allegation = serde_json::json!({"witness_kind":"contract_hypothesis_graph","disposition":Disposition::Supported,"warrant":warrant,"reference":"claimed core theorem"});
        assert!(serde_json::from_value::<qcore_schemas::SemanticWitness>(allegation).is_err());
        // Even a caller bypassing wire decoding cannot turn a graph/report
        // disposition into a certificate: certificate constructors are private.
        let forged = VerificationOutcome {
            verdict: ProposedVerdict::Safe,
            diagnostics: vec![],
            certificate: None,
            composite: None,
        };
        let checked = compose_verification(vec![forged]);
        assert_eq!(checked.verdict, ProposedVerdict::Unknown);
        assert!(checked.certificate.is_none());
        assert!(checked.diagnostics.iter().any(|d| d.code == "QP014"));
    }
}
