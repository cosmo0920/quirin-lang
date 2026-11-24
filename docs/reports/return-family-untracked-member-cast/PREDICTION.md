# RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST: prediction

Committed before the implementation commit. Base `7bfa0414b` (the tip of
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST, rule 74r, approved to land as
r72; it sits on rule 74m's `7678fe5f1`, approved as r68, on main
`3ca85c7a6`). USER DECISION 2026-11-24: "launch the
RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST card".

## What was known when this was written

No implementation was measured.

* **Reachability**, from the finding card's base probe (its report §1) and
  the follow-up it registered: `((const struct s *)p)->n` and
  `((const long *)p)[0]` through a null untracked `p` are `Safe`. Family S's
  `scalar_read` makes an untracked root a member or subscript base only when
  `->` or `[` comes right after the name. In `( ( T * ) p ) -> n` a `)` comes
  after it, so `bases` is empty and the flow check never sees the access.
* **Census.** [scripts/census.py](scripts/census.py) reads every translation
  unit that the route's two compile databases name: Fluent Bit 046accf6's
  `fluent-bit-clang-production` and Monkey's `monkey-build`, which together
  cover `src/`, `plugins/`, `lib/` and Monkey. It tokenises each
  `return … ;` (multi-line included, comments and literals blanked) and
  reports a name that reaches `->` or `[` only through closing parentheses
  whose groups hold nothing before the name but `(` and groups of
  identifiers and `*`s. That is the spelling the planned test reads. It
  finds **96 `return`s** (`private/rmc/census-route.txt`, not committed).
  Headers are not read.
  * **71 contain a call-shaped `NAME (`.** Family S refuses a call of any
    kind, so the change cannot reach them: `ORIGVFS ( pVfs ) ->`,
    `G ( L ) ->`, `CFL_SDS_HEADER ( s ) ->`,
    `ntohs ( ( ( struct sockaddr_in * ) sa ) -> sin_port )`, `strcmp (…)`,
    `get_wchar_table_entry ( encoding ) ->` and the like. The census also
    counts `f ( x ) ->` as a hit, and that spelling is refused as a call.
  * **7 take an address or return a pointer.** Two are
    `& ( ( WASMModuleInstance * ) inst ) -> e -> common` and its AOT twin
    (`wasm_native.c:383/388`), which family S refuses for the `&`. Five
    have a pointer-valued result (`lz4.c:2724`, `aot_runtime.c:186`,
    `wasm_memory.c:590`, `wasm_runtime_common.c:1786`, `bh_list.c:93`),
    which is not family S's.
  * **15 dereference a formal parameter.** These are `LZ4_read16/32` and
    `LZ4_read_ARCH` (`lz4.c:389–391`), `rdxxhash.c:167/610`, `st.c:368`,
    sqlite's `pStmt ? ( ( Vdbe * ) pStmt ) -> db : 0` (three),
    `wasm_runtime_common.c:3296/3304`, `yyjson.c:676/689`,
    `zstd_preSplit.c:36` and `zstd_v01.c:2056`. An unbound formal
    discharges the new base obligation: the typed access route takes
    "unbound" as discharged, and the finding card pins
    `long w(const long *p) { return *(long *)p; }` as `Safe`. So these do
    not move unless the body reassigns the formal.
  * **3 have a local root, all in one function:** `lj_debug_line`
    (`lj_debug.c:128–132`),
    `( BCLine ) ( ( const uint8_t * ) lineinfo ) [ pc ]`, where
    `lineinfo = proto_lineinfo(pt)` is tested non-null (`&& lineinfo`)
    before the return. It moves only if family S admits the `return` at
    the base and the row is `Safe` there, and only if the flow check does
    not carry the non-null test to the read. It is the only candidate.

The brief expected `((T *)ctx)->f` on callback context pointers to move
many rows. The census finds that spelling at a `return` mostly inside calls,
which family S refuses, or on a formal, which discharges. Rows outside the
two shapes are an unpredicted move, and the report will account for each
one.

## Planned change

The untracked root's member and subscript base tests keep their current
reading (`->` or `[` right after the name). A second test is added. It reads
back from the root through `(` and through every group
`cast_semantics::type_group_open` reads as a type (rule 12h's
transparency, no second detector), counting the opening parentheses. It then
reads forward through at most that many `)`, and the root is a base when
`->` or `[` follows. A `->` root also carries its C02-P-ALIAS reading, as
`p->f` does. So `((T *)p)->f`, `((T *)p)[i]`, `((T *)(p))->f`, `(p)->f` and
nested groups make `p` a base exactly as `p->f` and `p[i]` do. The set of
bases and `->` roots only grows, and family S refuses exactly where it
refused. No result is admitted that the base did not admit. A new base adds
the typed access route's obligation (`unknown_alias` on a possibly-null or
unknown binding, a `use_after_release` candidate on a released one) and
never removes one.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe` -> `Unknown`, at a `return` family S admits whose
expression accesses an untracked, possibly-null root through a cast group or
a parenthesis. **Expected: none**, with at most one row
(`luajit::lj_debug.c::lj_debug_line`, `Safe -> Unknown`) as the census's one
local-root candidate. No `Unknown` -> `Safe` move is possible.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have (rule 74r's card was refused for a missing
`compile_commands.json`). The integrator should expect the opt-in route's
result. A move there would be one this file does not predict, and it would
be reported.
