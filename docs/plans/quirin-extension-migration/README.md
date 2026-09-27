# Planned Quirin extension migration

Status: EXT-01 compatibility implementation is in validation; physical renames (EXT-02/03) and retirement (EXT-04) remain planned. See the [EXT-01 report](../../reports/extension-compatibility/README.md). Decision recorded 2026-09-27 against `2e44e963`. The active checkpoint change keeps `.qo`; the source/ABI renames below belong to subsequent migration cards. This plan does not supersede current loader behavior or rewrite historical evidence.

| Meaning | Target | Existing | Classification rule |
|---|---|---|---|
| Quirin checkpoint object | `.qo` | `.qo` | Binary checkpoint; schema/version migration is independent of extension migration |
| Quirin formal verification source | `.qv` | `.qcore` | **Contract mode: `module contract;`**, as explicitly directed by the user |
| Quirin executable/runtime source | `.qr` | `.qcore` | Executable/runtime source; classify `module library;` inputs by their actual consumer before renaming |
| Quirin C ABI definitions | `.qrlib` | `.qcorelib` | Existing native-library ABI definition/manifest grammar; renaming does not widen linkage or contract authority |

`.qv` identifies contract mode, not a proof certificate and not evidence that a contract body has been proved. `.qr` identifies runtime source, not a native executable. The extensions do not change grammar, semantics, checking obligations, provider trust or verdicts. Keep `.qcore` as a migration alias until the retirement gate below passes; do not silently interpret a contract as an executable from its suffix.

## Inventory at the planning base

`git ls-files` records **616 `.qcore` files**: compiler 223, contract 1, crates 46, docs 182, examples 41, providers 123. Of these, **182 are under frozen `docs/reports/`**, and 434 are elsewhere. There are **5 `.qcorelib` files** (four compiler fixtures, one example), and no tracked `.qo`, `.qv`, `.qr`, `.qrlib` or `.qc` files. Virtual `.qc` source names occur in tests and are separate from tracked files. These are filename counts, not contract coverage.

A lexical scan finds 195 `module contract;` candidates and 421 requiring classification; these include archived inputs and are **not** an approved rename list. The implementation card must obtain module mode from the real parser, identify intentional malformed fixtures, distinguish library inputs from contract-mode inputs, and publish a per-file old/new mapping. Re-inventory at that card's base; the repository is changing concurrently.

Known consumers requiring coordinated work:

- Frontend: `crates/qcore-frontend/src/contract_inclusion.rs` restricts included module suffixes; module-mode checks and diagnostic source paths must remain meaningful.
- Contracts: `crates/qcore-contract/src/{bundle,load_path_manifest,contract_views}.rs`, provider module exports, `contract/corpus.json`, and every named manifest/include/lock/module reference.
- Auditing: `crates/qcore-audit/src/contract_pin.rs` discovers `.qcore` modules; source-contract/profile loaders and the shared source-analysis crate must resolve the same declaration identity.
- Provider packaging: `providers/contracts.py`, `providers/package.py`, load-path generation, inventories, package digests and integrity checks.
- Runtime/native linkage: `quirin check`/`emit`, `.qcorelib` native-library fixtures and their `from …` contract references, generated artifact naming and command examples.
- Tooling: tracked-file test inventories, fixture include paths, source fixtures in Rust/Python, documentation links, editor/file-type associations, CI and package/install commands.

The initial search is a starting inventory, not evidence that every consumer is listed. Each card reruns the repository-wide suffix/reference census and records unhandled references explicitly.

## Candidate cards and order

### EXT-01 — Classification and compatibility readers

Generate the tracked-file mapping with parser-derived module modes and a separate frozen-archive list. Add `.qv`, `.qr` and `.qrlib` acceptance to the appropriate readers while retaining legacy spellings. Define wrong-extension/wrong-mode errors without accepting new syntax; a formal/runtime mismatch must not select a less restrictive checker. Define one module identity when a legacy/new spelling refers to the same declaration: reject ambiguous duplicate candidates rather than choose by search order. Classify `module library;`, mixed multi-file invocations and malformed fixtures explicitly.
The compatibility readers introduced here emit the EXT-04 legacy-extension warning from their first release, including for retained frozen `.qcore` inputs; warning behavior is not deferred until retirement.

Acceptance: old commands retain their verdicts and emitted C bytes; renamed temporary copies have matching results except recorded path spellings. Tests cover both aliases, mismatched modes, duplicate modules, includes, provider load paths, lock evaluation and native-library contract references. Follow the standing gate and add matching Lean only where semantics actually change. No bulk rename before this card passes.

### EXT-02 — Contract-mode `.qv` migration

Rename live contract-mode sources and update includes, provider exports, bundle selections, contract inventories, `.qrlib`/legacy native-manifest references, tests and current documentation atomically. Deliberately update load-bearing contract-count/path tables and package digests; preserve contract text and checked clause counts. Path-addressed lock/module records require explicit migration with declaration/source identity comparisons, never a blind textual substitution or blanket re-pin. Verify that no formerly held/Unknown result becomes Safe because a declaration fell out of discovery or acquired the wrong pin.

Acceptance: tracked mapping accounts for every renamed file, inventories select the same declarations, pins preserve reviewed identities, provider build/verify/conformance gate and core standing gate pass, and archived corpus comparisons attribute every path/digest delta with zero unexplained verdict movement. Keep historical report files byte-exact.

### EXT-03 — Runtime `.qr` and ABI `.qrlib` migration

Rename classified runtime sources and ABI-definition inputs, update current examples/CLI help, fixture paths, native linkage references, build/install tooling and file-type associations. Follow EXT-01's recorded decision for library-mode sources. Runtime and ABI changes may be implemented independently after compatible readers exist, but integrate one at a time.

Acceptance: reference programs emit identical C and retain diagnostics/verdicts apart from explicit source-path changes; native-library linkage and contract attribution remain total; all tracked live references resolve and the standing gate passes.

### EXT-04 — Defaults, deprecation and retirement

Make generated/current examples prefer the new extensions; publish the compatibility window and exact legacy behavior. Remove legacy reader support only after the reference census, package consumers and downstream migration evidence show it can be retired. Historical archives and replay inputs retain old names and a documented compatibility tool/version when necessary. **User requirement: legacy `.qcore` inputs in frozen reports remain usable but are marked outdated and produce a visible warning when loaded.** Attach the naming status to the report index or new companion metadata outside the frozen files, linking to this migration policy and the replacement suffix appropriate to the module mode. The label is about outdated source naming; it does not rewrite the historical evidence or silently invalidate its checkpoint claims.

The compatibility reader emits a stable legacy-extension warning with input path, detected mode and recommended suffix on stderr, and records the warning in its machine-readable diagnostics when that interface exists. A historical input remains supported during the declared compatibility window; warning emission must not change its safety verdict or allow a wrong-mode dispatch. Do not allocate a diagnostic code without the repository namespace procedure. Add tests for warning emission on archived and other legacy inputs, no extension warning for current suffixes, and preservation of the frozen file bytes.

Acceptance: no active loader/package/test depends unintentionally on a legacy suffix; outdated naming metadata and legacy warning tests pass; alias-collision and wrong-mode refusal controls pass; pinned parity evidence is archived before any legacy route is deleted. No retirement date is implied by this plan.

## Resume and evidence requirements

These are prospective cards, not completed implementations or additions to the acceptance matrix. When dispatched, each receives a fresh base, generated task-reading union, committed prediction, bounded ownership, report and full standing gate per `docs/agents/operations.md`. Provider changes add the provider gate. Keep frozen reports unchanged and store rename maps, path/digest attribution and before/after parity in the new card's own report. Any classifier ambiguity or sole tracker displaced by migration must be recorded as an owned follow-up before closing a card.
