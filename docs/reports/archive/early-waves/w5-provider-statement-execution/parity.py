"""Check trusted source parsing/evaluation against the pre-extraction commit."""
from pathlib import Path
import subprocess, re, hashlib, json
root = Path(__file__).resolve().parents[3]
relative = "crates/qcore-c-gateway/src/boolean_source.rs"
old = subprocess.check_output(["git", "show", "70fb813:" + relative], cwd=root, text=True)
new = (root / relative).read_text()
def function(source, name):
    start = source.index("fn " + name + "(")
    end = source.index("{", start) + 1
    depth = 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return re.sub(r"\s+", "", re.sub(r"//[^\n]*", "", source[start:end]))
rows = []
for name in ["record_layout", "validate_field_layout", "kind", "children", "evaluate", "execute", "field", "expression", "statement", "field_kind", "import_chunk_frame_helper", "import_chunk_source_with_profile", "input_identity", "derive_summary"]:
    left, right = function(old, name), function(new, name)
    if name == "derive_summary":
        right = right.replace("statement.record(),", "")
    assert left == right, name
    rows.append({"function": name, "sha256": hashlib.sha256(left.encode()).hexdigest()})
output = {"baseline": "70fb813", "normalization": "comments/whitespace; derive_summary only appends captured statement.record()", "functions": rows}
Path(__file__).with_name("statement-parity.json").write_text(json.dumps(output, indent=2) + "\n")
print("Verified", len(rows), "trusted importer/evaluator functions")
