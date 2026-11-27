# CHUNKIO-META-LEN-BOUNDS-CONTRACTS: prediction (frozen before implementation code)

Base: local `main` at `cdad4dfd0` (QUANT-INDEX-CHECKS landed; corpus 756 / 9 /
4,069, `resource-diagnostics.json` `ce57d194…`; system 85 / 4 / 359; bounds rows
`822a7633…`, 162 Safe / 4,759 Unknown; combined 111 Safe / 4,801 Unknown / 9
Unsafe; follow-ups 705; Lean 288 audits). Written after reading the bounds
consumer (`qcore_contract::bounds_contract`, `bounds_quant`,
`qcore_c_gateway::index_extent`), the resource pin machinery
(`qcore_audit::contract_revision`, `contract_pin`, `c_identity`), the provider
selection (`qcore_audit::providers`, `qcore_contract::load_path_manifest`,
`providers/contracts.py`, `package.py`) and both pinned sources' header, before
any implementation code.

## The source (read at both revisions)

`lib/chunkio/include/chunkio/cio_file_st.h` is byte-identical at `66910c10`
(the corpus source in `build/cycle19`) and `046accf6` (a private clone), sha256
`02eb3909…`:

* `static inline uint16_t cio_file_st_get_meta_len(char *map)` reads `map[22]`
  and `map[23]` (line 76-79);
* `static inline void cio_file_st_set_meta_len(char *map, uint16_t len)` writes
  `map[22]` and `map[23]` (line 82-86).

## The contracts (one module, `qcore-provider-chunkio`)

`providers/qcore-provider-chunkio/contracts/chunkio/meta_len_bounds.qv`, bounds
contract v1, no review marker (written at the user's request after reading the
source, and pinned):

* get: `quant checks map, Idx #i: 22 <= #i && #i < 24;` and
  `ensures normal bounds_frame_reads(map);` (rule 11e: the only pointer formal,
  so the closed frame is that one clause; the body only reads);
* set: the same check and `ensures normal bounds_frame_writes(map);`.

The admitted range is the exact one, `[22, 24)`: both bodies use exactly
indices 22 and 23, so it is exactly true of each body, where the header
reading `[0, 24)` (`CIO_FILE_HEADER_MIN`) states 22 indices the body never
uses. Both demand the same of a caller (cover indices 22 and 23: 24 bytes).

## Design (transport minimum)

1. **Placement and packaging.** Each load-path crate record gains a generated
   `bounds_view` beside `resource_view`: the owned modules the bounds loader
   accepts, computed by the probe (`probe_resource_views` also probes the
   bounds loader), written by `contracts.py sync`, checked by `check`; a
   module in both views, or a view naming an unowned module, refuses. The
   provider package carries the module through the unchanged inventory
   (`contracts.json` lists every file under `contracts/`); no audit route reads
   contract modules from a package, so no package format changes.
2. **Pins.** A bounds lock `bounds-contracts.lock` beside the crate's
   `Cargo.toml`, the C06.2 lock format (version 2) with the bounds
   declaration's text digest (`text_sha256`, which covers the quant clause's
   text) and reviewed identities. `quirin-audit pin --bounds` writes and checks
   it; `pin --verify-contracts` verifies resource and bounds locks. At audit
   time a declaration out of force (no lock, not in lock, contract changed,
   source changed) is **held** with reason `contract-pin-mismatch`, never Safe;
   a held declaration is Pending at most and accuses nothing.
3. **Selection.** With `QCORE_BOUNDS_PROPERTY`, the pinned route (corpus
   arbiter) and the configured route (`current`) load every bounds view on the
   load path their resource manifest names, evaluate the bounds pins over the
   audited source, and analyse every unit under that environment; a sidecar
   `<rows>.contracts.json` records the selection and the pin section. Without
   the variable nothing is loaded and every artifact is byte-identical.

## Predicted measurements

| Route | Prediction | Why |
|---|---|---|
| Corpus arbiter (custody) | byte-identical: 756 / 9 / 4,069, `ce57d194…` | no custody code reads bounds contracts; the bounds module is outside every resource view, so no resource selection loads it |
| System route | byte-identical (85 / 4 / 359) | as above |
| Tail-pause slice | byte-identical | as above |
| Typed-path parity (three routes) | identical multisets except the one row the BEFORE binary itself varies (`cio_file_st_get_meta_len` line 2223, GAPO-PARITY-DECISIONS-NONDETERMINISTIC) | as above |
| Corpus with `QCORE_BOUNDS_PROPERTY`: custody report | byte-identical to the custody run | the bounds environment feeds only the bounds rows |
| Corpus with `QCORE_BOUNDS_PROPERTY`: bounds rows | every row's `contracts_sha256` changes (the environment is no longer empty); verdicts move only for the two functions' own rows: every row of `cio_file_st_get_meta_len` and `cio_file_st_set_meta_len` Unknown → **Safe** (conditional on the declared check), pins in force at `66910c10` | the body's accesses `map[22]`, `map[23]` are decided inside `[22, 24)` and in bounds of the seeded 24-element extent |
| Their callers | stay Unknown | every corpus caller passes `cf->map` (a member, no extent, CG16-BOUNDS-POINTER-EXTENTS) or its own unextended formal (`cio_file_st_get_content`, `get_content_len`, `infer_content_len`); the obligation (cover indices 22 and 23) is unestablished |
| Bounds Unsafe | none added | no caller establishes an extent smaller than 24 |
| Combined | Safe rises by the moved rows whose custody row is Safe; nothing else moves | composition |

Any other move is a defect of this card, or a correction to be named.

## Predicted controls

| Control | Predicted |
|---|---|
| Both declarations at `66910c10` and `046accf6`, pinned by `pin --bounds` | `pin --bounds --check` exit 0 at both |
| Stale pin (lock records only another revision whose sites differ) | `source-changed`, held: the body Pending, never Safe |
| Changed clause (lock digest differs) | `contract-changed`, held |
| Changed body (sites differ) | `source-changed`, held |
| No lock / no entry | `no-lock` / `not-in-lock`, held |
| Pin in force | resolved: body Safe |
| `pin --verify-contracts` after a clause edit | stale bounds lock, exit 1 |
| Bounds module selected by a resource selection | refused (not in a resource view) |
| A `bounds_view` naming a resource module, or a module in both views | load path refused |
| Bounds module with `#include` / `contract inherit` | refused by the bounds loader (inheritance not landed) |

Lean: `QCore.BoundsPinHold` (new) registered in `AuditBoundsPinHold.lean`
(288 → 289): a declaration's standing is resolved exactly when no review
marker and the pin is in force; a pin mismatch is held, hence never Safe and
never accusing (through `QCore.IndexExtent`'s verdict table); re-pinning
restores the resolved standing; vectors shared with the Rust standing.

Follow-ups: amend `CG16-BOUNDS-CONTRACT-TRANSPORT` (what landed; remaining:
checkpoint `.qo` transport, inclusion/inheritance, the cross-unit refutation
phase on the routes); register the neighbouring `cio_file_st_*` accessors as
one follow-up with what each needs. Follow-ups 705 → 706.
