# Wave 31 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

C gateway scalar writes, expression frames and conditional custody (2026-09-13):
`C-GATEWAY-WAVE` implements the selected `AOM-POISON-NOOP-INCREMENT` and
bounded `AOM-POISON-CONDITION-FRAME` repairs, and an acyclic conditional-identity
capability for `AOM-DESTROY-JOINED-PROVENANCE`. Scalar writes forget stale branch
facts, including the production `flag = !0` case exposed during the staged
review; complete expression refusals spare only positively unwritable automatic
bindings; a guarded release may settle a known local acquisition while retaining
the lookup's `UnknownAlias`. Unshadowed, non-address-taken locals and shallow
release contracts bound that last rule; the production `input_chunk_get` residual
remains open. Sixteen abstract Lean rule theorems carry explicit extraction,
contract and no-UB premises; no whole-C correctness claim is made. The preceding
worktree sweep retains all four secondary checkouts because each has unique
unmerged work, and verifies the existing recovery hashes. Fresh BEFORE reproduces
the wave-30 diagnostic and chunk-contract archives byte for byte; the five staged
measurements, every changed leaf, provisional diagnostic boundaries and new
baseline are owned by `docs/reports/c-gateway-wave/`. Final gate: **4,484** workspace
tests (0 failed, 8 ignored, `--no-fail-fast`), **200** Python tests, **123** Lean
audits, both probes exit 0, fmt/clippy clean; provider 122 carried unchanged.
The sandbox failure, its green outside-sandbox repeat and the final source's
separate green repeat are all retained. Contracts remain 74, CT mapping remains
42/50 (proof 2 of 42), and full language conformance remains false.

Documentation structure and bounded retrieval (2026-09-13): the main guides
adopt the compact-context reading strategy, and `DEVELOPMENT.md` becomes a
table of contents over focused setup, layout, cycle-evidence and increment
pages under `docs/development/`. Forty-one pages split at original section or
record boundaries; every old heading remains reachable from the root guide.
The former guide's full text is reconstructible from the migration record:
relative links are rebased to their original targets, the two current
build examples gain the standing gate's already-required `--no-fail-fast`,
and current setup/layout pages use the `quirin` binary name while keeping
the `qcorec` package name.
Historical checkpoint prose and all 44 standing constraints are preserved;
new increment paragraphs now go to the file named by the history index.
This changes documentation retrieval, not compiler semantics, verdicts or
conformance. The measurements, preservation checks and gate results are in
`docs/reports/development-docs-structure/`; the automated packet router and
the planned reduction of always-loaded agent guidance remain separate work.

<!-- migrated-section:start -->
Wave 31, card `CT42-PROTOCOL-CELL` (2026-09-13): the exploratory
shared-protocol-cell card, closed as **survey plus the bounded F4 slice**
rather than withdrawal. The survey found the cell notion already real at
model level, *declared*: `disjoint_footprint` carries the identity
(`Descriptor { descriptor, cell }`), the refusal (`SharedProtocolCell`,
`Q664`) and the `DeclaredProtocolCellIdentity` premise, while
`source.socket.v1`'s completed connection is the one source shape with two
descriptor-valued names over one shared logical state — its cross-name
refusals pre-exist, but the shared state had no identity (mutual `peer`
references). What landed is the derivation that connects them:
`qcore_core::socket_footprint` derives protocol cells from an **admitted**
socket program (fail-closed — a refused program derives nothing; the
`connect` target read from the checker's own `resolve_addresses`, descriptors
numbered identically to the certificate's), giving each completed connection
one cell with two custodians and every other socket a singleton, then
projects one `DeclareDescriptorAlias` per socket onto the **unedited**
footprint checker, where the pair is refused at the existing `Q664` and the
listener decides disjoint. Zero new codes, zero reserved words, no emission,
checker or route-selection file touched (byte-identity structural). Lean:
`QCore.SocketFootprint` (freshness invariant, pairing theorems, composition
with `decideDisjoint`; audits 121 → **122**, `propext`/`Quot.sound` only).
Deliberate boundaries, each named in the report: a datagram pair is not a
cell (`CT42-CELL-UDP-QUEUE`), the pending phase stays the transport
checker's (`CT42-CELL-PENDING-PHASE`), no dup-like source form exists
(`CT42-CELL-DUP-SOURCE`), the derived-premise narrowing needs a T09 edit
(`CT42-CELL-DERIVED-PREMISE`). CT42 stays **unmapped** — manifest, matrix
and CT fixtures untouched; the wave-17 C-twin blocker
(`CT-CTWIN-DESCRIPTOR`) stands; the readiness statement and the mapping
lane's two candidate arms are `docs/reports/ct42-protocol-cell/README.md`
§5. Card gate: 4,473 workspace tests (0 failed, 8 ignored,
`--no-fail-fast`), fmt/clippy clean, both probes exit 0, `Passed 122 Lean
audits`, and the Python row 199 OK of 200 with the one failure being the
disclosed lean-audit matrix tripwire (the committed matrix pins 121 audits;
regeneration is the integrator's, constraint 25).

Worktree census hygiene (2026-09-13, post-wave-30 close): the
`SOCKET-INET-ADDRESSES` census test
(`std_socket_inet.rs::this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`)
walked the repository root recursively and, when run from the **primary
checkout**, recursed into `.claude/worktrees/*` — sibling checkouts of this
same repository — finding their copies of the two published examples and
failing against the pinned two-file expectation. The walker now skips any
subdirectory carrying its own `.git` entry (a nested git worktree or embedded
checkout is another tree, not this one), the same environment-shape family as
the wave-29 submodule-diagnostic precondition. The census itself is
unweakened: over one checkout's tree it visits exactly the files it always
visited (the >400-file floor and the two-example pin are untouched), and the
other test walkers were surveyed — every other `read_dir` use starts inside a
bounded directory, so this was the only root-recursive walk. Measured in both
environments: 36/36 in an isolated worktree, 36/36 from the primary checkout
after the fix.

<!-- migrated-section:end -->
