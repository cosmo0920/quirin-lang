# CHUNKIO-META-LEN-BOUNDS-CONTRACTS

Wave 48, round 19. The user's request (2026-10-04), after QUANT-INDEX-CHECKS
probed chunkio with test-only, review-held declarations: "Then, we must write
down the listed cio_file functions' contracts." Gateway rule
[12r](../../agents/gateway.md#constraint-12r); contract specification
[C09.1](../../QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned)
(status paragraph) and C06.2; grammar
[G05b](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05b-bounds-contract-v1);
providers rule [Bounds view](../../agents/providers.md). Lean:
`QCore.BoundsPinHold` ([`proof/QCore/BoundsPinHold.lean`](../../../proof/QCore/BoundsPinHold.lean),
audited in `proof/AuditBoundsPinHold.lean`, 18 theorems). Base: local main
`cdad4dfd0`, rebased onto `e2031f65a` (GAPO-PARITY-DECISIONS-NONDETERMINISTIC)
before any AFTER measurement. Prediction frozen in [PREDICTION.md](PREDICTION.md)
(`9818a889b`, `bbb17765d` after the rebase) before implementation code.

**Published property and scope.** CG16's `c.bounds.index-extent.v1` (rules
11s, 11e, 12l), now with reviewed, pinned declarations carried to the audit
routes. Custody is unchanged.

## 1. The contracts

`lib/chunkio/include/chunkio/cio_file_st.h` is byte-identical at `66910c10`
(the corpus source) and `046accf6` (a private `quirin-audit prepare` clone),
sha256 `02eb3909…`. The two bodies (lines 76-86) read, resp. write, `map[22]`
and `map[23]`, the big-endian metadata length that closes the 24-byte header
(`CIO_FILE_HEADER_MIN`). The module
[`chunkio/meta_len_bounds.qv`](../../../providers/qcore-provider-chunkio/contracts/chunkio/meta_len_bounds.qv)
in `qcore-provider-chunkio` states, for each:

```qcore
requires bounds_contract_v1;
quant checks map, Idx #i: 22 <= #i && #i < 24;
ensures normal bounds_frame_reads(map);    // set: bounds_frame_writes(map)
```

* **Range.** The exact indices, `[22, 24)`, not the header `[0, 24)`: both
  are true of the bodies, but only the first is *exactly* their use; both
  give every caller the same obligation (cover indices 22 and 23, i.e. 24
  bytes from `map`).
* **Frame (rule 11e).** `map` is the only pointer formal, so the closed frame
  is one clause; get only reads through it, set only writes through it.
  Without a frame every call would be CG16's unstated call, withdrawing the
  caller's facts.
* **Placement.** chunkio's provider crate, never libc or the Fluent Bit crate;
  in the crate's `bounds_view`, never its resource view (resource v1 rejects
  the vocabulary).
* **Review status.** No `bounds_review_pending` marker: written at the user's
  request after reading the source at both revisions, and pinned there with
  the tool, as resource contracts are. The pin route does **not** mark them
  review-pending; it holds them only when their pin is not in force. No
  sign-off is recorded anywhere: the lock records the revisions the
  identities were taken at.

## 2. Transport (the minimum of CG16-BOUNDS-CONTRACT-TRANSPORT)

| Mechanism | What landed | Controls |
|---|---|---|
| (a) placement and packaging | `bounds_view` per crate record of `providers/contract-load-path.json`, computed by the probe (`probe_resource_views` also probes the bounds loader), written by `contracts.py sync`, checked by `check`; the Rust reader refuses an unsorted or unowned view and a module in both views. Packages: the module travels in the unchanged inventory (`contracts.json` lists every file under `contracts/`); no audit route reads contract modules from a package, so no format changed | `providers/tests/test_contract_load_path.py` (generated, checked, omitted module, refused module, overlap); `tests/bounds_pin_hold.rs::a_bounds_module_never_reaches_a_resource_selection` (resource selection of the module refuses; overlap and unowned refused by the reader) |
| (b) C06.2 pins | `bounds-contracts.lock` beside the crate's `Cargo.toml`, C06.2's lock format with the declaration's text digest (`text_sha256`, the quant clause's text included); `quirin-audit pin --bounds` (pin, `--check`, `--regenerate`, `--drop-revision`), `--verify-contracts` verifies both kinds; audit-time decision `pin_standing`, a mismatch holding the declaration (`contract-pin-mismatch`) | `tests/bounds_pin_hold.rs`: unpinned (`no-lock`) held, pinned in force, changed body (`source-changed`, a stale pin), changed clause (`contract-changed`, also a stale lock under `--verify-contracts`), missing entry (`not-in-lock`); `decision_vectors_match_lean` |
| (c) selection on the routes | `bounds_selection::select_for_route` on the pinned route (corpus arbiter), the configured route (`current`) and the contracts route: every bounds view on the manifest's load path, pins evaluated over the audited source, record `ROWS.contracts.json` | `tests/bounds_pin_hold.rs::the_custody_report_is_unchanged_by_the_bounds_selection`; the arbiter below |

Pinned with the tool ([`scripts/pin.sh`](scripts/pin.sh), log
[`artifacts/pin.log`](artifacts/pin.log)): both declarations at `046accf6`, then
`66910c10` joined the same identity (equal sites); `pin --bounds --check` exit 0
at both ([`artifacts/pin-check.log`](artifacts/pin-check.log)).

Not landed (the entry stays open, amended): `.qo` checkpoint transport of
bounds rows, G05a inclusion and inheritance for bounds modules (still
rejected), the cross-unit refutation phase on the routes, and tip tracking for
bounds pins (refused by `pin --bounds`).

## 3. A finding on the way: quant checks did not compile

A tracked bounds module is a `.qv` contract module, so the contract corpus test
compiles it. Before this card the frontend's operator gate refused the `&&` of
every `quant checks` predicate at `Q038` (connectives were admitted only in an
`if` condition): QUANT-INDEX-CHECKS' G05c predicates could not appear in a
tracked module, which its probe (declarations written by `run.sh`) never
exercised. The gate now admits `&&` and `||` inside a `quant checks` clause's
predicate span and nowhere else in a contract module
(`compiler/tests/quant_check_connectives.rs`: three predicates compile; `&&` in
a `requires`, `||` in an `ensures` and a single `&` inside a quant check still
refuse).

## 4. Prediction against outcome

Binaries: BEFORE `9c91a239…` built at `e2031f65a`, AFTER `b53ecd02…` built at
`ed71e910b` (the final code), saved under `private/chunkio-meta-len-bounds-contracts/bin/`
and checked against each run's `run.json`. A first BEFORE (`1dfe3df3…`,
`cdad4dfd0`) was measured before the rebase and gave the same custody and
bounds digests. Scripts: [`scripts/`](scripts/); digests and comparisons in
[`artifacts/`](artifacts/).

| Route | BEFORE | AFTER | Prediction |
|---|---|---|---|
| Corpus arbiter (custody) | 756 / 9 / 4,069, `ce57d194…` | identical | identical: held |
| System route | `eab76f04…` | identical | identical: held |
| Tail-pause slice | `bfe1ff94…` | identical | identical: held |
| Typed-path parity (corpus / system / slice) | 669,449 / 141,186 / 50,426 rows | equal multisets (`072a299c…`, `3ff6cf21…`, `6fc088c2…`) | identical: held (the corpus stream is now deterministic, GAPO) |
| Corpus with `QCORE_BOUNDS_PROPERTY`: custody report | `ce57d194…` | identical | identical: held |
| Bounds rows | `822a7633…`, 162 Safe / 4,759 Unknown | `bf5c8e97…`, 162 Safe / 4,759 Unknown | see below |
| Combined | 111 Safe / 4,801 Unknown / 9 Unsafe | identical | Safe rises by the moved rows: **not reproduced** (nothing moved) |

The custody comparison is [`artifacts/identity-routes.json`](artifacts/identity-routes.json)
(20 files, identical after erasing the run's directory name and preparation
durations). The bounds comparison is
[`artifacts/bounds-compare.json`](artifacts/bounds-compare.json):

* **Selection.** `bounds contracts (pinned): 2 declaration(s) from 1
  module(s); 2 pinned in force, 0 held` ([record](artifacts/after-bounds.contracts.json)).
* **Every row's identity changes** (4,921 rows; one environment digest
  `06b325a5…` → `79bd0d18…`), as predicted.
* **No verdict moves.** Reason tallies by function are identical.
* **9 rows change a reason** (9 judgements in 9 chunkio functions:
  `adjust_layout` calls set, `cio_file_calculate_checksum`,
  `cio_file_format_check`, `cio_file_scan_dump`, `cio_file_write_metadata`,
  `get_available_size`, `cio_meta_cmp`, `cio_meta_read`, `cio_meta_size` call
  get): the call precondition reads ``requires quant checks map, Idx #i: 22 <=
  #i && #i < 24`` instead of ``states no bounds contract``, still
  `bounds-missing-extent`, because each passes `cf->map`, a member with no
  extent (CG16-BOUNDS-POINTER-EXTENTS). No bounds Unsafe was added.

**Prediction miss (named).** The prediction (and the brief's expectation)
had the two accessors' own rows move Unknown → Safe. They have **no rows** on
any audit route: they are `static inline` definitions in a header, and the
routes enumerate each unit's main-source definitions only
(`LegacyFunctionScope::MainSourceDefinitions`; the BEFORE rows contain no
`cio_file_st_*` function). Their bodies are checked against the declarations
only through the API: `crates/qcore-c-gateway/tests/chunkio_meta_len_bounds.rs`
analyses the two bodies as the pinned header defines them under the shipped
module (Safe; a 24-byte caller Safe, a 23-byte caller a fixture violation
Unsafe, a `cf->map` caller Unknown; with a pin hold: Pending, Pending, Unknown,
Unknown). On the routes the declarations are applied at callers as `Declared`
premises (C09.1). Recorded on the transport entry.

**mk_vhost_set probe (round 12).** Run with the AFTER binary
([`scripts/tip-probe.sh`](scripts/tip-probe.sh)): tip `mk_vhost_set` Unknown
(1 diagnostic), `mk_vhost_set_property` Unknown (2); master `mk_vhost_set`
Unknown (6), `mk_vhost_set_property` Unknown (2). The BEFORE binary cannot read
the AFTER load path (`bounds_view` is a new field), so it was not re-run; the
resource selection it would load is unchanged (chunkio's resource view is the
same three modules) and the custody routes above are byte-identical.

## 5. Neighbouring accessors (not written; registered)

`CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS` records what each of the other
`cio_file_st_*` accessors in the same header needs:

| Accessor | Body | Needs |
|---|---|---|
| `cio_file_st_get_hash(map)` (l.70) | `return map + 2;` | a derived result `bounds_return_derived(map, 2, 4, bytes)` (the CRC32) and a read frame: writable today, useful only once callers have an extent |
| `cio_file_st_get_meta(map)` (l.89) | `return map + 24;` | a derived result whose length is the metadata length read from `map[22..23]`: a symbolic length tied to a value read from the object (CG16-BOUNDS-SYMBOLIC-RANGES, relational postconditions) |
| `cio_file_st_get_content(map)` (l.95) | `map + 24 + get_meta_len(map)` | a symbolic offset from header content and the object's size relation: not expressible today |
| `cio_file_st_infer_content_len(map, size)` (l.104) | scalar arithmetic, one `get_meta_len(map)` | the same quant check `22 <= #i && #i < 24` (an escape rule 12l(4) decides inside): writable today |
| `cio_file_st_set_content_len(map, len)` (l.116) | writes `map[10..13]` through `(uint8_t *) &map[10]` | the check `10 <= #i && #i < 14`, read through a local derived by a byte-view cast (rule 12h) in the quant body check, which follows only direct, copied and `P + k` accesses |
| `cio_file_st_get_content_len(map, size, page_size, tainted)` (l.129) | reads `map[10..13]` the same way and `map[24 + meta_len]` under `size > content_offset` | `bounds_extent(map, size, bytes)` (size relates to `map`), the cast-view reading, and a symbolic index (CG16-BOUNDS-SYMBOLIC-RANGES) |

## 6. Follow-ups

* Amended: `CG16-BOUNDS-CONTRACT-TRANSPORT` (what landed; remaining: `.qo`
  transport, inclusion and inheritance, the cross-unit refutation phase, tip
  tracking for bounds pins, and checking header-defined declarations' bodies
  on the routes).
* Registered: `CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS` (section 5). Follow-ups
  704 → 705.
* Not retired: this card's id was never registered.

## 7. Gate

Filled in by the gate records (section added at the gate).
