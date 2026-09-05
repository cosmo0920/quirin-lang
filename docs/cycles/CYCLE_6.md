# Cycle 6: Error Regions and Checked Quant Opening

**Status: Done**

Cycle 6 implements Stage 6 of the prototype roadmap. It extends the Cycle 5
typed-error exit maps with path-specific, sealed cleanup evidence. Cleanup-CFG
lowering and C emission remain Cycle 7 work.

## Phase 0: warm-up prerequisite

Before Cycle 6 implementation begins:

- mark Cycle 5 as done;
- move the Cycle 5 record to `docs/cycles/`;
- update repository documentation to point at Cycle 6 as the active cycle.

## Executable Rust semantics

The Rust core provides:

- stack-disciplined region frames with an entry ledger, acquisition/rollback
  log, and region-local strict-credit remainder;
- exact path-specific cleanup packs on typed-error exits, including paths that
  fail after only partial initialization;
- checked `quant open` restricted to the matching catch and exact region;
- a linear cleanup capability whose only Cycle 6 eliminator is `discharge`;
- reverse-acquisition (LIFO) destructor and rollback execution;
- exact restoration of the region entry ledger after discharge;
- error-only abandonment of the opened region's remaining strict credit;
- rejection of cleanup-free catches with live or credit obligations;
- rejection of missing, duplicate, fallible, and pre-rethrow discharge;
- nested cleanup in deterministic inner-before-outer order;
- transfer of owned resources into error payloads with removal from the local
  cleanup log, preventing duplicate ownership and double cleanup;
- explicit diagnostics Q302 through Q309 for invalid cleanup behavior.

## Lean verification obligations

`QCore/ErrorRegion.lean` defines the corresponding region frame, cleanup pack,
linear capability, opening, discharge, payload exclusion, and nested ordering.
It proves:

1. sealing retains the exact path's acquisition log and credit remainder;
2. `quantOpen` accepts the pack's exact region and rejects a different region;
3. discharge restores the recorded baseline ledger;
4. discharge order is the reverse of acquisition order and covers exactly the
   resources in the pack;
5. discharge abandons exactly the region-local credit recorded in the pack;
6. the cleanup capability is consumed once and cannot be discharged again;
7. cleanup-free catch cannot abandon live obligations or strict credit;
8. resources moved to an error payload are absent from cleanup, while every
   remaining cleanup resource is absent from the payload;
9. partial initialization seals exactly the initialized path prefix;
10. nested cleanup is deterministic: inner LIFO actions precede outer LIFO
    actions.

No theorem may contain `sorry`, `admit`, or a project-defined `axiom`.

## Complete Lean verification instructions

### Prerequisites

Install Elan and ensure it is on `PATH`. Do not select a Lean version manually:
`proof/lean-toolchain` pins the project toolchain, and Lake reads
`proof/lakefile.toml`.

From the repository root, verify the selected tools:

```sh
rustc --version
cargo --version
cd proof
lake --version
cd ..
```

### 1. Reject proof placeholders

From the repository root:

```sh
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then
  echo 'Lean verification failed: placeholder or project axiom found'
  exit 1
else
  echo 'Lean source audit passed: no placeholders or project axioms'
fi
```

Comments in `AuditCycle6.lean` do not contain those reserved proof commands, so
this scan covers every checked Lean source without exclusions.

### 2. Build every imported proof module

```sh
cd proof
lake build
```

The root module `QCore.lean` imports `QCore.ErrorRegion`, so a successful build
elaborates and kernel-checks all Cycle 6 definitions and proofs together with
the proofs from Cycles 1–5.

### 3. Print theorem axiom dependencies

Still in `proof/`:

```sh
lake env lean AuditCycle6.lean
```

This command asks Lean to print the axiom dependencies of every Cycle 6 safety
theorem. Nine Cycle 6 theorems report no dependencies. The region-matching and
list-membership proofs report only Lean's standard `propext`, with the two
decidable payload-filter proofs additionally reporting `Quot.sound`. Any
unexpected project axiom fails the source audit in step 1 and must be
investigated.

### 4. Verify executable correspondence

Return to the repository root and run:

```sh
cd ..
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The Rust tests exercise the same Cycle 6 cases modeled by the Lean theorems:
partial initialization, baseline restoration, LIFO cleanup, credit handling,
linear consumption, payload exclusion, and nested ordering.

### One-shot acceptance sequence

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle6.lean
```

## Verified completion result

Completion is verified by 62 passing Rust tests, warning-free strict Clippy, a
successful 11-job Lean build, a source audit with no proof placeholders or
project axioms, and a theorem-dependency audit reporting only the standard
foundational dependencies described above.
