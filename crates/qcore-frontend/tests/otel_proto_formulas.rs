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

//! Card **OTEL-PROTO-AUDIT**: every protocol-derived obligation of the
//! OpenTelemetry decoders and encoders (protobuf-c's unpack facts, the site
//! obligations, the site classes and the pinned protocol identity) is a
//! formula of the C04.18.9 typed rendering over the OTP block of registered
//! terms (`RENDERED_TERMS`, block `OTP`; five `otp_` sorts in
//! `RENDERED_VALUE_SORTS`), checked by the premise-formula validator
//! `guarded_proposition`. The formulas are the report's
//! `docs/reports/otel-proto-audit/formulas.json`, which the census JSON cites
//! by id. Prose is refused. The Lean model `QCore.OtelProtoObligations`
//! states one theorem per formula, `formula_<id>`, bound here by name and
//! text.

use qcore_frontend::premise_formula::{
    RENDERED_TERMS, RENDERED_VALUE_SORTS, RenderedSort, guarded_proposition, term,
};

const TABLE: &str = include_str!("../../../docs/reports/otel-proto-audit/formulas.json");

/// The `[id, formula]` rows of the report's table (one per line; no
/// formula contains a quotation mark).
fn formulas() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in TABLE.lines() {
        let line = line.trim();
        if !line.starts_with("[\"") {
            continue;
        }
        let body = line
            .trim_start_matches("[\"")
            .trim_end_matches(',')
            .trim_end_matches("\"]");
        let (id, formula) = body.split_once("\", \"").expect("an [id, formula] row");
        assert!(!formula.contains('"'), "{id}");
        out.push((id.to_owned(), formula.to_owned()));
    }
    out
}

#[test]
fn every_formula_is_a_validated_typed_rendering() {
    let rows = formulas();
    assert_eq!(rows.len(), 22);
    for (id, formula) in &rows {
        assert!(id.starts_with("otp_"), "{id}");
        guarded_proposition(id, formula).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let mut ids = rows.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), rows.len());
}

#[test]
fn prose_is_refused_by_the_validator() {
    for prose in [
        "an absent sub-message is a NULL pointer",
        "\"#otp_nonnull(#m,#f)\"",
        "forall (#s: otp_site) { #otp_unregistered(#s) }",
        // A registered term in the wrong sort.
        "forall (#s: otp_site) { #otp_unpacked(#s) }",
        // A free variable.
        "#otp_guarded(#s)",
        // Not the canonical printing.
        "forall (#s: otp_site) { #otp_guarded(#s) => #otp_cited(#s) }",
    ] {
        assert!(guarded_proposition("otp_probe", prose).is_err(), "{prose}");
    }
}

#[test]
fn every_otp_term_is_used_typed_and_bound_to_lean() {
    let rows = formulas();
    let otp = RENDERED_TERMS
        .iter()
        .filter(|t| t.block == "OTP")
        .collect::<Vec<_>>();
    assert_eq!(otp.len(), 35);
    let registered = |s: &str| s == "integer" || RENDERED_VALUE_SORTS.iter().any(|(v, _)| *v == s);
    for t in &otp {
        assert!(t.name.starts_with("otp_"), "#{}", t.name);
        assert!(term(t.name).is_none(), "#{} is a premise term", t.name);
        assert_eq!(
            RENDERED_TERMS.iter().filter(|u| u.name == t.name).count(),
            1,
            "#{}",
            t.name
        );
        let sorts: Vec<&str> = match t.sort {
            RenderedSort::Value(v) => vec![v],
            RenderedSort::Predicate(a) => a.to_vec(),
            RenderedSort::Function(a, r) => a.iter().copied().chain([r]).collect(),
        };
        assert!(sorts.iter().all(|s| registered(s)), "#{}", t.name);
        let spelled = format!("#{}", t.name);
        assert!(
            rows.iter().any(|(_, f)| f
                .split(|c: char| !(c == '#' || c == '_' || c.is_ascii_alphanumeric()))
                .any(|w| w == spelled)),
            "#{} is named by no formula",
            t.name
        );
    }
    let sorts = RENDERED_VALUE_SORTS
        .iter()
        .filter(|(s, _)| s.starts_with("otp_"))
        .count();
    assert_eq!(sorts, 5);
    // The OTP block is the last block, contiguous, after the G1 block (card
    // P0-G1-CLOSURE-RECORD), which follows the CX block.
    let blocks: Vec<_> = RENDERED_TERMS.iter().map(|t| t.block).collect();
    let first = blocks.iter().position(|b| *b == "OTP").unwrap();
    assert!(blocks[first..].iter().all(|b| *b == "OTP"));
    assert_eq!(blocks[first - 1], "G1");
    // Every formula has its Lean theorem and is stated there as written;
    // every term has its Lean model entry.
    let lean = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../proof/QCore/OtelProtoObligations.lean"),
    )
    .expect("QCore.OtelProtoObligations");
    for (id, formula) in &rows {
        assert!(
            lean.contains(&format!("theorem formula_{id} ")),
            "no Lean theorem formula_{id}"
        );
        assert!(
            lean.contains(&format!("\"{formula}\"")),
            "the Lean formula table does not state {id} as written"
        );
    }
    for t in &otp {
        assert!(
            lean.contains(&format!("(\"{}\"", t.name)),
            "no Lean otpTerms entry for #{}",
            t.name
        );
    }
}
