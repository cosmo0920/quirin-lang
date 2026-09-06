# W5 generic resource accounting

Generic loop-summary checking, exact/maximum iteration evidence, ledger
morphisms and categorical account checking move from the compiler's Cycle17
aggregate into `qcore-core::resource_accounting`. All item bodies and all five
generic test bodies are byte-identical to main `51bbfc0`; `parity.json` records
the extracted ranges' hashes. Core retains zero runtime/workspace dependencies.

Compiler `research_extensions` reexports preserve the existing public names
and type identity. A compatibility regression checks the core-owned loop and
ledger types and the qualified checker function signature. The single
aggregate test and Cycle17 orchestration stay in compiler, because they
combine the generic kernel with the unrestricted-C gateway and native runner.

The exact/maximum loop distinction, independent loop-certificate recheck and
negative maximum-count regressions are preserved. Native ledger conversion
interprets a supplied execution record; it does not establish its provenance.
No source-loop support, new custody theorem, native provenance proof or source
verdict promotion is introduced. Existing Lean definitions are unchanged.

The integration gate includes the newly merged whole-program frontend and
standalone provider-package branches. Logs and counts are archived here after
completion. Separate provider conformance and two schema0.2.0 reproducible
builds are recorded in `../w5-provider-packages/`. No C source importer was
changed in this relocation; a subsequent combined full-corpus run covers the
concurrent W4 forwarding increment.
