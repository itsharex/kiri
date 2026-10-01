"""Check scheduling boundaries without native builds or GitHub mutations."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("ci_plan", Path(__file__).with_name("ci-plan.py"))
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)


class PlanTests(unittest.TestCase):
    def auto(self, *paths):
        return policy.plan("pull_request", "refs/pull/71/merge", {}, list(paths))

    def test_docs_and_evidence_do_not_build_or_render(self):
        result = self.auto("README_ZH.md", "docs/qa/report.md", "docs/qa/after.png")
        self.assertEqual(set(result.values()), {"quick", "false"})

    def test_frontend_runs_renderer_without_native_packages(self):
        result = self.auto("src/windows/EditorWindow.tsx")
        self.assertEqual(result["renderer"], "true")
        self.assertFalse(any(value == "true" for key, value in result.items() if key.startswith(("package_", "native_"))))

    def test_shared_backend_and_capability_check_every_target_without_packages(self):
        for path in ("src-tauri/src/core/geometry.rs", "src-tauri/capabilities/image-close.json", "src-tauri/Cargo.lock"):
            with self.subTest(path=path):
                result = self.auto(path)
                for target in policy.TARGET_PREFIXES:
                    self.assertEqual(result[f"native_{target}"], "true")
                    self.assertEqual(result[f"package_{target}"], "false")

    def test_each_platform_only_checks_affected_backend(self):
        for target, prefixes in policy.TARGET_PREFIXES.items():
            with self.subTest(target=target):
                result = self.auto(prefixes[0] + ".rs")
                for other in policy.TARGET_PREFIXES:
                    self.assertEqual(result[f"native_{other}"], str(other == target).lower())

    def test_manual_package_profiles_and_full(self):
        for profile in ("linux", "windows", "macos", "full"):
            result = policy.plan("workflow_dispatch", "refs/heads/qa", {"profile": profile}, [])
            self.assertEqual(result["renderer"], "true")
            for target in policy.TARGET_PREFIXES:
                self.assertEqual(result[f"package_{target}"], str(profile in {target, "full"}).lower())
            self.assertEqual(result["wayland"], str(profile in {"linux", "full"}).lower())

    def test_release_tags_full_but_main_push_never_automatically_packages(self):
        for event, ref in (("push", "refs/heads/main"), ("pull_request", "refs/pull/1/merge")):
            result = policy.plan(event, ref, {}, ["src-tauri/Cargo.toml"])
            self.assertFalse(any(result[f"package_{target}"] == "true" for target in policy.TARGET_PREFIXES))
        result = policy.plan("push", "refs/tags/v1.6.7", {}, [])
        self.assertEqual(result["profile"], "full")
        self.assertTrue(all(value == "true" for key, value in result.items() if key != "profile"))

    def test_unknown_diff_and_quick_do_not_disable_native_checks(self):
        for event, paths in (("pull_request", None), ("workflow_dispatch", [])):
            result = policy.plan(event, "refs/heads/qa", {}, paths)
            self.assertEqual(result["renderer"], "true")
            self.assertTrue(all(result[f"native_{target}"] == "true" for target in policy.TARGET_PREFIXES))
            self.assertTrue(all(result[f"package_{target}"] == "false" for target in policy.TARGET_PREFIXES))

    def test_old_dispatch_reuses_one_package_and_invalid_inputs_fail(self):
        result = policy.plan("workflow_dispatch", "refs/heads/qa", {"linux_candidate_run_id": "36815503794"}, [])
        self.assertEqual(result["profile"], "recheck-linux")
        self.assertEqual(result["wayland"], "true")
        self.assertFalse(any(value == "true" for key, value in result.items() if key.startswith(("native_", "package_"))))
        for inputs in ({"profile": "wrong"}, {"profile": "recheck-linux"}, {"linux_candidate_run_id": "1;bad"},
                       {"profile": "linux", "linux_candidate_run_id": "1"}):
            with self.subTest(inputs=inputs), self.assertRaises(ValueError):
                policy.plan("workflow_dispatch", "refs/heads/qa", inputs, [])

    def test_quality_gate_rejects_failure_cancellation_and_unexpected_skips(self):
        flags = self.auto("src/windows/EditorWindow.tsx")
        needs = {"plan": {"result": "success", "outputs": flags}, "fast-checks": {"result": "success"},
                 "countdown-ui": {"result": "success"}, **{name: {"result": "skipped"} for name in
                 ("test-rust", "build-linux", "build-windows", "build-macos", "test-linux-wayland")}}
        self.assertEqual(policy.check_results(needs), [])
        for result in ("failure", "cancelled", "skipped"):
            altered = {**needs, "countdown-ui": {"result": result}}
            self.assertEqual(policy.check_results(altered), ["countdown-ui"])
        self.assertIn("plan", policy.check_results({**needs, "plan": {"result": "failure"}}))
        self.assertIn("plan outputs", policy.check_results({**needs, "plan": {"result": "success", "outputs": {}}}))


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location("provenance", Path(__file__).with_name("linux-package-provenance.py"))
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.folder = Path(self.tmp.name)
        (self.folder / "kiri.deb").write_bytes(b"public isolated package fixture")
        self.manifest = {"schema_version": 1, "repository": "yuxino/kiri", "workflow_path": self.module.WORKFLOW,
                         "run_id": 101, "run_attempt": 1, "source_sha": "a" * 40,
                         "package": self.module.package_details(self.folder)}
        self.write_manifest()
        self.run = {"id": 101, "run_attempt": 1, "repository": {"full_name": "yuxino/kiri", "id": 1},
                    "head_repository": {"full_name": "yuxino/kiri"}, "head_sha": "a" * 40,
                    "path": self.module.WORKFLOW, "status": "completed", "event": "workflow_dispatch", "html_url": "fixture"}
        self.job = {"name": self.module.BUILD_JOB, "id": 202, "html_url": "fixture", "status": "completed",
                    "conclusion": "success", "steps": [{"name": "Build Debian package without updater signing", "conclusion": "success"}]}
        self.artifact = {"id": 303, "name": self.module.ARTIFACT, "expired": False,
                         "workflow_run": {"id": 101, "repository_id": 1, "head_sha": "a" * 40}}

    def write_manifest(self):
        (self.folder / "provenance.json").write_text(json.dumps(self.manifest))

    def verify(self, event="workflow_dispatch", own_run="101"):
        def command(*args):
            if args[:3] == ("git", "rev-parse", "HEAD"):
                return "a" * 40 if own_run == "101" else "b" * 40
            if args[:3] == ("git", "cat-file", "-p"):
                return "tree " + "c" * 40 + "\n\nfixture\n"
            if args[:2] == ("git", "diff"):
                return ""
            raise AssertionError(args)
        def api(url, raw=False):
            if raw:
                return "2026-10-01T00:00:00Z [command]/usr/bin/git log -1 --format=%H\n2026-10-01T00:00:00Z " + "a" * 40 + "\n"
            return self.run
        def items(url, key):
            return [self.job] if key == "jobs" else [self.artifact]
        evidence = {}
        with patch.dict(os.environ, {"GITHUB_REPOSITORY": "yuxino/kiri", "GITHUB_RUN_ID": own_run,
                                     "GITHUB_EVENT_NAME": event}, clear=True), \
                patch.object(self.module, "api", side_effect=api), \
                patch.object(self.module, "api_items", side_effect=items), \
                patch.object(self.module, "command", side_effect=command), \
                patch.object(self.module, "check_sources", return_value={"allowed_changes": []}):
            self.module.verify(SimpleNamespace(run_id="101", package_dir=self.folder, output=self.folder / "evidence.json"), evidence)
        return evidence

    def test_new_manual_and_automatic_builds_use_their_own_manifest(self):
        for event in ("workflow_dispatch", "pull_request", "push"):
            with self.subTest(event=event):
                evidence = self.verify(event=event)
                self.assertFalse(evidence["reused_package"])
                self.assertTrue(evidence["verified"])

    def test_manual_recheck_reuses_original_package(self):
        self.assertTrue(self.verify(own_run="102")["reused_package"])

    def test_automatic_run_cannot_use_another_run(self):
        with self.assertRaisesRegex(RuntimeError, "own package"):
            self.verify(event="pull_request", own_run="102")

    def test_new_manual_build_cannot_use_legacy_manifest_fallback(self):
        (self.folder / "provenance.json").unlink()
        with self.assertRaisesRegex(RuntimeError, "must contain provenance"):
            self.verify()
        # The existing explicit legacy-reuse contract is preserved.
        self.assertTrue(self.verify(own_run="102")["verified"])

    def test_wrong_attempt_or_package_digest_is_rejected(self):
        self.manifest["run_attempt"] = 2
        self.write_manifest()
        with self.assertRaisesRegex(RuntimeError, "identity"):
            self.verify()
        self.manifest["run_attempt"] = 1
        self.write_manifest()
        (self.folder / "kiri.deb").write_bytes(b"changed bytes")
        with self.assertRaisesRegex(RuntimeError, "recorded filename/checksum"):
            self.verify()

    def test_reuse_rejects_failed_build_fork_or_unfinished_run(self):
        self.job["conclusion"] = "failure"
        with self.assertRaisesRegex(RuntimeError, "must have succeeded"):
            self.verify(own_run="102")
        self.job["conclusion"] = "success"
        self.run["head_repository"]["full_name"] = "another/fork"
        with self.assertRaisesRegex(RuntimeError, "fork"):
            self.verify(own_run="102")
        self.run["head_repository"]["full_name"] = "yuxino/kiri"
        self.run["status"] = "in_progress"
        with self.assertRaisesRegex(RuntimeError, "has not completed"):
            self.verify(own_run="102")

    def test_expired_or_wrong_origin_artifact_is_rejected(self):
        self.artifact["expired"] = True
        with self.assertRaisesRegex(RuntimeError, "unexpired"):
            self.verify()
        self.artifact["expired"] = False
        self.artifact["workflow_run"]["head_sha"] = "d" * 40
        with self.assertRaisesRegex(RuntimeError, "Artifact source"):
            self.verify()


if __name__ == "__main__":
    unittest.main()
