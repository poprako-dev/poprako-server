from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-spacing-style.py")
SPEC = importlib.util.spec_from_file_location("check_spacing_style", SCRIPT)

assert SPEC is not None
assert SPEC.loader is not None

CHECKER_MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER_MODULE
SPEC.loader.exec_module(CHECKER_MODULE)


class RustSpacingCheckerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.checker = CHECKER_MODULE.RustSpacingChecker()
        self.path = Path("fixture.rs")

    def analyze(self, source: str, *, build_fixes: bool = False):
        return self.checker.analyze_source(
            self.path,
            source.encode(),
            build_fixes=build_fixes,
        )

    def test_cfg_attribute_and_statement_are_one_unit(self) -> None:
        analysis = self.analyze(
            """fn example() {
    //
    let router = router();

    #[cfg(feature = \"swagger-ui\")]
    let router = router.merge(swagger());

    router
}
"""
        )

        self.assertEqual(analysis.diagnostics, ())

    def test_cfg_attribute_on_block_is_one_unit(self) -> None:
        analysis = self.analyze(
            """async fn example() {
    //
    let ctrl_c = ctrl_c();

    #[cfg(unix)]
    {
        terminate().await;
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await;
    }
}
"""
        )

        self.assertEqual(analysis.diagnostics, ())

    def test_fix_does_not_split_attribute_from_statement(self) -> None:
        source = """fn example() {
    //
    let first = 1;
    #[cfg(test)]
    let second = 2;
}
"""
        analysis = self.analyze(source, build_fixes=True)
        fixed = CHECKER_MODULE.apply_edits(source.encode(), analysis.edits)

        self.assertEqual(
            fixed.decode(),
            """fn example() {
    //
    let first = 1;

    #[cfg(test)]
    let second = 2;
}
""",
        )

    def test_enum_variants_require_a_blank_line(self) -> None:
        source = """enum Payload {
    /// First payload.
    First,
    /// Second payload.
    Second,
}
"""
        analysis = self.analyze(source, build_fixes=True)
        fixed = CHECKER_MODULE.apply_edits(source.encode(), analysis.edits)

        self.assertEqual(
            [diagnostic.code for diagnostic in analysis.diagnostics],
            ["BLK001"],
        )
        self.assertEqual(
            fixed.decode(),
            """enum Payload {
    /// First payload.
    First,

    /// Second payload.
    Second,
}
""",
        )

    def test_multi_field_struct_requires_a_block_start_separator(self) -> None:
        source = """struct Payload {
    first: String,
    second: String,
}
"""
        analysis = self.analyze(source, build_fixes=True)
        fixed = CHECKER_MODULE.apply_edits(source.encode(), analysis.edits)

        self.assertEqual(
            [diagnostic.code for diagnostic in analysis.diagnostics],
            ["BLK000"],
        )
        self.assertEqual(
            fixed.decode(),
            """struct Payload {
    //
    first: String,
    second: String,
}
""",
        )

    def test_nested_variant_struct_and_following_variant_do_not_overlap(self) -> None:
        source = """enum Payload {
    First {
        first: String,
        second: String,
    },
    Second,
}
"""
        analysis = self.analyze(source, build_fixes=True)
        fixed = CHECKER_MODULE.apply_edits(source.encode(), analysis.edits)

        self.assertEqual(
            [diagnostic.code for diagnostic in analysis.diagnostics],
            ["BLK000", "BLK001"],
        )
        self.assertEqual(
            fixed.decode(),
            """enum Payload {
    First {
        //
        first: String,
        second: String,
    },

    Second,
}
""",
        )


if __name__ == "__main__":
    unittest.main()
