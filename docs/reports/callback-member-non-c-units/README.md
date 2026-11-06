# CALLBACK-MEMBER-NON-C-UNITS

Round 26 B (user request 2026-11-06: "we needed to cover more areas to fill
the gaps of verifying as Safe or Provisional Unsafe units"; queued with "Queue
all six"). Base `7b726587b`; prediction `83e0a3f07`, change `0807a69b8`.
Gateway rule 12n amended in place (no new rule id); contract specification
C04.19.3 likewise.

## The two units

The user's `quirin-audit current` run at Fluent Bit `046accf6` refused 205
rows `callback-member-open` only because "2 unit(s) of the run may name it
but were not read (first: lib/ripser-1.2.1/ripser.cpp: a unit that is not C
(`cpp`))". The member pre-pass (rule 12n) preprocesses every unit of the
compile database with its own command and compares its identifiers with the
**union** of every invoked member's closure-graph tokens and the S3 lifetime
protocol's tokens; a non-C unit naming any of them was unread and opened
**every** member. [`census.txt`](census.txt) (from
[`scripts/non_c_census.py`](scripts/non_c_census.py), read-only, over the
run's compile database and sidecar) lists the database's 19 non-C units:

| Unit | Kind | Final tokens it names | Before | After |
|---|---|---|---|---|
| `lib/ripser-1.2.1/ripser.cpp` | C++ | `event` (libstdc++'s `std::ios_base::event`) | unread, opens every member | opens no callback member |
| `src/simdutf/flb_simdutf_connector.cpp` | C++ | `cmt`, `event`, `evl` (via `fluent-bit/flb_log.h` -> `cmetrics/cmetrics.h`, which defines `struct cmt`; `struct mk_event_loop *evl` fields) | unread, opens every member | opens `cmt.log_cb` only |
| `lib/librdkafka-2.15.0/src-cpp/*.cpp` (12) | C++ | rdkafka tokens | outside the program (scope file, `not-linked`) | unchanged |
| `lib/simdutf-amalgamation-5.5.0/.../simdutf.cpp`, `src/ripser/flb_ripser_wrapper.cpp` | C++ | none | skipped | unchanged |
| `huf_decompress_amd64.S`, `lj_vm.S`, `invokeNative_em64.s` | assembly | none (the `.s` text is empty under `-E`) | skipped | unchanged (follow-up below) |

`event`, `evl` and `flb_input_collector` are the S3 lifetime protocol's
tokens (`quant registered #coll: flb_input_collector.event |->
flb_input_collector.evl;`, `input_collector_registration.qv`), in the
union only because the pre-pass also reads lifetime facts; `cmt` is
`cmt.log_cb`'s record tag. The closure counted both units because the
unread map was run-global: one name of any token opened every member.

## The reading

Chosen: **a scoped C++ unit opens only the members whose own closure-graph
tokens it names.** It is the reading by which the pre-pass already skips a
C unit naming no token (`QCore.AppRkCallbacks.skipping_a_silent_unit_keeps_the_run_set`,
user decision F1): a store the closure reads (a member access, an
initializer, a whole-object or opaque write of a record that carries the
member, a call of a setter or formal's function, an address taken) needs
the record's complete definition, which spells its tag or a typedef of it,
or the function's name; C++ adds no way to write a member of a record whose
definition the unit does not hold. The per-member graph tokens are the ones
the closure computes (record tags, containers and union-held records that
carry a function pointer, setters and formals' functions), so the layout
and alias evidence of rules 12n's keys is the same as for skipping. It
applies only where the text is the whole unit
(`member_prepass::scoped_cxx`): a C++ extension, no `-include`, `-Xclang`,
precompiled-header or module flag in the command, no module declaration,
import or `#pragma clang module` in the text. Everything else keeps
opening every member: a failed preprocessing (macro-built names,
generated headers), an assembly unit (it writes through offsets and
symbols no record names), a C unit whose AST is not read, a C++ unit whose
text is not the whole unit. A C++ unit that declares the record (an
`extern "C"` redeclaration included) keeps the member open whether or not
it stores, and a unit is never scoped against a token it was not checked
for. The lifetime standing (S3) still reads every unread unit as a declared
assumption; the fold stays demote-only and nothing is promoted.

Not chosen, and why:

- *Exclude the units* (scope file or denominator): not a reading; refused
  by the brief.
- *A clang++ AST read of a unit that declares the record*: would decide
  `flb_simdutf_connector.cpp` for `cmt.log_cb` (its Clang AST, under its
  own command, names `cmt` only in 2 `CXXRecordDecl`, 2 `FieldDecl`, 5
  `FunctionDecl` and 4 `ParmVarDecl` lines; no expression or statement
  names it). Fail-closed does not settle it: the unit's code (libstdc++
  inlines, `memcpy` into `temp_buffer`) writes through `void *`/`char *`
  that the C reading does not follow either, and the brief lists opaque
  casts through `void *` as undecided. Registered as a follow-up with
  options; it moves no status today (below).

Rule text: [rule 12n](../../agents/rules/12n.md) ("An unread C++ unit is
scoped to the members it names"). Code:
`crates/qcore-c-gateway/src/member_run_facts.rs` (`RunMemberFacts::scoped`,
`checked`, `with_scoped`, `opens`, `opening`; the closure counts only the
opening units), `crates/qcore-audit/src/member_prepass.rs` (`scoped_cxx`,
`module_line`, the scoped tokens, the section's `non_c_units` with each
scoped unit's tokens and the units opening each member; the facts digest
covers `scoped` and `checked` when present, so a run without scoped units
keeps its digest).

## Evidence

- Fixtures `crates/qcore-audit/tests/callback_member_non_c_units.rs`
  (current route, real pre-pass, two invoked members): a C++ unit naming
  only `other_s` closes `handle_s.conf.err_cb` (its caller `Safe`) and keeps
  `other_s.cb` open; a C++ unit that declares `handle_s` and stores into the
  member, and one that only declares it, keep it open; a C++ unit whose
  preprocessing fails, an assembly unit naming `other_s`, a C++ unit under
  `-include` and one importing a module open every member. Gateway unit
  tests `member_run_facts::tests` (`scoped_vectors_hold`,
  `a_scoped_unit_opens_only_the_members_it_names`).
- No command of the 046accf6 production (1,419) and Monkey (142) compile
  databases uses a flag the whole-text guard names (`-include*`, `-Xclang`,
  `-fmodule*`, `-fcxx-modules`, `-fprebuilt-module-path`,
  `-fimplicit-module*`, `-emit-pch`, `--precompile`): scanned once with
  `shlex` over both databases, *measured once, harness not shipped*.
- One mutation, *measured once, harness not shipped*: with the scoped map
  emptied (`with_scoped(tokens.clone(), BTreeMap::new())`) the two closing
  fixtures fail and the three refusing ones pass; the source was restored
  by rewriting its bytes (SHA-256 `3a06f118…` before and after).
- Lean `QCore.MemberNonCUnits` (audited by `proof/AuditMemberNonCUnits.lean`,
  9 declarations): `an_undecided_unit_keeps_the_member_open`,
  `a_named_graph_token_keeps_the_member_open`,
  `an_unchecked_graph_token_keeps_the_member_open`,
  `a_scoped_unit_naming_no_graph_token_holds_no_store`,
  `a_scoped_unit_naming_no_graph_token_keeps_the_run_set`,
  `a_closed_run_read_every_unit`, **`the_closed_run_set_covers_every_store`**
  (with the read units' readings exact and the reading's premise as a
  hypothesis, a closed run set contains every designator any unit, read or
  unread, stores), `the_recorded_reading_is_the_checked_reading`,
  `scoped_vectors_hold` (bound to `SCOPED_VECTORS`).

## Prediction and measurement

[PREDICTION.md](PREDICTION.md), committed before the change, against
`baseline:c8c7edf2165a2065c234f3a63b4eed23e937ced6`: no row moves on the
corpus, system or slice routes, whose pre-passes also leave 34 units unread
for failed preprocessing (sources the prepared builds never generated), so
every member stays open there (leaf text "36 unit(s)" becomes "34
unit(s)"). On the user's `current` run: the 10 `rd_kafka_s.rk_conf` members,
`rk_share_consumer.share_acknowledgement_commit_cb` and
`consume_ctx.consume_cb` close empty and `rd_kafka_op_s.rko_serve` closes
over its four serve functions; `cmt.log_cb` stays open on the connector,
`cio_options.log_cb` and the `rko_op_cb` family on their own C reasons. The
205 rows keep refusing with those reasons, the 8 that named a member that
closes now on `rko_op_cb`; no function, unit or uncontracted-callee change.
The integrator runs:

```text
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=baseline:c8c7edf2165a2065c234f3a63b4eed23e937ced6 --side tip=rev:0807a69b8 --declared docs/reports/callback-member-non-c-units/PREDICTION.md
```

and reruns the user's `current` command (`resource-diagnostics.invocation.json`
of the 046accf6 report), whose `callback_members.prepass.non_c_units`
section names each member's opening units.

## Follow-ups

- `CALLBACK-MEMBER-CXX-AST-STORES` (registered): `cmt.log_cb` over a C++
  unit that declares `struct cmt` but has no expression naming it; options
  and a recommendation in the entry.
- `MEMBER-PREPASS-ASSEMBLY-UNITS` (registered): an assembly unit naming no
  token is skipped although the skipping premise is C's.
- `MEMBER-PREPASS-NOT-TEXTUAL-SKIP` (registered): the skip of a unit naming
  no token does not apply this card's whole-text guard.
- `S3-PREPASS-UNREAD-GRAPH-ISOLATION` (amended): the two C++ units are now
  isolated per member; the 34 preprocessing failures and the lifetime
  standing remain.
