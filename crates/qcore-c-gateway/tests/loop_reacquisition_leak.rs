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

//! Card LOOP-REACQUISITION-LEAK-ACCUSATION (gateway rule 42): a leak only an
//! earlier generation of a refused re-acquisition reports is reported under
//! the refusal (`legacy_loop_reacquisition_leak.rs`), on route G.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`
//! surfaces. `Safe` here is `Safe` over synthetic trusted test assumptions and
//! promotes nothing; `Unsafe` is provisional. "Before" in a comment is the
//! card's census commit `0fb3c63ee` (the rule absent), measured with the
//! withdrawal decision forced to `false`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The declarations occupy lines 1-4, so a fixture's line `k` is `k + 4`.
const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t;
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
void *q_maybe_alloc(void); void q_nullable_free(void *p);
int r_fill(void *dst, int n);
";

const REFUSED: &str = "loop re-acquisition: an earlier generation that may be live is still held";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qv", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qv",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_nullable.qv",
            include_str!("fixtures/contracts/q_nullable.qv"),
        ),
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
            eprintln!("skipping Clang-dependent LOOP-REACQUISITION-LEAK-ACCUSATION test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<(usize, LegacyDiagnosticKind)>,
}

impl Run {
    /// The fixture lines (declarations subtracted) of every leak; the
    /// implicit return at the end of a `void` body is line 0.
    fn leak_lines(&self) -> BTreeSet<usize> {
        self.diagnostics
            .iter()
            .filter(|(_, kind)| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .map(|(line, _)| if *line == 0 { 0 } else { line - 4 })
            .collect()
    }

    fn refused(&self) -> bool {
        self.diagnostics.iter().any(|(_, kind)| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description == REFUSED)
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

/// `function` is a complete definition of `subject`.
fn analyze(function: &str) -> Run {
    let source = format!("{DECLARATIONS}{function}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let result = &summary_of(&outcome).functions["subject"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.line, diagnostic.kind.clone()))
            .collect(),
    }
}

fn verdict(status: AnalysisStatus) -> &'static str {
    match status {
        AnalysisStatus::Safe => "safe",
        AnalysisStatus::Unsafe => "unsafe",
        _ => "unknown",
    }
}

/// `(shape, fixture, verdict, leak lines kept, refused)`, the verdicts shared
/// with `QCore.LoopReacquisitionLeak.shapeVectors`.
const SHAPE_VECTORS: &[(&str, &str, &str, &[usize], bool)] = &[
    // The snappy shape: one acquisition only pass 1 runs. Before: leaks at
    // 9 (a pass-2 re-check, the current generation kept live by the
    // path-insensitive pass guard), 17 (after the free), 25 (the failure
    // arm) and 28 (the return), the last three reported only by the
    // previous generation. The rule withdraws exactly those three.
    (
        "snappy-shape",
        "char *subject(const char *in, int inlen) {
  int pass;
  char *out = 0;
  for (pass = 1; pass <= 2; pass++) {
    int of = 0;
    int uof = 0;
    while (of < inlen) {
      if (in[of] == 0)
        return 0;
      if (pass == 1) {
        of += 1;
        uof += 2;
        continue;
      }
      if (r_fill(out + uof, 2)) {
        q_free(out);
        return 0;
      }
      of += 1;
      uof += 2;
    }
    if (pass == 1) {
      out = q_maybe_alloc();
      if (!out)
        return 0;
    }
  }
  return out;
}",
        "unsafe",
        &[9],
        true,
    ),
    // Before: provisional Unsafe, leaks at 8 (the failure arm) and 13 (the
    // return), both only the previous generation's. After: `Unknown` under
    // the refusal at 6.
    (
        "refused-earlier-generation",
        "void *subject(int n) {
  int pass;
  void *p = 0;
  for (pass = 1; pass <= 2; pass++) {
    if (pass == 1) {
      p = q_maybe_alloc();
      if (!p)
        return 0;
    } else {
      r_fill(p, n);
    }
  }
  return p;
}",
        "unknown",
        &[],
        true,
    ),
    // snappy.c:1706/:1712 spelled as the source spells it: the failure arm
    // tested through `__builtin_expect` (`unlikely`), which rule 5h reads as
    // its operand. Before: leaks at 8 (the failure arm, the snappy :1712
    // leaf) and 13, both only the previous generation's. After: `Unknown`.
    (
        "refused-earlier-generation-expect",
        "void *subject(int n) {
  int pass;
  void *p = 0;
  for (pass = 1; pass <= 2; pass++) {
    if (pass == 1) {
      p = q_maybe_alloc();
      if (__builtin_expect(!p, 0))
        return 0;
    } else {
      r_fill(p, n);
    }
  }
  return p;
}",
        "unknown",
        &[],
        true,
    ),
    // A true leak inside the refused loop: the fresh generation is live at
    // the `n < 0` return of the iteration that acquired it. It still
    // validates; the joined leaks at 8 and 15 are withdrawn.
    (
        "refused-current-generation-leak",
        "void *subject(int n) {
  int pass;
  void *p = 0;
  for (pass = 1; pass <= 2; pass++) {
    if (pass == 1) {
      p = q_maybe_alloc();
      if (!p)
        return 0;
      if (n < 0)
        return 0;
    } else {
      r_fill(p, n);
    }
  }
  return p;
}",
        "unsafe",
        &[10],
        true,
    ),
    // Rule 11a's own loop leak: every earlier generation leaks and the
    // re-acquisition is not refused, so the deferred report is published at
    // the implicit return.
    (
        "unrefused-loop-leak",
        "void subject(int flag, int n) {
  void *p;
  for (;;) { p = q_maybe_alloc(); if (p == 0) break; q_observe(p); }
}",
        "unsafe",
        &[0],
        false,
    ),
    // No re-acquisition: the acquisition precedes the loop.
    (
        "no-reacquisition-loop-leak",
        "void subject(int flag, int n) {
  void *p = q_alloc();
  while (flag) {
    if (n)
      return;
    q_observe(p);
  }
  q_free(p);
}",
        "unsafe",
        &[5],
        false,
    ),
    (
        "no-reacquisition-loop-safe",
        "void subject(int flag, int n) {
  void *p = q_alloc();
  while (flag) {
    q_observe(p);
  }
  q_free(p);
}",
        "safe",
        &[],
        false,
    ),
];

#[test]
fn the_pinned_shapes_decide_as_the_model() {
    if !clang_available() {
        return;
    }
    for (shape, function, expected, kept, refused) in SHAPE_VECTORS {
        let run = analyze(function);
        assert_eq!(verdict(run.status), *expected, "{shape}: {run:?}");
        assert_eq!(
            run.leak_lines(),
            kept.iter().copied().collect(),
            "{shape}: {run:?}"
        );
        assert_eq!(run.refused(), *refused, "{shape}: {run:?}");
        // Fail-closed: a withdrawal leaves the refusal, so never `Safe`.
        if *refused {
            assert_ne!(run.status, AnalysisStatus::Safe, "{shape}: {run:?}");
        }
    }
}

const LEAN: &str = include_str!("../../../proof/QCore/LoopReacquisitionLeak.lean");

#[test]
fn lean_shape_vectors_match_the_rust_table() {
    let table = LEAN
        .split("def shapeVectors : List (String × Verdict) := [")
        .nth(1)
        .expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    let rows = table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let row = row
                .trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')');
            let (shape, verdict) = row.split_once(',').expect("two cells");
            (
                shape.trim().trim_matches('"').to_owned(),
                verdict
                    .trim()
                    .trim_start_matches('.')
                    .trim_start_matches('«')
                    .trim_end_matches('»')
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let rust = SHAPE_VECTORS
        .iter()
        .map(|(shape, _, verdict, _, _)| ((*shape).to_owned(), (*verdict).to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}
