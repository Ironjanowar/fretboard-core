#!/usr/bin/env python3
"""Oracle fixture checker for task C01.

Frozen CLI (see ``.hermes-tmp/c01-contract.md``)::

    python3 tools/oracle/check_export.py --left DIR --right DIR \\
        [--expect-source-commit <40 hex>]

Exit 0 when both runs are consistent, exit 1 otherwise. Every problem is
reported as one ``ERROR: <message>`` line on stderr; stdout stays empty.

A *run directory* is a fixture export containing ``manifest.json``,
``run-info.json`` and the fixture files. ``manifest.json`` and
``run-info.json`` are never part of the compared payload; every other file in
the directory must be listed in ``manifest.json``, and vice versa.

Identify is sharded (Amendment 1 of the contract): the identify stream is
written as ``identify.jsonl`` or as contiguous ``identify-NN.jsonl`` shards of
at most 16 MiB each, and ``case_id`` uniqueness holds across the whole shard
set.

Python 3 standard library only.
"""

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path

# The 13 fixed (unsharded) fixtures that every run directory must contain.
FIXED_FIXTURES = (
    "catalogs.json",
    "chords.jsonl",
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

CATALOGS_FIXTURE = "catalogs.json"
IDENTIFY_PATTERN = re.compile(r"^identify(?:-(\d{2}))?\.jsonl$")
MAX_IDENTIFY_SHARD_BYTES = 16 * 1024 * 1024

# Files that live in a run directory but are never fixtures.
NON_FIXTURE_FILES = frozenset({"manifest.json", "run-info.json"})

# Manifest fields that must agree between the two runs.
PROVENANCE_FIELDS = (
    "fixture_schema_version",
    "case_seed",
    "source_commit",
    "elixir_version",
    "otp_version",
    "exporter_sha256",
)

READ_CHUNK_BYTES = 1024 * 1024


class RunInspection:
    """What the per-run checks learned about one run directory."""

    def __init__(self, label, run_dir, manifest, listed_names, disk_names):
        self.label = label
        self.run_dir = run_dir
        self.manifest = manifest
        self.listed_names = listed_names
        self.disk_names = disk_names


def fail(errors, message):
    """Record one problem message (printed as ``ERROR: <message>``)."""
    errors.append(message)


def sha256_of_file(path):
    """SHA-256 of the file's exact bytes."""
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(READ_CHUNK_BYTES), b""):
            digest.update(chunk)
    return digest.hexdigest()


def jsonl_record_count(path):
    """Number of JSONL records: non-blank lines."""
    count = 0
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            if line.strip():
                count += 1
    return count


def catalog_key_count(path):
    """Number of top-level keys of the catalogs.json JSON object."""
    with open(path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError("catalogs.json must contain a top-level JSON object")
    return len(data)


def actual_record_count(path, name):
    """Records of one fixture per the frozen rule."""
    if name == CATALOGS_FIXTURE:
        return catalog_key_count(path)
    return jsonl_record_count(path)


def disk_fixture_names(run_dir, label, errors):
    """Every file in the run directory except manifest.json and run-info.json."""
    if not run_dir.is_dir():
        fail(errors, "%s: run directory %s does not exist" % (label, run_dir))
        return []
    names = []
    for name in sorted(os.listdir(run_dir)):
        if name in NON_FIXTURE_FILES:
            continue
        if (run_dir / name).is_file():
            names.append(name)
    return names


def load_manifest(run_dir, label, errors):
    """Load and shape-check manifest.json; returns the parsed object or None."""
    path = run_dir / "manifest.json"
    if not path.is_file():
        fail(errors, "%s: manifest.json is missing from %s" % (label, run_dir))
        return None
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        fail(errors, "%s: manifest.json is not readable JSON: %s" % (label, exc))
        return None
    if not isinstance(manifest, dict):
        fail(errors, "%s: manifest.json must contain a JSON object" % label)
        return None
    for field in PROVENANCE_FIELDS:
        if field not in manifest:
            fail(errors, "%s: manifest.json is missing required field %r" % (label, field))
    return manifest


def manifest_file_names(manifest, label, errors):
    """Validated ``files`` names, in manifest order."""
    entries = manifest.get("files")
    if not isinstance(entries, list):
        fail(errors, "%s: manifest.json field 'files' must be a JSON array" % label)
        return []
    names = []
    for entry in entries:
        if not isinstance(entry, dict):
            fail(errors, "%s: manifest.json files entry is not a JSON object: %r" % (label, entry))
            continue
        name = entry.get("name")
        if not isinstance(name, str) or not name:
            fail(errors, "%s: manifest.json files entry has no string 'name': %r" % (label, entry))
            continue
        if name in NON_FIXTURE_FILES:
            fail(errors, "%s: manifest.json must not list %s as a fixture" % (label, name))
            continue
        names.append(name)
    for name in sorted(set(names)):
        if names.count(name) > 1:
            fail(errors, "%s: manifest.json lists %s more than once" % (label, name))
    if names != sorted(names):
        fail(
            errors,
            "%s: manifest.json 'files' is not sorted by name ascending: %s"
            % (label, ", ".join(names)),
        )
    return names


def check_listing_matches_disk(listed_names, disk_names, label, errors):
    """manifest.files must be exactly the fixture files present on disk."""
    listed = set(listed_names)
    present = set(disk_names)
    for name in sorted(listed - present):
        fail(
            errors,
            "%s: manifest.json lists %s but no such file is in the run directory"
            % (label, name),
        )
    for name in sorted(present - listed):
        fail(
            errors,
            "%s: run directory contains %s, which manifest.json does not list"
            % (label, name),
        )


def check_file_entry(run_dir, entry, label, errors):
    """Check one manifest files entry against the bytes on disk."""
    name = entry["name"] if isinstance(entry.get("name"), str) else None
    if name is None:
        return
    path = run_dir / name
    if not path.is_file():
        return  # Reported by check_listing_matches_disk.
    try:
        digest = sha256_of_file(path)
    except OSError as exc:
        fail(errors, "%s: %s could not be read: %s" % (label, name, exc))
        return
    if entry.get("sha256") != digest:
        fail(
            errors,
            "%s: %s sha256 is %r in manifest.json but %s on disk"
            % (label, name, entry.get("sha256"), digest),
        )
    try:
        records = actual_record_count(path, name)
    except (OSError, ValueError) as exc:
        fail(errors, "%s: %s records could not be counted: %s" % (label, name, exc))
        return
    if entry.get("records") != records:
        fail(
            errors,
            "%s: %s records is %r in manifest.json but %d on disk"
            % (label, name, entry.get("records"), records),
        )


def check_fixed_fixtures_present(disk_names, label, errors):
    """All 13 fixed fixtures must exist in the run directory."""
    present = set(disk_names)
    for name in FIXED_FIXTURES:
        if name not in present:
            fail(errors, "%s: required fixture %s is missing from the run directory" % (label, name))


def identify_file_names(disk_names):
    """Identify fixture names present on disk."""
    return sorted(name for name in disk_names if IDENTIFY_PATTERN.match(name))


def check_identify_shards(run_dir, disk_names, label, errors):
    """Identify form, shard numbering contiguity and the 16 MiB shard limit."""
    names = identify_file_names(disk_names)
    if not names:
        fail(
            errors,
            "%s: no identify fixture found; expected identify.jsonl or identify-NN.jsonl"
            % label,
        )
        return
    numbered = []
    for name in names:
        match = IDENTIFY_PATTERN.match(name)
        if match.group(1):
            numbered.append(int(match.group(1)))
        try:
            size = (run_dir / name).stat().st_size
        except OSError as exc:
            fail(errors, "%s: %s could not be stat'ed: %s" % (label, name, exc))
            continue
        if size > MAX_IDENTIFY_SHARD_BYTES:
            fail(
                errors,
                "%s: identify shard %s is %d bytes, over the %d byte shard limit"
                % (label, name, size, MAX_IDENTIFY_SHARD_BYTES),
            )
    if not numbered:
        return
    if "identify.jsonl" in names:
        fail(
            errors,
            "%s: identify.jsonl is present alongside numbered shards; use one form only"
            % label,
        )
    numbered.sort()
    if numbered != list(range(1, len(numbered) + 1)):
        fail(
            errors,
            "%s: identify shard numbering is not contiguous from 01: found %s"
            % (label, ", ".join("identify-%02d.jsonl" % number for number in numbered)),
        )


def read_jsonl_case_ids(path, label, name, errors):
    """case_id per line of one JSONL file, in order."""
    case_ids = []
    try:
        handle = open(path, "r", encoding="utf-8")
    except OSError as exc:
        fail(errors, "%s: %s could not be read: %s" % (label, name, exc))
        return case_ids
    with handle:
        for number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except ValueError as exc:
                fail(errors, "%s: %s line %d is not valid JSON: %s" % (label, name, number, exc))
                continue
            case_id = record.get("case_id") if isinstance(record, dict) else None
            if not isinstance(case_id, str) or not case_id:
                fail(errors, "%s: %s line %d has no string 'case_id'" % (label, name, number))
                continue
            if case_id in case_ids:
                fail(
                    errors,
                    "%s: %s has duplicate case_id %r (line %d)"
                    % (label, name, case_id, number),
                )
                continue
            case_ids.append(case_id)
    return case_ids


def check_case_ids_unique(run_dir, names, label, errors):
    """case_id uniqueness within each file and across the identify shard set."""
    shard_ids = {}
    for name in names:
        if not name.endswith(".jsonl"):
            continue
        case_ids = read_jsonl_case_ids(run_dir / name, label, name, errors)
        if IDENTIFY_PATTERN.match(name):
            shard_ids[name] = case_ids
    seen = {}
    for name in sorted(shard_ids):
        for case_id in shard_ids[name]:
            if case_id in seen:
                fail(
                    errors,
                    "%s: identify case_id %r appears in both %s and %s"
                    % (label, case_id, seen[case_id], name),
                )
            else:
                seen[case_id] = name


def inspect_run(run_dir, label, errors):
    """All per-run checks: manifest vs disk, fixtures, identify, case_ids."""
    disk_names = disk_fixture_names(run_dir, label, errors)
    manifest = load_manifest(run_dir, label, errors)
    listed_names = []
    if manifest is not None:
        listed_names = manifest_file_names(manifest, label, errors)
        if not isinstance(manifest.get("files"), list):
            listed_names = []
        check_listing_matches_disk(listed_names, disk_names, label, errors)
        entries = manifest.get("files")
        if isinstance(entries, list):
            for entry in entries:
                if isinstance(entry, dict) and isinstance(entry.get("name"), str):
                    check_file_entry(run_dir, entry, label, errors)
    check_fixed_fixtures_present(disk_names, label, errors)
    check_identify_shards(run_dir, disk_names, label, errors)
    if run_dir.is_dir():
        present = set(disk_names)
        names_to_check = set(name for name in listed_names if name in present)
        names_to_check |= set(identify_file_names(disk_names))
        check_case_ids_unique(run_dir, sorted(names_to_check), label, errors)
    return RunInspection(label, run_dir, manifest, listed_names, disk_names)


def manifest_entries_by_name(manifest):
    """Map fixture name -> manifest entry."""
    entries = manifest.get("files") if isinstance(manifest, dict) else None
    if not isinstance(entries, list):
        return {}
    return {
        entry["name"]: entry
        for entry in entries
        if isinstance(entry, dict) and isinstance(entry.get("name"), str)
    }


def compare_provenance(left, right, errors):
    """Provenance fields and per-file records/sha256 must agree between runs."""
    if left.manifest is None or right.manifest is None:
        return
    for field in PROVENANCE_FIELDS:
        if left.manifest.get(field) != right.manifest.get(field):
            fail(
                errors,
                "manifest.json %s differs between runs: left=%r right=%r"
                % (field, left.manifest.get(field), right.manifest.get(field)),
            )
    left_entries = manifest_entries_by_name(left.manifest)
    right_entries = manifest_entries_by_name(right.manifest)
    for name in sorted(set(left_entries) | set(right_entries)):
        if name not in left_entries:
            fail(errors, "manifest.json files differ between runs: %s is only in right" % name)
        elif name not in right_entries:
            fail(errors, "manifest.json files differ between runs: %s is only in left" % name)
        else:
            for field in ("records", "sha256"):
                if left_entries[name].get(field) != right_entries[name].get(field):
                    fail(
                        errors,
                        "manifest.json %s entry %s differs between runs: left=%r right=%r"
                        % (name, field, left_entries[name].get(field), right_entries[name].get(field)),
                    )


def files_are_identical(left_path, right_path):
    """True when the two files have identical bytes."""
    if left_path.stat().st_size != right_path.stat().st_size:
        return False
    with open(left_path, "rb") as left_handle, open(right_path, "rb") as right_handle:
        while True:
            left_chunk = left_handle.read(READ_CHUNK_BYTES)
            right_chunk = right_handle.read(READ_CHUNK_BYTES)
            if left_chunk != right_chunk:
                return False
            if not left_chunk:
                return True


def compare_fixture_bytes(left, right, errors):
    """Fixture bytes must be identical between the two runs."""
    left_names = set(left.disk_names)
    right_names = set(right.disk_names)
    for name in sorted(left_names ^ right_names):
        side = "right" if name in right_names else "left"
        fail(errors, "fixture files differ between runs: %s is only in %s" % (name, side))
    for name in sorted(left_names & right_names):
        try:
            identical = files_are_identical(left.run_dir / name, right.run_dir / name)
        except OSError as exc:
            fail(errors, "%s could not be read for byte comparison: %s" % (name, exc))
            continue
        if not identical:
            fail(errors, "%s bytes differ between the two runs" % name)


def check_expected_source_commit(expected, runs, errors):
    """--expect-source-commit must match both manifests."""
    for run in runs:
        if run.manifest is None:
            continue
        actual = run.manifest.get("source_commit")
        if actual != expected:
            fail(
                errors,
                "%s: manifest.json source_commit is %r, expected %r"
                % (run.label, actual, expected),
            )


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Check that two oracle fixture runs are consistent.",
    )
    parser.add_argument("--left", required=True, help="left run directory")
    parser.add_argument("--right", required=True, help="right run directory")
    parser.add_argument(
        "--expect-source-commit",
        default=None,
        help="expected 40 hex source commit for both manifests",
    )
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    errors = []
    left = inspect_run(Path(args.left), "left", errors)
    right = inspect_run(Path(args.right), "right", errors)
    compare_provenance(left, right, errors)
    compare_fixture_bytes(left, right, errors)
    if args.expect_source_commit is not None:
        check_expected_source_commit(args.expect_source_commit, (left, right), errors)
    for message in errors:
        print("ERROR: %s" % message, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
