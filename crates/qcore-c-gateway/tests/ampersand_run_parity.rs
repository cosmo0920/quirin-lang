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

//! Card AMPERSAND-RUN-PARITY-OTHER-READERS (gateway rule 74e): every address
//! decision of the unfused token stream reads a run of `&` tokens by its
//! parity (`ampersand_runs`). These fixtures pin the six spellings through
//! rule 10a's frame, the reader rule 74d introduced the parity for, now
//! reading the shared helper: a name the frame drops is poisoned by every
//! refused statement (`u_read()`), so its guarded release is refused, and a
//! name it keeps survives. The loop-owner and thread-entry readers the card
//! fixed are pinned beside their harnesses in `quirin-source-analysis`'s
//! `loop_owner_typed_route.rs`, `loop_owner_holder_region.rs` and
//! `loop_owner_coro_plugin.rs`.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`
//! surface and a test-local borrowed `l_use`. `Safe` here is `Safe` over
//! synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void l_use(void *p); int u_read(void);
";

const L_USE: &str = "module contract;
extern fn l_use(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "q_base.qv".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        ("l_use.qv".to_owned(), L_USE.to_owned()),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn unresolved(&self, name: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == name)
        })
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("these fixtures project partially")
        }
    }
}

fn source(body: &str) -> String {
    format!("{DECLARATIONS}int subject(int flag, int n) {{\n{body}\n}}\n")
}

/// `subject(int flag, int n) { body }`.
fn analyze(body: &str) -> Run {
    let source = source(body);
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let result = &summary_of(&outcome).functions["subject"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

/// The body a guarded release follows: `p` built, `spelling` evaluated, a
/// refused statement, then `if (p) q_free(p);`.
fn guarded(spelling: &str) -> String {
    format!(
        "void *p = 0; long t = 0;\nif (n > 0) p = q_alloc();\n{spelling}\nu_read();\n\
         if (p) q_free(p);\nreturn (int) t;"
    )
}

/// The address is taken: the refused statement may write `p` through it, so
/// the guarded release is refused (identity unresolved), never a leak.
fn assert_taken(spelling: &str) {
    let run = analyze(&guarded(spelling));
    assert_eq!(run.status, AnalysisStatus::Unknown, "{spelling}: {run:?}");
    assert!(run.unresolved("p"), "{spelling}: {run:?}");
    assert_eq!(run.leaks(), 0, "{spelling}: {run:?}");
}

/// No address is taken: `p` survives the refused statement and its guarded
/// release is decided; the row is `Unknown` on the refused statement alone.
fn assert_not_taken(spelling: &str) {
    let run = analyze(&guarded(spelling));
    assert_eq!(run.status, AnalysisStatus::Unknown, "{spelling}: {run:?}");
    assert!(!run.unresolved("p"), "{spelling}: {run:?}");
    assert_eq!(run.leaks(), 0, "{spelling}: {run:?}");
}

/// `p && &q`: an odd run (three `&` tokens) ends in the address operator.
#[test]
fn a_logical_and_then_an_address_takes_it() {
    if !clang_available() {
        return;
    }
    assert_taken("t = flag && &p;");
}

/// `p && q`: an even run is a logical AND and takes no address.
#[test]
fn a_logical_and_takes_no_address() {
    if !clang_available() {
        return;
    }
    assert_not_taken("t = flag && p;");
    // Without the refused statement the same body is decided.
    let run = analyze(
        "void *p = 0; long t = 0;\nif (n > 0) p = q_alloc();\nt = flag && p;\n\
         if (p) q_free(p);\nreturn (int) t;",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

/// `p & q`: a single `&` after an operand is a binary AND, which the frame
/// keeps reading as it always did (the harmless direction: it drops the
/// name after it, here through the group `( p != 0 )`). The parity changes
/// nothing for a run of one.
#[test]
fn a_binary_and_keeps_its_reading() {
    if !clang_available() {
        return;
    }
    assert_taken("t = n & (p != 0);");
}

/// `&&&q` without spaces is the same token stream as `&& &q`.
#[test]
fn three_adjacent_ampersands_take_the_address() {
    if !clang_available() {
        return;
    }
    assert_taken("t = flag&&&p;");
}

/// `&(q)`: the address operator before a group takes the address of the
/// name inside it.
#[test]
fn an_address_of_a_group_takes_it() {
    if !clang_available() {
        return;
    }
    assert_taken("void **s = &(p); t = s != 0;");
}

/// `a & &q`: a binary AND with a pointer operand, which C11 6.5.10p2 forbids.
/// Clang rejects it, so the spelling the parity premise excludes never
/// reaches a reader (its tokens are those of `a && q`).
#[test]
fn a_binary_and_with_an_address_operand_is_rejected() {
    if !clang_available() {
        return;
    }
    let source = source(&guarded("t = n & &p;"));
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    );
    assert!(outcome.is_err(), "{outcome:?}");
}
