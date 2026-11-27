# K4-FORMAL-CUSTODY-ALIASES: re-census and stop (no semantic change)

Card K4 of round 37 (2026-11-25), base `5087f9295` (origin/main, round 34
landed). Bundle K4 of
[CURRENT-ACCESS-ALIAS-CENSUS](../../language-and-tooling/current-access-alias-census/README.md): the
four formal-custody alias groups (`formal-copied`, `formal-handed-contracted`,
`formal-handed-uncontracted`, `formal-not-handed`). Owner follow-ups:
`W47-ALIAS-CALLEE-BORROWED-CLOSURE` (primary) and `W47-ALIAS-FORMAL-COPY`,
which AG2-UNRESOLVED-ALIAS (rule 62) has already retired.

**Outcome.** The card stops after its Method step 1, as the brief requires
when the remaining yield is near zero. The bundle now frees **3 rows alone**.
On 11-07 it freed 16. Of those 3 rows, one is a narrow typedef gap. The other
two are release wrappers whose verdict waits on a W4 summary choice that
fail-closed does not settle. No analyzer, contract, Lean or test code is
changed, and no rule id is taken (80 is unused). §5 gives the options and a
recommendation.

## 1. Re-census against the user's 2026-11-24 run

**Method.** As in [K5 §1](../k5-local-origin-aliases/README.md). The report
(Fluent Bit `046accf6`, written 2026-11-24 09:58: 1,173 Safe / 3,750 Unknown /
11 Unsafe rows) was read once, read-only, into function rows
([scripts/extract.py](scripts/extract.py)). The scripts then matched each leaf
to the census:

- [scripts/recensus.py](scripts/recensus.py) matches every `unknown_alias`
  leaf of an Unknown row to the 11-07 census leaf of the same
  `(file, function, variable, origin)` and takes that leaf's W47 group.
- [scripts/cooccur.py](scripts/cooccur.py) lists each K4 row's other blockers.
- [scripts/mentions.py](scripts/mentions.py) classifies, lexically, how the
  formal appears in the actuals of the uncontracted callees the census names.
- [scripts/groups.py](scripts/groups.py) gives the census uses and the source
  text at each leaf.
- [scripts/typedef_formals.py](scripts/typedef_formals.py) finds formals
  declared through a pointer typedef.

Outputs are in [artifacts/](artifacts/).

| K4 group | Leaves 11-07 | Rows 11-07 | Leaves 11-24 | Rows 11-24 | Freed by this group alone 11-07 / 11-24 |
|---|---:|---:|---:|---:|---:|
| `formal-handed-uncontracted` | 3,803 | 886 | 3,725 | 866 | 0 / 0 |
| `formal-copied` | 273 | 241 | 155 | 134 | 13 / 0 |
| `formal-handed-contracted` | 150 | 96 | 139 | 88 | 3 / 3 |
| `formal-not-handed` | 63 | 46 | 62 | 46 | 0 / 0 |
| **K4 total** | **4,289** | **1,211** | **4,081** | **1,085** | **16 / 3** (bundle) |

On 11-24 one more row, `context_get_head`, has only `new` leaves. It is not
K4: it is K5's call-result base (`K5-CALL-RESULT-BASE-LOADS`).

**The 16 frozen functions.**

- **12 are now `Safe`, all through AG2-UNRESOLVED-ALIAS (rule 62).** Each one
  is in AG2's PREDICTION.md. They are the comparators, the password and timer
  callbacks and `get_context_arena`, each a formal copied into a local that is
  only read through.
- **Three remain blocked alone, with the same leaf as on 11-07:**
  `metric_escape`, `rd_dl_close` and `mk_config_print_error_msg`.
- **One has a changed blocker:** `rd_kafka_broker_start_reauth_cb`. Its alias
  leaf is gone, and the constructed-state refusal of `rd_kafka_op_new0` it hid
  now blocks it. AG2 §6 recorded this.

**Remaining yield.** 3 rows. `formal-handed-uncontracted` is still the
largest alias group and still frees 0 rows alone. Its other blockers on the
1,085 K4 rows:

| Other blockers | Rows |
|---|---:|
| IND | 861 |
| UNC-RUN | 813 |
| other `unsupported` | 492 |
| COND | 467 |
| UNC-EXT | 199 |

Rows freed if K4 went together with other families:

| K4 together with | Rows freed |
|---|---:|
| UNC-RUN | 69 |
| IND | 53 |
| the other alias groups | 8 |
| IND and the other alias groups | 79 |
| UNC-RUN, IND, COND, RET and the other alias groups | 496 |

([artifacts/cooccur.json](artifacts/cooccur.json)).

**How the formal is mentioned** (`formal-handed-uncontracted`, one count per
row and formal; [artifacts/mentions.json](artifacts/mentions.json)):

- **The row also carries an uncontracted-call leaf: 883 pairs.** In 721 of
  them the formal is handed bare (`f(p)`) to the uncontracted callee. That is
  the callee-closure work itself. In 70, the formal is mentioned only as a
  member read or a member address.
- **The row carries no such leaf: 112 pairs.** In 38 the formal is handed
  bare. In 31 it is mentioned only as a member read or address, such as
  `cfl_list_add(&map->metrics, ...)`.

A member-address actual can be linked into the callee's list, which retains
`p` (`cfl_list_add`). Admitting it therefore needs the "cannot bind, release
or retain p through that position" argument the owner's DISCHARGE asks for.
It frees no row alone, because the group frees none.

**What earlier cards refused on purpose** (nothing below is re-admitted):

- **AG2 (rule 62) withholds its reading from a formal copy that is stepped,
  written through, returned or stored.** All 155 remaining `formal-copied`
  leaves are such copies: `const BYTE* ip = istart; ... ip++`,
  `hashTable[h] = p` through `(U32*) tableBase`. AG2's reason: the report is
  W4's binding signal (`legacy_summary::bind_unresolved_formals`), and
  removing it stopped summary derivation. The owner is
  `W43-INDEX-TRAVERSAL-FORMAL-COPY`.
- **Rule 5b's closure answers "possibly bound" for any mention of a name in
  an uncontracted call's actual.** `W47-ALIAS-CALLEE-BORROWED-CLOSURE` keeps
  this, and P4's validated Borrowed summaries already relax it position by
  position (`tail_file_lstat`, W48-CALLEE-CLOSURE-R3). The owner's DISCHARGE
  forbids "a relaxation for an uncontracted callee's own position".
- **K5 §4 left the release of a caller's formal to a contract or a W4
  summary** (`net_plain_close`).

## 2. The three rows, reduced and reproduced

[scripts/probe.rs](scripts/probe.rs) is a scratch integration test. It is
kept here, outside the test tree, and not compiled. It ran at base as
`cargo test -p qcore-c-gateway --test k4_probe_scratch -- --nocapture`, exit
0. Each body is the row reduced to its shape, with synthetic contracts that
mirror the provider's (`cfl_sds_len`, `cfl_sds_cat_safe`, `free`, `dlclose`'s
`resource_kind(handle, dynamic_library)`, `exit`'s `resource_no_return`).
Output: [artifacts/probe-base-5087f9295.txt](artifacts/probe-base-5087f9295.txt).

| Row | Reduction (base answer) | Bisected cause |
|---|---|---|
| `metric_escape` (`cmt_encode_prometheus.c:80`) | `typedef char *x_sds_t; void w(char **buf, x_sds_t d) { for (...) switch (d[i]) { default: x_cat(buf, d + i, 1); } }`: `Unknown`, `unknown_alias:d` at the switch | `decl(d) = x_sds_t ∧ x_sds_t ≡ char * ∧ subscript(d, i) ∈ cond ∧ actual(d + i)` ⇒ refused. The same body with `decl(d) = char *` is `Safe` (B1). Without the arithmetic actual it is `Safe` (B3), and so is the arithmetic without the subscript loop (B4). Only 1 of the 4,081 K4 leaves has a typedef'd pointer formal ([artifacts/typedef-formals.json](artifacts/typedef-formals.json)). |
| `rd_dl_close` (`rddl.c:151`) | `void w(x_hnd_t *h) { x_dlclose((void *) h); }`: `Unknown`, `unknown_alias:h` | `released(h) ∧ kind(h) = dynamic_library ∧ ¬heap(kind(h))` ⇒ W4 infers no `Released` summary (`W43-DESCRIPTOR-W4-SUMMARIES`), so the row keeps the binding signal's report. With the heap `x_free` the same body is `Safe` under its `Released` summary. |
| `mk_config_print_error_msg` (`mk_config.c:134`) | `void w(char *v, char *p) { x_log(.., v, p); x_free(p); x_exit(1); }`: `Unknown`, `unknown_alias:p` | `released(p) ∧ ∀ e ∈ exits(w). ¬normal(e)`: `x_exit` is `resource_no_return`, so W4 observes no normal exit to validate `Released` on. Without the exit the body is `Safe`. |

## 3. Shapes left, with named reasons (no shape chosen)

| Shape | Rows alone (11-24) | Reason it keeps its refusal | Owner |
|---|---:|---|---|
| formal handed bare to an uncontracted callee | 0 | `mention(p, actual_k(g)) ∧ ¬contracted(g) ∧ ¬validatedBorrowed(g, k)` ⇒ `mayBind(p)`: what the callee does with `p` is unknown | `W47-ALIAS-CALLEE-BORROWED-CLOSURE` (bottom-up callee closure; K4 + UNC-RUN = 69 rows) |
| formal mentioned only as a member read or address in an uncontracted actual | 0 | `actual_k(g) = &p->f` can be linked and so retain `p`, as in `cfl_list_add`; a member read `p->f` hands no copy of `p`, but no row is freed by it alone | `W47-ALIAS-CALLEE-BORROWED-CLOSURE` |
| formal copy stepped, written through, returned or stored | 0 | rule 62 withholds by design (W4's binding signal) | `W43-INDEX-TRAVERSAL-FORMAL-COPY` |
| formal written through (`*cntp = …`, `(*p)++`), not handed | 0 | out-slot stores; no row alone | `W47-ALIAS-CALLEE-BORROWED-CLOSURE` |
| typedef'd pointer formal subscripted, then an arithmetic actual | 1 (`metric_escape`) | a reader on the subscript/arithmetic path does not desugar the typedef the way the spelled `char *` is read; not yet located | `K4-TYPEDEF-POINTER-FORMAL-ARITHMETIC` (registered) |
| release of a kinded, non-heap formal | 1 (`rd_dl_close`) | W4 infers no kinded `Released` summary | `W43-DESCRIPTOR-W4-SUMMARIES` (amended) |
| release of a formal before a no-return call | 1 (`mk_config_print_error_msg`) | no normal exit, so no W4 summary to decide the row under | `K4-RELEASE-BEFORE-NO-RETURN-SUMMARY` (registered) |

## 4. Records

- No PREDICTION.md: no semantic change was made, so the card predicts no row
  move on any route.
- [integration-amendments.json](integration-amendments.json) records the
  re-census and the registrations:
  - it amends `W47-ALIAS-CALLEE-BORROWED-CLOSURE` (re-census and the mention
    split) and `W43-DESCRIPTOR-W4-SUMMARIES` (`rd_dl_close`);
  - it registers `K4-TYPEDEF-POINTER-FORMAL-ARITHMETIC` and
    `K4-RELEASE-BEFORE-NO-RETURN-SUMMARY`.
- No card gate, and no targeted Rust or Lean check: no code changed. The only
  build was the probe's (`private/target-k4`, own target directory).

## 5. Options for the coordinator (a decision is needed)

**A. Close K4 here (recommended).** Land this census. Fold
`formal-handed-uncontracted` into the bottom-up callee-closure lane
(`W48-BOTTOM-UP-RESIDUAL-PLAN`), where its 69 "K4 + UNC-RUN" rows belong: each
needs the callee's validated Borrowed position, never an alias reading.
Expected effect on `current`: none.

**B. A narrow typedef card (`K4-TYPEDEF-POINTER-FORMAL-ARITHMETIC`).**
- What: locate the reader that answers differently for
  `decl(d) = T ∧ T ≡ char *` than for `decl(d) = char *`, and desugar it as
  rule 62 already does.
- Yield: at most 1 row (`metric_escape`, `Safe` once reduced).
- Soundness: a typedef is the same type (C11 6.7.8p3), so the change admits
  nothing the spelled form does not.
- Cost: card-medium or card-high, about a day with Lean parity if the reader
  is semantic.

**C. A W4 summary choice for release wrappers.** This is a user-owned
decision, because fail-closed does not settle it:
- **C1, kinded release:** whether W4 may infer `Released` for a non-heap kind
  (`dynamic_library`, `rd_dl_close`). It needs
  `W43-DESCRIPTOR-W4-SUMMARIES`' kind-preserving summary mode.
- **C2, release then no return:** whether a body whose every exit diverges
  may be decided under `Released ∧ no_return`
  (`mk_config_print_error_msg`).

Each frees 1 row. Neither is an alias reading.

Recommendation: **A**, with **B** queued as a small follow-up if a one-row
card is worth it. **C** waits for the user.
