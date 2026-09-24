"""Card W43-INDIRECT-ACCESS-GENERAL: a Python mirror of the planned typed
scalar-access classifier, used only to FORECAST the admission before it is
implemented. It reads the census labels (the declaration reader's own answers,
streamed by the `indirect-census` feature) rather than any second reader.

Where the census does not carry a fact the Rust rule reads, the mirror says so
in `MIRROR_GAPS` instead of guessing silently.

`classify(record)` returns `None` (not admitted) or a dict
`{"shape": ..., "bases": [...], "destination": name-or-None}`.
"""

import re

MIRROR_GAPS = [
    "a record-pointer base's member class is read from the census chain for that base; "
    "a statement naming the same base twice with different members uses the first chain",
]

PUNCTUATION = {"(", ")", "=", "+", "-", "/", "%", "<", ">", "!", "~", "^", "|", "?", ":", "*", "&"}
COMPOUND = {"+", "-", "*", "/", "%", "&", "|", "^"}


def is_name(text):
    return bool(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", text))


def is_literal(text):
    return text == "__qcore_literal__" or text[:1].isdigit()


class Facts:
    def __init__(self, record):
        self.declared = record.get("declared", {})
        self.members = {}
        for chain in record.get("chains", []):
            if chain.get("base") and chain.get("member") and len(chain.get("steps", [])) == 1:
                self.members.setdefault((chain["base"], chain["member"]), chain["member_class"])

    def label(self, name):
        return self.declared.get(name)

    def kind(self, name):
        label = self.label(name) or ""
        return label.split("|")[0]

    def element(self, name):
        label = self.label(name) or ""
        match = re.search(r"\|element:(\w+)", label)
        return match.group(1) if match else None

    def scalar(self, name):
        return self.kind(name) == "scalar"

    def enum(self, name):
        return self.kind(name) == "enum_constant"

    def scalar_type(self, name):
        return self.kind(name) == "scalar_type_name" or name in {
            "char", "short", "int", "long", "signed", "unsigned", "float", "double", "_Bool", "const",
        }


def matching(texts, start, open_, close):
    depth = 0
    for index in range(start, len(texts)):
        if texts[index] == open_:
            depth += 1
        elif texts[index] == close:
            depth -= 1
            if depth == 0:
                return index
    return None


def index_is_scalar(texts, facts):
    """The existing zero-access scalar walker: declared scalars, enum
    constants, numeric literals and the admitted punctuation (no `*`, `&`)."""
    if not texts:
        return False
    for text in texts:
        if is_name(text):
            if not (facts.scalar(text) or facts.enum(text)):
                return False
        elif text[:1].isdigit():
            continue
        elif text not in {"(", ")", "=", "+", "-", "/", "%", "<", ">", "!", "~", "^", "|", "?", ":"}:
            return False
    return True


def access_at(texts, index, facts):
    """Parse one typed scalar access starting at `index` in operand position.
    Returns (end_index_exclusive, base_or_None, is_dereference) or None."""
    text = texts[index]
    if text == "*":
        if index + 1 >= len(texts):
            return None
        name = texts[index + 1]
        if not is_name(name) or facts.element(name) != "Pointer":
            return None
        after = texts[index + 2] if index + 2 < len(texts) else None
        if after in ("[", "(", ".") or (after == "-" and index + 3 < len(texts) and texts[index + 3] in (">", "-")):
            return None
        if after == "+" and index + 3 < len(texts) and texts[index + 3] == "+":
            return None
        return index + 2, name, True, "*p"
    if not is_name(text):
        return None
    nxt = texts[index + 1] if index + 1 < len(texts) else None
    if nxt == "-" and index + 2 < len(texts) and texts[index + 2] == ">":
        if index + 3 >= len(texts) or not is_name(texts[index + 3]):
            return None
        if not facts.kind(text).startswith("record_pointer"):
            return None
        if facts.members.get((text, texts[index + 3])) != "Scalar":
            return None
        end = index + 4
        if end < len(texts) and (texts[end] in (".", "[", "(") or (texts[end] == "-" and end + 1 < len(texts) and texts[end + 1] == ">")):
            return None
        return end, text, True, "p->f"
    if nxt == ".":
        if index + 2 >= len(texts) or not is_name(texts[index + 2]):
            return None
        if not facts.kind(text).startswith("record:"):
            return None
        if facts.members.get((text, texts[index + 2])) != "Scalar":
            return None
        end = index + 3
        if end < len(texts) and (texts[end] in (".", "[", "(") or (texts[end] == "-" and end + 1 < len(texts) and texts[end + 1] == ">")):
            return None
        return end, None, False, "s.f"
    if nxt == "[":
        if facts.element(text) not in ("Pointer", "Array"):
            return None
        close = matching(texts, index + 1, "[", "]")
        if close is None or not index_is_scalar(texts[index + 2:close], facts):
            return None
        end = close + 1
        if end < len(texts) and (texts[end] in (".", "[", "(") or (texts[end] == "-" and end + 1 < len(texts) and texts[end + 1] == ">")):
            return None
        return end, text, True, "p[i]"
    return None


def expression(texts, facts, bases, accesses):
    """Walk a scalar expression that may contain typed scalar accesses."""
    expect_operand = True
    depth = 0
    index = 0
    while index < len(texts):
        text = texts[index]
        if expect_operand:
            if text == "(":
                close = matching(texts, index, "(", ")")
                inner = texts[index + 1:close] if close is not None else []
                if close is not None and inner and all(is_name(t) and facts.scalar_type(t) for t in inner):
                    index = close + 1
                    continue
                depth += 1
                index += 1
                continue
            if text in ("-", "+", "!", "~"):
                if index + 1 < len(texts) and texts[index + 1] == text and text in "+-":
                    return False
                index += 1
                continue
            if is_literal(text):
                expect_operand = False
                index += 1
                continue
            parsed = access_at(texts, index, facts)
            if parsed is not None:
                end, base, _, kind = parsed
                accesses.append(kind)
                if base is not None and base not in bases:
                    bases.append(base)
                index = end
                expect_operand = False
                continue
            if is_name(text):
                if index + 1 < len(texts) and texts[index + 1] == "(":
                    return False
                if facts.scalar(text) or facts.enum(text):
                    index += 1
                    expect_operand = False
                    continue
            return False
        # after an operand
        if text == ")":
            if depth == 0:
                return False
            depth -= 1
            index += 1
            continue
        pair = texts[index:index + 2]
        if pair in (["+", "+"], ["-", "-"]):
            return False
        if pair in (["=", "="], ["!", "="], ["<", "="], [">", "="], ["<", "<"], [">", ">"], ["&", "&"], ["|", "|"]):
            index += 2
            expect_operand = True
            continue
        if text in ("+", "-", "*", "/", "%", "<", ">", "^", "|", "&", "?", ":"):
            index += 1
            expect_operand = True
            continue
        return False
    return not expect_operand and depth == 0


def strip_parens(tokens):
    while len(tokens) >= 2 and tokens[0] == "(" and matching(tokens, 0, "(", ")") == len(tokens) - 1:
        tokens = tokens[1:-1]
    return tokens


def single_access(tokens, facts):
    tokens = strip_parens(list(tokens))
    if not tokens:
        return None
    parsed = access_at(tokens, 0, facts)
    if parsed is None or parsed[0] != len(tokens):
        return None
    return parsed


def assignment(texts):
    depth = 0
    for index, text in enumerate(texts):
        if text in ("(", "["):
            depth += 1
        elif text in (")", "]"):
            depth -= 1
        elif text == "=" and depth == 0:
            nxt = texts[index + 1] if index + 1 < len(texts) else ""
            prev = texts[index - 1] if index else ""
            if nxt == "=" or prev in ("=", "!"):
                return None
            if prev in ("<", ">"):
                if index >= 2 and texts[index - 2] == prev:
                    return index, 2
                return None
            if prev in COMPOUND:
                return index, 1
            return index, 0
    return None


def classify(record):
    facts = Facts(record)
    texts = list(record["texts"])
    if not texts or any(t in ("{", "}", ";", ",") for t in texts):
        return None
    found = assignment(texts)
    if found is None:
        # `++ ACCESS`, `-- ACCESS`, `ACCESS ++`, `( ACCESS ) ++`
        if texts[:2] in (["+", "+"], ["-", "-"]):
            parsed = single_access(texts[2:], facts)
            if parsed and parsed[2]:
                return {"shape": "increment:" + parsed[3], "bases": [parsed[1]], "destination": None, "accesses": [parsed[3]]}
            return None
        if texts[-2:] in (["+", "+"], ["-", "-"]):
            body = texts[:-2]
            if body and body[0] == "*":
                return None  # `*p++` increments the pointer
            parsed = single_access(body, facts)
            if parsed and parsed[2]:
                return {"shape": "increment:" + parsed[3], "bases": [parsed[1]], "destination": None, "accesses": [parsed[3]]}
        return None
    equal, operator = found
    left = texts[: equal - operator]
    right = texts[equal + 1:]
    bases, accesses = [], []
    destination = None
    parsed = single_access(left, facts)
    if parsed is not None:
        end, base, _, kind = parsed
        accesses.append(kind)
        if base is not None:
            bases.append(base)
        shape = ("compound-store:" if operator else "store:") + kind
    else:
        # A declared scalar name, optionally after scalar type specifiers.
        if not left or not is_name(left[-1]) or not facts.scalar(left[-1]):
            return None
        if left[:-1] and not all(is_name(t) and facts.scalar_type(t) for t in left[:-1]):
            return None
        if left[:-1] and operator:
            return None
        destination = left[-1]
        shape = "compound-into-scalar" if operator else "read-into-scalar"
    if not expression(right, facts, bases, accesses):
        return None
    if not accesses:
        return None
    return {"shape": shape, "bases": bases, "destination": destination, "accesses": accesses}
