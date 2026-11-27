# Resume FB-AUDIT-02-RETRY-LIFETIME

Bounded source review and the isolated C1 branch experiment are complete.
Read README.md, evidence.json and followups.json. Source review is pinned to
Fluent Bit 864c3d97; the historical build database belongs separately to 66910c10.
Run `python3 verify_evidence.py` here to check eight files, 63 line anchors,
four build-copy hashes and the archived extracted function.

The original agent ran in /tmp/fb-retry-card; its logs and experiment outputs
were archived under artifacts/ (executable omitted). build_c1_harness.py
recreates the C from the pinned exact function; compile/run commands are in README.
Use a fresh worktree-private directory for new experiment outputs rather than
overwriting archived artifacts. No full-binary fault injection was performed.
Next: reproduce C1 in the real scheduler, then C2 queue-node allocation failure;
C3/C4 need the stated queue/lifetime reachability witnesses. No semantic card is
closed. Combined standing-gate state is in the parent validation.json.
