# O01 — One reopen corruption/checksum observation family — 2026-09-08

A **native evidence** card, closed by a Rust implementation and an audited Lean
formalization, with the native fixture family as its measured input.

**Closing evidence.**

| Deliverable | Contents |
|---|---|
| [`crates/qcore-core/src/chunk_integrity.rs`](../../../crates/qcore-core/src/chunk_integrity.rs) | the relation **decided in Rust** — content classification derived from the recorded bytes with an independent crc32, the injection axis kept separate, allocation and endpoint custody split between library and harness, a total-or-reject record parser — with **44 tests**, 30 of them a mutation matrix |
| [`compiler/examples/chunk_file_integrity_observation.c`](../../../compiler/examples/chunk_file_integrity_observation.c) | the native observer: eight exclusive fixtures, real `open`/`mmap`/`close` against the pinned ChunkIO sources under `CIO_CHECKSUM` |
| [`proof/QCore/ChunkIntegrityObservation.lean`](../../../proof/QCore/ChunkIntegrityObservation.lean) | the matching Lean model, **42 theorems** |
| [`proof/AuditChunkIntegrityObservation.lean`](../../../proof/AuditChunkIntegrityObservation.lean) | the axiom audit — every theorem depends only on `propext`, `Classical.choice` and `Quot.sound` (32 → **33 audits**) |
| [`contract/qcore/chunkio/filesystem_integrity_observation.qcore`](../../../contract/qcore/chunkio/filesystem_integrity_observation.qcore) | the declared observation policy, 21 clauses, evaluated over the recorded executions |
| [`logs/`](logs/) | the archived run: `records.txt`, `observations.jsonl`, `metadata.json`, `processes.jsonl`, `validation.json`, `contract_result.json`, and each process's `case-N.stderr` |

**Supplementary, not the closing evidence.**

| Artifact | Contents |
|---|---|
| [`scripts/cycle19_file_integrity_observations.py`](../../../scripts/cycle19_file_integrity_observations.py) | preparation only: extract the pinned sources, configure, build under sanitizers, seed, run, hand the records to the Rust validator |
| [`scripts/tests/test_file_integrity_fixtures.py`](../../../scripts/tests/test_file_integrity_fixtures.py) | 6 supplementary tests on the seeds and the archived identities |

## 1. What is observed

Eight processes, one per fixture, each on its own exclusive temporary
directory, each opening one chunk of one stream through the **real pinned
ChunkIO** `cio_chunk_open` → `cio_file_open` path with `CIO_CHECKSUM` enabled,
so the format check runs its crc32 comparison over the mapped content.

`CIO_CHECKSUM` is the reason this family exists. The existing
[existing-file profiles](../cycle19-file-existing/README.md) — recorded in
[`docs/reports/cycle19-reopen-failures`](../cycle19-reopen-failures/README.md)
and its siblings — never enabled it, so `CIO_ERR_BAD_CHECKSUM` had never been
reached by any recorded execution in this repository. It is reached here.

### 1.1 The two axes, and why they are separate

| Axis | What it is | Where it comes from |
|---|---|---|
| **content class** | what the bytes on disk are | derived from the recorded image, by repeating the pinned layout and checksum rules in Rust with an independent crc32 |
| **injected operation failure** | which call the harness suppressed | a property of the wrapper: a fixed error returned **without reaching the kernel** |

They are recorded in separate fields, decided by separate rules, and a record
that claims one because the other happened is refused (§4.2, rows M01–M10).
The measured reason they cannot be collapsed is in the recorded data itself:

* a corrupted image reports `CIO_CORRUPTED` (`-3`) from `cio_chunk_open`'s
  out-parameter and leaves a fine-grained code on the chunk (`-10`, `-11`,
  `-13`);
* a **clean** image whose `mmap` was suppressed reports `CIO_ERROR` (`-1`) and
  leaves **no** fine-grained code at all — the same `0` a fully successful open
  leaves.

So the fine-grained error code alone cannot tell a suppressed call from a clean
success, which is exactly why the class is derived from the bytes and never
read out of the report. That is `the_error_code_alone_cannot_separate_the_axes`
in Lean and `a_corrupted_input_and_an_injected_failure_are_distinct_classes` in
Rust.

## 2. Identities

Everything the run compiled and executed is pinned or recorded.

| Identity | Value |
|---|---|
| Fluent Bit revision | `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` |
| `lib/chunkio/src/cio_file.c` | `1c3eb285307108deb2b22c68cf282c0121619a35bbe36adc216f276d51925257` |
| `lib/chunkio/deps/crc32/crc32.c` | `ce628f5a1a3f63bb17d144f77ee0595ff7869047fc70f59cd0a48853e00e0803` |
| `lib/chunkio/include/chunkio/cio_file_st.h` | `02eb39090bf82c246c1fc3b29f3b121259a066656956c8ac5fb2f4cbc13d9133` |
| observer source | recorded in `logs/metadata.json` as `observer_sha256` |
| shared observer header, memory observer | `shared_observer_sha256`, `memory_observer_sha256` |
| driver, validator, validator example, contract | `driver_sha256`, `validator_sha256`, `validator_example_sha256`, `contract_sha256` |
| built binary | `binary_sha256` |
| compile database | `database_sha256` |
| records, observations, contract result | `records_sha256`, `observations_sha256`, `contract_result_sha256` |
| every unit's source | `pinned_source_sha256`, all 14 |
| compiler, configure | Ubuntu clang 21.1.8, CMake 4.2.3, 15 recorded commands |
| host | Linux `7.0.0-30-generic`, `x86_64` |

The three load-bearing files are compared against constants inside the driver
and a mismatch **refuses before anything is compiled**. The other eleven are
recorded, and are pinned transitively by the commit.

### 2.1 One recorded premise about the build

The pinned Fluent Bit **build tree** is not available in this environment: the
developer checkout has moved off the pin, and the existing observation drivers
require `<fluent-bit>/build/compile_commands.json` at the pinned revision. This
family therefore extracts the pinned `lib/chunkio` subtree into an exclusive
directory and configures **ChunkIO's own standalone CMake** with Clang, which
generates the configure-time `cio_info.h` and a compile database of exactly the
14 units. Only the preprocessor arguments that configure produced are reused
(`-D__FILENAME__=__FILE__`, the three `CIO_HAVE_*` definitions and the include
path set); the sanitizer, optimization and instrumentation arguments are the
harness's, and every command is recorded.

**This is a recorded premise, not an equivalence claim.** The compiled *source
bytes* are the pinned upstream blobs; the *build configuration* is ChunkIO's
own, not Fluent Bit's. No claim is made that a Fluent Bit build of the same
sources would produce the same object code, and nothing here is compared
against the pinned 185-unit artifacts.

## 3. The fixture family and its post-states

Each fixture is a 4096-byte image (case 403 is 8 bytes) built from the pinned
24-byte layout: `0xC1 0x00`, a big-endian crc32 of the content region at offset
2, a big-endian content length at offset 10, a metadata length at offset 22,
then the four payload bytes `seed`.

| Case | Profile | Content class | Injection (suppressed) | Chunk | `open` err | chunk err | alloc acq/lib/out | fd epoch/lib/harness | fd residual | map epoch/lib | harness reclaimed |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 400 | `clean_checksum` | clean | none (0) | 1 | 0 | 0 | 10/10/0 | 1/1/0 | 0 | 1/1 | 0 |
| 401 | `bad_checksum` | bad_checksum | none (0) | 0 | -3 | -10 | 10/10/0 | 1/1/0 | 0 | 1/1 | 0 |
| 402 | `bad_magic` | bad_magic | none (0) | 0 | -3 | -11 | 10/10/0 | 1/1/0 | 0 | 1/1 | 0 |
| 403 | `truncated_header` | truncated_header | none (0) | 0 | -3 | -13 | 10/10/0 | 1/1/0 | 0 | 1/1 | 0 |
| 404 | `truncated_content` | truncated_content | none (0) | 0 | -3 | -13 | 10/10/0 | 1/1/0 | 0 | 1/1 | 0 |
| 405 | `clean_injected_map` | clean | mmap (1) | 0 | -1 | 0 | 10/10/0 | 1/1/0 | 0 | 0/0 | 0 |
| 406 | `bad_checksum_injected_close` | bad_checksum | close (1) | 0 | -3 | -10 | 10/10/0 | 1/0/1 | 1 | 1/1 | 1 |
| 407 | `clean_injected_close` | clean | close (1) | 1 | 0 | 0 | 10/10/0 | 1/0/1 | 1 | 1/1 | 1 |

Five recorded classes, three suppressed operations, and case 406 carries both
axes at once without either standing in for the other.

### 3.1 Content post-states

**Every one of the eight fixtures is byte-identical before and after**, checked
by comparing the full recorded images, not a length or a digest of a prefix. In
particular:

* the corrupted images are **not repaired** — case 401's edited payload byte
  and stale stored checksum are both still there afterwards, and case 402's
  destroyed magic byte is still zero;
* case 404's declared content length is not rewritten, so the inference branch
  of `cio_file_st_get_content_len` was not taken;
* the clean control wrote nothing at all.

Device and inode identity, and file size, survive every case.

### 3.2 Endpoint post-states, library against harness

Descriptor and mapping epochs are counted separately, and each is split between
what the **library** released through a forwarded call and what the **harness**
reclaimed after the parent context was freed:

* 8 descriptor epochs acquired: **6 library closes**, **2 harness
  reclamations** — exactly the two suppressed-close cases, and nothing else;
* 7 mapping epochs acquired, **7 library unmaps**, **0 harness reclamations**;
  case 405 acquires no mapping epoch at all, because the suppressed `mmap`
  stored `MAP_FAILED`;
* 80 allocation generations acquired across the family, **80 released by the
  library**, **0 by the harness**, **0 outstanding**.

A suppressed close leaves the descriptor open *because the syscall never ran*.
Per [O03](../remaining-o03/README.md) §3, that is the POSIX.1-2024 post-state,
not the Linux one, and this card does not derive a
`qcore_core::os_endpoint` post-state from it. The Rust test
`a_suppressed_call_yields_no_operating_system_post_state` pins that the verdict
carries the injection and the residual and nothing else.

### 3.3 Sanitizers

All eight processes were built with `-fsanitize=address,undefined -O0 -g
-fno-omit-frame-pointer -fno-builtin`, ran under
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and **all eight exited 0**.
The only diagnostics on stderr are the library's own three
`cio_file_unix.c` error lines for the injected `EIO` returns, archived in
`logs/` alongside each case. LeakSanitizer requires execution outside ptrace
containment, as the earlier Cycle 19 packages record.

### 3.4 Repetition

The family was run twice, into two different output directories. All eight case
blocks came back **byte-identical**; the only difference in the whole record
text is the recorded `binary=` digest, because the build embeds absolute paths
and the two output directories differ. This is a repetition observation on one
host, not a reproducible-build claim.

## 4. The Rust relation — 44 tests

`validate_case` refuses; it never downgrades, and the only verdict it can reach
is `Verdict::Observed`. `policy_values` emits `static_certificate` and
`durability_proved` as constant zero, and a test pins that.

### 4.1 The classification is independent

`crc32` is a self-contained bitwise CRC-32 (reflected, `0xedb88320`, init and
final xor `0xffffffff`), checked against the standard `"123456789"` →
`0xcbf43926` value, so it agrees with ChunkIO's `deps/crc32` table-driven
implementation without calling it. `classify_content` repeats the pinned
decision order — length, magic, declared lengths, checksum — and tests pin that
an image which is both magic-broken and checksum-broken is classified by the
earlier rule, exactly as the observed library decides it.

### 4.2 The mutation matrix — 30 rows, all refused

| # | Mutation | Refusal |
|---|---|---|
| M01 | claim a corruption that is not in the bytes | `DeclaredClassIsNotInTheBytes` |
| M02 | repair the stored checksum while still claiming the failure | `DeclaredClassIsNotInTheBytes` (derived `Clean`) |
| M03 | the image changed class across the execution | `ContentClassChanged` |
| M04 | a corrupted input borrows an injection's `open` error | `OutcomeDisagreesWithClassification` |
| M05 | an injected failure borrows a corruption code | `OutcomeDisagreesWithClassification` |
| M06 | a refused open reports a returned chunk | `OutcomeDisagreesWithClassification` |
| M07 | a clean open reports a refusal | `OutcomeDisagreesWithClassification` |
| M08 | an injection with no suppressed call | `InjectionIsNotAccountedFor` |
| M09 | a suppressed call with no injection | `InjectionIsNotAccountedFor` |
| M10 | a suppressed call with errno zero | `InjectedErrnoDisagrees` |
| M11 | the allocation ledger does not balance | `AllocationsDoNotBalance` |
| M12 | a generation is left outstanding | `OutstandingAllocations` |
| M13 | the harness claims a library allocation release | `HarnessClaimedAnAllocationRelease` |
| M14 | a suppressed close credited to the library | `LibraryCreditedWithASuppressedCall` |
| M15 | the descriptor ledger does not balance | `EndpointsDoNotBalance` |
| M16 | the mapping ledger does not balance | `EndpointsDoNotBalance` |
| M17 | a residual with no reclamation | `ResidualAndReclamationDisagree` |
| M18 | a reclamation with no residual | `ResidualAndReclamationDisagree` |
| M19 | a residual that no suppression explains | `ResidualIsNotExplainedBySuppression` |
| M20 | the fixture's inode was replaced | `FixtureIdentityChanged` |
| M21 | the fixture was resized | `FixtureIdentityChanged` |
| M22 | an unknown record schema | `Malformed` |
| M23 | a missing identity line | `Malformed` |
| M24 | a truncated record block | `Malformed` |
| M25 | a block that closes on another case | `Malformed` |
| M26 | an unknown content class name | `Malformed` |
| M27 | an unknown injected operation name | `Malformed` |
| M28 | an odd-length byte image | `Malformed` |
| M29 | empty record text | `Malformed` |
| M30 | one forged case in an otherwise real family | the whole family is refused |

M14 is the row the card's second acceptance item names directly: moving one
suppressed close from the harness column to the library column, and adjusting
the residual and the reclamation count to match, still refuses — the separation
cannot be laundered by making the totals agree.

The remaining 14 tests are positive: the archived family validates, covers all
five classes and all three injections, changed no fixture content, released
every allocation generation through the library, left a descriptor for the
harness only where a close was suppressed, acquired no mapping epoch for the
suppressed `mmap`, and never publishes a certificate.

## 5. The Lean model — 42 theorems

`QCore.ChunkIntegrityObservation` states the same relation over an abstract
`Image` record. Extracting an image's size, identification bytes, declared
lengths and checksums from the bytes is an explicit **premise** on this side —
the Rust side recomputes all of them — as is the claim that a recorded
execution is the execution it says it is.

| Group | Theorems |
|---|---|
| enumerations and classification | `mem_allClasses`, `mem_allInjections`, `every_content_class_is_reachable`, `a_short_image_is_a_size_failure_whatever_else_is_wrong`, `broken_magic_is_decided_before_the_checksum`, `a_clean_image_satisfies_every_rule`, `a_disagreeing_checksum_is_a_checksum_failure` |
| the two axes stay separate | `an_injection_never_changes_the_derived_class`, `a_corruption_is_never_an_injected_failure`, `the_error_code_alone_cannot_separate_the_axes`, `a_corrupted_image_decides_the_outcome_alone`, `only_a_corrupted_image_reports_corruption`, `the_corruption_codes_are_pairwise_distinct`, `a_chunk_is_returned_only_for_a_clean_unsuppressed_open` |
| custody | `accepted_allocations_balance`, `accepted_leaves_no_outstanding_generation`, `the_harness_never_releases_a_library_allocation`, `accepted_endpoint_epochs_are_split`, `a_residual_is_exactly_a_harness_reclamation`, `a_suppressed_call_is_never_credited_to_the_library`, `only_a_suppressed_close_leaves_a_descriptor_residual`, plus the five acceptance projections |
| the verdict | `no_arm_publishes_anything_but_an_observation`, `a_refused_record_publishes_nothing`, `a_verdict_carries_both_axes_separately`, `a_verdict_reports_only_harness_reclamations` |
| non-vacuity and refusals | `the_relation_accepts_the_recorded_shapes`, `the_combined_case_keeps_both_axes`, and ten refusal theorems mirroring M01–M21 |

Four of them (`a_corruption_is_never_an_injected_failure`,
`a_corrupted_image_decides_the_outcome_alone`,
`only_a_corrupted_image_reports_corruption`,
`a_chunk_is_returned_only_for_a_clean_unsuppressed_open`) are decided over the
whole `5 × 3` class-by-injection domain.

No new axiom, no `sorry`, no placeholder.

## 6. Deliberate pin updates

Two pinned expectations were updated on purpose, because a new `.qcore` module
was added:

| Pin | Before | After |
|---|---|---|
| `compiler/tests/qcore_contract_corpus.rs` corpus size | 59 | **60** |
| its clause table | — | `+ ("chunkio/filesystem_integrity_observation.qcore", "native_file_integrity_observation", 21)` |
| `compiler/tests/contract_loader.rs` module count | 59 | **60** |
| its `(selected, v1_declarations, rejected_declarations)` | `(5, 19, 144)` | `(5, 19, **145**)` |

The new module carries **no** `requires resource_contract_v1` marker, so the
resource loader rejects it rather than loading it — the same status every other
observation policy has. The five selected resource projections and their 19
assumptions are unchanged; only the rejected count moves.

## 7. Acceptance items

| # | Item | Where |
|---|---|---|
| 1 | pin source/build/observer hashes; distinguish corrupted input from injected operation failure | §2 (three refusing constants plus fourteen recorded source digests, observer, binary, database, validator and contract digests); §1.1, §3, and the Rust/Lean separation results — `ContentClass` derived from bytes with an independent crc32, `Injection` recorded separately, M01–M10 refusing every crossover, and the measured `-3`/`-1` and `-10`/`0` split |
| 2 | preserve residual custody on every observed failure, with library and harness releases separated | §3.2 — 80/80/0 allocation generations, 8 descriptor epochs as 6 library closes and 2 harness reclamations, 7 mapping epochs all library; `EndpointLedger` and `AllocationLedger` carry both columns, `check_endpoint` requires them to balance *and* requires a suppressed call to leave the harness something to do; M13, M14, M17, M18, M19; Lean `the_harness_never_releases_a_library_allocation`, `a_residual_is_exactly_a_harness_reclamation`, `a_suppressed_call_is_never_credited_to_the_library` |
| 3 | replay/mutation tests and appropriate native sanitizers pass in the declared environment | §4.2 (30 mutations, all refused; the archived family replays from the committed record text), §3.3 (eight ASan/UBSan/LSan processes, all exit 0), §8 |

Out of scope, as the card requires: **no universal corruption recovery** — the
family says what four specific malformed images did on one host under one
build, and the `Verdict` type has no other value — and **no static `Safe`
promotion**: no raw Cycle 19 unit moves, no analyzer code was touched, and the
policy's `static_certificate` and `durability_proved` are constant zero.

## 8. Gate results

Run in this card's isolated worktree, based at `848fa32`, with the Libevent
submodule initialized. The Lean toolchain is at `~/.elan/bin`; both Lean gates
were run with `PATH="$HOME/.elan/bin:$PATH"`.

| Gate | Command | Before | After | Exit |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace` | 1387 passed, 1 ignored | **1431 passed, 0 failed, 1 ignored** (+44) | 0 |
| Format | `cargo fmt --all -- --check` | clean | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | 0 | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | 89 | **95** (+6) | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | 32 | **33** | 0 |
| Lean axiom surface | `lake env lean -DwarningAsError=true AuditChunkIntegrityObservation.lean` | — | all **42** theorems on `propext` / `Quot.sound` only | 0 |
| Provider tests | `cargo test --manifest-path providers/Cargo.toml --workspace` | 58 | 58 (untouched) | 0 |
| Native family | `python3 -B scripts/cycle19_file_integrity_observations.py --fluent-bit <checkout> --output <dir>` | — | 8 processes, all exit 0; 8 validated; 8 contract-checked | 0 |

## 9. Verdicts carried forward unchanged

| Item | Value | Changed by O01 |
|---|---|---|
| Raw Cycle 19 units | 2 Safe / 42 provisional Unsafe / 137 Unknown + 4 data-only | no |
| Pinned 185-unit artifacts | untouched (CORPUS-REPIN remains open) | no |
| CT coverage | 20/50 mapped | no |
| Contract corpus | 59 → **60 modules** (§6) | yes, deliberately |
| `qcore-schemas` | 5.0.0 | no |
| Full language conformance | false | no |

## 10. Files written

| Path | Change |
|---|---|
| `crates/qcore-core/src/chunk_integrity.rs` | **new** — the relation and its 44 tests |
| `crates/qcore-core/src/lib.rs` | module declaration and re-exports only |
| `crates/qcore-core/tests/data/o01_records.txt` | **new** — the archived record text the tests replay |
| `compiler/examples/chunk_file_integrity_observation.c` | **new** — the native observer |
| `compiler/examples/validate_integrity_observations.rs` | **new** — records → observations, refusing on any refusal |
| `compiler/examples/native_file_profile_observer.h` | additive: endpoint call tallies, a context-flags hook and an `mmap` suppression, all defaulting to the existing behaviour |
| `contract/qcore/chunkio/filesystem_integrity_observation.qcore` | **new** — the observation policy |
| `compiler/tests/qcore_contract_corpus.rs`, `compiler/tests/contract_loader.rs` | deliberate pin updates (§6) |
| `proof/QCore/ChunkIntegrityObservation.lean`, `proof/AuditChunkIntegrityObservation.lean`, `proof/QCore.lean` | **new** model, **new** audit, one import line |
| `scripts/cycle19_file_integrity_observations.py` | **new** — the preparation driver |
| `scripts/tests/test_file_integrity_fixtures.py` | **new** — 6 supplementary tests |
| `docs/reports/remaining-o01/README.md`, `logs/*` | this record and the archived run |

`AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance.json`,
`docs/reviews/r0_3/acceptance_matrix.json` and `crates/qcore-schemas` were not
edited; §12 proposes the ledger text instead. No existing observation driver,
checker, analyzer or diagnostic code was changed, and **no diagnostic code was
added** — this card introduces no diagnostics at all, so the fully assigned
`Q610`–`Q677` bands are untouched.

## 11. Open boundaries

* **Not wired into any production audit route.** `qcore-core` is a
  dependency-free kernel; the only caller of `chunk_integrity` is the
  `validate_integrity_observations` example and the tests. Nothing in
  `qcore-audit`, the gateway or the Cycle 19 compatibility adapter consumes it.
* **Eight fixed executions on one host.** The family says nothing about other
  images, other filesystems, other kernels, concurrent access, or ChunkIO's
  behaviour on inputs it did not see.
* **No corruption *recovery* is observed.** Every corrupted case is refused at
  open; `cio_file_read_prepare`, the `CIO_RETRY` path and `cio_scan`'s recovery
  of a corrupted stream are not exercised. That is the obvious follow-on.
* **Metadata is always empty.** Every fixture declares a zero metadata length,
  so the checksummed region is always the two length bytes plus four payload
  bytes. Nonzero metadata, `adjust_layout` and `cio_file_write_metadata` are
  untouched.
* **`CIO_TRIM_FILES`, `CIO_FULL_SYNC` and the taint flag are off**, so the
  inference branch of `cio_file_st_get_content_len` — the one branch that
  *writes* to a mapped image during classification — was never taken. A fixture
  that takes it would be the first case where the recorded content legitimately
  changes, and the relation would refuse it today (`ContentClassChanged` /
  `content_changed`), by design: it is out of the declared profile.
* **The build configuration is ChunkIO's own, not Fluent Bit's** (§2.1), and
  no object-code equivalence with a Fluent Bit build is claimed.
* **The Rust and Lean models are two statements of one relation**, written to
  agree and checked separately; no extraction or mechanical correspondence ties
  them, the same boundary O03 and F01 record.
* **A suppressed call is not an OS post-state**, and no `os_endpoint`
  post-state is derived from one here. Binding the two would need the real,
  unsuppressed failure O03's unit U7 names.

## 12. Proposed ledger text (for the integrator to consolidate)

> **Reopen corruption/checksum observations (O01):** a bounded eight-fixture
> family executes the pinned ChunkIO reopen path with `CIO_CHECKSUM` enabled,
> reaching `CIO_ERR_BAD_CHECKSUM` for the first time in a recorded run.
> Five content classes — clean, bad checksum, bad magic, truncated header,
> truncated content — and three injected operation failures (none, suppressed
> `mmap`, suppressed `close`) are recorded on **separate axes**: the class is
> derived from the recorded image by an independent crc32 and the pinned layout
> rules, the injection is a property of the harness wrapper, and a record that
> claims one because the other happened is refused. The measured reason they
> cannot be merged is that a clean image with a suppressed `mmap` reports
> `CIO_ERROR` and leaves the same zero fine-grained code a successful open
> leaves, while every corrupted image reports `CIO_CORRUPTED` with a distinct
> code. All eight fixtures are byte-identical before and after — corrupted
> images are not repaired — and device, inode and size survive. Eighty
> allocation generations are acquired and all eighty released by the library;
> eight descriptor epochs yield six library closes and two harness reclamations,
> exactly the two suppressed-close cases; seven mapping epochs are all released
> by the library, and the suppressed `mmap` acquires none. Library releases and
> harness reclamations are separate counters and a suppressed close credited to
> the library is refused even when the totals are made to agree. The relation is
> **decided in Rust** in the dependency-free
> `crates/qcore-core/src/chunk_integrity.rs` (44 tests, 30 of them a mutation
> matrix over the archived record text, all refused) and **formalized in Lean**
> as `QCore.ChunkIntegrityObservation` (42 theorems in a new
> `AuditChunkIntegrityObservation`, four decided over the whole class-by-
> injection domain, depending only on `propext` and `Quot.sound`). A new 21-clause
> `contract/qcore/chunkio/filesystem_integrity_observation.qcore` policy
> evaluates all eight recorded executions; the contract corpus moves 59 → 60 and
> the loader's rejected-declaration count 144 → 145, both deliberate pin updates,
> with the five v1-selected projections unchanged. Eight sanitized processes
> (ASan/UBSan/LSan) exit 0 on Linux `7.0.0-30-generic` with Clang 21.1.8. The
> pinned Fluent Bit build tree is unavailable, so the family compiles the pinned
> `lib/chunkio` sources — three of them checked against refusing digest constants
> — through ChunkIO's own standalone CMake; that build configuration is a
> recorded premise and no object-code equivalence with a Fluent Bit build is
> claimed. **No production audit route consumes the module**, no analyzer code
> changed, no diagnostic was added, and nothing is promoted: the recorded run is
> an observation under its declared environment, never an OS or durability
> guarantee, and a suppressed call yields no `os_endpoint` post-state.
> Gate: 1,431 workspace tests, 33 Lean audits, 95 Python tests, 58 provider
> tests, both reevaluation examples exit 0, fmt and clippy clean. Evidence:
> `docs/reports/remaining-o01/README.md`.

Acceptance-matrix rows this card supports (O01): the corruption/checksum
fixture family, its Rust decision procedure and its Lean formalization. It
supplies **no** production-integration evidence class.
