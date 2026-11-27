# LEAN-4-35-TOOLCHAIN

Moves the Lean pin from `leanprover/lean4:v4.34.0` to `leanprover/lean4:v4.35.0`,
renames the two Lean binders spelled `given`, and makes
`scripts/check_lean_audits.py` refuse a Lean other than the pinned one. No Rust,
contract, model operation or theorem statement changes, so no corpus figure can
move and no arbiter run is due. Base: local `main` `ca3d827fe`.

**User decision (2026-10-15), verbatim.** Asked whether to launch the rename
card and whether it should also move the pin to 4.35, the user answered: "Yes,
we need do this." Read as: launch the card and move the pin to Lean 4.35.0.

## Inventory of 4.35 incompatibilities

Four sources, each reproducible from `scripts/`:

1. **The parser token table** of `import Lean` + `import Std` (the widest
   imports the proof uses), printed under both toolchains
   (`scripts/token_table.lean`, `scripts/inventory.sh`,
   `artifacts/inventory/token_table.diff`): 4.35 adds the keywords `recall`,
   `recall?`, `monotonicity_by`, the prefixes `estack⟨` / `EStack⟨` and the
   quotation `` `(grind_mod| ``, and drops `epost⟨` / `EPost⟨` and `▸*`.
2. **The 4.35 parser source** (`src/lean/Lean/Parser/Command.lean` in the
   installed toolchain). `given`, `requires` and `ensures` are *not* tokens: they
   are `nonReservedSymbol`s of the new def contract clauses (lean4#14825), and
   `optDeclSig` and `defTypeSpec` wrap the binders and the type in
   `withForbiddens #["given", "requires", "ensures"]`. `optDeclSig` is the
   signature of `def`, `abbrev`, `example`, `inductive`/`coinductive`/`class
   inductive` and `structure` headers, constructors and structure fields;
   `theorem`, `instance`, `opaque` and `axiom` use `declSig`, which forbids
   nothing. So the three words fail only as names in an `optDeclSig`
   signature; the brief's observation that only
   `SummaryValidationPremises.lean` failed matches this.
   `scripts/reserved_names.lean` shows each case (`artifacts/inventory/reserved_names-v4.35.0.txt`;
   it elaborates under 4.34).
3. **The 4.35.0 release notes** (read online): the `given` clause (#14825),
   `recall`/`recall?` (#14899), `monotonicity_by` (#14861), `EPost` replaced
   (#14836), the Hoare triple form removed (#14872), `mvcgen` deprecated
   (#14874), `rwa … at h₁ h₂` / `at *` deprecated (#14937), `Lean.reduceBool`,
   `reduceNat`, `trustCompiler` removed (#14953), `Decidable` redefined over
   `Bool` (#8309), `constructor` warning when several constructors match
   (#14854), stricter instance synthesis (#14624), range-notation precedence
   (#15054), singleton-notation indentation (#14350), `Lean.Grind.ToInt` removed
   (#14727).
4. **A search of every tracked `.lean` file** (702: 377 `QCore/` modules, the
   root, 324 audits; none outside `proof/`) and the full build under 4.35, which
   compiles all 378 library modules with `warningAsError = true` and runs all
   324 audits.

| 4.35 change | Uses in `proof/` | Effect |
| --- | --- | --- |
| `given` forbidden in `optDeclSig` | `SummaryValidationPremises.ownInForce`'s binder | parse error; renamed |
| (same word in a `theorem` binder) | `SourceRecord.projection_returns_what_construction_supplied`'s hypothesis | accepted by 4.35; renamed so no Lean name spells a contract keyword |
| `requires`, `ensures` forbidden in `optDeclSig` | none as identifiers (prose and string literals only) | none |
| `recall`, `recall?`, `monotonicity_by` keywords | none | none |
| `▸*`, `EPost`, `epost⟨`, `estack⟨`, Hoare triples, `mvcgen`, removed `Lean.reduce*` | none | none |
| `rwa [r] at h₁ h₂` / `at *` deprecated | 7 uses, each `at` one hypothesis (the form that stays) | no warning |
| `constructor` warns on several matches; stricter instances; ranges; singletons | 218 `constructor` tactic lines | the build is warning-free |
| `BEq` on `Int`, `UIntN`, `IntN`, `Fin` now synthesized through the order packages | `==` / `!=` on these types in model definitions | 88 audited declarations' axiom closures gain `Classical.choice` (below) |

## Renames (names only; no proof changes)

| File | Before | After | Why |
| --- | --- | --- | --- |
| `proof/QCore/SummaryValidationPremises.lean:57,60` | `def ownInForce … (given : Option Contract)`, `\| none => given` | `supplied` | required: 4.35 parse error |
| `proof/QCore/SourceRecord.lean:261,264` | `(given : value a f = some x)`, `rw [given] at carried` | `assigned` | not required; `supplied` is already a `def` in that file |

No Rust, Python or test text names either binder (the vector-binding and
proof-leg tests read other modules; `rg` over `compiler`, `crates`, `scripts`,
`providers`). No proof needed a change under 4.35.

## Builds and audits under both versions

`scripts/both_versions.sh` ran the three runs below in one hold of the shared
heavy lock (`artifacts/runs/lock-*.txt`), each through `scripts/run_lean.sh`:
`lake build` plus every audit via `scripts/check_lean_audits.py`, from
`proof/`, with only `proof/lean-toolchain` selecting the version (the guard's
`Lean X matches proof/lean-toolchain` line opens each log). Private logs:
`private/lean-4-35-toolchain/runs-1/<run>/lean.log`.

| Run | Tree | Active Lean | Exit | Audits | Wall |
| --- | --- | --- | --- | --- | --- |
| `base-434` | base's two files, pin 4.34.0 | 4.34.0 | 0 | 324 passed | 146 s (cold build) |
| `renamed-434` | renamed, pin 4.34.0 | 4.34.0 | 0 | 324 passed | 14 s (incremental) |
| `renamed-435` | renamed, pin 4.35.0 | 4.35.0 | 0 | 324 passed | 136 s (full rebuild) |

`scripts/compare_audits.py` compares two transcripts audit by audit (joining the
axiom lists 4.35 wraps over several lines):

- **base-434 → renamed-434** (`artifacts/runs/compare-base-renamed-434.json`):
  the same 324 audits in order, the same 8,565 printed declarations in order,
  every axiom set identical, every other output line identical.
- **renamed-434 → renamed-435** (`artifacts/runs/compare-434-435.json`): the
  same 324 audits and the same 8,565 declarations in the same order (the
  theorem set is unchanged), no axiom outside `propext`, `Classical.choice`,
  `Quot.sound`, and every other audit output line identical, including
  `AuditAll`'s whole-project closure and statement checks. **88 declarations'
  axiom closures grow**: 38 `propext` → all three, 27 `propext, Quot.sound` →
  all three, 23 none → all three; in 21 audits (`AuditCycle19` 18,
  `AuditOpCallbacks` 9, `AuditNativePlatformMatrix` 8, `AuditRetentionGuard` 8,
  …; the full list is in the JSON).

**Why the closures grow.** Under 4.35, `BEq Int` is
`Std.PreorderPackage.toBEq` of `Int.instLinearOrderPackage`, not
`instBEqOfDecidableEq Int.instDecidableEq`, and that package's closure includes
`Classical.choice`. The same holds for `UIntN`, `IntN`, `Fin` and pairs,
options and lists of them; `==` on `Nat`, `String`, `Char`, `Bool`, and
`decide`, `<`, `max`, `compare` on `Int` are unchanged
(`scripts/beq_axioms.lean`, `artifacts/inventory/beq_axioms.diff`). In the
project, `QCore.ChunkSource.Expr.eval` (`==`/`!=` on `Int`) carries all three
axioms under 4.35 while its derived `DecidableEq` instances carry none, and the
`release_busy_*` theorems, proved by `decide` about `run`, inherit them. The
allowed set is unchanged and the audits accept all three, so the growth is
recorded here, not repaired: no proof or definition is restructured
(constructivity of these closures is not a project requirement; rule text
allows exactly these three).

## Pin move

| File | Change |
| --- | --- |
| `proof/lean-toolchain` | `leanprover/lean4:v4.34.0` → `leanprover/lean4:v4.35.0` |
| `docs/agents/validation.md` (Lean CI hygiene) | amended in place: the pinned 4.35.0, the guard, the both-versions check, the five names |
| `docs/development/build-and-test.md` | the pin, the guard, the names |

Nothing else names the version: `proof/lake-manifest.json` has no packages and
`lakefile.toml` no dependency, so there is no Mathlib or other revision to
check; CI's `leanprover/lean-action@v1` reads `proof/lean-toolchain`, and its
`check_lean_audits.py --skip-build` step now runs the guard. The platform
evidence page and STATUS named no Lean version. `docs/reevaluation/audit_metadata.json`
records `v4.33.1` for its 2026-09-05 audit and stays frozen (constraint 21), as
do historical reports and increments that name 4.34.0.

## The version guard

`check_lean_audits.py` now reads the pin (`pinned_lean_version`: a
`leanprover/lean4:vX.Y.Z[-pre]` line, anything else refused) and, before any
build or audit, runs `lake env lean --version` inside `proof/` — exactly the
binary the audit command `lake env lean` runs — and fails with
`active Lean A is not the pinned Lean P (proof/lean-toolchain)` when they
differ; a failed or unreadable query or a timeout fails too
(`check_pinned_toolchain`). Checked by hand on this worktree: with the 4.34.0
pin, `ELAN_TOOLCHAIN=leanprover/lean4:v4.35.0` makes the guard fail naming both
versions. Tests (`scripts/tests/test_check_lean_audits.py`):
`ToolchainGuardTests` (8: pin parsing incl. a pre-release, refused pins, the
repository pin, a match querying `lake` inside `proof`, three mismatches naming
both versions, a failed and an unreadable query, a timeout, `main` refusing
before any build or audit) and, at the process boundary,
`test_a_lean_other_than_the_pin_exits_nonzero_before_any_build` (a stub `lake`
reporting another version: exit 1 with the message, and the build never runs).
The existing process tests' stub `lake` now answers the version query with the
real pin.

## Patterns new Lean code must avoid

- `given`, `requires`, `ensures` as a name anywhere in a `def`, `abbrev`,
  `example`, `inductive`/`structure` header, constructor or field signature
  (binders or type). They still parse in `theorem`/`instance` signatures and in
  bodies, but use none of them as names, so code moves freely between forms.
- `recall`, `recall?`, `monotonicity_by` as names anywhere.
- An identifier `estack` or `EStack` immediately followed by `⟨`.
- `rwa [r] at h₁ h₂` or `rwa [r] at *` (deprecated; a warning fails the build):
  write `rw [r] at h₁ h₂ <;> assumption`.
- `constructor` where several constructors match (a warning): use `constructor!`
  or name the constructor.
- Do not rely on `#print axioms` closures of `==` on `Int`/`UIntN`/`IntN`/`Fin`
  being free of `Classical.choice`.

## Gate

The standing gate ran on the final code and records tree `b1d371dd7` (clean,
pin 4.35.0, active Lean 4.35.0) through `scripts/gate-whole.sh` under one hold
of the shared heavy lock (11:57:37–12:06:20 UTC), every step's output and exit
in `private/lean-4-35-toolchain/gate-g1/`, nothing piped, no rerun
([gate.json](gate.json), exits in `gate/`):

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 499 units, 512 summaries; 8,098 passed, 0 failed, 15 ignored (`private/lean-4-35-toolchain/tests-g1/`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | `Lean 4.35.0 matches proof/lean-toolchain`; 324 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 374 tests (365 + 9 new) |
| `python3 scripts/build_docs_index.py --check` | 0 | |

Then `scripts/finish-gate.sh`: `build_matrix.py . --gate gate.json` 0 (the
matrix's validation block takes this gate), the Python suite 0 (374), the docs
index 0 and its check 0. `providers/` is unchanged, so the provider gate is not
due. The heavier build evidence is the three runs above (cold builds under each
version); the gate's Lean step was incremental on the 4.35 build.

## Follow-ups

None registered: no residual is open. The ledger stays at 783.
