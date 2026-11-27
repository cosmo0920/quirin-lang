# W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: frozen prediction

Frozen before the repair is committed, from the reach census of the BEFORE tree.
This file is not edited after the AFTER run; corrections go in the README.

## Inputs

* **Base:** local `main` `b4430a14`. The BEFORE run of this worktree's release
  binary reproduces the combined baseline
  (`docs/reports/w44-indirect-chain-access/integration/artifacts/`):
  * `diagnostics`, `chunk-contracts` and `source` are byte-identical;
  * `contracts` is identical after the checkout-root alias;
  * `source-effects` differs only in the checkout-path digests the brief lists.

  The record is [before-identity.json](before-identity.json).
* **Census:** commits `f326255b` + `f4fc5ef0`, the `indirect-census` feature,
  stream `QCORE_DESTINATION_SUFFIX_CENSUS`. The census run reproduces all five
  BEFORE artifacts byte for byte ([census-identity.json](census-identity.json)).
  The stream (18,597,327 bytes, SHA-256 `6e9c7356…`) is not committed. Its
  analysis is: [census/reach-census.txt](census/reach-census.txt),
  [census/reach-census.json.gz](census/reach-census.json.gz) and
  [census/misnamed-real-names.txt](census/misnamed-real-names.txt). The script
  is [scripts/suffix_census.py](scripts/suffix_census.py).

## Reach

The census records every statement or `for` clause whose left-hand side (before
the first `=`) is not headed by `*` and whose first place-spelling suffix starts
after index 0.

* **38,183 raw records**, **10,979 distinct** records (functions are analysed
  more than once). Six raw records (two distinct) are left unjoined:
  `format_metric` of `unit_seq` 17 matches four `cmt_encode_*.c` units, and all
  six are declarator records.
* **7,853 `declarator`** records. The prefix is identifiers, `*` and `,` only,
  and every one is a declaration (`size_t n`, `struct T * p`,
  `rd_segment_t * seg , * next`). No prefix carries a keyword such as
  `return`, `else` or `case`. **The repair names each of them exactly as
  before.**
* **3,126 `sub-object`** records, split by the last prefix token:

  | Last prefix token | Records |
  |---|---:|
  | `>` (`->`) | 2,389 |
  | `(` | 427 |
  | `.` | 204 |
  | `,` | 58 |
  | other | 48 |

  In 683 of them the first `=` is part of `==`, so the left-hand side is not an
  assignment at all.
* **1,450 sub-object records were admitted** by the BEFORE classifier. Each
  bound its right-hand side to a name that is not the destination:

  | Lowering | Records |
  |---|---:|
  | `Alias` | 1,148 |
  | `Clear` | 295 |
  | `ForgetScalar` | 6 |
  | `ContractCall` | 1 |

  The named root is:

  * **undeclared: 1,416.** Nothing in the function has that name
    (`(rkbuf)->rkbuf_err = …` named `rkbuf_err`).
  * **a local of this body: 24.** Of these, 10 are mentioned on a later line and
    2 are passed later as whole call arguments, for example
    `md->topics[i].topic = NULL` clearing the local `topic` in
    `rd_kafka_parse_Metadata0`.
  * **a formal: 10.** One is later passed as an argument.

  All 34 are listed in
  [census/misnamed-real-names.txt](census/misnamed-real-names.txt).

## Rule forecast (per distinct sub-object record)

| Forecast | Records | What happens |
|---|---:|---|
| `refused` | **1,450** | admitted BEFORE; refused AFTER by the lexical gate. No typed route admits any of them: `typed` is 0. |
| `unchanged-gate` | 742 | already refused by the lexical gate |
| `unchanged-pre-gate` | 506 | refused, or admitted, by a rule before the gate that reads no destination |
| `exchange` | 428 | refused BEFORE for a reason after the gate. The description at that line becomes the gate's. |

## Frozen predictions

1. **Functions: 294 / 85 / 4,455 → 292 / 84 / 4,458** (Safe / provisional
   Unsafe / Unknown).
   * `lz4.c` **`LZ4_write16`** and **`LZ4_write32`**, **Safe → Unknown**. These
     are the only two Safe functions whose certification relied on a misnamed
     destination. Each body is one statement,
     `((LZ4_unalign *) memPtr)->u16 = value` (resp. `u32`), which was lowered
     as `Alias { u16 ← value }`: a binding of an undeclared name instead of the
     store through `memPtr`.
   * `cmt_decode_msgpack.c` **`unpack_basic_type`**, **provisional Unsafe →
     Unknown**. Its only acquisition, `(*map)->opts = calloc(…)`, was a
     `ContractCall` whose result was bound to the undeclared name `opts`. Its
     two leaks (L8378, L8418, "acquired at cmt_decode_msgpack.c:1754") were
     accusations about that name, and they are withdrawn with the lowering.
   * No other status moves. `rd_kafka_handle_OffsetFetch` and
     `flb_input_chunk_get_direct_routes` gain refusals but keep their leaks
     (L21228, L19544): their acquisitions are not on refused lines, and the
     refusals' write frames do not name the acquired bindings.
   * The other 168 functions that gain a refusal are already Unknown.
2. **Units unchanged: 3 / 43 / 135 / 4.**
   * `lz4.c` is already Unknown.
   * `cmt_decode_msgpack.c` keeps nine other provisional-Unsafe functions.
3. **Leak 271 → 269.** Only `unpack_basic_type`'s two leaves are withdrawn;
   none is added.
4. **`unknown_alias`: down by about 1,101.** That many BEFORE leaves sit at a
   refused `Alias`'s line and name its source (`unresolved resource identity
   \`RD_KAFKA_RESP_ERR__UNDERFLOW\``). They are the old lowering's reports and
   leave with it, unless another statement on the same preprocessed line
   reports the same text. A small number may be **added** downstream by the new
   refusals' write frames, only in the 173 functions that gain a refusal.
5. **`unsupported`: at most +188** gate leaves (refused or exchanged lines that
   had no gate leaf) and **at most −428** exchanged descriptions. A description
   stays where another statement at the same line still reports it.
6. **0** `double_release`, `invalid_release` and `use_after_release`
   candidates. `summary_conflict` stays at 1.
7. **Locality.** Every changed leaf lies in one of the 634 functions that carry
   a sub-object record, and every status move is one of the three above.
8. **The repair names every declarator record and every whole-place left-hand
   side exactly as before.** No leaf changes at a line whose only census record
   is a declarator record.
