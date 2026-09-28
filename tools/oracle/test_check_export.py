"""Behaviour tests for the oracle fixture checker, ``tools/oracle/check_export.py``.

These are *checker* unit tests: every test builds tiny synthetic run
directories in a temporary directory and invokes the frozen CLI as a
subprocess::

    python3 tools/oracle/check_export.py --left <dir> --right <dir> \\
        [--expect-source-commit <40 hex>]

They never import checker internals and never consume real oracle output.
Synthetic fixtures exist only to specify the checker contract; the oracle
fixtures themselves are produced by the pinned Elixir exporter.

Frozen contract (see ``.hermes-tmp/c01-contract.md`` and plan ``04-core-phases.md``
section 2): exit 0 when both runs are consistent, exit 1 otherwise, and each
problem prints one ``ERROR: <message>`` line to stderr. Nothing is printed to
stdout on success.
"""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

# The test lives in tools/oracle/, so the repository root is two levels up.
REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = os.path.join("tools", "oracle", "check_export.py")

# The 14 fixture files of a run directory, in manifest (name ascending) order.
# manifest.json and run-info.json are present in the directory but are never
# listed in manifest["files"].
FIXTURE_FILES = (
    "catalogs.json",
    "chords.jsonl",
    "identify.jsonl",
    "analyzer.jsonl",
    "tunings.jsonl",
    "surfaces.jsonl",
    "scales.jsonl",
    "keys.jsonl",
    "multi-keys.jsonl",
    "progressions.jsonl",
    "page-params.jsonl",
    "query-transport.jsonl",
    "page-events.jsonl",
    "key-groups.jsonl",
)
JSONL_FILES = tuple(name for name in FIXTURE_FILES if name.endswith(".jsonl"))

SOURCE_COMMIT = "a" * 40
OTHER_SOURCE_COMMIT = "b" * 40
EXPORTER_SHA256 = "c" * 64
OTHER_EXPORTER_SHA256 = "d" * 64
ELIXIR_VERSION = "1.19.5"
OTP_VERSION = "27"
CASE_SEED = 0
FIXTURE_SCHEMA_VERSION = 1


# --------------------------------------------------------------------------
# Synthetic fixture construction
# --------------------------------------------------------------------------


def jsonl_bytes(records):
    """Serialize records as canonical, newline-terminated JSONL."""
    return "".join(
        json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
        for record in records
    ).encode("utf-8")


def jsonl_record(case_id, operation, output=None, source_function=None):
    """One canonical JSONL record per the frozen record shape."""
    record = {
        "case_id": case_id,
        "operation": operation,
        "input": {"root": "C"},
        "output": output if output is not None else {"notes": ["C", "E", "G"]},
    }
    if source_function is not None:
        record["baseline_source_function"] = source_function
    return record


def default_contents():
    """Byte contents of the 14 fixture files for a valid synthetic run."""
    contents = {}
    for name in JSONL_FILES:
        operation = "op_" + name[: -len(".jsonl")].replace("-", "_")
        contents[name] = jsonl_bytes(
            [jsonl_record("%s/case1" % operation, operation),
             jsonl_record("%s/case2" % operation, operation)]
        )
    contents["catalogs.json"] = (
        json.dumps(
            {
                "chord_qualities": {"major": 1, "minor": 2},
                "instruments": ["guitar", "ukelele"],
                "progressions": ["I_IV_V"],
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        + b"\n"
    )
    return contents


def record_count(name, data):
    """Actual record count: JSONL lines, or top-level keys for catalogs.json."""
    if name == "catalogs.json":
        return len(json.loads(data.decode("utf-8")))
    return len([line for line in data.decode("utf-8").split("\n") if line.strip()])


def build_manifest(
    contents,
    *,
    source_commit=SOURCE_COMMIT,
    elixir_version=ELIXIR_VERSION,
    otp_version=OTP_VERSION,
    exporter_sha256=EXPORTER_SHA256,
    case_seed=CASE_SEED,
    fixture_schema_version=FIXTURE_SCHEMA_VERSION,
    records_overrides=None,
):
    """A manifest that is self-consistent with ``contents`` unless overridden."""
    records_overrides = records_overrides or {}
    files = []
    for name in sorted(FIXTURE_FILES):
        data = contents[name]
        files.append(
            {
                "name": name,
                "records": records_overrides.get(name, record_count(name, data)),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    return {
        "fixture_schema_version": fixture_schema_version,
        "case_seed": case_seed,
        "source_commit": source_commit,
        "elixir_version": elixir_version,
        "otp_version": otp_version,
        "exporter_sha256": exporter_sha256,
        "files": files,
    }


def default_run_info():
    return {
        "generated_at": "2026-09-28T00:00:00Z",
        "duration_ms": 1234,
        "host": "Linux 6.18.40 x86_64",
        "oracle_out": "dist/oracle-run-a",
        "elixir_version": ELIXIR_VERSION,
        "otp_version": OTP_VERSION,
    }


def make_run(
    root,
    name,
    *,
    contents=None,
    manifest=None,
    records_overrides=None,
    source_commit=SOURCE_COMMIT,
    elixir_version=ELIXIR_VERSION,
    otp_version=OTP_VERSION,
    exporter_sha256=EXPORTER_SHA256,
    case_seed=CASE_SEED,
    fixture_schema_version=FIXTURE_SCHEMA_VERSION,
    run_info=None,
):
    """Materialize a synthetic run directory and return (run_dir, contents, manifest)."""
    run_dir = Path(root) / name
    run_dir.mkdir(parents=True, exist_ok=True)
    if contents is None:
        contents = default_contents()
    for filename, data in contents.items():
        (run_dir / filename).write_bytes(data)
    if manifest is None:
        manifest = build_manifest(
            contents,
            source_commit=source_commit,
            elixir_version=elixir_version,
            otp_version=otp_version,
            exporter_sha256=exporter_sha256,
            case_seed=case_seed,
            fixture_schema_version=fixture_schema_version,
            records_overrides=records_overrides,
        )
    (run_dir / "manifest.json").write_bytes(
        (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode("utf-8")
    )
    (run_dir / "run-info.json").write_bytes(
        (json.dumps(run_info if run_info is not None else default_run_info(),
                    sort_keys=True, indent=2) + "\n").encode("utf-8")
    )
    return run_dir, contents, manifest


def invoke(left, right, extra_args=()):
    """Run the frozen checker CLI against two run directories."""
    args = [
        sys.executable,
        CHECKER,
        "--left", str(left),
        "--right", str(right),
        *extra_args,
    ]
    return subprocess.run(
        args,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )


# --------------------------------------------------------------------------
# Tests
# --------------------------------------------------------------------------


class CheckExportTestCase(unittest.TestCase):
    """Shared setup: every test owns a fresh temporary directory."""

    def setUp(self):
        temp_dir = tempfile.TemporaryDirectory(prefix="oracle-checker-test-")
        self.addCleanup(temp_dir.cleanup)
        self.tmp = Path(temp_dir.name)

    def identical_pair(self, **kwargs):
        """Two byte-identical valid runs; returns (left, right)."""
        left, _, _ = make_run(self.tmp, "left", **kwargs)
        right, _, _ = make_run(self.tmp, "right", **kwargs)
        return left, right

    def assert_accepted(self, proc):
        self.assertEqual(
            0, proc.returncode,
            msg="expected exit 0, got %d; stderr=%r" % (proc.returncode, proc.stderr),
        )
        self.assertEqual(
            "", proc.stdout,
            msg="expected empty stdout on success, got %r" % proc.stdout,
        )
        self.assertEqual(
            "", proc.stderr,
            msg="expected empty stderr on success, got %r" % proc.stderr,
        )

    def assert_rejected(self, proc, context=""):
        self.assertEqual(
            1, proc.returncode,
            msg="expected exit 1 %s, got %d; stderr=%r"
            % (context, proc.returncode, proc.stderr),
        )
        errors = [line for line in proc.stderr.splitlines() if line.startswith("ERROR: ")]
        self.assertTrue(
            errors,
            msg="expected at least one 'ERROR: ' line on stderr %s; stderr=%r"
            % (context, proc.stderr),
        )


class HappyPathTests(CheckExportTestCase):
    def test_identical_runs_pass_with_no_output(self):
        left, right = self.identical_pair()
        self.assert_accepted(invoke(left, right))

    def test_expect_source_commit_matching_both_manifests_is_accepted(self):
        left, right = self.identical_pair()
        self.assert_accepted(
            invoke(left, right, ["--expect-source-commit", SOURCE_COMMIT])
        )

    def test_identical_record_list_order_is_accepted(self):
        contents = default_contents()
        contents["scales.jsonl"] = jsonl_bytes(
            [
                jsonl_record(
                    "op_scales/case1",
                    "op_scales",
                    output={"notes": ["C", "D", "E", "F", "G", "A", "B"]},
                ),
                jsonl_record(
                    "op_scales/case2",
                    "op_scales",
                    output={"notes": ["D", "E", "F#", "G", "A", "B", "C#"]},
                ),
            ]
        )
        left, _, _ = make_run(self.tmp, "left", contents=contents)
        right, _, _ = make_run(self.tmp, "right", contents=dict(contents))
        self.assert_accepted(invoke(left, right))

    def test_run_info_differences_are_not_part_of_the_compared_payload(self):
        left, _, _ = make_run(self.tmp, "left")
        right, _, _ = make_run(
            self.tmp,
            "right",
            run_info={
                "generated_at": "2027-01-01T12:34:56Z",
                "duration_ms": 98765,
                "host": "Darwin 24.0.0 arm64",
                "oracle_out": "dist/oracle-run-b",
                "elixir_version": ELIXIR_VERSION,
                "otp_version": OTP_VERSION,
            },
        )
        self.assert_accepted(invoke(left, right))


class ManifestDiskConsistencyTests(CheckExportTestCase):
    def test_wrong_expected_source_commit_is_rejected(self):
        left, right = self.identical_pair()
        proc = invoke(left, right, ["--expect-source-commit", OTHER_SOURCE_COMMIT])
        self.assert_rejected(proc, "for --expect-source-commit mismatch")

    def test_stale_sha256_after_bytes_changed_is_rejected(self):
        stale_contents = default_contents()
        stale_contents["chords.jsonl"] = jsonl_bytes(
            [
                jsonl_record("op_chords/case1", "op_chords", output={"notes": ["C", "E", "G#"]}),
                jsonl_record("op_chords/case2", "op_chords"),
            ]
        )
        left, _, _ = make_run(self.tmp, "left")
        right, _, _ = make_run(self.tmp, "right")
        # Bytes change on disk after the manifest was written (same line count).
        (left / "chords.jsonl").write_bytes(stale_contents["chords.jsonl"])
        (right / "chords.jsonl").write_bytes(stale_contents["chords.jsonl"])
        self.assert_rejected(
            invoke(left, right), "for a fixture whose bytes changed after the manifest"
        )

    def test_missing_manifest_listed_file_is_rejected(self):
        left, _ = make_run(self.tmp, "left")[:2]
        right, _ = make_run(self.tmp, "right")[:2]
        (left / "analyzer.jsonl").unlink()
        (right / "analyzer.jsonl").unlink()
        self.assert_rejected(
            invoke(left, right), "for a manifest-listed file missing from disk"
        )

    def test_unlisted_extra_file_in_run_directory_is_rejected(self):
        left, _ = make_run(self.tmp, "left")[:2]
        right, _ = make_run(self.tmp, "right")[:2]
        stray = jsonl_bytes([jsonl_record("op_stray/case1", "op_stray")])
        (left / "stray.jsonl").write_bytes(stray)
        (right / "stray.jsonl").write_bytes(stray)
        self.assert_rejected(
            invoke(left, right), "for an unlisted extra file in the run directory"
        )

    def test_manifest_records_disagreeing_with_jsonl_line_count_is_rejected(self):
        left, _, _ = make_run(self.tmp, "left", records_overrides={"chords.jsonl": 3})
        right, _, _ = make_run(self.tmp, "right", records_overrides={"chords.jsonl": 3})
        self.assert_rejected(
            invoke(left, right),
            "for manifest records disagreeing with the JSONL line count",
        )

    def test_manifest_records_disagreeing_with_catalog_key_count_is_rejected(self):
        # default catalogs.json has 3 top-level keys.
        left, _, _ = make_run(self.tmp, "left", records_overrides={"catalogs.json": 5})
        right, _, _ = make_run(self.tmp, "right", records_overrides={"catalogs.json": 5})
        self.assert_rejected(
            invoke(left, right),
            "for manifest records disagreeing with the catalog top-level key count",
        )


class JsonlIntegrityTests(CheckExportTestCase):
    def test_duplicate_case_id_within_a_jsonl_file_is_rejected(self):
        contents = default_contents()
        contents["identify.jsonl"] = jsonl_bytes(
            [
                jsonl_record("op_identify/case1", "op_identify"),
                jsonl_record("op_identify/case1", "op_identify"),
            ]
        )
        left, _, _ = make_run(self.tmp, "left", contents=contents)
        right, _, _ = make_run(self.tmp, "right", contents=dict(contents))
        self.assert_rejected(
            invoke(left, right), "for a duplicate case_id inside a .jsonl file"
        )


class OrderingTests(CheckExportTestCase):
    def test_changed_line_order_between_runs_is_rejected(self):
        left, _, _ = make_run(self.tmp, "left")
        right_contents = default_contents()
        right_contents["scales.jsonl"] = jsonl_bytes(
            list(
                reversed(
                    [
                        jsonl_record("op_scales/case1", "op_scales"),
                        jsonl_record("op_scales/case2", "op_scales"),
                    ]
                )
            )
        )
        right, _, _ = make_run(self.tmp, "right", contents=right_contents)
        self.assert_rejected(
            invoke(left, right), "for the same lines in a different order between runs"
        )


class ProvenanceTests(CheckExportTestCase):
    def test_elixir_version_mismatch_between_runs_is_rejected(self):
        left, _ = make_run(self.tmp, "left")[:2]
        right, _ = make_run(self.tmp, "right", elixir_version="1.20.0")[:2]
        self.assert_rejected(
            invoke(left, right), "for differing elixir_version provenance"
        )

    def test_exporter_sha256_mismatch_between_runs_is_rejected(self):
        left, _ = make_run(self.tmp, "left")[:2]
        right, _ = make_run(self.tmp, "right", exporter_sha256=OTHER_EXPORTER_SHA256)[:2]
        self.assert_rejected(
            invoke(left, right), "for differing exporter_sha256 provenance"
        )

    def test_source_commit_mismatch_between_runs_is_rejected(self):
        left, _ = make_run(self.tmp, "left")[:2]
        right, _ = make_run(self.tmp, "right", source_commit=OTHER_SOURCE_COMMIT)[:2]
        self.assert_rejected(
            invoke(left, right), "for differing source_commit provenance"
        )


# --------------------------------------------------------------------------
# Identify sharding (Amendment 1, 2026-09-28)
#
# ``identify`` is written as contiguous, zero-padded ``identify-<NN>.jsonl``
# shards; a single ``identify.jsonl`` is only the legal degenerate form when
# the whole stream fits in one shard. A shard starts when adding the next
# record would push the current shard past 16 MiB (16 * 1024 * 1024 bytes).
# ``case_id`` uniqueness holds across the whole shard set, not per file.
# --------------------------------------------------------------------------

IDENTIFY_SHARD_LIMIT = 16 * 1024 * 1024


def build_manifest_for_contents(
    contents,
    *,
    source_commit=SOURCE_COMMIT,
    elixir_version=ELIXIR_VERSION,
    otp_version=OTP_VERSION,
    exporter_sha256=EXPORTER_SHA256,
    case_seed=CASE_SEED,
    fixture_schema_version=FIXTURE_SCHEMA_VERSION,
    records_overrides=None,
):
    """A manifest listing exactly ``contents``, whatever shard layout it uses.

    ``build_manifest`` is fixed to the 14-file pre-amendment layout; this
    variant derives ``files`` from the mapping that is actually written to
    disk, so manifest and disk can be made to agree for sharded runs.
    """
    records_overrides = records_overrides or {}
    files = []
    for name in sorted(contents):
        data = contents[name]
        files.append(
            {
                "name": name,
                "records": records_overrides.get(name, record_count(name, data)),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    return {
        "fixture_schema_version": fixture_schema_version,
        "case_seed": case_seed,
        "source_commit": source_commit,
        "elixir_version": elixir_version,
        "otp_version": otp_version,
        "exporter_sha256": exporter_sha256,
        "files": files,
    }


def sharded_identify_contents(shards):
    """Fixture contents with ``identify`` split into the given shards.

    ``shards`` maps a shard filename to the list of ``case_id`` values it
    holds, in generation order, e.g.::

        sharded_identify_contents({
            "identify-01.jsonl": ["op_identify/case1"],
            "identify-02.jsonl": ["op_identify/case2"],
        })
    """
    contents = default_contents()
    del contents["identify.jsonl"]
    for name, case_ids in shards.items():
        contents[name] = jsonl_bytes(
            [jsonl_record(case_id, "op_identify") for case_id in case_ids]
        )
    return contents


def oversized_identify_contents(filler_bytes=IDENTIFY_SHARD_LIMIT):
    """Contents whose single identify shard exceeds the 16 MiB shard limit."""
    contents = default_contents()
    del contents["identify.jsonl"]
    record = jsonl_record(
        "op_identify/huge", "op_identify", output={"blob": "x" * filler_bytes}
    )
    contents["identify-01.jsonl"] = jsonl_bytes([record])
    return contents


def make_run_with(contents, root, name, **kwargs):
    """Materialize a run whose manifest lists exactly ``contents``.

    Returns just the run directory (unlike ``make_run``, whose tuple would
    otherwise capture the fixture bytes in every caller).
    """
    manifest = build_manifest_for_contents(contents, **kwargs)
    run_dir, _, _ = make_run(root, name, contents=contents, manifest=manifest)
    return run_dir


class IdentifyShardingTests(CheckExportTestCase):
    def test_sharded_identify_accepted(self):
        contents = sharded_identify_contents(
            {
                "identify-01.jsonl": ["op_identify/case1"],
                "identify-02.jsonl": ["op_identify/case2"],
            }
        )
        left = make_run_with(contents, self.tmp, "left")
        right = make_run_with(dict(contents), self.tmp, "right")
        self.assert_accepted(invoke(left, right))

    def test_contiguous_numbering_gap_is_rejected(self):
        contents = sharded_identify_contents(
            {
                "identify-01.jsonl": ["op_identify/case1"],
                "identify-03.jsonl": ["op_identify/case2"],
            }
        )
        left = make_run_with(contents, self.tmp, "left")
        right = make_run_with(dict(contents), self.tmp, "right")
        self.assert_rejected(
            invoke(left, right), "for a gap in identify shard numbering"
        )

    def test_case_id_duplicated_across_shards_is_rejected(self):
        contents = sharded_identify_contents(
            {
                "identify-01.jsonl": ["op_identify/case1"],
                "identify-02.jsonl": ["op_identify/case1"],
            }
        )
        left = make_run_with(contents, self.tmp, "left")
        right = make_run_with(dict(contents), self.tmp, "right")
        self.assert_rejected(
            invoke(left, right),
            "for a case_id duplicated across the identify shard set",
        )

    def test_oversized_identify_shard_is_rejected(self):
        contents = oversized_identify_contents()
        # The shard really is over the frozen limit; the test would be vacuous
        # otherwise. The filler is one JSONL line, so it is a valid record.
        self.assertGreater(
            len(contents["identify-01.jsonl"]), IDENTIFY_SHARD_LIMIT
        )
        left = make_run_with(contents, self.tmp, "left")
        right = make_run_with(dict(contents), self.tmp, "right")
        self.assert_rejected(
            invoke(left, right), "for an identify shard larger than 16 MiB"
        )

    def test_missing_required_base_fixture_is_rejected(self):
        # Manifest and disk agree with each other: scales.jsonl is absent from
        # both. The 13 fixed fixtures are still required.
        contents = default_contents()
        del contents["scales.jsonl"]
        left = make_run_with(contents, self.tmp, "left")
        right = make_run_with(dict(contents), self.tmp, "right")
        self.assert_rejected(
            invoke(left, right),
            "for a missing required base fixture that manifest and disk agree on",
        )


if __name__ == "__main__":
    unittest.main()
