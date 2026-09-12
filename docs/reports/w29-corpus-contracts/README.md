# W29-CORPUS-CONTRACTS — two faithful size-query contracts decide thirteen uncontracted-call refusals — 2026-09-12

## Headline

The wave-29 arbiter card. One new contract module,
[`contract/qcore/fluent_bit/input_chunk_sizes.qcore`](../../../contract/qcore/fluent_bit/input_chunk_sizes.qcore),
declares **two** trusted assumptions — `flb_input_chunk_get_size` and
`flb_input_chunk_get_real_size`, both read against their pinned bodies at
Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` — and the 185-unit
corpus re-measurement attributes its entire movement to exactly the thirteen
`unsupported` leaves that named those two callees:

| measure | BEFORE | AFTER |
|---|---:|---:|
| units | 185 — **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** | **identical** |
| projected functions | 4,834 — **150 Safe / 84 provisional Unsafe / 4,600 Unknown** | **identical** |
| `leak` | **266** | **266 — unmoved** |
| `unknown_alias` | **4,344** | **4,344 — unmoved** |
| `unsupported` | **49,773** | **49,760** (−13, itemised leaf by leaf in §5) |
| unit status moves / function status moves / reason-code moves | — | **0 / 0 / 0** |
| units with any diagnostic change | — | **1** (`fluent-bit-chunk/flb_input_chunk.c`) |
| leaves added / removed | — | **0 / 13** |

**Nothing is promoted.** Every function that carried one of the thirteen
refusals is `Unknown` before and after, carrying its other refusals unchanged;
the unit stays provisional `Unsafe` on its unchanged five leak leaves. The
leak pin **does not move** — there is still no fifth move in constraint 7's
history — and `unknown_alias` does not move either: at all thirteen sites the
actual resolved, so the admitted call produced **no** replacement refusal (the
`unresolved resource identity` substitution the prediction allowed for
appeared zero times).

Contract corpus **73 → 74 modules**, resource-v1 declarations **39 → 41**,
manifest-selected modules **18 → 19** — each pin updated deliberately
(constraint 5) in `compiler/tests/qcore_contract_corpus.rs`,
`compiler/tests/contract_loader.rs`,
`crates/qcore-c-gateway/tests/{prod_contracts,neighbours}.rs` and
`contract/cycle19-resource-contracts.json`.

No Rust source changes (test pins only), no new diagnostic code (the
`Q-NAMESPACE-DECISION` ice box needed nothing), no new keyword, no Lean module
(§8), no provider change.

---

## 1. The candidate, and why this one

The card's brief: a SMALL number of callees whose **faithful** contracts
decide standing `Unknown`-class leaves. The baseline
(`docs/reports/gateway-c-location/artifacts/diagnostics.json`) carries exactly
**thirteen** `unsupported` leaves naming these two callees — eleven for
`flb_input_chunk_get_real_size` (L18068, L18229, L18424, L18932, L19661,
L19723, L20005, L20116, L20214, L20418, L20436) and two for
`flb_input_chunk_get_size` (L18190, L18375) — all inside
`fluent-bit-chunk/flb_input_chunk.c`, the one corpus unit that defines them,
and nowhere else in the 185 units. Both callees sit in the constructor/destroy
neighbourhood the PROD-CONTRACTS → NEIGHBOURS → AOM chain has been deciding
card by card (`PROD-CONTRACTS-O`), and both were bounded **before** the AFTER
run in [`prediction.md`](prediction.md).

## 2. The contracts, read against the pinned bodies

The module's own comment block carries the full line-cited reading; the
substance:

* **`flb_input_chunk_get_size`** (`src/flb_input_chunk.c:756-759`) is one
  statement: `return cio_chunk_get_content_size(ic->chunk);`. The ChunkIO
  callee (`lib/chunkio/src/cio_chunk.c:315-334`) resets the chunk's own error
  field and reads a backend length field. Nothing allocated, nothing freed,
  no pointer stored.
* **`flb_input_chunk_get_real_size`** (`:767-790`) calls four things, each
  read: `cio_chunk_get_real_size` (same shape, plus a `stat`-backed
  `cio_file_real_size` on the filesystem arm), this module's own
  `flb_input_chunk_get_size`, the declared logging vocabulary
  (`flb_debug` with the declared `flb_input_chunk_get_name` in its argument
  list), and `cio_meta_size` (`lib/chunkio/src/cio_meta.c:75-89`), whose
  filesystem arm remaps the chunk's **own** file into the chunk's **own**
  state (`mmap_file(ctx, ch, 0)`). The rest is scalar arithmetic on locals.

On every path the caller keeps exactly the obligations it had before the
call, so both declarations are `resource_borrowed(ic)` with
`resource_return_none()` — the same reading, for the same reasons, as
`fluent_bit/input_chunk_name.qcore`. Writes into `ic`'s own object graph (the
error-field reset, the remap) are not custody movement, exactly as
`input_chunk_write_header`'s writes through `chunk` were not.

**ABI and modes (constraint 8).** The C declarations in force are
`include/fluent-bit/flb_input_chunk.h:167-168`,
`ssize_t f(struct flb_input_chunk *ic)`. `ssize_t` resolves through the
typedef chain to `long int`, whose LP64 projection admits `{i64, isize}`;
the module spells `isize`. The parameter is one object-pointer level,
`ptr<mut, u8>`. The mode gate compares a declared duty against a validated
W4 summary of the body in the unit; both bodies contain calls the projection
cannot model, so W4 derives no summary and nothing is compared — and the
measured corpus confirms it: **zero** `SummaryConflict` movements, zero
withdrawal descriptions, and the thirteen sites simply admit.

**Nullability is deliberately not claimed.** Both bodies dereference `ic`
unconditionally, but `resource_borrowed` imposes no non-null precondition —
the weaker statement, deliberately (a mode demanding entry custody would
delete the caller's failure paths). The wave-23 `nullable`/`nonnull` decision
belongs to `resource_released` declarations; nothing here releases.

**Declared although defined in the unit.** The PROD-CONTRACTS arrangement,
now used a fifth and sixth time: a declared contract always wins over a
W4-inferred summary, a disagreement is a `SummaryConflict` diagnostic, and
the unit still analyses both bodies — their own function rows are `Unknown`
before and after.

**NOT CLAIMED**, named in the module: the returned value's meaning (the `-1`
arms), the remap's success or the residency it leaves, retention (B03/R13),
and `cio_chunk_get_content_size` itself stays undeclared
(`W29-CONTENT-SIZE-UNDECLARED`, §7).

---

## 3. The route

```sh
git submodule update --init --recursive          # FIRST: the libevent pin gate fires at the END of a run
sh measure/prepare-pin.sh                        # archived as prepare-pin.sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-w29
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
sh measure/run-audit.sh before measure/bin/audit # manifest still at 18 modules
python3 -B measure/byte_identity.py measure/runs/before
# ... land the manifest edit selecting fluent_bit/input_chunk_sizes.qcore ...
sh measure/run-audit.sh after2 measure/bin/audit
python3 -B measure/census.py --before measure/runs/before --after measure/runs/after2
```

A **fresh** Clang pin, per the wave-20 discipline: no earlier card's pin and
not the developer database. The pin's own production database is copied into
`<source>/build/compile_commands.json` and **both runs read the same bytes**
(SHA-256 `b9dbb8f0e15f9111…`, **1,415 entries**, `/usr/bin/clang` spelled).
Both counted runs: **exit 0, zero gateway failures, all six scopes summing to
185 units**, per-scope lines identical.

**This card changes no Rust, so the BEFORE and AFTER runs executed the same
binary** — SHA-256 `b9e76958c24e7445…`, built once in this worktree and
executed from worktree-private `measure/bin/`, never from `target/`. The only
input difference between the two runs is the contract manifest and the module
it selects (the wave-23 AOM-DESTROY-SIBLINGS shape). The new `.qcore` file was
on disk during the BEFORE run but **unselected**, so it was not an input of
that run; the manifest itself was edited only after the BEFORE run and its
byte-identity check completed.

**One discarded attempt, recorded rather than hidden.** The first AFTER
invocation stopped mid-run with exit **101**, its stdout truncated mid-line
inside `librdkafka/rdkafka_queue.c` with no panic text and **no artifacts
written** (only the process-start `contracts.json` existed). Nothing was
measured from it; the log is archived
([`logs/audit-after-discarded-attempt.txt.gz`](logs/audit-after-discarded-attempt.txt.gz)).
The re-run at byte-identical inputs completed exit 0 with all 185 units.
This is an environment-class event about one process execution, not evidence
about the analysed source, and it is reported the way §L21.1 rule 1 requires
such events to be treated: it contributes to no verdict.

### 3.1 The BEFORE run reproduces the baseline byte for byte

The authoritative baseline is card GATEWAY-C-LOCATION's archive — the run the
tip's leak pin of 266 and its 73-module contract corpus describe.
`docs/reports/aom-binding-poison/` and `docs/reports/wave23-c-operations/`
remain the frozen archives beneath it and are **not touched** (constraint 21).

[`byte_identity.py`](byte_identity.py),
[`logs/byte-identity-before.txt`](logs/byte-identity-before.txt):

| artifact | archived baseline | BEFORE run |
|---|---|---|
| `diagnostics.json` | `61ac6bb537c99ff0…` | **identical** (raw and canonical) |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** (raw and canonical) |
| `source.json`, `source-effects.json`, `contracts.json` | — | differ: the checkout-root and identity-digest class |

and `diff_artifacts.py --archived docs/reports/gateway-c-location/artifacts`
([`logs/diff-summary-before.txt`](logs/diff-summary-before.txt),
[`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz))
itemises **zero** differences in every section that carries a verdict:

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 18/18, manifest 18/18, audit_selection 5/5
--- diagnostics:     units 185/185, +0 -0 ~0
--- source:          envelope key `compile_entry`
--- source-effects:  3 result rows changed
```

The residues are the classes every prior re-pin has recorded — this
worktree's absolute path differs from wave 27's — and this card can say so
exactly: `source.json`, `source-effects.json` **and `chunk-contracts.json`
are byte-identical between this card's own two runs**
([`artifact-manifest.json`](artifact-manifest.json)).

---

## 4. BEFORE → AFTER, itemised

[`logs/diff-summary-before-to-after.txt`](logs/diff-summary-before-to-after.txt),
[`artifacts/diff-before-to-after.json.gz`](artifacts/diff-before-to-after.json.gz):

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 18 -> 19 (+1: qcore/fluent_bit/input_chunk_sizes.qcore)
--- diagnostics:     units 185/185, +0 -0 ~1;  envelope diagnostic_counts
--- source:          (no differences)
--- source-effects:  results 96/96 +0 -0 ~0
```

Three of the five artifacts are **byte-identical**; `contracts.json` grows by
exactly the selected module; `diagnostics.json` moves in exactly one unit.

## 5. Every moved leaf, attributed

[`census.py`](census.py), transcript [`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 266 -> 266 (+0);  unknown_alias: 4344 -> 4344 (+0);  unsupported: 49773 -> 49760 (-13)
units with diagnostic-count changes: 1
leaves added: 0; leaves removed: 13
```

The thirteen removed leaves, each `(function, preprocessed line, category,
mechanism)` — the mechanism is the same for all thirteen: *the call rule now
finds a declared contract, the actual resolves, and the call is admitted with
every parameter borrowed and a custody-free scalar result*:

| function | line | origin | category | leaf |
|---|---|---|---|---|
| `flb_input_chunk_get_real_size` | L18190 | `flb_input_chunk.c:779` | `unsupported` | uncontracted call `flb_input_chunk_get_size` |
| `flb_input_chunk_get_projected_write_size` | L18375 | `flb_input_chunk.c:1024` | `unsupported` | uncontracted call `flb_input_chunk_get_size` |
| `flb_input_chunk_release_space` | L18068 | `flb_input_chunk.c:593` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_release_route` | L18229 | `flb_input_chunk.c:831` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_get_projected_write_size` | L18424 | `flb_input_chunk.c:1087` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_map` | L18932 | `flb_input_chunk.c:1749` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_destroy_corrupted` | L19661 | `flb_input_chunk.c:2638` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_destroy` | L19723 | `flb_input_chunk.c:2724` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `memrb_input_chunk_release_space` | L20005 | `flb_input_chunk.c:3174` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `input_chunk_append_raw` | L20116 | `flb_input_chunk.c:3336` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `input_chunk_append_raw` | L20214 | `flb_input_chunk.c:3522` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_flush` | L20418 | `flb_input_chunk.c:3794` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |
| `flb_input_chunk_flush` | L20436 | `flb_input_chunk.c:3831` | `unsupported` | uncontracted call `flb_input_chunk_get_real_size` |

(The `origin` column is the wave-27 presented coordinate of each internal
line, read from the archived artifact.)

Where the prediction landed and where it was better than predicted: item 1
exactly (13 → 0); item 2's allowance for replacement leaves was **not
needed** (zero appeared — every actual at the thirteen sites resolved, and
sparing the thirteen statements' poison retired no further leaf, consistent
with constraint 10a's already-bounded poison); items 3, 4 and 5 exactly.

## 5.1 The pin statements

* **Leak 266 → 266.** Unmoved. The four deliberate moves of constraint 7's
  history stand as recorded; there is **no fifth move**. Not one leak leaf
  was added, removed or re-attributed.
* **`unknown_alias` 4,344 → 4,344.** Unmoved, leaf for leaf.
* **`unsupported` 49,773 → 49,760.** Moved by exactly the thirteen decided
  leaves above, in the deciding direction, zero absorbed.
* **Zero promotions.** Unit histogram, function histogram, every unit
  `reason_code`: byte-identical. `flb_input_chunk.c` stays provisional
  `Unsafe` on its unchanged five leak leaves — a provisional `Unsafe` is
  never an upstream defect claim.

---

## 6. Gate

| step | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,392 passed, 0 failed, 8 ignored** across 215 test binaries, exit 0 ([`logs/workspace-tests.txt.gz`](logs/workspace-tests.txt.gz)) — the wave-28 close's own count; this card adds no test function (the new corpus rows are data inside existing tests) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean, exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **Passed 115 Lean audits**, exit 0 ([`logs/lean-audits-summary.txt`](logs/lean-audits-summary.txt)) — unchanged from the wave-28 close, as §8 requires |
| `python3 -m unittest discover -s scripts/tests` | **Ran 193 tests … OK**, exit 0 — the acceptance-matrix derived-facts tripwire stays green: `corpus.contract_modules` is derived live on both sides of its comparison, so the 73 → 74 growth moves no committed-vs-produced fact |

No constraint-16 row tripped in the final `--no-fail-fast` run: all five
load-sensitive provider/host rows passed inside it, and the 215-binary count
is the evidence the run reached every crate. An earlier run of the suite at
the same tree — taken while sibling worktrees' gates were saturating the
machine (load average 27-36) and before three contract-pin tables in
`compiler/tests/contract_loader.rs` had been updated — recorded those three
pins red (the pins doing their job on a deliberately grown corpus, since
updated) and then met a `rust-lld` SIGBUS when the shared disk reached 100%;
both facts are environment/sequencing records, and the final run above is
the measurement.

Provider code and packages are unchanged; the ledger's 122 provider tests are
carried from the preceding checkpoint. The five packages were **built** for
the audit route (the pinned route selects the ChunkIO package through
`QCORE_CHUNKIO_PACKAGE`), but no template and no `qcore-schemas` version
moved, so constraint 18's rebuild-and-verify obligation is not engaged.

---

## 7. Boundaries and named follow-ons

| name | statement |
|---|---|
| `W29-CONTENT-SIZE-UNDECLARED` | `cio_chunk_get_content_size` is called directly at one refused site (`flb_input_chunk_map` L18822) with the chained actual `ic->chunk`; a ChunkIO-side declaration is a different library's module with its own body reading and was not landed on this card's measurement. The wrapper contracts decide nothing at that site. |
| `W29-SIZE-SIBLINGS-UNDECLARED` | the neighbouring queries the same unit calls uncontracted — `flb_input_chunk_is_up`, `flb_input_chunk_set_up`, `flb_input_chunk_total_size`, `input_chunk_metadata_view` and others — were **not** declared. Each needs its own body reading (several mutate residency state, where borrowed-only is not obviously the whole story); declaring them on the strength of these two neighbours' shape would be the unfaithful-declaration defect card P07 recorded. |
| `W29-AFTER-ATTEMPT-DISCARDED` | the first AFTER invocation exited 101 with truncated output and no artifacts; discarded, archived, re-run green at identical inputs (§3). An environment-class event, no verdict contribution. |

---

## 8. Lean

No Lean module is added or amended, on the AOM-DESTROY-SIBLINGS precedent:
this card changes **no semantic surface** — no checker rule, no loader rule,
no emitted byte; the BEFORE and AFTER runs executed the same binary, and the
only change is a declarative input (a contract module and the manifest row
selecting it) plus deliberately updated test pins. Invariant 6 binds semantic
changes; a grown premise set is not one. The audit count is expected
unchanged at the wave-28 close's 115.

---

## 9. What was touched

* `contract/qcore/fluent_bit/input_chunk_sizes.qcore` — new.
* `contract/cycle19-resource-contracts.json` — one manifest row.
* `compiler/tests/qcore_contract_corpus.rs` — corpus pin 73 → 74, two
  `EXPECTED_CONTRACTS` rows.
* `compiler/tests/contract_loader.rs` — (18, 39, 145) → (19, 41, 145).
* `crates/qcore-c-gateway/tests/prod_contracts.rs`,
  `crates/qcore-c-gateway/tests/neighbours.rs` — manifest count 18 → 19.
* `docs/reports/w29-corpus-contracts/` — this report and its evidence.
* `DEVELOPMENT.md` — one increment paragraph; `AGENTS.md` — headline numbers
  (corpus row, contract corpus, baseline pointer), per the sync rule.

---

## 10. For the integrator

The corpus baseline moves to `docs/reports/w29-corpus-contracts/artifacts/`.
The chain beneath it, every predecessor frozen (constraint 21):
`docs/reports/wave23-c-operations/artifacts/` (the frozen corpus baseline) →
`docs/reports/aom-destroy-siblings/artifacts/` (wave 23) →
`docs/reports/aom-binding-poison/artifacts/` (wave 25) →
`docs/reports/gateway-c-location/artifacts/` (wave 27, presentation-only) →
**this archive** (wave 29). The ledger's corpus row changes in exactly one
figure: `unsupported` 49,773 → **49,760** (−13, all thirteen decided by
declaration, attributed in §5). Leak stays **266**, `unknown_alias` stays
**4,344**, the unit and function histograms are unchanged, and nothing is
promoted. Contract corpus **74 modules**.
