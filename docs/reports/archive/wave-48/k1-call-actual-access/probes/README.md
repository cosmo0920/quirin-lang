# K1 unit probes

A scratch integration test ([k1_probe_scratch.rs.txt](k1_probe_scratch.rs.txt), kept as text, not a cargo target) ran `analyze_unrestricted_c11` on
one function `w` per line of a probe file, over the generic fixture contracts
`descriptor_generic.qv` and `derived_result_generic.qv` (`d_len` and `d_copy`
declared here with `void *` parameters, `Borrowed` positions; `k_free`
releases; `d_consume` consumes), printing the status and the sorted diagnostic
keys ([scripts/probe-run.sh](../scripts/probe-run.sh)). The probe files are
kept here; the outputs below are the runs as printed (`B` the base
`3ca85c7a6`, `T` the rule-75 tip `be1f3c8c5`).

## The base reads the generic shapes (B, `probes1.txt`, `probes3.txt`, `probes10.txt`)

`Safe` on the base, each: `d_len(b->name)`, `d_len(b->next->name)` (statement,
assignment, return, condition, `while` condition, a condition beside `&&`),
`d_len((char *)b->name)`, `d_len(b->name ? b->name : "")`, `d_len(a[i])`,
`d_copy(b->name, s, 3)`, `return b ? d_len(b->name) : 0`,
`return d_len(b->name) == 0`, `d_copy(a->s, b->s, 23)` (array decay),
`d_len(&t->lock)` over a union-typed member, `d_len(&rk->eos.tmr)`,
`d_copy(&rkb->rk->timers, &rkb->tmr, 1)`, a typedef-spelled base
(`box_t *b`), the `rd_kafka_dbg` expansion
`if ((__builtin_expect(((rk->conf).debug &(0x100)), 0))) d_len(&rk->conf);`.

Refused on the base (each a follow-up below, none K1's rule):

| Probe | Base |
|---|---|
| `return 31 * d_len(b->name);` (and `return 31 * d_len(s);`) | Unknown: return expression |
| `long r = 31 * d_len(b->name);` | Unknown: ``call to uncontracted function `d_len` `` (a contracted callee) |
| `d_len((const char *)&b->n);` | Unknown: gate (a cast of an address, CG08) |
| `x = d_len(r->pfd);`, `struct q pfd[2]` (record-array decay) | Unknown: gate |
| `d_copy(b->name, b->name, sizeof(*b->name) * b->n);` | Unknown: gate (`sizeof (*v)`) |

## The parenthesised root (B then T, `probes7.txt`, `probes8.txt`)

| Probe (`t_t *t`, a typedef-spelled struct pointer) | B | T |
|---|---|---|
| `d_len(&(t)->n);` | Unknown: gate | Safe |
| `d_len(&(t)->lock);` (`mtx_t`, a typedef union or struct) | Unknown: gate | Safe |
| `if (d_len(&(t)->n)) return 1;` | Unknown: branch condition | Safe |
| `return d_len(&(t)->n);` | Unknown: gate | Safe |
| `d_len(&t->n); d_copy(&(t)->n, t, 1);` | Unknown: gate | Safe |
| `d_len(&t->n);`, `d_len((t)->name);`, `if ((t)->n > 2)`, `(t)->n = 2;`, `long x = (t)->n;` | Safe | Safe |
| `d_len(&(t)->lock);` with `struct t *t` (lexically typed) | Safe | Safe |

## Near-misses keep their refusal (T, `probes9.txt`)

`k_free(t)` before or after `d_len(&(t)->n)`: Unknown (gate, `unknown_alias:t`);
`g(&(t))` elsewhere: Unknown (gate, uncontracted `g`); `t = mk()`: Unknown;
`t = u`: Unknown (`unknown_alias:u`); `(t)++`: Unknown (gate, increment);
owned local `t = k_alloc(…)` before or after `k_free(t)`: Unknown (gate);
`d_len(&(t)->name)` (a pointer member): Unknown (gate); a walk
`while (c--) { d_len(&(t)->n); t = t->next; }`: Safe (the never-bound reading
of `t = t->next` is unchanged).
