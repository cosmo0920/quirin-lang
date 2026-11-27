# W4: immutable aliases of original output slots

Baseline `04f86d5`, 2026-09-07. The generic C gateway now infers custody summaries
for output wrappers that first copy a formal pointer into a same-type local alias,
including chains and multiple output slots. For example:

```c
void fill(void **out) {
    void **first = out;
    void **alias = first;
    q_make(alias);
}
```

Under the declared `q_make` Out contract, this wrapper now validates its original
caller's slot. The preceding regression suite explicitly rejected the simple
alias form; that rejection is replaced with positive alias tests and retained
negative cases for reassignment, shadowing and escaped pointer authority.

## Identity before custody inference

A private normalizer runs before resource CFG construction. It recognizes only
initial, unconditional, single local declarations with bare name initializers.
Each copies an original formal or an earlier alias of the identical explicit
flat pointer type, with at least two pointer levels. Its type vocabulary excludes
typedefs, attributes, volatile/atomic/restrict qualifiers, arrays, casts and
side-effecting initializers. Alias names must be fresh and distinct from formals.

The normalizer inventories **every subsequent occurrence** of every recognized
alias. Only a bare (optionally parenthesized) argument in a modeled Out position,
or the target of a direct `*alias = ...` statement, is admitted. Assignments to
the alias, increments, shadowing declarations, address escape, borrowed calls,
indirect reads and branch-local alias definitions cannot disappear through this
route. Unsupported forms retain the ordinary conservative analysis.

After those checks, alias initialization becomes empty statements and permitted
uses name the original formal. Token positions/lines are retained. Original-formal
mutation and shadow guards still run on the resulting CFG. Alias normalization
creates neither a contract nor write authority: the original slot must be
sanctioned by the candidate's Out mode, freshly initialized, and discharged at
every modeled exit. Existing validation rejects repeated output writes, aliasing
caller slots, leaked overwritten owners, missing initialization and unresolved
nullable handbacks. A direct branch assigning literal null can support nullable
Out; forwarding an unresolved nullable callee output remains unsupported.

Acyclic internal wrappers consume only already-derived, revalidated contracts.
Declared contracts keep precedence and disagreement remains a SummaryConflict.
Certificates retain the **original preprocessed C source**. Independent certificate
checking repeats normalization, candidate derivation and body validation; serialized
summary labels or caller-supplied alias maps are not trusted. No public alias
constructor, schema extension or provider-specific route is added.

## Tests, native run and model proof

Six additional gateway tests cover direct aliases, chains, stores, two outputs,
acyclic composition, literal-null branches, an adversarial use table, caller leaks,
shared caller slots, declared precedence, actual C11 validation, source mutation
and forged summary rejection. The shared C fixture `out_alias_wrappers.c` passes
real preprocessing and independent certificate rederivation. Its `fill`, `maybe`
and `two` wrappers also execute in `native_driver.c`, covering non-null and null
paths. Four allocations are returned through the original slots and released
exactly once, with actual pointer and per-generation payload checks. ASan, UBSan
and LSan pass; LSan runs untraced because it cannot operate under tracing.
External allocator/free contracts remain assumptions; this native run is bounded
observation of the concrete fixture, not proof of every external implementation.

Six Lean lemmas extend the existing output-slot model with immutable alias copies.
They preserve the exact source slot and all other bindings, reject overwrite of
an existing alias, compose two copies, and retain original sanction/unwritten-slot
requirements at forwarding. Source type equality, dominating initialization,
complete use classification and no shadowing remain explicit premises. This is
not a proof of token extraction or general C pointer aliasing. Lean build and all
20 axiom audits pass without project axioms or placeholders.

```sh
cargo test -p qcore-c-gateway --lib
clang -std=c11 -Wall -Wextra -Werror -g -O1 \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  docs/reports/w4-immutable-out-aliases/native_driver.c -o /tmp/qcore-out-alias-native
ASAN_OPTIONS=detect_leaks=1 /tmp/qcore-out-alias-native
```

## Validation and remaining scope

All **791 workspace tests**, **31 provider tests**, **43 Python tests**, formatting,
Clippy, both reevaluation probes, Lean build and **20 Lean audits** pass. Three
initial Clippy findings were corrected before the final gate. Source and log
hashes, native command/executable identity and exact stdout/stderr are archived.

The full 185-unit Cycle19 source, diagnostics, chunk-contract and source-effect
artifacts remain byte-identical to baseline: **2 Safe /42 provisional Unsafe /
137 Unknown**, plus four data-only units, and **3 checked /93 Unknown** source-effect
functions. No pinned unit is promoted. CT coverage remains **10/50 mapped,
40 unmapped** and the contract corpus retains 58 modules.

General pointer aliases, non-prefix or mutable bindings, alias-selected removal,
relational branch reasoning, arbitrary recursive components and cost summaries
remain open. This increment supports immutable copies of original output slots;
it does not add general memory-access safety, lifetime inference or heap shape
analysis. The broader W4 and W2–W7 goals remain active. Existing subagents remain
stopped by their reported usage limit, so this work continued locally.
