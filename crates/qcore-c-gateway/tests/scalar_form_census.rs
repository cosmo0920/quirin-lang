//! Card SCALAR-STATEMENT-FORMS (round 26 E): the census driver, measurement
//! only (feature `scalar-form-census`).
//!
//! `QCORE_SCALAR_FORM_UNIT` names one `clang -E -P` translation unit and
//! `QCORE_SCALAR_FORM_ENTRY` one function it defines; the
//! analyzer reads it with no contracts, and `QCORE_SCALAR_FORM_CENSUS` receives
//! the classifier's own records of the five refusal families. The harness is
//! `docs/reports/scalar-statement-forms/scripts/`.
#![cfg(feature = "scalar-form-census")]

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect, analyze_unrestricted_c11};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[test]
#[ignore = "census driver: run by the card's harness with QCORE_SCALAR_FORM_UNIT set"]
fn census_one_unit() {
    let Some(path) = std::env::var_os("QCORE_SCALAR_FORM_UNIT") else {
        return;
    };
    let source = std::fs::read_to_string(&path).expect("the unit reads");
    // One contract no corpus body calls: the census reads the classifier's
    // refusals, not any contract's effect.
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "census.qcore".to_owned(),
        "module contract;\nextern fn qcore_census_unused(p: ptr<mut, u8>) -> unit\n\
         contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
         ensures normal resource_return_none(); };"
            .to_owned(),
    )]))
    .expect("the census contract loads");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            dialect: CCompilerDialect::Gnu11,
            ..Default::default()
        },
        &std::env::var("QCORE_SCALAR_FORM_ENTRY").unwrap_or_else(|_| "main".to_owned()),
        &contracts,
    );
    let shape = match &result {
        Ok(qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Safe(certificate)) => {
            format!("Safe {}", certificate.summary.functions.len())
        }
        Ok(qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Unsafe(summary)) => {
            format!("Unsafe {}", summary.functions.len())
        }
        Ok(qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Unknown {
            partial,
            reason,
            ..
        }) => format!(
            "Unknown {:?} {reason}",
            partial.as_ref().map(|summary| summary.functions.len())
        ),
        Err(error) => format!("error {error:?}"),
    };
    println!("unit {}: {shape}", path.to_string_lossy());
    // `QCORE_SCALAR_FORM_VERDICTS`: every function's status and diagnostic
    // keys, for the card's with/without-rule comparison.
    if let Some(out) = std::env::var_os("QCORE_SCALAR_FORM_VERDICTS") {
        let summary = match &result {
            Ok(qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Safe(certificate)) => {
                Some(&certificate.summary)
            }
            Ok(qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Unsafe(summary)) => {
                Some(summary)
            }
            Ok(qcore_c_gateway::legacy_c::UnrestrictedLegacyResult::Unknown {
                partial, ..
            }) => partial.as_ref(),
            Err(_) => None,
        };
        let rows = summary
            .map(|summary| {
                summary
                    .functions
                    .values()
                    .map(|function| {
                        let mut keys = function
                            .diagnostics
                            .iter()
                            .map(|diagnostic| format!("{}:{:?}", diagnostic.line, diagnostic.kind))
                            .collect::<Vec<_>>();
                        keys.sort();
                        (
                            function.function.clone(),
                            (format!("{:?}", function.status), keys),
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        std::fs::write(out, serde_json::to_string(&rows).expect("rows serialize"))
            .expect("the verdict file writes");
    }
}
