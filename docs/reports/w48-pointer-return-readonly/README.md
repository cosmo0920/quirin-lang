# W48 pointer return through a read-only call actual

Card W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT, base `fa04aab0f`.
The committed prediction, implementation, and measured gate belong to this
report. The before corpus must be rerun on this base before a movement claim.

## Source shape under review

The pinned librdkafka `rdkafka_admin.c` has six
`rd_kafka_admin_result_ret_*` helpers that pass
`&rko->rko_u.admin_result.results` to `rd_list_cnt` and then return the
list's `rl_elems` pointer. Four other accessors use the same list-count and
pointer-return shape. The pinned `rdlist.h` defines the static inline
`rd_list_cnt(const rd_list_t *rl)` with the sole statement
`return rl->rl_cnt;`. This body reads its actual and does not write through
it, retain it, or return a pointer.

The current gateway's `position_pointer_root` refuses `rko` because the
lexical and AST `may_bind` readers conservatively mark names in call actuals.
The exception must be tied to a checked read-only body and an exact actual,
must keep every other binding cause, and must leave the normal call's own
verdict obligations in force. A `Borrowed` custody mode by itself does not
establish a read-only frame.
