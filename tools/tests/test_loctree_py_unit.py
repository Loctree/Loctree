import os
import sys
import tempfile
import textwrap
from pathlib import Path
import unittest

# Add repo root to sys.path so we can import loctree.py
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import loctree


class ParseExtensionsTests(unittest.TestCase):
    def test_empty_string_returns_none(self) -> None:
        self.assertIsNone(loctree.parse_extensions(""))

    def test_trims_dots_and_whitespace_and_lowercases(self) -> None:
        result = loctree.parse_extensions("  .Py, RS , ,  Tsx  ")
        # Order is not important, but the set contents are.
        self.assertEqual(result, {"py", "rs", "tsx"})


class HiddenFilesTests(unittest.TestCase):
    def _make_options(self, root: Path, show_hidden: bool) -> loctree.Options:
        return loctree.Options(
            extensions=None,
            ignore_paths=[],
            use_gitignore=False,
            max_depth=None,
            color="never",
            output="human",
            summary=False,
            summary_limit=5,
            show_hidden=show_hidden,
            loc_threshold=loctree.DEFAULT_LOC_THRESHOLD,
        )

    def test_ds_store_treated_like_other_hidden_files(self) -> None:
        """.DS_Store should behave like any other dotfile.

        - When show_hidden is False, it is omitted.
        - When show_hidden is True, it is included in the listing and stats.
        """

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)

            # Visible file
            (root / "visible.txt").write_text("line1\nline2\n")
            # Generic hidden file
            (root / ".hidden.txt").write_text("one\n")
            # macOS metadata file we want to treat as a normal hidden file
            (root / ".DS_Store").write_text("ds-store\n")

            # 1) Hidden files off: only visible.txt should be counted
            options = self._make_options(root, show_hidden=False)
            lines, large_entries, stats = loctree.collect_lines(root, options)

            self.assertEqual(stats["files"], 1)
            self.assertEqual(stats["totalLoc"], 2)
            paths = {entry[2] for entry in lines}
            self.assertIn("visible.txt", paths)
            self.assertNotIn(".hidden.txt", paths)
            self.assertNotIn(".DS_Store", paths)

            # 2) Hidden files on: all three files should be visible to the collector
            options = self._make_options(root, show_hidden=True)
            lines, large_entries, stats = loctree.collect_lines(root, options)

            self.assertEqual(stats["files"], 3)
            # 2 (visible.txt) + 1 (.hidden.txt) + 1 (.DS_Store)
            self.assertEqual(stats["totalLoc"], 4)
            paths = {entry[2] for entry in lines}
            self.assertIn("visible.txt", paths)
            self.assertIn(".hidden.txt", paths)
            self.assertIn(".DS_Store", paths)


if __name__ == "__main__":  # pragma: no cover - convenience for local runs
    unittest.main()
