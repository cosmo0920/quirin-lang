# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Slot accounting of the elastic gate (card GATE-ELASTIC)."""
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_slot as slot_mod  # noqa: E402

SCRIPT = Path(__file__).resolve().parents[1] / "gate_slot.py"
GIB_KIB = 1024 * 1024


def meminfo(directory, available_gib, total_gib=60):
    path = Path(directory) / "meminfo"
    path.write_text(f"MemTotal:       {total_gib * GIB_KIB} kB\n"
                    f"MemFree:        1 kB\n"
                    f"MemAvailable:   {available_gib * GIB_KIB} kB\n")
    return path


class Arithmetic(unittest.TestCase):
    def test_budgets_are_binary_mib(self):
        self.assertEqual(slot_mod.parse_budget("7G"), 7168)
        self.assertEqual(slot_mod.parse_budget("5g"), 5120)
        self.assertEqual(slot_mod.parse_budget("4G"), 4096)
        self.assertEqual(slot_mod.DEFAULT_BUDGET, "7G")
        for bad in ("6G", "3G", "7", "7GB", ""):
            with self.assertRaises(slot_mod.Refusal, msg=bad):
                slot_mod.parse_budget(bad)

    def test_audit_slots_are_floor_of_available_over_budget_at_least_one(self):
        # This host's reading at the time of the card: 55,210,764 kB available.
        self.assertEqual(slot_mod.audit_slot_count(55_210_764, 7168), 7)
        self.assertEqual(slot_mod.audit_slot_count(55_210_764, 5120), 10)
        self.assertEqual(slot_mod.audit_slot_count(55_210_764, 4096), 13)
        self.assertEqual(slot_mod.audit_slot_count(14 * GIB_KIB - 1, 7168), 1)
        self.assertEqual(slot_mod.audit_slot_count(14 * GIB_KIB, 7168), 2)
        self.assertEqual(slot_mod.audit_slot_count(1, 7168), 1, "at least one slot")

    def test_the_ceiling_covers_every_budget(self):
        for total in (8, 60, 64, 256):
            ceiling = slot_mod.audit_slot_ceiling(total * GIB_KIB)
            for budget in slot_mod.BUDGETS_MIB.values():
                self.assertLessEqual(slot_mod.audit_slot_count(total * GIB_KIB, budget), ceiling)

    def test_jobs_cap_mirrors_the_audit_planning_budget(self):
        # crates/qcore-audit/src/parallel.rs: (budget - 1024) / 2048, refused below 3072.
        self.assertEqual(slot_mod.audit_jobs_cap(7168), 3)
        self.assertEqual(slot_mod.audit_jobs_cap(5120), 2)
        self.assertEqual(slot_mod.audit_jobs_cap(4096), 1)
        with self.assertRaises(slot_mod.Refusal):
            slot_mod.audit_jobs_cap(3071)

    def test_parallel_rs_still_has_that_formula(self):
        source = (Path(__file__).resolve().parents[2] / "crates/qcore-audit/src/parallel.rs") \
            .read_text(encoding="utf-8")
        self.assertIn("Some(budget) if budget >= 3072 => Ok(requested.min((budget - 1024) / 2048))",
                      source)


class BudgetPassing(unittest.TestCase):
    cwd = Path("/")

    def test_current_route_gets_the_flag(self):
        argv, env, record = slot_mod.apply_budget(
            ["target/release/quirin-audit", "current", "--jobs", "8", "cfg"], {}, 5120, self.cwd)
        # The two-word form, the only one current_cli.rs reads (ledger
        # GATE-SLOT-CURRENT-BUDGET-SPELLING).
        self.assertEqual(argv, ["target/release/quirin-audit", "current",
                                "--memory-budget-mib", "5120", "--jobs", "8", "cfg"])
        self.assertEqual(env["QUIRIN_GATE_MEMORY_BUDGET_MIB"], "5120")
        self.assertNotIn("QCORE_AUDIT_JOBS", env)
        self.assertEqual(record["jobs_cap"], 2)

    def test_current_route_with_a_different_budget_refuses(self):
        with self.assertRaises(slot_mod.Refusal):
            slot_mod.apply_budget(["quirin-audit", "current", "--memory-budget-mib", "7168"],
                                  {}, 4096, self.cwd)
        argv, _, _ = slot_mod.apply_budget(
            ["quirin-audit", "current", "--memory-budget-mib", "4096"], {}, 4096, self.cwd)
        self.assertEqual(argv.count("--memory-budget-mib"), 1)
        # An existing flag is read in either spelling and left as given.
        with self.assertRaises(slot_mod.Refusal):
            slot_mod.apply_budget(["quirin-audit", "current", "--memory-budget-mib=7168"],
                                  {}, 4096, self.cwd)
        argv, _, _ = slot_mod.apply_budget(
            ["quirin-audit", "current", "--memory-budget-mib=4096"], {}, 4096, self.cwd)
        self.assertEqual(argv, ["quirin-audit", "current", "--memory-budget-mib=4096"])

    def test_other_routes_get_a_lowered_job_variable(self):
        _, env, record = slot_mod.apply_budget(
            ["python3", "scripts/run_pinned_audit.py", "out", "abc"],
            {"QCORE_AUDIT_JOBS": "16"}, 7168, self.cwd)
        self.assertEqual(env["QCORE_AUDIT_JOBS"], "3")
        self.assertEqual(record["QCORE_AUDIT_JOBS"], {"before": "16", "after": "3"})
        _, env, _ = slot_mod.apply_budget(["quirin-audit", "pinned"], {"QCORE_AUDIT_JOBS": "1"},
                                          7168, self.cwd)
        self.assertEqual(env["QCORE_AUDIT_JOBS"], "1", "a lower request is kept")

    def test_an_explicit_jobs_flag_above_the_cap_refuses(self):
        with self.assertRaises(slot_mod.Refusal):
            slot_mod.apply_budget(["python3", "scripts/run_pinned_audit.py", "o", "c",
                                   "--jobs", "4"], {}, 7168, self.cwd)
        slot_mod.apply_budget(["quirin-audit", "pinned", "--jobs=3"], {}, 7168, self.cwd)

    def test_configured_route_reads_its_configuration(self):
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / "audit.json"
            config.write_text(json.dumps({"schema_version": 1, "jobs": 8}))
            with self.assertRaises(slot_mod.Refusal):
                slot_mod.apply_budget(["quirin-audit", str(config)], {}, 7168, self.cwd)
            config.write_text(json.dumps({"schema_version": 1, "jobs": 8,
                                          "memory_budget_mib": 7168}))
            _, _, record = slot_mod.apply_budget(["quirin-audit", str(config)], {}, 7168, self.cwd)
            self.assertEqual(record["mechanism"], "configuration memory_budget_mib")
            config.write_text(json.dumps({"schema_version": 1, "memory_budget_mib": 4096}))
            with self.assertRaises(slot_mod.Refusal):
                slot_mod.apply_budget(["quirin-audit", str(config)], {}, 7168, self.cwd)


class RouteAdmission(unittest.TestCase):
    """Ledger CORPUS-PEAK-ABOVE-AUDIT-SLOT-BUDGET: the pinned corpus is admitted
    at its measured budget; its workers stay planned at the slot budget."""

    def test_the_pinned_corpus_is_recognised_behind_wrappers(self):
        wrapped = ["unshare", "-rm", "sh", "-c", "exec \"$@\"", "ns", "python3",
                   "/tmp/quirin-measure/src/scripts/run_pinned_audit.py", "out", "66910c10",
                   "--jobs", "3"]
        self.assertEqual(slot_mod.route_class(wrapped), "pinned-corpus")
        self.assertEqual(slot_mod.route_class(["target/release/quirin-audit", "pinned"]),
                         "pinned-corpus")
        for other in (["quirin-audit", "current", "--checkout", "x"],
                      ["quirin-audit", "config.json"], ["python3", "-c", "pass"]):
            self.assertIsNone(slot_mod.route_class(other), other)

    def test_the_corpus_admission_budget_covers_its_measured_peak(self):
        # CFG-RELEASE's measured process peak at --jobs 3, in bytes.
        self.assertGreater(slot_mod.ROUTE_ADMISSION_MIB["pinned-corpus"] * 1024 * 1024,
                           10_129_997_824)
        corpus = ["python3", "scripts/run_pinned_audit.py", "o", "c"]
        for budget in slot_mod.BUDGETS_MIB.values():
            self.assertEqual(slot_mod.admission_budget(corpus, budget), (11264, "pinned-corpus"))
            self.assertEqual(slot_mod.admission_budget(["quirin-audit", "current"], budget),
                             (budget, None))
        # The workers stay planned at the slot budget, not the admission budget.
        _, env, _ = slot_mod.apply_budget(corpus, {}, 7168, Path("/"))
        self.assertEqual(env["QCORE_AUDIT_JOBS"], "3")


class Accounting(unittest.TestCase):
    """Real flock(2) slots in a scratch gate directory."""

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.root = Path(self.scratch.name)
        self.legacy = self.root / "legacy.lock"
        self.gates = []

    def tearDown(self):
        for gate in self.gates:
            gate.release()
        self.scratch.cleanup()

    def gate(self, available_gib=60, total_gib=60):
        gate = slot_mod.Gate(self.root / "gate", self.legacy,
                             meminfo(self.root, available_gib, total_gib), poll=0.01,
                             echo=lambda line: None)
        self.gates.append(gate)
        return gate

    def blocked(self, cls, budget=None, **kwargs):
        with self.assertRaises(TimeoutError):
            self.gate(**kwargs).acquire(cls, budget, timeout=0.1)

    def test_fixed_classes_have_their_slot_counts(self):
        first = self.gate().acquire("lean", timeout=1)
        second = self.gate().acquire("lean", timeout=1)
        self.assertEqual({first["slot"], second["slot"]}, {0, 1})
        self.assertEqual(first["slot_count"], 2)
        self.blocked("lean")
        self.gate().acquire("rust-suite", timeout=1)
        self.blocked("rust-suite")

    def test_audit_slots_follow_available_memory_and_budget(self):
        # 15 GiB available at 7 GiB per audit: two slots.
        first = self.gate(15).acquire("audit", 7168, timeout=1)
        second = self.gate(15).acquire("audit", 7168, timeout=1)
        self.assertEqual((first["slot_count"], second["slot_count"]), (2, 2))
        self.blocked("audit", 7168, available_gib=15)
        # The same reading at a 4 GiB budget admits a third.
        third = self.gate(15).acquire("audit", 4096, timeout=1)
        self.assertEqual(third["slot_count"], 3)
        self.assertEqual(third["slot"], 2)

    def test_a_corpus_admission_budget_admits_fewer(self):
        # 23 GiB available: three 7 GiB audits, but only two 11 GiB corpus runs.
        corpus = slot_mod.ROUTE_ADMISSION_MIB["pinned-corpus"]
        first = self.gate(23).acquire("audit", corpus, timeout=1)
        second = self.gate(23).acquire("audit", corpus, timeout=1)
        self.assertEqual((first["slot_count"], second["slot_count"]), (2, 2))
        self.blocked("audit", corpus, available_gib=23)
        self.assertEqual(self.gate(23).acquire("audit", 7168, timeout=1)["slot"], 2)

    def test_at_least_one_audit_slot(self):
        self.assertEqual(self.gate(1).acquire("audit", 7168, timeout=1)["slot_count"], 1)
        self.blocked("audit", 7168, available_gib=1)

    def test_the_rust_suite_excludes_every_audit_and_waits_for_them(self):
        audit = self.gate(60)
        audit.acquire("audit", 4096, timeout=1)
        self.blocked("rust-suite")
        audit.release()
        suite = self.gate(60).acquire("rust-suite", timeout=1)
        self.assertEqual(suite["audit_slots_held"], list(range(15)))
        self.blocked("audit", 4096, available_gib=60)
        self.blocked("audit", 7168, available_gib=60)
        # Lean is not an audit: it still runs beside the suite.
        self.gate().acquire("lean", timeout=1)

    def test_a_legacy_holder_excludes_every_slot(self):
        fd = os.open(self.legacy, os.O_RDWR | os.O_CREAT)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)  # what `flock <legacy lock> <cmd>` does
            for cls, budget in (("rust-suite", None), ("audit", 7168), ("lean", None)):
                self.blocked(cls, budget)
        finally:
            os.close(fd)
        self.gate().acquire("lean", timeout=1)

    def test_an_untimed_wait_blocks_on_the_legacy_lock_until_it_is_released(self):
        import threading
        fd = os.open(self.legacy, os.O_RDWR | os.O_CREAT)
        fcntl.flock(fd, fcntl.LOCK_EX)
        released = threading.Event()

        def release():
            released.set()
            os.close(fd)

        timer = threading.Timer(0.3, release)
        timer.start()
        try:
            record = self.gate().acquire("lean")  # no timeout: blocks in the kernel queue
        finally:
            timer.join()
        self.assertTrue(released.is_set(), "admitted only after the legacy holder released")
        self.assertEqual(record["legacy_lock"]["mode"], "shared")

    def test_a_slot_holder_excludes_a_legacy_job_but_not_other_slots(self):
        self.gate().acquire("lean", timeout=1)
        fd = os.open(self.legacy, os.O_RDWR | os.O_CREAT)
        try:
            with self.assertRaises(BlockingIOError):
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            os.close(fd)
        self.gate().acquire("audit", 7168, timeout=1)  # slot jobs share the legacy lock

    def test_release_frees_the_legacy_lock_and_clears_the_holder_note(self):
        gate = self.gate()
        gate.acquire("rust-suite", timeout=1, holder={"argv": ["x"]})
        note = json.loads((self.root / "gate" / "rust-suite" / "0").read_text())
        self.assertEqual(note["cls"], "rust-suite")
        gate.release()
        self.assertEqual((self.root / "gate" / "rust-suite" / "0").read_text(), "")
        fd = os.open(self.legacy, os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            os.close(fd)

    def test_unknown_class_refuses(self):
        with self.assertRaises(slot_mod.Refusal):
            self.gate().acquire("build", timeout=0.1)


class CommandLine(unittest.TestCase):
    def run_slot(self, root, *args, extra_env=None):
        env = dict(os.environ, QUIRIN_GATE_DIR=str(root / "gate"),
                   QUIRIN_GATE_LEGACY_LOCK=str(root / "legacy.lock"),
                   QUIRIN_GATE_MEMINFO=str(meminfo(root, 30)))
        env.pop(slot_mod.HELD_ENV, None)
        env.update(extra_env or {})
        return subprocess.run([sys.executable, str(SCRIPT), *args], env=env,
                              capture_output=True, text=True)

    def test_exit_code_and_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            receipt = root / "receipt.json"
            probe = ("import os, sys; print(os.environ['QCORE_AUDIT_JOBS'], "
                     "os.environ['QUIRIN_GATE_HELD']); sys.exit(3)")
            completed = self.run_slot(root, "audit", "--memory-budget", "5G", "--receipt",
                                      str(receipt), "--", sys.executable, "-c", probe)
            self.assertEqual(completed.returncode, 3, completed.stderr)
            self.assertEqual(completed.stdout.split(), ["2", "audit/0"])
            document = json.loads(receipt.read_text())
            self.assertEqual(document["class"], "audit")
            self.assertEqual(document["exit"], 3)
            self.assertEqual(document["slot_record"]["slot"], 0)
            self.assertEqual(document["slot_record"]["slot_count"], 6)  # 30 GiB / 5 GiB
            self.assertEqual(document["budget"]["label"], "5G")
            self.assertEqual(document["budget"]["memory_budget_mib"], 5120)
            self.assertEqual(document["slot_record"]["legacy_lock"]["mode"], "shared")

    def test_a_stub_current_route_accepts_the_inserted_budget(self):
        """A stub `quirin-audit current` that, like current_cli.rs, reads only
        the two-word `--memory-budget-mib N` runs under the slot."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stub = root / "quirin-audit"
            stub.write_text(
                "#!" + sys.executable + "\n"
                "import sys\n"
                "args = sys.argv[1:]\n"
                "assert args[0] == 'current'\n"
                "budget = None\n"
                "rest = iter(args[1:])\n"
                "for arg in rest:\n"
                "    if arg == '--memory-budget-mib':\n"
                "        budget = next(rest)\n"
                "    elif arg == '--jobs':\n"
                "        next(rest)\n"
                "    else:\n"
                "        print('unknown argument', arg, file=sys.stderr)\n"
                "        sys.exit(1)\n"
                "print(budget)\n")
            stub.chmod(0o755)
            receipt = root / "receipt.json"
            completed = self.run_slot(root, "audit", "--receipt", str(receipt), "--",
                                      str(stub), "current", "--jobs", "3")
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(completed.stdout.split(), ["7168"])
            document = json.loads(receipt.read_text())
            self.assertEqual(document["argv_run"][1:4], ["current", "--memory-budget-mib", "7168"])
            self.assertEqual(document["budget"]["admission_mib"], 7168)
            self.assertIsNone(document["budget"]["route_class"])

    def test_the_receipt_records_admission_and_the_childs_peak(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            corpus = scripts / "run_pinned_audit.py"
            # Hold 64 MiB of touched memory, then report the job cap it got.
            corpus.write_text("import os\nx = b'q' * (64 * 1024 * 1024)\n"
                              "print(os.environ['QCORE_AUDIT_JOBS'], len(x))\n")
            receipt = root / "receipt.json"
            completed = self.run_slot(root, "audit", "--receipt", str(receipt), "--",
                                      sys.executable, str(corpus), "out", "66910c10")
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(completed.stdout.split()[0], "3")
            document = json.loads(receipt.read_text())
            self.assertEqual(document["budget"]["memory_budget_mib"], 7168)
            self.assertEqual(document["budget"]["admission_mib"], 11264)
            self.assertEqual(document["budget"]["route_class"], "pinned-corpus")
            self.assertEqual(document["slot_record"]["slot_count"], 2)  # 30 GiB / 11 GiB
            self.assertGreaterEqual(document["max_rss_kib"], 64 * 1024)
            self.assertTrue(document["max_rss_within_admission"])

    def test_refusals_run_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "ran"
            touch = [sys.executable, "-c", f"open({str(marker)!r}, 'w')"]
            for args in (["lean", "--memory-budget", "4G", "--", *touch],
                         ["audit", "--memory-budget", "6G", "--", *touch]):
                self.assertEqual(self.run_slot(root, *args).returncode, 2, args)
            nested = self.run_slot(root, "rust-suite", "--", *touch,
                                   extra_env={slot_mod.HELD_ENV: "lean/0"})
            self.assertEqual(nested.returncode, 2)
            self.assertFalse(marker.exists())

    def test_a_nested_audit_under_the_suite_runs_in_the_parent_slot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            receipt = root / "nested.json"
            completed = self.run_slot(root, "audit", "--receipt", str(receipt), "--",
                                      sys.executable, "-c", "pass",
                                      extra_env={slot_mod.HELD_ENV: "rust-suite/0,audit/0"})
            self.assertEqual(completed.returncode, 0, completed.stderr)
            document = json.loads(receipt.read_text())
            self.assertEqual(document["nested_under"], ["rust-suite/0", "audit/0"])
            self.assertEqual(document["budget"]["memory_budget_mib"], 7168)

    def test_the_wait_record_says_waiting_until_the_slot_is_held(self):
        """Card MEASURE-QUOTA-AND-INCREMENTAL (R7): a real exclusive flock on the
        audit admission lock, as a rust-suite job holds it, keeps an audit job
        waiting; its live record says so, then says admitted, with the wait,
        before the command runs."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            admission = root / "gate" / "audit" / "admission"
            admission.parent.mkdir(parents=True)
            fd = os.open(admission, os.O_RDWR | os.O_CREAT, 0o666)
            fcntl.flock(fd, fcntl.LOCK_EX)
            record = root / "wait.json"
            env = dict(os.environ, QUIRIN_GATE_DIR=str(root / "gate"),
                       QUIRIN_GATE_LEGACY_LOCK=str(root / "legacy.lock"),
                       QUIRIN_GATE_MEMINFO=str(meminfo(root, 30)))
            env.pop(slot_mod.HELD_ENV, None)
            probe = f"import json; print(json.load(open({str(record)!r}))['state'])"
            process = subprocess.Popen(
                [sys.executable, str(SCRIPT), "audit", "--poll", "0.02", "--receipt",
                 str(root / "r.json"), "--wait-record", str(record), "--",
                 sys.executable, "-c", probe],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 30
                while not record.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                waiting = json.loads(record.read_text())
                self.assertEqual(waiting["schema"], "quirin-gate-wait/1")
                self.assertEqual(waiting["state"], "waiting")
                self.assertEqual(waiting["class"], "audit")
                time.sleep(0.5)
                self.assertEqual(json.loads(record.read_text())["state"], "waiting")
            finally:
                fcntl.flock(fd, fcntl.LOCK_UN)
                os.close(fd)
            stdout, stderr = process.communicate(timeout=60)
            self.assertEqual(process.returncode, 0, stderr)
            self.assertEqual(stdout.split(), ["admitted"])
            admitted = json.loads(record.read_text())
            self.assertEqual(admitted["state"], "admitted")
            self.assertGreaterEqual(admitted["waited_seconds"], 0.5)
            receipt = json.loads((root / "r.json").read_text())
            self.assertEqual(receipt["slot_record"]["waited_seconds"],
                             admitted["waited_seconds"])
            self.assertFalse((root / f".wait.json.{process.pid}.partial").exists())

    def test_a_nested_job_is_admitted_at_once_in_its_wait_record(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            record = root / "wait.json"
            completed = self.run_slot(root, "audit", "--wait-record", str(record),
                                      "--receipt", str(root / "r.json"), "--",
                                      sys.executable, "-c", "pass",
                                      extra_env={slot_mod.HELD_ENV: "rust-suite/0,audit/0"})
            self.assertEqual(completed.returncode, 0, completed.stderr)
            document = json.loads(record.read_text())
            self.assertEqual((document["state"], document["waited_seconds"]), ("admitted", 0.0))

    def test_a_signal_is_reported_as_128_plus_n(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            completed = self.run_slot(root, "lean", "--receipt", str(root / "r.json"), "--",
                                      sys.executable, "-c",
                                      "import os, signal; os.kill(os.getpid(), signal.SIGTERM)")
            self.assertEqual(completed.returncode, 128 + 15)


if __name__ == "__main__":
    unittest.main()
