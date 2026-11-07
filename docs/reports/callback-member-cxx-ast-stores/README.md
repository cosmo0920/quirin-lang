# CALLBACK-MEMBER-CXX-AST-STORES

Records only: **no analyzer, gateway, contract, Lean or test change.** The
card was launched to implement option (b) of the follow-up that
CALLBACK-MEMBER-NON-C-UNITS registered (a clang++ AST reading of a scoped C++
unit's stores). Four user decisions taken while it ran postponed the reading
and set the verdict policy for C++ code. This report keeps the worked-out
design, says why there is no target now, and records the decisions. The base
is `b5fc23151`, which is origin/main `532e03c97` merged with
CALLBACK-MEMBER-NON-C-UNITS's tip `4e61b815b` (not landed). Gateway rule 12n
and C04.19.3 are unchanged, and no rule id is assigned.

## User decisions (2026-11-07, verbatim)

1. On the launch of round 26's third card (option (b) was the
   recommendation): "Just push and launch 3". That chose option (b).
2. On ripser: "Also, we just auditing for the ripser's C ABI bridge we should
   postpone to fill the gap of CXX callbacks of risper." Only ripser's C ABI
   bridge is audited. That is the `extern "C"` block of
   `include/fluent-bit/ripser/flb_ripser_wrapper.h`
   (`flb_ripser_compute_betti_from_dense_distance`,
   `flb_ripser_compute_intervals_from_dense_distance` and the callback type
   `flb_ripser_interval_cb`) and its definitions in
   `src/ripser/flb_ripser_wrapper.cpp`, as seen by
   `plugins/processor_tda/tda.c`. ripser's C++ internals are postponed:
   `lib/ripser-1.2.1/ripser.cpp`, `ripser_internal.hpp`, and the wrapper's
   static C++ callbacks (`betti_interval_cb_impl`,
   `interval_recorder_cb_bridge`, `generic_interval_cb_bridge`).
3. On simdutf: "simdutf should be treated as same manner." Only simdutf's C
   ABI bridge is audited. That is the `extern "C"` block of
   `include/fluent-bit/simdutf/flb_simdutf_connector.h` (the
   `flb_simdutf_connector_*` entry points, with no callback type), as seen by
   `src/flb_unicode.c`. The C++ side of
   `src/simdutf/flb_simdutf_connector.cpp` is postponed, including its AST
   reading for `cmt.log_cb`. That unit was option (b)'s only target, so the
   card stops before any semantic change.
4. On the verdict of C++ code, the user first said: "Oh, it means unsafe for
   now because C++'s lifetime is somewhat ambiguous for humans and computers
   so they should be in Unsafe by CXX conditions." The integrator pointed out
   that hard invariant 2 allows `Unsafe` only for a validated reachable
   violation. It offered three choices: Unknown with a named reason
   (recommended), Pending for human review, or Unsafe by policy, which would
   need an invariant amendment. The user chose **"Unknown, named
   (Recommended)"**.
5. Completing that policy: "Once modeled them in the far future, they will be
   treated as Safe or Unsafe by their audited conditions."

**The resulting policy.** A row whose verdict depends on C++ lifetime is
`Unknown`, with the named reason class `cxx-lifetime-unmodeled`. The name is
free: it appears nowhere in the tree at the base. The class covers ripser's
and simdutf's C++ internals and the bodies behind their C ABI bridges. It is
counted and reported as its own class, stays in the denominator, and is
never `Safe`.

The class is **interim**. Once C++ lifetime is modelled (through
RIPSER-CXX-CALLBACKS, SIMDUTF-CXX-CALLBACKS or a successor), each row leaves
the class and gets the verdict its audit proves:

- `Safe` only after every obligation discharges;
- `Unsafe` only for a validated reachable violation;
- `Unknown` otherwise.

The class is never a permanent verdict. It is implemented by the registered
item CXX-LIFETIME-UNMODELED, not by this card.

## The run's non-C units, and why there is no target now

This uses CALLBACK-MEMBER-NON-C-UNITS's census of the 19 non-C units of the
user's `current` run's compile database at Fluent Bit `046accf6`
([its census.txt](../callback-member-non-c-units/census.txt), read at this
base). Nothing was re-run.

| Unit | What it is | Final pre-pass tokens it names | Under the parent's rule 12n | Needs option (b)? |
|---|---|---|---|---|
| `lib/ripser-1.2.1/ripser.cpp` | ripser C++ internal | `event` (libstdc++'s `std::ios_base::event`, an S3 lifetime token only) | opens no callback member | no: postponed, RIPSER-CXX-CALLBACKS |
| `src/ripser/flb_ripser_wrapper.cpp` | ripser C ABI bridge: definitions plus the static C++ callbacks | none | skipped | no: postponed (bridge body is C++) |
| `src/simdutf/flb_simdutf_connector.cpp` | simdutf C ABI bridge: definitions, with a C++ body | `cmt`, `event`, `evl` | opens `cmt.log_cb` only | was the only target: postponed, SIMDUTF-CXX-CALLBACKS |
| `lib/simdutf-amalgamation-5.5.0/src/simdutf/simdutf.cpp` | simdutf C++ internal | none | skipped | no: postponed |
| `lib/librdkafka-2.15.0/src-cpp/*.cpp` (12) | librdkafka's C++ wrapper `rdkafka++`, neither ripser nor simdutf | rdkafka tokens | outside the program (`not-linked`, re-verified each run) | no, while it stays unlinked |
| `huf_decompress_amd64.S`, `lj_vm.S`, `invokeNative_em64.s` | assembly, not C++ | none | skipped | no: MEMBER-PREPASS-ASSEMBLY-UNITS |

The only unit whose verdict option (b) would have changed is
`flb_simdutf_connector.cpp` (`cmt.log_cb`). Decision 3 postpones it. No
other non-C unit of the run needs the reading today. The rdkafka++ units
would need it only if a build linked the wrapper. The scope file's
`not-linked` evidence is re-checked every run, and a dropped exclusion is
read like any other unit, so under the parent's rule a C++ unit naming
rdkafka tokens would then open every member it names. Fail-closed holds
without the reading.

**What stays as it is.** `cmt.log_cb` stays open on the connector, as
CALLBACK-MEMBER-NON-C-UNITS leaves it. The 43 `cmt_log_print` rows
(`cmt_counter_create`, `cmt_gauge_create`, `cmt_untyped_create`,
`cmt_summary_create`, `cmt_histogram_create`, `cmt_exp_histogram_create`,
`filter_context_label_key_value`) keep `callback-member-open` with that
reason. Each row also holds unresolved identities and uncontracted calls.
No row, unit or count moves, and there is no prediction to measure.

**The bridges' C callers are in no measured route.** The current run's 186
configurations, the corpus route's 185, the system route's 15 and the
slice route's 16 (reports at `baseline:7b726587b`, units listed by file)
contain neither
`plugins/processor_tda/tda.c` nor `src/flb_unicode.c`. No diagnostic in them
names a `flb_ripser_*` or `flb_simdutf_connector_*` function. So no row
today rests on a bridge. Auditing the bridges is a new selection plus
contracts, registered as CXX-C-ABI-BRIDGES.

## Which existing refusals already give Unknown on C++ code

C++ code reaches rows today in two places. In both, an existing refusal or
standing already keeps the row non-Safe. The named class therefore starts as
a **re-attribution**:

- **Rule 12n, `callback-member-open`**: "N unit(s) of the run may name it
  but were not read (first: `<unit>`: a unit that is not C (`cpp`)…)". At
  `046accf6`, after the parent card, this covers the connector's opening of
  `cmt.log_cb` (the 43 rows above).
- **The S3 lifetime standing** (`loop_lifetime_summary::standing`). Every
  unread unit adds the reason "unread `<unit>`: a unit that is not C" to the
  declared assumption `loop-protocol-unchecked:flb_input_collector.event`.
  At `046accf6` those units are `ripser.cpp` and the connector. The standing
  already carries many C reasons as well. Whether any `Safe` or `Pending`
  row rests on a standing whose C++ reasons are its only unread-unit reasons
  is for the implementing card to count. Moving such a row to `Unknown`
  would be new semantics, a demotion only, and would need Lean parity.

A row the class covers keeps its current verdict when that verdict is
already `Unknown`. The class only names it and counts it.

## Option (b) as worked out (kept, not implemented)

Option (b) as the user chose it: read the scoped C++ unit's AST, and open a
member when any expression, statement, method, constructor or default member
initializer names a graph token or a typedef that aliases one. Writes
through `void *` are read as the C reading reads them. This design is kept
so that RIPSER-CXX-CALLBACKS, SIMDUTF-CXX-CALLBACKS or a successor can start
from it.

- **Where it applies.** It applies only to a unit the parent's rule 12n scopes
  (`member_prepass::scoped_cxx` is `Ok(true)`: a C++ extension, a whole-text
  `-E`, no precompiled-header or module flag, no module line) whose scoped
  tokens meet a member's graph tokens. That is exactly the case where the
  parent opens the member. Every other unit keeps the parent's reading.
- **How the AST is read.** clang++ runs under the unit's own recorded command
  (`member_prepass::own_arguments`, `-c`/`-o`/dependency flags dropped) with
  `-fsyntax-only -Xclang -ast-dump=json`. The output goes through the
  gateway's existing JSON AST deserializer (`deep_unit_ast::from_slice`, as
  `callback_member_closure::read_unit` uses it), with no new textual scanner.
  The `-Xclang` is the reader's own flag. The guard refuses one only in the
  recorded command.
- **What "code names a token" means.** The reader walks every function body
  (`FunctionDecl`, `CXXMethodDecl`, `CXXConstructorDecl`,
  `CXXDestructorDecl`, `CXXConversionDecl`, a lambda's `CXXRecordDecl` call
  operator), every `CXXCtorInitializer`, every `FieldDecl` with an in-class
  initializer (NSDMI), every namespace-scope or static-member `VarDecl`
  initializer, and every `ParmVarDecl` default argument. Within those nodes it
  reads every node's `type.qualType` and `type.desugaredQualType` (so `auto`,
  `decltype` and aliases desugar), `referencedDecl`, `referencedMemberDecl`,
  `ctorType`, `argType` and `typeArg` (casts, `sizeof`, `offsetof`, `new`),
  and the class of a `CXXConstructExpr` or `CXXTemporaryObjectExpr`. Words
  are matched on identifier boundaries. A pure declaration (`CXXRecordDecl`,
  a field without an initializer, a bodiless `FunctionDecl`, a
  `ParmVarDecl` without a default) names nothing. At `046accf6` the
  connector names `cmt` only in such declarations: 2 `CXXRecordDecl`, 2
  `FieldDecl`, 5 `FunctionDecl` and 4 `ParmVarDecl` lines, per the parent's
  census.
- **Aliases.** A `TypedefDecl`, `TypeAliasDecl` or `UsingDecl` whose type
  names a graph token adds its own name as a token, iterated to a fixed
  point. A store through the alias then opens the member even where Clang
  prints only the sugared name.
- **Templates.** A dependent body (`CXXDependentScopeMemberExpr`,
  `UnresolvedLookupExpr`, `UnresolvedMemberExpr`) names nothing by itself.
  Every implicit or explicit instantiation that Clang's JSON emits under its
  `FunctionTemplateDecl` or `ClassTemplateDecl` is walked like a body, so an
  instantiation over the record names it. If an instantiation's body is
  absent (an `extern template` with no definition visible), that body lives
  in another unit, which the run reads on its own account.
- **Reference-bound and pointer-to-member stores.** `T &r = *p; r.f = g;`
  types `r`'s `DeclRefExpr` as the record. `&TAG::f` types as
  `R (*TAG::*)(…)`, and a store through `.*` or `->*` types its operands with
  the record. Both name the token.
- **Undecided ASTs open every member.** These are: a non-zero exit, JSON that
  does not parse, a root that is not a `TranslationUnitDecl`, an AST that
  contains errors (`RecoveryExpr`, `containsErrors`), and the parent's
  undecided cases (a failed preprocessing, a precompiled-header or module
  flag, a module declaration or import).
- **Writes through `void *`.** These are read exactly as the C reading reads
  them. A `COPIERS` call (`memcpy`, `memmove`, `bcopy` and the builtins)
  opens a record only when its destination, after `strip` (parens and
  implicit casts), has the record's type. An untyped write is no store. The
  premise is **P-CG22-MEMBER-CLOSED-WORLD**'s exclusion as C04.19.3 states
  it: "every unit that can store into the member is in the run (a unit
  outside it, or a type-punned store, is not seen)". A write through a
  `void *` or `char *` that does not spell the record is such a type-punned
  store. The connector's `memcpy(temp_buffer.get(), input, len)` has a
  `char16_t *` destination, so it is none. Under (b), any code naming the
  record still opens the member, which is stronger than the C reading's
  typed copy test. Option (c), not chosen, would have opened every member
  on every cast or copied `void *`/`char *` write.
- **The intended Lean.** `QCore.MemberNonCUnits` would gain a
  `Text.ast (codeNames : String → Bool)` reading that opens when code names a
  graph token or an alias of one. Its premise would be a hypothesis, not an
  axiom: a unit holding a store names a graph token in a code node, because
  Clang types every stored lvalue with its record. That premise would cover
  the four forms above. `the_closed_run_set_covers_every_store` would be
  re-proved over `undecided`, `whole` and `ast` readings, and the vectors
  would be bound to the Rust table.
- **The intended fixtures.** These are current-route fixtures beside
  `tests/callback_member_non_c_units.rs`:
  - a C++ unit declaring the record with no code naming it (closes);
  - typedef-aliased, `using`-aliased, template-instantiated, reference-bound,
    pointer-to-member and NSDMI stores (each open);
  - a constructor-initializer store (open);
  - an AST with errors (open);
  - the parent's preprocessing, PCH and module cases (open).
- **The prediction it would have carried.** On the `current` run,
  `cmt.log_cb` would close over the connector's account. The 43 rows would
  stay `Unknown` on their unresolved identities and uncontracted calls, with
  no status move. Corpus, system and slice would not move: their 34
  preprocessing failures keep every member open.

Under decision 4, if the reading is ever built, a member it closes over a C++
unit still leaves every row that depends on that unit's C++ lifetime in
`cxx-lifetime-unmodeled` until that lifetime is modelled.

## Records

- [integration-amendments.json](integration-amendments.json):
  - amends CALLBACK-MEMBER-CXX-AST-STORES to postponed (left open, not
    discharged);
  - registers RIPSER-CXX-CALLBACKS and SIMDUTF-CXX-CALLBACKS (postponed,
    each quoting its decision);
  - registers CXX-C-ABI-BRIDGES (auditing the two `extern "C"` bridges,
    which nothing tracked);
  - registers CXX-LIFETIME-UNMODELED (the named class).

  The ids were checked free over the whole tree and the acceptance matrix
  at this base. The amend applies after CALLBACK-MEMBER-NON-C-UNITS's own
  amendments, which register the entry it amends.
- No PREDICTION.md: nothing is measured.
