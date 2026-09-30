#!/usr/bin/env python3
"""Check one Android engine artifact (``fretboard-engine-<version>.aar``).

Frozen contract (see ``scripts/tests/test_check_aar.py``):

* ``--aar <path> --manifest <path>`` is the only invocation.
* Exit 0 and print **nothing** when the artifact conforms.
* Exit 1 and print one ``ERROR: <message>`` line per problem to stderr
  otherwise; stdout stays empty in both cases.

The artifact carries a copy of the published metadata at
``META-INF/fretboard-engine/metadata.json``, so the sidecar is checked against
the artifact *bytes* and not only against the filename. The ten frozen metadata
fields are listed in ``FROZEN_FIELDS`` below and must agree everywhere.

The checker requires the published ABI set and validates that the sidecar,
embedded metadata, and archive layout agree. The frozen API revision itself
(currently 8, ``fixtures/contract/api-v8.json``) is the caller's value, read by
``scripts/build_aar.sh`` from the contract and pinned in the test suite.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import zipfile

# The generated binding package, frozen with the adapter API (uniffi.toml).
BINDING_PACKAGE = "dev.ironjanowar.fretboard.core"

# The native architectures carried by every published artifact.
SHIPPED_ABIS = ["arm64-v8a", "x86_64"]

ELF_MACHINES = {
    "arm64-v8a": ("EM_AARCH64", 183),
    "x86_64": ("EM_X86_64", 62),
}

# The native library the engine crate produces (cdylib name in Cargo.toml).
NATIVE_LIBRARY = "libfretboard_mobile_ffi.so"

# Where the artifact carries its own metadata copy.
EMBEDDED_METADATA_PATH = "META-INF/fretboard-engine/metadata.json"

# The consumer shrinker rules the artifact must ship.
CONSUMER_RULES = "proguard.txt"

# The artifact's manifest, required in the archive.
ANDROID_MANIFEST = "AndroidManifest.xml"

# The compiled binding classes.
CLASSES_JAR = "classes.jar"

FROZEN_FIELDS = (
    "artifact_version",
    "source_commit",
    "tool_versions",
    "uniffi_runtime_dependency",
    "api_version",
    "snapshot_schema_version",
    "binding_package",
    "abis",
    "min_sdk",
    "licenses",
)

ARTIFACT_NAME = re.compile(r"^fretboard-engine-(?P<version>.+)\.aar$")
SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")
MAVEN_COORDINATES = re.compile(r"^[^:\s]+:[^:\s]+:[^:\s]+$")


class Report:
    """Collects the problems found in one artifact."""

    def __init__(self) -> None:
        self.problems: list[str] = []

    def reject(self, message: str) -> None:
        """Record one problem; the checker never raises on bad input."""
        self.problems.append(message)

    def emit(self) -> int:
        """Print the problems and return the process exit code."""
        for problem in self.problems:
            print(f"ERROR: {problem}", file=sys.stderr)
        return 1 if self.problems else 0


def read_json(path: str, report: Report, what: str):
    """Read one JSON document, rejecting instead of raising."""
    try:
        with open(path, "rb") as handle:
            return json.loads(handle.read().decode("utf-8"))
    except FileNotFoundError:
        report.reject(f"{what} is missing: {path}")
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        report.reject(f"{what} cannot be read as JSON: {path}: {error}")
    return None


def require_fields(document, report: Report, what: str) -> bool:
    """Check that every frozen field is present in one metadata document."""
    if not isinstance(document, dict):
        report.reject(f"{what} must be a JSON object")
        return False
    complete = True
    for field in FROZEN_FIELDS:
        if field not in document:
            report.reject(f"{what} is missing the required field {field!r}")
            complete = False
    return complete


def check_field_types(document, report: Report, what: str) -> None:
    """Check the frozen types of the metadata fields that are present."""
    if not isinstance(document, dict):
        return

    version = document.get("artifact_version")
    if not isinstance(version, str) or not version:
        report.reject(f"{what} artifact_version must be a non-empty string")

    commit = document.get("source_commit")
    if not isinstance(commit, str) or not SOURCE_COMMIT.match(commit):
        report.reject(
            f"{what} source_commit must be 40 lowercase hex characters"
        )

    tool_versions = document.get("tool_versions")
    if not isinstance(tool_versions, dict) or not tool_versions:
        report.reject(f"{what} tool_versions must be a non-empty object")
    elif not all(
        isinstance(name, str) and isinstance(value, str) and name and value
        for name, value in tool_versions.items()
    ):
        report.reject(
            f"{what} tool_versions must map non-empty tool names to versions"
        )

    dependency = document.get("uniffi_runtime_dependency")
    if not isinstance(dependency, str) or not MAVEN_COORDINATES.match(dependency):
        report.reject(
            f"{what} uniffi_runtime_dependency must be Maven coordinates "
            "'group:artifact:version'"
        )

    for field in ("api_version", "snapshot_schema_version", "min_sdk"):
        value = document.get(field)
        if not isinstance(value, int) or isinstance(value, bool):
            report.reject(f"{what} {field} must be an integer")

    package = document.get("binding_package")
    if package != BINDING_PACKAGE:
        report.reject(
            f"{what} binding_package must be {BINDING_PACKAGE!r}, got {package!r}"
        )

    abis = document.get("abis")
    if not isinstance(abis, list) or not abis:
        report.reject(f"{what} abis must be a non-empty list")
    elif not all(isinstance(abi, str) and abi for abi in abis):
        report.reject(f"{what} abis must carry non-empty ABI names")
    elif len(set(abis)) != len(abis):
        report.reject(f"{what} abis must not repeat an ABI")
    elif abis != SHIPPED_ABIS:
        report.reject(f"{what} abis must be exactly {SHIPPED_ABIS!r}, got {abis!r}")

    licenses = document.get("licenses")
    if not isinstance(licenses, list) or not licenses:
        report.reject(f"{what} licenses must be a non-empty list")
    elif not all(isinstance(license_id, str) and license_id for license_id in licenses):
        report.reject(f"{what} licenses must carry non-empty license ids")


def compare_documents(manifest, embedded, report: Report) -> None:
    """Check that the sidecar and the artifact's own copy agree field by field."""
    for field in FROZEN_FIELDS:
        if field not in manifest or field not in embedded:
            continue
        if manifest[field] != embedded[field]:
            report.reject(
                f"the published manifest disagrees with the artifact metadata on "
                f"{field!r}: {manifest[field]!r} != {embedded[field]!r}"
            )


def duplicate_names(names):
    """Return sorted ZIP member names that occur more than once."""
    seen = set()
    duplicates = set()
    for name in names:
        if name in seen:
            duplicates.add(name)
        seen.add(name)
    return sorted(duplicates)


def check_elf_header(data: bytes, path: str, abi: str, report: Report) -> None:
    """Check the ELF identity, object type, and machine for one native library."""
    if len(data) < 64:
        report.reject(f"{path} has a truncated ELF header ({len(data)} bytes)")
        return
    if data[:4] != b"\x7fELF":
        report.reject(f"{path} is not an ELF file")
        return
    if data[4] != 2:
        report.reject(f"{path} must be ELF64 (EI_CLASS 2), got {data[4]}")
        return
    if data[5] != 1:
        report.reject(f"{path} must be little-endian (EI_DATA 1), got {data[5]}")
        return
    object_type = int.from_bytes(data[16:18], "little")
    if object_type != 3:
        report.reject(f"{path} must be a shared object (ET_DYN), got e_type {object_type}")
        return

    machine_spec = ELF_MACHINES.get(abi)
    if machine_spec is None:
        return
    machine_name, machine = machine_spec
    actual = int.from_bytes(data[18:20], "little")
    if actual != machine:
        report.reject(
            f"{path} must use {machine_name} ({machine}) for {abi}, got e_machine {actual}"
        )


def check_native_libraries(archive, names, abis, report: Report) -> None:
    """Check the ``jni/<abi>/`` layout against the declared ABI set."""
    declared = set(abis)
    present: dict[str, list[str]] = {}
    for name in names:
        parts = name.split("/")
        if len(parts) != 3 or parts[0] != "jni":
            continue
        present.setdefault(parts[1], []).append(parts[2])

    for abi in sorted(declared):
        libraries = present.get(abi, [])
        if NATIVE_LIBRARY not in libraries:
            report.reject(
                f"the declared ABI {abi!r} has no jni/{abi}/{NATIVE_LIBRARY}"
            )
        for library in libraries:
            path = f"jni/{abi}/{library}"
            try:
                check_elf_header(archive.read(path), path, abi, report)
            except (KeyError, OSError, RuntimeError, zipfile.BadZipFile) as error:
                report.reject(f"{path} cannot be read: {error}")
            if library != NATIVE_LIBRARY:
                report.reject(
                    f"unexpected native library {path}; the engine "
                    f"artifact carries only {NATIVE_LIBRARY}"
                )

    for abi in sorted(set(present) - declared):
        report.reject(
            f"native libraries are present for ABI {abi!r}, which the metadata "
            "does not declare"
        )

    if not present:
        report.reject("the artifact carries no jni/<abi>/ native libraries")


def check_binding_classes(archive, names, report: Report) -> None:
    """Check that ``classes.jar`` holds binding classes in the frozen package."""
    if CLASSES_JAR not in names:
        report.reject(f"the artifact has no {CLASSES_JAR}")
        return

    prefix = BINDING_PACKAGE.replace(".", "/") + "/"
    try:
        with archive.open(CLASSES_JAR) as handle:
            with zipfile.ZipFile(handle) as classes:
                entries = classes.namelist()
    except (KeyError, zipfile.BadZipFile, OSError) as error:
        report.reject(f"{CLASSES_JAR} cannot be read: {error}")
        return

    generated = [
        entry
        for entry in entries
        if entry.startswith(prefix) and entry.endswith(".class")
    ]
    if not generated:
        report.reject(
            f"{CLASSES_JAR} carries no generated binding class under "
            f"{BINDING_PACKAGE}"
        )


def check_artifact(path: str, manifest_path: str) -> int:
    """Check one artifact and its published manifest; returns the exit code."""
    report = Report()

    match = ARTIFACT_NAME.match(path.rsplit("/", 1)[-1])
    if match is None:
        report.reject(
            f"the artifact file name must be fretboard-engine-<version>.aar, "
            f"got {path.rsplit('/', 1)[-1]!r}"
        )
    filename_version = match.group("version") if match else None

    manifest = read_json(manifest_path, report, "the published manifest")
    if manifest is not None:
        require_fields(manifest, report, "the published manifest")
        check_field_types(manifest, report, "the published manifest")
        if (
            filename_version is not None
            and manifest.get("artifact_version") != filename_version
        ):
            report.reject(
                "the artifact file name version "
                f"{filename_version!r} disagrees with the manifest "
                f"artifact_version {manifest.get('artifact_version')!r}"
            )

    try:
        archive = zipfile.ZipFile(path)
    except (FileNotFoundError, zipfile.BadZipFile, OSError) as error:
        report.reject(f"the artifact cannot be read as a ZIP archive: {error}")
        return report.emit()

    with archive:
        names = archive.namelist()
        duplicates = duplicate_names(names)
        if duplicates:
            for name in duplicates:
                report.reject(f"the artifact has duplicate ZIP member path {name!r}")
            return report.emit()

        if EMBEDDED_METADATA_PATH not in names:
            report.reject(
                f"the artifact has no embedded metadata at {EMBEDDED_METADATA_PATH}"
            )
            embedded = None
        else:
            try:
                embedded = json.loads(
                    archive.read(EMBEDDED_METADATA_PATH).decode("utf-8")
                )
            except (UnicodeDecodeError, json.JSONDecodeError) as error:
                report.reject(
                    f"the embedded metadata cannot be read as JSON: {error}"
                )
                embedded = None

        if embedded is not None:
            if require_fields(embedded, report, "the embedded metadata"):
                check_field_types(embedded, report, "the embedded metadata")
                if manifest is not None and isinstance(manifest, dict):
                    compare_documents(manifest, embedded, report)

            abis = embedded.get("abis")
            if isinstance(abis, list) and all(isinstance(abi, str) for abi in abis):
                check_native_libraries(archive, names, abis, report)
            else:
                report.reject(
                    "the declared ABI list is unusable, so the native layout "
                    "cannot be checked"
                )

        for required in (ANDROID_MANIFEST, CONSUMER_RULES):
            if required not in names:
                report.reject(f"the artifact has no {required}")

        check_binding_classes(archive, names, report)

    return report.emit()


def main(argv: list[str] | None = None) -> int:
    """Parse the frozen CLI and check one artifact."""
    parser = argparse.ArgumentParser(
        description="Check one fretboard-engine AAR against its manifest."
    )
    parser.add_argument("--aar", required=True, help="path to the artifact")
    parser.add_argument(
        "--manifest", required=True, help="path to artifact-manifest.json"
    )
    arguments = parser.parse_args(argv)
    return check_artifact(arguments.aar, arguments.manifest)


if __name__ == "__main__":
    sys.exit(main())
