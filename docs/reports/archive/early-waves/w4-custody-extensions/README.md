# W4 bounded custody extensions

Date: 2026-09-07. Extends the [initial W4 implementation](../w4-custody-inference/README.md).
The inherited unfinished implementation has been reviewed, repaired and validated.
This closes bounded `Out`, `Stored`, `Reallocated` and unresolved nullable-return
inference. It does **not** close W4: `Removed`, general out-slot aliases, recursive
components, cost summaries, the Juliet stretch corpus and crate relocation remain open.
Cycle 19 raw results remain **unpromoted**.

## Implementation and trust boundary

- Direct `*out = value` stores are named by the importer, then modeled only when
  the current candidate or declared contract identifies `out` as an out slot.
  Every exit must initialize each promised slot with a fresh, distinct, live,
  non-null resource, or an explicit null constant under a nullable `Out` mode.
  A resource overwritten in a slot loses its leak exemption. Returning the same
  resource both through a slot and as the function result fails validation.
- `Stored` inference records the exact formal owner of each contained formal.
  Every exit must exhibit the same containment, exactly as declared; fresh
  resources attached to caller-held aggregates still yield no summary.
- Nullable owned-return inference permits the abstract escape only under a
  nullable result contract, and validates that contract by fresh analysis.
  The caller must still resolve nullness before non-null resource operations.
- Reallocation inference distinguishes unresolved replacement, resolved success,
  and failure retaining the old formal. A wrapper that frees the old argument
  on failure is instead summarized as `Released` plus nullable `Owned`; treating
  it as realloc would falsely promise the caller retained custody.
- Declared contracts still win. Every inferred summary is validated by a fresh
  analyzer run and independently rederived by the certificate firewall.
  Non-null reallocation summaries, ambiguous values, inconsistent exit effects,
  unsupported stores and missing summaries remain rejected.

The reviewed draft had a concrete soundness defect: `out = 0; *out = p;` still
used the original symbolic out-slot name and inferred delivery to the caller.
The transfer function now rejects any reassignment of an out formal, including
assignment of a call result or passing its address to another `Out` API. Tracking
the changed pointer would require an alias model not present here. A regression
was run failing before this repair and passing afterward.

Recursion is intentionally unchanged: ascending installation begins with no
inferred summaries, and cannot bootstrap recursive assume-guarantee claims.
The regression `bad(p) { bad(p); }` acquires no summary; its caller's obligation
remains visible. No greatest-fixed-point or termination assumption was added.

## Validation

The standing gate passed on this worktree:

```sh
git -c protocol.file.allow=always submodule update --init --recursive
cargo test --workspace                       # 451 tests passed
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe
cargo run -p qcore-compiler --example reevaluation_followup
(cd proof && PATH=/home/cosmo/.elan/bin:$PATH lake build)
(cd proof && PATH=/home/cosmo/.elan/bin:$PATH lake env lean AuditW4.lean)
```

The summary regression module now has 41 tests. Coverage includes all new modes,
caller leaks and double release, unwritten/overwritten/aliased out slots, nullable
results, reassigned out pointers, stored custody, the realloc-discard trap,
recursive non-inference, declared precedence, and Clang-backed Out/reallocation
certificates that reject summary tampering.

`proof/QCore/CustodySummary.lean` extends the executable validation relation with
out slots, exact containment and reallocation outcomes. The inherited proof
failure for parameterized modes was repaired and the stale audit names updated.
The [axiom audit](lean-axioms.log) contains standard Lean axioms only (`propext`,
`Quot.sound`, and `Classical.choice`); no project axioms or placeholders. These
are theorems about the validation relation, not a proof of the C importer or
arbitrary C alias semantics.

## Corpus comparison

Both full audit runs use Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and the available forced-Libevent
Monkey compilation database. The baseline analyzer is unchanged from
`43040e7d7ed006bc8a98b8987d7e0f2d3974c929`, run by the sibling Cycle 19 evidence
worktree; its observation harness changes do not alter the generic analyzer.

```sh
cargo run --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/w4-extension-source.json /tmp/w4-extension-diagnostics.json
```

All **185 translation units** retain their verdicts, source function sets and
projection coverage: **3 Safe / 98 provisional Unsafe / 80 Unknown / 4 data-only**.
The baseline already includes the original W4 `cio_error.c` improvement. No unit
is promoted by this extension. All 29 Monkey units were measured in this run.

| Diagnostic | Baseline | Extension | Delta |
| --- | ---: | ---: | ---: |
| Leak | 947 | 1,001 | +54 |
| Unknown alias | 8,700 | 8,679 | -21 |
| Unsupported | 42,123 | 42,188 | +65 |
| Double release | 35 | 35 | 0 |

Within unchanged units, 13 functions move Unknown to Safe through validated
inference and nine move Unknown to provisional Unsafe. Additional leak
allegations occur in `cmt_encode_opentelemetry.c` (+6), `rdkafka_cgrp.c` (+1), and
`rdunittest_acknowledge.c` (+47), all already provisional-Unsafe units. They are
not validated upstream defect claims. Naming previously opaque indirect stores
also changes unsupported diagnostics; no hardcoded analysis route was removed.

[parity.json](parity.json) records all unit comparisons, original report hashes
and aggregate diagnostics; [function-diagnostic-changes.json](function-diagnostic-changes.json)
records per-function status changes and exact added/removed diagnostic multisets.
[Baseline](baseline-audit.log) and [extension](extension-audit.log) console logs
retain source/projection findings. The large full resource reports remain
reproducible by the command above; their hashes bind the comparison inputs.
