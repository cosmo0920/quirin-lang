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
