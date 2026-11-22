
### Race merge with integrate/r57

origin/main moved to 1b7028430 (RK-TOPIC-PARTITION-LIST-CONSTRUCTORS: a new librdkafka contract module, the librdkafka locks and contract-load-path.json; the same provider as SP6's cjson.qv, so a wider targeted check). Merged in 3c79b276c (increment page and acceptance matrix resolved, amendments re-applied, contracts.py sync, records regenerated, land.py check and providers/contracts.py check exit 0; r57 brings no string hypothesis). Targeted check: `run_test_binaries.py --no-doc --packages qcore-contract,qcore-frontend,qcore-compiler` exit 0 (221 units, 2883 passed, 0 failed, 4 ignored); `cargo test --manifest-path providers/Cargo.toml -p qcore-provider-librdkafka --locked --offline` exit 0; `python3 -m unittest discover -s scripts/tests` exit 0 (807 tests).

### Race merge with integrate/r58

origin/main moved to 94e0dd906 (NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP: qcore-c-gateway and a new Lean module; disjoint code, shared file proof/QCore.lean imports only). Merged in 663892ace (records resolved and regenerated; land.py check and providers/contracts.py check exit 0). Targeted check: `scripts/gate_lean.py --module QCore --module QCore.NullableLazyConstructor --module QCore.PremiseFormula.CjsonTypedAtoms --module QCore.LocalTypedAtoms` with the three audits AuditNullableLazyConstructor, AuditPremiseFormulaCjsonTypedAtoms, AuditLocalTypedAtoms: exit 0, 4 targets built, 3 audits passed.
