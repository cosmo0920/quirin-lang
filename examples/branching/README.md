# Branching

`if` and `else`, and everything a condition may contain.

| Example | What it demonstrates | Successful main result |
|---|---|---|
| [conditional_release.qcore](conditional_release.qcore) | The custody join: a resource released on both arms | `0` |
| [condition_tier.qcore](condition_tier.qcore) | Constant bindings, `\|`, `^`, `&`, `&&`, `\|\|`, `!` | `0` |
| [early_return.qcore](early_return.qcore) | An arm that returns, and what the join is then taken over | `1` |

Checked by [`compiler/tests/examples_suite.rs`](../../compiler/tests/examples_suite.rs):
every transcript and every refused program below is read out of this file and
compared with a real run or a real compilation.

```sh
cargo test -p qcore-compiler --test examples_suite
```

## The custody join

At the end of an `if`/`else` the checker takes a **custody join** over the arms.
Both arms of [conditional_release.qcore](conditional_release.qcore) release `a`,
so they agree and the join succeeds.

Delete one of the two `drop a;` and the module refuses:

<!-- refusal: Q046 -->
```qcore
error Failure { Failed; }

fn main() -> i32 {
    let a = qcore_acquire(1, true, Failure);
    if (3 < 5) {
        drop a;
    } else {
    }
    return 0;
}
```

<!-- diagnostic: Q046 -->
```text
refusal.qcore:5:5: Q046: the arms of this branch do not join in the resource-flow model: conditional branches have different net resource changes
    if (3 < 5) {
    ^
```

That is the conditional leak — the bug this construct exists to make
unwritable — and the diagnostic points at the branch, not at the arm, because
the fault is the disagreement rather than either arm on its own.

## Why the conditions are decidable, and what that does not decide away

Every condition in these three files is decided at check time. That is
deliberate: it makes the reference execution, the CFG observation and the
compiled program's own output single-valued, and therefore comparable line for
line.

What it does *not* decide away is the branch. Both arms are validated, both are
emitted as real basic blocks, and the emitted C computes `3 < 5` and
`(mask | 3) == 14` **itself**. So the agreement between the Rust evaluator and a
C compiler is a real cross-check rather than a tautology.

The emitted programs print their own instrumentation — the result, then the
acquisition and release trace — which is what the test parses and compares.
`conditional_release.qcore` writes:

<!-- transcript: conditional_release stdout -->
```text
OK 0
A 1
R 1
```

`OK 0` is the entry's result, `A 1` the acquisition of resource `1`, `R 1` its
release. One acquisition, one release, whichever arm ran.

## The condition tier

[condition_tier.qcore](condition_tier.qcore) uses all of it in one entry: a
constant binding read from inside a condition (`mask`), the three bitwise
operators, both connectives and unary `!`.

None of these is an operation the checker learned. Each is a **token-tier**
construct that lowers onto the one branch primitive:

| Written | Lowers to |
|---|---|
| `if (A \|\| B) { X } else { Y }` | `if (A) { X } else if (B) { X } else { Y }` |
| `if (A && B) { X } else { Y }` | `if (A) { if (B) { X } else { Y } } else { Y }` |
| `if (!C) { X } else { Y }` | `if (C) { Y } else { X }` — an arm swap |

Short-circuit is therefore **structural**, not asserted: the second operand of a
`||` is the condition of a branch sitting inside the first one's `else` arm, so
the emitted C reaches it on exactly one path. Each row of that table is pinned
by a test that compiles both spellings and compares the emitted C **byte for
byte** — nothing downstream can tell a connective or a `!` was written.

The custody join is unmoved by any of it. Every level runs the same join, which
is why all three arms of `condition_tier.qcore` release.

Its transcript is the same one-acquisition, one-release shape:

<!-- transcript: condition_tier stdout -->
```text
OK 0
A 1
R 1
```

### The parentheses are load-bearing

**C ranks `|`, `^` and `&` looser than the comparisons. This tier ranks them
tighter.** Rather than silently redefine a precedence every C programmer already
carries, the ungrouped spelling is refused:

<!-- refusal: Q045 -->
```qcore
error Failure { Failed; }

fn main() -> i32 {
    let mask = 12;
    let a = qcore_acquire(1, true, Failure);
    if (mask | 3 == 15) {
        drop a;
    } else {
        drop a;
    }
    return 0;
}
```

<!-- diagnostic: Q045 -->
```text
refusal.qcore:6:9: Q045: a bitwise comparison operand must be parenthesized: C ranks `|`, `^` and `&` looser than the comparisons while this tier must rank them tighter, so `flags | MASK == 0` would mean one thing to a C reader and another here. Write `(flags | MASK) == 0` and let the parentheses say which you mean
    if (mask | 3 == 15) {
        ^
```

Write `(mask | 3) == 15` and the two languages agree about what you meant.

The same rule runs in the other direction for `!`: C binds it *tighter* than
every comparison, so `!3 < 5` is `(!3) < 5` there and could only be `!(3 < 5)`
here. It is refused at `Q045` too, and `!(3 < 5)` is the program.

*Among* the bitwise operators the ranking is C's own and is therefore declared
rather than refused: `&` binds tighter than `^` binds tighter than `|`, so
`(4 | 1 & 1) == 5` needs no inner grouping and means what a C reader expects.

## An arm that returns

[early_return.qcore](early_return.qcore) has no `else`:

```qcore
if (3 < 5) {
    drop a;
    return 1;
}
drop a;
return 0;
```

The arm that returns discharges its custody at **its own exit** — the `drop a;`
before `return 1;` — which is what the route requires at every exit. Having
exited, it never reaches the join, so the join is taken over the arms that
survive to it.

Here exactly one arm survives, so the statements written after the branch belong
to it: they run on the fall-through path and nowhere else, and the driver lowers
them inside the implicit `else`. Every checked model then sees a branch both of
whose arms end the function, which is why the shape reads the way an early
return reads in C.

Remove either `drop a;` and it refuses — but at that *exit*, not at the join. A
conditional leak through an early return is still a conditional leak.

The entry returns `1`, and the transcript shows it:

<!-- transcript: early_return stdout -->
```text
OK 1
A 1
R 1
```

## Building and running

```sh
quirin check examples/branching/conditional_release.qcore
quirin emit examples/branching/conditional_release.qcore -o /tmp/branch.c
cc -std=c11 -Wall -Wextra -Werror -pedantic /tmp/branch.c -o /tmp/branch
/tmp/branch
```

All three examples check and emit through the CLI; they call nothing external.

## Not expressible here

* **A branch is a statement, not an expression.** There is no conditional
  expression, and `|>` does not appear in a condition (`(1 |> 2) == 3` refuses
  at `Q045`).
* **Chained comparisons are not a program.** `1 < 2 < 3` refuses at `Q045`
  rather than meaning either of the two things it could mean.
* **A branch on this route is not available on the task routes.** The
  copy-scalar task profile does not model one and refuses at `Q500`; see
  [../threads/](../threads/) for what those routes do model.
