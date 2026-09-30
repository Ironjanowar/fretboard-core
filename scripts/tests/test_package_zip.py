"""Public CLI tests for deterministic ZIP packaging."""

import os
import stat
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
PACKAGER = os.path.join("scripts", "package_zip.py")
FIXED_ZIP_TIMESTAMP = (1980, 1, 1, 0, 0, 0)
FIXED_FILE_MODE = stat.S_IFREG | 0o644


class PackageZipCliTests(unittest.TestCase):
    def setUp(self):
        temporary_directory = tempfile.TemporaryDirectory(prefix="package-zip-test-")
        self.addCleanup(temporary_directory.cleanup)
        self.tmp = Path(temporary_directory.name)
        self.source = self.tmp / "input"
        (self.source / "nested").mkdir(parents=True)

        # Create files in deliberately non-lexical order. The packager must not
        # expose filesystem traversal order in the resulting archive.
        last = self.source / "z-last.txt"
        middle = self.source / "nested" / "middle.bin"
        first = self.source / "a-first.txt"
        last.write_bytes(b"last\n")
        middle.write_bytes(b"\x00\xffpayload")
        first.write_bytes(b"first\n")

        # Source modes intentionally disagree; archive permissions are canonical
        # metadata, not a copy of the caller's filesystem permissions.
        last.chmod(0o755)
        middle.chmod(0o600)
        first.chmod(0o640)

    def package(self, output):
        result = subprocess.run(
            [sys.executable, PACKAGER, str(self.source), str(output)],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            0,
            result.returncode,
            msg="packager failed with stderr=%r and stdout=%r"
            % (result.stderr, result.stdout),
        )

    def set_source_mtime(self, timestamp):
        for path in [self.source, *self.source.rglob("*")]:
            os.utime(path, (timestamp, timestamp))

    def test_archive_is_canonical_and_independent_of_source_mtimes(self):
        first_archive = self.tmp / "first.zip"
        second_archive = self.tmp / "second.zip"

        self.set_source_mtime(978307200)  # 2001-01-01T00:00:00Z
        self.package(first_archive)
        self.set_source_mtime(1735689600)  # 2025-01-01T00:00:00Z
        self.package(second_archive)

        self.assertEqual(first_archive.read_bytes(), second_archive.read_bytes())

        expected_contents = {
            "a-first.txt": b"first\n",
            "nested/middle.bin": b"\x00\xffpayload",
            "z-last.txt": b"last\n",
        }
        with zipfile.ZipFile(first_archive) as archive:
            entries = archive.infolist()
            self.assertEqual(sorted(expected_contents), [entry.filename for entry in entries])
            self.assertEqual(
                expected_contents,
                {entry.filename: archive.read(entry) for entry in entries},
            )
            for entry in entries:
                with self.subTest(entry=entry.filename):
                    self.assertEqual(FIXED_ZIP_TIMESTAMP, entry.date_time)
                    self.assertEqual(FIXED_FILE_MODE, entry.external_attr >> 16)


if __name__ == "__main__":
    unittest.main()
