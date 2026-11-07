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
