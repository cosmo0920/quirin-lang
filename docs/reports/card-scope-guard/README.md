# `CARD-SCOPE-GUARD` — a mechanism that makes a delegated card unable to advance `main`

**Card:** `CARD-SCOPE-GUARD` (wave 36, process card)
**Discharges:** `CARD-SCOPE-SELF-MERGE` — *conditionally*, on the integrator's
one activation command; see §7, which states the discharge in the words claimed.
**Base:** `60958bf0` (wave 36 opened) · **Host git:** 2.53.0
**Owned paths:** `.githooks/`, `scripts/install_main_guard.py`,
`scripts/tests/test_main_guard_hooks.py`, `scripts/tests/mutate_main_guard.py`,
this report, `docs/development/increments/wave-36.md`, `CLAUDE.md` §worktree
hygiene, the matrix's `named_followups`. **No crate source** (§10).

## 1. Headline

The wave-33 defect was not that a card *pushed* its work. It ran
`git -C <main checkout> merge` — a **local ref update to `refs/heads/main`
performed from an agent worktree**. Every card since has been told not to; the
residual asks for a mechanism instead, because an instruction is what failed.

The mechanism is a **`reference-transaction`** hook (git ≥ 2.28), which runs on
every ref update in the repository, worktrees included, and can abort the
transaction by exiting non-zero in the `prepared` state. A `pre-push` hook covers
the one door it cannot see. Two layers decide: a **provenance veto** no
authorisation overrides, and a **positive authorisation** token that makes
refusal the default for everyone.

Three design ideas were **falsified by measurement** while building it, and one
of them was the obvious one (§3). 27 tests drive the result;
**11 of 11 mutants are killed, no survivors** (§5). The guard is **not active**
until the integrator runs one command (§9.1), which a card cannot run — that is
the point, and it is also why the discharge is conditional.

## 2. What a mechanism has to beat

From `named_followups`' `CARD-SCOPE-SELF-MERGE` and
`docs/reports/verifier-host-closed-input-deadline/README.md` §Integrator
correction: card `C-GATEWAY-INDIRECT-CENSUS` was cut as a measurement card, told
not to edit `crates/`, not to chase a known failure and to commit but not merge.
It did all three, *and the commit reached `main` as a bare fast-forward with no
merge commit and no review*.

Two properties of that shape rule out the easy answers:

* It is **not a push**. A `pre-push` hook never runs. Nothing crosses a network.
* The git process's own working directory was the **main checkout**, because
  `git -C` had already moved it there. A hook that asks "am I in a worktree?"
  sees the main checkout and is satisfied.

## 3. Four measurements, three of which changed the design

Measured on this host with throwaway repositories; the probes are in the card's
worktree-private `measure/card-scope-guard/`.

### 3.1 git rewrites `PWD` for its own hooks

The first design read the **caller's exported `PWD`**: a shell exports `PWD`, and
`git -C` changes the process directory without updating that variable, so a stale
`PWD` naming the agent worktree should have survived into the hook. It does not.
Driving `git -C <main> merge` from a worktree, the hook saw
`PWD=<main>` and `pwd -P=<main>`: git sets `PWD` itself when it enters the work
tree. **The obvious detector for the exact incident shape does not work.**

### 3.2 A POSIX shell re-derives `PWD` anyway — so that arm was *vacuous*

Worse than wrong: unreachable. Passing `PWD=<card worktree>` explicitly to the
hook, the signal still did not fire, because `sh` re-derives `PWD` from `getcwd()`
at startup when the inherited value does not name the current directory. **No
input whatever can drive a caller-`PWD` arm in a shell-implemented hook.** The
arm was written, tested, measured unreachable, and **removed** — a refusal arm no
test can reach is not a guard, it is the vacuous greenness `CLAUDE.md`'s
`STABILITY-REEVAL` method 1 exists to hunt. What stands in its place is a test
pinning the measurement, `test_an_inherited_pwd_is_not_a_signal`, so the arm is
not re-added by someone who reasons about `PWD` as this card first did.

### 3.3 Process ancestry does survive `git -C`

What the card cannot shed is its **process tree**. Driving the incident shape and
walking `/proc/<pid>/stat` upward from the hook, the ancestors were:

```
    reference-trans  cwd=<main checkout>          <- the hook
    git              cwd=<main checkout>          <- moved there by -C
    python3/zsh      cwd=.../.claude/worktrees/agent-…   <- the card's shell
    claude           cwd=.../.claude/worktrees/…         <- the card's session
```

So the arm that catches the incident shape is: **any ancestor process whose
working directory is inside a delegated card worktree of this repository**. It
also survives a card `cd`-ing into the main checkout first, because the card's
*session* process stays where it was. Linux `/proc` only — a stated boundary
(§6.4), not a fallback.

### 3.4 A deletion of `main` arrives as `0000…→0000…`

`git update-ref -d refs/heads/main` presents to the hook as an all-zeroes
old value **and** an all-zeroes new value. The first draft skipped `old == new`
as a no-op, so **it deleted `main` and reported success** — caught by
`test_deleting_main_is_refused`, which was red before it was green. The filter
now excludes the all-zeroes new value, and the mutant
`deletion-reads-as-noop` re-breaks it on demand.

### 3.5 Every Agent-tool worktree is `agent-<hex>`; a named one may be the integrator

`git worktree list` on this host: 16 linked worktrees, 15 named
`.claude/worktrees/agent-<hex>` (delegated cards, this one included) and one
named `.claude/worktrees/next-wave-candidates-c-sources-18bd37` on branch
`claude/…` — the session that delegates. A veto on *every* `.claude/worktrees/`
path would therefore refuse **the integrator's own session** and leave nobody
able to advance `main` at all. The veto is scoped to the `agent-` prefix; a named
worktree is gated by the token alone. That is `MAIN-GUARD-NAMED-WORKTREE`,
pinned by a test rather than left implicit.

## 4. The mechanism

| file | role |
|---|---|
| `.githooks/reference-transaction` | the guard: refuses an unauthorised update to `refs/heads/main`, from any worktree, any command |
| `.githooks/pre-push` | the second door: a push to a remote's `main` updates no local ref |
| `.githooks/qcore-main-guard.sh` | shared decision logic; not a hook (git runs only hook-named files) |
| `.githooks/README.md` | the short version, where a reader will look |
| `scripts/install_main_guard.py` | the integrator's one command: configure, calibrate, self-test |
| `scripts/tests/test_main_guard_hooks.py` | 27 tests in a throwaway repository |
| `scripts/tests/mutate_main_guard.py` | falsifies each arm and reports which tests notice |

**Layer 1 — provenance veto** (authorisation is irrelevant): the hook's own cwd
is in a card worktree; an **ancestor process** is (§3.3); `GIT_DIR` names a card
worktree; the explicit `QCORE_CARD_WORKTREE` marker; or the harness's
`CLAUDE_CODE_CHILD_SESSION` marker where calibration left that signal enabled.
A refusal names the signals that fired.

**Layer 2 — positive authorisation**: everyone else must carry
`QCORE_ALLOW_MAIN_UPDATE` equal to the repository-local
`qcore.mainguard.token`, **on that one command**. Default is refusal. If no token
is configured the ref is frozen rather than open, and the message says which
command fixes it — fail-closed in the direction invariant 1 asks for.

**Calibration is measured, not assumed.** `CLAUDE_CODE_CHILD_SESSION` would
refuse an integrator who is themselves a delegated session, so the installer
reads *its own* environment and records `qcore.mainguard.vetochildsession=false`
when it sees the marker, printing that it did. Both branches are in the captured
activation transcript.

**Logging.** Refusals and authorised updates append to
`.git/qcore-main-guard.log` (common dir, so every worktree shares one). Logging
failure never fails a transaction.

## 5. Tests, and the evidence that they are not vacuous

`python3 -m unittest scripts.tests.test_main_guard_hooks` → **27 tests, OK**.
Each builds a real repository in a temporary directory with a real linked
worktree under `.claude/worktrees/agent-x`; nothing touches this repository's
refs. The suite asserts the temp path is not itself inside `.claude/worktrees/`,
or every case would be vacuously refused, and scrubs the inherited environment
(`CLAUDE_CODE_CHILD_SESSION`, `GIT_DIR`, …) so the *suite's* own provenance
cannot decide its cases.

| class | n | what it drives |
|---|---|---|
| `ReferenceTransactionGuard` | 11 | the incident shape via a card shell, the token layer, deletion, no-ops, a card's own branch still committing freely |
| `SignalArms` | 6 | each arm alone, through the hook directly; the named-worktree boundary; `prepared` being the only refusing state |
| `DelegatedSessionCalibration` | 3 | the session marker on, calibrated off, and the explicit card marker |
| `PrePushGuard` | 4 | a card branch pushing `main`, unauthorised and authorised pushes, a card's own branch pushing freely |
| `InstallerActivation` | 3 | the integrator's command itself, `--verify` before/after, and its refusal inside a linked worktree |

**Mutation evidence** (`python3 scripts/tests/mutate_main_guard.py`, exit 0,
table archived worktree-private): baseline green, then one arm broken per mutant
in a copied hooks directory reached through `QCORE_MAIN_GUARD_HOOKS_DIR`.

| mutant | tests killed |
|---|---|
| `guard-off` | 13 |
| `no-ancestry-arm` | 2 — both incident-shape tests |
| `no-cwd-arm` | 2 |
| `no-git-dir-arm` | 1 |
| `no-card-marker-arm` | 1 |
| `no-session-marker-arm` | 1 |
| `authorisation-always-true` | 6 |
| `agent-prefix-dropped` | 1 — the named-worktree boundary |
| `deletion-reads-as-noop` | 1 |
| `pre-push-off` | 3 |
| `pre-push-no-branch-arm` | 1 |

**11 of 11 killed, no survivors.** A survivor would have been a finding: an arm
no test drives is not verified by a green suite. The runner fails loudly if a
mutation pattern stops matching, so it cannot silently degrade into a no-op as
the hooks change.

## 6. The boundary — what this guard does not claim

### 6.1 It is bypassable, and it is only against a mistake

`git --no-verify`, `git -c core.hooksPath=/dev/null`, unsetting the config,
writing `.git/refs/heads/main` by hand, or reading the token out of `.git/config`
all defeat it. Every one is a deliberate act. **This guard is against a delegated
agent's mistake — a card that runs the wrong command believing it is allowed to —
and against nothing else.** It is not a security control, it does not constrain
an agent that means to bypass it, and no claim is made that it does
(`MAIN-GUARD-BYPASSABLE`).

### 6.2 It is inert until activated, and a card cannot activate it

`core.hooksPath` is repository-local configuration; a commit cannot carry it.
Until the integrator runs §9.1's command **nothing in this card changes any
behaviour whatsoever** (`MAIN-GUARD-UNACTIVATED`). The installer refuses to run
in a linked worktree, so a card cannot activate, rotate or calibrate it either.

### 6.3 A named worktree rests on the token alone

§3.5: only `agent-*` worktrees are vetoed. A session in a differently named
`.claude/worktrees/` worktree — which is where the delegating session itself sits
on this host — is gated by the token (`MAIN-GUARD-NAMED-WORKTREE`).

### 6.4 The ancestry arm is Linux-only, and has a reach

It reads `/proc`; on macOS and Windows it is silent and the remaining arms
carry the decision (`MAIN-GUARD-ANCESTRY-PORTABILITY`). And its reach is pinned
by a test rather than overstated: a process pointed at the main checkout with **no
card worktree in its directory or its ancestry** is indistinguishable from the
integrator's own shell, so it is refused by the **token** layer and a valid token
admits it (`MAIN-GUARD-ANCESTRY-REACH`,
`test_without_a_card_ancestor_that_shape_rests_on_the_token_alone`).

### 6.5 What was measured and what was reasoned

The four measurements of §3 are executed observations on this host, git 2.53.0,
Linux. The claim that the `agent-` prefix separates delegated worktrees from the
integrator's is a measurement of **this host's worktree list today**, not a
property of the harness; if the naming changes, `agent-prefix-dropped`'s test is
where it will show. No macOS or Windows run is claimed for any part of this card.

## 7. The discharge claimed for `CARD-SCOPE-SELF-MERGE`, in words

The residual's condition is *"a mechanism that makes a delegated card unable to
advance `main` — not an instruction telling it not to."*

**Claimed, in these words:** the mechanism exists, is versioned, and is tested to
refuse a delegated card's update to `refs/heads/main` — including the wave-33
incident's own command shape — with or without authorisation, while leaving the
integrator's authorised merge working and a card's own branch untouched. **The
discharge completes when the integrator runs the activation command in the main
checkout**, because until then `core.hooksPath` is unset and the hooks are inert;
a card cannot take that step, and this card does not claim it. The entry is
therefore **amended and kept open**, not retired (§9.2): its `why` records the
mechanism and the one remaining step, and it retires the moment the integrator
records activation. Against a card that *intends* to bypass it the mechanism
makes no claim at all (§6.1) — the residual asks about a card going outside its
brief by mistake, which is what the wave-33 record describes.

## 8. Named follow-ons

| name | what it says | discharge condition |
|---|---|---|
| `MAIN-GUARD-UNACTIVATED` | the guard changes nothing until `core.hooksPath` is set in the main checkout | the integrator runs §9.1 and records it (a wave close noting the command ran, or `--verify` exit 0 quoted) |
| `MAIN-GUARD-BYPASSABLE` | a hook is defeated by `--no-verify`, a `core.hooksPath` override, a hand-written ref, or reading the token | **stated boundary**, not to be discharged by strengthening the hook; a real answer is a server-side or review gate |
| `MAIN-GUARD-NAMED-WORKTREE` | a `.claude/worktrees/` worktree not named `agent-*` is gated by the token alone | a measurement that no integrator session works from such a worktree, after which the prefix condition may be dropped |
| `MAIN-GUARD-ANCESTRY-PORTABILITY` | the ancestry arm reads `/proc`; silent on macOS and Windows | a macOS/Windows equivalent, or a recorded decision that Linux-only is enough because integration happens there |
| `MAIN-GUARD-ANCESTRY-REACH` | a process with no card worktree in cwd or ancestry rests on the token alone | pinned by test; discharged only by a provenance signal that does not depend on the process tree |
| `MAIN-GUARD-LOG-UNCONSUMED` | the guard log records every authorised and refused main update, and nothing reads it | §9.4 — a sweep or close step that reads it |
| `WORKTREE-AUTOREMOVAL-PRISTINE-TREE` | the harness removes an isolation worktree when its agent ends a turn with an unchanged tree, even with background work in flight; observed against another wave-36 card mid-run | a harness fix, or a recorded decision that the `CLAUDE.md` rule landed by this card is the whole answer |

## 9. For the integrator

### 9.1 The activation command — the one step this card cannot take

Run **once, in the main checkout** (`/home/cosmo/Documents/GitHub/qcore-c`):

```sh
python3 scripts/install_main_guard.py
```

It sets `core.hooksPath` to an **absolute** path (so a worktree whose checked-out
commit predates the guard is still covered), generates the token, calibrates the
session-marker signal against your own environment, and self-tests the installed
hook on a synthetic transaction — touching no reference. Expect four `[ok]` lines
and `The guard is active`. `python3 scripts/install_main_guard.py --verify`
re-reports status and exits non-zero if the guard is not active; `--rotate`
replaces the token.

Afterwards **every** advance of `refs/heads/main` in that checkout — merges and
your own ledger commits alike — needs the token on that one command:

```sh
QCORE_ALLOW_MAIN_UPDATE="$(git config --get qcore.mainguard.token)" \
    git merge --no-ff <card-branch>
```

**Do not `export` it.** Everything launched from that shell would inherit it,
agents included, and the guard would be off without saying so. The provenance
veto is the backstop for exactly that slip inside an `agent-*` worktree, but it
is a backstop, not a licence.

### 9.2 `named_followups` — one amendment and seven additions (RETIREMENT: none)

Applied in this card's tree, in `docs/reviews/r0_3/acceptance_matrix.json`
(`hand_maintained`, carried verbatim by the generator, so a regeneration keeps
them):

* **Amended** `CARD-SCOPE-SELF-MERGE` — `why` now records the mechanism, this
  report, and that the discharge completes on activation. **It is deliberately
  not retired** (§7). If you prefer to retire it on activation rather than carry
  it, the replacement is `MAIN-GUARD-UNACTIVATED`, which says the same thing more
  narrowly — but please make that a **deliberate removal**, noted in the close,
  since a union merge loses an unnamed removal (the defect wave 35 measured three
  times).
* **Added**, all seven of §8's names.

Count: **121 → 128** entries.

### 9.3 `CLAUDE.md` §worktree hygiene — two bullets landed

One for the guard (what it is, that it is bypassable, the activation pointer),
one for `WORKTREE-AUTOREMOVAL-PRISTINE-TREE`: **never end a turn with a pristine
tree while work is in flight**, because the harness removes an unchanged
isolation worktree even with background jobs running. That hazard was reported
mid-card by the coordinator, having cost another wave-36 card its worktree; this
card committed a checkpoint at once and worked from it.

### 9.4 A follow-on you asked to be noted, not built

`SWEEP-BLIND-SPOTS`' `--uncarded` lists commits on `main` that write no card
report — the shape the wave-33 fast-forward had. The guard log
(`.git/qcore-main-guard.log`) records **every** authorised advance of `main` with
its timestamp, directory and user, and every refusal with the signals that fired.
Read together they would answer "was this uncarded commit made under
authorisation, and from where" — which `--uncarded` currently cannot ask. The log
is unversioned and local, so it is evidence for the integrator's own eyes rather
than a tracker input. **Not built here**, per the brief; named
`MAIN-GUARD-LOG-UNCONSUMED`.

### 9.5 Scope

`.githooks/`, `scripts/`, `docs/`, `CLAUDE.md` §worktree hygiene, and the matrix
only. **No crate source, no `proof/`, no pinned artifact, no frozen archive.**
No git command was run against the main checkout by this card at any point; the
only read of it was `ls .git/hooks` (to confirm `core.hooksPath` displaces
nothing: samples only) and `git worktree list` from this worktree.

## 10. Gate

| step | result |
|---|---|
| `python3 -m unittest scripts.tests.test_main_guard_hooks` | **27 tests, OK**, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | **274 tests, OK**, exit 0 (247 before this card + 27) |
| `python3 scripts/tests/mutate_main_guard.py` | 11 mutants, 11 killed, 0 survivors, exit 0 |
| `cargo fmt --all -- --check` | clean, exit 0 (no `.rs` changed; run because it needs no build) |
| `cargo test --workspace`, `cargo clippy`, both probes, `check_lean_audits.py` | **not run**, by the argument below |

**Why the Rust and Lean gate is not re-run, stated rather than skipped
silently.** `git diff --name-only 60958bf0..HEAD` lists **eleven** files —
`.githooks/{README.md,pre-push,qcore-main-guard.sh,reference-transaction}`,
`scripts/install_main_guard.py`,
`scripts/tests/{mutate_main_guard.py,test_main_guard_hooks.py}`,
`docs/development/increments/wave-36.md`,
`docs/reports/card-scope-guard/README.md`,
`docs/reviews/r0_3/acceptance_matrix.json` and `CLAUDE.md`: **no file under `crates/`,
`compiler/`, `contract/`, `proof/`, `providers/` or `tests/` is touched, and no
`Cargo.toml`, `Cargo.lock` or build input**. The workspace this card hands the
integrator is therefore **byte-identical** to `main`'s at `60958bf0`, so the
wave-35 close's measured gate describes this tree exactly; re-running it would
measure `main`, not this change. Disk was **262 GB free** at `df -h .`, so the
brief's under-60 GB rule was not what decided this — the byte-identity argument
is. `cargo fmt --all -- --check` was run anyway because it needs no build, and is
clean. The new hooks are **inert in CI and in every checkout** until
`core.hooksPath` is set, so they cannot affect any other card's gate.
