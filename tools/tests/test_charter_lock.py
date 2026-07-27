import hashlib
import json
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
TOOL = REPO_ROOT / "tools" / "charter_lock.py"
TARGET = "spec/charter/systems/computer.md"
NOTICE = (
    "> **AI 保护锁：已经锁定；未经用户明确解锁，AI 只能提出建议，不得直接修改；"
    "授权修改完成后必须重新锁定。**"
)


class CharterLockTest(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.target = self.root / TARGET
        self.target.parent.mkdir(parents=True)
        self.write_target(f"{NOTICE}\n\n# Computer System\n\nOriginal body.\n")
        self.write_manifest()
        self.target.chmod(0o444)

    def tearDown(self):
        self.temporary_directory.cleanup()

    def digest(self):
        return hashlib.sha256(self.target.read_bytes()).hexdigest()

    def write_target(self, content):
        self.target.write_text(content, encoding="utf-8")

    def write_manifest(self, locks=None, version=1):
        if locks is None:
            locks = [{"path": TARGET, "state": "locked", "sha256": self.digest()}]
        manifest = {"version": version, "locks": locks}
        (self.root / "charter-locks.json").write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )

    def run_tool(self, *arguments):
        return subprocess.run(
            [sys.executable, str(TOOL), "--root", str(self.root), *arguments],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_valid_locked_target_passes_check_and_status(self):
        checked = self.run_tool("check")
        self.assertEqual(checked.returncode, 0, checked.stderr)
        self.assertIn("1 locked charter(s) valid", checked.stdout)

        status_result = self.run_tool("status")
        self.assertEqual(status_result.returncode, 0, status_result.stderr)
        self.assertIn("state=locked", status_result.stdout)
        self.assertIn("notice=present", status_result.stdout)
        self.assertIn("writable=no", status_result.stdout)

    def test_body_drift_fails_check(self):
        self.target.chmod(0o644)
        self.write_target(f"{NOTICE}\n\n# Computer System\n\nChanged body.\n")

        enforced = self.run_tool("enforce")
        result = self.run_tool("check")

        self.assertNotEqual(enforced.returncode, 0)
        self.assertIn("SHA-256 drift", enforced.stderr)
        self.assertNotEqual(stat.S_IMODE(self.target.stat().st_mode) & 0o222, 0)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA-256 drift", result.stderr)

    def test_missing_notice_fails_check(self):
        self.target.chmod(0o644)
        self.write_target("# Computer System\n\nOriginal body.\n")

        result = self.run_tool("check")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("first-line lock notice is missing", result.stderr)

    def test_invalid_and_duplicate_manifest_paths_fail(self):
        digest = self.digest()
        cases = (
            (
                [
                    {"path": TARGET, "state": "locked", "sha256": digest},
                    {"path": TARGET, "state": "locked", "sha256": digest},
                ],
                "duplicate lock path",
            ),
            (
                [{"path": "../computer.md", "state": "locked", "sha256": digest}],
                "invalid component",
            ),
            (
                [{"path": "spec/model/computer.md", "state": "locked", "sha256": digest}],
                "under spec/charter",
            ),
        )
        for locks, expected_error in cases:
            with self.subTest(expected_error=expected_error):
                self.write_manifest(locks=locks)
                result = self.run_tool("check")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_error, result.stderr)

    def test_lock_refuses_target_that_was_not_unlocked(self):
        result = self.run_tool("lock", TARGET)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expected unlocked", result.stderr)

    def test_unlock_makes_check_fail(self):
        unlocked = self.run_tool("unlock", TARGET)
        self.assertEqual(unlocked.returncode, 0, unlocked.stderr)
        self.assertTrue(stat.S_IMODE(self.target.stat().st_mode) & stat.S_IWUSR)

        checked = self.run_tool("check")

        self.assertNotEqual(checked.returncode, 0)
        self.assertIn("expected locked", checked.stderr)
        self.assertIn("write permission remains", checked.stderr)

    def test_relock_refreshes_hash_and_removes_all_write_bits(self):
        self.assertEqual(self.run_tool("unlock", TARGET).returncode, 0)
        updated = f"{NOTICE}\n\n# Computer System\n\nAuthorized change.\n"
        self.write_target(updated)

        relocked = self.run_tool("lock", TARGET)

        self.assertEqual(relocked.returncode, 0, relocked.stderr)
        manifest = json.loads((self.root / "charter-locks.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["locks"][0]["state"], "locked")
        self.assertEqual(manifest["locks"][0]["sha256"], self.digest())
        self.assertEqual(stat.S_IMODE(self.target.stat().st_mode) & 0o222, 0)
        self.assertEqual(self.run_tool("check").returncode, 0)

    def test_enforce_restores_clone_write_permissions(self):
        self.target.chmod(0o644)

        enforced = self.run_tool("enforce")

        self.assertEqual(enforced.returncode, 0, enforced.stderr)
        self.assertIn("restored read-only mode", enforced.stdout)
        self.assertEqual(stat.S_IMODE(self.target.stat().st_mode) & 0o222, 0)
        self.assertEqual(self.run_tool("check").returncode, 0)


if __name__ == "__main__":
    unittest.main()
