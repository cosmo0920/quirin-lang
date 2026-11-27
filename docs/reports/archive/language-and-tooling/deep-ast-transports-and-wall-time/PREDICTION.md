# DEEP-AST-TRANSPORTS-AND-WALL-TIME: prediction (frozen before implementation code)

Base `5214a3ec3`. Written after the BEFORE measurement and the profile, before
any analysis, transport or cache code. The only code committed before this
file is the measurement-only `unit-timing` feature of `qcore-audit` (compiled
out without the feature) and the scripts.

## BEFORE, as measured

* Binary `private/dat/bin/before` `3dcd3b2e…`, clean tree at the base. Corpus
  run `before1`: 881 / 9 / 3,944, report `ab8f5f14…`, verdict rows
  `321a8913…` (the brief's digest), 550.3 s; system 65.6 s; slice 29.5 s.
* `unit-timing` build of the same tree (`before-timing2`, `1af0f0a7…`), sampled
  run `bprof1` (report `ab8f5f14…`, unchanged by the feature): 185 unit
  analyses, **one phase** (no phase-two re-analysis on the corpus).
  `rdhttp.c` is analysed for **132.5 s**, dispatched at 412 s (its source file
  is small, so the heaviest-first dispatch by source size starts it last) and
  ending the run at 544.5 s: the whole tail of the arbiter is that one unit.
* Where `rdhttp.c`'s 265 own thread-samples go (`scripts/profile_unit.py`):
  * Clang wait 1 %, the rule-13o reparse 1 %, the depth scan 0 %: **the parse
    of the 750 MB document is not the cost**, and it is parsed once (one
    launch per unit on the corpus, the per-thread reuse answers the other
    scopes);
  * `call_exits::resolve` under the parity-only `call_exit_census` **33 %**: a
    whole-function walk per uncontracted call statement, so the four deep
    definitions (908 calls of the type-check macro's warning functions) walk
    their own macro-expanded bodies once per call;
  * `Parser::elaboration_node` **44 %**: per statement hook a walk from the
    function root that computes each node's begin and end line
    (`typed_access_path::node_line`, 18 % self) and tokenizes the source span
    of every candidate node (`tokenize`, 9 %): the type-check macro's
    expansion puts its whole subtree on one line, so every node of it is a
    candidate, and the same span is tokenized again for each node and each
    statement;
  * the rest (literal premises, the CFG build, other readers) about 20 %.
* Stack: every analysis worker has a 2,048 KiB stack; the greatest stack depth
  any worker reached over the whole corpus is **144 KiB** (`rdhttp.c`'s
  readers of its 224-level index included; pagemap high-water, release).

## Task B prediction (identity required)

Levers: (1) `call_exits::resolve` answered from one per-function table built
by one walk (every callee's references, the body's object names and its
`noreturn` declarations), held with the function's index entry; (2)
`elaboration_node`'s walk reads each node's begin and end line from a
per-function line tree built once (the same child order, so the same
traversal), and each source span is tokenized once per function. Both caches
live in the index of one launch identity (the document), are pure functions of
it, and are dropped with it.

* Every route: report, sidecars, verdict rows (`321a8913…`, system and slice
  digests) and parity multisets **byte-identical** to BEFORE.
* `rdhttp.c`'s own analysis: 132 s → **35–70 s**.
* Corpus wall (median of three, `--jobs 4`): 550 s → **450–500 s** (the tail
  becomes the slower of `rdhttp.c`'s shorter run and the end of
  `rdkafka_mock_handlers.c` at about 408 s).
* System and slice: within ±5 % (they hold no unit of this depth; the same
  readers get marginally cheaper).

## Task A prediction (rows may change only where a filtered AST is newly available)

Transports: the filtered launch (`typed_c_ast::ast_declarations`) for the
analysed-unit consultations (`definition_ast`, `appended_declarations`), and
the closed-world `translation_unit_ast`, read a document within rule 13o's
bound the same way (default parse first; on failure a depth scan; at most 256
levels reparsed without the limit on a 256 MiB stack). The source importers'
use of the filtered launch, the numeric property's whole-unit AST and the FFI
layout probe keep the default limit unless their readers are measured; the
two readers that already parse without a limit on their own stacks (typed
dispatch census, bounds side pass) are recorded, not changed.

* Corpus, system, slice: **no verdict-row change expected**. The 11y fallback
  is not consulted where the unit index exists (rule 13o), and no
  consultation of the retention guard or a 12u probe is expected on the four
  deep `rdhttp.c` definitions. If any row moves, it is in one of those four
  definitions and is attributed.
* Stack: the corpus worker high-water stays at most **256 KiB** of 2,048 KiB
  (release); the readers' recursion on the deepest admitted definition
  (`rd_http_ssl_configure`) is measured and fits.
* Wall time: no measurable change from Task A.
