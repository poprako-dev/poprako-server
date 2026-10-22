"""Share Rust syntax trees during one read-only checker run."""

from collections import OrderedDict
from contextlib import contextmanager
from contextvars import ContextVar

import tree_sitter
import tree_sitter_rust


_PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
_TREES: ContextVar[OrderedDict | None] = ContextVar("lint_trees", default=None)
MAX_TREES = 1024


@contextmanager
def analysis_session():
    """Release cached trees when the run finishes, including on failure."""
    token = _TREES.set(OrderedDict())
    try:
        yield
    finally:
        _TREES.reset(token)


def parse_rust(source: bytes) -> tree_sitter.Tree:
    trees = _TREES.get()
    if trees is None:
        return _PARSER.parse(source)
    if source not in trees:
        trees[source] = _PARSER.parse(source)
        if len(trees) > MAX_TREES:
            trees.popitem(last=False)
    trees.move_to_end(source)
    # Checkers own the returned tree; edits cannot corrupt another rule's view.
    return trees[source].copy()
