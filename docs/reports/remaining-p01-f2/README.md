# P01-F2 — Binding manifest declarations to the executable's embedded template

Work card P01-F2 (wave 5 follow-on to P01's finding F2), 2026-09-08. This card
closes the recorded gap in
[`docs/reports/remaining-p01/README.md`](../remaining-p01/README.md) §6: a
schema-valid, digest-intact edit of an installed package manifest's
declarations passed `qcorec plugins verify` and `qcorec plugins enable`, and
only `providers/package.py verify` — which *executes* the package's
`--verify-package` self-check — detected it.

The repair keeps the property that made the gap possible in the first place:
**the CLI verifier still does not execute the package.** The manifest template
is read out of the executable's *bytes*, and the installed manifest is compared
against it field for field.

Nothing about verdicts, provider semantics or the plugin protocol changes. All
five shipped packages remain protocol-v1 `Unknown` providers; no raw Cycle 19
unit is promoted; no diagnostic code is added or changed; `qcore-schemas` is
untouched.

## 1. Threat model

What the gate is against, and what it is not:

| Adversary capability | Before | After |
|---|---|---|
| Edit the installed `verifier-plugin.json` (schema-valid, `package_digest` left intact) | **accepted** by `verify`/`enable` | refused, naming the divergent field |
| Edit the installed manifest and its `package_digest` | refused (`executable digest mismatch`) | unchanged |
| Flip a byte in the executable | refused (`executable digest mismatch`) | unchanged |
| Strip, duplicate or truncate the embedded template | not checked | refused, with a distinct diagnostic per case |
| Append a second envelope so a forged template "outvotes" the real one | not checked | refused as ambiguous — neither envelope is accepted |
| **Rebuild or forge the executable itself** | out of scope | **still out of scope** |
| Change the package files after verification | out of scope | **still out of scope** |

The last two rows are the honest boundary. An actor who can produce the
executable controls its embedded template too, so the binding removes exactly
one class — *"edit the manifest, keep the binary"* — and nothing more. This is
not signature verification, not source authenticity, and not an
immutable-execution identity. The operational rule in `providers/README.md`
stands: retain the archive hash from a trusted build record.

### Why the field edits mattered

Neither edit could admit a `Safe` verdict — the core refuses every protocol-v1
`Safe` proposal and re-evaluates every v2 witness itself. What they *could*
change is what the host installs before `exec`:

* `limits.memory_bytes` and every `limits.additional` kind are exactly the
  values `linux-rlimit-v1` turns into `setrlimit` calls
  ([P02](../remaining-p02/README.md)); `linux-confined-v1` installs the same
  plan plus its confinement ([P03](../remaining-p03/README.md)). Widening
  `memory_bytes` from an edited manifest widened the enforced `RLIMIT_AS`.
* `limits.timeout_ms` is the wall deadline and, when `cpu_seconds` is not
  declared, the source of the derived `RLIMIT_CPU`.
* `targets[*]` and `capabilities` are the routing scope: a widened target sent
  a request to a provider whose binary does not claim that library.
* `filesystem_inputs` and `network_access` are declarations a host reads when
  deciding what to admit.

So an unnoticed manifest edit could not forge a verdict, but it could weaken an
*enforced* limit. That is the reason this card exists.

## 2. Binding semantics

### 2.1 The envelope

Each provider executable embeds its manifest template as one contiguous blob,
produced at compile time so it lands in `.rodata` as a single literal:

```rust
runtime::run(concat!(
    "<<<qcore.provider-template.v1:begin>>>\n",
    include_str!("../provider-template.json"),
    "<<<qcore.provider-template.v1:end>>>"
))
```

`providers/common/runtime.rs` strips the envelope by its outermost braces
rather than by the marker spelling — deliberately, so the provider does not
place a *second* copy of a marker in its own binary. Each shipped executable
therefore contains exactly one opening and one closing marker, which a test
asserts on the real built artifacts.

### 2.2 The gate

`crates/qcore-verifier-host/src/embedded_template.rs`:

1. `extract_template` counts marker occurrences in the executable's bytes.
   Zero of both → `Absent`. More than one of either → `Ambiguous`. Opening
   without a following closing marker (including reversed order) →
   `Unterminated`. Payload over 64 KiB → `Oversized`, refused *before*
   parsing. Otherwise the payload is parsed with the strict schema types and
   `validate()`d — a template that is not itself a valid manifest is
   `InvalidTemplate`.
2. `bind_manifest` compares the installed manifest against that template field
   by field, substituting only `package_digest`, which is bound to the
   recomputed SHA-256 of the same bytes that were scanned.
3. `verify_executable_package` calls the pair after its existing path
   containment and digest checks. A divergence returns a `PackageLoadError`
   whose message names the field, the installed value and the embedded value.

The comparison destructures `PluginManifest`, `PluginLimits` and
`SupportedTarget`, so adding a schema field without deciding how it binds is a
compile error rather than a silently unbound declaration.

### 2.3 Where it applies

| Route | Bound? |
|---|---|
| `qcorec plugins verify PACKAGE` | yes |
| `qcorec plugins enable PACKAGE --config …` | yes |
| `qcorec check/emit --plugins CONFIG` (registry load) | yes — on every load, before the pinned-manifest comparison |
| `qcore_audit::source_contract::SourceContractRegistry` | yes — it uses `verify_executable_package` |
| `ProcessProvider::from_package` | **no** — it is a plain loader that checks neither the digest nor the template. It was already so before this card; the conformance harness drives deliberately mutated packages through it. `providers/README.md` now points the registration snippet at `verify_executable_package` instead. |
| `providers/package.py verify` | yes, and then additionally executes the self-check |

## 3. Field coverage

Every field of `PluginManifest` is bound. The middle column names what the
field decides at run time; the right column names the diagnostic field key.

| Manifest field | What it decides | Diagnostic key |
|---|---|---|
| `plugin_id` | dispatch identity, diagnostic attribution | `plugin_id` |
| `package_version` | recorded identity | `package_version` |
| `protocol_versions` | admitted wire versions | `protocol_versions` |
| `evidence_versions` | admitted evidence schemas | `evidence_versions` |
| `entrypoint` | which file is executed | `entrypoint` |
| `package_digest` | executable identity | `package_digest` — compared to the **recomputed** digest, never to the template's build placeholder |
| `targets[i].library/revisions/profiles/dialects/platforms/backends` | routing scope | `targets[i].<part>`, plus `targets.len` |
| `capabilities` | routing scope | `capabilities` |
| `required_foreign_summaries` | required inputs | `required_foreign_summaries` |
| `required_observation_hooks` | required inputs | `required_observation_hooks` |
| `filesystem_inputs` | declared file access | `filesystem_inputs` |
| `network_access` | declared network use (must be `false`) | `network_access` |
| `limits.timeout_ms` | wall deadline; derived `RLIMIT_CPU` | `limits.timeout_ms` |
| `limits.memory_bytes` | `RLIMIT_AS` under `linux-rlimit-v1`/`linux-confined-v1` | `limits.memory_bytes` |
| `limits.output_bytes` | stdout cap and frame limit | `limits.output_bytes` |
| `limits.additional.<kind>` | `RLIMIT_CPU`, `RLIMIT_CORE`, `RLIMIT_FSIZE`, `RLIMIT_NOFILE`, `RLIMIT_NPROC` | `limits.additional.<kind>`, over the **union** of installed and embedded keys, so an added or a removed kind is named |
| `trust_class` | trust label | `trust_class` |
| `certificate_checker` | which core checker the package claims | `certificate_checker` |
| `lean_theorem_family` | claimed theorem family | `lean_theorem_family` |

`network_access` is compared like every other field, but a divergence is
unreachable today: `PluginManifest::validate` rejects `true` on both sides, so
that edit is refused earlier (case C). The unit mutation matrix therefore
exercises the other twenty field classes, each asserted schema-valid first.

Every kind `enforceable_limit_kinds(LinuxRlimitV1)` reports —
`core_bytes`, `cpu_seconds`, `max_file_bytes`, `max_open_files`,
`max_processes`, `memory_bytes` — is therefore bound, as are the two fields the
profiles read outside `setrlimit` (`timeout_ms`, `output_bytes`).

## 4. Refusal matrix

Measured on a freshly built `qcore.cmetrics` package (`providers/package.py
build`), one fresh copy per case, through `qcorec plugins verify`. Messages are
the exact stderr text.

| Case | Before (P01) | `qcorec plugins verify` now |
|---|---|---|
| Z. unmodified package | exit 0 | **exit 0** |
| A. one byte appended to `provider`, digest untouched | exit 1 | exit 1 `executable digest mismatch` |
| B. `package_digest` altered | exit 1 | exit 1 `executable digest mismatch` |
| C. `network_access: true` | exit 1 | exit 1 `verifier plugins may not request network access` |
| **D1. `limits.timeout_ms` widened** | **exit 0 — accepted** | exit 1 ``installed manifest field `limits.timeout_ms` diverges …: installed 600000, embedded 5000`` |
| **D2. `limits.memory_bytes` widened** (the enforced `RLIMIT_AS`) | **exit 0 — accepted** | exit 1 ``… `limits.memory_bytes` …: installed 4294967296, embedded 268435456`` |
| **D3. `limits.output_bytes` widened** | **exit 0 — accepted** | exit 1 ``… `limits.output_bytes` …: installed 8388608, embedded 4194304`` |
| **D4. `limits.additional.max_processes` invented** (the enforced `RLIMIT_NPROC`) | **exit 0 — accepted** | exit 1 ``… `limits.additional.max_processes` …: installed Some(4096), embedded None`` |
| **E1. `targets[0].library` `cmetrics` → `chunkio`** | **exit 0 — accepted** | exit 1 ``… `targets[0].library` …: installed "chunkio", embedded "cmetrics"`` |
| **E2. extra capability added** | **exit 0 — accepted** | exit 1 ``… `capabilities` …: installed {ResourceContracts, Concurrency}, embedded {ResourceContracts}`` |
| **E3. `filesystem_inputs` extended** | **exit 0 — accepted** | exit 1 ``… `filesystem_inputs` …: installed {"extra"}, embedded {}`` |
| **E4. `trust_class` → `third_party`** | **exit 0 — accepted** | exit 1 ``… `trust_class` …: installed ThirdParty, embedded FirstParty`` |
| F. envelope markers overwritten | not checked | exit 1 ``executable carries no `<<<qcore.provider-template.v1:begin>>>` manifest template envelope; its declarations cannot be bound without executing it`` |
| G. envelope duplicated by appending a copy | not checked | exit 1 `executable carries an ambiguous manifest template envelope (2 opening and 2 closing markers; exactly one of each is required)` |
| H. closing marker overwritten | not checked | exit 1 `executable's manifest template envelope is unterminated (no closing marker)` |
| I. payload corrupted in place | not checked | exit 1 `embedded manifest template is malformed: expected value at line 1 column 15` |
| J. embedded template invalid as a manifest | not checked | exit 1 `embedded manifest template is not a valid manifest: …` (unit-tested) |
| K. payload over 64 KiB | not checked | exit 1 `embedded manifest template is 65539 bytes, over the 65536-byte limit` (unit-tested) |
| **X. byte appended *and* `package_digest` recomputed** | exit 0 | **exit 0 — still accepted; see §9** |

`qcorec plugins enable` refuses identically and leaves the selection file
unwritten (measured for D1); `package.py verify` refuses every row above and
additionally executes the self-check. Rows D and E are P01's accepted cases —
the reproduction this card was opened for. Row X is the boundary that stays
open by construction: a *rebuilt* executable carries whatever template its
builder chose. No pre-existing refusal was relaxed, and no gate was weakened to
make a test pass.

## 5. Tests

| Where | What |
|---|---|
| `crates/qcore-verifier-host/src/embedded_template.rs` (5 unit tests) | round trip on an unmodified package; **20 declaration mutations each refused by their own field name**, with `validate()` asserted first so every mutation is schema-valid; the digest edit refused as the substituted field; absent / duplicated / unterminated / reversed / malformed / invalid-template envelopes; an oversized payload refused without parsing |
| `providers/conformance/tests/packages.rs` | on the **five real built packages**: exactly one opening and one closing marker per executable; the shipped package verifies; ten digest-intact declaration edits per package — including `limits.additional.cpu_seconds` and `limits.additional.max_processes` — each refused by field name, skipping any edit a package already declares; a stripped envelope refused as absent. **48 refusals** in total across the five packages. Tampering happens on a copy, so the shipped bytes are asserted unchanged and re-verified afterwards |
| `crates/qcorec/tests/plugin_packages.rs` | CLI-level: a digest-intact `package_version` edit now fails `plugins list` with the divergence message; a *coherently rebuilt* package at the same path binds and is then caught by the separate `changed since enable` pin, so both checks stay exercised and distinguishable |
| `crates/qcore-audit/tests/source_contract_config.rs` | the configured-audit package fixture carries an envelope; the existing identity/digest/target refusals are unchanged (a test that names its own `package_digest` still keeps it, so the digest-mismatch row stays a digest-mismatch row) |
| `providers/conformance/tests/configured_audit.rs` | `a_failed_selected_process_stays_unknown_without_an_in_process_fallback` previously induced the process failure by editing the manifest — which the host now catches first. It is re-induced where only execution can see it: an executable that carries exactly the template its manifest declares, verifies, and then refuses to answer. The property (provider failure stays `Unknown`, no in-process fallback, no promotion) is unchanged |

The CLI and audit fixtures write a plain file whose bytes carry the envelope.
That is faithful rather than a shortcut: the gate reads bytes and launches
nothing, which is exactly the property under test — one fixture is a shell
script whose `exit` precedes the envelope, and it still binds.

## 6. Lean

`proof/QCore/PackageTemplateBinding.lean`, audited by
`proof/AuditPackageTemplateBinding.lean` (28 theorems, no new axiom; every
declaration depends only on `propext` and `Quot.sound`).

The judgment on whether Lean carries content here: extraction is *not* a byte
comparison — it is a uniqueness decision over occurrences, and its refusals
have to be shown disjoint from acceptance. The module proves:

* **Round trip**: an executable made of marker-free prefix, payload and suffix
  around one envelope extracts exactly that payload
  (`extract_envelope`), and the honest package binds
  (`an_unmodified_package_binds`) — so the gate is not vacuously closed.
* **Uniqueness**: a marker-free executable is `absent`
  (`extract_absent`); two envelopes are `ambiguous`, so an appended forged
  template cannot outvote the real one (`extract_duplicate_is_ambiguous`,
  `a_second_envelope_is_named_ambiguous`); truncated and reversed markers are
  `unterminated`.
* **No invention**: every accepted payload is a run the executable itself
  brackets (`extract_payload_is_bracketed`).
* **Completeness of the comparison**: `bind` accepts *iff* the digest matches
  and every declared field agrees (`bind_eq_bound_iff`); a reported divergence
  is a real one (`divergent_names_a_real_divergence`); and **for each of the
  fifteen fields** there is a pair differing only there that is refused by that
  field's name (`every_field_is_bound`) — the field enumeration is
  non-vacuous, which is the property the Rust destructuring enforces at compile
  time.
* **Fail-closed**: no envelope, an unparseable template, or any field
  divergence never binds (`no_envelope_never_binds`,
  `a_malformed_template_never_binds`,
  `a_digest_intact_declaration_edit_never_binds`), and the refusal vocabulary
  is disjoint from acceptance (`refusals_are_not_acceptance`).
* **The substitution is exactly one field**: the template's own digest slot
  never changes the verdict (`the_template_digest_is_not_consulted`).

Deliberate reductions, stated in the module header: bytes are abstract tokens
(so nothing here says the marker spelling is unambiguous in a real ELF file —
the implementation decides that by counting, as modeled); field values are
natural numbers, so field *equality* is modeled and field *shape* is not; and
parsing a payload into a manifest is an opaque parameter, so JSON syntax,
`serde` strictness and `PluginManifest::validate` are outside the model.

## 7. Gate results

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1478 passed, 0 failed, 1 ignored** (102 suites) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **35 Lean audits passed** (34 before; `AuditPackageTemplateBinding.lean` is the new one) |
| `cargo test --manifest-path providers/Cargo.toml --workspace --offline` | **58 passed, 0 failed** (19 suites; same count as the baseline — the new coverage extends an existing test) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --offline -- -D warnings` | clean |
| `python3 -B providers/package.py build` + `verify` on all five packages | all five build and verify (self-check included) |
| `python3 -m unittest discover -s scripts/tests` | 95 passed |

Honest note on flakiness: one full providers run reported
`configured_second_codebase_has_real_process_checked_source_evidence` failing
with `QP006 provider timed out` under heavy parallel load; it passes on its own
and on a rerun with bounded test threads. That is a provider wall-deadline
flake on a loaded host, unrelated to this card's change — the same class as the
`RLIMIT_NPROC` flake recorded in
[`docs/reports/remaining-p03/README.md`](../remaining-p03/README.md) §9.1. No
assertion was weakened.

## 8. Compatibility and deliberate decisions

* **Package format change.** An executable that carries no envelope no longer
  verifies. Every package built before this change must be rebuilt; there is
  deliberately no acceptance path for a template-less executable, because that
  would be exactly the downgrade the card forbids. Operationally: rebuild with
  `providers/package.py build` and re-`enable` the reviewed directory.
* **No `qcore-schemas` change.** Manifest serialization is byte-identical; the
  schema crate is untouched. `qcore_verifier_host` re-exports
  `qcore_schemas::sha256_hex` so package tooling need not depend on the schema
  crate directly — an additive re-export, not a schema change.
* **No package-version bump.** The wire protocol, the manifest schema, the
  responses and the declared fields are unchanged; what changed is the bytes of
  every executable, which the `package_digest` already identifies, and the
  five packages' pinned `package_version`/manifest comparisons stay valid.
* **`ProcessProvider::from_package` unchanged.** Tightening the plain loader
  would break the conformance harness's deliberate mutation drives, which need
  a loader that does not verify. The README now directs callers to
  `verify_executable_package`.
* **Diagnostic wording.** The refusals are host `PackageLoadError` messages,
  not `Q`/`QP` codes; no code was added, removed or changed.

## 9. Open boundaries

* **A rebuilt or forged executable is not detected.** The binding ties the
  manifest to the binary; it does not attest the binary. Signature
  verification and source authenticity remain absent by design.
* **Time of check to time of use.** Package files may change between
  verification and dispatch. The registry re-verifies on every load, which
  narrows but does not close the window; no immutable-file mechanism is added.
* **The template is data, not behavior.** Nothing here shows the executable
  *acts* within the limits or targets its template declares; only the host's
  execution profile enforces limits, and only for the kinds P02/P03 name.
* **`from_package` remains unverified** (P09-style route consolidation, not in
  scope here).
* **Marker collision is decided by counting, not by proof.** If some future
  build embedded the marker spelling twice, the package would be refused as
  ambiguous rather than mis-bound; that is the fail-closed direction, and a
  test asserts the count is 1 on the five shipped artifacts.
* Nothing in this card promotes a verdict, changes CT coverage (20/50 mapped),
  the contract corpus (60 modules) or the four pinned 185-unit artifacts.
