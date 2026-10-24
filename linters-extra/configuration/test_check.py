"""Exercise the checked-in configuration against real shared checker fixtures."""

import copy
from contextlib import redirect_stderr
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import check
from rust_style_lint.runner import CHECKER_NAMES, load_checker, load_config


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = {
    "no-allow": '#[allow(unused)]\nfn work() {}\n',
    "no-inline-format": 'fn work() { println!("{value}"); }\n',
    "no-inline-tests": '#[cfg(test)]\nmod tests { #[test] fn sample() {} }\n',
    "spacing-style": 'fn work() {\n    let first = 1;\n    let second = 2;\n}\n',
    "use-style": 'fn work() {}\nuse std::mem;\n',
    "generic-where": 'fn work<T: Copy>() {}\n',
    "no-type-hint": 'fn work() { let value: u8 = 1; }\n',
    "no-redundant-destructure": 'fn work() { let pair = make(); let (a, b) = pair; }\n',
    "item-layout": 'pub struct Item;\nconst VALUE: u8 = 1;\nimpl Item { fn work() {} }\n',
    "doc-comment-coverage": 'pub struct Undocumented;\n',
    "forbidden-identifiers": 'fn work() { let msg = 1; }\n',
    "module-dependency": 'pub struct Owner;\nmod child { fn work(_: super::Owner) {} }\n',
    "visibility-style": 'pub(crate) fn work() {}\n',
    "trait-use-anonymous": 'use external::MethodTrait;\nfn work(value: Value) { value.call(); }\n',
    "prefer-if-let-guard": 'fn work(value: Option<u8>) { match value { Some(v) => consume(v), None => return, } }\n',
    "no-unnamed-fields": 'pub enum Choice { Pair(u8, u8) }\n',
}
CONTEXT_FIXTURES = {
    "module": 'mod NAME {}\n',
    "function": 'fn NAME() {}\n',
    "let": 'fn work() { let NAME = 1; }\n',
    "parameter": 'fn work(NAME: u8) {}\n',
    "const": 'const NAME: u8 = 1;\n',
    "static": 'static NAME: u8 = 1;\n',
    "enum_variant": 'enum Choice { NAME }\n',
    "type": 'struct NAME;\n',
    "field": 'struct Item { NAME: u8 }\n',
    "macro_field": 'fn work() { tracing::info!(NAME = 1); }\n',
}


class ConfigurationTests(unittest.TestCase):
    def setUp(self):
        self.config = load_config(ROOT)

    def diagnostics(self, checker, source, config=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(source)
            section = self.config.get(checker) if config is None else config
            found = load_checker(checker).check(root, section)
            if checker == "forbidden-identifiers":
                found.extend(check.module_violations(root, section))
            return found

    def test_real_config_is_valid_and_every_checker_detects_its_fixture(self):
        self.assertEqual(check.validate(self.config), [])
        self.assertEqual(set(FIXTURES), set(CHECKER_NAMES))
        for name, source in FIXTURES.items():
            with self.subTest(checker=name):
                self.assertTrue(self.diagnostics(name, source))

    def test_every_configured_word_and_prefix_triggers_in_each_selected_context(self):
        section = self.config["forbidden-identifiers"]
        self.assertEqual(len(section["words"]), 30)
        self.assertEqual(len(section["prefixes"]), 5)
        for table, identity in (("words", "word"), ("prefixes", "prefix")):
            for rule in section[table]:
                name = rule[identity]
                for context in rule["contexts"]:
                    with self.subTest(table=table, name=name, context=context):
                        source = CONTEXT_FIXTURES[context].replace("NAME", name)
                        found = self.diagnostics("forbidden-identifiers", source)
                        self.assertTrue(any(v.code == rule["code"] for v in found), found)

    def test_disabled_or_missing_checker_is_an_error(self):
        for name in CHECKER_NAMES:
            for value in (False, None):
                config = copy.deepcopy(self.config)
                if value is None:
                    del config["linters"][name]
                else:
                    config["linters"][name] = value
                with self.subTest(name=name, value=value):
                    self.assertIn(f"checker must be enabled: {name}", check.validate(config))

    def test_missing_empty_or_unknown_contexts_are_errors(self):
        for table in ("words", "prefixes"):
            for contexts in (None, [], ["unsupported"]):
                config = copy.deepcopy(self.config)
                rule = config["forbidden-identifiers"][table][0]
                if contexts is None:
                    del rule["contexts"]
                else:
                    rule["contexts"] = contexts
                with self.subTest(table=table, contexts=contexts):
                    self.assertTrue(check.validate(config))
            config = copy.deepcopy(self.config)
            config["forbidden-identifiers"][table] = []
            self.assertTrue(check.validate(config))

    def test_unknown_keys_and_dead_trait_lists_are_errors(self):
        for section, key in (("linters", "imaginary"), ("trait-use-anonymous", "external_traits"), ("trait-use-anonymous", "macro_traits"), ("forbidden-identifiers", "imaginary")):
            config = copy.deepcopy(self.config)
            config[section][key] = True
            with self.subTest(section=section, key=key):
                self.assertTrue(check.validate(config))

    def test_invalid_types_and_missing_sections_are_errors(self):
        for section in check.SECTION_KEYS:
            config = copy.deepcopy(self.config)
            del config[section]
            with self.subTest(section=section):
                self.assertTrue(check.validate(config))
        for contexts in ("function", [None], [123]):
            config = copy.deepcopy(self.config)
            config["forbidden-identifiers"]["words"][0]["contexts"] = contexts
            self.assertTrue(check.validate(config))

    def test_unrelated_names_and_test_modules_are_accepted(self):
        self.assertEqual(self.diagnostics("forbidden-identifiers", 'fn work(value: u8) { let item = value; }\n'), [])
        self.assertEqual(self.diagnostics("forbidden-identifiers", '#[cfg(test)]\nmod err { fn res() {} }\n'), [])

    def test_cli_fails_when_project_config_is_missing(self):
        with tempfile.TemporaryDirectory() as directory:
            errors = io.StringIO()
            with patch.object(sys, "argv", ["check.py", "--root", directory]), redirect_stderr(errors):
                self.assertEqual(check.main(), 1)
            self.assertIn("CFG001", errors.getvalue())

    def test_ignored_word_message_is_rejected(self):
        config = copy.deepcopy(self.config)
        config["forbidden-identifiers"]["words"][0]["message"] = "Unused diagnostic"
        self.assertTrue(check.validate(config))

    def test_repository_and_term_module_skips_are_rejected(self):
        for key, value in (("skip_module_paths", ["part/repo"]), ("ignore_files", ["src/usecase/term.rs"])):
            config = copy.deepcopy(self.config)
            config["forbidden-identifiers"][key] = value
            with self.subTest(key=key):
                self.assertTrue(check.validate(config))


if __name__ == "__main__":
    unittest.main()
