#!/usr/bin/env python3
"""frames.py SAMPLES.jsonl... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME: for each
sampled run, the busy thread-samples (a stack holding a project frame other
than the coordinator's wait) and how many hold each frame of interest (the
readers the card touched and their hooks), inclusively, as counts and shares."""
import json
import re
import sys

PROJECT = re.compile(r"^(<)?(qcore_|quirin_)")
FRAMES = [
    "elaboration_node", "node_tokens_once", "::node_tokens", "get_or_compute",
    "LineTree", "line_tree", "call_exit_census", "call_exits::resolve",
    "unit_declarations", "values_from_slice", "deep_unit_ast", "translation_unit_ast_with",
    "typed_access_path::node_line", "tokenize_with_conditionals", "definition_ast",
    "appended_declarations", "launch_declarations", "drop_in_place<serde_json::value::Value>",
    "malloc", "free",
]


def main():
    for path in sys.argv[1:]:
        busy = 0
        counts = dict.fromkeys(FRAMES, 0)
        for line in open(path):
            for thread in json.loads(line)["threads"]:
                stack = thread["stack"] if isinstance(thread, dict) else thread
                project = [f for f in stack if PROJECT.search(f)]
                if not project or "parallel::run_ordered" in project[0] or project[0].startswith("quirin_audit::main"):
                    continue
                busy += 1
                joined = "\n".join(stack)
                for frame in FRAMES:
                    if frame in joined:
                        counts[frame] += 1
        print(path, "busy", busy)
        for frame, count in counts.items():
            if count:
                print(f"   {frame}: {count} ({100 * count / busy:.1f} %)")


if __name__ == "__main__":
    main()
