# ARM-OBLIGATIONS-UNIT-GRAIN prediction

Committed before any code edit, against base
`3e53c34fe100b72dd014224bfa87d8719d71b108` (origin/main after ARM-OBLIGATIONS
and CI-RUST-JOB-PEAK). The base side of the measurement is the verified
published baseline `baseline:98b1bb785` (all three live profiles);
`98b1bb785..3e53c34fe` changes no analyzer code (CI, package scripts and
records only).

The card narrows the carriage of a rule package admission's obligations from
the unit to the rows whose derivation consulted the admitted declaration.
Rule 37's `asserted-allocation-premise` admission is per specification
(`rd_calloc`, `rd_strdup`). A row keeps the specification's obligations when
the specification's name is reached from the row through the consultations
of its own work (rule 13f's per-function consultations, its applied callees,
its decided callback candidates and its own name), closed over every in-unit
function whose work it consulted. Positions whose attribution is not
established (the hosts' consultations, a nested analysis handing its
carriage up) keep the unit grain.

Expected movement against `baseline:98b1bb785`:

- **No verdict moves on any route.** Rule 37's admission carries a premise
  only (`premises_alone_keep_the_verdict`); no package returns an open
  obligation, so dropping a premise from a row cannot change its status or
  diagnostics.
- **corpus:** of the 2,693 rows of the 54 librdkafka units that carry
  `asserted_allocation` at base, most lose it and become content-changed:
  their `rule_obligations` field is omitted again and their
  `hypothesis_context.assumptions` returns to the digest of the sources
  alone. The rows that keep it are the rows whose derivation reaches
  `rd_calloc` or `rd_strdup`. At base, 271 carrying rows (256 `Unknown`, 12
  `Safe`, 3 `Unsafe`, in 53 units) name one of the two in their rule 13f
  `contract_dependencies`; the kept set should be close to those rows. It can
  be somewhat larger, because the card's closure also follows in-unit
  functions with a declaration rather than an inferred summary, and it can
  differ slightly because the attribution reads only the unit's own
  analysis, not the union the audit's recording holds. Expected kept rows:
  roughly 250 to 600; expected rows that lose it: roughly 2,100 to 2,450.
  Rows outside those 54 units are unchanged.
- **system, slice:** no row carries an obligation at base; both stay
  byte-identical under a shared `--build-commit`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

A row moving toward `Safe`, a row gaining an obligation it did not carry at
base, or a carrying row outside the 54 units is a soundness finding.
