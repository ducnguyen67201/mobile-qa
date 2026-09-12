"""Regression check for content-only generation and nonmutating drift detection."""

import tempfile
import unittest
from pathlib import Path

from scripts.contracts import synchronize


class ExportTests(unittest.TestCase):
    def test_content_sync_preserves_mtime_and_detects_add_change_delete(self):
        with (
            tempfile.TemporaryDirectory() as left,
            tempfile.TemporaryDirectory() as right,
        ):
            staged, dest = Path(left), Path(right)
            source = staged / "apps/web/src/api/generated/types.gen.ts"
            source.parent.mkdir(parents=True)
            source.write_text("original")
            self.assertEqual(len(synchronize(staged, dest, False)), 1)
            target = dest / source.relative_to(staged)
            before = target.stat().st_mtime_ns
            self.assertEqual(synchronize(staged, dest, False), [])
            self.assertEqual(target.stat().st_mtime_ns, before)
            source.write_text("changed")
            self.assertEqual(len(synchronize(staged, dest, True)), 1)
            self.assertEqual(target.read_text(), "original")
            source.unlink()
            self.assertEqual(len(synchronize(staged, dest, True)), 1)
            synchronize(staged, dest, False)
            self.assertFalse(target.exists())
            source.write_text("new")
            self.assertEqual(len(synchronize(staged, dest, True)), 1)


if __name__ == "__main__":
    unittest.main()
