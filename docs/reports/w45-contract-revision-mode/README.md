# W45-CONTRACT-REVISION-MODE: pinned and flexible contracts, pinned by default

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Base `74579af6` (local `main`, "Record the w45-fb-retry-fault-injection
integration gate"), with `main` at `84ebad57` (W45-FB-RETRY-LIFETIME
integrated) merged in at the integration review. A resource-v1 contract, or one clause of it, now says
`revision pinned` or `revision flexible`, and **pinned is the default**. A
pinned contract is in force only where its text is the pinned one and the
audited source matches one of the **reviewed identities** recorded for it in a
generated lock (`contracts.lock`, one per provider crate). Each identity names
the revisions it was reviewed at; the committed locks carry both revisions this
repository audits, the corpus's `66910c10` and upstream `046accf6`.
Every mismatch is withheld: calls relying on it are `Unknown` under the new
reason code `contract-pin-mismatch`, never `Safe` or `Unsafe`. A flexible
contract binds by name and declared signature. It is checked against any body
the run analyses, and on a trusted function it is a listed assumption. Every
libc contract is flexible and forms one standing assumption. `quirin-audit pin`
writes the locks after review, and `quirin-audit compare` reports the change
between two runs.

The corpus arbiter's diagnostics are byte-identical to the current baseline,
W45-FB-RETRY-LIFETIME's integration artifacts, and before the merge they were
byte-identical to the W45-PARENTHESISED-CALLEE baseline (§5). The retry card's
system slice gives the same verdicts under the revision mode (§6.1). The normative text is contract
specification [C06.2](../../QCORE_CONTRACT_SPEC_r0_3.md#c062-revision-modes-pinned-and-flexible-contracts),
the grammar is [G05](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1),
and the reason category is a new row of language specification §L21.1.

## 1. Decisions

**Syntax** follows the deferred-review marker. It is contract-level, a
requirement beside the selector, or clause-level, a wrapper around one
`ensures normal` clause:

```qcore
extern fn malloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

    ensures normal resource_revision(resource_released(p, nonnull), pinned);
```

**Semantics of a clause mode.** A contract is applied at a call site as one
conjunction (C06.1), so no part of one can be in force without the rest. A
declaration is therefore flexible exactly when its contract-level mode is
flexible and no clause is marked pinned. An unmarked clause, the selector
included, takes the contract-level mode. One pinned clause pins the whole
contract. A clause marked flexible inside a pinned contract leaves it pinned,
and the report names the marked clause. The loader refuses the whole module for:

* a duplicate or conflicting contract-level mode;
* a clause mode that restates the contract-level one;
* an unknown mode word;
* a marker nested in a marker, whether a revision or a review marker, in either order;
* a clause marker on the selector or a literal premise;
* either form under the wrong introducer.

**Not inherited.** A mode records how a text was reviewed against its own
function's source, and an inheriting declaration names another function.
Elaboration therefore drops the parent's markers. Without this, `flb_malloc`
would silently have become flexible by inheriting `libc::malloc`.

**Pinned identity.** For each pinned declaration the lock records:

* the SHA-256 of the contract's normalised declaration text;
* one or more **reviewed identities**. Each is every top-level site of the
  function (file, kind, the SHA-256 of the normalised declaration and, for a
  definition, of the body) together with every source revision it was reviewed
  at. Revisions whose sites are equal share one identity.

Normalised means C tokens joined by one space, so comments and layout do not
count and every other edit does. For a Git checkout the sites are read from
the blobs of `HEAD` (`crates/qcore-audit/src/c_identity.rs`).

A pinned contract is in force exactly when its contract text is the pinned one
and the audited sites equal those of **some** reviewed identity. Every identity
names its revisions; there is no wildcard.

Several identities per entry were added at the integration review. The
repository audits Fluent Bit at two revisions at once: the corpus at
`66910c10` and the system slice at `046accf6`. A single identity per entry
would force a choice between them for every contract whose body differs.

Including the contract text is a choice beyond the brief. A pin is a review of
the pair, so an edited contract voids every identity (`contract-changed`) until
it is pinned again.

The mismatch reasons are `no-lock`, `not-in-lock` (no entry, or no identity
left), `contract-changed` and `source-changed` (the sites match no identity;
the detail names each identity's revisions and the files that differ).

**Where a lock lives.** A crate's lock sits beside its `Cargo.toml`. A module
outside every crate is covered by the nearest `contracts.lock` above it, up to
a `Cargo.toml` or Git directory. A qualified `crate:module` key resolves
through the load path.

**Lock format** (JSON, `lock_version` **2**, sorted, refused when malformed):

```json
{ "lock_version": 2, "generated_by": "quirin-audit pin", "note": "…do not edit by hand…",
  "owner": "qcore-provider-ctraces", "normalisation": "c-tokens-v1",
  "pinned": [ { "function": "ctr_destroy", "module": "ctraces/context_lifetime.qcore",
                "contract_sha256": "…",
                "identities": [
                  { "reviewed_at": [ { "kind": "git", "revision": "046accf6…" } ],
                    "sites": [ { "path": "lib/ctraces/include/ctraces/ctraces.h", "kind": "declaration", "declaration_sha256": "…" },
                               { "path": "lib/ctraces/src/ctraces.c", "kind": "definition", "declaration_sha256": "…", "body_sha256": "4b17…" } ] },
                  { "reviewed_at": [ { "kind": "git", "revision": "66910c10…" } ],
                    "sites": [ …, { "path": "lib/ctraces/src/ctraces.c", "kind": "definition", "declaration_sha256": "…", "body_sha256": "5bbd…" } ] } ] } ],
  "flexible": [ { "function": "…", "module": "…", "contract_sha256": "…" } ] }
```

The lock is refused when:

* an identity has no revision;
* a revision appears in two identities;
* two identities have equal sites;
* identities are not sorted by their first revision.

A directory tree (no Git) is the revision-less source `{"kind": "directory"}`.

Format version 1 (one source per lock, card version before the review) is
still read, each entry as one identity. The committed locks were migrated to
version 2 by `pin` itself; the migration left every entry unchanged
([log](locks/v2-migrate-66910c10.log)).

**`quirin-audit pin`** never runs during an audit.

* `quirin-audit pin --source-root DIR` **adds or refreshes** the identity of
  that checkout's `HEAD` for every declaration of every crate on the load path.
  It never drops an identity recorded at another revision. The concurrent
  cards' new contracts are pinned at integration with one command per reviewed
  revision.
* A declaration whose contract text changed starts again from this revision
  alone, and `pin` says so.
* `--function NAME` restricts the addition to named entries.
* `--regenerate` rebuilds each lock from this revision alone. It is the only
  form that replaces an unreadable lock.
* `--drop-revision REV` removes a revision from every identity, and an identity
  left with none goes. REV is a full revision, a prefix of at least seven
  characters, or `directory`. It reads no source.
* `--check` never writes. For the checkout's revision it reports each entry as
  recorded there, in force through an identity reviewed at another revision,
  not in force, or stale. It exits 1 unless every entry is recorded there.
  `--check --revision REV` reads REV's blobs from the object store instead of
  `HEAD`'s, with no checkout. It uses the same decision (`pin_standing`) as an
  audit.
* `--modules DIR` pins a directory outside the providers.
* `--verify-contracts` needs no source. It checks that each lock lists the
  current declarations, modes and contract digests.
  `providers/contracts.py check` runs it.

**libc.** Each of the 100 libc declarations gains `requires
resource_revision(flexible);`, and each module gains a short note. A report
lists every libc flexible contract without an analysed body under one standing
assumption, `libc-c-standard-abi`. The C library's version stays in the build's
provenance.

**Change report.** `quirin-audit compare --before REPORT --after REPORT`
(`revision_compare.rs`) reports:

* selected and missing sources;
* contracts in force added, removed and changed, by mode, status and text
  digest;
* pin mismatches, new and resolved;
* flexible and standing assumptions;
* per-function obligations added and removed, as (category, description)
  multisets without lines;
* verdict moves, with `violation_detected` (to `Unsafe`) kept separate from
  `coverage_lost` (`Safe` to `Unknown`, with the reason codes of its
  refusals).

## 2. Implementation

| Place | What changed |
|---|---|
| `crates/qcore-schemas/src/contract_revision.rs` (new), `contract.rs`, `reason.rs` | `RevisionMode`, `RevisionScope`, `RevisionMarker`, `DeclaredRevision`, `PinMismatchReason`, `PinMismatch`; `ResourceContracts::{revision, pin_mismatch}` (skipped when empty) and `withhold_for_pin_mismatch`; `ReasonCategory::ContractPinMismatch` and the summary's pin component. 15.0.0 → **16.0.0** |
| `crates/qcore-contract/src/contract_revision.rs` (new), `contract_loader.rs`, `contract_inclusion.rs` | marker parsing total-or-reject; `check_source_assumptions` accepts a withheld declaration only if pinned and equal to its fresh load; elaboration strips markers; `declaration_digests` |
| `crates/qcore-c-gateway/src/legacy_pin_hold.rs` (new), `legacy_c.rs`, `legacy_generic_selection.rs` | the hold, beside each review-hold hook: call sites of a withheld callee rewritten to `qcore_pin_mismatch__NAME`, refused, counted in the unit summary |
| `crates/qcore-audit/src/{c_identity,contract_revision,contract_pin,revision_compare}.rs` (new) | the scanner, locks, `apply_pins`, the `contract_revision` section, `pin`, `compare` |
| `current.rs`, `no_build.rs`, `lib.rs`, `compiler/examples/support/cycle19_compatibility.rs` | `apply_pins` right after loading; section and tally at the end (the configured route: section only) |
| `providers/quirin-provider-libc/contracts/libc/*.qcore`, `providers/*/contracts.lock` (10 new), `contracts.py` | libc flexible; the initial locks; `check` verifies the locks |

## 3. The committed locks

The ten locks were first taken at Fluent Bit `66910c10`, the source of the
pinned corpus build (`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`,
[log](locks/pin-66910c10-final.log)). At the integration review:

1. They were migrated to version 2 at `66910c10`
   ([log](locks/v2-migrate-66910c10.log)).
2. The identities of `046accf6`, current upstream master and the system
   slice's revision, were added ([log](locks/v2-add-046accf6.log)).
3. After `main` was merged, W45-FB-RETRY-LIFETIME's contracts were pinned, at
   `046accf6` first ([log](locks/merged-add-046accf6.log)) and then at
   `66910c10` ([log](locks/merged-add-66910c10.log)). Those contracts are
   `fluent_bit/task_users.qcore` and `fluent_bit/output_singleplex.qcore`, and
   the new clauses of `mk_list_add` and `mk_list_del`.

The [census](locks/census.json) records the result:

| | Entries |
|---|---:|
| pinned, one identity reviewed at both `66910c10` and `046accf6` | 60 |
| pinned, two identities (`ctr_destroy`, `flb_output_task_flush`), one per revision | 2 |
| flexible (libc) | 100 |

Every pinned function has at least one definition in each identity.

**Review behind the `046accf6` identities.**

* 54 entries have sites at `046accf6` equal token for token to those reviewed
  at `66910c10`, so they joined the existing identity.
* `ctr_destroy` is the only function whose text differs. Its 046accf6 body
  (CTraces v0.7.2) adds `if (!ctx) { return; }` before the unchanged release
  path.
* The contract `resource_released(ctx, nonnull)` still holds: for a nonnull
  context the body releases it exactly as before. The contract is now stricter
  than the body needs, never weaker.
* It was therefore reviewed and added as a second identity.

**Review behind the retry card's identities.**

* That card wrote and checked its contracts against `046accf6`, so their
  `046accf6` identities record that review.
* `mk_list_add`'s clauses changed, so its earlier identity was voided (`pin`
  printed `contract text changed; 1 earlier identit(ies) voided`) and
  re-recorded.
* At `66910c10`, seven of the eight functions are token-identical to
  `046accf6` and joined the same identity
  ([check](locks/merged-check-66910c10.log)).
* `flb_output_task_flush` differs at `66910c10`. Its contract states OR-7,
  which the retry card **refuted**: checked against the body, it gives a
  provisional Unsafe candidate on the threaded arm, which rolls the user back
  and still returns 0.
* At `66910c10` the threaded arm is token-identical. The one difference in
  user counting is the NULL-`out_flush` path, which returns -1 before adding a
  user and so satisfies the contract's `restored` clause. The contract
  therefore says the same thing about both bodies, and the body check
  reproduces the same refutation at both.
* It has one identity per revision.

`pin --check` results:

* `66910c10`: every entry recorded ([log](locks/v2-check-66910c10.log); after
  the merge, [log](locks/merged-check2-66910c10.log)).
* `046accf6`: every entry recorded ([log](locks/v2-check-046accf6.log); after
  the merge, [log](locks/merged-check2-046accf6.log)).
* `864c3d97`, the system selection's revision: 55 of 55 entries in force
  through the identities above, exit 1 because 864c3d97 itself is not recorded
  ([log](locks/v2-check-864c3d97.log)).

Upstream history holds exactly two textual identities of `ctr_destroy`. Its
body changed once, at `fb5fd48c7` (CTraces v0.7.2), and its prototype not at
all since CTraces was vendored. Both identities are now reviewed. `--check
--revision` shows that:

* at `b3eec839c` and `270a842f9` (2022) `ctr_destroy` is in force through the
  `66910c10` identity ([log](locks/v2-check-ctraces-b3eec839c.log),
  [log](locks/v2-check-ctraces-270a842f9.log));
* at `8fbcc433` (`270a842f9~1`), before CTraces was vendored, it is **not in
  force** (`source-changed`, both identities named)
  ([log](locks/v2-check-ctraces-before-270a842f9.log)).

A third body, reviewed at neither revision, is exercised on the fixture's Git
history (§4).

The corpus inputs map to library revisions vendored inside the `66910c10`
build ([versions](locks/library-versions.json)):

| Library | Where | Version |
|---|---|---|
| ChunkIO | `lib/chunkio` | 1.5.4 |
| CMetrics | `lib/cmetrics` | 2.2.2 |
| CTraces | `lib/ctraces` | 0.7.1 (0.7.2 at `046accf6`) |
| CFL | `lib/cfl` | 1.0.0 |
| Monkey | `lib/monkey` (and a vendored `lib/chunkio/deps/monkey`) | 1.8.9 |
| librdkafka | `lib/librdkafka-2.15.0` | 2.15.0 |
| mpack | `lib/mpack-amalgamation-1.1.1`, with copies under `lib/{cmetrics,cprofiles,ctraces}/lib/mpack` | 1.1.1 |
| msgpack-c | `lib/msgpack-c` | not read |

Each pinned function's sites include every vendored copy. `mpack_write_*`
therefore pins eight sites.

## 4. Tests

* `crates/qcore-contract/tests/contract_revision_mode.rs` (10 tests):
  - the default and round trips;
  - one pinned clause pins the contract, and a flexible clause leaves a pinned one pinned;
  - duplicate, conflicting, restated and nested modes;
  - modes on unsupported items (the selector, a literal premise, the wrong
    introducer, an unknown word, malformed arguments, and nesting with a
    review marker in both orders);
  - removing the markers restores the environment byte for byte;
  - the withheld-pin recheck, including two forged records;
  - digests ignore layout and see every token;
  - a mode is not inherited.
* `crates/qcore-c-gateway/tests/contract_pin_hold.rs` (5 tests):
  - `Safe` in force and `Unknown` for a mismatch;
  - a withheld pin accuses nothing, so a double release and a leak become `Unknown`;
  - re-pinning restores the verdicts byte for byte;
  - a defined callee does not lift the hold;
  - with Clang, the unit reason, and a certificate whose recheck reproduces the
    withheld map (a dropped map is refused).
* `crates/qcore-audit/tests/contract_revision_audit.rs` (3 tests):
  - end to end, a missing lock is out of force (`no-lock`), then `pin` and
    `pin --check`;
  - a body change is `source-changed` with its held caller, `pin --check`
    exits 1, and `compare` classifies the loss;
  - re-pinning the changed body gives the body/contract conflict;
  - restoring the body and pin reproduces the first report byte for byte;
  - an edited contract is `contract-changed`;
  - malformed locks refuse; a plain `pin` refuses an unreadable lock and
    `--regenerate` replaces it;
  - the mutation table of §7.
* `crates/qcore-audit/tests/contract_revision_identities.rs` (3 tests, added at
  the integration review), on a real Git history of the fixture:
  - an entry pinned at revisions A and B is in force at both and out of force
    at C, whose `box_drop` body differs from both;
  - adding B leaves A in force, and `pin --check` reports per revision;
  - `--check --revision` reads a revision without a checkout;
  - `--drop-revision` removes A, and an unknown revision refuses;
  - a contract edit voids every identity at both revisions, and re-pinning
    starts from one;
  - `--regenerate` rebuilds from one revision;
  - a version-1 lock is read as one identity per entry and re-pins to the
    same version-2 lock;
  - malformed version-2 locks refuse (a revision in two identities, an
    identity with no revision, an unknown version).
* `crates/qcore-audit/src/c_identity.rs` (4 unit tests); `crates/qcore-schemas/tests/reason_codes.rs` (4 new corpus rows, 2 refused spellings).
* Existing tests whose synthetic contracts had no lock now mark them
  `revision flexible`, because those contracts describe external APIs with no
  source to pin. The affected tests are `no_build_cli`, `pending_review_report`,
  `root_source_set`, `second_source_set` and `configured_cli` (via
  `tests/fixtures/alloc_*.qcore`).
* `compiler/tests/qcore_contract_corpus.rs` adds one clause to each of the 100
  libc pins. The libc texts changed deliberately (constraint 18).

## 5. Corpus identity

Prediction, as the brief states and recorded here after the fact (no separate
commit preceded the implementation):

* no verdict moves, because the locks are taken at the arbiter's own source;
* one additive sidecar key;
* the contract sidecar's libc texts change.

After the merge the arbiter ran on the merged tree, with the version-2 locks
and the retry card's contracts pinned (`run_audit.py private/rm-arbiter-5
2ef12d99 --jobs 8`, exit 0). It tallies `contract revision: 62 pinned (62 in
force, 0 not in force), 0 flexible checked, 0 flexible trusted, 100 libc under
the standing assumption`; every pin matches through the `66910c10` identity.
[`scripts/identity.sh`](scripts/identity.sh) compares it with the current
baseline, `docs/reports/w45-fb-retry-lifetime/integration/artifacts/`
([results](identity/)).

Earlier runs against the W45-PARENTHESISED-CALLEE baseline gave the same
results:
* before the merge, with the version-2 locks (`private/rm-arbiter-4`,
  `2501709c`; [results](identity/pre-merge/));
* with the first, version-1 locks (`private/rm-arbiter-2`, `9f02f741`).

| Artifact | Result |
|---|---|
| diagnostics (verdict-bearing) | **byte-identical** (`baseline_identity.py` exit 0) |
| chunk-contracts, source | byte-identical |
| contracts | identical as JSON once the card's edits are undone ([`contracts_difference.py`](scripts/contracts_difference.py), exit 0). The edits are: the additive `contract_revision` key, the 16 libc texts, and the two included libc parents' texts and digests in `contract_inclusion` |
| source-effects | identical after masking digests: 10 distinct path-bound digests, as every W45 card discloses |

The pilot's system slice agrees. At both revisions the card's report is
identical to the base binary's except for each compile task's
`resource_contract_sources` digest and the inventory digest over it, because
the libc texts are contract sources. Its sidecar differs only by the same edits
([base-vs-card](pilot/base-vs-card.json)).

## 6. Pilot at two revisions

The slice was run on private builds prepared from clean clones at `864c3d97`
and `046accf6` under `private/w45-fb/w45-contract-revision-mode/`
([scripts](scripts/)). Each revision had three runs:

* the base binary;
* this card's binary with the committed contracts;
* this card's binary with a second contract set
  ([`pilot_variant.py`](scripts/pilot_variant.py)), which marks the eight
  `fluent_bit/input_chunk_*` contracts flexible. Their functions are defined in
  `src/flb_input_chunk.c`, an audited unit.

All runs report 8 files and 15 configurations, 0 Safe / 3 provisional Unsafe /
12 Unknown units. The results below are the integration review's rerun
(`card2-slice`, `variant2-slice`), with the version-2 locks that carry both
revisions' identities.

**Finding: the contracts on main barely touch the code that changed.** Between
the revisions, `src/flb_input_chunk.c` changed in
`flb_input_chunk_write_header_v2`, `input_chunk_get` and
`input_chunk_append_raw`. None of them has a contract. No pinned function's
declaration or body changed in any slice file. The only pinned contract whose
body changed is `ctr_destroy`, in `lib/ctraces`.

Under the card's first, one-identity locks, `ctr_destroy` was out of force at
`046accf6` (`source-changed`). With its `046accf6` identity added (§3):

* it is in force at `864c3d97`, through the identity reviewed at `66910c10`;
* it is in force at `046accf6`, through its own identity;
* it is out of force at an unreviewed revision (§3).

Of the 55 pins in force at each revision, 54 match the shared identity. No
slice unit calls `ctr_destroy`.

| Change report ([summary](pilot/summary.json)) | Result |
|---|---|
| committed contracts, `864c3d97` → `046accf6` ([compare](pilot/compare-revisions.json)) | contracts in force +0 −0 ~0; pin mismatches 0 → 0; obligations +76 −2 across 13 functions of the four changed slice files; violation detected 0, coverage lost 0, coverage gained 0; `flb_output_task_flush_error` added (both configurations, `Unknown`); standing libc assumption 100 at both |
| committed vs second set at `046accf6` ([compare](pilot/compare-contract-sets.json)) | 8 contracts changed, `pinned-in-force` → `flexible-checked`; obligations +0 −0 and no verdict moves |
| second set, `864c3d97` → `046accf6` ([compare](pilot/compare-variant-revisions.json)) | the same moves as the committed set; the 8 flexible contracts stay `flexible-checked` at both revisions |

The second set shows flexible re-verification on real code. It analyses the
same bodies, re-checked against the flexible contracts at each revision, with
the verdicts of the pinned-and-matching contracts.

Two limits of the comparison:

* The obligation comparison excludes lines, but two leak descriptions embed a
  source line (`acquired at src/flb_output.c:288`). A moved line therefore
  reads as one obligation removed and one added.
* `compare` does not report which identity a contract in force matched
  (`ctr_destroy`'s moves from the `66910c10` identity to the `046accf6` one).
  Each run's section records it as `matched_reviewed_at`.

### 6.1 The singleplex pilot, after W45-FB-RETRY-LIFETIME

Once the retry card was on `main` and its contracts were pinned (§3), the
slice was run with the merged binary at both revisions (`merged-slice`,
[`scripts/singleplex.sh`](scripts/singleplex.sh), [summary](singleplex/summary.json)).
62 pinned contracts are in force at each revision.

| Retry-card contract | `864c3d97` | `046accf6` | Body in the slice |
|---|---|---|---|
| `flb_output_task_singleplex_enqueue` (OR-1) | in force (shared identity) | in force (shared identity) | checked: provisional Unsafe at both (OR-1 refuted) |
| `flb_output_task_flush` (OR-7) | in force through the `66910c10` identity | in force through the `046accf6` identity | checked: provisional Unsafe at both (OR-7 refuted) |
| `flb_task_retry_destroy` | in force (shared) | in force (shared) | checked: Unknown |
| `flb_output_thread_pool_flush`, `flb_task_users_inc`, `flb_task_users_dec`, `mk_list_add`, `mk_list_del` | in force (shared) | in force (shared) | not in the slice (trusted pins) |

None went out of force, because no retry-card function changed between the
two slice revisions. The change report
([compare](singleplex/compare-revisions.json)) between them reads: contracts
in force +0 −0 ~0, pin mismatches 0 → 0, obligations +70 −0, violation detected
0, coverage lost 0.

**The retry card's verdicts are unchanged under the revision mode.** At
`046accf6`, [`slice_verdicts.py`](scripts/slice_verdicts.py) compares the run
with the retry card's archived slice (`artifacts/slice-after-046accf6/`)
([result](singleplex/slice-verdicts-vs-retry-card.json)):

* 463 unit and function rows, each with its diagnostic multiset, and 0
  differences;
* units 0 / 2 / 13;
* distinct functions 24 / 2 / 208 / 3;
* OR-1 and OR-7 provisional Unsafe (refuted).

This discharges the singleplex pilot the brief deferred. It is not registered
as a follow-up.

## 7. Controlled mutations

The fixture is generic and held out:
[`crates/qcore-audit/tests/fixtures/revision_mode/`](../../../crates/qcore-audit/tests/fixtures/revision_mode/).
It has three audited bodies with contracts of their own, two callers, and ten
contracts. The harness is [`scripts/mutations.py`](scripts/mutations.py), with
per-mutation logs in [mutations/](mutations/) and the
[table](mutations/table.json) (`all_met: true`). The Rust test
`the_mutation_table` replays it in the gate.

| Mutation | Flexible (bodies re-verified) | Pinned (lock at the unmutated fixture) |
|---|---|---|
| rename a local (`box_count`) | **accepted** (`Safe`) | `box_count` out of force (`source-changed`) |
| reorder independent statements (`box_two`) | **accepted** (`Safe`) | `box_two` out of force |
| dropped release (`box_drop`) | **coverage lost**: `box_drop` `Unknown` (summary conflict), `user_drop` `Unknown` | `box_drop` out of force; `user_drop` `Unknown` (`contract-pin-mismatch`) |
| missing counter rollback (`box_count`) | **violation detected**: `Unsafe` (leak on the failure path) | out of force; the body's own leak is still `Unsafe` |
| changed transfer (`box_keep`) | **coverage lost**: `box_keep` `Unknown` (summary conflict), `user_keep` `Unknown` | `box_keep` out of force; `user_keep` `Unknown` (`contract-pin-mismatch`) |
| premature destroy (`box_drop`) | **coverage lost**: `box_drop` `Unknown` (unresolved alias), `user_drop` `Safe` through the contract | `box_drop` out of force; `user_drop` `Unknown` (`contract-pin-mismatch`) |

Under flexible, no defect is accepted and both harmless refactors are. The
premature destroy is lost coverage in the body and not a detected violation.
Its caller stays `Safe` through the declared contract. That is the flexible
semantics: the body is where the defect is reported. Under pinned, every body
change takes the mutated function's contract, and only that one, out of force.
The count is measured with the shipped harness.

## 8. Lean

[`proof/QCore/ContractRevisionMode.lean`](../../../proof/QCore/ContractRevisionMode.lean),
audited by [`proof/AuditContractRevisionMode.lean`](../../../proof/AuditContractRevisionMode.lean),
has 25 theorems:

* **The default:** `the_default_is_pinned`, `one_pinned_clause_pins_the_contract`,
  `a_flexible_contract_without_pinned_clauses_is_flexible`.
* **Not in force:** `a_mismatched_pin_is_not_in_force`,
  `a_mismatched_pin_contributes_nothing`, `applied_is_all_or_nothing`,
  `a_verdict_resting_on_a_mismatched_pin_is_unknown` and its corollary
  `a_mismatched_dependency_is_neither_safe_nor_unsafe`,
  `the_pin_reason_never_supports_unsafe`.
* **Several reviewed identities** (added at the integration review; the model
  is in force exactly when the text matches and some identity matches):
  `a_pin_is_in_force_iff_its_text_and_some_identity_match`,
  `a_contract_edit_voids_every_identity`,
  `adding_an_identity_keeps_every_reviewed_one`,
  `pinned_at_two_revisions_holds_at_both_and_nowhere_else`.
* **Re-pinning** (adds the audited identity, keeping the others while the text
  matches): `repinning_restores_the_verdict_in_force`,
  `matching_pins_give_the_verdict_in_force`,
  `removing_a_mismatch_restores_verdicts_exactly`.
* **Flexible re-verification:** `a_flexible_contract_is_in_force_at_every_identity`,
  `a_checked_body_is_still_checked`, `flexible_and_matching_pinned_check_alike`,
  `a_trusted_flexible_contract_is_a_listed_assumption`.
* **The unit reason:** `a_pin_held_summary_is_never_a_source_statement`,
  `a_held_summary_says_nothing_about_the_source`,
  `a_summary_without_held_calls_is_unchanged`.

`QCore.ReasonCategory` gains the ninth category and two families, and every
existing theorem re-proves over it.

Identities are abstract. That a lock digest is a faithful identity of a
function's text is the scanner's boundary, pinned by its tests.

## 9. Gate

See [gate.md](gate.md): commands, exits, counts and log paths, measured at the
card's final tree.

## 10. Integration and follow-ups

**At integration of W45-FB-BACKLOG-CUSTODY (and any later contracts).** Its
new contracts are pinned by default and have no lock entry. They are out of
force (`not-in-lock`), and `providers/contracts.py check` fails until the
integrator reviews them and adds their identities, one command per reviewed
revision:

```sh
# the revision they were written against (a checkout at 046accf6)
quirin-audit pin --source-root <checkout at 046accf6>
# the corpus revision, so the arbiter sees them in force where their text agrees
quirin-audit pin --source-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
```

Neither command drops an identity recorded at the other revision. After the
second, `pin --check` at `66910c10` names every function whose text differs
there; each needs its own review before its identity is kept. This is exactly
what was done for W45-FB-RETRY-LIFETIME (§3).

Registered in `named_followups` (428 → 434 on `main` at `84ebad57`) from [followups.json](followups.json)
by [scripts/update_ledger.py](scripts/update_ledger.py), each with its
discharge condition:

* `W45-REVISION-PIN-ROUTES` (pins are evaluated by `quirin-audit`'s routes and
  the arbiter only)
* `W45-REVISION-LOCK-NOT-PACKAGED`
* `W45-REVISION-SCANNER-LEXICAL`
* `W45-REVISION-HOLD-REPORTING`
* `W45-REVIEW-PREFIX-COLLISION-UNPINNED` (found in passing: the review hold
  documents a test that does not exist)
* `W45-REVISION-COMPARE-MATCHED-IDENTITY` (added at the integration review)
