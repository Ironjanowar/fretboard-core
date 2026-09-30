"""Behaviour tests for the Android engine artifact checker, ``scripts/check_aar.py``.

These are *checker* unit tests: every test builds a tiny synthetic AAR (a ZIP
with the expected layout) and a synthetic artifact manifest in a temporary
directory, then invokes the frozen CLI as a subprocess::

    python3 scripts/check_aar.py --aar <path> --manifest <path>

They never import checker internals and never consume a real engine artifact.
Synthetic fixtures exist only to specify the checker contract; the shipped AAR
is produced by task ``C05`` / ``Delivery.D01`` and is never one of these files.

Frozen contract
---------------

Exit 0 when the artifact is conforming, exit 1 otherwise; every problem prints
one ``ERROR: <message>`` line to stderr; nothing at all is printed to stdout on
success. Rejections exit exactly 1 (not an arbitrary nonzero code), matching the
sibling checker ``tools/oracle/check_export.py``.

The artifact is ``fretboard-engine-<version>.aar`` (release asset name from
``06-delivery.md`` D01 item 8). ``--manifest`` is the published
``artifact-manifest.json`` sidecar; the AAR carries a copy of the same metadata
at ``META-INF/fretboard-engine/metadata.json`` so the sidecar can be checked
against the artifact *bytes* and not only against the filename.

Artifact metadata fields (frozen, snake_case, top level of both JSON documents):

    artifact_version           str    e.g. "0.1.0"; must equal the filename version
    source_commit              str    40-char lowercase hex core commit
    tool_versions              object non-empty map of tool name -> version string
    uniffi_runtime_dependency  str    Maven coordinates "group:artifact:version"
    api_version                int    frozen adapter API version (currently 8)
    snapshot_schema_version    int    frozen snapshot schema version (currently 1)
    binding_package            str    "dev.ironjanowar.fretboard.core" (uniffi.toml)
    abis                       list   ["arm64-v8a", "x86_64"]
    min_sdk                    int    Android minimum API level (29 per DEC-10)
    licenses                   list   non-empty list of non-empty license ids

All ten fields are required in both documents, and the published manifest must
agree with the embedded metadata on all ten.

AAR layout the checker must validate:

    jni/<abi>/libfretboard_mobile_ffi.so   one native library per declared ABI
    classes.jar                            ZIP containing the generated
                                           binding classes under the frozen
                                           binding package
    proguard.txt                           consumer shrinker rules
    AndroidManifest.xml                    present in the archive
    META-INF/fretboard-engine/metadata.json  embedded artifact metadata

See ``docs/toolchains.md`` and ``docs/decisions.md`` (``DEC-10``):
``arm64-v8a`` and ``x86_64`` ship and ``minSdk`` is 29. The checker requires
that shipped ABI set and compares the archive against the metadata declaration.
"""

import io
import json
import os
import struct
import subprocess
import sys
import tempfile
import unittest
import warnings
import zipfile
from pathlib import Path

# The test lives in scripts/tests/, so the repository root is two levels up.
REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = os.path.join("scripts", "check_aar.py")

ARTIFACT_VERSION = "0.1.0"
SOURCE_COMMIT = "a" * 40
OTHER_SOURCE_COMMIT = "b" * 40

TOOL_VERSIONS = {
    "rust": "1.98.1",
    "uniffi": "0.32.2",
    "cargo_ndk": "3.5.4",
    "ndk": "28.2.13676358",
    "agp": "9.4.1",
    "gradle": "9.8.0",
    "kotlin": "2.4.20",
}
UNIFFI_RUNTIME_DEPENDENCY = "net.java.dev.jna:jna:5.17.0"
API_VERSION = 8
SNAPSHOT_SCHEMA_VERSION = 1
BINDING_PACKAGE = "dev.ironjanowar.fretboard.core"
ABI_ARM64 = "arm64-v8a"
ABI_X86_64 = "x86_64"
ABI_ARMEABI_V7A = "armeabi-v7a"
SHIPPED_ABIS = (ABI_ARM64, ABI_X86_64)
EM_X86_64 = 62
EM_AARCH64 = 183
MIN_SDK = 29
LICENSES = ["MIT", "Apache-2.0"]

NATIVE_LIB_NAME = "libfretboard_mobile_ffi.so"
EMBEDDED_METADATA_PATH = "META-INF/fretboard-engine/metadata.json"
CONSUMER_RULES_NAME = "proguard.txt"

FROZEN_METADATA_FIELDS = (
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

# A minimal, valid-enough class-file header; the checker only needs the entry
# names inside classes.jar, never the bytecode semantics.
CLASS_FILE_BYTES = b"\xca\xfe\xba\xbe\x00\x00\x00\x34"
CONSUMER_RULES_BYTES = (
    b"-keep class dev.ironjanowar.fretboard.core.** { *; }\n"
)


# --------------------------------------------------------------------------
# Synthetic fixture construction
# --------------------------------------------------------------------------


def zip_bytes(entries):
    """Build a ZIP in memory from a mapping or an ordered entry sequence."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        items = entries.items() if hasattr(entries, "items") else entries
        with warnings.catch_warnings():
            warnings.filterwarnings(
                "ignore", message="Duplicate name: .*", category=UserWarning
            )
            for name, data in items:
                archive.writestr(name, data)
    return buffer.getvalue()


def json_bytes(document):
    """Serialize a metadata document the way the release tooling writes it."""
    return (json.dumps(document, sort_keys=True, indent=2) + "\n").encode("utf-8")


def elf64_shared_object(machine):
    """Build a complete ELF64 header for a little-endian shared object."""
    identification = b"\x7fELF\x02\x01\x01\x00" + b"\x00" * 8
    return identification + struct.pack(
        "<HHIQQQIHHHHHH",
        3,  # e_type: ET_DYN
        machine,
        1,  # e_version: EV_CURRENT
        0,  # e_entry
        0,  # e_phoff
        0,  # e_shoff
        0,  # e_flags
        64,  # e_ehsize
        56,  # e_phentsize
        0,  # e_phnum
        64,  # e_shentsize
        0,  # e_shnum
        0,  # e_shstrndx
    )


NATIVE_LIBRARY_BYTES_BY_ABI = {
    ABI_ARM64: elf64_shared_object(EM_AARCH64),
    ABI_X86_64: elf64_shared_object(EM_X86_64),
}


def binding_classes_jar(package=BINDING_PACKAGE):
    """A ``classes.jar`` holding generated binding classes in ``package``."""
    prefix = package.replace(".", "/")
    return zip_bytes(
        {
            prefix + "/FretboardCore.class": CLASS_FILE_BYTES,
            prefix + "/FretboardCore$UniffiLib.class": CLASS_FILE_BYTES,
        }
    )


def artifact_metadata(**overrides):
    """A self-consistent artifact metadata document, with overrides applied."""
    metadata = {
        "artifact_version": ARTIFACT_VERSION,
        "source_commit": SOURCE_COMMIT,
        "tool_versions": dict(TOOL_VERSIONS),
        "uniffi_runtime_dependency": UNIFFI_RUNTIME_DEPENDENCY,
        "api_version": API_VERSION,
        "snapshot_schema_version": SNAPSHOT_SCHEMA_VERSION,
        "binding_package": BINDING_PACKAGE,
        "abis": list(SHIPPED_ABIS),
        "min_sdk": MIN_SDK,
        "licenses": list(LICENSES),
    }
    metadata.update(overrides)
    return metadata


def without(document, field):
    """A copy of ``document`` with ``field`` removed (a missing-field case)."""
    copy = dict(document)
    del copy[field]
    return copy


def aar_bytes(
    *,
    abis=SHIPPED_ABIS,
    extra_abi_dirs=(),
    stale_library_abi=None,
    duplicate_native_path_abi=None,
    binding_classes_package=BINDING_PACKAGE,
    include_binding_classes=True,
    include_consumer_rules=True,
    embedded_metadata=None,
    include_embedded_metadata=True,
    native_library_bytes_by_abi=NATIVE_LIBRARY_BYTES_BY_ABI,
):
    """A synthetic AAR whose layout matches ``abis`` and the given switches."""
    entries = {
        "AndroidManifest.xml": b"<manifest />\n",
        "R.txt": b"",
    }
    for abi in abis:
        entries["jni/%s/%s" % (abi, NATIVE_LIB_NAME)] = (
            native_library_bytes_by_abi[abi]
        )
    for abi in extra_abi_dirs:
        entries["jni/%s/%s" % (abi, NATIVE_LIB_NAME)] = elf64_shared_object(
            EM_AARCH64
        )
    if stale_library_abi is not None:
        entries["jni/%s/libstale_engine.so" % stale_library_abi] = (
            native_library_bytes_by_abi[stale_library_abi]
        )
    if include_binding_classes:
        entries["classes.jar"] = binding_classes_jar(binding_classes_package)
    if include_consumer_rules:
        entries[CONSUMER_RULES_NAME] = CONSUMER_RULES_BYTES
    if include_embedded_metadata:
        entries[EMBEDDED_METADATA_PATH] = json_bytes(
            embedded_metadata if embedded_metadata is not None else artifact_metadata()
        )
    archive_entries = list(entries.items())
    if duplicate_native_path_abi is not None:
        native_path = "jni/%s/%s" % (duplicate_native_path_abi, NATIVE_LIB_NAME)
        archive_entries.append(
            (native_path, native_library_bytes_by_abi[duplicate_native_path_abi])
        )
    return zip_bytes(archive_entries)


def make_case(
    root,
    *,
    version=ARTIFACT_VERSION,
    filename=None,
    manifest=None,
    embedded_metadata=None,
    **aar_kwargs
):
    """Materialize a synthetic AAR plus manifest; returns (aar_path, manifest_path).

    Unless overridden, the manifest declares ``version``, the filename encodes
    ``version``, and the embedded metadata mirrors the manifest -- so a default
    case is conforming and every test states exactly one deviation.
    """
    if manifest is None:
        manifest = artifact_metadata(artifact_version=version)
    if filename is None:
        filename = "fretboard-engine-%s.aar" % version
    if embedded_metadata is None:
        embedded_metadata = manifest
    aar_path = Path(root) / filename
    aar_path.write_bytes(
        aar_bytes(embedded_metadata=embedded_metadata, **aar_kwargs)
    )
    manifest_path = Path(root) / "artifact-manifest.json"
    manifest_path.write_bytes(json_bytes(manifest))
    return aar_path, manifest_path


def invoke(aar_path, manifest_path):
    """Run the frozen checker CLI against one artifact and its sidecar."""
    return subprocess.run(
        [
            sys.executable,
            CHECKER,
            "--aar",
            str(aar_path),
            "--manifest",
            str(manifest_path),
        ],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )


# --------------------------------------------------------------------------
# Tests
# --------------------------------------------------------------------------


class CheckAarTestCase(unittest.TestCase):
    """Shared setup: every test owns a fresh temporary directory."""

    def setUp(self):
        temp_dir = tempfile.TemporaryDirectory(prefix="aar-checker-test-")
        self.addCleanup(temp_dir.cleanup)
        self.tmp = Path(temp_dir.name)

    def assert_accepted(self, proc):
        self.assertEqual(
            0,
            proc.returncode,
            msg="expected exit 0, got %d; stderr=%r"
            % (proc.returncode, proc.stderr),
        )
        self.assertEqual(
            "",
            proc.stdout,
            msg="expected empty stdout on success, got %r" % proc.stdout,
        )
        self.assertEqual(
            "",
            proc.stderr,
            msg="expected empty stderr on success, got %r" % proc.stderr,
        )

    def assert_rejected(self, proc, context=""):
        self.assertEqual(
            1,
            proc.returncode,
            msg="expected exit 1 %s, got %d; stderr=%r"
            % (context, proc.returncode, proc.stderr),
        )
        errors = [
            line for line in proc.stderr.splitlines() if line.startswith("ERROR: ")
        ]
        self.assertTrue(
            errors,
            msg="expected at least one 'ERROR: ' line on stderr %s; stderr=%r"
            % (context, proc.stderr),
        )


class HappyPathTests(CheckAarTestCase):
    def test_conforming_artifact_is_accepted_with_no_output(self):
        aar, manifest = make_case(self.tmp)
        self.assert_accepted(invoke(aar, manifest))

    def test_shipped_abi_set_is_accepted(self):
        metadata = artifact_metadata(abis=list(SHIPPED_ABIS))
        aar, manifest = make_case(
            self.tmp, manifest=metadata, abis=SHIPPED_ABIS
        )
        self.assert_accepted(invoke(aar, manifest))


class NativeLibraryTests(CheckAarTestCase):
    def test_native_libraries_mislabeled_with_the_other_architecture_are_rejected(self):
        swapped_libraries = {
            ABI_ARM64: NATIVE_LIBRARY_BYTES_BY_ABI[ABI_X86_64],
            ABI_X86_64: NATIVE_LIBRARY_BYTES_BY_ABI[ABI_ARM64],
        }
        aar, manifest = make_case(
            self.tmp, native_library_bytes_by_abi=swapped_libraries
        )
        self.assert_rejected(
            invoke(aar, manifest),
            "when each ABI directory contains the other architecture's ELF library",
        )

    def test_artifact_missing_a_required_shipped_abi_is_rejected(self):
        metadata = artifact_metadata(abis=[ABI_ARM64])
        aar, manifest = make_case(
            self.tmp, manifest=metadata, abis=(ABI_ARM64,)
        )
        self.assert_rejected(
            invoke(aar, manifest),
            "when the artifact and metadata both omit the required x86_64 ABI",
        )

    def test_missing_native_library_for_declared_abi_is_rejected(self):
        aar, manifest = make_case(self.tmp, abis=())
        self.assert_rejected(
            invoke(aar, manifest),
            "for a declared ABI with no native library in the archive",
        )

    def test_unexpected_native_library_for_undeclared_abi_is_rejected(self):
        aar, manifest = make_case(self.tmp, extra_abi_dirs=(ABI_ARMEABI_V7A,))
        self.assert_rejected(
            invoke(aar, manifest),
            "for a native library for an ABI the manifest does not declare",
        )

    def test_stale_differently_named_native_library_in_declared_abi_is_rejected(self):
        aar, manifest = make_case(self.tmp, stale_library_abi=ABI_ARM64)
        self.assert_rejected(
            invoke(aar, manifest),
            "for a stale, differently named native library in a declared ABI directory",
        )

    def test_duplicate_entries_for_exact_native_library_path_are_rejected(self):
        native_path = "jni/%s/%s" % (ABI_X86_64, NATIVE_LIB_NAME)
        aar, manifest = make_case(
            self.tmp, duplicate_native_path_abi=ABI_X86_64
        )
        with zipfile.ZipFile(aar) as archive:
            self.assertEqual(2, archive.namelist().count(native_path))
        self.assert_rejected(
            invoke(aar, manifest),
            "for duplicate ZIP entries with the exact path %s" % native_path,
        )


class BindingClassesTests(CheckAarTestCase):
    def test_missing_generated_binding_classes_are_rejected(self):
        aar, manifest = make_case(self.tmp, include_binding_classes=False)
        self.assert_rejected(
            invoke(aar, manifest), "for an archive without classes.jar"
        )

    def test_binding_classes_for_a_foreign_package_are_rejected(self):
        aar, manifest = make_case(
            self.tmp, binding_classes_package="com.example.other"
        )
        self.assert_rejected(
            invoke(aar, manifest),
            "for binding classes outside the frozen package",
        )

    def test_missing_consumer_shrinker_rules_are_rejected(self):
        aar, manifest = make_case(self.tmp, include_consumer_rules=False)
        self.assert_rejected(
            invoke(aar, manifest), "for an archive without consumer shrinker rules"
        )


class VersionAndFilenameTests(CheckAarTestCase):
    def test_manifest_version_disagreeing_with_the_filename_is_rejected(self):
        aar, manifest = make_case(
            self.tmp,
            version=ARTIFACT_VERSION,
            filename="fretboard-engine-9.9.9.aar",
        )
        self.assert_rejected(
            invoke(aar, manifest),
            "for a manifest version that disagrees with the artifact filename",
        )

    def test_filename_without_a_version_is_rejected(self):
        aar, manifest = make_case(self.tmp, filename="engine.aar")
        self.assert_rejected(
            invoke(aar, manifest),
            "for a filename that is not fretboard-engine-<version>.aar",
        )

    def test_manifest_version_disagreeing_with_the_embedded_metadata_is_rejected(
        self,
    ):
        aar, manifest = make_case(
            self.tmp, embedded_metadata=artifact_metadata(artifact_version="9.9.9")
        )
        self.assert_rejected(
            invoke(aar, manifest),
            "for a manifest version that disagrees with the artifact contents",
        )


class MetadataAgreementTests(CheckAarTestCase):
    """The published manifest must agree with the metadata embedded in the AAR."""

    def _rejected_for_embedded(self, **overrides):
        aar, manifest = make_case(
            self.tmp, embedded_metadata=artifact_metadata(**overrides)
        )
        self.assert_rejected(
            invoke(aar, manifest),
            "for embedded metadata %r disagreeing with the manifest" % (overrides,),
        )

    def test_min_sdk_disagreeing_with_the_artifact_contents_is_rejected(self):
        self._rejected_for_embedded(min_sdk=21)

    def test_api_version_disagreeing_with_the_artifact_contents_is_rejected(self):
        self._rejected_for_embedded(api_version=9)

    def test_snapshot_schema_version_disagreeing_with_the_artifact_contents_is_rejected(
        self,
    ):
        self._rejected_for_embedded(snapshot_schema_version=2)

    def test_source_commit_disagreeing_with_the_artifact_contents_is_rejected(self):
        self._rejected_for_embedded(source_commit=OTHER_SOURCE_COMMIT)

    def test_declared_abi_list_disagreeing_with_the_artifact_contents_is_rejected(
        self,
    ):
        self._rejected_for_embedded(abis=[ABI_X86_64])


class RequiredMetadataTests(CheckAarTestCase):
    def test_missing_licenses_list_is_rejected(self):
        manifest = without(artifact_metadata(), "licenses")
        aar, manifest_path = make_case(
            self.tmp, manifest=manifest, embedded_metadata=manifest
        )
        self.assert_rejected(invoke(aar, manifest_path), "for a missing licenses list")

    def test_missing_uniffi_runtime_dependency_is_rejected(self):
        manifest = without(artifact_metadata(), "uniffi_runtime_dependency")
        aar, manifest_path = make_case(self.tmp, manifest=manifest)
        self.assert_rejected(
            invoke(aar, manifest_path),
            "for missing UniFFI runtime dependency coordinates",
        )

    def test_missing_tool_versions_is_rejected(self):
        manifest = without(artifact_metadata(), "tool_versions")
        aar, manifest_path = make_case(self.tmp, manifest=manifest)
        self.assert_rejected(
            invoke(aar, manifest_path), "for a missing tool-versions map"
        )

    def test_missing_min_sdk_is_rejected(self):
        manifest = without(artifact_metadata(), "min_sdk")
        aar, manifest_path = make_case(self.tmp, manifest=manifest)
        self.assert_rejected(invoke(aar, manifest_path), "for a missing min_sdk")

    def test_embedded_metadata_missing_a_required_field_is_rejected(self):
        embedded = without(artifact_metadata(), "licenses")
        aar, manifest = make_case(self.tmp, embedded_metadata=embedded)
        self.assert_rejected(
            invoke(aar, manifest),
            "for embedded metadata without the licenses list",
        )

    def test_archive_without_embedded_metadata_is_rejected(self):
        aar, manifest = make_case(self.tmp, include_embedded_metadata=False)
        self.assert_rejected(
            invoke(aar, manifest), "for an archive with no embedded artifact metadata"
        )


if __name__ == "__main__":
    unittest.main()
