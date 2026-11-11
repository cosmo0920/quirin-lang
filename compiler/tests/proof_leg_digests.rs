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

//! `CT-PROOF-LEG-3` (wave 41) — the cross-check `PROOF-LEG-DIGESTS-UNCROSSCHECKED`
//! asks for.
//!
//! **The defect this repairs, as measured by `STABILITY-REEVAL-4` (wave 40):**
//! every proof leg pins a digest the Lean kernel decides, as a *bare Rust
//! literal that nothing compares with its Lean twin*. The Lean audit proves
//! the Lean digest of the **Lean** term; the Rust leg recomputes the digest
//! from the **artifact** and compares it with its own literal; but the two
//! literals are independent copies, and nothing under `compiler/tests/` or
//! `crates/` read any `.lean` file. Someone who re-derived a digest from new
//! bytes and updated both literals would be green in both gates — a silent
//! re-transcription.
//!
//! This file closes that by reading both sides and comparing them, **total or
//! reject in both directions**:
//!
//! * every `theorem ct<N>…digest : … = <numeral> :=` in `proof/QCore/` must
//!   have at least one Rust constant for the same case, and every such
//!   constant must carry the same numeral;
//! * every `const CT<N>_LEAN_…DIGEST: u64 = <numeral>;` under
//!   `compiler/tests/` must have a Lean theorem for the same case — a Rust pin
//!   with no Lean twin is as much a defect as the reverse;
//! * the set of cases that carry a digest pin at all is itself pinned below,
//!   so a leg added with neither side registered trips here rather than
//!   passing silently.
//!
//! What it does **not** establish: a verified extraction. The Lean numeral is
//! decided by the kernel (`by decide`, checked by the standing
//! `scripts/check_lean_audits.py` gate) and read here out of the source text
//! the kernel checked; this test proves the two *texts* agree, which is
//! exactly the re-transcription control that was missing. The artifact side
//! stays each leg's own business.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The cases that carry a digest pin today, pinned deliberately. A new proof
/// leg adds its case here in the same change that adds its Lean theorem and
/// its Rust constant — the fail-closed table precedent of
/// `ct_corpus.rs::PROOF_GATED_CASES`.
const CASES_WITH_A_DIGEST_PIN: &[&str] = &["CT01", "CT03", "CT07", "CT37", "CT50"];

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

/// The tracked files directly in `directory` with `extension`, sorted. Card
/// W45-TEST-HYGIENE: an untracked or ignored file there is not a pin site.
fn tracked_directly_under(directory: &Path, extension: &str) -> Vec<PathBuf> {
    tracked_files::tracked_files(directory)
        .into_iter()
        .filter(|path| path.parent() == Some(directory))
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .collect()
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate sits directly under the repository root")
        .to_path_buf()
}

/// One pinned digest, wherever it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pin {
    case: String,
    symbol: String,
    value: u128,
    site: String,
}

fn lean_case_of(name: &str) -> Option<String> {
    let rest = name.strip_prefix("ct")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || !name.ends_with("digest") {
        return None;
    }
    Some(format!("CT{digits}"))
}

fn rust_case_of(name: &str) -> Option<String> {
    let rest = name.strip_prefix("CT")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || !name.ends_with("DIGEST") || !rest[digits.len()..].starts_with("_LEAN_")
    {
        return None;
    }
    Some(format!("CT{digits}"))
}

fn numeral(text: &str) -> Option<u128> {
    let digits: String = text
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

/// `theorem ct37_trace_digest : mixTrace referenceRun = 17239871955803798867 :=`
fn lean_pins(root: &Path) -> Vec<Pin> {
    let directory = root.join("proof").join("QCore");
    let entries = tracked_directly_under(&directory, "lean");
    let mut pins = Vec::new();
    for path in entries {
        let text = fs::read_to_string(&path).expect("a readable Lean module");
        for (index, line) in text.lines().enumerate() {
            let Some(rest) = line.strip_prefix("theorem ") else {
                continue;
            };
            let symbol = rest
                .split([' ', ':'])
                .next()
                .unwrap_or_default()
                .to_string();
            let Some(case) = lean_case_of(&symbol) else {
                continue;
            };
            let equation = rest
                .split_once(" = ")
                .unwrap_or_else(|| {
                    panic!(
                        "{}:{}: a digest theorem states an equation",
                        path.display(),
                        index + 1
                    )
                })
                .1;
            let value = numeral(equation).unwrap_or_else(|| {
                panic!(
                    "{}:{}: a digest theorem's right-hand side is a numeral",
                    path.display(),
                    index + 1
                )
            });
            pins.push(Pin {
                case,
                symbol,
                value,
                site: format!(
                    "{}:{}",
                    path.strip_prefix(root).unwrap_or(&path).display(),
                    index + 1
                ),
            });
        }
    }
    pins
}

/// `const CT37_LEAN_TRACE_DIGEST: u64 = 17239871955803798867;`
fn rust_pins(root: &Path) -> Vec<Pin> {
    let directory = root.join("compiler").join("tests");
    let entries = tracked_directly_under(&directory, "rs");
    let mut pins = Vec::new();
    for path in entries {
        let text = fs::read_to_string(&path).expect("a readable test file");
        for (index, line) in text.lines().enumerate() {
            let Some(rest) = line.trim_start().strip_prefix("const ") else {
                continue;
            };
            let symbol = rest
                .split(':')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            let Some(case) = rust_case_of(&symbol) else {
                continue;
            };
            let value = rest
                .split_once(" = ")
                .and_then(|(_, tail)| numeral(tail))
                .unwrap_or_else(|| {
                    panic!(
                        "{}:{}: a digest constant is initialized with a numeral",
                        path.display(),
                        index + 1
                    )
                });
            pins.push(Pin {
                case,
                symbol,
                value,
                site: format!(
                    "{}:{}",
                    path.strip_prefix(root).unwrap_or(&path).display(),
                    index + 1
                ),
            });
        }
    }
    pins
}

/// Each case's Lean-decided digest, refusing two Lean theorems that decide
/// different values for one case.
fn lean_by_case(pins: &[Pin]) -> BTreeMap<String, Pin> {
    let mut decided: BTreeMap<String, Pin> = BTreeMap::new();
    for pin in pins {
        if let Some(existing) = decided.get(&pin.case) {
            assert_eq!(
                existing.value, pin.value,
                "{} and {} decide different digests for {}",
                existing.site, pin.site, pin.case
            );
        } else {
            decided.insert(pin.case.clone(), pin.clone());
        }
    }
    decided
}

/// **The cross-check.** Every Rust digest literal equals the value its Lean
/// side decides; a silent re-transcription on either side goes red here.
#[test]
fn every_proof_leg_digest_literal_equals_the_value_its_lean_side_decides() {
    let root = repository_root();
    let lean = lean_by_case(&lean_pins(&root));
    let rust = rust_pins(&root);
    assert!(
        !rust.is_empty(),
        "the proof legs pin digests; finding none means this reader broke"
    );
    for pin in &rust {
        let decided = lean.get(&pin.case).unwrap_or_else(|| {
            panic!(
                "{} pins a digest for {} that no Lean theorem decides",
                pin.site, pin.case
            )
        });
        assert_eq!(
            pin.value, decided.value,
            "{} ({}) pins {} for {}, but {} ({}) decides {}",
            pin.site, pin.symbol, pin.value, pin.case, decided.site, decided.symbol, decided.value
        );
    }
}

/// The other direction, and the reason this is total rather than a spot
/// check: a Lean digest theorem with no Rust pin is a leg whose artifact is
/// compared with nothing, and the set of cases carrying a pin is itself
/// pinned.
#[test]
fn the_digest_pins_agree_case_for_case_in_both_directions() {
    let root = repository_root();
    let lean = lean_by_case(&lean_pins(&root));
    let rust = rust_pins(&root);

    let lean_cases: Vec<&str> = lean.keys().map(String::as_str).collect();
    assert_eq!(
        lean_cases, CASES_WITH_A_DIGEST_PIN,
        "the Lean digest theorems are pinned; a new leg registers its case here deliberately"
    );

    let mut rust_cases: Vec<String> = rust.iter().map(|pin| pin.case.clone()).collect();
    rust_cases.sort();
    rust_cases.dedup();
    let rust_cases: Vec<&str> = rust_cases.iter().map(String::as_str).collect();
    assert_eq!(
        rust_cases, CASES_WITH_A_DIGEST_PIN,
        "every case with a Lean digest theorem carries a Rust pin, and no other case does"
    );
}

/// The inventory itself, so the report's table is a measurement. Printed with
/// `--nocapture`; asserted so the reader cannot silently return nothing.
#[test]
fn the_digest_inventory_is_reported() {
    let root = repository_root();
    let lean = lean_pins(&root);
    let rust = rust_pins(&root);
    for pin in &lean {
        println!(
            "LEAN\t{}\t{}\t{}\t{}",
            pin.case, pin.symbol, pin.value, pin.site
        );
    }
    for pin in &rust {
        println!(
            "RUST\t{}\t{}\t{}\t{}",
            pin.case, pin.symbol, pin.value, pin.site
        );
    }
    // Measured at the card's tip: five Lean theorems, six Rust constants —
    // CT01's is written twice (`ct01_proof_leg.rs` and `ct12_ct14_proof_reach.rs`),
    // which the residual's own inventory named only once.
    assert_eq!(lean.len(), 5);
    assert_eq!(rust.len(), 6);
}
