# Windows test portability and F02 pin repair

Recorded on 2026-09-23 on Windows MSVC. The checkout uses CRLF for tracked
text files; all source and policy comparisons in this change handle that
checkout spelling in memory. No tracked file was converted to LF on disk.

The Windows F02 native test previously observed `bfa572914cb96413` while its
pin expected `6df369991465940e`. A Windows run now passes with the emitted-C
pair `bfa572914cb96413` and `9cbe67a70a420d52`. The host-unconditional
comparison of the Windows and Linux pin sites is active and passes. The
[native run](windows-f02.log) and [pin agreement run](f02-agreement.log) each
record exit 0, with 1/1 and 2/2 tests passing. The agreement reader was
previously mutation-tested by `STABILITY-REEVAL-3`; its assertion is no
longer ignored. This discharges `F02-DIGEST-PIN-DUPLICATED`,
`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`, and
`F02-AGREEMENT-TEST-SURFACED-BY-PROSE-ONLY`. The live acceptance matrix
retired those three entries and its derived count moved from 285 to 282.

Windows uses LLP64. Linux LP64 source summaries and pthread fixtures cannot
be treated as native Windows evidence. The Windows tests now assert the
specific parser, source-identity, or projection refusal at that boundary;
the corresponding Linux evidence assertions remain in their own host tests.
Windows native scalar-call and write/lend-task tests execute with MSVC.
Fixture mutations and pinned digest comparisons account for checkout CRLF
only in memory. This includes the CT07 single-writer fixture mutation, whose
multiline anchor now matches the in-memory source on Windows. These changes
do not promote any Unknown verdict to Safe.

The task's private validation logs under
`private/windows-test-fixes-20260923/` record the workspace, formatting,
Clippy, example, Lean, and Python gate runs, including earlier failed
attempts and their corrected reruns. The two early full workspace runs
stopped at Windows linker file locks while another Cargo run used the same
test binaries; they are not treated as a passing gate.
