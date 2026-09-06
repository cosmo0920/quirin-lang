#!/usr/bin/env python3
"""Create a separate synthetic C codebase and all data needed by qcore-audit."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path(sys.argv[1]).resolve()
root.mkdir(parents=True, exist_ok=False)
checkout = root / "checkout"
(checkout / "src").mkdir(parents=True)
sources = {
    "balanced.c": "extern void *alloc(void); extern void dispose(void *p);\nvoid balanced(void) { void *p = alloc(); dispose(p); }\n",
    "leak.c": "extern void *alloc(void);\nvoid leak(void) { void *p = alloc(); }\n",
    "opaque.c": "extern void opaque(void);\nvoid unknown_effect(void) { opaque(); }\n",
}
for name, source in sources.items():
    (checkout / "src" / name).write_text(source)
for args in [
    ["init", "--quiet"],
    ["add", "src"],
    ["-c", "user.name=Audit Demo", "-c", "user.email=audit@example.invalid", "commit", "--quiet", "-m", "Pin synthetic resource example"],
]:
    subprocess.run(["git", "-C", str(checkout), *args], check=True)
commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
(root / "memory.qcore").write_text("""module contract;
extern fn alloc() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};
extern fn dispose(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
};
""")

def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")

write(root / "contracts.json", {"schema_version": 1, "modules": ["memory.qcore"]})
library = {
    "pin": "sample", "source_roots": [{"root": "src", "selection": "top_level"}],
    "audited_files": [{"path": "src/" + name, "sha256": hashlib.sha256(source.encode()).hexdigest()} for name, source in sources.items()],
    "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
    "api_dispositions": {}, "accepted_builds": [], "required_evidence": [], "typed_import": None,
    "callback_summaries": {}, "boolean_field_models": [], "provider": None,
}
write(root / "profiles/sample/profile.json", {
    "schema_version": 1, "source_set": "sample",
    "pins": {"sample": {"git_commit": commit, "clean_paths": [], "description": "Synthetic resource projection demo"}},
    "libraries": {"sample": library}, "resource_contract_manifest": "../../contracts.json", "provider_manifests": [],
})
write(root / "compile_commands.json", [
    {"directory": str(checkout), "file": "src/" + name, "arguments": ["cc", "-std=c11", "-c", "src/" + name, "-o", name + ".o"]} for name in sources
])
write(root / "audit.json", {
    "schema_version": 1, "profile_directory": "profiles/sample", "checkout": "checkout",
    "compile_database": "compile_commands.json", "libraries": ["sample"], "report": "output/report.json",
})
print(root / "audit.json")
