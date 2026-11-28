# K2-BUILTIN-BIT-COUNTS: prediction

Frozen before any tip measurement. Base `287dce8ed` (r86).

**Disclosure.** The gateway code (rule 94, `bit_count_builtins.rs` and its
one hook), the Lean module and the fixtures were drafted in the worktree
before this file was committed. The drafted code was run only on the card's
fixtures (`crates/qcore-c-gateway/tests/k2_no_typed_access.rs` section (3),
whose base answers the probe commit pins under [probes/](probes/)); nothing
had been measured on a native route. The base side of the opt-in current
run was queued for an audit slot.

## Census at base

The bit-count builtins are spelled in the prepared tree
(`private/fb-046accf6`, Fluent Bit 046accf6) by these units of the opt-in
current route's libraries (cmetrics, ctraces, librdkafka, chunkio,
fluent_bit, and Monkey linked): librdkafka's `snappy.c` (macros `rd_clz`,
`rd_ctz`, `rd_ctz64` = `__builtin_clz`, `__builtin_ctz`, `__builtin_ctzll`)
and `lz4.c`. No other unit of those libraries spells one
(`grep -rl '__builtin_c[lt]z'` over the tree's `.c`/`.h`: the other hits are
nghttp2, mruby, jemalloc, sqlite, zstd, yyjson, luajit and libbacktrace,
none analysed by the route).

| Function | Site | Expected at tip |
|---|---|---|
| `log2_floor` (`snappy.c`) | `return n == 0 ? -1 : 31 ^ rd_clz(n);`, `n` a `u32` formal never written | **Unknown -> Safe**: the call is the `?:`'s false arm, `n == 0` proves `n ≠ 0` there; its sole blocker (K2-NO-TYPED-ACCESS's re-census) is this leaf |
| `find_lsb_set_non_zero`, `find_lsb_set_non_zero64` (`snappy.c`) | `return rd_ctz(n);`, `return rd_ctz64(n);`, unguarded (the name is the caller's precondition) | unchanged (Unknown, base leaf byte for byte), if analysed at all (both are `RD_UNUSED` static inline) |
| `LZ4_NbCommonBytes` (`lz4.c`) | four returns `(unsigned)__builtin_c{t,l}z{,ll}((U{32,64})val) >> 3`; `assert(val != 0)` is `((void)0)` | **unchanged Unknown** with its four base leaves: no guard; the 32-bit arms also narrow `val` |
| `compress_fragment` (`snappy.c`) | calls `log2_floor(table_size)` | unchanged Unknown (pointer arithmetic and other leaves); at most a leaf trade at that call if its base leaf names `log2_floor` |
| callers of `find_lsb_set_non_zero{,64}` and `LZ4_NbCommonBytes` | | unchanged |

## Predicted moves

* opt-in current (`--monkey-linked`, the route this host runs): `log2_floor`
  Unknown -> Safe; no other status move; at most one leaf trade in
  `compress_fragment`.
* system (eight Fluent Bit engine files, none spelling a bit count): none.
* slice: none. Corpus and twelve-home (whole librdkafka): `log2_floor`
  Unknown -> Safe where the route holds `snappy.c`; nothing else.
* No Safe -> Unknown; no leak, double release or Unsafe added or removed;
  no Unknown -> Safe beyond `log2_floor`.

```measure-prediction
route corpus
move librdkafka::snappy.c::log2_floor Unknown -> Safe
route twelve-home
move librdkafka::snappy.c::log2_floor Unknown -> Safe
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have (`build/cycle19`); the card records the refusal and measures
the opt-in current and system routes from `git archive` trees of base and
tip, each with its own release binary.
