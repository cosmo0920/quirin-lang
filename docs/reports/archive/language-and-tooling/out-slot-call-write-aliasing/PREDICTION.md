# OUT-SLOT-CALL-WRITE-ALIASING prediction

Committed before the semantic change, on a branch cut from
OUT-PARAMETER-SCALAR-FRAME-PREMISE's tip `7c1a474bb`. The measurement's base
side is `rev:aa3999333` (that card's measured tip); neither parent card is
landed.

The change: a call whose actual at a non-`Out` position may designate caller
memory (another formal's value, a local pointer's value, a member, a call
result, a file-scope or static object, a record) poisons every *bound*
sanctioned out slot `*X` of the analysis, as the typed scalar store's
refused-write frame already does. The slot is then `Ambiguous` at the exits
it reaches, so a W4 `Out` summary, a rule 11r(4)/(6)/(7) class or a rule
12s(2) row that needed it does not validate. Excluded: scalar, null,
string-literal and function-pointer actuals, a pointer whose pointee Clang
types `const` at the position, the address or decay of a pointer-free
automatic local or of a formal's own variable, and the slot a callee's `Out`
position itself writes.

The [census](census/) (AST, every `Safe` row of the four populations plus
every summary any row reads, and the parent cards' predicted `Safe` rows)
finds five rows that validate a pointer out slot, all in chunkio and
librdkafka, and exactly one that passes it across such a call:
`cio_file_content_copy` (`*out_buf = buf; … if (set_down) cio_chunk_down(ch);`,
`ch` another formal, `struct cio_chunk *`, declared `resource_borrowed(ch)`).
No callee contract states what `cio_chunk_down` writes, so its row loses the
class rule 11r(6)/(7) gave it and returns to the base's
`store through `*out_buf` without an out-parameter contract`:

```measure-prediction
route corpus
move cio_file_content_copy Safe -> Unknown
delta Safe -1
route system
none
route slice
none
```

`cio_utils_read_file` and `cio_memfs_content_copy` call nothing after the
slot store and stay `Safe` with the `effective-type:` premise;
`rd_slice_reader` and `rd_slice_peeker` hand their slot to
`rd_slice_reader0`'s derived out slot and call nothing after it (and no
sanctioned slot is theirs: their summaries are all `Borrowed`). No caller
moves: `cio_file_content_copy` is called only from `cio_chunk.c`, another
unit, and rule 4c is off on these routes. No row's leaves change otherwise:
outside a contract that sanctions an out slot (the probes, the validations,
and a declared `resource_out` body, of which no corpus unit defines one) the
rule has nothing to poison.

**The user's `current` run** (Fluent Bit 046accf6): `cio_file_content_copy`
`Unknown` again (it was predicted `Safe` by the parent card; at the user's
report it is `Unknown`), with the leaf it had before the parent card; no
unit status changes; no callee leaves the uncontracted list and no function
leaves the "only uncontracted" set. **No currently published `Safe` row
(baseline `7b726587b`, the user's `current` report) depends on the gap**: the
one row that would is `Safe` only on the unlanded parent branch.
