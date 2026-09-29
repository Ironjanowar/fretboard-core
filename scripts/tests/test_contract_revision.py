"""Behaviour tests for the frozen adapter API revision and the build wiring.

The artifact metadata's ``api_version`` is not invented by the build script: it is
read from a frozen contract fixture in ``fixtures/contract/api-vN.json``. Three
places therefore have to agree, and a task that changes the exported surface has
to move all three:

* the fixture that records the revision (``api-vN.json``);
* ``scripts/build_aar.sh``, which reads exactly one of them into the artifact's
  metadata;
* ``scripts/tests/test_check_aar.py``, whose pinned revision documents which
  revision the shipped artifact carries.

Leaving one of the three behind is the drift this module rejects. It has happened:
a slice shipped a binding with the URL import and both snapshot codecs reachable
while the metadata it produced still declared the previous revision and reported
``snapshot`` unsupported, which is exactly the "unimplemented feature passed off as
an empty musical result" the capability flags exist to prevent.

These tests read files as text and JSON; they build and execute nothing.
"""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

# The test lives in scripts/tests/, so the repository root is two levels up.
REPO_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_DIR = REPO_ROOT / "fixtures" / "contract"
BUILD_SCRIPT = REPO_ROOT / "scripts" / "build_aar.sh"
CHECKER_TESTS = REPO_ROOT / "scripts" / "tests" / "test_check_aar.py"

REVISION_FILE = re.compile(r"^api-v(?P<revision>\d+)\.json$")
CONTRACT_LINE = re.compile(
    r'^CONTRACT_FILE="\$\{REPO_ROOT\}/fixtures/contract/(?P<file>api-v\d+\.json)"$',
    re.MULTILINE,
)
PINNED_REVISION = re.compile(r"^API_VERSION = (?P<revision>\d+)$", re.MULTILINE)


def frozen_revisions() -> dict[int, Path]:
    """Every frozen revision, keyed by the revision its own file name carries."""
    found: dict[int, Path] = {}
    for path in sorted(CONTRACT_DIR.glob("api-v*.json")):
        match = REVISION_FILE.match(path.name)
        if match is not None:
            found[int(match.group("revision"))] = path
    return found


def shipped_revision() -> int:
    """The revision the build reads into the artifact's metadata."""
    match = CONTRACT_LINE.search(BUILD_SCRIPT.read_text(encoding="utf-8"))
    if match is None:
        raise AssertionError(f"{BUILD_SCRIPT} declares no CONTRACT_FILE")
    return int(REVISION_FILE.match(match.group("file")).group("revision"))


def checker_revision() -> int:
    """The revision the checker's test suite pins as the shipped one."""
    match = PINNED_REVISION.search(CHECKER_TESTS.read_text(encoding="utf-8"))
    if match is None:
        raise AssertionError(f"{CHECKER_TESTS} pins no API_VERSION")
    return int(match.group("revision"))


class FrozenRevisionTests(unittest.TestCase):
    def test_the_frozen_revisions_are_contiguous(self):
        revisions = sorted(frozen_revisions())
        self.assertEqual(
            revisions,
            list(range(1, len(revisions) + 1)),
            "a frozen revision must exist for every number from 1 up, with no gap",
        )

    def test_a_revision_file_declares_its_own_revision(self):
        for revision, path in frozen_revisions().items():
            document = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(
                document["api_version"],
                revision,
                f"{path.name} declares api_version {document['api_version']}",
            )

    def test_the_build_reads_the_newest_frozen_revision(self):
        newest = max(frozen_revisions())
        self.assertEqual(
            shipped_revision(),
            newest,
            "build_aar.sh must read the newest frozen revision, so the artifact's "
            "metadata cannot claim an older surface than the one it carries",
        )

    def test_the_checker_pins_the_shipped_revision(self):
        self.assertEqual(
            checker_revision(),
            shipped_revision(),
            "the checker's pinned revision must document the revision that ships",
        )

    def test_the_shipped_revision_reports_the_snapshot_codec_as_reachable(self):
        newest = max(frozen_revisions())
        capabilities = json.loads(
            frozen_revisions()[newest].read_text(encoding="utf-8")
        )["capabilities"]
        self.assertTrue(
            capabilities["snapshot"],
            f"revision {newest} carries the snapshot codec, so its capability flag "
            "must not report it as unsupported",
        )


if __name__ == "__main__":
    unittest.main()
