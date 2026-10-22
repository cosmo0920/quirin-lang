# AUDIT-IR-CLANG-TRANSPORT

Base: `31c5ed500d9d4a1448a623b839e1644adb19efd1`.

Prediction before implementation: Clang stdout currently remains as one owned
byte vector throughout decoding and consumer construction. Replace transport
storage/readers with bounded reading where the existing full-document and
retry semantics permit it. Preserve subprocess status/stderr and input failure
precedence, complete JSON validation, default versus 256-level admission,
root checks, node budgets, source identity, cross-declaration facts and every
published record. Malformed suffixes must never publish a partial result.
Existing declaration visitors already reduce tree retention; do not duplicate
or change the other cards' member closure and CFG/report work.

The intended outcome is a measured reduction in transient single-unit memory,
including Clang and any charged file cache, with exact source-reading parity.
No new semantic admission, verdict movement, selected-configuration omission,
cache authority or default retained IR capture is intended. Temporary persisted AST
transport uses `.qc` and is disposable, never trusted evidence. Retain cleanup
on errors and unwinding. Existing matching Lean depth/transport definitions
remain applicable if admission is unchanged; any semantic edit requires matching
audited Lean. Run malformed/depth/large-input controls and the full standing
gate. Serialize heavy gates and measurements with
`/tmp/quirin-audit-ir-next-heavy.lock`. The parent's single full-run receipt is
context only, not before/after evidence for this change, and the parent memory
target remains open.

The implementation uses a 1 MiB raw-payload threshold and spools larger
complete translation-unit dumps before parsing (allocator capacity and decoded
values are not included in that threshold). The public
whole-unit reader, compact typed index and lifetime-source reader use the new
seekable input. Existing root and per-declaration visitors, lifetime depth/node
checks, facts construction and source hashing are unchanged. Filtered streams
and the older callback-based byte-slice transport remain separate follow-ons;
this card does not change member-facts retention or report products.

Subprocess stdin, stdout and stderr drain concurrently. Input-write errors keep
their prior precedence; transport I/O failures precede child rejection, and
child exit/stderr precedes parsing. The reader returns
nothing on a malformed suffix. Default decoding runs first; a refusal scans the
same complete bytes for the same 256-level depth bound, seeks back and retries
on the existing dedicated stack, retaining the default error if retry fails.
The matching `QCore.DeepUnitAst` and `QCore.DeepAstTransports` models are unchanged.

The spool is a private create-new `.qc` file with Unix mode 0600, immediately
unlinked on Unix, owned by a delete-on-close handle on Windows, and removed
on ordinary cleanup on other platforms. It supplies no
cache authority and enables no persisted IR reuse. On 64-bit Linux, after
synchronizing writes and in read intervals, `posix_fadvise(DONTNEED)` requests
release of charged disk-backed cache pages. Advice is not a hard memory bound.
`TMPDIR` selects storage through the standard library: tmpfs still charges the
raw spool to memory, so its result must not be described as disk-backed paging.
Both temporary-storage conditions belong in the control measurement. Seeking
resets the cache-advice positions so retries discard the ranges they actually
read; the round-trip control asserts this after repeated rewinds.


## Measured single-unit outcome

The [final memory receipt](memory.json) records four successful sequential
controls on one shared Linux x86-64 host. Each processes the same 30,000-function
translation unit and retains the complete decoded AST. Whole-cgroup peaks include
Clang, the worker and charged file cache; build/setup is excluded.

| Temporary storage | Buffered peak (bytes) | Seekable peak (bytes) | Peak reduction | Worker seconds, before → after |
| --- | ---: | ---: | ---: | ---: |
| Disk-backed | 5,897,371,648 | 5,410,885,632 | 486,486,016 bytes (8.249%) | 11.432 → 13.865 |
| Tmpfs (`/tmp`) | 5,896,781,824 | 5,900,382,208 | −3,600,384 bytes (−0.061%) | 12.498 → 13.959 |

All four driver/worker runs exit **0**, with zero swap peak, OOM and OOM-kill
events, unchanged inputs and successful service cleanup. Each publishes the
same complete 286,930,122-byte normalized AST, SHA-256
`1bac523b85f815ed5d9326637ccdd3fcc79de252a3893d6172877b641cc23aac`.
The disk-backed pair demonstrates a bounded reduction with longer elapsed time;
the tmpfs pair demonstrates **no reduction**. Default temporary storage on this
host is tmpfs, so the observed benefit requires selecting disk-backed `TMPDIR`.
This one pair per storage condition is not a repeatability, cold/warm or
production-audit claim and does not discharge the parent's 6 GB target.

Exact commands, configurations, executable/source hashes, raw instrumentation,
cleanup and private log paths are in the receipt. The reused meter's inherited
“Configured resource audit” label is explicitly corrected there to this
single-unit transport-control scope. The campaign driver exits **0**; private
results are under `private/audit-ir-clang-transport/controls3/` and its log is
`private/audit-ir-clang-transport/measurement3-driver.log`.

## Integration and residuals

Integrate this card's source and evidence onto the eventual combined source;
reconcile the new paragraph in `docs/STATUS.md`, the AUDIT-IR Card subsection
and the one increment in `wave-48-part11.md` with sibling cards. No shared
checkout, provider contracts, frozen inputs, prior receipts or other card's
worktree was edited. The source change does not require a new semantic rule;
existing matching audited Lean admission definitions remain authoritative.

The remaining byte-slice transport callers and filtered streams still allocate
complete stdout buffers. Stderr remains buffered in full to preserve diagnostics.
Lifetime-source still retains its complete decoded
`Value` and cross-declaration facts; this card changes its raw input ownership,
not its fact census. An individual large declaration/string or decoded unit can
still exceed a runner's budget. Spooling introduces temporary-storage I/O and
can refuse on an unavailable/full temporary filesystem. Cache-discard advice
is best effort; tmpfs and other platforms cannot inherit a measured disk-backed
Linux reduction. No complete current-resource, cold/warm, platform-matrix or
whole-C correctness claim follows from a single-unit control. Those remain
parent follow-ons, not an excuse to omit selected configurations.

[Focused controls](focused-controls.json) retain the real-Clang before/after
preflight's identical complete normalized AST bytes and an isolated storage
failure: unavailable temporary storage exits **1** (expected), emits a transport
failure and publishes no AST. The same unchanged 2,000-definition input with
valid storage exits **0**, with all 2,000 definitions retained. This is one
observed storage fault/positive control, not a production mutation campaign.
These controls were rerun against the final measured executable: exits are
0/0/1/0, with exact argv, configurations, binary/source hashes and private paths
in the receipt. The [development preflight](focused-controls-development.json)
is preserved separately; its observations are not added to the final results. Ordinary source checks also cover complete malformed suffixes,
subprocess rejection/stderr precedence, simultaneous large stdin/stdout/stderr,
spool cleanup, duplicate keys, scalar/Unicode failures and depth boundaries.


[The full standing gate](gate.json) passes **8,506 Rust tests / 0 failed /
15 ignored, 537 units / 550 summaries; 362 Lean audits; 374 Python tests**.
The six focused Rust controls are included in the full count. Submodules,
Clang recording, formatting, all-target Clippy, both reevaluation examples and
the final release build exit 0. The private per-unit Rust summary is
`private/test-binaries/20261022T083727Z-1522714/summary.json`; other command logs
and exits are under `private/audit-ir-clang-transport/gate2/`. The receipt records
absolute paths, all argv, the unique target directory and the 1,500-file Rust
snapshot, which matches the final source. An initial invalid context topic
(exit 2) and the initial sandbox-read-only setup attempt are disclosed beside
the accepted corrections. No failed gate result was discarded or borrowed.

[Measurement attempts](measurement-attempts.json) retain the initial shared-disk
exhaustion and the next attempt stopped while queued for the seek fix. Neither
is a successful measurement; their private directories and logs are retained.

The final gate validates implementation `1a397b951943a1dd9dc7f0b3f1a4ec6d1383e4e1`,
including the seek-counter fix. The [prior gate](gate-before-seek-fix.json)
retains its earlier implementation checkpoint; its counts are not added to the
final run. All 1,500 tracked Rust files match the final run’s source snapshot.

## Reproducing the bounded control

Build `cargo build --release -p qcore-c-gateway --example clang_transport_measure`
in the card worktree with its private `CARGO_TARGET_DIR`, then run
`scripts/measure.py` from this report with `--binary` naming that executable
and `--output` naming a fresh directory. The script generates 30,000 shallow C
function definitions and serializes four runs under the shared heavy-job lock:
buffered reference and seekable transport with disk-backed `TMPDIR`, then the
same pair with `/tmp` (tmpfs on this host). It writes configurations, complete
normalized `.qc` ASTs and a full-byte comparison receipt. The normalization
consistently renames Clang's process-local hexadecimal node addresses; the
generated source contains no hexadecimal string values.

Both modes retain the same complete `serde_json::Value`, normalize its addresses
and publish the entire AST. The buffered reference transcribes the original
successful shallow-document transport in the same executable; it is not a
separately built base audit binary. The frozen admission measurement utility
runs each executable in its own systemd cgroup with a requested 7,000,000,000-byte
limit and no swap. Its generic receipt text says “Configured resource audit”;
that inherited label does **not** describe these single-unit transport controls.
The benchmark argv and this scope govern. Build/setup is excluded, host cache
state is uncontrolled, and one sequential pair is neither a cold/warm campaign
nor a production-audit result.
