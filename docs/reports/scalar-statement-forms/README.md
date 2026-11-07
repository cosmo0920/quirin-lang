# SCALAR-STATEMENT-FORMS (round 26 E)

Card brief: round 26 E (user request of 2026-11-06, "cover more areas to fill
the gaps of verifying as Safe or Provisional Unsafe units"). Base `7b726587b`;
measurement side `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`.

## Census (committed before any analyzer change)

**Evidence.** The user's `quirin-audit current` report at Fluent Bit
`046accf6` (186 configurations), read only through its leaf rows. The five
families carry 1,741 leaves: `unsupported assignment expression` 810,
`unrecognized assignment destination` 293, `braced initializer, expression,
or declaration is not modeled` 293, `unmodeled increment or decrement
expression` 242, ``C operator `sizeof` requires type/operand analysis`` 103.
They sit in 548 functions of 103 units; 25 functions have no leaf outside
these families (the same 25 as the pinned corpus at the base).

**Method: the classifier's own view.** A leaf names a preprocessed line, and
macro expansions put many statements on one line (`rd_kafka_buf_read_i32`
alone expands to a dozen), so the source line does not say which statement
refused. The card adds a measurement-only stream (cargo feature
`scalar-form-census`, off by default and enabled by no workspace member;
`src/scalar_form_census.rs`): `classify_simple` records every statement it
refuses with one of the five refusals, with its token run, and the statement
fallback dispatcher records whether a fallback answered it. A census driver
(`tests/scalar_form_census.rs`, ignored, built only with the feature) runs the
analyzer over each unit's own `clang -E -P` text (`scripts/pp.py`, from the
run's compile database), with one unused contract. A final refusal is a
classified refusal no fallback answered; it is joined to the run's leaf by
unit, function, line and refusal (`scripts/join.py`; the driver's second
`-E -P` keeps every non-blank line, checked per unit). 1,695 of the 1,741
leaves are matched; the other 46 are statements whose classification in the
run depended on a contract the driver does not load. Each matched statement's
reading is classified with the types Clang gives its names (every
`VarDecl`/`ParmVarDecl` of the function, `scripts/shapes.py`).

Per-leaf rows: [`census.jsonl`](census.jsonl); counts:
[`census-summary.json`](census-summary.json). Counts below are statement
readings (a leaf may match several statements of one line).

| Family (leaves) | Typed shape (Clang) | Readings | Rule 41 |
|---|---|---|---|
| assignment expression (810) | pointer-typed declaration (`uint64_t *_vp = &_uva;`, the `rd_kafka_buf_read_*` expansions) | 899 | refused: pointer destination |
| | pointer-typed assignment (string literal, pointer value) | 49 | refused: pointer destination |
| | scalar destination, pointer operand (pointer difference, pointer cast) | 62 | refused: pointer value |
| | several scalar declarators (`size_t of = 0, r;`) | 28 | admitted |
| | scalar destination, scalar value the lexical reader did not read | 14 | admitted unless the value holds `?:` |
| | `volatile` object (`double volatile val = 1.0;`) | 2 | refused: volatile (rule 11h) |
| | pointer declarator among several, path declaration | 14 | refused |
| destination (293) | path store `*(&X) = (int32_t)_uva - 1` (array-count macro) | 96 | refused: unread destination (follow-up) |
| | path store of a pointer or string | 84 | refused (CG11's pointer stores) |
| | pointer compound `p += n` | 54 | refused: pointer destination |
| | scalar compound `len -= 8192*3`, `length += addl` | 35 | admitted |
| | scalar compound with a pointer operand | 3 | refused: pointer value |
| | path compound `x->f += n`, `a[i] \|= m` | 9 | admitted through the typed reader |
| braced (293) | compound literal | 93 | refused (rule 11f's) |
| | record/array initializer list (pointer members, calls) | 122 | refused (rule 11f's) |
| | statement expression (`container_of`, `cfl_list_entry_next`) | 54 | refused (rule 11g's) |
| | type declaration (`typedef struct {…} t_a;`, static-assert `enum`) | 30 | admitted when it declares no object; an object of an inline record (`union {…} u;`) refused |
| step (242) | scalar named object, whole statement or `for` clause (`i++`, `len--`) | 97 | admitted |
| | pointer step (`next++`, `*op++`) | 61 | refused: pointer destination |
| | step nested in a store or call actual (`a[n++] = p`, `f(x, i++)`) | 63 | refused: unread destination |
| | `j--, c++` (comma in a `for` clause), `volatile`, undeclared | 6 | refused |
| sizeof (103) | scalar statement with an unevaluated `sizeof` (`size_t __len2 = (size_t)(sizeof(_v));`) | 78 | admitted |
| | with a pointer operand, a string or a subscript | 23 | refused unless every operand is read |

The 25 functions blocked only by these families: five pointer-element stores
(`append_attribute_to_*`, `append_metric_to_scope_metrics`), a string-literal
pointer assignment (`append_histogram_metric_value`), compound literals and
initializer lists holding pointers or calls (`cmt_histogram_buckets_default_create`,
`rd_kafka_toppar_pause`/`_resume`, `rd_kafka_MetadataRequest`,
`unittest_rdvarint`, `update_logs_tag_records_untracked`), a `volatile`
local (`cmt_metric_dec`), pointer difference (`LZ4HC_countBack`), a cast
pointer store (`ut_write_msgset_v2_header`), a `for`-clause comma
(`chunk_header`), the tokenizer reading `offset - -2000` as a decrement
(`rd_kafka_offset2str`, follow-up), and eight the rule's forms cover:
`LZ4_stream_t_alignment`, `LZ4_streamHC_t_alignment` (local typedef),
`LZ4F_returnErrorCode`, `XXH32_canonicalFromHash`, `XXH64_canonicalFromHash`
(static-assert enum), `LZ4HC_rotatePattern` (unevaluated `sizeof`),
`rd_kafka_version_str`, `_next_target` (several declarators).

**Overlap checked.** W43-INDIRECT-PROTECTED-FORMAL-INCREMENT (`(*n)++`,
`p->count++` behind the formal-mutation gate) refuses with its own refusal,
not one of these five, so this rule is never asked there.
`docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` is the expression-position
design; this rule reads statements and `for` clauses only.

## Rule 41

The rule text is [rule 41](../../agents/rules/41.md) (id checked against
every numeric and lettered id in `docs/agents/rules/`, the registry and the
`integrate/r3`..`r5` branches; 40 is reserved for LINK-STANDING, and no
existing rule's premise covers all six forms: the nearest, 10s's elaborator,
lowers object writes only for names the lexical reader read and is never
asked for a `for` clause). Code: `src/scalar_statement_forms.rs` (the
decision, `VECTORS`, the Clang shape and class readers),
`src/legacy_scalar_statement_forms.rs` (the two hooks), one reader flag in
`src/typed_access_path.rs` (`unevaluated_queries`, set only by this rule),
thin hooks in `legacy_c.rs` (last in `expression_statement_lowered`) and
`legacy_parenthesised_callee.rs` (a refused `for` clause), and the registry
row (`QCORE_GATEWAY_RULES_OFF=41` answers nothing).

| Form | Admitted when | Lowering |
|---|---|---|
| `X++`, `--X`, `X op= E` | `X` a named automatic scalar (Clang type, every declaration of the spelling scalar, not protected) or a scalar typed path; `E` read by the typed reader | `ForgetScalar X`, or `IndirectScalarAccess` with the path's per-hop obligations (frame when written through a pointer) |
| `X = E`, `T X = E;` | as above, the value needing rule 41's reading | the same |
| `T a = E1, b;` | every declarator as above (statement position only) | one instruction per declarator, in order |
| `typedef …;`, `struct/union/enum {…};` | declares no object, no variably modified `typedef`, no `typedef` or enumerator spelling an object of the function | `NoOp` |
| `T x = { E };` | `x` a scalar object | as `T x = E;` |
| `sizeof E`, `sizeof(T)` | Clang does not evaluate the operand | reads nothing |

Refused by name (the older refusal stays byte for byte): `pointer-destination`,
`pointer-value`, `volatile-object`, `aggregate-destination`,
`inline-record-object`, `variably-modified`, `static-storage`,
`shadowed-name`, `protected-formal`, `unread-destination`, `unread-value`,
`mismatched-form`; each consultation is a `QCORE_TYPED_PATH_PARITY` row
(`position: "scalar-statement-form"`).

**Lean.** `proof/QCore/ScalarStatementForms.lean`, audited by
`proof/AuditScalarStatementForms.lean` (22 theorems, axioms only `propext`
and `Quot.sound`): the decision and its vectors (`decision_vectors_hold`,
`admitted_destination`, `pointer_destination_refuses`, `volatile_refuses`,
`inline_record_object_refuses`, `variably_modified_type_refuses`,
`pointer_value_refuses`, `evaluated_query_refuses`); the forms as stores
(`compound_is_assignment`, `step_is_compound`, `postfix_step_side_effect`);
the lowering's soundness (`named_store_refines_forget`,
`compound_refines_forget`, `step_refines_forget`,
`path_store_refines_write`, `declarators_are_sequential`,
`type_declaration_is_noop`); frame preservation (`run_preserves_frame`,
`runs_preserve_frame`, `an_owned_object_stays_owed`,
`a_released_base_has_no_write`); and `a_declined_statement_is_never_safe`.
The Rust table `VECTORS` is the Lean `vectors` row for row
(`the_decision_vectors_hold`).

**Fixtures.** `tests/scalar_statement_forms.rs` (15, each with the complete
diagnostic vector): admitted steps, compound assignments and `for` clauses in
a function the lexical reader cannot read, a compound store through a live
record, a compound store through a released record (`Unsafe`,
`use_after_release`), several declarators, type declarations, unevaluated
`sizeof`, a braced scalar initializer; refused `volatile`, pointer
difference, an inline union object, a `for`-clause comma, a shadowed spelling,
a pointer declarator, an enumerator spelling an object. The unit test
`off_every_refusal_stands` (in `scalar_statement_forms.rs`) runs one fixture
with the rule on (`Safe`) and off (its three refusals).

## Driver comparison (with and without the rule)

The census driver over the same 103 units, once with
`QCORE_GATEWAY_RULES_OFF=41` and once without (`scripts/ab.sh`,
`scripts/compare.py`; no contracts, so a contracted call stays refused):
782 five-family leaves removed (245 in 93 functions the report lists, the
rest in header inline functions), six functions `Unknown` to `Safe`
(`LZ4_stream_t_alignment`, `LZ4_streamHC_t_alignment`, `LZ4F_returnErrorCode`,
`LZ4_getIndexOnHash`, `LZ4HC_rotatePattern`, `_next_target`), no new
diagnostic of any other kind, and only leaves a refusal's frame caused or a
now-summarised callee's call removed elsewhere
([`rule-comparison.json`](rule-comparison.json)). `XXH32/64_canonicalFromHash`
and `rd_kafka_version_str` lose their only family leaf and keep a libc call
the driver does not contract. The final binary was re-run on the moved
units' five translation units with no difference.

## Prediction

[`PREDICTION.md`](PREDICTION.md) (committed `d5495ea53`, before the semantic
change): corpus nine rows `Unknown` to `Safe`, system and slice none, no unit
status change. The integrator runs the declared measurement.
