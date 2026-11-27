# CONTRACT-PIN-TIP-MODE: prediction (frozen before implementation)

Frozen before any Rust implementation code of the card. Disclosed order: the
Lean model `proof/QCore/ContractTipMode.lean` was drafted (uncommitted) while
the BEFORE arbiter ran, before this file was written; no Rust code existed.
The user's notice amendment (info/warning naming the function, repository,
ref and resolved commit; received from the integrator before this file was
written) is part of the design below, not an amendment after freezing.

Base: local `main` at `73beab5d8` (BEFORE measured there, detached, release
binary built at the base). The card's own first commit is the cherry-picked
draft contract `fd4a0da6b` (`monkey/vhost_set.qv` and its load-path rows).

## Design decisions (the tip mode)

1. **The mode lives in the lock alone.** No `.qv` surface changes: the
   declaration stays pinned (the default). Its lock entry holds either
   reviewed identities (today's pinned entry) or exactly one **tip identity**,
   never both, so an earlier reviewed revision can never keep a tip contract
   in force after the tip moves.
2. **Lock format version 3** adds a top-level `tip` list (sorted, unique, a
   function in at most one of `pinned`, `flexible`, `tip`). The writer emits
   version 3 only for a lock holding a tip entry, else version 2, so every
   existing lock stays byte-identical. Versions 1 and 2 are still read; a
   version-2 lock with a tip entry, or a version-3 lock without one, is
   malformed.
3. **A tip entry** records the function, module, contract digest, the
   repository (`name`, an informational `url`, and the absolute local `path`
   where it was found when pinned), the ref as a full refname
   (`refs/heads/…`), an optional free-text `release_label` (no semantics), and
   the **reviewed** record: the commit the person reviewed and its sites.
4. **Locating the repository** on any host: the environment variable
   `QUIRIN_TIP_REPOSITORIES` (`NAME=PATH` entries in the platform's path-list
   form) first, then the lock's recorded `path`; the URL is never fetched.
   Only read-only Git commands run in it (`rev-parse`, `merge-base
   --is-ancestor`, `ls-tree`, `cat-file`); nothing writes to it. A malformed
   variable refuses the audit.
5. **What a check resolves**: the ref's current commit in the located
   repository; the reviewed commit must be an ancestor of (or equal to) it —
   a rewritten branch or another repository at the path is not tracked; the
   function's sites are read from that commit's blobs, never a working tree,
   and compared with the audited source's sites exactly (paths included).
6. **Standing**: in force exactly when the contract text is the pinned one,
   the tip resolved and the sites are equal. A tip that cannot be resolved —
   repository not located, ref missing, not descended from the reviewed
   commit, unreadable blob, or no site of the function at the tip — is held
   under the new reason `tip-unresolved` with the failure named; a resolved
   tip whose sites differ is held under `source-changed` with a tip-specific
   detail. Both hold calls under `contract-pin-mismatch` exactly as a pinned
   mismatch does (never `Safe` or `Unsafe`; `Pending` under the held-call rule).
7. **Reviewed versus tracked**: the sidecar's record of every tip entry
   carries the resolved commit, how the repository was located, the reviewed
   commit, whether the tip moved since review and whether its sites changed
   since review; every tip contract in force is a **named assumption**
   (`assumptions.tip_tracked`, one record per function, named
   `tip-tracked:FUNCTION`), and its inventory status is `tip-in-force`, never
   `pinned-in-force`, so no verdict resting on it claims a reviewed pin.
8. **The notice** (user amendment): every tip entry, wherever it is loaded,
   pinned, checked or reported (`pin`, `pin --check`, `--verify-contracts`,
   each audit route's console tally, the sidecar's `contract_revision`
   section, `compare`), carries a notice naming the function, repository,
   ref, release label and resolved commit and stating that the contract
   targets a future, unreleased release (a release candidate), not a released
   version. Severity `info` when it is in force for the audited source,
   `warning` when it is not (with the reason) or cannot be resolved. With no
   audited source (`--verify-contracts`) it is `info` when the tip resolves on
   the host and `warning` when it does not. Presentation and evidence only:
   it is computed after every verdict and is never read by one. A run with no
   tip entry is byte-identical (no new key, unchanged tally line).
9. **`pin`**: `--track-tip REF` (with `--function`, a Git checkout as
   `--source-root`, optional `--repository-name`, `--repository-url`,
   `--release-label`) records a tip entry, replacing a reviewed entry;
   `--untrack-tip` (with `--function`) converts a tip entry back to a reviewed
   identity of the checkout's `HEAD`; every other `pin` run, `--regenerate`
   and `--drop-revision` keep tip entries as written (a changed contract text
   is reported and leaves the entry out of force until re-tracked).
   `pin --check` counts tip entries in force "through the tip" (exit 0
   allows them) and an out-of-force tip like any mismatch (exit 1);
   `--verify-contracts` counts tip entries as pinned declarations.
10. **`compare`**: a `tip_tracking` section — per function, each run's
    standing, ref, resolved commit and notice, whether the tip moved between
    the runs — and its counts; the console summary names tip moves.
11. **No path-prefix mapping**: a vendored copy under another directory
    (`lib/monkey/…`) does not match the tip's sites; registered as a residual.

## Predicted outcomes

* **Corpus (arbiter, `66910c10`, `--jobs 4`)**: custody **byte-identical** to
  BEFORE — `resource-diagnostics.json` digest unchanged, 647 Safe / 10
  provisional Unsafe / 4,177 Unknown, 0 Pending. Reason: nothing in the
  corpus calls `mk_vhost_set`, and its own body (`lib/monkey/mk_server/
  mk_lib.c`) is analysed as an undeclared definition whether the declaration
  is absent (BEFORE) or withheld (AFTER).
* **Corpus sidecar and log**: the libevent lock digest changes (version 3,
  one tip entry); `contract_revision` gains one pinned declaration not in
  force, `mk_vhost_set`, reason **`source-changed`** against the tip
  (the tip resolves on this host through the lock's path; the corpus's sites
  are under `lib/monkey/` and its body still has the defect), 0 held call
  sites, one **warning** notice; the console tally gains the tip clause and
  the notice line.
* **Monkey checkout** (`pin --check --source-root
  /home/cosmo/Documents/GitHub/monkey --crate qcore-provider-libevent
  --function mk_vhost_set`): **in force through the tip**
  `refs/heads/cosmo0920-missing-va_end-on-null` at `21a2c190`, **info**
  notice, exit 0.
* **Controls** (new tests): tip resolves and matches → in force, info; tip
  moved to a commit whose sites differ → `source-changed`, warning; repository
  not located, ref missing, diverged → `tip-unresolved`, warning; a reviewed
  pin beside a tip entry unaffected; sidecar records the resolved commit;
  verdicts byte-identical between a tip in force and the same contract pinned
  at the same sites (and between an unresolved tip and a missing entry); a
  notice per surface with the right severity.
* **Lean**: one new module `QCore.ContractTipMode`, 16 theorems, registered in
  a new `AuditContractTipMode.lean`; Lean audits 277 → 278.
* **Contract corpus pins**: one module and one resource-v1 declaration more
  (`monkey/vhost_set.qv`, `mk_vhost_set`); its clauses only if the reading
  against the fixed body corrects one.

## BEFORE (measured, base `73beab5d8`, release binary built there)

`private/contract-pin-tip-mode/before/` (worktree-private), exit 0, `--jobs 4`:
647 Safe / 10 provisional Unsafe / 4,177 Unknown functions;
`resource-diagnostics.json` `ed384eb0efbf64df53c804907ea24ac9440326c1165d6d021a3f0e84f2d6660f`
(= main's recorded identity); `resource-diagnostics.contracts.json`
`af927d8eac26e81e4391e92eb54aeeb282f039ce81284e6a268b395b3692a0e0`;
`audit.log` `39b9601b695ba83f6dab2ae34ac1392296a387c44fe8d744cc2ca5bfe976f03d`;
contract revision: 131 pinned, all in force, 120 libc under the standing
assumption, 0 held. Predicted AFTER: custody digest `ed384eb0…` unchanged;
132 pinned (131 in force, 1 not in force: `mk_vhost_set`, `source-changed`),
0 held; one tip entry, one warning notice.
