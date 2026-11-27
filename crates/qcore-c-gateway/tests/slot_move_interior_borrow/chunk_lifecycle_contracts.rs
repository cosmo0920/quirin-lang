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

//! Card FB-CHUNK-LIFECYCLE-CONTRACTS: the contract its census found statable
//! from the callee's own body, `flb_input_name` (production text,
//! `providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_name.qv`),
//! proved on a reproduction of the pinned body (`src/flb_input.c:1143-1150`
//! at Fluent Bit 046accf6) in both directions: the faithful body keeps the
//! declaration in its defining unit (gateway rule 13a), and a body that hands
//! back anything but a member read of `ins` has it withdrawn there. A `Safe`
//! here is over the production declarations and the fixture's own C and
//! promotes nothing. A module of `slot_move_interior_borrow.rs` (rule 13a's own
//! target), so the suite gains no test target (GATE-SCOPE-READ-CENSUS-FLOOR).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{DerivedExtent, ResourceParameterMode, ResourceReturnMode};

use super::clang_available;

/// The refusal of a return the gateway does not model (`clang_gap.rs`'s `RETURN`).
const RETURN: &str = "unmodeled call, aggregate or indirect access in return expression";

const NAME: &str = include_str!(
    "../../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_name.qv"
);
const MEMORY: &str =
    include_str!("../../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const STRING: &str =
    include_str!("../../../../providers/quirin-provider-libc/contracts/libc/string.qv");
/// The card's marker in the production module: the text before it is the
/// module as it stood at the card's base.
const MARKER: &str = "// FB-CHUNK-LIFECYCLE-CONTRACTS.";

/// The C the fixtures share. `__SIZE_TYPE__` keeps the libc positions
/// readable under LLP64 as under LP64 (docs/development/windows-types.md).
const PRELUDE: &str = "\
typedef __SIZE_TYPE__ size_t;
void *malloc(size_t size); void *calloc(size_t count, size_t size); void free(void *ptr);
size_t strlen(const char *s);
struct flb_input_instance { char *alias; char *name; int id; };
const char *flb_input_name(struct flb_input_instance *ins);
";

/// The pinned body, verbatim but for whitespace.
const BODY: &str = "\
const char *flb_input_name(struct flb_input_instance *ins)
{
    if (ins->alias) {
        return ins->alias;
    }

    return ins->name;
}
";

fn contracts(with_card: bool) -> ResourceContracts {
    let name = if with_card {
        NAME
    } else {
        NAME.split_once(MARKER).expect("the card marker").0
    };
    load_resource_contract_sources(&BTreeMap::from([
        ("fluent_bit/input_chunk_name.qv".to_owned(), name.to_owned()),
        ("libc/memory.qv".to_owned(), MEMORY.to_owned()),
        ("libc/string.qv".to_owned(), STRING.to_owned()),
    ]))
    .expect("the production modules load")
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    kinds: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn mentions(&self, key: &str) -> bool {
        self.kinds.iter().any(|kind| kind.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The row of `caller` in the unit `PRELUDE + source`.
fn caller(source: &str, with_card: bool) -> Row {
    row(source, "caller", with_card)
}

/// The row of `name` in the unit `PRELUDE + source`, analysed from `name`.
fn row(source: &str, name: &str, with_card: bool) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        name,
        &contracts(with_card),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let row = &summary_of(&outcome)
        .unwrap_or_else(|| panic!("no summary: {outcome:?}"))
        .functions[name];
    Row {
        status: row.status,
        kinds: row
            .diagnostics
            .iter()
            .map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

/// A caller that reads the name and owns nothing else.
const READ: &str = "size_t caller(struct flb_input_instance *ins) { const char *n = flb_input_name(ins); \
     return n ? strlen(n) : 0; }";

const UNCONTRACTED: &str = "call to uncontracted function `flb_input_name`";

#[test]
fn the_declaration_is_a_borrowed_instance_and_a_parent_borrowed_result() {
    let loaded = contracts(true);
    let contract = loaded.contract("flb_input_name").expect("declared");
    assert_eq!(contract.parameters, vec![ResourceParameterMode::Borrowed]);
    assert!(
        matches!(
            contract.result,
            ResourceReturnMode::Derived {
                parameter: 0,
                extent: DerivedExtent::Borrowed,
                nullable: true,
                ..
            }
        ),
        "{:?}",
        contract.result
    );
    assert!(contracts(false).contract("flb_input_name").is_none());
}

#[test]
fn a_caller_unit_reads_the_name_under_the_declaration() {
    if !clang_available() {
        return;
    }
    // Base (no declaration): the call is uncontracted.
    let base = caller(READ, false);
    assert!(!base.safe() && base.mentions(UNCONTRACTED), "{base:?}");
    // Tip: the borrowed read is decided.
    let tip = caller(READ, true);
    assert!(tip.safe(), "{tip:?}");
}

#[test]
fn the_name_owns_nothing_and_ends_with_its_instance() {
    if !clang_available() {
        return;
    }
    // Releasing the borrowed name is refused, never accused.
    let released = caller(
        "void caller(struct flb_input_instance *ins) { char *n = (char *) flb_input_name(ins); \
         free(n); }",
        true,
    );
    assert!(
        released.status == AnalysisStatus::Unknown && !released.mentions("InvalidRelease"),
        "{released:?}"
    );
    // A use after the instance's release is refused.
    let ended = caller(
        "size_t caller(void) { struct flb_input_instance *ins = calloc(1, sizeof *ins); \
         const char *n; size_t len; if (!ins) { return 0; } n = flb_input_name(ins); \
         free(ins); len = n ? strlen(n) : 0; return len; }",
        true,
    );
    assert!(!ended.safe(), "{ended:?}");
}

#[test]
fn the_pinned_body_keeps_the_declaration_in_its_defining_unit() {
    if !clang_available() {
        return;
    }
    let tip = caller(&format!("{BODY}\n{READ}"), true);
    assert!(tip.safe() && !tip.mentions(UNCONTRACTED), "{tip:?}");
}

#[test]
fn a_body_handing_back_anything_else_has_the_declaration_withdrawn() {
    if !clang_available() {
        return;
    }
    // A fresh copy is not a member read of `ins` (rule 13a): the declaration
    // is withdrawn in this unit and the body's own summary governs, so the
    // caller that drops the result is not Safe.
    let copy = caller(
        &format!(
            "const char *flb_input_name(struct flb_input_instance *ins)\n\
             {{ char *copy = malloc(8); (void) ins; return copy; }}\n{READ}"
        ),
        true,
    );
    assert!(!copy.safe(), "{copy:?}");
    // A body that releases the instance contradicts `resource_borrowed(ins)`
    // (constraint 8's mode gate): the declaration is withdrawn in this unit,
    // the body's own summary governs, and the caller's release of the same
    // instance is no longer Safe.
    let released = caller(
        "const char *flb_input_name(struct flb_input_instance *ins)\n\
         { free(ins); return 0; }\n\
         void caller(void) { struct flb_input_instance *ins = calloc(1, sizeof *ins); \
         if (!ins) { return; } (void) flb_input_name(ins); free(ins); }",
        true,
    );
    assert!(!released.safe(), "{released:?}");
}

/// `get_input_chunk_source_name` (`src/flb_input_chunk.c:525-538` at
/// 046accf6), its types cut to the members it reads: the lifecycle row the
/// declaration reaches in the system slice.
const SOURCE_NAME: &str = "\
struct cio_stream { char *name; };
struct cio_chunk { struct cio_stream *st; };
struct flb_input_chunk { int fs_backlog; void *chunk; struct flb_input_instance *in; };
const char *get_input_chunk_source_name(struct flb_input_chunk *input_chunk)
{
    struct cio_chunk *chunk;

    if (input_chunk->fs_backlog == 1 && input_chunk->chunk != 0) {
        chunk = (struct cio_chunk *) input_chunk->chunk;

        if (chunk->st != 0 && chunk->st->name != 0) {
            return chunk->st->name;
        }
    }

    return flb_input_name(input_chunk->in);
}
";

#[test]
fn the_source_name_shape_trades_its_uncontracted_call() {
    if !clang_available() {
        return;
    }
    let base = row(SOURCE_NAME, "get_input_chunk_source_name", false);
    assert!(base.mentions(UNCONTRACTED), "{base:?}");
    let tip = row(SOURCE_NAME, "get_input_chunk_source_name", true);
    // The call is decided, but its actual is a member read, so the return of
    // the call is refused as the other return already was: the row trades the
    // uncontracted leaf for a return-expression leaf and stays not Safe.
    assert!(!tip.safe() && !tip.mentions(UNCONTRACTED), "{tip:?}");
    assert_eq!(
        tip.kinds
            .iter()
            .filter(|kind| kind.as_str() == RETURN)
            .count(),
        2,
        "{tip:?}"
    );
}
