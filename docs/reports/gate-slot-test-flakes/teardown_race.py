#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Mode (d) in isolation (card GATE-SLOT-TEST-FLAKES): runs
``test_gate_slot_queue.Stale.test_a_live_owner_without_a_lock_is_not_reclaimed``
from a given ``scripts/tests`` directory and reports, at the moment its
tearDown removes the scratch gate, which job threads are still alive and how
long tearDown took.

    python3 docs/reports/gate-slot-test-flakes/teardown_race.py <scripts/tests dir>
"""
from pathlib import Path
import sys
import time
import unittest

tests = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(tests.parent))
sys.path.insert(0, str(tests))
import test_gate_slot_queue as queue  # noqa: E402

seen = {}
original_cleanup = None


def setUp(self):
    queue.Queue._orig_setUp(self)
    cleanup = self.scratch.cleanup

    def watched():
        seen["alive_at_cleanup"] = [j.name for j in self.jobs if j.thread.is_alive()]
        seen["teardown_seconds"] = round(time.monotonic() - seen["teardown_started"], 2)
        cleanup()
    self.scratch.cleanup = watched


def tearDown(self):
    seen["teardown_started"] = time.monotonic()
    queue.Queue._orig_tearDown(self)


queue.Queue._orig_setUp, queue.Queue.setUp = queue.Queue.setUp, setUp
queue.Queue._orig_tearDown, queue.Queue.tearDown = queue.Queue.tearDown, tearDown
suite = unittest.TestSuite([queue.Stale("test_a_live_owner_without_a_lock_is_not_reclaimed")])
result = unittest.TextTestRunner(verbosity=0).run(suite)
print(f"{tests}: {seen}; errors={len(result.errors)} failures={len(result.failures)}")
