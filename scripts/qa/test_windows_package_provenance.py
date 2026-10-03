"""A recheck must not replace accepted tag binaries with another candidate."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("windows_provenance", Path(__file__).with_name("windows-package-provenance.py"))
provenance = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provenance)


class AcceptedCandidate(unittest.TestCase):
    def setUp(self):
        self.run = {"id": 123, "repository": {"full_name": provenance.REPOSITORY},
                    "head_repository": {"full_name": provenance.REPOSITORY},
                    "path": provenance.WORKFLOW, "event": "push", "head_branch": "v1.6.8",
                    "head_sha": "a" * 40, "status": "completed", "conclusion": "success"}
        self.jobs = [{"name": provenance.JOB, "conclusion": "success", "steps": [
            {"name": name, "conclusion": "success"} for name in (
                "Package and verify portable Windows build", "Install and smoke-test both Windows packages",
                "Upload Windows bundle")]}]

    def validate(self):
        provenance.validate(self.run, self.jobs, 123, "v1.6.8", "a" * 40)

    def test_accepts_successful_official_tag_and_desktop_checks(self):
        self.validate()

    def test_rejects_other_run_repository_or_fork(self):
        for key, value in (("id", 124), ("repository", {"full_name": "other/kiri"}),
                           ("head_repository", {"full_name": "other/kiri"})):
            with self.subTest(key=key):
                original = copy.deepcopy(self.run)
                self.run[key] = value
                with self.assertRaises(RuntimeError):
                    self.validate()
                self.run = original

    def test_rejects_non_tag_or_different_source(self):
        for key, value in (("path", "release.yml"), ("event", "pull_request"),
                           ("head_branch", "main"), ("head_sha", "b" * 40)):
            with self.subTest(key=key):
                original = copy.deepcopy(self.run)
                self.run[key] = value
                with self.assertRaises(RuntimeError):
                    self.validate()
                self.run = original

    def test_rejects_incomplete_or_failed_build(self):
        for key, value in (("status", "in_progress"), ("conclusion", "failure")):
            with self.subTest(key=key):
                original = copy.deepcopy(self.run)
                self.run[key] = value
                with self.assertRaises(RuntimeError):
                    self.validate()
                self.run = original

    def test_rejects_missing_duplicate_or_failed_windows_job(self):
        original = copy.deepcopy(self.jobs)
        for jobs in ([], original * 2, [{**original[0], "conclusion": "failure"}]):
            self.jobs = jobs
            with self.assertRaises(RuntimeError):
                self.validate()

    def test_rejects_missing_or_failed_package_smoke_steps(self):
        for index in range(3):
            for value in ("failure", "skipped"):
                self.jobs[0]["steps"][index]["conclusion"] = value
                with self.assertRaises(RuntimeError):
                    self.validate()
                self.jobs[0]["steps"][index]["conclusion"] = "success"
        self.jobs[0]["steps"] = []
        with self.assertRaises(RuntimeError):
            self.validate()


class ScreenshotDependency(unittest.TestCase):
    def test_every_native_package_workflow_installs_the_image_dependency(self):
        root = Path(__file__).resolve().parents[2]
        consumers = []
        for workflow in (root / ".github/workflows").glob("*.yml"):
            text = workflow.read_text()
            if "scripts/qa/windows-release-native.py" in text:
                consumers.append(workflow.name)
                with self.subTest(workflow=workflow.name):
                    self.assertTrue(any("pip install" in line and "pywinauto==" in line
                                        and "Pillow==11.3.0" in line for line in text.splitlines()))
        self.assertGreaterEqual(len(consumers), 4)


if __name__ == "__main__":
    unittest.main()
