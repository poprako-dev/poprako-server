"""Validate reuse and isolation across rule calls and source changes."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "linters"))
sys.path.insert(0, str(ROOT / "linters-extra"))

import analysis_session as analysis
import production_source as production


class AnalysisSessionTests(unittest.TestCase):
    def test_rules_reuse_parses_but_cannot_mutate_cached_trees(self):
        source = b"fn example() {}"
        with patch.object(analysis, "_PARSER", wraps=analysis._PARSER) as parser:
            with analysis.analysis_session():
                tree = analysis.parse_rust(source)
                tree.edit(start_byte=0, old_end_byte=0, new_end_byte=1,
                          start_point=(0, 0), old_end_point=(0, 0), new_end_point=(0, 1))
                self.assertEqual(analysis.parse_rust(source).root_node.end_byte, len(source))
                analysis.parse_rust(b"fn changed() {}")
            with analysis.analysis_session():
                analysis.parse_rust(source)
            self.assertEqual(parser.parse.call_count, 3)

    def test_tree_cache_has_a_fixed_capacity(self):
        with patch.object(analysis, "MAX_TREES", 2), patch.object(analysis, "_PARSER", wraps=analysis._PARSER) as parser:
            with analysis.analysis_session():
                for source in (b"fn a() {}", b"fn b() {}", b"fn c() {}", b"fn a() {}"):
                    analysis.parse_rust(source)
            self.assertEqual(parser.parse.call_count, 4)

    def test_new_run_observes_changed_sources_and_module_cfg(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            parent = root / "src/lib.rs"
            source = root / "src/fixture.rs"
            parent.write_text("#[cfg(test)] mod fixture;")
            source.write_text("fn first() {}")
            with patch.object(production, "_production_source", wraps=production._production_source) as read:
                with production.source_session():
                    self.assertNotIn(b"first", production.production_source(source, root))
                    production.production_source(source, root)
                self.assertEqual(read.call_count, 1)
            parent.write_text("mod fixture;")
            source.write_text("fn second() {}")
            with production.source_session():
                self.assertIn(b"second", production.production_source(source, root))

    def test_failure_releases_session_state(self):
        with self.assertRaises(RuntimeError):
            with analysis.analysis_session(), production.source_session():
                raise RuntimeError("fixture")
        self.assertIsNone(analysis._TREES.get())
        self.assertIsNone(production._SOURCES.get())


if __name__ == "__main__":
    unittest.main()
