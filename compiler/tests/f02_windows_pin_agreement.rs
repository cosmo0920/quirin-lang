//! The Linux-side half of `F02-DIGEST-PIN-DUPLICATED`'s discharge, landed by
//! `WAVE-38-CLOSE` (wave 38); the Windows half stays open as
//! `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`.
//!
//! The two F02 emitted-C digests are pinned twice: in `f02_trace_relation.rs`
//! (Linux-reachable) and in `windows_threads.rs` (behind
//! `cfg(target_os = "windows")`, whose comment asks the reader to "keep these
//! pins paired"). Both files compute the same FNV-1a over the same LF bytes of
//! the same two programs, so the literals must agree — and until this file
//! nothing checked that they did. This test reads both files as text (so it
//! runs on every host, Windows `cfg` notwithstanding) and compares the pins.
//!
//! It is `#[ignore]`d on purpose. At `4f663934` the two files DISAGREE:
//! `windows_threads.rs` still carries the fourth-move pair
//! (`6df369991465940e` / `0da423be2f16b8bb`) while `f02_trace_relation.rs`
//! pins the fifth (`bfa572914cb96413` / `9cbe67a70a420d52`), so the Windows
//! F02 row has been red on every Windows host since `fa48211a` and no Linux
//! gate could see it (card `STABILITY-REEVAL-3`, wave 38, §1.2). The Windows
//! literals may move only in a card that records a Windows run of
//! `windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins`
//! (the wave-38 close's deferred decision, in the `HTTP-V1-WINDOWS-RUN`
//! pattern); re-pinning them from Linux is the shape `37bc9004` was named for.
//! Run it on demand with
//! `cargo test -p qcore-compiler --test f02_windows_pin_agreement -- --ignored`:
//! it is red until that card lands and must go green with it, at which point
//! the `#[ignore]` comes off and both residuals retire. The un-ignored
//! companion below keeps the reader honest: if either literal site moves out
//! from under the extraction, the standing gate says so.

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

/// Runs in the standing gate: the extraction still reaches both pin sites, and
/// both files still spell the same FNV-1a constants, so the ignored comparison
/// below compares what it claims to compare.
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

/// Red at `4f663934` by design (see the module comment); green only once a
/// Windows-run card re-pins `windows_threads.rs`.
#[test]
#[ignore = "F02-WINDOWS-PIN-STALE-SINCE-WAVE-36: the Windows twin pins the fourth-move digests; its re-pin needs a Windows run of windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins (wave-38 close, deferred). Run with --ignored to reproduce the red."]
fn the_windows_f02_pins_agree_with_the_linux_ones() {
    let (linux_round_trip, linux_concurrent) = linux_pins();
    let (windows_round_trip, windows_concurrent) = windows_pins();
    assert_eq!(
        (windows_round_trip.as_str(), windows_concurrent.as_str()),
        (linux_round_trip.as_str(), linux_concurrent.as_str()),
        "windows_threads.rs pins a digest pair f02_trace_relation.rs disproves (F02-DIGEST-PIN-DUPLICATED)"
    );
}
