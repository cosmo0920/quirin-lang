# ARM-OBLIGATIONS prediction

Committed before any code edit, against base
`db2795100a760efa0a2cbcbe13217bffc3cf4733`.

The card makes the gateway carry a rule package's admission obligations into
the published row instead of refusing them: a `Premise` into the row's
assumptions, an `Open` obligation into the denominator as `Unknown`. The only
admission with obligations today is rule 37's `asserted-allocation-premise`
arm at the checked-specification settlement (`Obligation::Premise
("asserted_allocation")` for `rd_calloc` and `rd_strdup`); no other arm
returns one. Its premise is carried only into the rows of a unit in which the
specification stands (an admission whose specification is withdrawn has no
row resting on it).

On the live routes that never happens. In the published base outputs
(`private/baselines/db2795100…/`, the integrator's `baseline:db2795100`
record) no row of any route lists `rd_calloc` or `rd_strdup` among the
declarations in force: the corpus has 185 rows consulting `rd_calloc` and 129
consulting `rd_strdup`, every one under `withdrawn` (the header-inline bodies
are outside each unit's derivation scope); system and slice have none. So no
row gains a premise or an open obligation, no row field is added (the new
fields are omitted when empty) and no `assumptions` digest changes (it is
re-derived only for a row carrying a premise). BASE -> TIP is therefore an
identity on all three live routes, up to the measure tool's known
revision/working-directory digest substitution
(MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY), which must be checked one to one.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The rows the change does move are fixture rows: units that define
`rd_calloc`/`rd_strdup` in scope (`tests/rule37_switch.rs`, the
`checked_allocator_constructors` fixtures) gain `asserted_allocation` in
their assumptions with their verdicts unchanged. No row may move toward
`Safe`; any such row is a soundness finding.
