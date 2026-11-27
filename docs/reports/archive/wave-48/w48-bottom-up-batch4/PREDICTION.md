# Prediction — frozen before semantic implementation

Base `2d8f6d34257db14031a2578ce14f941bee25fc19`. WIP checkpoint
`35aaecd3e455c332c59913b15f1dad43dd60cfe5` changes report text only. Exact-base
analyzer release sources were built and preserved immutably before changes;
build HEAD is the documentation checkpoint. The synthetic baseline readiness
fixture passes 1/0 with the expected Unknown branch refusal.

## Priority and readiness

Batch3's fresh ranking puts logger first (score3), scheduler second (score2),
then score1 rows. Actual pinned source `66910c10` confirms logger allocates an
op/log string, dispatches conf->log_cb or calls rd_kafka_q_enq. Its dependency
rd_kafka_q_enq1 recursively forwards, either enqueues or destroys/replies, and
stores serving callbacks. Neutral recursion cannot prove this effectful cycle.
No neutral callback, consumed-op contract or acyclic graph premise is added.
Scheduler destroys TLS-retained libco_params and unlinks a scheduler member;
mk_thread_destroy dispatches cb_destroy and co_delete before freeing its frame.
These ownership/callback dependencies require independent checked closure or a
protected premise decision. The next cJSON score1 rows require richer Removed
return transport and lookup membership; they are not this card's boundary.

The next ready score1 row is cio_file_is_up: Linux macros expand to
`(cf->fd != -1) && (cf->map != NULL)`. Each leaf already has a checked reader:
scalar field comparison and pointer-member null read. The whole condition
refuses because the call-free desugaring requires a newer typed-path or
elaborated leaf; a plan consisting entirely of older admitted typed reads
is thrown away. The baseline synthetic exact-shaped condition confirms this.
This is a composition gap, not a missing resource-effect declaration.

## Bounded implementation

Retain a refused call-free condition's existing fully classified desugared
plan when it carries at least one `IndirectScalarAccess` instruction. Reuse
all existing leaf admission, alias/liveness and per-hop checks and the existing
short-circuit/loop CFG wiring. No new leaf syntax, pointer value, scalar value,
null premise, call effect or return transport is introduced. All old admissions
keep their route. A failed leaf keeps the whole refusal; descriptor, switch,
write, volatile/atomic and unknown-call guards remain. Bind the admission truth
table and checked-read event trace to matching audited Lean. Faithful C
extraction and Rust/model correspondence remain boundary28 premises.

Expected corpus baseline: 967 Safe / 3,858 Unknown / 0 Pending / 9 provisional
Unsafe, 4,834 rows. Expected targeted gain: cio_file_is_up and cio_chunk_is_up
under an explicit selected-program cio_file_is_up home added to the report's
copy of the existing link map. Predict 0–2 targeted Safe gains; incidental
call-free compound predicates may remove refusals or expose real alias/lifetime
obligations. Attribute every moved row; preserve every prior Safe/Unsafe unless
a newly exposed defect is explained. No new validated upstream defect claimed.
System/slice statuses and null/runtime verdicts expected unchanged, except any
incidental exact applicable composition and their explicit custody propagation.
All projection differences require separate denominator accounting.

Close with full standing gate, paired fresh corpus/system/slice attribution,
null/runtime checks, negative controls, refreshed ranking and own records. No
provider contract edit is planned; conditional provider gate applies if that
changes. Freeze prior reports/source pins, keep broader residual plan open,
record shared-ledger/increment updates for serial integration.
