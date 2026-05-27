from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
NEXT_ID = REPO_ROOT / 'scripts' / 'roadmap-next-id.sh'


def run_next_id(roadmap: Path, script: Path = NEXT_ID) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ['bash', str(script), str(roadmap)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


class RoadmapHelperTests(unittest.TestCase):
    def test_roadmap_next_id_prints_only_next_id_after_duplicate_check(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            roadmap = Path(temp_dir) / 'ROADMAP.md'
            roadmap.write_text('721. old\n723. helper era\n724. guard\n')

            result = run_next_id(roadmap)

        self.assertEqual(0, result.returncode)
        self.assertEqual('725\n', result.stdout)
        self.assertEqual('', result.stderr)

    def test_roadmap_next_id_fails_fast_on_helper_era_duplicate(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            roadmap = Path(temp_dir) / 'ROADMAP.md'
            roadmap.write_text('722. legacy\n999. first\n999. duplicate\n')

            result = run_next_id(roadmap)

        self.assertNotEqual(0, result.returncode)
        self.assertEqual('', result.stdout)
        self.assertIn('duplicate ROADMAP numeric id(s)', result.stderr)
        self.assertIn('999', result.stderr)
        self.assertNotIn('1000', result.stdout)

    def test_roadmap_next_id_fails_closed_when_checker_is_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            script_dir = Path(temp_dir) / 'scripts'
            script_dir.mkdir()
            copied_next_id = script_dir / 'roadmap-next-id.sh'
            shutil.copy2(NEXT_ID, copied_next_id)
            roadmap = Path(temp_dir) / 'ROADMAP.md'
            roadmap.write_text('724. guard\n')

            result = run_next_id(roadmap, copied_next_id)

        self.assertNotEqual(0, result.returncode)
        self.assertEqual('', result.stdout)
        self.assertIn('required ROADMAP id checker not found or not readable', result.stderr)
        self.assertIn('refusing to print a next id', result.stderr)


if __name__ == '__main__':
    unittest.main()
