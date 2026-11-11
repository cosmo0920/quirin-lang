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

//! Unit tests for `source.region.v1`'s header reader (card
//! `QUANT-REGION-SOURCE`).
use super::*;
use crate::frontend::{StatementKind, parse_source_program};

/// Parse one entry whose single statement is an `error region`, and hand the
/// reader the parts the parser recorded.
fn clauses(body: &str) -> Result<Vec<RegionCatchSyntax>, SourceDiagnostic> {
    let source = format!(
        "error Failure {{ Failed; }}\nerror Other {{ Bad; }}\nfn main() -> i32 {{\n{body}\nreturn 0;\n}}\n"
    );
    let program = parse_source_program(&source).expect("the fixture parses");
    let declaration = program
        .declarations
        .iter()
        .find(|declaration| declaration.value.name == "main")
        .expect("the entry is declared");
    let statement = declaration
        .value
        .body
        .as_ref()
        .expect("the entry has a body")
        .statements
        .first()
        .expect("the entry has a statement");
    let StatementKind::ErrorRegion {
        blocks,
        catch_bindings,
        ..
    } = &statement.value
    else {
        panic!("the fixture's first statement is an error region");
    };
    read_catch_headers(&program, blocks, catch_bindings, statement.span)
}

#[test]
fn a_plain_catch_carries_its_error_and_opens_nothing() {
    let read = clauses("error region R { } catch (Failure) { }").unwrap();
    assert_eq!(
        read,
        vec![RegionCatchSyntax {
            error: "Failure".to_owned(),
            opened_region: None,
            binding: None,
            span: read[0].span,
        }]
    );
}

#[test]
fn a_quant_open_clause_carries_the_region_it_opens_and_the_name_it_binds() {
    let read = clauses("error region R { } catch (Failure) quant open R as pack { }").unwrap();
    assert_eq!(read[0].error, "Failure");
    assert_eq!(read[0].opened_region.as_deref(), Some("R"));
    assert_eq!(read[0].binding.as_deref(), Some("pack"));
}

#[test]
fn the_opened_region_is_read_as_written_rather_than_assumed() {
    // The reader does **not** decide that a clause opens its own region: it
    // reports the name that was written, and `qcore_core::error_region` is what
    // refuses a mismatch (`Q303`). Reading it here instead would move a rule
    // out of the model and into a parser.
    let read = clauses("error region R { } catch (Failure) quant open S as pack { }").unwrap();
    assert_eq!(read[0].opened_region.as_deref(), Some("S"));
}

#[test]
fn every_clause_of_a_multi_clause_region_is_read_in_written_order() {
    // Positional reading: clause `i`'s header is the tokens between block `i`
    // and block `i + 1`, so the clauses come back in order and a nested brace
    // group inside the body cannot be mistaken for one.
    let read =
        clauses("error region R { } catch (Failure) quant open R as first { } catch (Other) { }")
            .unwrap();
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].error, "Failure");
    assert_eq!(read[0].binding.as_deref(), Some("first"));
    assert_eq!(read[1].error, "Other");
    assert_eq!(read[1].binding, None);
}

#[test]
fn a_pattern_that_is_not_one_word_is_refused() {
    let refusal = clauses("error region R { } catch (Failure e) { }").unwrap_err();
    assert_eq!(refusal.code, "Q302");
    assert!(
        refusal.message.contains("REGION-CATCH-PAYLOAD-BINDING"),
        "{}",
        refusal.message
    );
}

#[test]
fn the_code_of_each_model_refusal_is_the_one_that_pass_already_allocates() {
    // Stated once here so this route and
    // `qcore_core::region_task_credit::RegionTaskDiagnostic::code` cannot
    // drift: these are the `ErrorRegion` pass's own codes, not new ones.
    use qcore_core::error_region::ErrorRegionError;
    assert_eq!(
        error_region_code(&ErrorRegionError::QuantOpenOutsideCatch),
        "Q302"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::WrongRegionOpened {
            expected: "R".into(),
            actual: "S".into(),
        }),
        "Q303"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::CleanupPackDuplicated("p".into())),
        "Q304"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::CleanupPackNotDischarged),
        "Q305"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::CatchWithoutRequiredQuantOpen {
            region: "R".into(),
            error: "Failure".into(),
        }),
        "Q306"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::RethrowWithUndischargedCleanup),
        "Q307"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::FallibleCleanup),
        "Q308"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::ResourceInPayloadAndCleanupPack(1)),
        "Q309"
    );
    assert_eq!(
        error_region_code(&ErrorRegionError::ErrorNotInEffectRow("Failure".into())),
        "Q300"
    );
}

#[test]
fn the_models_message_is_carried_verbatim_without_a_doubled_code() {
    use qcore_core::error_region::ErrorRegionError;
    // The model writes its own code into its `Display`; the presentation
    // carries that code in the `code` field, so it is written once.
    assert_eq!(
        error_region_message(&ErrorRegionError::CleanupPackNotDischarged),
        "cleanup pack is not discharged"
    );
    // A refusal whose message carries no code of its own is untouched.
    assert_eq!(
        error_region_message(&ErrorRegionError::InsufficientCredit),
        "insufficient strict credit"
    );
}
