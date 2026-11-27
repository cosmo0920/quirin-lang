"""Unit-scoped reproductions of the tracked-object rows (and two mutants)."""
import os

here = os.path.dirname(os.path.abspath(__file__))
current = open(os.path.join(here, "rdregex.current.i")).read()
assert current.count("regfree(&re);") == 1
mutants = {
    "mutant-void-member": "regfree(&re); (void) re.re_nsub;",
    "mutant-cond-member": "regfree(&re); if (re.re_nsub) r = 2;",
    "mutant-void-peek": "regfree(&re); (void) regexec(&re, str, 0, 0, 0);",
}
for name, text in mutants.items():
    with open(os.path.join(here, f"rdregex.current.{name}.i"), "w") as f:
        f.write(current.replace("regfree(&re);", text))
rows = []
for function in ["rd_regex_match", "rd_regex_exec", "rd_regex_comp", "rd_regex_destroy"]:
    for tree in ["current", "corpus"]:
        rows.append((f"unit/{tree}/{function}", f"{here}/rdregex.{tree}.i|{function}"))
for name in mutants:
    rows.append((f"unit/{name}/rd_regex_match", f"{here}/rdregex.current.{name}.i|rd_regex_match"))
with open(os.path.join(here, "units.tsv"), "w") as f:
    for label, body in rows:
        f.write(f"{label}\tunit\t{body}\n")
print(len(rows), "rows")
