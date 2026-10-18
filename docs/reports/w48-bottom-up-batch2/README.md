# W48 bottom-up batch 2

Base `1543259f0e32c8a003a9062e7de7f59a42dd4519`; prediction committed before
implementation at `0dc191d3f`. Work in progress; final measured evidence follows
only after the arbiter and standing gate complete.

The bounded body rule admits a complete typed self-step of a scalar-pointer
formal only when every reference to its exact Clang declaration is a scalar
dereference read or a self-step, and its own contract fixes neither entry
custody nor an out slot. Copies, pointer actuals, addresses, pointee writes,
rebinding and unresolved references retain the old refusal. The existing
`PointerStep` transfer derives an interior pointer, leaves all custody state
unchanged and refuses losing the sole name of a live object. Its ordinary
statement, callee and caller obligations remain in force. The initial target
is `crc32c.c::gf2_matrix_times` (`mat++`); bounds are a separate property.

The batch link map adds the external Safe `LZ4_compressBound` body in `lz4.c`
to W44's five homes under the same explicit actual-link/type/calling-convention/
interposition premise. Source presence does not prove that premise. Five
legacy `lz4hc.c` wrappers are the measured-base candidates this import serves;
the audit still re-derives home and caller and may expose deeper obligations.

`cJSON_DetachItemViaPointer` remains undecided: success returns exactly the
child it removed from the parent list, while NULL transfers nothing. No
fresh-result or Borrowed-only declaration is introduced. The parent has the
conditional-detach decision options; work on this independent batch proceeds.
