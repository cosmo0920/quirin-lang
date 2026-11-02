#!/usr/bin/env python3
"""Result-set and caller-test census of Monkey's cell-bearing creators (card
CELL-GUARD-RESULT-RANGE; gateway rule 38, contract specification C04.6.2 and
C06.5, grammar G05d and G05e).

USER DIRECTION 2026-11-03 (verbatim): "The Fluent Bit's caller tests
sometimes check the incorrect assertions so we need to pay attention for the
assertions and actual returning values of Fluent Bit functions."

Usage:
    census.py --root REV=PATH [--root REV=PATH ...] --write census.json
    census.py --root REV=PATH [--root REV=PATH ...] --check census.json

A root is a Fluent Bit checkout (Monkey under ``lib/monkey/``) or, with a
revision label starting ``monkey-``, a Monkey checkout (the corpus's own
Monkey source). Every file is read only.

Part 1, the bodies. Every definition of the creators and of the helpers their
results come from (``BODY_FUNCTIONS``) in Monkey's ``mk_event.c`` and its five
backends is extracted lexically (comments and strings blanked, a body opens
with ``{`` in column 0), with the SHA-256 of its normalised text and its
``return`` expressions. ``bodies.json`` reads each body (keyed by function and
digest, so one reading covers every tree holding the same body): the value
set of each ``return``, once from the body alone (a call's result is its C
type's range unless an in-unit helper's reading bounds it) and once with the
C library's documented result sets (``rests_on`` names each). The tool checks
that the reading lists exactly the body's returns and that the stated result
is their union.

Part 2, the result sets. A creator's result set per backend is the one of the
backend helper its public body returns (``mk_event.c`` forwards through
``loop->data``; checked in part 1). The configured backend is read from the
build: every Fluent Bit build here defines no ``MK_EVENT_LOOP_*`` and is a
Linux build, so it includes ``mk_event_epoll.c`` (``MK_HAVE_TIMERFD_CREATE``
defined); the corpus's Monkey build defines ``MK_EVENT_LOOP_LIBEVENT``.

Part 3, the caller tests. Every call of a creator outside ``mk_core/`` is
listed with its enclosing function, how its result is held (a plain name, a
member, inside a condition, discarded, returned) and the first condition
after it that tests the holder before the holder is written again.
``sites.json`` gives each site the edge its caller treats as failure (keyed by
revision-independent text). The tool then classifies the test against the
callee's contract classes (``CONTRACT``) and its result set:

* ``exact``      the test's failure set equals the contract's failure set on
                 every integer;
* ``redundant``  they differ only outside the result set (``== -1`` inside
                 {-1, 0});
* ``wrong``      some value of the result set is classified otherwise (a
                 failure treated as success, a success treated as failure),
                 or the test compares against a value the body never
                 returns; the misclassified values are listed;
* ``untested``   no test of the result before it is overwritten or dropped.

Each class is computed twice: with the C library's result sets (the actual
result set) and from the bodies alone (which shows what a body check without
the C library's result sets could decide). ``--check`` recomputes everything
and refuses (exit 1) a census file that differs, an unread body, a reading
whose returns differ from the body's, an unannotated site, an unused
annotation, or a changed tree digest. The census binds itself to the sources
through the SHA-256 of every file it read, folded into one digest per tree.

The reader is lexical, a census tool and not an analyzer: the annotations are
the reading, and what it cannot parse it reports rather than drops.
"""

import argparse
import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
BODIES = HERE / "bodies.json"
SITES = HERE / "sites.json"

MIN, MAX = -2147483648, 2147483647
CREATORS = ("mk_event_channel_create", "mk_event_timeout_create")
BODY_FUNCTIONS = CREATORS + (
    "_mk_event_channel_create",
    "_mk_event_timeout_create",
    "_mk_event_add",
    "_mk_event_update",
    "mk_event_libevent_socketpair",
)
BACKENDS = ("epoll", "kqueue", "select", "poll", "libevent")
EVENT_FILES = ("mk_event.c",) + tuple(f"mk_event_{b}.c" for b in BACKENDS)
# The contract's failure class (providers/qcore-provider-libevent/contracts/
# monkey/event_loop.qv): the cells are fresh under `#result == 0` for the
# channel and under `#result != -1` for the timer.
CONTRACT = {
    "mk_event_channel_create": ("!=", 0),
    "mk_event_timeout_create": ("==", -1),
}
SUFFIXES = (".c", ".h")
SKIP_DIRS = {".git", "build"}


def blank(text):
    """Comments and string/char literals to spaces, newlines kept."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(c + re.sub(r"[^\n]", " ", text[i + 1:j - 1]) + c)
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


HEADER = re.compile(r"^[A-Za-z_][^;\n]*?\b(\w+)\s*\(", re.M)


def functions(code):
    """(name, start, end) of every function body: it opens with `{` in
    column 0 after a header line that starts in column 0, and closes with the
    next `}` in column 0."""
    spans = []
    for m in re.finditer(r"^\{", code, re.M):
        head = code[max(0, m.start() - 600):m.start()]
        headers = list(HEADER.finditer(head))
        if not headers:
            continue
        name = headers[-1].group(1)
        if name in ("if", "while", "for", "switch", "return", "sizeof"):
            continue
        close = re.compile(r"^\}", re.M).search(code, m.end())
        end = close.end() if close else len(code)
        start = m.start() - len(head) + headers[-1].start()
        spans.append((name, start, end))
    return spans


def norm(text):
    return re.sub(r"\s+", " ", text).strip()


def line_of(code, pos):
    return code.count("\n", 0, pos) + 1


def balanced(code, open_pos):
    """The index just past the `)` matching the `(` at open_pos."""
    depth = 0
    for k in range(open_pos, len(code)):
        if code[k] == "(":
            depth += 1
        elif code[k] == ")":
            depth -= 1
            if depth == 0:
                return k + 1
    return len(code)


# ---- intervals ---------------------------------------------------------

def parse_set(spec):
    """'-1', '0', '[0,MAX]', 'int', unions with '|' -> sorted intervals."""
    out = []
    for part in spec.split("|"):
        part = part.strip()
        if part == "int":
            out.append((MIN, MAX))
            continue
        m = re.fullmatch(r"\[(\S+?),(\S+?)\]", part)
        if m:
            lo, hi = (MIN if m.group(1) == "MIN" else int(m.group(1)),
                      MAX if m.group(2) == "MAX" else int(m.group(2)))
        else:
            lo = hi = int(part)
        out.append((lo, hi))
    return merge(out)


def merge(intervals):
    out = []
    for lo, hi in sorted(intervals):
        if out and lo <= out[-1][1] + 1:
            out[-1] = (out[-1][0], max(out[-1][1], hi))
        else:
            out.append((lo, hi))
    return out


def show(intervals):
    def one(lo, hi):
        if (lo, hi) == (MIN, MAX):
            return "int"
        if lo == hi:
            return str(lo)
        return "[%s,%s]" % ("MIN" if lo == MIN else lo, "MAX" if hi == MAX else hi)
    return " | ".join(one(lo, hi) for lo, hi in intervals) or "{}"


def holds(op, k, v):
    return {"==": v == k, "!=": v != k, "<": v < k, "<=": v <= k, ">": v > k, ">=": v >= k}[op]


def points(intervals, constants):
    """Representative points: every endpoint, every constant and its
    neighbours, clipped to the intervals. A predicate built from comparisons
    with these constants is constant between consecutive points."""
    cands = {MIN, MAX}
    for c in constants:
        cands |= {c - 1, c, c + 1}
    for lo, hi in intervals:
        cands |= {lo, hi, lo + 1, hi - 1}
    return sorted(v for v in cands if any(lo <= v <= hi for lo, hi in intervals))


def subset_where(intervals, constants, pred):
    """The sub-intervals of `intervals` on which pred holds (pred is a
    comparison predicate over `constants`, so it is constant on each gap)."""
    pts = points(intervals, constants)
    out = []
    for i, v in enumerate(pts):
        if pred(v):
            out.append((v, v))
        if i + 1 < len(pts):
            a, b = v + 1, pts[i + 1] - 1
            if a <= b and any(lo <= a and b <= hi for lo, hi in intervals) and pred(a):
                out.append((a, b))
    return merge(out)


# ---- part 1: bodies ----------------------------------------------------

def monkey_prefix(rev):
    return "" if rev.startswith("monkey-") else "lib/monkey/"


def read_bodies(rev, root, files_read):
    found = []
    for name in EVENT_FILES:
        rel = monkey_prefix(rev) + "mk_core/" + name
        path = pathlib.Path(root) / rel
        raw = path.read_bytes()
        files_read[rel] = hashlib.sha256(raw).hexdigest()
        code = blank(raw.decode("utf-8", "replace"))
        ordinal = {}
        for fname, start, end in functions(code):
            if fname not in BODY_FUNCTIONS:
                continue
            text = code[start:end]
            n = ordinal.get(fname, 0)
            ordinal[fname] = n + 1
            returns = [norm(m.group(1)) for m in re.finditer(r"\breturn\b([^;]*);", text)]
            found.append({
                "file": name,
                "function": fname,
                "ordinal": n,
                "line": line_of(code, start),
                "sha256": hashlib.sha256(norm(text).encode()).hexdigest(),
                "returns": returns,
            })
    return found


def check_reading(body, reading, problems):
    where = f"{body['file']}:{body['function']}#{body['ordinal']}"
    rets = [r["expr"] for r in reading["returns"]]
    if rets != body["returns"]:
        problems.append(f"{where}: reading returns {rets} differ from the body's {body['returns']}")
    for key in ("body_only", "libc"):
        union = merge([iv for r in reading["returns"] for iv in parse_set(r[key])])
        stated = parse_set(reading["result"][key])
        if union != stated:
            problems.append(f"{where}: stated {key} result {show(stated)} is not the union {show(union)}")


# ---- part 3: callers ---------------------------------------------------

CALL = re.compile(r"\b(" + "|".join(CREATORS) + r")\s*\(")


def statement_bounds(code, pos, fstart):
    """The start (after the previous `;`, `{` or `}`) and the end (the `;`)
    of the statement holding pos, parentheses balanced."""
    depth = 0
    k = pos
    while k > fstart:
        c = code[k - 1]
        if c == ")":
            depth += 1
        elif c == "(":
            depth -= 1
        elif c in ";{}" and depth <= 0:
            break
        k -= 1
    start = k
    depth = 0
    k = pos
    while k < len(code):
        c = code[k]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        elif c == ";" and depth <= 0:
            break
        k += 1
    return start, k


def condition_around(code, pos, start):
    """If pos sits inside `if (` / `while (` / `CHECK(` opened before it in
    this statement, the condition text."""
    for m in re.finditer(r"\b(if|while|CHECK|TEST_CHECK|TEST_ASSERT)\s*\(", code[start:pos]):
        open_pos = start + m.end() - 1
        close = balanced(code, open_pos)
        if close > pos:
            return m.group(1), norm(code[open_pos + 1:close - 1])
    return None


def first_test(code, after, fend, holder):
    """The first condition after `after` mentioning holder, or None when
    the holder is written first. Returns (keyword, condition, line)."""
    pat_h = re.escape(holder).replace(r"\ ", r"\s*")
    write = re.compile(r"(?<![\w>.])" + pat_h + r"\s*=(?!=)")
    test = re.compile(r"\b(if|while|CHECK|TEST_CHECK|TEST_ASSERT)\s*\(")
    k = after
    while True:
        m = test.search(code, k, fend)
        w = write.search(code, k, fend)
        if m is None or (w is not None and w.start() < m.start()):
            return None
        close = balanced(code, m.end() - 1)
        cond = norm(code[m.end():close - 1])
        if re.search(r"(?<![\w>.])" + pat_h + r"(?!\w)", cond):
            return m.group(1), cond, line_of(code, m.start())
        k = close


def parse_test(cond, holder):
    """`H OP K`, `K OP H`, `H`, `!H` -> (op, k) of the true edge, else None."""
    c = cond
    while c.startswith("(") and balanced(c, 0) == len(c):
        c = c[1:-1].strip()
    h = re.escape(norm(holder))
    lit = r"\(?\s*(-?\s*\d+)\s*\)?"
    m = re.fullmatch(h + r"\s*(==|!=|<=|>=|<|>)\s*" + lit, c)
    if m:
        return m.group(1), int(m.group(2).replace(" ", ""))
    m = re.fullmatch(lit + r"\s*(==|!=|<=|>=|<|>)\s*" + h, c)
    if m:
        mirror = {"==": "==", "!=": "!=", "<": ">", ">": "<", "<=": ">=", ">=": "<="}
        return mirror[m.group(2)], int(m.group(1).replace(" ", ""))
    if re.fullmatch(h, c):
        return "!=", 0
    if re.fullmatch(r"!\s*" + h, c):
        return "==", 0
    return None


def direct_test(cond, callee):
    """A condition holding the call itself: `CALL(...) OP K`."""
    m = re.search(r"\b" + callee + r"\s*\(", cond)
    close = balanced(cond, m.end() - 1)
    call = cond[m.start():close]
    return parse_test(cond.replace(call, "@R"), "@R")


def classify(op, k, failure_edge, callee, result):
    cop, ck = CONTRACT[callee]

    def test_failure(v):
        t = holds(op, k, v)
        return t if failure_edge == "true" else not t

    def contract_failure(v):
        return holds(cop, ck, v)

    consts = [k, ck]
    everywhere = [(MIN, MAX)]
    if not subset_where(everywhere, consts, lambda v: test_failure(v) != contract_failure(v)):
        return "exact", {}
    wrong_success = subset_where(result, consts, lambda v: contract_failure(v) and not test_failure(v))
    wrong_failure = subset_where(result, consts, lambda v: not contract_failure(v) and test_failure(v))
    detail = {}
    if wrong_success:
        detail["failure_treated_as_success"] = show(wrong_success)
    if wrong_failure:
        detail["success_treated_as_failure"] = show(wrong_failure)
    if op in ("==", "!=") and not any(lo <= k <= hi for lo, hi in result):
        detail["constant_never_returned"] = k
    if detail:
        return "wrong", detail
    return "redundant", {}


def enclosing(spans, pos):
    for name, start, end in spans:
        if start <= pos < end:
            return name, start, end
    return None, 0, 0


def read_sites(rev, root, files_read):
    sites = []
    base = pathlib.Path(root)
    for path in sorted(base.rglob("*")):
        rel = path.relative_to(base).as_posix()
        if path.suffix not in SUFFIXES or set(rel.split("/")) & SKIP_DIRS:
            continue
        if "/mk_core/" in "/" + rel and pathlib.PurePosixPath(rel).name.startswith("mk_event"):
            continue
        try:
            raw = path.read_bytes()
        except OSError:
            files_read[rel] = "unreadable"
            continue
        files_read[rel] = hashlib.sha256(raw).hexdigest()
        if not CALL.search(raw.decode("utf-8", "replace")):
            continue
        code = blank(raw.decode("utf-8", "replace"))
        spans = functions(code)
        for m in CALL.finditer(code):
            callee = m.group(1)
            fname, fstart, fend = enclosing(spans, m.start())
            if fname is None or fname == callee:
                continue
            start, end = statement_bounds(code, m.start(), fstart)
            stmt = norm(code[start:end])
            cond = condition_around(code, m.start(), start)
            site = {
                "file": rel, "line": line_of(code, m.start()), "function": fname,
                "callee": callee,
            }
            if cond is not None:
                site.update(holder_kind="condition", holder=None, test=cond[1],
                            test_line=site["line"], parsed=direct_test(cond[1], callee))
            else:
                lhs = re.match(r"^(.*?)(?<![=!<>])=(?!=)\s*" + callee + r"\s*\(", stmt)
                if stmt.startswith("return"):
                    site.update(holder_kind="returned", holder=None, test=None, parsed=None)
                elif lhs:
                    holder = norm(lhs.group(1))
                    holder = re.sub(r"^(?:[A-Za-z_]\w*\s+)+(?=[A-Za-z_]\w*$)", "", holder)
                    kind = "name" if re.fullmatch(r"[A-Za-z_]\w*", holder) else "member"
                    t = first_test(code, end, fend, holder)
                    site.update(holder_kind=kind, holder=holder,
                                test=t[1] if t else None, test_line=t[2] if t else None,
                                parsed=parse_test(t[1], holder) if t else None)
                elif stmt.startswith(callee):
                    site.update(holder_kind="discarded", holder=None, test=None, parsed=None)
                else:
                    site.update(holder_kind="unparsed", holder=None, test=None, parsed=None,
                                statement=stmt)
            site["key"] = "|".join([
                site["file"], site["function"], callee, site["holder_kind"],
                site["holder"] or "", site["test"] or "",
            ])
            sites.append(site)
    return sites


# ---- part 4: tests of stored results -----------------------------------

# Members (and one Monkey local) that hold a creator's descriptor: the
# timers' results (assigned from the call or from its plain-name holder) and
# the channels' slots. A later comparison of one with a constant is a test of
# the stored result; each needs a reading in sites.json `stored`, which states
# the holder's absent value and the class (a lexical name match can name a
# member no creator writes: such a reading says `outside`).
STORED = (
    r"fd_timer|frame_fd|flush_fd|shutdown_fd|timer_fd|window\s*\.\s*fd_hop|window\s*\.\s*fd|timeout_fd"
    r"|(?:ch_events|ch_parent_events|ch_thread_events|notification_channels|ch_manager"
    r"|ch_self_events|ch_notif|control_channel|lib_ch_manager|lib_ch_start)\s*\[\s*[01]\s*\]"
    r"|signal_channel_[rw]"
)
STORED_TEST = re.compile(
    r"(?<![\w.>])(?:[A-Za-z_]\w*\s*(?:->|\.)\s*)*(?:" + STORED + r")\s*(?:==|!=|<=|>=|<|>)\s*-?\s*\d+\b"
    r"|(?<![\w.])-?\d+\s*(?:==|!=|<=|>=|<|>)\s*(?:[A-Za-z_]\w*\s*(?:->|\.)\s*)*(?:" + STORED + r")"
)


def read_stored(rev, root, files_read):
    found = []
    base = pathlib.Path(root)
    for rel, digest in sorted(files_read.items()):
        if digest == "unreadable" or not rel.endswith(SUFFIXES):
            continue
        if "/mk_core/" in "/" + rel and pathlib.PurePosixPath(rel).name.startswith("mk_event"):
            continue
        text = (base / rel).read_bytes().decode("utf-8", "replace")
        if not re.search(STORED, text):
            continue
        code = blank(text)
        spans = functions(code)
        for m in STORED_TEST.finditer(code):
            fname, _, _ = enclosing(spans, m.start())
            test = norm(m.group(0))
            found.append({
                "file": rel, "line": line_of(code, m.start()), "function": fname,
                "test": test, "key": f"{rel}|{fname}|{test}",
            })
    return found


def tree_digest(files_read):
    h = hashlib.sha256()
    for rel in sorted(files_read):
        h.update(f"{rel}\0{files_read[rel]}\n".encode())
    return h.hexdigest()


# ---- the census --------------------------------------------------------

def build(roots):
    bodies_reading = json.loads(BODIES.read_text())
    site_reading = json.loads(SITES.read_text())
    problems = []
    used_bodies, used_sites, used_stored = set(), set(), set()
    trees = {}
    for rev, root in roots:
        files_read = {}
        bodies = read_bodies(rev, root, files_read)
        by_key = {}
        for body in bodies:
            key = f"{body['file']}:{body['function']}@{body['sha256'][:16]}"
            body["reading"] = key
            reading = bodies_reading["bodies"].get(key)
            if reading is None:
                problems.append(f"{rev}: unread body {body['file']}:{body['function']}#{body['ordinal']} ({key})")
                continue
            used_bodies.add(key)
            check_reading(body, reading, problems)
            by_key[(body["file"], body["function"], body["ordinal"])] = reading
        # Part 2: the creators' result sets per backend (the helper the
        # public body returns; the configured arm for the timer).
        results = {}
        for creator in CREATORS:
            public = by_key.get(("mk_event.c", creator, 0))
            if public is None:
                problems.append(f"{rev}: no reading of {creator} in mk_event.c")
                continue
            if [r["expr"].split("(")[0] for r in public["returns"]] != [f"_{creator}"]:
                problems.append(f"{rev}: {creator} does not forward one helper call")
            arm = bodies_reading["configured_arm"].get(creator, {})
            for backend in BACKENDS:
                helper = by_key.get((f"mk_event_{backend}.c", f"_{creator}", arm.get(backend, 0)))
                if helper is None:
                    problems.append(f"{rev}: no reading of _{creator} in {backend}")
                    continue
                results.setdefault(creator, {})[backend] = helper["result"]
        configured = "libevent" if rev.startswith("monkey-") else "epoll"
        sites = read_sites(rev, root, files_read)
        stored = read_stored(rev, root, files_read)
        for test in stored:
            reading = site_reading["stored"].get(test["key"])
            if reading is None:
                problems.append(f"{rev}: unannotated stored-result test {test['key']} ({test['file']}:{test['line']})")
                continue
            used_stored.add(test["key"])
            test["class"] = reading["class"]
            test["reading"] = reading["reading"]
        for site in sites:
            reading = site_reading["sites"].get(site["key"])
            if reading is None:
                problems.append(f"{rev}: unannotated site {site['key']} ({site['file']}:{site['line']})")
                continue
            used_sites.add(site["key"])
            site["reading"] = reading["reading"]
            if site["holder_kind"] in ("returned", "discarded") or site["test"] is None:
                site["class"] = {"libc": "untested", "body_only": "untested"}
                continue
            if site["parsed"] is None:
                site["class"] = {"libc": reading.get("class", "unparsed"),
                                 "body_only": reading.get("class", "unparsed")}
                if "class" not in reading:
                    problems.append(f"{rev}: unparsed test needs a stated class: {site['key']}")
                continue
            op, k = site["parsed"]
            site["parsed"] = [op, k]
            site["failure_edge"] = reading["failure_edge"]
            res = results[site["callee"]][reading.get("backend", configured)]
            site["class"] = {}
            for which in ("libc", "body_only"):
                cls, detail = classify(op, k, reading["failure_edge"], site["callee"],
                                       parse_set(res[which]))
                site["class"][which] = cls
                if detail:
                    site["class"][which + "_detail"] = detail
            # Rule 38 today: a status witness (`== 0` / `!= 0` guard) is
            # decided only by a null test, a descriptor witness (`!= -1` /
            # `== -1`) only by a validity test (W43's facts table).
            status = CONTRACT[site["callee"]][1] == 0
            site["decides_today"] = (k == 0 and op in ("==", "!=")) if status else (
                (op, k) in {("==", -1), ("!=", -1), ("<", 0), (">=", 0), ("<=", -1), (">", -1)})
        trees[rev] = {
            "configured_backend": configured,
            "tree_sha256": tree_digest(files_read),
            "files_read": len(files_read),
            "event_files": {f: files_read[monkey_prefix(rev) + "mk_core/" + f] for f in EVENT_FILES},
            "bodies": [{k: v for k, v in b.items() if k != "returns"} for b in bodies],
            "result_sets": results,
            "sites": sites,
            "site_files": {s["file"]: files_read[s["file"]] for s in sites + stored},
            "stored_result_tests": stored,
        }
    for key in bodies_reading["bodies"]:
        if key not in used_bodies:
            problems.append(f"unused body reading {key}")
    for key in site_reading["stored"]:
        if key not in used_stored:
            problems.append(f"unused stored-result reading {key}")
    for key in site_reading["sites"]:
        if key not in used_sites:
            problems.append(f"unused site annotation {key}")
    tally = {}
    for rev, tree in trees.items():
        for s in tree["sites"]:
            for which in ("libc", "body_only"):
                c = tally.setdefault(rev, {}).setdefault(which, {})
                cls = s.get("class", {}).get(which, "unannotated")
                c[cls] = c.get(cls, 0) + 1
        for s in tree["stored_result_tests"]:
            c = tally.setdefault(rev, {}).setdefault("stored", {})
            c[s.get("class", "unannotated")] = c.get(s.get("class", "unannotated"), 0) + 1
    return {
        "schema": "cell-guard-result-range-census/1",
        "contract_failure_class": {k: f"#result {op} {v}" for k, (op, v) in CONTRACT.items()},
        "trees": trees,
        "tally": tally,
    }, problems


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", action="append", required=True, help="REV=PATH")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--write")
    g.add_argument("--check")
    args = ap.parse_args()
    roots = [tuple(r.split("=", 1)) for r in args.root]
    census, problems = build(roots)
    text = json.dumps(census, indent=1, sort_keys=True) + "\n"
    if args.write:
        pathlib.Path(args.write).write_text(text)
    else:
        if pathlib.Path(args.check).read_text() != text:
            problems.append(f"{args.check} differs from the recomputed census")
    for p in problems:
        print("problem:", p)
    print(f"{len(problems)} problems")
    for rev, t in census["tally"].items():
        print(rev, json.dumps(t, sort_keys=True))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
