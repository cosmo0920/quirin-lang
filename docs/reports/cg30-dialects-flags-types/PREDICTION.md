# CG30-DIALECTS-FLAGS-TYPES: prediction

Frozen before any AFTER measurement, on base `b6a3dc42`. Honest ordering note:
the implementation was written while the BEFORE build and the census ran, so
this prediction is not prior to the code. It is prior to every AFTER run and
to the BEFORE totals, and it is fixed by the census below, not by an AFTER.

## Census (`scripts/census.py`, `private/cg30dft/census-1/`)

Every compile command of the four databases the measurements read was re-run
as `clang -E -P` to standard output (read-only).

| Database | `.c` units | `-std=` values | `--target` / `-target` / `-mabi` / `-m32`/`-m64` / `-arch` |
|---|---:|---|---|
| corpus (Fluent Bit, pinned source) | 1,396 | gnu17 2,607; gnu99 184; gnu90 91; c99 40 | none |
| corpus (Monkey) | 142 | gnu17 284; gnu99 142 | none |
| system | 1,400 | gnu17 2,615; gnu99 184; gnu90 91; c99 40 | none |
| tail-pause slice | 1,402 | gnu17 2,619; gnu99 184; gnu90 91; c99 40 | none |

(A command can carry `-std=` more than once; counts are occurrences.) No unit
asks for C17, C23 or C2y, and none selects a target or ABI. C99 and GNU90 are
refused by the configured reader today and stay refused (not in this card's
seven identities).

Extension spellings in the preprocessed text (units with at least one):
`_Complex` family 1,287 of 1,396 Fluent Bit units and 134 of 142 Monkey units
(glibc `bits/floatn.h` typedefs), vector spellings 839 Fluent Bit units
(`flb_simd.h` pulls the SSE2 headers under `FLB_HAVE_SIMD`), `_BitInt` 0,
fixed point 0. In raw `.c` text, vector types are used by five Fluent Bit
definitions: `json_find_escapable_simd` and `is_float` (`src/flb_pack.c`),
`flb_utils_write_str_escaped` and `flb_utils_write_str_raw`
(`src/flb_utils.c`), `flb_skip_leading_zeros_simd`
(`plugins/in_tail/tail_file.c`). No analysed definition's own source spells
`_Complex`, `_BitInt` or a fixed-point type.

## Predictions

1. **Dialects and targets move nothing.** Every measured unit is GNU99/GNU17
   (no registered difference; the request digest keeps its historical
   dialect byte) and carries no target flag. The pinned corpus route reads
   commands through `compiler_options_compatibility`, which this card does
   not change. Corpus, system and slice: no status or leaf change from the
   dialect and target hooks.
2. **The extension prefilter fires almost everywhere** (glibc's `_Complex`
   typedefs), so route G reads each selected definition's typed AST; the
   hook adds a named refusal only to a definition whose own AST holds an
   extension type, node or operator.
3. **Vector policy is decided by the BEFORE statuses of the five SIMD
   definitions above** (measured demand, the brief's rule):
   * if none is `Safe` in BEFORE, vectors are refused like the other three
     families: the five (and any definition whose AST carries a vector type
     through a call or member) gain one named leaf each and **no status
     moves**;
   * if any is `Safe`, that is measured demand: the vector family is adopted
     as custody-neutral data (Clang admits only arithmetic element types, so
     a vector value carries no resource identity) and no vector refusal is
     added; then nothing moves for vectors either.
4. **Complex, `_BitInt`, fixed point:** no corpus, system or slice definition
   uses one in its own AST, so no leaf and no status moves. A definition
   whose typed AST cannot be read in a unit that spells an extension is
   refused as well; predicted count **0** (every analysed definition's AST
   is read by the existing typed routes).
5. Totals therefore stay **629 / 10 / 4,195** (corpus), **44 / 2 / 188**
   (system) and **29 / 1 / 239 / 1 Pending** (slice). No Safe → non-Safe move
   is predicted; any that appears is attributed and named in the report.
