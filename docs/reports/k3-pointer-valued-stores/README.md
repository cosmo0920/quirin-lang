# K3-POINTER-VALUED-STORES

Round 34 card K3 (USER DECISION 2026-11-24, verbatim: "Let's get started out of
four"), the second implementation card of
[CURRENT-ACCESS-ALIAS-CENSUS](../current-access-alias-census/README.md)
(bundle K3, census tag `pointer-valued-store`). Owner
`CG11-UNRESOLVED-OWNER-POINTER-STORES` (+ `CG11-UNREAD-POINTER-PLACES`),
gateway rule [10v](../../agents/gateway.md#constraint-10v), amended; no new rule
id. Base `3ca85c7a6`. [PREDICTION.md](PREDICTION.md) was committed (`54490918e`)
before the semantic change (`19c8902e4`).

- **Lean.** `QCore.PointerStoreDerived`
  ([`proof/QCore/PointerStoreDerived.lean`](../../../proof/QCore/PointerStoreDerived.lean),
  audited in `proof/AuditPointerStoreDerived.lean`), and one value class added
  to `QCore.PointerStore`.
- **Rust.** [`pointer_store_derived.rs`](../../../crates/qcore-c-gateway/src/pointer_store_derived.rs),
  one hook in `pointer_store.rs`, one arm in `legacy_pointer_store.rs`.

## 1. Re-census (11-07 vs 11-24)

[scripts/recensus.py](scripts/recensus.py) joins every IND/COND/RET leaf of the
user's `current` report of 2026-11-24 (1,173 Safe / 3,750 Unknown / 11 Unsafe;
the same Fluent Bit `046accf6` compile database, so the analysed text and its
line numbers are the 11-07 census's) to the 11-07 census's tag at the same
`(file, function, line, family)`, and frees a row alone exactly as the census's
`rank.py` does. 718 IND/COND/RET leaves of the 11-24 report have no 11-07 group
(`unmapped`, never counted removable; the 11-07 run's 160 unjoined groups and
Monkey lines among them). Output: [evidence/recensus-1124.json](evidence/recensus-1124.json).

| Measure | 11-07 (census) | 11-24 (re-census) |
|---|---:|---:|
| `pointer-valued-store` leaves (IND / COND / RET) | 2,428 (2,102 / 259 / 67) | 2,167 (1,846 / 254 / 67) |
| Unknown rows touched | 929 | 845 |
| rows freed alone (distinct functions) | 53 (53) | 49 (49) |

Of the 11-07 frozen list of 53:

- **5 are Safe now**: `cJSON.c::suffix_object`, `rd_kafka_buf_set_maker`,
  `rd_kafka_mock_partition_set_leader0`, `rd_kafka_topic_partition_new`
  (AG1's rule 61, a typedef-spelled constructor) and `ut_ack_create_message`;
- **2 changed blocker**: `rd_kafka_Node_new` (AG1 re-attributed its one-arm
  `ret->rack = rd_strdup(rack)` to rule 11c's field-link escape, on purpose)
  and `rd_kafka_pattern_new` (now also `call to uncontracted function
  rd_regex_comp`);
- **46 still blocked by this tag alone** (43 mapped; Monkey's `request_set`,
  `mk_vhost_set_property` and `mk_buffer_cat` are the same statements on
  unmapped lines);
- 6 rows not on the list are freed alone at 11-24 (`unpack_opts_unit`,
  `rd_kafka_sasl_cyrus_cb_getrealm`, the two SCRAM `out->ptr = rd_malloc(…)`
  helpers, `rd_map_insert`, `ut_add_batches_to_map`).

**The 49 by sub-shape**, with what earlier cards refused on purpose (AG1's
section 1.2 and its "refused on purpose" list, rule 10v's own text); nothing
refused there is re-admitted here:

| Sub-shape | Owner / why it stays | Rows of the 49 |
|---|---|---:|
| an owned result or a local acquisition stored into a **formal's** member (`rbuf->rbuf_extra = rd_malloc(…)`, `map->parent = (void *) counter`, `rl->rl_elems = rd_realloc(rl->rl_elems, …)`, `span->attributes = set_attributes_from_ctr(attr)`) | `MEMBER-EFFECT-STORES` (rule 38i names it): a body reading alone would hide the member's new custody from callers; its card designs the member effect | 19 (+2 unmapped Monkey) |
| a formal's or local's pointer stored into a member, or arithmetic over a formal (`pmi->member = member`, `ctx->options.log_cb = log_cb` (declared `void (*log_cb)`, a `void *`), `params.rktpars = rktpars`, `p.data = buf + init`) | Q36 example C's missing owner premise, decision class 1 (`retained_by`) | 6 (+1 unmapped Monkey) |
| an owned result stored into a member of a record this body acquired or declared (`del_records->offsets = rd_kafka_topic_partition_list_copy(…)`, `alteration->user = rd_strdup(username)`, `buckets.p = rd_calloc(…)` into a local record returned by value, `rko->rko_rktp = rd_kafka_toppar_keep0(…)`) | rule 61 / O04-B08's premises (section 3); the reference-count keep is decision class 3 | 11 |
| **arithmetic over the destination's own object** (`stream->state = (pb_byte_t *) stream->state + count`, `w->op_limit = w->op + len`, `io[i].iov_base = (uint8_t *) io[i].iov_base + bytes`) | **read by this card** (section 2) | 3 |
| other: nanopb's `PB_RETURN_ERROR` in a return comma, `EVP_sha*()` results, a conditional of a pointer read, list and map macro expansions (`TAILQ_REMOVE`, `LIST_INSERT_HEAD`, `RD_MAP_SET`/`GET`), a stale local copy (`buf_write`, `LZ4_renormDictT`), `*result = *availrealms` through an out slot | section 3 | 10 |

The rows were classified by reading each statement and body;
[evidence/shape-lex-1124.json](evidence/shape-lex-1124.json) is a lexical first
cut over all 2,167 leaves ([scripts/shape_lex.py](scripts/shape_lex.py)): list
and queue macro expansions 599, calls 582, names 416, member reads 147,
addresses 67, arithmetic 68 (18 of them this card's shape, on 11 rows:
[evidence/derived-yield-1124.json](evidence/derived-yield-1124.json),
[scripts/derived_yield.py](scripts/derived_yield.py)), other 288.

**The remaining yield of K3's own owners is near zero.** The census tag
excludes null stores (W46's census files them as `null-store`), so
`CG11-UNRESOLVED-OWNER-POINTER-STORES`' identity-free values barely appear
(4 string or null leaves bundle-wide), and every large sub-shape is a design
refusal or another owner's. This card therefore reads one narrow shape and
hands the rest back with options (section 6).

## 2. The shape read: arithmetic over the destination's own object

`P = Q ± E` (and `E + Q`), under pointer-to-pointer casts only, with `E` an
integer and `Q` a pointer read of

- **the same place** as `P`: the same side-effect-free spelling (declarations
  by id, members, index operands, literals; `pointer_store_derived::same_spelling`), or
- **the same object** as `P`: both CG10 places root-only over one root.

**Why it is sound.** C11 6.5.6p8: `Q ± E` points into the array object `Q`
points into (or one past it); every other result is undefined. After the store
the destination's object references only objects it already referenced through
`Q`, so the store adds no reference relation, which is exactly what Q36's
example C lacks (`s->p = q` ties `*s` to `*q`: the missing owner premise).
Lean: `a_derived_store_adds_no_reference` (over an object's member provenance,
any read member `q`, the same place when `q = m`) against
`a_foreign_value_may_add_a_reference`. The value carries no identity the
projection tracks: rule 10v's old-value check (no binding of a place rooted at
the destination's root carries a target or is unknown) covers `Q`, rooted at
the same root. The class is neutral, so the owner decides as for every other
neutral value (`an_admitted_reading_is_decided_by_the_owner`: untracked admits;
tracked or unresolved keeps the gate's refusal byte for byte), the read and the
offset carry their own per-hop obligations (`Reader::path` with a pointer read,
`Reader::expression`), and the body still publishes no summary
(`has_pointer_store`). The old value of `P` is P-CG11-CALLER-SLOT's, as for
every rule-10v store.

**Refused, by name** (fixtures in section 4): arithmetic over a formal's or
local's name, another object's pointer, a place read through a pointer past
the root other than the destination itself, a local copy of the place (`d =
s->state; s->state = d + n`: the copy may be stale; nanopb `buf_write`, lz4
`LZ4_renormDictT`), an integer round trip, and any operand the typed reader
refuses (a call in an index).

## 3. Refused shapes, with named reasons

| Shape (rows of the 49) | Reason it stays refused | Follow-up |
|---|---|---|
| owned value into a formal's member (19 + 2) | admitting the body alone would let a caller read the parent as untouched while its member took custody; the member effect is `MEMBER-EFFECT-STORES`' design | `MEMBER-EFFECT-STORES` (amended) |
| formal or local pointer into a member, arithmetic over a formal (6 + 1) | Q36 example C: no owner premise bounds the pointee's lifetime by the owner's; decision class 1 | none new (rule 10v, `W45-SUB-OBJECT-STORE-FORMAL-VALUE`) |
| owned result into an acquired or declared record (11) | the fixtures of the shape are `Safe` at the base ([evidence/base-shape-probe.txt](evidence/base-shape-probe.txt): `s->p = q_alloc()` into `struct S *s = q_alloc()`), so each unit fails a rule-61/B08 premise this card did not isolate (the units are 19k-27k analysed lines; the whole-unit reproduction was stopped after 52 minutes of release analysis); a local record returned by value (`rd_map_alloc_buckets`) is not B08's pointer base; `rd_kafka_toppar_keep0` is a reference-count keep (class 3) | `K3-ACQUIRED-RECORD-CALL-RESULT-STORES` |
| local copy of the place (`buf_write`, `LZ4_renormDictT`) | the copy may be stale: a write to the place between the copy and the store would make `d + n` point into an object the owner no longer references | `K3-DERIVED-STORE-LOCAL-COPY` |
| nanopb `PB_RETURN_ERROR` (`return (stream->errmsg = stream->errmsg ? stream->errmsg : "…"), 0;`; 67 RET + 4 IND leaves on 34 rows, 1 row freed alone) | a conditional of the place's own value and a string literal is identity-free, but it sits in a return comma the return route does not read | `K3-SELF-OR-LITERAL-ERRMSG-STORE` |
| `rk->rk_conf.sasl.scram_evp = EVP_sha256()` (1 row) | a call's result is never read as identity-free without a contract that says the result has static storage | `K3-STATIC-RESULT-POINTER-STORES` |
| list and map macro expansions, `*result = *availrealms`, a conditional of a pointer read (5 rows) | container custody (decision class 1, rule 46), rule 10v's `*slot` exclusion, a pointer read | none new |

## 4. Tests and Lean

- [`tests/k3_pointer_valued_stores.rs`](../../../crates/qcore-c-gateway/tests/k3_pointer_valued_stores.rs),
  5 tests: admitted (the place's own value, a cast, `E + Q`, `Q - E`, another
  member of the object, an embedded element, the same element through a
  pointer member, a loop); refused (a formal's buffer, another object, a
  place past the root on either side, a local copy, an integer round trip, a
  call in the index); a tracked owner (released, returned); adversarial
  releases (the owner released first, a release through the advanced place,
  through an alias taken before the store, of the read value's object); the
  summary gate (the callee `Safe`, its caller not).
- [`tests/unit/pointer_store_derived.rs`](../../../crates/qcore-c-gateway/tests/unit/pointer_store_derived.rs),
  3 tests: every reading vector is Clang's reading (16 vectors), the Lean
  vectors are the Rust table, the spelling comparison.
- **Re-pins, each named in its test:**
  - `src/pointer_store.rs` `DECISION_VECTORS` (and the Lean table): the
    arithmetic row `s->name = s->name + 1` became `s->name = t->name + 1`
    (still `value-shape`); two rows added (`s->name = s->name + 1`,
    same-object-derived, untracked, `admit`; `o->name = o->name + 1`, tracked,
    `owned-field`), 17 -> 19;
  - `tests/cg11_pointer_stores.rs`: the refused-values row likewise, and the
    row count 17 -> 19;
  - `tests/unit/pointer_store.rs` classifies through `classify_store` (the
    reader's own classification) and lists the new class;
  - `tests/unit/audit_ir_fragments.rs`: the new label round-trips.
- **Lean** `QCore.PointerStoreDerived`, 12 theorems, audited in
  `proof/AuditPointerStoreDerived.lean` (`propext`, `Quot.sound` only; no
  `sorry`, no project axiom): `a_derived_store_adds_no_reference`,
  `a_foreign_value_may_add_a_reference`, `offset_keeps_provenance`,
  `readMember_mem`, `admitted_only_from_the_own_object`,
  `a_non_integer_offset_refuses`, `an_integer_cast_refuses`,
  `another_root_refuses`, `a_place_past_the_root_refuses`,
  `a_refused_reading_keeps_the_refusal`,
  `an_admitted_reading_is_decided_by_the_owner`, `reading_vectors_hold`.
  `QCore.PointerStore` gains the value `sameObjectDerived` (neutral); its 18
  theorems hold unchanged.

Stated, not proved: P-CG11-CALLER-SLOT (unchanged), faithful Clang extraction
of types, cast kinds, declaration ids and spellings (trust boundary 28), and
C11 6.5.6p8 itself (the model takes it as `offset`).

## 5. Unit reproduction

[scripts/unit_probe.rs.txt](scripts/unit_probe.rs.txt) (a throwaway test,
kept as text, never a cargo target) runs `analyze_legacy_c_translation_unit`
over each unit's analysed text with `cycle19-resource-contracts.json`, the
manifest the user's run read; [scripts/leaf_lines.py](scripts/leaf_lines.py)
writes the texts from the run's own compile database. Release build at the
implementation commit `19c8902e4`, 15 smaller units
([evidence/unit-manifest.txt](evidence/unit-manifest.txt)), 491 s:
[evidence/unit-reproduction-tip.txt](evidence/unit-reproduction-tip.txt).

- **`buf_read`, `writer_set_expected_length` and `mk_iov_consume` are `Safe`**,
  each with no diagnostic; in the user's report each carried exactly the one
  derived-store leaf.
- Every other probed row keeps its refusal at the same lines.
- Harness limit: in chunkio, cmetrics, ctraces and Monkey server units the
  harness adds `C extension types could not be excluded … (CG30)`, which the
  route does not (it passes the configuration's dialect); those rows' other
  leaves agree with the report.

## 6. Options for the rest of the bundle (hand-back)

The 11-07 forecast (20 Safe, band 13-26) assumed the bundle's 53 rows were one
card's. They are not: 3 are this card's shape, and the other 46 belong to
decisions this card may not take alone.

1. **`MEMBER-EFFECT-STORES`, the body half first** (19 rows of the 49, + 2
   Monkey): read `P->F = acquire()` / `P->F = local` into an unrebound formal
   parent as a hand-off, with the summary withheld and every caller refused
   at the call (rule 38j's minted-contract pattern). **Recommended next**: the
   largest share, and fail-closed for callers by construction.
2. **`K3-ACQUIRED-RECORD-CALL-RESULT-STORES`** (11 rows): isolate which rule-61
   / B08 premise each constructor fails (the parity stream on the
   integrator's `current` run); likely contract or alias-spelling gaps rather
   than new semantics.
3. **Example C's owner premise** (7 rows): a user decision whether a stated
   premise (a formal's pointee outlives the record it is stored into) may
   stand for constructors that store a formal into a fresh record.

## 7. Prediction against outcome

Measured here only by the unit reproduction (section 5); the pinned routes and
`current` are the integrator's.

| Prediction | Outcome |
|---|---|
| `buf_read`, `writer_set_expected_length`, `mk_iov_consume` Unknown -> Safe | **Held** in the unit reproduction |
| the other 8 touched rows stay Unknown | not reproduced (their units were not in the small set, or the rows have other leaves); expected |
| no Safe -> non-Safe, no Unknown -> Unsafe | **Held** on the fixtures and the reproduced units |
| callers do not move | **Held** (`a_body_with_a_derived_store_publishes_no_summary`) |

## 8. Shared-file hook points

None of the brief's shared files (`legacy_c.rs`, `legacy_cfg_builder.rs`,
`legacy_transfer.rs`, `legacy_classify.rs`, `typed_access_path.rs`,
`typed_field_access.rs`) is edited. Outside the new module:

- `crates/qcore-c-gateway/src/pointer_store.rs`: the
  `ValueClass::SameObjectDerived` variant, its code, its neutral arm, its
  `Deserialize` entry; two decision vectors and one re-pinned row;
  `classify_store` (new function, above `root_only`); in `read_store`, the
  classification call (delimited `Card K3-POINTER-VALUED-STORES hook`) and the
  derived value's read and offset (one delimited block after the address
  value's path).
- `crates/qcore-c-gateway/src/legacy_pointer_store.rs`: one arm in `value_class`.
- `crates/qcore-c-gateway/src/lib.rs`: `mod pointer_store_derived;` after
  `mod pointer_store;`.
- `proof/QCore/PointerStore.lean`: one constructor, one neutral arm, the vector
  table; generated `proof/QCore.lean` regenerated.
- `docs/agents/rules/10v.md`; generated `docs/agents/gateway.md` and
  `docs/agents/README.md` regenerated.

## 9. Gate

**No card gate closes this card**, on the user's instruction relayed by the
integrator (2026-11-24, verbatim: "We don't need to run the each of K-related
gates. We must run the merged K-related tip's gate."). The scoped gate
(`python3 scripts/gate_scope.py --base 3ca85c7a6 --out private/gate-1`, mode
full: the diff reaches `qcore-c-gateway`) was started at `19c8902e4`, ran its
submodule (exit 0), fmt (0) and Clippy (0) steps, and was **stopped on that
instruction** while its Rust suite waited in the audit queue (position 9);
the two PIDs it had started (`gate_scope.py`, its `gate_slot.py rust-suite`)
were killed, nothing else. No Rust-suite, Lean-all or Python-suite result of
it exists; the integrator's merged landing gate is the arbiter.

Targeted checks run here (logs under `private/k3/`, worktree-private):

| Command | Exit | Count |
|---|---:|---|
| `cargo test -p qcore-c-gateway --test k3_pointer_valued_stores` | 0 | 5 passed |
| `cargo test -p qcore-c-gateway --test cg11_pointer_stores` (after the re-pins) | 0 | 12 passed |
| `cargo test -p qcore-c-gateway --lib pointer_store` | 0 | 12 passed (pointer_store, pointer_store_derived, legacy pointer-store flow, audit IR label) |
| `cargo clippy -p qcore-c-gateway --lib --tests -- -D warnings` | 0 | |
| `cargo fmt --all -- --check` (the gate's step) | 0 | |
| `gate_slot.py lean -- gate_lean.py --module QCore.PointerStoreDerived --module QCore.PointerStore --audit AuditPointerStoreDerived.lean --audit AuditPointerStore.lean` | 0 | 3 targets built, 2 audits passed |
| `python3 scripts/apply_amendments.py docs/reports/k3-pointer-valued-stores/integration-amendments.json --check` | 0 | 3 amended, 4 registered |

Process note: while stopping the large-unit reproduction (section 5) a
`pkill -f` on this worktree's own probe-binary path was used, against the
rule; it matched only that binary (unique to this worktree) and its own shell.
