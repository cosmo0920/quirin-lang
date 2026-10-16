#!/usr/bin/env python3
"""Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: classify every access and
identity leaf of the chain census (chain_blockers_census.py's output, taken
from an `indirect-census` run so each access leaf carries the gate's own view).

Classes (the brief's):

* (a) an access a member-path reading already handles elsewhere but not here;
* (b) a missing declaration;
* (c) a real analysis gap that needs a rule or projection change;
* (d) an identity lost through a list or an id lookup.

An identity leaf whose name an earlier refusal of the same body poisoned is
not a blocker of its own: it is attributed to that refusal's class and kind
(`cascade`), so it clears exactly when its root does.

Access kinds are decided from the census view (tag, member class, base
cause) and the statement; identity kinds from the name's binding in the
function's own source text. Every leaf the rules below do not cover lands in
`unclassified`, which must be empty: such a leaf is read by hand and added to
OVERRIDES with its reason.

usage: classify.py RECORDS.jsonl SOURCE_ROOT CHAIN.json OUT.json
"""
import collections
import json
import re
import sys

# kind -> (class, meaning)
KINDS = {
    "fn-pointer-member-record": ("a", "member of a record the lexical reader refuses whole because one member is a function pointer (RECORD-BODY-TOTAL-OR-REJECT); the member-path and typed readings admit the same access on any record the reader reads"),
    "fn-pointer-parameter-function": ("a", "every declaration of a function is unread because one formal is a function pointer (the declaration reader's ParameterListToken refusal); the typed readings admit the same accesses in any function whose declarations are read"),
    "callback-member-call": ("c", "a call through a function-pointer member (rules 12n/13u: callback member closure), or a test of one; no reader change admits it"),
    "logging-macro": ("c", "Fluent Bit's logging macros: a nested call (flb_input_name, flb_input_plugin_log_suppress_check) or an array member (`char name[32]`) inside a variadic logging call or its condition (CG05 nested-call effects, array-member decay)"),
    "errno": ("c", "`flb_errno()` reads errno through `*__errno_location()` (CG05-CALL-ROOTED-PLACES)"),
    "pointer-store-into-member": ("c", "a pointer stored into a member of a caller's object or of a fresh object (`config->evl = evl`, `sched->config = config`, `ins->ht_log_chunks = NULL`): the projection keeps a stored formal pointer or a store into a caller's record refused (W45-SUB-OBJECT-STORES, C00)"),
    "member-release": ("c", "a release or close of an object or descriptor held in a member of a formal outside rule W45-CALLEE-SUMMARY-CASCADE's destructor premises P1-P4 (flb_sds_destroy(ins->alias), cmt_destroy(ins->cmt), close of a member-held descriptor)"),
    "member-descriptor-slots": ("c", "addresses of a member's descriptor array elements handed to mk_event_channel_create / flb_pipe_create (`&ins->ch_events[0]`): descriptors written into a caller's memory have no identity (C04.6)"),
    "uncontracted-call-with-member": ("b", "a member actual of a call to a callee with no contract in force (pthread_mutex_lock/unlock, memcpy of a thread id)"),
    "interior-address-of-looked-up-record": ("d", "the interior address or member of a record obtained by an id lookup or list iteration (`get_collector`, `mk_list_foreach`)"),
    # identity kinds
    "list-or-lookup": ("d", "the name is bound by a list iteration, an id lookup or an event-loop iteration"),
    "void-thread-argument": ("c", "the name is a `void *` thread or callback argument cast to a record pointer: no caller identity crosses pthread_create / mk_utils_worker_spawn"),
    "uncontracted-result": ("b", "the name is bound to the result of a call with no contract in force on this route"),
    "tls-get": ("c", "the name is read from thread-local storage (FLB_TLS_GET)"),
    "formal-may-bind": ("c", "a store or pointer read through a formal the body itself may bind: it releases it (`flb_free(thi)`, `flb_free(ctx)`) or links or unlinks it (`mk_list_add/del(&req->_head, ...)`), so the typed readings' never-bound premise fails and a store through the caller-held record has no identity (C00, store direction)"),
    "formal-handed-to-uncontracted-call": ("b", "a formal handed whole to a callee with no contract or validated summary on this route (another unit's function, or a same-unit function whose own row is Unknown), which poisons it for every later use"),
    "nested-or-condition-call": ("c", "a nested call in an actual or a call in a branch condition (CG05 nested-call effects, rule 10y)"),
    "after-retention-undecided": ("c", "a use of the interior a refused retention named (rule 12t `retention-undecided`): it clears only with the lifetime model (FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME, S3)"),
    "pointer-read-from-member": ("c", "a pointer loaded from a member of a formal (`thi = ins->thi`) names no identity the projection tracks; its null test and uses refuse"),
}

# Read by hand (the line, the statement and the leaves before it in the
# body). (function, origin line, leaf kind) -> (kind or `cascade:KIND`, note)
OVERRIDES = {
    ("collector_create", 2250, "identity"): ("cascade:fn-pointer-parameter-function", "`ins` is written by the refused `coll->instance = ins` (2246) of a function whose declarations are unread"),
    ("input_thread_instance_create", 241, "access"): ("pointer-store-into-member", "`thi->config = ins->config`: a pointer read from a formal stored into the fresh record"),
    ("input_thread_instance_create", 240, "identity"): ("pointer-store-into-member", "`thi->ins = ins`: the formal stored into the fresh record"),
    ("input_thread_instance_create", 244, "identity"): ("cascade:pointer-store-into-member", "`thi` poisoned by the refused stores at 240-241"),
    ("input_thread_instance_create", 256, "identity"): ("cascade:pointer-store-into-member", "as 244"),
    ("input_thread_instance_create", 303, "identity"): ("cascade:pointer-store-into-member", "a store into the formal `ins` of a value read through the poisoned `thi`"),
    ("flb_create", 181, "access"): ("member-descriptor-slots", "`flb_pipe_create(config->ch_data)`: the member array of descriptors as an out actual"),
    ("flb_destroy", 237, "access"): ("formal-may-bind", "`mk_event_del(ctx->event_loop, ctx->event_channel)`: pointer members of `ctx`, which the body frees at 258"),
    ("flb_destroy", 246, "identity"): ("cascade:formal-may-bind", "as 237"),
    ("flb_destroy", 258, "identity"): ("cascade:formal-may-bind", "as 237"),
    ("flb_input_collector_resume", 3246, "identity"): ("cascade:interior-address-of-looked-up-record", "`event = &coll->event` (3238) refused, `coll` from get_collector"),
    ("flb_input_collector_resume", 3247, "identity"): ("cascade:interior-address-of-looked-up-record", "as 3246"),
    ("flb_input_collector_resume", 3257, "identity"): ("cascade:interior-address-of-looked-up-record", "as 3246 (`event` and `coll`)"),
    ("flb_input_collector_resume", 3258, "identity"): ("cascade:interior-address-of-looked-up-record", "as 3246"),
    ("flb_input_collector_resume", 3259, "identity"): ("cascade:interior-address-of-looked-up-record", "as 3246"),
    ("flb_input_exit_all", 2155, "identity"): ("cascade:list-or-lookup", "`p = ins->p` with `ins` from mk_list_entry"),
    ("flb_input_thread_instance_exit", 587, "identity"): ("pointer-read-from-member", "`struct flb_input_thread_instance *thi = ins->thi;` then `if (thi == NULL)`"),
    ("flb_input_thread_instance_exit", 601, "identity"): ("pointer-read-from-member", "as 587"),
    ("input_thread", 333, "identity"): ("void-thread-argument", "`thi = (struct flb_input_thread_instance *) data;` in the thread entry"),
    ("input_thread", 334, "identity"): ("cascade:void-thread-argument", "`ins = thi->ins` through the unresolved `thi`"),
    ("input_thread", 335, "identity"): ("cascade:void-thread-argument", "`p = ins->p` through the unresolved `ins`"),
    ("input_thread", 462, "identity"): ("list-or-lookup", "`connection = (struct flb_connection *) event;` with `event` from the loop's ready list"),
    ("input_thread", 463, "identity"): ("list-or-lookup", "as 462"),
    ("input_thread", 466, "identity"): ("list-or-lookup", "as 462"),
    ("input_thread_instance_destroy", 202, "identity"): ("formal-may-bind", "a store through `thi`, which the body frees at 226"),
    ("input_thread_instance_destroy", 210, "identity"): ("formal-may-bind", "as 202"),
    ("input_thread_instance_destroy", 213, "identity"): ("formal-may-bind", "as 202"),
    ("input_thread_instance_destroy", 218, "identity"): ("formal-may-bind", "as 202"),
    ("input_thread_instance_destroy", 221, "identity"): ("formal-may-bind", "as 202"),
    ("input_thread_instance_destroy", 226, "identity"): ("formal-may-bind", "as 202"),
    # system route (046accf6)
    ("flb_engine_start", 1327, "access"): ("formal-handed-to-uncontracted-call", "`event = &config->event_flush`: an interior of `config`, which the body hands whole to uncontracted callees (flb_engine_started, flb_engine_shutdown, ...)"),
    ("flb_engine_start", 1328, "identity"): ("cascade:formal-handed-to-uncontracted-call", "`event` from 1327"),
    ("flb_engine_start", 1329, "identity"): ("cascade:formal-handed-to-uncontracted-call", "as 1328"),
    ("flb_engine_start", 1483, "access"): ("formal-handed-to-uncontracted-call", "`event = &config->event_shutdown`, as 1327"),
    ("flb_engine_start", 1484, "identity"): ("cascade:formal-handed-to-uncontracted-call", "`event` from 1483"),
    ("flb_engine_start", 1485, "identity"): ("cascade:formal-handed-to-uncontracted-call", "as 1484"),
    ("flb_engine_start", 1486, "identity"): ("cascade:formal-handed-to-uncontracted-call", "as 1484"),
    ("flb_engine_start", 1564, "access"): ("formal-handed-to-uncontracted-call", "`&config->event_shutdown` handed to mk_event_timeout_destroy, as 1327 (the typed call actual refuses on the bound root)"),
    **{("flb_engine_start", line, "identity"): ("list-or-lookup", "`event` bound by flb_event_priority_live_foreach over the engine loop (1454)")
       for line in (1455, 1572, 1576, 1580, 1589, 1592, 1613, 1620, 1624, 1625, 1629)},
    **{("flb_engine_start", line, "identity"): ("list-or-lookup", "`connection = (struct flb_connection *) event` with `event` from the loop's ready list")
       for line in (1602, 1603, 1605)},
    **{("flb_sched_request_create", line, "identity"): ("cascade:fn-pointer-member-record", "`request` is read by the refused `timer->data = request` (290)")
       for line in (303, 304, 305)},
    **{("flb_sched_retry_now", line, "identity"): ("cascade:fn-pointer-member-record", "`request` is read by the refused `timer->data = request` (975)")
       for line in (979, 980, 981)},
    ("flb_sched_timer_coro_cb_create", 714, "identity"): ("uncontracted-result", "`timer` is written through `&timer` by flb_sched_timer_cb_create, whose own row is Unknown (no validated summary)"),
    ("schedule_request_now", 134, "identity"): ("after-retention-undecided", "`event = &timer->event` is retained by mk_event_timeout_create into `config->evl` at 133 (rule 12t retention-undecided)"),
    ("schedule_request_now", 146, "identity"): ("after-retention-undecided", "as 134"),
}

REFUSED_PAREN = None


def body_of(text, function):
    # A definition: the name after a declaration-specifier run that starts a
    # line, its parameter list, then `{` (a comment or a call never matches).
    m = re.search(r"(?m)^(?:[A-Za-z_][\w]*[ \t\*]+)+" + re.escape(function)
                  + r"\s*\([^;{]*?\)\s*\{", text)
    if not m:
        return None, None
    name_at = m.start() + m.group(0).rindex(function + "(") if (function + "(") in m.group(0) \
        else m.start() + m.group(0).index(function)
    depth = 0
    for i in range(m.end() - 1, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text.count("\n", 0, name_at) + 1, text[name_at:i + 1]
    return None, None


def formals_of(body):
    """The formal names of a definition whose text starts at its name."""
    if not body:
        return set()
    open_at = body.index("(")
    depth = 0
    for i in range(open_at, len(body)):
        if body[i] == "(":
            depth += 1
        elif body[i] == ")":
            depth -= 1
            if depth == 0:
                break
    names = set()
    group, depth = "", 0
    for ch in body[open_at + 1:i] + ",":
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            found = re.findall(r"\(\s*\*\s*(\w+)\s*\)|(\w+)\s*(?:\[[^]]*\])?\s*$", group.strip())
            for a, b in found:
                if a or b:
                    names.add(a or b)
            group = ""
        else:
            group += ch
    names.discard("void")
    return names


def access_kind(leaf, refused_tags, fn_members, formals=frozenset()):
    source = leaf.get("source") or ""
    view = leaf.get("census")
    if re.search(r"\bflb_errno\s*\(", source):
        return "errno"
    if re.search(r"\bflb_(plg_)?(error|warn|info|debug|trace)\s*\(", source) or \
            re.search(r"\bsnprintf\s*\(.*->name", source):
        return "logging-macro"
    if view is None:
        if re.search(r"->\s*cb_\w+\s*&&|->\s*cb_\w+\s*\)", source):
            return "callback-member-call"
        if re.search(r"flb_input_collector_pause\s*\(", source):
            return "uncontracted-call-with-member"
        return None
    chains = view["chains"]
    if any(c["member"] in fn_members.get(c["tag"] or "", set()) for c in chains):
        return "callback-member-call"
    store = re.match(r"\s*\w+\s*->\s*\w+\s*=\s*(.*?);", source)
    if view["position"] == "AssignmentDestination" and store and (
            store.group(1).strip() in formals or "->" in store.group(1)):
        return "pointer-store-into-member"
    causes = [c["base_cause"] for c in chains]
    if any(isinstance(cause, dict) and "FunctionRefused" in cause and
           cause["FunctionRefused"].get("ParameterListToken") == "(" for cause in causes):
        return "fn-pointer-parameter-function"
    if re.search(r"\b(pthread_mutex_(un)?lock|memcpy)\s*\(", source):
        return "uncontracted-call-with-member"
    if re.search(r"\b(flb_sds_destroy|cmt_destroy|flb_free|mk_event_closesocket)\s*\(", source) and \
            "mk_event_del" not in source:
        return "member-release"
    if re.search(r"mk_event_channel_create|flb_pipe_create", source) and any(
            c["member_class"] == "NoMember" for c in chains) and not any(
            c["tag"] in refused_tags and c["member_class"] == "Unresolved" for c in chains):
        return "member-descriptor-slots"
    if re.match(r"\s*if\s*\(\s*\w+\s*\(", source) or re.search(r"\w\s*\(\s*\w+\s*\(", source):
        return "nested-or-condition-call"
    if any(c["tag"] in refused_tags and c["member_class"] == "Unresolved" for c in chains):
        if all(c["base_kind"] == "AutomaticLocal" for c in chains
               if c["tag"] in refused_tags) and leaf.get("looked_up"):
            return "interior-address-of-looked-up-record"
        return "fn-pointer-member-record"
    if view["position"] == "AssignmentDestination":
        return "pointer-store-into-member"
    if any(c["tag"] in refused_tags for c in chains):
        return "fn-pointer-member-record"
    return None


LOOKUP = re.compile(r"(mk_list_entry|mk_list_foreach|get_collector|flb_event_priority_live_foreach|mk_event_foreach|container_of)")


def identity_kind(name, body_lines, line_index, access_bases, uncontracted, formals=frozenset()):
    # The binding of `name` before the leaf.
    before = body_lines[:line_index]
    joined = "\n".join(before)
    whole = "\n".join(body_lines)
    if re.search(r"FLB_TLS_GET\s*\(\s*" + re.escape(name) + r"\s*\)", whole):
        return "tls-get"
    if re.search(r"\b" + re.escape(name) + r"\s*=\s*\(?\s*(struct\s+\w+\s*\*\s*)?\)?\s*FLB_TLS_GET", joined):
        return "tls-get"
    for text in reversed(before):
        if re.search(r"(mk_list_foreach\w*|flb_event_priority_live_foreach|mk_event_foreach)\s*\(\s*" + re.escape(name) + r"\b", text):
            return "list-or-lookup"
        m = re.search(r"\b" + re.escape(name) + r"\s*=\s*(.*)", text)
        if m and "==" not in text[m.start():m.start() + len(name) + 3]:
            rhs = m.group(1)
            if LOOKUP.search(rhs):
                return "list-or-lookup"
            if re.match(r"\(\s*struct\s+\w+\s*\*\s*\)\s*(data|arg)\b|(data|arg)\s*;", rhs):
                return "void-thread-argument"
            call = re.match(r"(\w+)\s*\(", rhs)
            if call and call.group(1) in uncontracted:
                return ("uncontracted-result", call.group(1))
            break
    # An earlier refusal of this body that names it poisons it first.
    if name in access_bases:
        return "cascade"
    if name in formals:
        # The typed readings' premise is body-wide (`bound_names` over the
        # whole body), so a hand-off after the leaf counts too.
        for callee in sorted(uncontracted):
            if re.search(r"\b" + re.escape(callee) + r"\s*\([^;]*\b" + re.escape(name) + r"\s*[,)]", whole):
                return ("formal-handed-to-uncontracted-call", callee)
        if re.search(r"\b(flb_free|free)\s*\(\s*" + re.escape(name) + r"\s*\)", whole) or \
                re.search(r"mk_list_(add|del)\w*\s*\(\s*&\s*" + re.escape(name) + r"\s*->", whole):
            return "formal-may-bind"
    return None


def main():
    records_path, root, chain_path, out = sys.argv[1:5]
    refused_tags = set()
    for raw in open(records_path):
        r = json.loads(raw)
        if r["refused_on"] == "(":
            refused_tags.add(r["tag"])
    chain = json.load(open(chain_path))
    texts = {}
    leaves_out = []
    counts = collections.Counter()
    rows_by_class = collections.defaultdict(set)
    for row in chain["rows"]:
        unit = row["unit"]
        if unit not in texts:
            texts[unit] = open(f"{root}/{unit}", errors="replace").read()
        first, body = body_of(texts[unit], row["function"])
        body_lines = body.split("\n") if body else []
        formals = formals_of(body)
        fn_members = {"flb_input_plugin": {"cb_exit", "cb_init", "cb_pre_run", "cb_pre_exit", "cb_collect"},
                      "mk_event": {"handler"}, "flb_input_collector": {"cb_collect"},
                      "flb_sched_timer": {"cb"}}
        uncontracted = set()
        for description in row["other"]:
            m = re.match(r"call to uncontracted function `(\w+)`", description)
            if m:
                uncontracted.add(m.group(1))
        looked_up = set()
        for text in body_lines:
            m = re.search(r"\b(\w+)\s*=\s*(get_collector|mk_list_entry)\b", text)
            if m:
                looked_up.add(m.group(1))
            m = re.search(r"mk_list_foreach\w*\s*\(\s*(\w+)", text)
            if m:
                looked_up.add(m.group(1))
        access_bases = {}
        resolved = []
        for leaf in row["leaves"]:
            line = int(leaf["origin"].rsplit(":", 1)[1])
            index = line - first if first else None
            key = (row["function"], line, leaf["kind"])
            note = None
            if key in OVERRIDES:
                kind, note = OVERRIDES[key]
                if kind.startswith("cascade:"):
                    kind = ("cascade", kind.split(":", 1)[1])
                elif leaf["kind"] == "access":
                    for c in (leaf.get("census") or {}).get("chains", []):
                        access_bases.setdefault(c["base"], kind)
            elif leaf["kind"] == "access":
                bases = [c["base"] for c in (leaf.get("census") or {}).get("chains", [])]
                leaf = dict(leaf, looked_up=any(b in looked_up for b in bases))
                kind = access_kind(leaf, refused_tags, fn_members, formals)
                for b in bases:
                    access_bases.setdefault(b, kind)
                # The destination of a refused `NAME = ...` is poisoned too.
                dest = re.match(r"\s*(\w+)\s*=[^=]", leaf.get("source") or "")
                if dest:
                    access_bases.setdefault(dest.group(1), kind)
            else:
                name = leaf["name"]
                kind = identity_kind(name, body_lines, index, access_bases, uncontracted, formals) \
                    if index is not None else None
                if kind == "cascade":
                    root_kind = access_bases.get(name)
                    note = f"cascade of {root_kind}"
                    kind = ("cascade", root_kind)
                elif isinstance(kind, tuple):
                    note = f"{'handed to' if kind[0].startswith('formal') else 'result of'} {kind[1]}"
                    kind = kind[0]
                elif kind is None and name in looked_up:
                    kind = "list-or-lookup"
                elif kind is None and name in access_bases:
                    note = f"cascade of {access_bases[name]}"
                    kind = ("cascade", access_bases[name])
            if isinstance(kind, tuple):
                cls = KINDS.get(kind[1], ("?", ""))[0] if kind[1] else "?"
                label = f"cascade:{kind[1]}"
            elif kind:
                cls = KINDS[kind][0]
                label = kind
            else:
                cls, label = "unclassified", "unclassified"
            resolved.append({
                "function": row["function"], "unit": unit, "unit_index": row["unit_index"],
                "leaf": leaf["kind"], "origin": leaf["origin"], "name": leaf.get("name"),
                "source": leaf.get("source"), "kind": label, "class": cls, "note": note,
                # A leaf whose own kind is another class but whose chain also
                # ends in a member of a record refused for a function-pointer
                # member: that reading must land too before it can clear.
                "also_behind_fn_pointer_record": label != "fn-pointer-member-record" and any(
                    c["tag"] in refused_tags and c["member_class"] == "Unresolved"
                    for c in (leaf.get("census") or {}).get("chains", [])),
                "also_behind_fn_pointer_parameter": label != "fn-pointer-parameter-function" and any(
                    isinstance(c["base_cause"], dict) and "FunctionRefused" in c["base_cause"]
                    for c in (leaf.get("census") or {}).get("chains", [])),
            })
            counts[(leaf["kind"], cls, label)] += 1
            rows_by_class[cls].add((unit, row["unit_index"], row["function"]))
        leaves_out.extend(resolved)
    summary = collections.defaultdict(lambda: {"access": 0, "identity": 0, "rows": 0})
    for (leaf_kind, cls, label), n in counts.items():
        summary[cls][leaf_kind] += n
    for cls, rows in rows_by_class.items():
        summary[cls]["rows"] = len(rows)
    result = {
        "card": "FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS",
        "chain": chain_path,
        "kinds": {k: {"class": v[0], "meaning": v[1]} for k, v in KINDS.items()},
        "refused_paren_tags": sorted(refused_tags),
        "by_class": dict(sorted(summary.items())),
        "by_kind": [
            {"leaf": k[0], "class": k[1], "kind": k[2], "leaves": n}
            for k, n in sorted(counts.items())
        ],
        "unclassified": [l for l in leaves_out if l["class"] in ("unclassified", "?")],
        "leaves": leaves_out,
    }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(json.dumps(result["by_class"]))
    for item in result["by_kind"]:
        print(item)
    print("unclassified:", len(result["unclassified"]))


if __name__ == "__main__":
    main()
