# W48 bottom-up batch 2

Base `1543259f0e32c8a003a9062e7de7f59a42dd4519`; prediction committed before
implementation at `0dc191d3f`. Work in progress; final measured evidence follows
only after the arbiter and standing gate complete.

The bounded body rule admits a complete typed self-step of a scalar-pointer
formal only when every reference to its exact Clang declaration is a scalar
dereference read or a self-step whose pointer result is discarded, and its own contract fixes neither entry
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

The model is `QCore.IndexTraversal.formalStepEligible` and `checkedFormalStep`:
non-scalar, fixed and exposed formals refuse; a checked step preserves the
custody component and caller slots. The Rust gate is bound to all eight Lean
boolean vectors. Extraction of Clang references and Rust/Lean correspondence
remain explicit review premises. The full-body frame is checked before this
exception is considered; unsupported references cannot be hidden by a local
step admission. Existing path, return, own-contract and W4 checks still decide
the function. No new syntax, contract declaration or diagnostic category is
introduced.

Focused validation admits the exact `gf2_matrix_times` body and its caller,
keeps calls/copies/address/storage refused, and keeps explicit Owned and Out
formals fixed. The amended deepest-admitted AST test walks an unused formal
through the entire depth-255 `else if` and depth-253 conditional trees, measuring
236 KiB in each case against the existing 1 MiB budget; the next deeper trees
still refuse. These are model and fixture checks, not corpus coverage claims.

The same-unit no-extra-body candidates at the merged base remain four admin
wrappers. The current cross-unit ceiling includes those four and five legacy
LZ4 wrappers, rather than the dated predecessor's twelve: the merged base has
already discharged other homes. The three stateful/continuation LZ4 wrappers
may expose retention obligations after their nested call becomes available.
The no-extra-body and all-body plans assume hypothetical admissions and are
never counted as Safe outcomes.
