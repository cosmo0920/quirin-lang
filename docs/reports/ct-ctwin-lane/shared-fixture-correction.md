# Correction note: §6.1's duplicate sweep

Card `CT-SHARED-FIXTURE-DISCLOSURE` wrote this note on 2026-09-23. It sits beside
the frozen [report](README.md), which stays unchanged (constraint 21).

A re-measurement of §6.1 corrects three points. The finding itself stands: the
two pairs it names are real and were undisclosed.

1. **The hashes in the §6.1 table are MD5, not SHA-256, and the table does not
   say so.** `a842abac…`, `1e1761bb…` and `63e938d3…` are the MD5 prefixes of
   `CT01/case.qcore`, `CT02/case.qcore` and `CT03/case.qcore`. The same files'
   SHA-256 digests are `131f635d…`, `c775f721…` and `7c75577b…`, and
   `CT-SHARED-FIXTURE-DISCLOSURE` records the SHA-256 digests. Both identify the
   same groups.
2. **The hazard runs the other way.** §6.1 says that editing `CT02/case.qcore`
   or `CT03/case.qcore` "silently changes what CT14's and CT04's controls
   test". It does not. The harness reads each control from the controlling case's
   own directory (`directory.join(&control.source)`), and those files are
   separate copies. An edit to either side leaves the other file untouched. What
   it silently ends is the *identity*: the control stops being the other case's
   program, and nothing said so until now.
3. **The sweep covered only `.qcore`/`.c` files under `fixtures/ct/`.** Hashing
   every file under `compiler/tests/fixtures/` finds **nine** groups that
   contain a CT file, not three. The sweep could not see six of them. Two are
   header groups (`CT01`/`CT12` `owned_transfer.h`; `CT14`/`CT37`
   `completion_before_join.h`), which wave 37's `CT-LANE-5` §9 later measured.
   The other four are copies shared with non-CT fixtures (CT08, CT09 and both
   of CT10's controls). This card measures those four for the first time.

All nine groups are now disclosed and pinned by
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`.
`CT-FIXTURE-CROSS-CASE-DUPLICATES` retires against
[that report](../ct-shared-fixture-disclosure/README.md).
