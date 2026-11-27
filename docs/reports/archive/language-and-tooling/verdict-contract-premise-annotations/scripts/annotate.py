#!/usr/bin/env python3
"""annotate.py REPO [--check] -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS: attach
each premise the card found to exactly the declarations it covers, as
`requires resource_premise(NAME, "STATEMENT");` (contract specification
C04.18), inserted after the declaration's last contract-level marker (the
resource-v1 selector, or the libc modules' `resource_revision(flexible)`), and
add one comment line under the comment that states it. Nothing else in a
declaration changes. Idempotent refusal: a declaration that already carries
the clause, or an anchor that is not found exactly once, fails the run.
`--check` only verifies that every premise is present exactly once.

Every statement is the comment's own claim, restated so a reviewer can check
it against the cited source: no backslash, no double quote, no control
character (the loader's rule)."""
import pathlib
import re
import sys

FB = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/"
RK = "providers/qcore-provider-librdkafka/contracts/librdkafka/"
LIBC = "providers/quirin-provider-libc/contracts/libc/"

ASSERTED = ("asserted_allocation",
            "the result is non-null because it is allocated with {how} (rd.h:{lines}), which "
            "asserts its result, and the audited compile commands define no NDEBUG, so it never "
            "returns NULL in the audited build")

# (module, function, name, statement)
PREMISES = [
    (FB + "output_singleplex.qv", "flb_output_task_singleplex_enqueue", "synchronous_output_reachability",
     "in-tree output plugins never set FLB_OUTPUT_SYNCHRONOUS, so this function runs in production only "
     "for an output (a proxy plugin) that does; any verdict on it holds under that premise"),
    (FB + "loop_owner.qv", "flb_input_collector_resume", "ingestion_active",
     "C3: the call returns 0 without resuming the collector while config->is_ingestion_active is false "
     "(src/flb_input.c:3221 at 66910c10, :3241 at 046accf6 and 34155955), otherwise it re-registers the "
     "collector's own event and sets running; a return of 0 means the collector runs only while "
     "ingestion is active"),
    (FB + "input_chunk_name.qv", "flb_input_chunk_get_name", "untracked_handback",
     "the result aliases into ic (it is ic->chunk->name) and this projection does not track it; "
     "resource_return_none is faithful only where the result is never bound or stored, which the "
     "gateway's admission of a call with a None result and only Borrowed parameters checks per site "
     "(card O04-TRACE premises N6 and N8); the alias itself is outside every claim here "
     "(O00 B03/R13, O04-TRACE-HANDBACK)"),
    (RK + "cjson.qv", "cJSON_New_Item", "cjson_allocator_hooks",
     "hooks->allocate returns a fresh allocation or NULL, which cJSON_Delete releases through the "
     "matching deallocate hook: malloc and free in global_hooks (cJSON.c:178, :186) unless "
     "cJSON_InitHooks installed a user pair, a premise of whoever installed it; nothing is stated "
     "about the hooks changing between the allocation and its release"),
    (RK + "cjson.qv", "add_item_to_array", "cjson_list_invariant",
     "cJSON's list invariant, kept by every cJSON list operation: a non-empty array's child->prev is "
     "its last element, never NULL; if a caller broke it, the child->prev == NULL arm returns true "
     "without linking item, which this declaration would read as a transfer"),
    (RK + "client.qv", "rd_kafka_toppar_new0", ASSERTED[0],
     ASSERTED[1].format(how="rd_calloc", lines="139-143")),
    (RK + "client.qv", "rd_kafka_toppar_desired_add", ASSERTED[0],
     "the result is non-null because its last source is rd_kafka_toppar_new0's reference, allocated "
     "with rd_calloc (rd.h:139-143), which asserts its result, and the audited compile commands define "
     "no NDEBUG, so it never returns NULL in the audited build"),
    (RK + "client.qv", "rd_kafka_q_new0", ASSERTED[0],
     ASSERTED[1].format(how="rd_malloc", lines="145-149")),
    (RK + "client.qv", "ut_create_mock_queue", ASSERTED[0],
     ASSERTED[1].format(how="rd_calloc", lines="139-143")),
    (LIBC + "errno.qv", "__errno_location", "nested_call_admission",
     "a dereference of this call's result is read only where the gateway's nested-call admission "
     "holds at the call site (premises N1-N10 of cards O04-TRACE and CONSTRUCTOR-RESIDUALS, checked "
     "per site): the call is the statement's only nested call and a whole argument of a contracted "
     "call at a declared Borrowed position modulo the dereference wrapper (N4-D), its written arity "
     "is this declaration's, and the unit's own declaration of the callee returns exactly one pointer "
     "level over a scalar type (N10); under them the argument binds, releases, escapes and addresses "
     "nothing the projection tracks"),
    (LIBC + "errno.qv", "strerror", "nested_call_admission",
     "a call of this function nested whole in another call's argument is read only where card "
     "O04-TRACE's nested-call admission holds at the call site; this declaration meets its premises "
     "N3 (a contract in force), N6 (result mode None) and N7 (every parameter Borrowed), and the "
     "admission checks the rest per site"),
]

NOTE = ("Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: stated on {who} as "
        "`requires resource_premise({name}, \"...\")` (contract spec C04.18), so every "
        "verdict that rests on {it} names it.")


def note(who, name, it):
    import textwrap
    return textwrap.fill(NOTE.format(who=who, name=name, it=it), width=79,
                         initial_indent="// ", subsequent_indent="// ",
                         break_on_hyphens=False, break_long_words=False) + "\n"

# (module, anchor line in the comment, who, name, it): a note after the anchor.
NOTES = [
    (FB + "output_singleplex.qv", "// premise.\n", "`flb_output_task_singleplex_enqueue`",
     "synchronous_output_reachability", "the declaration"),
    (FB + "loop_owner.qv", "//   `running` (C3's ingestion premise);\n", None, None, None),
    (FB + "input_chunk_name.qv", "// `O04-TRACE-HANDBACK`).\n", "`flb_input_chunk_get_name`",
     "untracked_handback", "the declaration"),
    (RK + "cjson.qv", "// the hooks between an allocation and its release.\n", "`cJSON_New_Item`",
     "cjson_allocator_hooks", "the declaration"),
    (RK + "cjson.qv", "//     linking `item`, which this declaration would read as a transfer.\n",
     None, None, None),
    (RK + "client.qv", "// and `046accf6` and is byte-identical (`scripts/body.py`).\n",
     "the four non-null results below (`rd_kafka_toppar_new0`, `rd_kafka_toppar_desired_add`, "
     "`rd_kafka_q_new0`, `ut_create_mock_queue`)", "asserted_allocation", "one of them"),
    (LIBC + "errno.qv", "// at all, refuses every such site under this declaration exactly as without it.\n",
     "`__errno_location`", "nested_call_admission", "the declaration"),
    (LIBC + "errno.qv", "// `errno` is stated.\n", "`strerror`", "nested_call_admission", "the declaration"),
]

# Notes whose anchor sits inside a bullet list get their own wording.
INLINE = {
    (FB + "loop_owner.qv"): ("//   `running` (C3's ingestion premise);\n",
                             "//   `running` (C3's ingestion premise, stated on the declaration as\n"
                             "//   `requires resource_premise(ingestion_active, \"...\")`, contract spec\n"
                             "//   C04.18, card VERDICT-CONTRACT-PREMISE-ANNOTATIONS);\n"),
    (RK + "cjson.qv"): ("//     linking `item`, which this declaration would read as a transfer.\n",
                        "//     linking `item`, which this declaration would read as a transfer.\n"
                        "//     Stated on the declaration as `requires\n"
                        "//     resource_premise(cjson_list_invariant, \"...\")` (contract spec C04.18,\n"
                        "//     card VERDICT-CONTRACT-PREMISE-ANNOTATIONS).\n"),
}

FORBIDDEN = re.compile(r'["\\]|[\x00-\x1f\x7f]')


def clause(name, statement):
    return f'    requires resource_premise({name}, "{statement}");\n'


def declaration_span(text, function):
    start = text.find(f"extern fn {function}(")
    if start < 0 or text.find(f"extern fn {function}(", start + 1) >= 0:
        raise SystemExit(f"declaration {function}: not found exactly once")
    end = text.index("};", start)
    return start, end


def main():
    repo = pathlib.Path(sys.argv[1])
    check = "--check" in sys.argv
    texts = {}
    for module, function, name, statement in PREMISES:
        if FORBIDDEN.search(statement) or not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            raise SystemExit(f"{function}: statement or name the loader rejects")
        text = texts.setdefault(module, (repo / module).read_text(encoding="utf-8"))
        start, end = declaration_span(text, function)
        body = text[start:end]
        wanted = clause(name, statement)
        present = body.count(f"resource_premise({name},")
        if check:
            if present != 1 or wanted not in body:
                raise SystemExit(f"{function}: premise {name} not present exactly once as written")
            continue
        if present:
            raise SystemExit(f"{function}: already carries {name}")
        lines = body.splitlines(keepends=True)
        last_marker = max(i for i, l in enumerate(lines)
                          if l.strip() in ("requires resource_contract_v1;", "requires resource_revision(flexible);"))
        lines.insert(last_marker + 1, wanted)
        texts[module] = text[:start] + "".join(lines) + text[end:]
    if check:
        print(f"{len(PREMISES)} premises present")
        return
    for module, anchor, who, name, it in NOTES:
        text = texts.setdefault(module, (repo / module).read_text(encoding="utf-8"))
        if text.count(anchor) != 1:
            raise SystemExit(f"{module}: anchor not found exactly once: {anchor!r}")
        if who is None:
            old, new = INLINE[module]
            assert old == anchor
            texts[module] = text.replace(anchor, new)
        else:
            texts[module] = text.replace(anchor, anchor + "//\n" + note(who, name, it))
    for module, text in texts.items():
        (repo / module).write_bytes(text.encode("utf-8"))
        print("wrote", module)


main()
