# OUT-PARAMETER-BODY-SUMMARY prediction

Committed before the semantic change, against base `532e03c97` (origin/main).
The measurement's base side is `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`;
`532e03c97` differs from it only in gate tooling and pinned-route checkpoints.

The change has two parts (see the report):

1. **Scalar stores named for an out slot** (rule 10k(5)). `*B = V` lowered as
   an out-slot `Alias`/`Clear` after every lexical route declined is read by
   the W46 AST statement reader, and only its scalar-store lowering replaces
   the store. The gap it closes is a pointee whose type is a typedef-defined
   enumeration (`rd_kafka_resp_err_t *errp; *errp = RD_KAFKA_RESP_ERR__X;`):
   the lexical reader keeps that typedef `Opaque`, while the same function's
   `*errp = rkt->rkt_err` already reached the AST reader through the indirect
   access refusal and was admitted by its enum arm.
2. **Unwritten return-code classes** (rule 11r(6), 12s(2)). Rule 11r(4)'s
   per-value body may now read a slot left unwritten on every exit of its
   class as that class's `resource_borrowed` beside `resource_out` in another
   class, validated with the slot sanctioned and `Unwritten` required at every
   exit.

Corpus rows carrying `store through … without an out-parameter contract` at
the baseline: 126 functions, 235 leaves, 7 sole-blocked
([`census/baseline-rows.json`](census/baseline-rows.json)). A row can change
status only when every other leaf already discharges, so only the seven can
become `Safe`:

* `rd_kafka_message_timestamp` (`*tstype = RD_KAFKA_TIMESTAMP_NOT_AVAILABLE`)
  and `rd_kafka_toppar_get_avail` (four `*errp = RD_KAFKA_RESP_ERR__…`) are
  part 1's shape and are predicted `Unknown -> Safe`.
* `cio_utils_read_file`, `cio_file_content_copy` and `cio_memfs_content_copy`
  are part 2's shape (`int`, literal returns, fresh on `0`, unwritten on
  `-1`) but each also stores a `size_t` through a second formal **after** the
  pointer slot (`*buf = data; *size = st.st_size;`). That scalar store's frame
  may write the first slot's object unless the two formals are known not to
  alias, so the slot is `Ambiguous` at the success exit and the class does not
  validate. They stay `Unknown`, as the brief's "aliasing between out formals
  keeps the site refused" requires (see the report's stop).
* `mk_vhost_get` stores borrowed list elements and
  `rd_kafka_consumer_group_metadata_write` a possibly-null `rd_malloc` result;
  neither is in scope. Both stay `Unknown`.

None of the seven has a caller in its own unit, and none is named by the
eight-home link map, so no caller moves. No row is predicted to become
`Unsafe`: the non-sole rows keep at least one other refusal; that some
part-1 row exposes a validated defect a removed refusal had hidden is the
residual risk the measurement checks.

```measure-prediction
route corpus
move rd_kafka_message_timestamp Unknown -> Safe
move rd_kafka_toppar_get_avail Unknown -> Safe
delta Safe +2
route system
none
route slice
none
```

System (10 rows, 16 leaves) and slice (5 rows, 10 leaves) have no
sole-blocked row; leaves may trade there, no status moves.

**The user's `current` run** (Fluent Bit 046accf6): 130 functions carry the
leaf (243 leaves over every configuration), the same seven sole-blocked.
Predicted: `rd_kafka_message_timestamp` and `rd_kafka_toppar_get_avail` become
`Safe` in every configuration that compiles them; their units' statuses do
not change (other rows stay `Unknown`). No callee leaves the uncontracted
list and no function leaves the "only uncontracted" set, because neither
function is called within its own unit and rule 4c is off on `current`.
