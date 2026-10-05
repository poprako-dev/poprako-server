#!/usr/bin/env python3
"""Exercise lint policy regressions with disposable manifest fixtures."""

import importlib.util
from pathlib import Path
import tempfile
import tomllib
import unittest


PROJECT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "lint_policy", PROJECT / "scripts/check-lint-policy.py"
)
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


class LintPolicyTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="poprako-lint-policy-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.root_manifest = self.root / "Cargo.toml"
        self.root_manifest.write_text((PROJECT / "Cargo.toml").read_text())
        self.member_manifest = self.root / "member/Cargo.toml"
        self.member_manifest.parent.mkdir()
        self.member_manifest.write_text("[lints]\nworkspace = true\n")
        self.metadata = {
            "workspace_root": str(self.root),
            "workspace_members": ["root", "member"],
            "packages": [
                {"id": "root", "manifest_path": str(self.root_manifest)},
                {"id": "member", "manifest_path": str(self.member_manifest)},
            ],
        }

    def test_all_members_inherit(self):
        self.assertEqual(POLICY.inspect_policy(self.metadata), [])

    def test_new_member_without_inheritance_is_rejected(self):
        self.member_manifest.write_text('[package]\nname = "new-member"\n')
        self.assertTrue(any("inherit" in error for error in POLICY.inspect_policy(self.metadata)))

    def test_member_policy_override_is_rejected(self):
        self.member_manifest.write_text('[lints.clippy]\npedantic = "allow"\n')
        self.assertTrue(any("inherit" in error for error in POLICY.inspect_policy(self.metadata)))

    def test_root_and_member_follow_identical_inheritance_rules(self):
        content = self.root_manifest.read_text().replace("[lints]\nworkspace = true\n", "")
        self.root_manifest.write_text(content)
        self.assertTrue(any("inherit" in error for error in POLICY.inspect_policy(self.metadata)))

    def test_every_required_rule_rejects_downgrades(self):
        original = self.root_manifest.read_text()
        policy = tomllib.loads(original)["workspace"]["lints"]

        for tool, rules in policy.items():
            for name, setting in rules.items():
                with self.subTest(tool=tool, rule=name):
                    before = f'{name} = "{setting}"'
                    after = f'{name} = "allow"'

                    if isinstance(setting, dict):
                        before = f'{name} = {{ level = "{setting["level"]}", priority = -1 }}'
                        after = f'{name} = {{ level = "warn", priority = -1 }}'

                    self.assertIn(before, original)
                    self.root_manifest.write_text(original.replace(before, after, 1))
                    self.assertTrue(POLICY.inspect_policy(self.metadata))

        self.root_manifest.write_text(original)

    def test_missing_rule_and_group_priority_are_rejected(self):
        content = self.root_manifest.read_text().replace('expect_used = "deny"\n', '')
        content = content.replace('priority = -1', 'priority = 0', 1)
        self.root_manifest.write_text(content)
        errors = POLICY.inspect_policy(self.metadata)
        self.assertTrue(any("expect_used" in error for error in errors))
        self.assertTrue(any("priority" in error for error in errors))

    def test_new_workspace_allow_is_rejected(self):
        content = self.root_manifest.read_text().replace(
            '[workspace.lints.clippy]\n',
            '[workspace.lints.clippy]\nfuture_not_send = "allow"\n',
        )
        self.root_manifest.write_text(content)
        self.assertTrue(any("weakens" in error for error in POLICY.inspect_policy(self.metadata)))


if __name__ == "__main__":
    unittest.main()
