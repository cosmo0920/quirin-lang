# W46 contract and typed-statement expansion prediction

Integration base: `207cebe4cdb745f6216ff2b511c4b7fd134581b9`, after the first
36-contract expansion. The user requests more contracts, additional Wave 46
cards implemented by Sol 6 at medium reasoning, and a stale-worktree sweep.
Astra selects gaps and integrates. Three independent lanes are dispatched:

- W46-METRIC-AGGREGATE-CONTRACTS: six histogram/summary scalar getters, after
  reviewing atomic loads and conversions at both pinned Fluent Bit revisions.
- W46-FLUENT-ROUTING-QUERY-CONTRACTS: five missing scalar predicates/queries.
  The already-declared mask-size query is retained; invalid-index mask bit
  helpers still reach logging callbacks and are excluded.
- W46-ACCESS-PATH-STATEMENTS: P2 of the unsupported-coverage queue. P1's frozen
  prediction was 45 functions freed (38 Safe, 7 Unknown), plus three Safe with
  a hypothetical cross-unit summary. All 48 remain Unknown on this base, so
  the preceding six contract-driven gains do not overlap that list. The new
  layer keeps old routes authoritative until parity; P3 positions remain
  dependent and undispatched.

The base corpus is 411 Safe / 9 provisional Unsafe / 4,414 Unknown functions;
185 units: 5 Safe / 8 provisional Unsafe / 168 Unknown / 4 data-only. The
system slice has 28 Safe / 2 provisional Unsafe / 204 Unknown unique projected
functions plus 3 exclusions. Evidence is the preceding expansion's archived
AFTER. Its OR-1/OR-7 refutations and 10 held caller sites must remain unless a
separately justified semantic correction says otherwise.

Expected contract effect: fewer unsupported leaves, no promised whole-function
gain, no new resource model or lifecycle proof. Expected statement-layer effect:
measure against P1's frozen per-function lists, not a leaf-count safety claim.
Source pins remain premises. Every changed diagnostic and verdict, especially
new violations or Safe-to-non-Safe corrections, must be attributed. The
contract and statement effects will be measured separately before their union.
No historical report is rewritten. Every card retains actual gate failures.

The stale-worktree sweep is operational: only clean, merged, unlocked worktrees
are removed after private evidence is archived and verified. Active worktrees,
locked worktrees, untracked work and branch refs remain intact.
