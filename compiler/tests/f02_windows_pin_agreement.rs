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

//! Cross-host check for the two F02 emitted-C digest pins.
//!
//! The two F02 emitted-C digests are pinned twice: in `f02_trace_relation.rs`
//! (Linux-reachable) and in `windows_threads.rs` (behind
//! `cfg(target_os = "windows")`, whose comment asks the reader to "keep these
//! pins paired"). Both files compute the same FNV-1a over the same LF bytes of
//! the same two programs, so the literals must agree — and until this file
//! nothing checked that they did. This test reads both files as text (so it
//! runs on every host, Windows `cfg` notwithstanding) and compares the pins.
//!
//! The Windows native row now passes with the fifth-move pair
//! (`bfa572914cb96413` / `9cbe67a70a420d52`). This comparison runs in the
//! ordinary gate on every host, so future drift fails without a Windows run.

const LINUX_FILE: &str = include_str!("f02_trace_relation.rs");
const WINDOWS_FILE: &str = include_str!("windows_threads.rs");

/// The first 16-hex-digit string literal after `anchor` in `text`.
fn hex16_literal_after(text: &str, anchor: &str) -> Option<String> {
    let start = text.find(anchor)? + anchor.len();
    let rest = &text[start..];
    let mut search = 0;
    while let Some(open) = rest[search..].find('"') {
        let begin = search + open + 1;
        let end = begin + rest[begin..].find('"')?;
        let candidate = &rest[begin..end];
        if candidate.len() == 16 && candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some(candidate.to_string());
        }
        search = end + 1;
    }
    None
}

fn linux_pins() -> (String, String) {
    (
        hex16_literal_after(LINUX_FILE, "const ROUND_TRIP_C_DIGEST: &str = ").expect(
            "f02_trace_relation.rs no longer spells ROUND_TRIP_C_DIGEST as a 16-hex literal",
        ),
        hex16_literal_after(LINUX_FILE, "const CONCURRENT_C_DIGEST: &str = ").expect(
            "f02_trace_relation.rs no longer spells CONCURRENT_C_DIGEST as a 16-hex literal",
        ),
    )
}

fn windows_pins() -> (String, String) {
    (
        hex16_literal_after(WINDOWS_FILE, "\"round-trip\",").expect(
            "windows_threads.rs no longer pins a 16-hex digest after its \"round-trip\" row",
        ),
        hex16_literal_after(WINDOWS_FILE, "\"concurrent\",").expect(
            "windows_threads.rs no longer pins a 16-hex digest after its \"concurrent\" row",
        ),
    )
}

/// The extraction still reaches both pin sites and the same FNV-1a constants.
#[test]
fn both_f02_pin_sites_are_still_where_this_test_reads_them() {
    let (linux_round_trip, linux_concurrent) = linux_pins();
    let (windows_round_trip, windows_concurrent) = windows_pins();
    for pin in [
        &linux_round_trip,
        &linux_concurrent,
        &windows_round_trip,
        &windows_concurrent,
    ] {
        assert_eq!(pin.len(), 16, "{pin}");
    }
    for constant in ["0xcbf2_9ce4_8422_2325", "0x0000_0100_0000_01b3"] {
        assert!(
            LINUX_FILE.contains(constant),
            "f02_trace_relation.rs lost {constant}"
        );
        assert!(
            WINDOWS_FILE.contains(constant),
            "windows_threads.rs lost {constant}"
        );
    }
}

#[test]
fn the_windows_f02_pins_agree_with_the_linux_ones() {
    let (linux_round_trip, linux_concurrent) = linux_pins();
    let (windows_round_trip, windows_concurrent) = windows_pins();
    assert_eq!(
        (windows_round_trip.as_str(), windows_concurrent.as_str()),
        (linux_round_trip.as_str(), linux_concurrent.as_str()),
        "windows_threads.rs pins a digest pair f02_trace_relation.rs disproves (F02-DIGEST-PIN-DUPLICATED)"
    );
}
