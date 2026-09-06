# Cycle 19: reopen size-lookup and mapping failures

Five fresh native processes exercise the pinned ChunkIO down/up/parent-teardown
chain on exclusive existing-file fixtures. Selected fstat/stat or mmap calls are
suppressed with EIO; they never reach the kernel. Other selected calls forward
normally. ASan, UBSan and leak checking pass, as does runtime contract replay for
all five executions. All 50 allocation generations are released (five temporary,
45 parent teardown). Ten descriptor epochs and six mapping epochs are acquired.
The library releases all six mappings and eight descriptors; two descriptors
survive parent release and are reclaimed separately by the harness.

| Fixed sequence | Observed result |
|---|---|
| Down; suppress fstat during reopen; allow stat fallback | Reopen succeeds, acquires a second mapping and descriptor epoch, reloads the four original bytes, and increments the up counter. Parent teardown releases both endpoints. |
| Down; suppress fstat and stat; retry down/up | Failed up retains the new descriptor/inode, records zero filesystem size, and leaves the mapping absent, chunk down and up counter zero. Both retries reject without acquiring or releasing another epoch. Parent teardown closes the retained descriptor. |
| Same, also suppress parent close | All parent allocations are released while the descriptor remains live. Only the separately recorded harness close releases it. |
| Down; suppress mmap; retry up/down | No new mapping is acquired, but the backend stores MAP_FAILED and retains the new descriptor. The library reports up-ring membership while the up counter remains zero. Up retry rejects; down retry and parent teardown attempt munmap(MAP_FAILED), which returns EINVAL. Parent close releases the descriptor. |
| Same mapping failure; sync; retry down; suppress parent close | Sync returns success without a live mapping or a sync syscall in this already-synced profile. Failed unmaps preserve the stored sentinel; parent release retains the descriptor for separate harness reclamation. |

Neither a failed reopen nor a zero up counter discharges descriptor custody.
A non-null backend mapping field does not establish a live mapping: MAP_FAILED
remains distinct from both null and an acquired mapping epoch. These fixed
injections do not establish arbitrary kernel error post-states, production
reachability, universal cleanup or durability.

The source/build identity is recorded in metadata.json, including Fluent Bit
revision `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, compiled source and observer
hashes, compiler flags and trace/contract hashes. The independent adapter checks
exact operation results, consumed suppression counts, ordered diagnostics,
allocation separation, endpoint epochs, inode probes, reciprocal rings, counters,
content, parent cleanup and process completion. Mutation tests reject omitted or
forged releases, changed endpoint identities, invented successful mappings,
erased residuals and sanitizer failures. Positive reuse/relocation tests allow
numeric descriptors and mapping addresses to be reused across released epochs.

The 28-clause QCore policy evaluates recorded executions only. Eleven Lean
theorems cover the bounded transition projection and its custody distinctions;
they are not a general refinement proof for the C source. The successful map,
cleanup and sync arms retain the fixed profile premises documented in the model.

```sh
python3 -B scripts/cycle19_file_reopen_failure_observations.py --fluent-bit /path/to/pinned/fluent-bit --output /tmp/cycle19-reopen-failures
python3 -B -m unittest discover -s scripts/tests -v
```

LeakSanitizer requires execution outside ptrace containment. The initial
sandboxed rerun failed for that environmental reason; the subsequent native
rerun passed with leak checking enabled. Compressed logs and validation.json
record the standing gate, Python tests and Lean axiom audit. parity.json compares
the four pinned source/diagnostic reports, with any pre-existing W4 movement
identified explicitly. The historical descriptor report predates W4: its one
Unknown → Safe change in `chunkio/cio_error.c` is the previously recorded W4
result; this observation increment changes no analyzer code. Contract source
keys differ only by checkout path. Source and chunk-contract reports are equal.
SHA256SUMS binds this evidence package.

Result boundary: artifact = recorded executable and pinned ChunkIO sources;
property = observed descriptor/mapping custody across reopen failures and parent
teardown; semantic profile = recorded 64-bit Linux, checksums and trim disabled,
clean 4096-byte existing files; scope = five exclusive one-stream/one-chunk
sequences; bounds = four content bytes, two descriptor epochs and at most two
mapping epochs, fixed suppressed calls, no concurrency; assumptions = trusted
compiler/observer, selected wrappers, libc/endpoint probes and exclusive fixtures;
evidence version = trace schema 1 plus SHA256SUMS. No static Safe/Unsafe
certificate is issued. Raw units remain unpromoted; Cycle 19 remains in progress.

Remaining work includes reopen corruption/checksum and capacity/metadata variants,
general caller/constructor provenance, synchronization and OS failure semantics,
aggregate source custody, wrapper lifecycle and full Monkey/Cycle 18 composition.
