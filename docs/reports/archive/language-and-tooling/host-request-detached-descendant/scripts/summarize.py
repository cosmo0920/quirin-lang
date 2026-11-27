#!/usr/bin/env python3
"""summarize.py DIR [SIDE...] -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT (from
HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's): one table row per (side,
condition) from campaign.sh's loadbisect JSON files: runs, and per probe case
how many runs deviated from the case's required outcome. This card's nine
cases get a column each; the earlier card's nine are summed in one column.
The base binary's assertions are the base-compatible ones (refusing cases
assert "not Completed, exit 0"), the tip's are exact
(`RequestHeldByDescendant` / `RequestUnconsumed` / `Completed`)."""
import collections
import json
import pathlib
import sys

CASES = [
    ("a_descendant_holding_an_unread_request_is_refused", "held silent"),
    ("a_detached_descendant_holding_an_unread_request_is_refused", "held detached"),
    ("a_descendant_holding_an_undelivered_large_request_is_refused", "held large"),
    ("a_descendant_that_reads_the_request_and_keeps_holding_it_is_refused", "held after reading"),
    ("a_descendant_that_holds_an_unread_request_briefly_is_refused", "held briefly"),
    ("a_descendant_that_reads_the_whole_request_after_the_exit_is_accepted", "descendant reads"),
    ("a_descendant_that_reads_the_whole_request_late_is_accepted", "reads late"),
    ("a_descendant_that_reads_a_whole_large_request_is_accepted", "reads large"),
    ("a_background_descendant_with_stdin_from_dev_null_is_accepted", "background /dev/null"),
]
EARLIER = [
    "a_provider_that_never_reads_a_small_request_is_refused",
    "a_provider_that_never_reads_a_small_request_and_exits_late_is_refused",
    "a_provider_that_closes_its_input_and_answers_is_refused",
    "a_provider_that_reads_part_of_its_request_is_refused",
    "a_provider_that_never_reads_a_large_request_is_refused",
    "a_provider_that_reads_its_request_to_end_of_file_is_accepted",
    "a_provider_that_reads_a_large_request_to_end_of_file_is_accepted",
    "a_provider_that_reads_exactly_its_frame_and_exits_before_the_close_is_accepted",
    "a_provider_that_reads_exactly_a_large_frame_is_accepted",
]
CONDITIONS = [("stall", "{stall} ms host stall"), ("write", "first write +30 ms"),
              ("close", "close +30 ms"), ("both", "both delays + 20 ms stall")]


def label(kind: str, level: dict) -> str:
    stall = level["stall_ms"]
    if kind == "stall":
        return "idle" if not stall else f"{stall} ms host stall"
    return dict(CONDITIONS)[kind].format(stall=stall)


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    sides = sys.argv[2:] or ["base", "after"]
    print("| Binary | Condition | Runs | " + " | ".join(name for _, name in CASES)
          + " | earlier nine |")
    print("|---|---|---:|" + "---:|" * (len(CASES) + 1))
    total = collections.Counter()
    for side in sides:
        for kind, _ in CONDITIONS:
            path = root / f"{side}-{kind}.json"
            if not path.exists():
                continue
            data = json.loads(path.read_text())
            for level in data["levels"]:
                runs = level["runs"]
                failed = collections.Counter(
                    failure["test"] for run in runs for failure in run["failures"]
                )
                unparsed = sum(1 for run in runs if run["exit"] != 0 and not run["failures"])
                cells = [str(failed.get(test, 0)) for test, _ in CASES]
                cells.append(str(sum(failed.get(test, 0) for test in EARLIER)))
                total[side, "runs"] += len(runs)
                total[side, "deviations"] += sum(failed.values()) + unparsed
                extra = f" (+{unparsed} runs failed without a parsed test)" if unparsed else ""
                print(f"| {side} `{data['binary_sha256'][:8]}` | {label(kind, level)}{extra} | "
                      f"{len(runs)} | " + " | ".join(cells) + " |")
    for side in sides:
        print(f"\n{side}: {total[side, 'runs']} runs, {total[side, 'deviations']} deviating case-runs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
