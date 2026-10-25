#!/usr/bin/env python3
"""Exercise release gates with local fixtures, without release builds or APIs."""

import copy
import importlib.util
import json
from pathlib import Path
import shlex
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "release_readiness", Path(__file__).with_name("release-readiness.py")
)
READINESS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(READINESS)
SHA = "1" * 40
WORKFLOW = {
    "id": 42, "run_attempt": 1, "head_sha": SHA, "head_branch": "main",
    "event": "push", "conclusion": "success",
    "repository": {"full_name": "poprako-dev/poprako-server"},
}
EVIDENCE = {
    "source_commit": SHA, "run_id": "42", "run_attempt": "1", "result": "success",
    "source_image": READINESS.IMAGE + "sha256:" + "a" * 64,
    "local_image": f"poprako-server-prod:sha-{SHA}", "image_id": "sha256:" + "b" * 64,
}


class ReleaseReadinessTest(unittest.TestCase):
    def test_sbom_uses_an_explicit_output_name_instead_of_a_tool_default(self):
        script = (READINESS.ROOT / "scripts/ci-release.sh").read_text().replace("\\\n", " ")
        commands = [shlex.split(line) for line in script.splitlines()
                    if line.startswith("cargo cyclonedx --format")]
        self.assertEqual(len(commands), 1)
        command = commands[0]
        self.assertIn("--override-filename", command)
        basename = command[command.index("--override-filename") + 1]
        self.assertIn(f'cp {basename}.json "$sbom_file"', script)

    def test_first_stable_release_requires_one_root_commit_and_an_annotated_tag(self):
        with patch.object(READINESS, "run", side_effect=[SHA, "tag", SHA, "", "1"]) as commands:
            READINESS.validate_checkout("v1.0.0", "1.0.0", SHA)
            commands.assert_any_call("git", "rev-list", "--all", "--count")
        for outputs in ([SHA, "commit"], ["2" * 40], [SHA, "tag", "2" * 40],
                        [SHA, "tag", SHA, "", "2"]):
            with self.subTest(outputs=outputs), patch.object(READINESS, "run", side_effect=outputs), self.assertRaises(ValueError):
                READINESS.validate_checkout("v1.0.0", "1.0.0", SHA)

    def test_snapshot_requires_consistent_versions_and_maintained_notes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["."]\n[workspace.package]\nversion = "1.0.0"\n'
                '[package]\nname = "poprako-server"\nversion.workspace = true\n'
            )
            (root / "Cargo.lock").write_text(
                '[[package]]\nname = "poprako-server"\nversion = "1.0.0"\n'
            )
            (root / "docs/releases").mkdir(parents=True)
            (root / "docs/swagger.json").write_text(json.dumps({"info": {"version": "1.0.0"}}))
            (root / "docs/releases/1.0.0.md").write_text("First stable release.\n")
            (root / "CHANGELOG.md").write_text("## [1.0.0]\nFirst stable release.\n")
            self.assertEqual(READINESS.validate_snapshot(root, "v1.0.0"), "1.0.0")
            for tag in ("1.0.0", "v01.0.0", "v1.0.0extra", "v1.0", "v1.0.0-rc.1"):
                with self.subTest(tag=tag), self.assertRaises(ValueError):
                    READINESS.validate_snapshot(root, tag)
            for file, replacement in (
                ("Cargo.lock", '[[package]]\nname = "poprako-server"\nversion = "0.1.0"\n'),
                ("docs/swagger.json", '{"info":{"version":"0.1.0"}}'),
                ("docs/releases/1.0.0.md", ""),
                ("CHANGELOG.md", "## [Unreleased]\n"),
            ):
                path = root / file
                original = path.read_text()
                path.write_text(replacement)
                with self.subTest(file=file), self.assertRaises(ValueError):
                    READINESS.validate_snapshot(root, "v1.0.0")
                path.write_text(original)

    def test_deployment_requires_exact_commit_attempt_and_image_identity(self):
        READINESS.validate_deployment(EVIDENCE, SHA, WORKFLOW)
        for field, value in (
            ("source_commit", "2" * 40), ("run_id", "41"), ("run_attempt", "2"),
            ("result", "failure"), ("source_image", "ghcr.io/poprako-dev/poprako-server:latest"),
            ("local_image", "poprako-server-prod:latest"), ("image_id", "sha256:short"),
        ):
            evidence = dict(EVIDENCE, **{field: value})
            with self.subTest(field=field), self.assertRaises(ValueError):
                READINESS.validate_deployment(evidence, SHA, WORKFLOW)

    def test_ci_must_be_successful_main_push_in_the_same_repository(self):
        READINESS.validate_run(WORKFLOW, SHA, "poprako-dev/poprako-server")
        for field, value in (
            ("head_sha", "2" * 40), ("head_branch", "feature"), ("event", "workflow_dispatch"),
            ("conclusion", "failure"), ("repository", {"full_name": "other/repo"}),
            ("run_attempt", 2),
        ):
            workflow = dict(WORKFLOW, **{field: value})
            with self.subTest(field=field), self.assertRaises(ValueError):
                READINESS.validate_run(workflow, SHA, "poprako-dev/poprako-server")

    def test_production_requires_at_least_one_independent_approval(self):
        approval = {"state": "approved", "user": {"login": "reviewer"},
                    "environments": [{"name": "production"}]}
        self.assertEqual(READINESS.production_approvers([approval], {"preparer"}), ["reviewer"])
        self.assertEqual(READINESS.production_approvers(
            [dict(approval, state="rejected"), approval,
             dict(approval, user={"login": "preparer"})], {"preparer"}), ["reviewer"])
        for reviews in ([], [dict(approval, environments=[{"name": "release"}])],
                        [dict(approval, user={"login": "preparer"})],
                        [dict(approval, state="rejected")]):
            with self.subTest(reviews=reviews), self.assertRaises(ValueError):
                READINESS.production_approvers(reviews, {"preparer"})

    def test_ci_pagination_retains_jobs_and_runs_after_first_page(self):
        pages = [{"jobs": [{"name": "Rust"}]}, {"jobs": [{"name": "Deploy production"}]}]
        with patch.object(READINESS, "run", return_value=json.dumps(pages)):
            self.assertEqual(READINESS.paginated("owner/repo", "endpoint", "jobs"),
                             pages[0]["jobs"] + pages[1]["jobs"])

    def test_rerun_cannot_reuse_approval_history_from_an_earlier_attempt(self):
        workflow = dict(WORKFLOW, run_attempt=2)
        with patch.object(READINESS, "paginated", return_value=[workflow]), patch.object(
            READINESS, "api"
        ) as approval_api, self.assertRaisesRegex(ValueError, "first-attempt"):
            READINESS.collect_deployment("poprako-dev/poprako-server", SHA, Path("unused.json"))
        approval_api.assert_not_called()

    def test_missing_http_job_blocks_release_even_when_other_jobs_pass(self):
        workflow = copy.deepcopy(WORKFLOW)
        workflow.update({"actor": {"login": "preparer"}, "triggering_actor": {"login": "preparer"}})
        with patch.object(READINESS, "paginated", side_effect=[
            [workflow], [{"name": name, "conclusion": "success"} for name in (
                "Rust", "TypeScript", "Diesel migrations", "Dependency security", "Production image", "Deploy production")]
        ]), self.assertRaisesRegex(ValueError, "HTTP integration"):
            READINESS.collect_deployment("poprako-dev/poprako-server", SHA, Path("unused.json"))

    def test_successful_ci_deployment_and_approval_are_written_to_release_evidence(self):
        workflow = copy.deepcopy(WORKFLOW)
        workflow.update({"actor": {"login": "preparer"}, "triggering_actor": {"login": "preparer"},
                         "html_url": "https://github.com/poprako-dev/poprako-server/actions/runs/42"})
        jobs = [{"name": name, "conclusion": "success"} for name in (
            "Rust", "TypeScript", "Diesel migrations", "Dependency security", "HTTP integration",
            "Production image", "Deploy production")]
        reviews = [{"state": "approved", "user": {"login": "reviewer"},
                    "environments": [{"name": "production"}]}]

        def download(*args):
            self.assertEqual(args[:3], ("gh", "run", "download"))
            destination = Path(args[args.index("--dir") + 1])
            (destination / "deployment.json").write_text(json.dumps(EVIDENCE))
            return ""

        with tempfile.TemporaryDirectory() as directory, patch.dict(READINESS.os.environ, {"GITHUB_ACTOR": "preparer"}), patch.object(
            READINESS, "paginated", side_effect=[[workflow], jobs]
        ), patch.object(READINESS, "api", return_value=reviews), patch.object(READINESS, "run", side_effect=download):
            output = Path(directory) / "release-deployment.json"
            READINESS.collect_deployment("poprako-dev/poprako-server", SHA, output)
            evidence = json.loads(output.read_text())
            self.assertEqual(evidence["approvers"], ["reviewer"])
            self.assertEqual(evidence["source_commit"], SHA)
            self.assertEqual(evidence["source_image"], EVIDENCE["source_image"])
            self.assertEqual(evidence["approval_history"], reviews)


if __name__ == "__main__":
    unittest.main()
