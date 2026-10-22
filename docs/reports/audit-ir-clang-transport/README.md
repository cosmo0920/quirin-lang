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
cache authority or default `.qc` capture is intended. Temporary persisted AST
transport uses `.qc` and is disposable, never trusted evidence. Retain cleanup
on errors and unwinding. Existing matching Lean depth/transport definitions
remain applicable if admission is unchanged; any semantic edit requires matching
audited Lean. Run malformed/depth/large-input controls and the full standing
gate. Serialize heavy gates and measurements with
`/tmp/quirin-audit-ir-next-heavy.lock`. The parent's single full-run receipt is
context only, not before/after evidence for this change, and the parent memory
target remains open.

The implementation keeps at most 1 MiB of small raw output in memory and
spools larger complete translation-unit dumps before parsing. The public
whole-unit reader, compact typed index and lifetime-source reader use the new
seekable input. Existing root and per-declaration visitors, lifetime depth/node
checks, facts construction and source hashing are unchanged. Filtered streams
and the older callback-based byte-slice transport remain separate follow-ons;
this card does not change member-facts retention or report products.

Subprocess stdin, stdout and stderr drain concurrently. Input-write errors keep
their prior precedence, then child exit/stderr, then parsing. The reader returns
nothing on a malformed suffix. Default decoding runs first; a refusal scans the
same complete bytes for the same 256-level depth bound, seeks back and retries
on the existing dedicated stack, retaining the default error if retry fails.
The matching `QCore.DeepUnitAst` and `QCore.DeepAstTransports` models are unchanged.

The spool is a private create-new `.qc` file with Unix mode 0600, immediately
unlinked on Unix and removed on ordinary cleanup elsewhere. It supplies no
cache authority and enables no persisted IR reuse. On 64-bit Linux, after
synchronizing writes and in read intervals, `posix_fadvise(DONTNEED)` requests
release of charged disk-backed cache pages. Advice is not a hard memory bound.
`TMPDIR` selects storage through the standard library: tmpfs still charges the
raw spool to memory, so its result must not be described as disk-backed paging.
Both temporary-storage conditions belong in the control measurement.
