#!/usr/bin/env python3
"""FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: read-only source census.

Reads prepared Fluent Bit exports only (never the user's checkout, never git):
the system route's 046accf6 export (whole tree: src/ and plugins/) and the
tail-pause slice's 90978ac6 export (its 13 selected files). For every

* collector registration (`flb_input_set_collector_{time,event,socket,server}(`),
* `mk_event_add(` / `mk_event_timeout_create(` site,
* timer-callback registration (`flb_sched_timer_{,coro_}cb_create(`),

and, for the lifetime reading, every removal (`mk_event_del`,
`mk_event_timeout_destroy`, `mk_event_channel_destroy`), channel creation,
wait, loop creation/destruction and store into an `evl` member, it records the
site, the enclosing function, the actuals as spelled and how the loop
(holder) actual and the registered (retained) actual resolve under the
holder-path reading of decision 1 (`FORMAL.FIELD[.FIELD]`):

* `formal`            the holder is a formal of the enclosing function;
* `formal-path`       a formal followed by a member path (`config->evl`):
                      spelled `config.evl`, resolvable at the call site;
* `local-of-formal-path` a local whose one write is a formal path;
* `local-iterated`    a local bound by list iteration (`mk_list_entry`,
                      `mk_list_foreach`) or a lookup by id: no path to a formal;
* `local-other`       any other local (several writes, a call's result, ...);
* `global`/`call`/`other`.

The registered actual is classified the same way, with `&X->f` read as the
interior `f` of `X`. A match is a site, not a finding. Output:
artifacts/loop-census.json.
"""
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
SYSTEM = ("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/"
          "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source")
SLICE = ("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/"
         "fluent-bit-clang-source")
SLICE_FILES = [
    "src/flb_input.c", "src/flb_input_thread.c", "src/flb_lib.c",
    "plugins/in_tail/tail.c", "plugins/in_tail/tail_config.c", "plugins/in_tail/tail_db.c",
    "plugins/in_tail/tail_dockermode.c", "plugins/in_tail/tail_file.c",
    "plugins/in_tail/tail_file_budget.c", "plugins/in_tail/tail_fs_inotify.c",
    "plugins/in_tail/tail_fs_stat.c", "plugins/in_tail/tail_multiline.c",
    "plugins/in_tail/tail_scan.c",
]
SYSTEM_FILES = [
    "plugins/in_storage_backlog/sb.c", "src/flb_input_chunk.c", "src/flb_storage.c",
    "src/flb_task.c", "src/flb_engine.c", "src/flb_engine_dispatch.c",
    "src/flb_scheduler.c", "src/flb_output.c",
]

GROUPS = {
    "collector": re.compile(r"\bflb_input_set_collector_(time|event|socket|server)\s*\("),
    "mk_event_add_or_timeout_create": re.compile(r"\bmk_event_(add|timeout_create)\s*\("),
    "timer": re.compile(r"\bflb_sched_timer_(?:coro_)?cb_create\s*\("),
    "removal": re.compile(r"\bmk_event_(del|timeout_destroy|channel_destroy|timeout_disable)\s*\("),
    "channel_create": re.compile(r"\bmk_event_channel_create\s*\("),
    "wait": re.compile(r"\bmk_event_wait(?:_2)?\s*\("),
    "loop_lifetime": re.compile(r"\bmk_event_loop_(create|destroy)\s*\("),
}
# The loop (holder) and registered (retained) positions of each operation.
POSITIONS = {
    "mk_event_add": (0, 4), "mk_event_timeout_create": (0, 3), "mk_event_del": (0, 1),
    "mk_event_timeout_destroy": (0, 1), "mk_event_timeout_disable": (0, 1),
    "mk_event_channel_create": (0, 3), "mk_event_channel_destroy": (0, 3),
    "mk_event_wait": (0, None), "mk_event_wait_2": (0, None),
    "mk_event_loop_destroy": (0, None), "mk_event_loop_create": (None, None),
}
EVL_STORE = re.compile(r"(?:\b\w+(?:->|\.))+(?:evl|ch_evl|event_loop)\s*=(?!=)")


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def blank(text):
    """Comments and string/char literals blanked, newlines kept."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(c + "".join(ch if ch == "\n" else " " for ch in text[i + 1:j - 1]) + c)
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def preprocessor_blank(text):
    """Directive lines blanked, and every `#elif` / `#else` arm blanked so
    the first arm alone gives the brace structure (a structural reading for
    locating functions, not a configuration)."""
    out = []
    stack = []  # per open group: whether its current arm is kept
    for line in text.split("\n"):
        directive = line.lstrip()
        if directive.startswith("#"):
            word = directive[1:].lstrip().split(None, 1)[0] if directive[1:].strip() else ""
            if word.startswith("if"):
                stack.append(True)
            elif word in ("elif", "else") and stack:
                stack[-1] = False
            elif word == "endif" and stack:
                stack.pop()
            out.append(" " * len(line))
        elif all(stack):
            out.append(line)
        else:
            out.append(" " * len(line))
    return "\n".join(out)


FUNC_HEAD = re.compile(r"(\w+)\s*\(([^;{}]*)\)\s*(?:__attribute__\s*\(\(.*?\)\)\s*)*$", re.S)


def functions(clean):
    """[(name, params, body_start, body_end)] for top-level definitions."""
    out = []
    depth = 0
    buffer_start = 0
    i = 0
    while i < len(clean):
        c = clean[i]
        if c == "{":
            if depth == 0:
                head = clean[buffer_start:i].strip()
                m = FUNC_HEAD.search(head)
                if m and m.group(1) not in ("if", "while", "for", "switch") and "=" not in head.split("(")[0]:
                    params = parameter_names(m.group(2))
                    end = match_brace(clean, i)
                    out.append((m.group(1), params, i, end))
                    i = end
                    buffer_start = end + 1
                    i += 1
                    continue
            depth += 1
        elif c == "}":
            depth = max(0, depth - 1)
            if depth == 0:
                buffer_start = i + 1
        elif c == ";" and depth == 0:
            buffer_start = i + 1
        i += 1
    return out


def match_brace(clean, i):
    depth = 0
    for j in range(i, len(clean)):
        if clean[j] == "{":
            depth += 1
        elif clean[j] == "}":
            depth -= 1
            if depth == 0:
                return j
    return len(clean) - 1


def parameter_names(text):
    names = []
    for part in split_top(text):
        part = part.strip()
        if not part or part == "void" or part == "...":
            continue
        fp = re.search(r"\(\s*\*\s*(\w+)\s*\)", part)
        if fp:
            names.append(fp.group(1))
            continue
        m = re.search(r"(\w+)\s*(?:\[[^\]]*\])*\s*$", part)
        if m:
            names.append(m.group(1))
    return names


def split_top(text):
    parts, depth, start = [], 0, 0
    for i, c in enumerate(text):
        if c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif c == "," and depth == 0:
            parts.append(text[start:i])
            start = i + 1
    parts.append(text[start:])
    return parts


def call_actuals(clean, open_paren):
    depth = 0
    for j in range(open_paren, len(clean)):
        if clean[j] == "(":
            depth += 1
        elif clean[j] == ")":
            depth -= 1
            if depth == 0:
                return [" ".join(a.split()) for a in split_top(clean[open_paren + 1:j])]
    return None


PATH = re.compile(r"^\(?\s*(?:\(\s*[\w\s\*]+\)\s*)?(&\s*)?(\w+)((?:\s*(?:->|\.)\s*\w+)*)\s*\)?$")


def classify(actual, params, body):
    if actual is None:
        return {"class": "none"}
    a = actual.strip()
    m = PATH.match(a)
    if not m:
        if re.match(r"^\w+\s*\(", a):
            return {"class": "call", "spelled": a}
        return {"class": "other", "spelled": a}
    amp, root, chain = m.group(1), m.group(2), m.group(3)
    fields = [f for f in re.split(r"\s*(?:->|\.)\s*", chain.strip()) if f]
    path = ".".join([root] + fields)
    kind = "interior" if amp and fields else ("address" if amp else "value")
    if root in params:
        cls = "formal-path" if fields else "formal"
        return {"class": cls, "path": path, "form": kind}
    if root == "NULL":
        return {"class": "null"}
    writes = local_writes(body, root)
    if writes is None:
        return {"class": "global", "path": path, "form": kind}
    rhs = [w for w in writes if w is not None]
    if any(re.search(r"\bmk_list_entry\b|\bcfl_list_entry\b|\bget_collector\b|\bflb_input_collector_get\b", w)
           for w in rhs):
        return {"class": "local-iterated", "path": path, "form": kind, "writes": rhs[:4]}
    if len(writes) == 1 and rhs:
        inner = classify(rhs[0], params, body) if rhs[0] != a else {"class": "other"}
        if inner["class"] in ("formal-path", "formal"):
            return {"class": "local-of-formal-path", "path": path, "form": kind,
                    "local_value": inner["path"] + ("" if inner.get("form") == "value" else f" ({inner['form']})")}
        if inner["class"] == "call":
            return {"class": "local-call-result", "path": path, "form": kind, "writes": rhs[:4]}
    return {"class": "local-other", "path": path, "form": kind, "writes": rhs[:4]}


def local_writes(body, name):
    """Writes to a local `name` in body: declaration initialisers and plain
    assignments; None when body declares no local of that name."""
    declared = re.search(r"(?:^|[;{}(,])\s*(?:const\s+)?(?:struct\s+\w+|unsigned\s+\w+|\w+)[\s\*]+" +
                         re.escape(name) + r"\s*(=|;|,|\[)", body)
    if not declared:
        return None
    writes = []
    for m in re.finditer(r"(?<![\w>.])" + re.escape(name) + r"\s*=(?!=)\s*([^;]*);", body):
        writes.append(" ".join(m.group(1).split()))
    if declared.group(1) == "=" and not writes:
        writes.append(None)
    return writes


def scan(root, files, label):
    sites = {k: [] for k in GROUPS}
    evl_stores = []
    digests = {}
    for rel in files:
        path = os.path.join(root, rel)
        if not os.path.exists(path):
            continue
        digests[rel] = sha256(path)
        with open(path, encoding="utf-8", errors="replace") as f:
            raw = f.read()
        blanked = blank(raw)
        # Sites are counted in every arm (directive lines blanked only); the
        # functions they stand in are located over the first arm alone.
        clean = "\n".join(" " * len(line) if line.lstrip().startswith("#") else line
                           for line in blanked.split("\n"))
        funcs = functions(preprocessor_blank(blanked))
        line_of = lambda pos: clean.count("\n", 0, pos) + 1
        for group, rx in GROUPS.items():
            for m in rx.finditer(clean):
                pos = m.start()
                enclosing = next(((n, p, s, e) for (n, p, s, e) in funcs if s < pos < e), None)
                callee = m.group(0).split("(")[0].strip()
                entry = {"site": f"{rel}:{line_of(pos)}", "callee": callee}
                if enclosing is None:
                    entry["kind"] = "declaration-or-definition"
                    sites[group].append(entry)
                    continue
                name, params, start, end = enclosing
                body = clean[start:end]
                actuals = call_actuals(clean, clean.index("(", m.start()))
                entry.update({"function": name, "parameters": params, "actuals": actuals})
                if group in ("collector", "timer"):
                    entry["kind"] = "call"
                    cb_index = 1 if group == "collector" else 3
                    entry["callback"] = actuals[cb_index] if actuals and len(actuals) > cb_index else None
                    entry["callback_class"] = callback_class(entry["callback"], raw)
                    entry["holder_actual"] = classify(actuals[0] if actuals else None, params, body)
                    if group == "timer":
                        entry["registered_actual"] = classify(actuals[4] if actuals and len(actuals) > 4 else None, params, body)
                    entry["loop_resolution"] = (
                        "id-forwarded: collector_create stores coll->evl := ins->thi->evl "
                        "(threaded) | config->evl; retained at collector_start through coll.evl"
                        if group == "collector" else
                        "sched.evl inside flb_sched_timer_cb_create (sched->evl := evl at "
                        "flb_sched_create); event &timer->event, timer published in sched->timers")
                else:
                    entry["kind"] = "call"
                    loop_at, data_at = POSITIONS.get(callee, (None, None))
                    if loop_at is not None and actuals:
                        entry["holder_actual"] = classify(actuals[loop_at], params, body)
                    if data_at is not None and actuals and len(actuals) > data_at:
                        entry["registered_actual"] = classify(actuals[data_at], params, body)
                sites[group].append(entry)
        for m in EVL_STORE.finditer(clean):
            pos = m.start()
            enclosing = next(((n, p, s, e) for (n, p, s, e) in funcs if s < pos < e), None)
            stmt = clean[pos:clean.find(";", pos)]
            evl_stores.append({"site": f"{rel}:{line_of(pos)}", "function": enclosing[0] if enclosing else None,
                               "store": " ".join(stmt.split())})
    for group in sites:
        sites[group].sort(key=lambda e: e["site"])
    return {"label": label, "file_sha256": digests, "sites": sites, "evl_stores": evl_stores}


def callback_class(actual, raw):
    if actual is None:
        return None
    a = actual.strip()
    if a == "NULL":
        return "null"
    if re.fullmatch(r"\w+", a):
        if re.search(r"^(?:static\s+)?(?:inline\s+)?[\w\s\*]+\b" + re.escape(a) + r"\s*\(", raw, re.M):
            return "function-designator (in file)"
        return "identifier (not defined in file)"
    return "other"


def tree_files(root):
    out = []
    for sub in ("src", "plugins"):
        for d, _, names in os.walk(os.path.join(root, sub)):
            for name in names:
                if name.endswith(".c"):
                    out.append(os.path.relpath(os.path.join(d, name), root))
    return sorted(out)


def summarise(scan_result):
    summary = {}
    for group, entries in scan_result["sites"].items():
        calls = [e for e in entries if e["kind"] == "call"]
        decls = [e for e in entries if e["kind"] != "call"]
        holder = {}
        retained = {}
        for e in calls:
            h = e.get("holder_actual", {}).get("class", "n/a")
            holder[h] = holder.get(h, 0) + 1
            r = e.get("registered_actual", {})
            if r:
                key = r.get("class", "n/a") + ("/" + r["form"] if r.get("form") else "")
                retained[key] = retained.get(key, 0) + 1
        summary[group] = {
            "matches": len(entries),
            "calls": len(calls),
            "declarations_or_definitions": len(decls),
            "files": len({e["site"].rsplit(":", 1)[0] for e in entries}),
            "functions": len({(e["site"].rsplit(":", 1)[0], e.get("function")) for e in calls}),
            "holder_actual": dict(sorted(holder.items())),
            "registered_actual": dict(sorted(retained.items())),
        }
    return summary


def main():
    tree = scan(SYSTEM, tree_files(SYSTEM), "system export 046accf6, src/ and plugins/")
    system = scan(SYSTEM, SYSTEM_FILES, "system route files, 046accf6")
    slice_ = scan(SLICE, SLICE_FILES, "tail-pause slice files, 90978ac6 export")
    out = {
        "card": "FB-EVENT-LOOP-RETENTION-AND-REGISTRATION",
        "kind": "read-only source census; sites, not findings",
        "sources": {"system": SYSTEM, "slice": SLICE},
        "tree": {"summary": summarise(tree), "sites": tree["sites"], "evl_stores": tree["evl_stores"]},
        "system_route": {"summary": summarise(system), "sites": system["sites"],
                         "evl_stores": system["evl_stores"], "file_sha256": system["file_sha256"]},
        "slice_route": {"summary": summarise(slice_), "sites": slice_["sites"],
                        "evl_stores": slice_["evl_stores"], "file_sha256": slice_["file_sha256"]},
    }
    dest = os.path.join(REPORT, "artifacts", "loop-census.json")
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
        f.write("\n")
    for label in ("tree", "system_route", "slice_route"):
        print("==", label)
        for group, s in out[label]["summary"].items():
            print(f"  {group}: {s}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
